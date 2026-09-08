//! Phase 6C: Digestion Efficiency Analysis Tool
//!
//! Two-pass workflow for assessing digestion efficiency:
//! 1. subset: Extract proteins from Pass 1 results, create subset FASTA
//! 2. annotate: Classify peptide termini from Pass 2 results
//!
//! Usage (paths relative to testing/):
//!     # Step 1: Subset FASTA to identified proteins
//!     digestion_efficiency subset \
//!         --tsv search-output/digestion-pass1/results.sage.tsv \
//!         --fasta inputs/UniProt-Human-UP000005640_canonical-2023_05.fasta \
//!         --output inputs/subset_identified_proteins.fasta
//!
//!     # Step 2: Annotate termini and compute digestion score
//!     digestion_efficiency annotate \
//!         --tsv search-output/digestion-pass2/results.sage.tsv \
//!         --fasta inputs/subset_identified_proteins.fasta \
//!         --output search-output/annotated_termini.tsv \
//!         --output-json search-output/digestion_efficiency_result.json
//!
//! Note: Sage handles decoy generation internally, so the subset FASTA only
//! needs target sequences. Decoys are generated on-the-fly during the search.
//!
//! Cargo.toml dependencies:
//!     clap = { version = "4", features = ["derive"] }
//!     serde = { version = "1", features = ["derive"] }
//!     serde_json = "1"

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};

use clap::{Parser, Subcommand};
use serde_json::json;

// =============================================================================
// CLI definition
// =============================================================================

#[derive(Parser)]
#[command(
    name = "digestion_efficiency",
    about = "Digestion Efficiency Analysis Tool"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Subset FASTA to proteins identified in Pass 1
    Subset {
        /// Path to results.sage.tsv from Pass 1
        #[arg(long)]
        tsv: String,
        /// Path to original FASTA file
        #[arg(long)]
        fasta: String,
        /// Path for output subset FASTA
        #[arg(long)]
        output: String,
        /// Peptide q-value threshold
        #[arg(long = "q-value", default_value_t = 0.01)]
        q_value: f64,
    },
    /// Annotate termini and report digestion breakdown
    Annotate {
        /// Path to results.sage.tsv from Pass 2
        #[arg(long)]
        tsv: String,
        /// Path to FASTA file (same as used in search)
        #[arg(long)]
        fasta: String,
        /// Path for output annotated TSV
        #[arg(long)]
        output: String,
        /// Path for JSON summary output
        #[arg(long = "output-json")]
        output_json: Option<String>,
        /// Peptide q-value threshold
        #[arg(long = "q-value", default_value_t = 0.01)]
        q_value: f64,
    },
}

// =============================================================================
// Shared utilities
// =============================================================================

/// Extract accession from FASTA header.
/// Returns the full UniProt format: sp|P12345|GENE_HUMAN
/// This matches what Sage outputs in the proteins column.
fn extract_accession(header: &str) -> String {
    let header = header.trim_start_matches('>');
    header.split_whitespace().next().unwrap_or("").to_string()
}

/// Remove modification annotations from peptide sequence.
/// Handles formats like: [+42.011]-PEPTIDE or PEPT[+15.995]IDE
fn strip_modifications(peptide: &str) -> String {
    // Remove bracketed modifications (non-nested, like the Python `\[.*?\]`).
    let mut without_brackets = String::with_capacity(peptide.len());
    let mut depth = 0i32;
    for c in peptide.chars() {
        match c {
            '[' => depth += 1,
            ']' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            _ if depth == 0 => without_brackets.push(c),
            _ => {}
        }
    }
    // Keep only uppercase A-Z (uppercasing first, mirroring .upper()).
    without_brackets
        .to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_uppercase())
        .collect()
}

/// A minimal TSV reader that maps header names to column indices,
/// mirroring Python's csv.DictReader behavior for tab-delimited files.
struct TsvReader<R: BufRead> {
    inner: R,
    headers: Vec<String>,
}

impl<R: BufRead> TsvReader<R> {
    fn new(mut inner: R) -> io::Result<Self> {
        let mut first = String::new();
        inner.read_line(&mut first)?;
        let headers = first
            .trim_end_matches(['\n', '\r'])
            .split('\t')
            .map(|s| s.to_string())
            .collect();
        Ok(TsvReader { inner, headers })
    }

    fn headers(&self) -> &[String] {
        &self.headers
    }

    /// Read the next row as a map of header -> value. Returns None at EOF.
    fn next_row(&mut self) -> io::Result<Option<Row>> {
        let mut line = String::new();
        let n = self.inner.read_line(&mut line)?;
        if n == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\n', '\r']);
        let values: Vec<String> = trimmed.split('\t').map(|s| s.to_string()).collect();
        Ok(Some(Row {
            headers: self.headers.clone(),
            values,
        }))
    }
}

/// A single parsed TSV row with header-keyed access.
struct Row {
    headers: Vec<String>,
    values: Vec<String>,
}

impl Row {
    fn get(&self, key: &str) -> Option<&str> {
        self.headers
            .iter()
            .position(|h| h == key)
            .and_then(|i| self.values.get(i))
            .map(|s| s.as_str())
    }

    /// Reconstruct a full output line given an ordered set of field names.
    fn to_line(&self, fieldnames: &[String], extra: &[(&str, &str)]) -> String {
        let mut parts: Vec<String> = Vec::with_capacity(fieldnames.len());
        for name in fieldnames {
            if let Some((_, v)) = extra.iter().find(|(k, _)| k == name) {
                parts.push((*v).to_string());
            } else {
                parts.push(self.get(name).unwrap_or("").to_string());
            }
        }
        parts.join("\t")
    }
}

// =============================================================================
// FASTA parsing (implemented directly; no external crate)
// =============================================================================

/// Iterate over (header, sequence) pairs from a FASTA file.
fn parse_fasta_entries(fasta_path: &str) -> io::Result<Vec<(String, String)>> {
    let file = File::open(fasta_path)?;
    let reader = BufReader::new(file);

    let mut entries = Vec::new();
    let mut header: Option<String> = None;
    let mut sequence_lines: Vec<String> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim_end_matches(['\n', '\r']).to_string();
        if line.starts_with('>') {
            if let Some(h) = header.take() {
                entries.push((h, sequence_lines.concat()));
            }
            // Reset UNCONDITIONALLY, matching Python's `sequence_lines = []`.
            // Clearing only inside the `if let` leaves any content that appeared
            // BEFORE the first '>' in the buffer, which then gets concatenated
            // onto the first entry's sequence.
            sequence_lines.clear();
            header = Some(line);
        } else {
            sequence_lines.push(line);
        }
    }
    if let Some(h) = header.take() {
        entries.push((h, sequence_lines.concat()));
    }
    Ok(entries)
}

/// Load FASTA file into a map: accession -> sequence.
fn load_fasta_sequences(fasta_path: &str) -> io::Result<BTreeMap<String, String>> {
    let mut sequences = BTreeMap::new();
    for (header, sequence) in parse_fasta_entries(fasta_path)? {
        let acc = extract_accession(&header);
        sequences.insert(acc, sequence);
    }
    Ok(sequences)
}

// =============================================================================
// Subset command
// =============================================================================

/// Extract unique protein accessions from Sage TSV.
/// Filters to target PSMs (label=1) at q-value threshold.
fn extract_accessions_from_tsv(tsv_path: &str, q_threshold: f64) -> io::Result<HashSet<String>> {
    let mut accessions = HashSet::new();
    let file = File::open(tsv_path)?;
    let mut reader = TsvReader::new(BufReader::new(file))?;

    while let Some(row) = reader.next_row()? {
        if row.get("label") != Some("1") {
            continue;
        }

        let peptide_q = match row.get("peptide_q") {
            Some(s) => match s.parse::<f64>() {
                Ok(v) => v,
                Err(_) => continue,
            },
            None => 1.0,
        };

        if peptide_q > q_threshold {
            continue;
        }

        let proteins = row.get("proteins").unwrap_or("");
        for protein in proteins.split(';') {
            let protein = protein.trim();
            if !protein.is_empty() {
                accessions.insert(protein.to_string());
            }
        }
    }

    Ok(accessions)
}

/// Write subset FASTA containing only specified accessions.
///
/// Note: Sage handles decoy generation internally, so we only need
/// target sequences. Decoys are generated on-the-fly during the search.
fn subset_fasta(
    fasta_path: &str,
    accessions: &HashSet<String>,
    output_path: &str,
) -> io::Result<usize> {
    let out = File::create(output_path)?;
    let mut writer = BufWriter::new(out);
    let mut entries_written = 0;

    for (header, sequence) in parse_fasta_entries(fasta_path)? {
        let acc = extract_accession(&header);
        if accessions.contains(&acc) {
            writeln!(writer, "{}", header)?;
            // Write sequence in 60-char lines.
            let bytes = sequence.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                let end = (i + 60).min(bytes.len());
                writer.write_all(&bytes[i..end])?;
                writer.write_all(b"\n")?;
                i = end;
            }
            entries_written += 1;
        }
    }

    Ok(entries_written)
}

fn cmd_subset(tsv: &str, fasta: &str, output: &str, q_value: f64) -> io::Result<()> {
    println!("Reading PSMs from: {}", tsv);
    let accessions = extract_accessions_from_tsv(tsv, q_value)?;
    println!(
        "Found {} unique target protein accessions at q<={}",
        accessions.len(),
        q_value
    );

    println!("Subsetting FASTA: {}", fasta);
    let entries_written = subset_fasta(fasta, &accessions, output)?;

    println!("Wrote {} entries to: {}", entries_written, output);
    println!("  (Sage will generate decoys internally during Pass 2)");
    Ok(())
}

// =============================================================================
// Annotate command
// =============================================================================

/// Check if peptide N-terminus is tryptic.
///
/// Tryptic N-terminus means:
/// - Peptide starts at protein N-terminus (position 0), OR
/// - Residue before peptide is K or R (and not followed by P)
fn is_tryptic_nterm(peptide: &[u8], protein_seq: &[u8], start_pos: usize) -> bool {
    if start_pos == 0 {
        return true;
    }

    let prev_residue = protein_seq[start_pos - 1];
    if prev_residue == b'K' || prev_residue == b'R' {
        if peptide[0] == b'P' {
            return false;
        }
        return true;
    }

    false
}

/// Check if peptide C-terminus is tryptic.
///
/// Tryptic C-terminus means:
/// - Peptide ends at protein C-terminus, OR
/// - Peptide ends with K or R (and next residue is not P)
fn is_tryptic_cterm(peptide: &[u8], protein_seq: &[u8], end_pos: usize) -> bool {
    if end_pos >= protein_seq.len() {
        return true;
    }

    let last_residue = peptide[peptide.len() - 1];
    if last_residue == b'K' || last_residue == b'R' {
        if end_pos < protein_seq.len() && protein_seq[end_pos] == b'P' {
            return false;
        }
        return true;
    }

    false
}

/// Find peptide position in protein sequence.
/// Returns Some((start, end)) with end exclusive, or None if not found.
fn find_peptide_position(peptide: &str, protein_seq: &str) -> Option<(usize, usize)> {
    protein_seq
        .find(peptide)
        .map(|start| (start, start + peptide.len()))
}

/// Every terminus class the report can carry, including the two that
/// `classify_terminus` does not return (`protein_not_found` is set by the
/// caller). This is the canonical set for the JSON `counts` object.
const TERMINUS_CLASSES: [&str; 6] = [
    "fully_tryptic",
    "semi_n_ragged",
    "semi_c_ragged",
    "non_tryptic",
    "not_found",
    "protein_not_found",
];

/// Classify peptide terminus specificity.
fn classify_terminus(peptide: &str, protein_seq: &str) -> &'static str {
    let (start_pos, end_pos) = match find_peptide_position(peptide, protein_seq) {
        Some(pos) => pos,
        None => return "not_found",
    };

    let pep_bytes = peptide.as_bytes();
    let prot_bytes = protein_seq.as_bytes();

    let n_tryptic = is_tryptic_nterm(pep_bytes, prot_bytes, start_pos);
    let c_tryptic = is_tryptic_cterm(pep_bytes, prot_bytes, end_pos);

    match (n_tryptic, c_tryptic) {
        (true, true) => "fully_tryptic",
        (false, true) => "semi_n_ragged",
        (true, false) => "semi_c_ragged",
        (false, false) => "non_tryptic",
    }
}

fn cmd_annotate(
    tsv: &str,
    fasta: &str,
    output: &str,
    output_json: Option<&str>,
    q_value: f64,
) -> io::Result<()> {
    println!("Loading FASTA: {}", fasta);
    let sequences = load_fasta_sequences(fasta)?;
    println!("Loaded {} protein sequences", sequences.len());

    println!("Processing PSMs from: {}", tsv);

    // Counters. BTreeMap keeps missed-cleavage keys sorted for reporting.
    // Seed EVERY terminus class at zero. An absent key and a zero count are
    // different claims, and a digestion report where a class is empty is
    // exactly where a reader needs the explicit 0. Python gets the same six
    // keys, but only as a side effect of `defaultdict` access materialising
    // them before `dict(counts)` runs - so this states the schema instead of
    // inheriting it by accident.
    let mut counts: BTreeMap<String, u64> = TERMINUS_CLASSES
        .iter()
        .map(|c| (c.to_string(), 0u64))
        .collect();
    let mut mc_counts: BTreeMap<i64, u64> = BTreeMap::new();
    let mut total_psms: u64 = 0;
    let mut filtered_psms: u64 = 0;

    let infile = File::open(tsv)?;
    let mut reader = TsvReader::new(BufReader::new(infile))?;

    // Output fieldnames = input headers + terminus_class.
    let mut fieldnames: Vec<String> = reader.headers().to_vec();
    fieldnames.push("terminus_class".to_string());

    let outfile = File::create(output)?;
    let mut writer = BufWriter::new(outfile);
    writeln!(writer, "{}", fieldnames.join("\t"))?;

    while let Some(row) = reader.next_row()? {
        total_psms += 1;

        if row.get("label") != Some("1") {
            continue;
        }

        let peptide_q = match row.get("peptide_q") {
            Some(s) => match s.parse::<f64>() {
                Ok(v) => v,
                Err(_) => continue,
            },
            None => 1.0,
        };

        if peptide_q > q_value {
            continue;
        }

        filtered_psms += 1;

        // Track missed cleavages.
        if let Some(s) = row.get("missed_cleavages") {
            if let Ok(mc) = s.parse::<i64>() {
                *mc_counts.entry(mc).or_insert(0) += 1;
            }
        }

        let peptide = row.get("peptide").unwrap_or("");
        let stripped_peptide = strip_modifications(peptide);
        let proteins = row.get("proteins").unwrap_or("");
        let protein_acc = proteins.split(';').next().unwrap_or("").trim();

        let terminus_class = match sequences.get(protein_acc) {
            Some(seq) if !seq.is_empty() => classify_terminus(&stripped_peptide, seq),
            _ => "protein_not_found",
        };

        *counts.entry(terminus_class.to_string()).or_insert(0) += 1;

        let line = row.to_line(&fieldnames, &[("terminus_class", terminus_class)]);
        writeln!(writer, "{}", line)?;
    }
    writer.flush()?;

    // Compute statistics.
    let count_of = |k: &str| *counts.get(k).unwrap_or(&0);
    let semi_n = count_of("semi_n_ragged");
    let semi_c = count_of("semi_c_ragged");
    let semi_total = semi_n + semi_c;

    let pct = |n: u64| {
        if filtered_psms > 0 {
            n as f64 / filtered_psms as f64 * 100.0
        } else {
            0.0
        }
    };

    let semi_pct = pct(semi_total);
    let n_ragged_pct = pct(semi_n);
    let c_ragged_pct = pct(semi_c);

    // MC distribution as percentages.
    let mut mc_distribution: BTreeMap<i64, f64> = BTreeMap::new();
    for (mc, count) in &mc_counts {
        mc_distribution.insert(*mc, pct(*count));
    }

    // NOTE: no composite "digestion score" is computed. A single judgmental
    // score assumes a cell-culture tryptic digest and misreads biofluids —
    // e.g. serum's ~32% semi-tryptic is endogenous proteolysis, not a bad
    // digest. The tool can't know sample type, so it presents the raw
    // breakdown and lets the user judge. (See NOTES "Known permanent
    // limitations".)

    // Print summary.
    println!("\n{}", "=".repeat(60));
    println!("DIGESTION EFFICIENCY REPORT");
    println!("{}", "=".repeat(60));
    println!("\nTotal PSMs in file: {}", total_psms);
    println!(
        "PSMs after filtering (q<={}, target): {}",
        q_value, filtered_psms
    );

    println!("\n--- Missed Cleavage Distribution ---");
    for (mc, count) in &mc_counts {
        let p = mc_distribution.get(mc).copied().unwrap_or(0.0);
        println!("  MC={}: {} ({:.1}%)", mc, format_thousands(*count), p);
    }

    println!("\n--- Terminus Classification ---");
    for cls in [
        "fully_tryptic",
        "semi_n_ragged",
        "semi_c_ragged",
        "non_tryptic",
        "not_found",
        "protein_not_found",
    ] {
        let c = count_of(cls);
        if c > 0 {
            println!("  {}: {} ({:.1}%)", cls, format_thousands(c), pct(c));
        }
    }

    println!(
        "\n  Total semi-tryptic: {} ({:.1}%)",
        format_thousands(semi_total),
        semi_pct
    );
    if semi_c > 0 {
        let nc_ratio = semi_n as f64 / semi_c as f64;
        println!("  N-ragged:C-ragged ratio: {:.1}:1", nc_ratio);
    }

    println!("\n  (No composite 'digestion score' — the raw breakdown above is");
    println!("   the report. Whether a given semi-tryptic rate is expected is a");
    println!("   sample-type call for the user, not the tool.)");

    println!("\nOutput written to: {}", output);

    // JSON output if requested.
    if let Some(json_path) = output_json {
        let n_c_ratio = if semi_c > 0 {
            Some(round2(semi_n as f64 / semi_c as f64))
        } else {
            None
        };

        let mc_dist: BTreeMap<String, u64> =
            mc_counts.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let mc_dist_pct: BTreeMap<String, f64> = mc_distribution
            .iter()
            .map(|(k, v)| (k.to_string(), round2(*v)))
            .collect();

        let counts_obj: BTreeMap<String, u64> =
            counts.iter().map(|(k, v)| (k.clone(), *v)).collect();

        let result = json!({
            "summary": {
                "total_psms": total_psms,
                "filtered_psms": filtered_psms,
                "q_value_threshold": q_value
            },
            "missed_cleavages": {
                "distribution": mc_dist,
                "distribution_pct": mc_dist_pct
            },
            "terminus_classification": {
                "counts": counts_obj,
                "fully_tryptic_pct": if filtered_psms > 0 { round2(pct(count_of("fully_tryptic"))) } else { 0.0 },
                "semi_tryptic_total": semi_total,
                "semi_tryptic_pct": round2(semi_pct),
                "semi_n_ragged_pct": round2(n_ragged_pct),
                "semi_c_ragged_pct": round2(c_ragged_pct),
                "n_c_ratio": n_c_ratio
            }
        });

        let json_file = File::create(json_path)?;
        let mut json_writer = BufWriter::new(json_file);
        serde_json::to_writer_pretty(&mut json_writer, &result)?;
        json_writer.flush()?;

        println!("JSON output written to: {}", json_path);
    }

    Ok(())
}

// =============================================================================
// Small formatting helpers
// =============================================================================

/// Round to 2 decimal places, matching Python's round(x, 2) closely enough
/// for reporting purposes.
fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Format an integer with thousands separators (mirrors Python's {:,}).
fn format_thousands(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let len = bytes.len();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

// =============================================================================
// Main entry point
// =============================================================================

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Subset {
            tsv,
            fasta,
            output,
            q_value,
        } => cmd_subset(&tsv, &fasta, &output, q_value),
        Command::Annotate {
            tsv,
            fasta,
            output,
            output_json,
            q_value,
        } => cmd_annotate(&tsv, &fasta, &output, output_json.as_deref(), q_value),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
