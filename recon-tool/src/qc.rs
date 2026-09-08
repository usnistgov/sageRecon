//! Minimal QC metrics from Sage PSM results.
//!
//! This module computes:
//! - Precursor mass accuracy (ppm) statistics
//! - Fragment mass accuracy (ppm) statistics
//! - Identification rate (from signal_fate if mzML provided)
//!
//! Per PLAN.md non-goals: This is minimal QC, not a general QC suite.
//! All metrics are computed from existing Sage TSV columns — no new computation.

use crate::sage_results::SageResults;
use crate::signal_fate::SignalFateResult;
use serde::{Deserialize, Serialize};

/// Mass accuracy statistics (in ppm)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassAccuracyStats {
    /// Mean mass error (ppm)
    pub mean: f64,
    /// Median mass error (ppm)
    pub median: f64,
    /// Standard deviation (ppm)
    pub std: f64,
    /// 5th percentile (ppm)
    pub percentile_5: f64,
    /// 95th percentile (ppm)
    pub percentile_95: f64,
    /// Minimum (ppm)
    pub min: f64,
    /// Maximum (ppm)
    pub max: f64,
}

/// Identification rate statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentificationRateStats {
    /// Percentage of MS2 spectra with passing PSM (requires mzML)
    pub psm_rate_pct: Option<f64>,
    /// Total identified spectra (unique scans)
    pub identified_spectra: usize,
    /// Total MS2 spectra (from mzML, if provided)
    pub total_ms2_spectra: Option<usize>,
    /// Unique peptides identified
    pub unique_peptides: usize,
    /// Unique proteins identified
    pub unique_proteins: usize,
}

/// Complete QC result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QcResult {
    /// Total PSMs analyzed
    pub total_psms: usize,
    /// Precursor mass accuracy statistics, computed over Sage's raw
    /// `precursor_ppm` column. ⚠ **SIGNED as of Sage v0.15.** This comment used
    /// to say ABSOLUTE, describing v0.14.x, and was not updated when the pin
    /// moved — the exact miss the upgrade checklist's step 4 exists to prevent.
    /// Measured 2026-09-02 on the committed v0.15.0-beta.2 serum output: 17591
    /// of 68817 rows negative (25.6%), min -121568.77. So `median` here is a
    /// signed median and CAN be negative.
    /// ⚠ It is still NOT a usable MS1 accuracy in an open search: the precursor
    /// delta carries the MODIFICATION mass, so this summary spans the whole open
    /// window. **For the MS1 bias use `calibration::compute_ms1_stats`,
    /// which measures it from the OPEN search's own near-zero clean subset.**
    /// recon does not run, need, or accept a closed reference search: the
    /// closed-TSV path and its `--closed-tsv` flag were retired 2026-09-01.
    pub precursor_ppm: MassAccuracyStats,
    /// Fragment mass accuracy statistics. ⚠ ABSOLUTE — `fragment_ppm` stayed
    /// absolute in v0.15 while `precursor_ppm` became signed. Do not merge them.
    /// There is no signed MS2 bias available from the default Sage TSV.
    pub fragment_ppm: MassAccuracyStats,
    /// Identification rate statistics
    pub identification_rate: IdentificationRateStats,
}

/// Compute QC metrics from Sage results.
///
/// # Arguments
/// * `results` - Parsed and filtered Sage results
/// * `signal_fate` - Optional signal fate result (for ID rate from mzML)
///
/// # Returns
/// QC statistics
pub fn compute_qc_stats(results: &SageResults, signal_fate: Option<&SignalFateResult>) -> QcResult {
    let total_psms = results.psms.len();

    if total_psms == 0 {
        return QcResult {
            total_psms: 0,
            precursor_ppm: MassAccuracyStats {
                mean: 0.0,
                median: 0.0,
                std: 0.0,
                percentile_5: 0.0,
                percentile_95: 0.0,
                min: 0.0,
                max: 0.0,
            },
            fragment_ppm: MassAccuracyStats {
                mean: 0.0,
                median: 0.0,
                std: 0.0,
                percentile_5: 0.0,
                percentile_95: 0.0,
                min: 0.0,
                max: 0.0,
            },
            identification_rate: IdentificationRateStats {
                psm_rate_pct: None,
                identified_spectra: 0,
                total_ms2_spectra: None,
                unique_peptides: 0,
                unique_proteins: 0,
            },
        };
    }

    // Compute precursor ppm statistics
    let precursor_ppms: Vec<f64> = results.psms.iter().map(|p| p.precursor_ppm).collect();
    let precursor_ppm = compute_ppm_stats(&precursor_ppms);

    // Compute fragment ppm statistics
    let fragment_ppms: Vec<f64> = results.psms.iter().map(|p| p.fragment_ppm).collect();
    let fragment_ppm = compute_ppm_stats(&fragment_ppms);

    // Compute identification rate
    let identified_spectra = results.unique_scan_count();

    // Get ID rate from signal_fate if available (computed from mzML)
    let (psm_rate_pct, total_ms2_spectra) = if let Some(fate) = signal_fate {
        if let Some(ref unid) = fate.unidentified {
            (
                Some(fate.by_count.identified_pct.unwrap_or(0.0)),
                Some(unid.total_ms2_spectra),
            )
        } else {
            (fate.by_count.identified_pct, None)
        }
    } else {
        (None, None)
    };

    // Count unique peptides and proteins
    let mut unique_peptides = std::collections::HashSet::new();
    let mut unique_proteins = std::collections::HashSet::new();

    for psm in &results.psms {
        unique_peptides.insert(&psm.peptide);
        // Proteins can be semicolon-separated, split them
        for protein in psm.proteins.split(';') {
            unique_proteins.insert(protein.to_string());
        }
    }

    let identification_rate = IdentificationRateStats {
        psm_rate_pct,
        identified_spectra,
        total_ms2_spectra,
        unique_peptides: unique_peptides.len(),
        unique_proteins: unique_proteins.len(),
    };

    QcResult {
        total_psms,
        precursor_ppm,
        fragment_ppm,
        identification_rate,
    }
}

/// Compute statistics for a vector of ppm values
fn compute_ppm_stats(values: &[f64]) -> MassAccuracyStats {
    if values.is_empty() {
        return MassAccuracyStats {
            mean: 0.0,
            median: 0.0,
            std: 0.0,
            percentile_5: 0.0,
            percentile_95: 0.0,
            min: 0.0,
            max: 0.0,
        };
    }

    let n = values.len() as f64;

    // Mean
    let sum: f64 = values.iter().sum();
    let mean = sum / n;

    // Standard deviation
    let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let std = variance.sqrt();

    // Sort for percentiles
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let min = sorted[0];
    let max = sorted[sorted.len() - 1];
    let median = sorted[sorted.len() / 2];

    // Percentiles
    let p5_idx = ((sorted.len() as f64) * 0.05) as usize;
    let p95_idx = ((sorted.len() as f64) * 0.95) as usize;
    let percentile_5 = sorted[p5_idx.min(sorted.len() - 1)];
    let percentile_95 = sorted[p95_idx.min(sorted.len() - 1)];

    MassAccuracyStats {
        mean,
        median,
        std,
        percentile_5,
        percentile_95,
        min,
        max,
    }
}

/// Print a summary of QC statistics
pub fn print_qc_summary(result: &QcResult) {
    println!("=== QC Statistics ===");
    println!();
    println!("Total PSMs: {}", result.total_psms);
    println!();

    println!("Precursor Mass Accuracy (ppm):");
    println!("  Mean: {:.2}", result.precursor_ppm.mean);
    println!("  Median: {:.2}", result.precursor_ppm.median);
    println!("  Std: {:.2}", result.precursor_ppm.std);
    println!("  5th percentile: {:.2}", result.precursor_ppm.percentile_5);
    println!(
        "  95th percentile: {:.2}",
        result.precursor_ppm.percentile_95
    );
    println!(
        "  Range: [{:.2}, {:.2}]",
        result.precursor_ppm.min, result.precursor_ppm.max
    );
    println!();

    println!("Fragment Mass Accuracy (ppm):");
    println!("  Mean: {:.2}", result.fragment_ppm.mean);
    println!("  Median: {:.2}", result.fragment_ppm.median);
    println!("  Std: {:.2}", result.fragment_ppm.std);
    println!("  5th percentile: {:.2}", result.fragment_ppm.percentile_5);
    println!(
        "  95th percentile: {:.2}",
        result.fragment_ppm.percentile_95
    );
    println!(
        "  Range: [{:.2}, {:.2}]",
        result.fragment_ppm.min, result.fragment_ppm.max
    );
    println!();

    println!("Identification Rate:");
    if let Some(rate) = result.identification_rate.psm_rate_pct {
        println!("  PSM rate: {:.1}%", rate);
    }
    println!(
        "  Identified spectra: {}",
        result.identification_rate.identified_spectra
    );
    if let Some(total) = result.identification_rate.total_ms2_spectra {
        println!("  Total MS2 spectra: {}", total);
    }
    println!(
        "  Unique peptides: {}",
        result.identification_rate.unique_peptides
    );
    println!(
        "  Unique proteins: {}",
        result.identification_rate.unique_proteins
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sage_results::{FilterStats, Psm, SageResults};
    use std::collections::HashMap;

    fn make_test_psm(precursor_ppm: f64, fragment_ppm: f64) -> Psm {
        Psm {
            scannr: 1,
            rank: 1,
            peptide: "PEPTIDE".to_string(),
            proteins: "PROTEIN".to_string(),
            expmass: 1000.0,
            calcmass: 1000.0,
            isotope_error: 0,
            delta_mass: 0.0,
            delta_mass_corrected: 0.0,
            hyperscore: 40.0,
            matched_intensity_pct: 0.5,
            longest_b: 5,
            longest_y: 6,
            ms2_intensity: 1000.0,
            peptide_q: 0.001,
            spectrum_q: 0.001,
            is_decoy: false,
            charge: 2,
            rt: 30.0,
            missed_cleavages: 0,
            semi_enzymatic: false,
            precursor_ppm,
            fragment_ppm,
            peptide_len: 7,
        }
    }

    #[test]
    fn test_ppm_stats() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = compute_ppm_stats(&values);

        assert!((stats.mean - 3.0).abs() < 0.01);
        assert!((stats.median - 3.0).abs() < 0.01);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
    }

    #[test]
    fn test_qc_stats() {
        let psms = vec![
            make_test_psm(1.0, 5.0),
            make_test_psm(2.0, 6.0),
            make_test_psm(3.0, 7.0),
        ];

        let results = SageResults {
            psms,
            filter_stats: FilterStats {
                total_before_filter: 3,
                decoys_removed: 0,
                q_filtered: 0,
                isotope_filtered: 0,
                total_after_filter: 3,
            },
            isotope_error_distribution: HashMap::new(),
        };

        let qc = compute_qc_stats(&results, None);

        assert_eq!(qc.total_psms, 3);
        assert!((qc.precursor_ppm.mean - 2.0).abs() < 0.01);
        assert!((qc.fragment_ppm.mean - 6.0).abs() < 0.01);
        assert_eq!(qc.identification_rate.unique_peptides, 1);
        assert_eq!(qc.identification_rate.unique_proteins, 1);
    }

    #[test]
    fn test_unique_counts() {
        let mut psms = vec![make_test_psm(1.0, 5.0), make_test_psm(2.0, 6.0)];
        psms[0].peptide = "PEPTIDEA".to_string();
        psms[0].proteins = "PROT1;PROT2".to_string();
        psms[1].peptide = "PEPTIDEB".to_string();
        psms[1].proteins = "PROT2;PROT3".to_string();

        let results = SageResults {
            psms,
            filter_stats: FilterStats {
                total_before_filter: 2,
                decoys_removed: 0,
                q_filtered: 0,
                isotope_filtered: 0,
                total_after_filter: 2,
            },
            isotope_error_distribution: HashMap::new(),
        };

        let qc = compute_qc_stats(&results, None);

        assert_eq!(qc.identification_rate.unique_peptides, 2);
        assert_eq!(qc.identification_rate.unique_proteins, 3); // PROT1, PROT2, PROT3
    }
}
