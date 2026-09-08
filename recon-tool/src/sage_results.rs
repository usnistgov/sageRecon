//! Sage results TSV parser with filtering and isotope correction.
//!
//! Parses `results.sage.tsv`, filters by q-value and decoy status,
//! and computes isotope-corrected delta masses.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// The set of TSV columns this parser requires from Sage. The pinned version is
/// `sage_runner::SAGE_VERSION`; read it there rather than trusting this comment.
///
/// Before upgrading the pinned Sage rev, diff this list against the
/// new version's CHANGELOG. In particular:
///
/// ⚠ `precursor_ppm` is SIGNED as of v0.15, which is the pinned version.
/// `fragment_ppm` is still absolute. Measured on the committed v0.15 serum
/// output: 17591 of 68817 precursor values negative, 0 fragment values negative.
/// Do not merge the two conventions.
///
/// ⚠ This list validates 21 columns, but `RawPsmRecord` deserializes 37 as
/// non-Option. `rt` is used (`mzml.rs`) and is NOT validated here, so its
/// removal upstream would raise a bare parse error instead of this schema error.
const REQUIRED_COLUMNS: &[&str] = &[
    "peptide",
    "proteins",
    "filename",
    "scannr",
    "rank",
    "expmass",
    "calcmass",
    "charge",
    "peptide_len",
    "missed_cleavages",
    "semi_enzymatic",
    "isotope_error",
    "precursor_ppm",
    "fragment_ppm",
    "hyperscore",
    "longest_b",
    "longest_y",
    "matched_intensity_pct",
    "ms2_intensity",
    "peptide_q",
    "spectrum_q",
    // Used by mzml.rs for the RT-window index. Added 2026-09-02: it was
    // consumed but never validated, so losing it upstream gave a bare parse
    // error instead of the schema error naming the column and the pin.
    "rt",
];

/// Validate that a Sage TSV has the expected column headers before parsing.
///
/// Fails fast with a clear error naming the missing column and the pinned Sage
/// version, so a binary swap or upstream schema change is immediately visible
/// rather than causing a silent misparse downstream.
pub fn validate_tsv_schema(tsv_path: &Path) -> Result<()> {
    use crate::sage_runner::{SAGE_COMMIT, SAGE_VERSION};

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_path(tsv_path)
        .with_context(|| {
            format!(
                "Failed to open TSV for schema check: {}",
                tsv_path.display()
            )
        })?;

    let headers = reader
        .headers()
        .with_context(|| format!("Failed to read TSV headers: {}", tsv_path.display()))?;

    let header_set: std::collections::HashSet<&str> = headers.iter().collect();

    let missing: Vec<&str> = REQUIRED_COLUMNS
        .iter()
        .copied()
        .filter(|col| !header_set.contains(col))
        .collect();

    if !missing.is_empty() {
        return Err(anyhow::anyhow!(
            "TSV schema mismatch in {}.\n\
             Missing columns: {}\n\
             This recon build expects Sage {} (commit {}).\n\
             If you updated the Sage binary, check the Sage CHANGELOG for \
             breaking column changes, update REQUIRED_COLUMNS and \
             SAGE_VERSION/SAGE_COMMIT in sage_runner.rs, and review \
             qc.rs for precursor_ppm sign semantics.",
            tsv_path.display(),
            missing.join(", "),
            SAGE_VERSION,
            SAGE_COMMIT,
        ));
    }

    Ok(())
}

/// ¹³C − ¹²C mass difference for isotope envelope spacing (Da)
/// This is the correct value for isotope peak spacing in peptide mass spectrometry.
/// Note: The free neutron rest mass (1.0086649158849 Da) is WRONG for this purpose.
pub const C13_C12_DIFF: f64 = 1.003354835;

/// Default q-value threshold for filtering
pub const DEFAULT_Q_THRESHOLD: f64 = 0.01;

/// Default decoy prefix
pub const DEFAULT_DECOY_PREFIX: &str = "rev_";

/// A single Peptide-Spectrum Match from Sage results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Psm {
    /// Spectrum scan number
    pub scannr: u32,
    /// Search engine rank for this PSM within its spectrum (1 = best)
    pub rank: u32,
    /// Peptide sequence (with modifications)
    pub peptide: String,
    /// Protein accessions (semicolon-separated)
    pub proteins: String,
    /// Experimental precursor mass (Da)
    pub expmass: f64,
    /// Calculated peptide mass (Da)
    pub calcmass: f64,
    /// Isotope error (number of neutrons off)
    pub isotope_error: i32,
    /// Raw delta mass: expmass - calcmass
    pub delta_mass: f64,
    /// Isotope-corrected delta mass
    pub delta_mass_corrected: f64,
    /// Sage hyperscore
    pub hyperscore: f64,
    /// PERCENT of MS2 intensity explained by matched ions, 0-100. Sage already
    /// multiplies by 100 (`scoring.rs`), so it must not be scaled again.
    pub matched_intensity_pct: f64,
    /// Longest consecutive b-ion series
    pub longest_b: u32,
    /// Longest consecutive y-ion series
    pub longest_y: u32,
    /// Total MS2 intensity
    pub ms2_intensity: f64,
    /// Peptide-level q-value
    pub peptide_q: f64,
    /// Spectrum-level q-value. Carried alongside `peptide_q` because NOTES
    /// standardises delta-band populations on `spectrum_q < 0.01` (it matches
    /// Sage's headline FDR count). The pipeline's own filter still uses
    /// `peptide_q`, so adding this moves no existing number; `tier_assignment`
    /// reads it for the statistical background.
    pub spectrum_q: f64,
    /// Whether this is a decoy hit
    pub is_decoy: bool,
    /// Charge state
    pub charge: u32,
    /// Retention time (minutes)
    pub rt: f64,
    /// Number of missed cleavages
    pub missed_cleavages: u32,
    /// Whether this is a semi-enzymatic peptide (one non-tryptic terminus)
    pub semi_enzymatic: bool,
    /// Precursor mass error in ppm
    pub precursor_ppm: f64,
    /// Fragment mass error in ppm
    pub fragment_ppm: f64,
    /// Peptide length (number of amino acids)
    pub peptide_len: u32,
}

/// Filtering options for PSM loading
#[derive(Debug, Clone)]
pub struct FilterOptions {
    /// Q-value threshold (default: 0.01)
    pub q_threshold: f64,
    /// Decoy prefix to filter out (default: "rev_")
    pub decoy_prefix: String,
    /// If true, only include PSMs with isotope_error == 0
    pub isotope_error_zero_only: bool,
    /// Keep decoy PSMs instead of dropping them at load (default: FALSE).
    ///
    /// Every existing caller wants decoys gone — a decoy is not an
    /// identification and must never reach a reported count. The ONE caller that
    /// sets this is Pass 2's digestion measurement, which needs the decoys in
    /// order to SUBTRACT them per specificity class, the way Preview does
    /// ("counts the number of hits ... and then corrects for the number of false
    /// hits estimated by the target/decoy approach", Kil et al. 2011).
    ///
    /// ⚠ When this is true, `SageResults::psms` is a MIXED population. Anything
    /// reading it must filter on `Psm::is_decoy` itself. `filter_stats.decoys_removed`
    /// stays 0, because none were.
    pub keep_decoys: bool,
}

impl Default for FilterOptions {
    fn default() -> Self {
        Self {
            q_threshold: DEFAULT_Q_THRESHOLD,
            decoy_prefix: DEFAULT_DECOY_PREFIX.to_string(),
            isotope_error_zero_only: false,
            keep_decoys: false,
        }
    }
}

/// Statistics about filtering applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterStats {
    /// Total PSMs before any filtering
    pub total_before_filter: usize,
    /// PSMs removed as decoys
    pub decoys_removed: usize,
    /// PSMs removed by q-value filter
    pub q_filtered: usize,
    /// PSMs removed by isotope_error filter (if enabled)
    pub isotope_filtered: usize,
    /// PSMs remaining after all filters
    pub total_after_filter: usize,
}

/// Parsed and filtered Sage results
#[derive(Debug)]
pub struct SageResults {
    /// Filtered PSMs
    pub psms: Vec<Psm>,
    /// Filtering statistics
    pub filter_stats: FilterStats,
    /// Isotope error distribution (before filtering)
    pub isotope_error_distribution: HashMap<i32, usize>,
}

impl SageResults {
    /// Get PSMs grouped by scan number (for chimera handling)
    pub fn psms_by_scan(&self) -> HashMap<u32, Vec<&Psm>> {
        let mut by_scan: HashMap<u32, Vec<&Psm>> = HashMap::new();
        for psm in &self.psms {
            by_scan.entry(psm.scannr).or_default().push(psm);
        }
        by_scan
    }

    /// Count of unique scans (for signal fate accounting)
    pub fn unique_scan_count(&self) -> usize {
        self.psms_by_scan().len()
    }

    /// Filter to isotope_error == 0 only (returns new SageResults)
    pub fn filter_isotope_zero(&self) -> SageResults {
        let psms: Vec<Psm> = self
            .psms
            .iter()
            .filter(|p| p.isotope_error == 0)
            .cloned()
            .collect();

        let isotope_filtered = self.psms.len() - psms.len();

        SageResults {
            filter_stats: FilterStats {
                total_before_filter: self.filter_stats.total_before_filter,
                decoys_removed: self.filter_stats.decoys_removed,
                q_filtered: self.filter_stats.q_filtered,
                isotope_filtered: self.filter_stats.isotope_filtered + isotope_filtered,
                total_after_filter: psms.len(),
            },
            psms,
            isotope_error_distribution: self.isotope_error_distribution.clone(),
        }
    }
}

/// Parse Sage results TSV file with filtering
pub fn parse_sage_results(tsv_path: &Path, options: &FilterOptions) -> Result<SageResults> {
    validate_tsv_schema(tsv_path)?;

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_path(tsv_path)
        .with_context(|| format!("Failed to open TSV: {}", tsv_path.display()))?;

    let mut all_psms: Vec<Psm> = Vec::new();
    let mut decoys_removed = 0usize;
    let mut q_filtered = 0usize;
    let mut isotope_filtered = 0usize;
    let mut isotope_distribution: HashMap<i32, usize> = HashMap::new();
    let mut total_rows = 0usize;

    for result in reader.deserialize() {
        let record: RawPsmRecord = result.with_context(|| "Failed to parse TSV row")?;
        total_rows += 1;

        // Track isotope error distribution before filtering
        let isotope_error = record.isotope_error.round() as i32;
        *isotope_distribution.entry(isotope_error).or_insert(0) += 1;

        // Check if decoy
        let is_decoy = record.proteins.starts_with(&options.decoy_prefix);
        if is_decoy && !options.keep_decoys {
            decoys_removed += 1;
            continue;
        }

        // Check q-value. The rule is `q <= threshold`, so the test is `>`, NOT
        // `>=`. It was `>=` here and `>` in `protein_index`, so a PSM sitting
        // exactly on 0.01 was in one population and out of the other. Ben settled
        // it 2026-08-31: q <= 0.01 everywhere.
        if record.peptide_q > options.q_threshold {
            q_filtered += 1;
            continue;
        }

        // Check isotope error if filtering enabled
        if options.isotope_error_zero_only && isotope_error != 0 {
            isotope_filtered += 1;
            continue;
        }

        // Compute delta masses
        let delta_mass = record.expmass - record.calcmass;
        // Use C13_C12_DIFF (1.003355 Da) for isotope correction, NOT the free neutron mass
        let delta_mass_corrected = delta_mass - (isotope_error as f64 * C13_C12_DIFF);

        // Extract scan number from scannr string like "controllerType=0 controllerNumber=1 scan=9681"
        let scannr = extract_scan_number(&record.scannr).unwrap_or(0);

        all_psms.push(Psm {
            scannr,
            rank: record.rank,
            peptide: record.peptide,
            proteins: record.proteins,
            expmass: record.expmass,
            calcmass: record.calcmass,
            isotope_error,
            delta_mass,
            delta_mass_corrected,
            hyperscore: record.hyperscore,
            matched_intensity_pct: record.matched_intensity_pct,
            longest_b: record.longest_b,
            longest_y: record.longest_y,
            ms2_intensity: record.ms2_intensity,
            peptide_q: record.peptide_q,
            spectrum_q: record.spectrum_q,
            is_decoy,
            charge: record.charge,
            rt: record.rt,
            missed_cleavages: record.missed_cleavages,
            semi_enzymatic: record.semi_enzymatic != 0,
            precursor_ppm: record.precursor_ppm,
            fragment_ppm: record.fragment_ppm,
            peptide_len: record.peptide_len,
        });
    }

    log::info!(
        "Parsed {} PSMs: {} decoys removed, {} q-filtered, {} isotope-filtered, {} remaining",
        total_rows,
        decoys_removed,
        q_filtered,
        isotope_filtered,
        all_psms.len()
    );

    Ok(SageResults {
        psms: all_psms,
        filter_stats: FilterStats {
            total_before_filter: total_rows,
            decoys_removed,
            q_filtered,
            isotope_filtered,
            total_after_filter: total_rows - decoys_removed - q_filtered - isotope_filtered,
        },
        isotope_error_distribution: isotope_distribution,
    })
}

/// Extract scan number from Sage's scannr string format
/// e.g., "controllerType=0 controllerNumber=1 scan=9681" -> 9681
fn extract_scan_number(scannr: &str) -> Option<u32> {
    // Look for "scan=" followed by digits
    if let Some(pos) = scannr.find("scan=") {
        let after_scan = &scannr[pos + 5..];
        let num_str: String = after_scan
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        return num_str.parse().ok();
    }
    // Fallback: try parsing the whole string as a number
    scannr.parse().ok()
}

/// Read the DISTINCT values of the `filename` column from a Sage results TSV.
///
/// Sage writes the source raw-file BASENAME (e.g. `b1906_...mzML.gz`) into every
/// row's `filename` column. Used by the `analyze` same-file provenance guard.
/// ⚠ Names are PERCENT-DECODED before they are returned. Sage v0.15 derives this
/// column from `Url::path()`, so `my file.mzML` arrives as `my%20file.mzML`
/// (`sage-cloudpath/src/lib.rs:36-38`). v0.14.7 returned it verbatim. Callers
/// compare against an OS basename, so decoding here keeps that comparison true
/// for any name holding a space or a non-ASCII character.
/// Runs schema validation first — same guard as `parse_sage_results`.
pub fn distinct_filenames(tsv_path: &Path) -> Result<Vec<String>> {
    validate_tsv_schema(tsv_path)?;

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_path(tsv_path)
        .with_context(|| format!("Failed to open TSV: {}", tsv_path.display()))?;
    let headers = reader.headers()?.clone();
    let i_filename = headers
        .iter()
        .position(|h| h == "filename")
        .ok_or_else(|| anyhow::anyhow!("TSV {} has no `filename` column", tsv_path.display()))?;

    let mut seen: Vec<String> = Vec::new();
    for rec in reader.records() {
        let rec = rec?;
        if let Some(f) = rec.get(i_filename) {
            let f = percent_encoding::percent_decode_str(f)
                .decode_utf8()
                .map(|d| d.to_string())
                .unwrap_or_else(|_| f.to_string());
            if !seen.iter().any(|s| s == &f) {
                seen.push(f);
            }
        }
    }
    Ok(seen)
}

/// Raw record structure matching Sage TSV columns
/// Note: Many fields are parsed but not used yet — they're available for future phases
/// Fields marked with Option<> may not be present in all Sage versions
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawPsmRecord {
    psm_id: u64,
    peptide: String,
    proteins: String,
    num_proteins: u32,
    filename: String,
    scannr: String, // Contains "controllerType=0 controllerNumber=1 scan=XXXX"
    rank: u32,
    label: i32,
    expmass: f64,
    calcmass: f64,
    charge: u32,
    peptide_len: u32,
    missed_cleavages: u32,
    semi_enzymatic: u32, // 0 or 1, not bool
    isotope_error: f64,  // Sage outputs as float
    precursor_ppm: f64,
    fragment_ppm: f64,
    hyperscore: f64,
    delta_next: f64,
    delta_best: f64,
    rt: f64,
    aligned_rt: f64,
    predicted_rt: f64,
    delta_rt_model: f64,
    // Ion mobility fields - optional, not present in all Sage versions
    #[serde(default)]
    ion_mobility: Option<f64>,
    #[serde(default)]
    predicted_mobility: Option<f64>,
    #[serde(default)]
    delta_mobility: Option<f64>,
    matched_peaks: u32,
    longest_b: u32,
    longest_y: u32,
    longest_y_pct: f64,
    matched_intensity_pct: f64,
    scored_candidates: u64,
    poisson: f64,
    sage_discriminant_score: f64,
    posterior_error: f64,
    spectrum_q: f64,
    peptide_q: f64,
    protein_q: f64,
    ms2_intensity: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isotope_correction_with_c13_c12_diff() {
        // If expmass = 1001.0, calcmass = 1000.0, isotope_error = 1
        // delta_mass = 1.0
        // corrected = 1.0 - (1 * 1.003354835) = -0.003354835
        // This is the CORRECT behavior using ¹³C−¹²C spacing
        let delta = 1.0;
        let corrected = delta - (1.0 * C13_C12_DIFF);
        assert!((corrected - (-0.003354835)).abs() < 1e-9);
    }

    #[test]
    fn test_c13_c12_diff_constant() {
        // Verify the constant is the correct ¹³C−¹²C mass difference
        // ¹³C = 13.003354835 Da, ¹²C = 12.000000000 Da
        assert!((C13_C12_DIFF - 1.003354835).abs() < 1e-9);
    }

    #[test]
    fn test_filter_options_default() {
        let opts = FilterOptions::default();
        assert_eq!(opts.q_threshold, 0.01);
        assert_eq!(opts.decoy_prefix, "rev_");
        assert!(!opts.isotope_error_zero_only);
    }

    #[test]
    fn test_validate_tsv_schema_missing_column() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        // Write a TSV with a deliberately missing required column (ms2_intensity)
        writeln!(f, "peptide\tproteins\tfilename\tscannr\trank\texpmass\tcalcmass\tcharge\tpeptide_len\tmissed_cleavages\tsemi_enzymatic\tisotope_error\tprecursor_ppm\tfragment_ppm\thyperscore\tlongest_b\tlongest_y\tmatched_intensity_pct\tpeptide_q").unwrap();
        let result = validate_tsv_schema(f.path());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("ms2_intensity"),
            "error should name missing column"
        );
        assert!(
            // Assert the VALUE, not the name. The literal SAGE_VERSION
            // appears in the error help sentence, so the old check passed
            // whatever the pin was, and its 0.14.7 arm went stale.
            msg.contains(crate::sage_runner::SAGE_VERSION),
            "error should cite the pinned version {}, got: {msg}",
            crate::sage_runner::SAGE_VERSION
        );
    }

    #[test]
    fn test_validate_tsv_schema_all_present() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        let header = REQUIRED_COLUMNS.join("\t");
        writeln!(f, "{header}").unwrap();
        let result = validate_tsv_schema(f.path());
        assert!(result.is_ok(), "all required columns present should pass");
    }

    #[test]
    fn test_validate_tsv_schema_extra_columns_ok() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        // Extra columns (e.g. new Sage additions) should not cause failure
        let mut cols: Vec<&str> = REQUIRED_COLUMNS.to_vec();
        cols.push("protein_groups");
        cols.push("some_future_column");
        writeln!(f, "{}", cols.join("\t")).unwrap();
        let result = validate_tsv_schema(f.path());
        assert!(result.is_ok(), "extra columns should be tolerated");
    }
    /// Sage v0.15 percent-encodes the `filename` column, because it derives it
    /// from `Url::path()`. recon compares it against an OS basename, so a name
    /// holding a space stopped matching and `analyze` aborted on the provenance
    /// guard. Measured: "my file #1.mzML.gz" arrives as "my%20file%20%231.mzML.gz".
    ///
    /// A real file is written here rather than a synthetic string, so the test
    /// exercises the actual CSV read path.
    #[test]
    fn distinct_filenames_decodes_percent_encoding() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("t.tsv");
        let mut f = std::fs::File::create(&p).unwrap();
        let cols: Vec<&str> = REQUIRED_COLUMNS.to_vec();
        writeln!(f, "{}", cols.join("\t")).unwrap();
        for enc in ["my%20file%20%231.mzML.gz", "Gr%C3%BC%C3%9Fe.mzML"] {
            let row: Vec<String> = cols
                .iter()
                .map(|c| {
                    if *c == "filename" {
                        enc.to_string()
                    } else {
                        "0".to_string()
                    }
                })
                .collect();
            writeln!(f, "{}", row.join("\t")).unwrap();
        }
        drop(f);

        let got = distinct_filenames(&p).unwrap();
        assert!(
            got.contains(&"my file #1.mzML.gz".to_string()),
            "space and # must decode, got {got:?}"
        );
        assert!(
            got.contains(&"Grüße.mzML".to_string()),
            "non-ASCII must decode, got {got:?}"
        );
    }
}
