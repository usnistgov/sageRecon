//! Signal fate accounting: where is the signal going?
//!
//! This module computes explained vs. unexplained signal by both count AND intensity.
//! Key insight: for chimeric scans (multiple PSMs per scan), we count the scan once
//! but use the scan's total MS2 intensity once (not summed across PSMs).
//!
//! Phase 5 addition: When mzML stats are provided, computes unidentified signal
//! (total MS2 - identified MS2) for complete signal fate accounting.

use crate::mod_discovery::{ModDiscoveryResult, NEAR_ZERO_THRESHOLD_DA};
use crate::mzml::MzmlStats;
use crate::sage_results::{FilterStats, Psm, SageResults};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Signal accounting by spectral count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalByCount {
    /// Number of identified spectra (unique scans)
    pub identified_spectra: usize,
    /// Total PSMs (may be > spectra due to chimeric scans)
    pub total_psms: usize,
    /// Percentage of spectra identified (if total known)
    pub identified_pct: Option<f64>,
}

/// Signal accounting by intensity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalByIntensity {
    /// Total MS2 intensity of identified spectra
    pub identified_intensity: f64,
    /// Percentage of intensity identified (if total known)
    pub identified_pct: Option<f64>,
}

/// Breakdown of identified signal by modification status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifiedBreakdown {
    /// By spectral count
    pub by_count: BreakdownByCount,
    /// By intensity
    pub by_intensity: BreakdownByIntensity,
}

/// Breakdown by count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakdownByCount {
    /// Unmodified spectra (delta ~0)
    pub unmodified: CountWithPct,
    /// Modified spectra with Unimod annotation
    pub modified_annotated: CountWithPct,
    /// Modified spectra without Unimod annotation
    pub modified_unannotated: CountWithPct,
}

/// Breakdown by intensity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakdownByIntensity {
    /// Unmodified intensity
    pub unmodified: IntensityWithPct,
    /// Modified annotated intensity
    pub modified_annotated: IntensityWithPct,
    /// Modified unannotated intensity
    pub modified_unannotated: IntensityWithPct,
}

/// Count with percentage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountWithPct {
    pub count: usize,
    pub pct: f64,
}

/// Intensity with percentage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntensityWithPct {
    pub intensity: f64,
    pub pct: f64,
}

/// Statistics about chimeric spectra
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChimeraStats {
    /// Total unique scans (spectra)
    pub unique_scans: usize,
    /// Scans with multiple PSMs (chimeric)
    pub scans_with_multiple_psms: usize,
    /// Percentage of scans that are chimeric
    pub chimera_rate_pct: f64,
    /// Total PSMs across all scans
    pub total_psms: usize,
    /// Average PSMs per chimeric scan
    pub avg_psms_per_chimeric_scan: f64,
}

/// Unidentified signal statistics (requires mzML parsing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnidentifiedSignal {
    /// Number of unidentified MS2 spectra
    pub unidentified_spectra: usize,
    /// Percentage of MS2 spectra that are unidentified
    pub unidentified_pct: f64,
    /// Total MS2 intensity that is unidentified
    pub unidentified_intensity: f64,
    /// Percentage of MS2 intensity that is unidentified
    pub unidentified_intensity_pct: f64,
    /// Total MS2 spectra in mzML file
    pub total_ms2_spectra: usize,
    /// Total MS2 TIC in mzML file
    pub total_ms2_tic: f64,
}

/// MS1 signal fate statistics (requires mzML MS1 extraction)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ms1SignalFate {
    /// Total MS1 TIC across all MS1 spectra
    pub total_ms1_tic: f64,
    /// MS1 intensity explained by identified PSMs (precursor intensities)
    pub explained_intensity: f64,
    /// Percentage of MS1 TIC explained by identified PSMs
    pub explained_pct: f64,
    /// MS1 intensity not explained by identified PSMs
    pub unexplained_intensity: f64,
    /// Percentage of MS1 TIC not explained
    pub unexplained_pct: f64,
    /// Number of PSMs with MS1 intensity found
    pub psms_with_ms1: usize,
    /// Number of PSMs without MS1 intensity (no peak found)
    pub psms_without_ms1: usize,
}

/// Complete signal fate result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalFateResult {
    /// Signal by count
    pub by_count: SignalByCount,
    /// Signal by intensity
    pub by_intensity: SignalByIntensity,
    /// Breakdown of identified signal
    pub identified_breakdown: IdentifiedBreakdown,
    /// Chimera statistics
    pub chimera_stats: ChimeraStats,
    /// Filtering statistics (from Sage results)
    pub filtering: FilterStats,
    /// Unidentified signal (only present when mzML stats provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unidentified: Option<UnidentifiedSignal>,
    /// MS1 signal fate (only present when --ms1-intensity flag used)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ms1_signal_fate: Option<Ms1SignalFate>,
}

/// Classification of a scan's modification status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModificationStatus {
    Unmodified,
    ModifiedAnnotated,
    ModifiedUnannotated,
}

/// Compute signal fate accounting from Sage results
///
/// # Arguments
/// * `results` - Parsed and filtered Sage results
/// * `mod_discovery` - Optional mod discovery result for annotation status
/// * `annotated_masses` - Set of delta masses that have Unimod annotations (from mod_discovery)
///
/// # Returns
/// Signal fate accounting result
pub fn compute_signal_fate(
    results: &SageResults,
    mod_discovery: Option<&ModDiscoveryResult>,
) -> SignalFateResult {
    // Build set of annotated delta masses from mod_discovery
    let annotated_masses: HashMap<i64, bool> = mod_discovery
        .map(|md| {
            md.peaks
                .iter()
                .filter(|p| !p.unannotated)
                .map(|p| {
                    // Bin to 0.01 Da for lookup
                    let bin = (p.delta_mass * 100.0).round() as i64;
                    (bin, true)
                })
                .collect()
        })
        .unwrap_or_default();

    // Group PSMs by scan for chimera handling
    let by_scan = results.psms_by_scan();

    // Compute chimera stats
    let unique_scans = by_scan.len();
    let chimeric_scans: Vec<_> = by_scan.values().filter(|v| v.len() > 1).collect();
    let scans_with_multiple_psms = chimeric_scans.len();
    let total_psms = results.psms.len();

    let avg_psms_per_chimeric_scan = if scans_with_multiple_psms > 0 {
        chimeric_scans.iter().map(|v| v.len()).sum::<usize>() as f64
            / scans_with_multiple_psms as f64
    } else {
        0.0
    };

    let chimera_stats = ChimeraStats {
        unique_scans,
        scans_with_multiple_psms,
        chimera_rate_pct: if unique_scans > 0 {
            100.0 * (scans_with_multiple_psms as f64) / (unique_scans as f64)
        } else {
            0.0
        },
        total_psms,
        avg_psms_per_chimeric_scan,
    };

    // Compute signal by count and intensity
    // For each scan, use the first PSM's intensity (all PSMs from same scan share the spectrum)
    let mut total_identified_intensity = 0.0;
    let mut unmodified_count = 0usize;
    let mut modified_annotated_count = 0usize;
    let mut modified_unannotated_count = 0usize;
    let mut unmodified_intensity = 0.0;
    let mut modified_annotated_intensity = 0.0;
    let mut modified_unannotated_intensity = 0.0;

    for psms in by_scan.values() {
        // Use first PSM's intensity for the scan
        let scan_intensity = psms[0].ms2_intensity;
        total_identified_intensity += scan_intensity;

        // Classify the scan by its "best" PSM (first one, which is typically highest scoring)
        let status = classify_scan(psms, &annotated_masses);

        match status {
            ModificationStatus::Unmodified => {
                unmodified_count += 1;
                unmodified_intensity += scan_intensity;
            }
            ModificationStatus::ModifiedAnnotated => {
                modified_annotated_count += 1;
                modified_annotated_intensity += scan_intensity;
            }
            ModificationStatus::ModifiedUnannotated => {
                modified_unannotated_count += 1;
                modified_unannotated_intensity += scan_intensity;
            }
        }
    }

    // Build breakdown
    let total_count = unique_scans as f64;
    let total_intensity = total_identified_intensity;

    let breakdown_by_count = BreakdownByCount {
        unmodified: CountWithPct {
            count: unmodified_count,
            pct: if total_count > 0.0 {
                100.0 * (unmodified_count as f64) / total_count
            } else {
                0.0
            },
        },
        modified_annotated: CountWithPct {
            count: modified_annotated_count,
            pct: if total_count > 0.0 {
                100.0 * (modified_annotated_count as f64) / total_count
            } else {
                0.0
            },
        },
        modified_unannotated: CountWithPct {
            count: modified_unannotated_count,
            pct: if total_count > 0.0 {
                100.0 * (modified_unannotated_count as f64) / total_count
            } else {
                0.0
            },
        },
    };

    let breakdown_by_intensity = BreakdownByIntensity {
        unmodified: IntensityWithPct {
            intensity: unmodified_intensity,
            pct: if total_intensity > 0.0 {
                100.0 * unmodified_intensity / total_intensity
            } else {
                0.0
            },
        },
        modified_annotated: IntensityWithPct {
            intensity: modified_annotated_intensity,
            pct: if total_intensity > 0.0 {
                100.0 * modified_annotated_intensity / total_intensity
            } else {
                0.0
            },
        },
        modified_unannotated: IntensityWithPct {
            intensity: modified_unannotated_intensity,
            pct: if total_intensity > 0.0 {
                100.0 * modified_unannotated_intensity / total_intensity
            } else {
                0.0
            },
        },
    };

    SignalFateResult {
        by_count: SignalByCount {
            identified_spectra: unique_scans,
            total_psms,
            identified_pct: None, // Will be set by compute_signal_fate_with_mzml
        },
        by_intensity: SignalByIntensity {
            identified_intensity: total_identified_intensity,
            identified_pct: None, // Will be set by compute_signal_fate_with_mzml
        },
        identified_breakdown: IdentifiedBreakdown {
            by_count: breakdown_by_count,
            by_intensity: breakdown_by_intensity,
        },
        chimera_stats,
        filtering: results.filter_stats.clone(),
        unidentified: None,
        ms1_signal_fate: None,
    }
}

/// Compute signal fate accounting with mzML statistics for complete accounting.
///
/// This function extends `compute_signal_fate` by adding unidentified signal
/// statistics from the mzML file.
///
/// # Arguments
/// * `results` - Parsed and filtered Sage results
/// * `mod_discovery` - Optional mod discovery result for annotation status
/// * `mzml_stats` - Statistics from mzML file (total MS2 count and TIC)
///
/// # Returns
/// Signal fate accounting result with unidentified signal
pub fn compute_signal_fate_with_mzml(
    results: &SageResults,
    mod_discovery: Option<&ModDiscoveryResult>,
    mzml_stats: &MzmlStats,
) -> SignalFateResult {
    // First compute the base signal fate
    let mut fate = compute_signal_fate(results, mod_discovery);

    // Now add the unidentified signal statistics
    let total_ms2 = mzml_stats.ms2_spectra;
    let identified_spectra = fate.by_count.identified_spectra;

    // Compute unidentified spectra (can't be negative)
    let unidentified_spectra = total_ms2.saturating_sub(identified_spectra);

    // Compute percentages
    let identified_pct = if total_ms2 > 0 {
        100.0 * (identified_spectra as f64) / (total_ms2 as f64)
    } else {
        0.0
    };

    let unidentified_pct = if total_ms2 > 0 {
        100.0 * (unidentified_spectra as f64) / (total_ms2 as f64)
    } else {
        0.0
    };

    // Compute intensity-based unidentified signal
    let total_ms2_tic = mzml_stats.total_ms2_tic;
    let identified_intensity = fate.by_intensity.identified_intensity;

    // Unidentified intensity (can't be negative - use 0 if identified > total)
    let unidentified_intensity = if total_ms2_tic > identified_intensity {
        total_ms2_tic - identified_intensity
    } else {
        0.0
    };

    let identified_intensity_pct = if total_ms2_tic > 0.0 {
        100.0 * identified_intensity / total_ms2_tic
    } else {
        0.0
    };

    let unidentified_intensity_pct = if total_ms2_tic > 0.0 {
        100.0 * unidentified_intensity / total_ms2_tic
    } else {
        0.0
    };

    // Update the identified percentages
    fate.by_count.identified_pct = Some(identified_pct);
    fate.by_intensity.identified_pct = Some(identified_intensity_pct);

    // Add unidentified signal
    fate.unidentified = Some(UnidentifiedSignal {
        unidentified_spectra,
        unidentified_pct,
        unidentified_intensity,
        unidentified_intensity_pct,
        total_ms2_spectra: total_ms2,
        total_ms2_tic,
    });

    fate
}

/// Classify a scan's modification status based on its PSMs
///
/// For chimeric scans with multiple PSMs, we use the "dominant" status:
/// - If any PSM is unmodified, the scan is unmodified (most signal explained by unmodified peptide)
/// - Otherwise, if any PSM is modified+annotated, use that
/// - Otherwise, modified+unannotated
fn classify_scan(psms: &[&Psm], annotated_masses: &HashMap<i64, bool>) -> ModificationStatus {
    let mut has_unmodified = false;
    let mut has_annotated = false;

    for psm in psms {
        let delta = psm.delta_mass_corrected;

        if delta.abs() < NEAR_ZERO_THRESHOLD_DA {
            has_unmodified = true;
        } else {
            // Check if this delta mass is annotated
            let bin = (delta * 100.0).round() as i64;
            if annotated_masses.contains_key(&bin) {
                has_annotated = true;
            }
        }
    }

    // Priority: unmodified > annotated > unannotated
    if has_unmodified {
        ModificationStatus::Unmodified
    } else if has_annotated {
        ModificationStatus::ModifiedAnnotated
    } else {
        ModificationStatus::ModifiedUnannotated
    }
}

/// Print a summary of signal fate results
pub fn print_signal_fate_summary(result: &SignalFateResult) {
    println!("=== Signal Fate Accounting ===");
    println!();

    // If we have mzML stats, show the complete picture
    if let Some(ref unid) = result.unidentified {
        println!("Total MS2 Spectra (from mzML): {}", unid.total_ms2_spectra);
        println!("Total MS2 TIC (from mzML): {:.2e}", unid.total_ms2_tic);
        println!();

        println!("Signal Fate (by count):");
        println!(
            "  Identified: {} ({:.1}%)",
            result.by_count.identified_spectra,
            result.by_count.identified_pct.unwrap_or(0.0)
        );
        println!(
            "  Unidentified: {} ({:.1}%)",
            unid.unidentified_spectra, unid.unidentified_pct
        );
        println!();

        println!("Signal Fate (by intensity):");
        println!(
            "  Identified: {:.2e} ({:.1}%)",
            result.by_intensity.identified_intensity,
            result.by_intensity.identified_pct.unwrap_or(0.0)
        );
        println!(
            "  Unidentified: {:.2e} ({:.1}%)",
            unid.unidentified_intensity, unid.unidentified_intensity_pct
        );
        println!();
    } else {
        println!("Identified Signal:");
        println!(
            "  Spectra (unique scans): {}",
            result.by_count.identified_spectra
        );
        println!("  Total PSMs: {}", result.by_count.total_psms);
        println!(
            "  Total MS2 intensity: {:.2e}",
            result.by_intensity.identified_intensity
        );
        println!();
        println!("  (Provide --mzml to compute unidentified signal)");
        println!();
    }

    println!("Chimera Statistics:");
    println!("  Unique scans: {}", result.chimera_stats.unique_scans);
    println!(
        "  Chimeric scans: {} ({:.1}%)",
        result.chimera_stats.scans_with_multiple_psms, result.chimera_stats.chimera_rate_pct
    );
    println!(
        "  Avg PSMs per chimeric scan: {:.2}",
        result.chimera_stats.avg_psms_per_chimeric_scan
    );
    println!();

    println!("Identified Breakdown (by count):");
    println!(
        "  Unmodified: {} ({:.1}%)",
        result.identified_breakdown.by_count.unmodified.count,
        result.identified_breakdown.by_count.unmodified.pct
    );
    println!(
        "  Modified (annotated): {} ({:.1}%)",
        result
            .identified_breakdown
            .by_count
            .modified_annotated
            .count,
        result.identified_breakdown.by_count.modified_annotated.pct
    );
    println!(
        "  Modified (unannotated): {} ({:.1}%)",
        result
            .identified_breakdown
            .by_count
            .modified_unannotated
            .count,
        result
            .identified_breakdown
            .by_count
            .modified_unannotated
            .pct
    );
    println!();

    println!("Identified Breakdown (by intensity):");
    println!(
        "  Unmodified: {:.2e} ({:.1}%)",
        result
            .identified_breakdown
            .by_intensity
            .unmodified
            .intensity,
        result.identified_breakdown.by_intensity.unmodified.pct
    );
    println!(
        "  Modified (annotated): {:.2e} ({:.1}%)",
        result
            .identified_breakdown
            .by_intensity
            .modified_annotated
            .intensity,
        result
            .identified_breakdown
            .by_intensity
            .modified_annotated
            .pct
    );
    println!(
        "  Modified (unannotated): {:.2e} ({:.1}%)",
        result
            .identified_breakdown
            .by_intensity
            .modified_unannotated
            .intensity,
        result
            .identified_breakdown
            .by_intensity
            .modified_unannotated
            .pct
    );
    println!();

    // MS1 signal fate (if available)
    if let Some(ref ms1) = result.ms1_signal_fate {
        println!("MS1 Signal Fate:");
        println!("  Total MS1 TIC: {:.2e}", ms1.total_ms1_tic);
        println!(
            "  Explained by IDs: {:.2e} ({:.1}%)",
            ms1.explained_intensity, ms1.explained_pct
        );
        println!(
            "  Unexplained: {:.2e} ({:.1}%)",
            ms1.unexplained_intensity, ms1.unexplained_pct
        );
        println!(
            "  PSMs with MS1 signal: {} / {} ({:.1}%)",
            ms1.psms_with_ms1,
            ms1.psms_with_ms1 + ms1.psms_without_ms1,
            100.0 * ms1.psms_with_ms1 as f64 / (ms1.psms_with_ms1 + ms1.psms_without_ms1) as f64
        );
        println!();

        // Show comparison table if we have both MS1 and MS2 intensity data
        if let Some(ref unid) = result.unidentified {
            println!("=== Signal Fate Comparison ===");
            println!();
            println!(
                "{:<20} {:>12} {:>16} {:>16}",
                "", "By Count", "By MS2 Intensity", "By MS1 Intensity"
            );
            println!("{}", "-".repeat(68));
            println!(
                "{:<20} {:>11.1}% {:>15.1}% {:>15.1}%",
                "Identified:",
                result.by_count.identified_pct.unwrap_or(0.0),
                result.by_intensity.identified_pct.unwrap_or(0.0),
                ms1.explained_pct
            );
            println!(
                "{:<20} {:>11.1}% {:>15.1}% {:>15.1}%",
                "Unidentified:",
                unid.unidentified_pct,
                unid.unidentified_intensity_pct,
                ms1.unexplained_pct
            );
            println!();
        }
    }

    println!("Filtering Statistics:");
    println!(
        "  PSMs before filter: {}",
        result.filtering.total_before_filter
    );
    println!("  Decoys removed: {}", result.filtering.decoys_removed);
    println!("  Q-value filtered: {}", result.filtering.q_filtered);
    println!(
        "  PSMs after filter: {}",
        result.filtering.total_after_filter
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sage_results::Psm;

    fn make_test_psm(scannr: u32, delta: f64, intensity: f64) -> Psm {
        Psm {
            scannr,
            rank: 1,
            peptide: "PEPTIDE".to_string(),
            proteins: "PROTEIN".to_string(),
            expmass: 1000.0 + delta,
            calcmass: 1000.0,
            isotope_error: 0,
            delta_mass: delta,
            delta_mass_corrected: delta,
            hyperscore: 40.0,
            matched_intensity_pct: 0.5,
            longest_b: 5,
            longest_y: 6,
            ms2_intensity: intensity,
            peptide_q: 0.001,
            spectrum_q: 0.001,
            is_decoy: false,
            charge: 2,
            rt: 30.0,
            missed_cleavages: 0,
            semi_enzymatic: false,
            precursor_ppm: 2.0,
            fragment_ppm: 5.0,
            peptide_len: 7,
        }
    }

    #[test]
    fn test_classify_scan_unmodified() {
        let psm = make_test_psm(1, 0.0, 1000.0);
        let psms = vec![&psm];
        let annotated: HashMap<i64, bool> = HashMap::new();

        let status = classify_scan(&psms, &annotated);
        assert_eq!(status, ModificationStatus::Unmodified);
    }

    #[test]
    fn test_classify_scan_modified_unannotated() {
        let psm = make_test_psm(1, 15.99, 1000.0);
        let psms = vec![&psm];
        let annotated: HashMap<i64, bool> = HashMap::new(); // No annotations

        let status = classify_scan(&psms, &annotated);
        assert_eq!(status, ModificationStatus::ModifiedUnannotated);
    }

    #[test]
    fn test_classify_scan_modified_annotated() {
        let psm = make_test_psm(1, 15.99, 1000.0);
        let psms = vec![&psm];
        let mut annotated: HashMap<i64, bool> = HashMap::new();
        annotated.insert(1599, true); // 15.99 * 100 = 1599

        let status = classify_scan(&psms, &annotated);
        assert_eq!(status, ModificationStatus::ModifiedAnnotated);
    }

    #[test]
    fn test_classify_chimeric_scan_priority() {
        // Chimeric scan with both unmodified and modified PSMs
        let psm1 = make_test_psm(1, 0.0, 1000.0); // Unmodified
        let psm2 = make_test_psm(1, 15.99, 1000.0); // Modified
        let psms = vec![&psm1, &psm2];
        let annotated: HashMap<i64, bool> = HashMap::new();

        // Unmodified takes priority
        let status = classify_scan(&psms, &annotated);
        assert_eq!(status, ModificationStatus::Unmodified);
    }

    #[test]
    fn test_chimera_stats() {
        // Create results with chimeric scans
        let psms = vec![
            make_test_psm(1, 0.0, 1000.0),
            make_test_psm(1, 15.99, 1000.0), // Same scan = chimeric
            make_test_psm(2, 0.0, 2000.0),   // Different scan
            make_test_psm(3, 0.98, 1500.0),  // Different scan
        ];

        let results = SageResults {
            psms,
            filter_stats: FilterStats {
                total_before_filter: 10,
                decoys_removed: 2,
                q_filtered: 4,
                isotope_filtered: 0,
                total_after_filter: 4,
            },
            isotope_error_distribution: HashMap::new(),
        };

        let fate = compute_signal_fate(&results, None);

        assert_eq!(fate.chimera_stats.unique_scans, 3);
        assert_eq!(fate.chimera_stats.scans_with_multiple_psms, 1);
        assert_eq!(fate.chimera_stats.total_psms, 4);
        assert!((fate.chimera_stats.chimera_rate_pct - 33.33).abs() < 0.1);
    }

    #[test]
    fn test_intensity_per_scan() {
        // Verify that intensity is counted once per scan, not per PSM
        let psms = vec![
            make_test_psm(1, 0.0, 1000.0),
            make_test_psm(1, 15.99, 1000.0), // Same scan, same intensity
        ];

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

        let fate = compute_signal_fate(&results, None);

        // Should be 1000.0, not 2000.0 (intensity counted once per scan)
        assert!((fate.by_intensity.identified_intensity - 1000.0).abs() < 0.01);
    }
}
