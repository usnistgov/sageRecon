//! Protein sequences from the search FASTA — the only way to know where a
//! peptide sits in its protein.
//!
//! Sage's TSV has no start-position column. It reports the peptide and the
//! accessions it came from, and nothing else about placement. A protein-terminal
//! modification cannot be decided without that placement, which is why
//! `curated_mods::needs_protein_context` routed every such candidate to the
//! abundance path until this module existed. See NOTES "Met-loss encoding".
//!
//! **What "protein position 0" buys.** It is rare on its own — 0.4-1.6% of
//! confident PSMs across the three test files — so it discriminates without any
//! residue argument. The peptide-level shortcut (does the peptide START with the
//! acceptor) is NOT the same test and is much weaker: on bcell, peptides
//! beginning with M are 3.17% of all PSMs against 1.09% at protein position 0,
//! and the shortcut passes every internal tryptic peptide that happens to begin
//! with M.
//!
//! **The Met-loss case reads residue 1, not residue 2.** The open search matched
//! the Met-RETAINED peptide and put the acetyl-minus-Met difference into the
//! delta mass, so the PSM's peptide still starts at protein position 0 with its
//! initiator Met. `TG=M` is the hard chemical requirement and this module proves
//! the position. Whether protein residue 2 makes excision plausible is the
//! reader's call — no enzymology is encoded here, deliberately.

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufWriter, Write};
use std::path::Path;

/// Minimum fraction of TARGET PSMs whose accessions must resolve in the FASTA.
///
/// The tripwire this guards: a mismatched FASTA resolves nothing, every
/// protein-terminal candidate then fails its acceptor test, and the report reads
/// "no protein N-terminal modifications present" — a confident wrong answer with
/// no symptom. Measured 100.00% on all three test files (12438/12438,
/// 55419/55419, 22298/22298), so 0.95 is a wide margin, not a tuned threshold.
pub const MIN_RESOLVED_FRACTION: f64 = 0.95;

/// Accession -> protein sequence, from the search FASTA.
#[derive(Debug, Default)]
pub struct ProteinIndex {
    seqs: HashMap<String, String>,
}

impl ProteinIndex {
    /// Read a FASTA into an accession -> sequence map.
    ///
    /// The key is the header's FIRST whitespace-delimited token, which is exactly
    /// what Sage writes into its `proteins` column (`sp|P09651|ROA1_HUMAN`).
    /// Matching on anything else -- a parsed UniProt accession, say -- would
    /// silently miss every entry whose header does not follow that convention.
    pub fn from_fasta(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read FASTA: {}", path.display()))?;
        let mut seqs: HashMap<String, String> = HashMap::new();
        let mut acc: Option<String> = None;
        let mut buf = String::new();
        for line in text.lines() {
            if let Some(header) = line.strip_prefix('>') {
                if let Some(a) = acc.take() {
                    seqs.insert(a, std::mem::take(&mut buf));
                }
                acc = header.split_whitespace().next().map(str::to_string);
                buf.clear();
            } else {
                buf.push_str(line.trim());
            }
        }
        if let Some(a) = acc {
            seqs.insert(a, buf);
        }
        Ok(Self { seqs })
    }

    pub fn len(&self) -> usize {
        self.seqs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seqs.is_empty()
    }

    /// The sequence for one accession, exactly as `proteins`-column keyed.
    ///
    /// Returns `None` for an accession the FASTA did not carry -- a decoy, or a
    /// mismatched database. Callers must treat that as "not testable", never as
    /// "not at a terminus"; `digestion::classify_terminus` has its own
    /// `ProteinNotFound` class for it.
    pub fn sequence(&self, accession: &str) -> Option<&str> {
        self.seqs.get(accession).map(String::as_str)
    }

    /// True when at least one listed accession is present in the index.
    ///
    /// Separate from `starts_protein` on purpose: "this peptide is not at a
    /// protein N-terminus" and "this accession is not in the FASTA I was given"
    /// are different facts, and only the second one means the input is wrong.
    pub fn resolves(&self, proteins: &str) -> bool {
        proteins.split(';').any(|a| self.seqs.contains_key(a))
    }

    /// True when `residues` is a prefix of any of the listed proteins.
    ///
    /// `residues` must already be stripped of Sage's inline mod brackets -- pass
    /// `peak_composition::residues_of(peptide)`, never the raw peptide, or a
    /// `C[+57.0215]` would be compared against protein sequence letter for letter.
    ///
    /// A `proteins` entry that is not in the index yields false, not an error.
    /// Decoys are the expected case: `rev_` accessions never appear in a target
    /// FASTA, so a decoy PSM can never be read as protein N-terminal. The
    /// wrong-FASTA case is caught by `resolution` instead, which is why this
    /// function is allowed to be quiet.
    pub fn starts_protein(&self, residues: &str, proteins: &str) -> bool {
        if residues.is_empty() {
            return false;
        }
        proteins
            .split(';')
            .filter_map(|a| self.seqs.get(a))
            .any(|s| s.starts_with(residues))
    }

    /// `(resolved, total)` over the TARGET PSMs in `psms`.
    ///
    /// Decoys are excluded because they cannot resolve by construction; counting
    /// them would drag the fraction down by the decoy rate and make the guard
    /// threshold meaningless.
    pub fn resolution(&self, psms: &[crate::sage_results::Psm]) -> (usize, usize) {
        let targets = psms.iter().filter(|p| !p.is_decoy);
        let mut resolved = 0usize;
        let mut total = 0usize;
        for p in targets {
            total += 1;
            if self.resolves(&p.proteins) {
                resolved += 1;
            }
        }
        (resolved, total)
    }
}

/// Minimum DISTINCT peptides a protein needs before Pass 2 will search it.
///
/// The two-peptide rule. Pass 2's job is an accurate space in which to measure
/// semi-tryptic termini, NOT to confirm identifications, so a protein resting on
/// a single peptide adds search space without adding evidence.
///
/// MEASURED on bcell 2026-08-29: **24.1 % of the 6485 proteins Pass 1 identified
/// rest on ONE distinct peptide.** Dropping them leaves 4921 proteins and costs
/// 2.02 % of Pass 2's confident PSMs. Sage's own `protein_q <= 0.01` is far
/// weaker here — it removes only 346 proteins (0.19 % of PSMs) — so it is not
/// used as the gate.
pub const MIN_PEPTIDES_PER_PROTEIN: usize = 2;

/// Unique TARGET protein accessions in a Sage TSV at `q_threshold`, keeping only
/// proteins supported by at least `min_peptides` DISTINCT peptide sequences.
///
/// Every accession of every `;`-separated list is collected, not just the first.
/// That is deliberate and is NOT the same rule `classify_terminus` uses: the
/// subset FASTA must contain every protein a Pass-2 peptide could be assigned
/// to, or Pass 2 would be searched against a database that cannot explain its
/// own hits. Narrowing to the first protein here would silently shrink the
/// search space. See NOTES "digestion_efficiency port".
///
/// Peptides are counted by RESIDUES, with Sage's inline modification brackets
/// stripped, so `PEPTIDEK` and `PEPT[+15.995]IDEK` count once rather than twice.
/// Counting the raw strings would let one peptide in several modified forms
/// masquerade as independent evidence — exactly what the two-peptide rule exists
/// to exclude.
pub fn target_accessions(
    tsv: &Path,
    q_threshold: f64,
    min_peptides: usize,
) -> Result<HashSet<String>> {
    let file = std::fs::File::open(tsv)
        .with_context(|| format!("failed to open Sage TSV: {}", tsv.display()))?;
    let mut lines = std::io::BufReader::new(file).lines();
    let header = match lines.next() {
        Some(h) => h?,
        None => anyhow::bail!("empty Sage TSV: {}", tsv.display()),
    };
    let cols: Vec<&str> = header.split('\t').collect();
    let idx = |name: &str| cols.iter().position(|c| *c == name);
    let (label_i, q_i, prot_i, pep_i) = match (
        idx("label"),
        idx("peptide_q"),
        idx("proteins"),
        idx("peptide"),
    ) {
        (Some(a), Some(b), Some(c), Some(d)) => (a, b, c, d),
        _ => anyhow::bail!(
            "Sage TSV {} lacks one of label/peptide_q/proteins/peptide",
            tsv.display()
        ),
    };

    let mut peptides: HashMap<String, HashSet<String>> = HashMap::new();
    for line in lines {
        let line = line?;
        let f: Vec<&str> = line.split('\t').collect();
        if f.get(label_i).copied() != Some("1") {
            continue;
        }
        // An unparseable q-value is treated as 1.0 and therefore excluded,
        // matching the Python prototype rather than aborting the run.
        let q: f64 = f.get(q_i).and_then(|s| s.parse().ok()).unwrap_or(1.0);
        if q > q_threshold {
            continue;
        }
        let residues = crate::peak_composition::residues_of(f.get(pep_i).copied().unwrap_or(""));
        if residues.is_empty() {
            continue;
        }
        for acc in f.get(prot_i).copied().unwrap_or("").split(';') {
            let acc = acc.trim();
            if !acc.is_empty() {
                peptides
                    .entry(acc.to_string())
                    .or_default()
                    .insert(residues.clone());
            }
        }
    }

    Ok(peptides
        .into_iter()
        .filter(|(_, peps)| peps.len() >= min_peptides)
        .map(|(acc, _)| acc)
        .collect())
}

/// One protein group: a representative accession, the accessions merged into it,
/// and the peptides it was assigned by the cover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProteinGroup {
    /// The accession used to stand for the group. Any member would do — see
    /// `members` — so the lexicographically smallest is picked to make the
    /// output deterministic rather than dependent on hash order.
    pub representative: String,
    /// Every accession in the group, representative included, sorted. More than
    /// one means the accessions are INDISTINGUISHABLE: identical peptide sets,
    /// so no evidence in this run can separate them.
    pub members: Vec<String>,
    /// How many peptides the cover assigned to this group. Groups are disjoint,
    /// so these sum to the number of covered peptides.
    pub peptide_count: usize,
}

/// Protein parsimony: the smallest set of accessions that explains every
/// confidently identified peptide.
///
/// Pass 2 searches a subset FASTA built from Pass 1's identifications. Without
/// parsimony that subset carries every accession any peptide mapped to, which on
/// liver is 3909 proteins where 1723 explain the same peptides. The excess is not
/// free: NOTES' non-enzymatic RAM table shows a 3909-protein semi-enzymatic index
/// being killed by the OS at 118.6 s while a 921-protein one built fine. A tighter
/// subset is what moves a search from "killed" to "runs".
///
/// **THE ORDER IS 7 -> 5 -> 6, and that is not the order it reads in.** Merging
/// indistinguishable proteins must happen BEFORE the cover, never after:
///
/// 1. **Merge indistinguishable accessions** (identical peptide sets). Exact,
///    deterministic, and it needs no arbitrary pick. On liver this alone takes
///    6261 accessions to 3744 groups.
/// 2. **Greedy set cover.** Repeatedly take the group explaining the most
///    still-unclaimed peptides, assign those peptides to it, and remove them from
///    every other group. A group emptied along the way is SUBSUMED and gets no
///    entry of its own.
/// 3. **Drop groups below `min_peptides`.**
///
/// ⚠ **Doing the merge last, as a final "combine identical peptide sets" step,
/// is a NO-OP and looks like one that worked.** The cover assigns each peptide to
/// exactly one group, so every surviving group's peptide set is DISJOINT from
/// every other. Two disjoint non-empty sets can never be identical, so nothing
/// can ever merge. Measured on liver: 0 merges after the cover, against 2517
/// accessions genuinely merged before it.
///
/// ⚠ **THIS CHANGES THE REPORTED DIGESTION NUMBER.** `min_peptides` applied to
/// post-cover counts is STRICTER than applying it to raw counts: on liver it drops
/// 969 groups covering 969 peptides, 6.5 % of the total. See NOTES "The subset
/// filter is justified by CENSORING" — as the subset shrinks, ragged-N converges
/// on Preview while missed cleavage moves the other way, and no subset size
/// matches Preview on both.
pub fn parsimonious_groups(
    tsv: &Path,
    q_threshold: f64,
    min_peptides: usize,
) -> Result<Vec<ProteinGroup>> {
    let pep_to_prot = peptide_to_proteins(tsv, q_threshold)?;
    Ok(cover(pep_to_prot, min_peptides))
}

/// Confident TARGET peptides mapped to the accessions they came from.
///
/// Peptides are MOD-STRIPPED: `residues_of` drops Sage's inline `[+15.9949]`
/// brackets, so a peptide seen modified and unmodified is one peptide for
/// grouping. The `proteins` string was verified stable across the PSMs of a
/// peptide (0 of 14954 varied on liver), so the first row for a peptide decides.
fn peptide_to_proteins(tsv: &Path, q_threshold: f64) -> Result<HashMap<String, Vec<String>>> {
    let file = std::fs::File::open(tsv)
        .with_context(|| format!("failed to open Sage TSV: {}", tsv.display()))?;
    let mut lines = std::io::BufReader::new(file).lines();
    let header = match lines.next() {
        Some(h) => h?,
        None => anyhow::bail!("empty Sage TSV: {}", tsv.display()),
    };
    let cols: Vec<&str> = header.split('\t').collect();
    let idx = |name: &str| cols.iter().position(|c| *c == name);
    let (label_i, q_i, prot_i, pep_i) = match (
        idx("label"),
        idx("peptide_q"),
        idx("proteins"),
        idx("peptide"),
    ) {
        (Some(a), Some(b), Some(c), Some(d)) => (a, b, c, d),
        _ => anyhow::bail!(
            "Sage TSV {} lacks one of label/peptide_q/proteins/peptide",
            tsv.display()
        ),
    };

    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for line in lines {
        let line = line?;
        let f: Vec<&str> = line.split('\t').collect();
        if f.get(label_i).copied() != Some("1") {
            continue;
        }
        // An unparseable q-value is treated as 1.0 and therefore excluded,
        // matching `target_accessions`.
        let q: f64 = f.get(q_i).and_then(|s| s.parse().ok()).unwrap_or(1.0);
        if q > q_threshold {
            continue;
        }
        let residues = crate::peak_composition::residues_of(f.get(pep_i).copied().unwrap_or(""));
        if residues.is_empty() {
            continue;
        }
        if out.contains_key(&residues) {
            continue;
        }
        let mut accs: Vec<String> = f
            .get(prot_i)
            .copied()
            .unwrap_or("")
            .split(';')
            .map(|a| a.trim())
            .filter(|a| !a.is_empty())
            .map(|a| a.to_string())
            .collect();
        accs.sort();
        accs.dedup();
        if !accs.is_empty() {
            out.insert(residues, accs);
        }
    }
    Ok(out)
}

/// Steps 1-3 of `parsimonious_groups`, on an in-memory map so it is testable
/// without a TSV.
fn cover(pep_to_prot: HashMap<String, Vec<String>>, min_peptides: usize) -> Vec<ProteinGroup> {
    // Invert to protein -> peptide set.
    let mut prot_to_pep: HashMap<&str, HashSet<&str>> = HashMap::new();
    for (pep, prots) in &pep_to_prot {
        for p in prots {
            prot_to_pep
                .entry(p.as_str())
                .or_default()
                .insert(pep.as_str());
        }
    }

    // --- STEP 7 (FIRST): merge indistinguishable accessions ---------------
    // Two accessions with identical peptide sets cannot be told apart by any
    // evidence in this run. Merging them here means the cover never has to pick
    // arbitrarily between them, and the pick it does make is recorded.
    let mut by_set: HashMap<Vec<&str>, Vec<&str>> = HashMap::new();
    for (prot, peps) in &prot_to_pep {
        let mut key: Vec<&str> = peps.iter().copied().collect();
        key.sort_unstable();
        by_set.entry(key).or_default().push(prot);
    }
    // Entity = (sorted members, peptide set). Sorted for determinism: HashMap
    // iteration order is not stable, and the cover's tie-break reads the
    // representative name.
    let mut entities: Vec<(Vec<&str>, HashSet<&str>)> = by_set
        .into_iter()
        .map(|(key, mut members)| {
            members.sort_unstable();
            (members, key.into_iter().collect::<HashSet<&str>>())
        })
        .collect();
    entities.sort_by(|a, b| a.0[0].cmp(b.0[0]));

    // --- STEP 5: greedy set cover -----------------------------------------
    let total_peptides = pep_to_prot.len();
    let mut remaining: Vec<(usize, HashSet<&str>)> = entities
        .iter()
        .enumerate()
        .map(|(i, (_, peps))| (i, peps.clone()))
        .collect();
    let mut groups: Vec<ProteinGroup> = Vec::new();
    let mut covered = 0usize;

    loop {
        // Most unclaimed peptides wins; ties go to the smallest representative
        // so the result does not depend on iteration order.
        let best = remaining
            .iter()
            .enumerate()
            .max_by(|(_, (ia, a)), (_, (ib, b))| {
                a.len()
                    .cmp(&b.len())
                    .then_with(|| entities[*ib].0[0].cmp(entities[*ia].0[0]))
            })
            .map(|(pos, _)| pos);
        let Some(pos) = best else { break };
        if remaining[pos].1.is_empty() {
            break;
        }
        let (ent_i, claimed) = remaining.swap_remove(pos);
        covered += claimed.len();
        groups.push(ProteinGroup {
            representative: entities[ent_i].0[0].to_string(),
            members: entities[ent_i].0.iter().map(|s| s.to_string()).collect(),
            peptide_count: claimed.len(),
        });
        // Remove the claimed peptides everywhere else. A group emptied here is
        // SUBSUMED and never gets an entry.
        remaining.retain_mut(|(_, peps)| {
            peps.retain(|p| !claimed.contains(p));
            !peps.is_empty()
        });
    }

    // THE INVARIANT: the cover is a PARTITION of the confident peptides. Every
    // peptide is claimed, and none is claimed twice. A cover that silently loses
    // peptides would shrink the Pass 2 search space and look like a win.
    // (Found by this assert during prototyping: subtracting the claimed set from
    // the winner ALIASED it to itself, emptying it, so every later subtraction
    // was a no-op and peptides were claimed repeatedly — 22784 claims over 14954
    // peptides, with no other symptom.)
    assert_eq!(
        covered, total_peptides,
        "protein cover is not a partition: {covered} claims over {total_peptides} peptides"
    );

    // --- STEP 6: drop groups below the peptide floor -----------------------
    groups.retain(|g| g.peptide_count >= min_peptides);
    groups.sort_by(|a, b| {
        b.peptide_count
            .cmp(&a.peptide_count)
            .then_with(|| a.representative.cmp(&b.representative))
    });
    groups
}

/// The representative accessions of `parsimonious_groups`, ready for
/// `write_subset_fasta`.
pub fn parsimonious_accessions(
    tsv: &Path,
    q_threshold: f64,
    min_peptides: usize,
) -> Result<HashSet<String>> {
    Ok(parsimonious_groups(tsv, q_threshold, min_peptides)?
        .into_iter()
        .map(|g| g.representative)
        .collect())
}

/// Write a subset FASTA holding only `accessions`, STREAMING the source.
///
/// The source FASTA is the full search database (13.7 MB / ~20k proteins on the
/// test set, and users will bring bigger ones). It is read one record at a time
/// and never collected into a `Vec`, so peak memory is one protein rather than
/// the whole database. `ProteinIndex::from_fasta` deliberately does the
/// opposite because it needs random access; this path does not.
///
/// Sequences are re-wrapped at 60 columns and lines are terminated with `\n`
/// on every platform. Sage generates decoys internally, so only target
/// sequences are written.
///
/// Returns the number of records written.
pub fn write_subset_fasta(
    source: &Path,
    accessions: &HashSet<String>,
    output: &Path,
) -> Result<usize> {
    let infile = std::fs::File::open(source)
        .with_context(|| format!("failed to read FASTA: {}", source.display()))?;
    let outfile = std::fs::File::create(output)
        .with_context(|| format!("failed to create subset FASTA: {}", output.display()))?;
    let reader = std::io::BufReader::new(infile);
    let mut writer = BufWriter::new(outfile);

    let mut written = 0usize;
    let mut keep = false;
    let mut seq = String::new();

    // Flush one buffered record. Wrapping happens here so a record split over
    // any number of source lines is emitted identically.
    fn flush(w: &mut impl Write, keep: bool, seq: &mut String) -> std::io::Result<()> {
        if keep {
            let b = seq.as_bytes();
            let mut i = 0;
            while i < b.len() {
                let end = (i + 60).min(b.len());
                w.write_all(&b[i..end])?;
                w.write_all(b"\n")?;
                i = end;
            }
        }
        seq.clear();
        Ok(())
    }

    for line in reader.lines() {
        let line = line?;
        if let Some(header) = line.strip_prefix('>') {
            flush(&mut writer, keep, &mut seq)?;
            let acc = header.split_whitespace().next().unwrap_or("");
            keep = accessions.contains(acc);
            if keep {
                writer.write_all(b">")?;
                writer.write_all(header.as_bytes())?;
                writer.write_all(b"\n")?;
                written += 1;
            }
        } else if keep {
            seq.push_str(line.trim());
        }
    }
    flush(&mut writer, keep, &mut seq)?;
    writer.flush()?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn fixture() -> (tempfile::TempDir, ProteinIndex) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.fasta");
        let mut f = std::fs::File::create(&path).unwrap();
        // Wrapped lines, a multi-token header, and a lowercase residue, because
        // real FASTA files have all three.
        writeln!(f, ">sp|P00001|AAA_HUMAN Alpha protein OS=Homo sapiens").unwrap();
        writeln!(f, "MASTQK").unwrap();
        writeln!(f, "LLNPR").unwrap();
        writeln!(f, ">sp|P00002|BBB_HUMAN Beta protein").unwrap();
        writeln!(f, "MGGYYR").unwrap();
        drop(f);
        let idx = ProteinIndex::from_fasta(&path).unwrap();
        (dir, idx)
    }

    #[test]
    fn wrapped_sequences_are_joined_and_headers_keyed_by_first_token() {
        let (_d, idx) = fixture();
        assert_eq!(idx.len(), 2);
        // MASTQK + LLNPR must be one sequence, or a peptide spanning the wrap
        // would never match.
        assert!(idx.starts_protein("MASTQKLLNPR", "sp|P00001|AAA_HUMAN"));
    }

    #[test]
    fn position_zero_is_a_prefix_test_not_a_membership_test() {
        let (_d, idx) = fixture();
        assert!(idx.starts_protein("MAST", "sp|P00001|AAA_HUMAN"));
        // Present in the protein, but not at position 0.
        assert!(!idx.starts_protein("LLNPR", "sp|P00001|AAA_HUMAN"));
        // Present at position 0 of a DIFFERENT protein in the list.
        assert!(idx.starts_protein("MGGY", "sp|P00001|AAA_HUMAN;sp|P00002|BBB_HUMAN"));
    }

    #[test]
    fn unknown_accessions_are_false_and_unresolved() {
        let (_d, idx) = fixture();
        assert!(!idx.starts_protein("MAST", "rev_sp|P00001|AAA_HUMAN"));
        assert!(!idx.resolves("rev_sp|P00001|AAA_HUMAN"));
        assert!(idx.resolves("rev_sp|P00001|AAA_HUMAN;sp|P00002|BBB_HUMAN"));
    }

    #[test]
    fn empty_residues_never_match() {
        let (_d, idx) = fixture();
        assert!(!idx.starts_protein("", "sp|P00001|AAA_HUMAN"));
    }
}
