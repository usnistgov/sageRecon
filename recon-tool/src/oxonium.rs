//! Oxonium ion screening for glycopeptide detection.
//!
//! Screens MS2 spectra for diagnostic oxonium ions that indicate glycopeptides.
//! Based on published glycoproteomics screening rules.
//!
//! Reference: `_dev/reference-notes/oxonium-ions.md`

use crate::mzml::Ms2Spectrum;
use serde::{Deserialize, Serialize};

/// Oxonium ion definitions with m/z values and names
#[derive(Debug, Clone)]
pub struct OxoniumIon {
    /// Ion name
    pub name: &'static str,
    /// Exact m/z value
    pub mz: f64,
    /// Whether this ion is mandatory for screening
    pub mandatory: bool,
}

/// Standard oxonium ions for glycopeptide screening
pub const OXONIUM_IONS: &[OxoniumIon] = &[
    OxoniumIon {
        name: "HexNAc",
        mz: 204.0867,
        mandatory: true,
    },
    OxoniumIon {
        name: "Hex-HexNAc",
        mz: 366.1395,
        mandatory: false,
    },
    OxoniumIon {
        name: "NeuAc",
        mz: 292.1027,
        mandatory: false,
    },
    OxoniumIon {
        name: "NeuAc-H2O",
        mz: 274.0921,
        mandatory: false,
    },
    OxoniumIon {
        name: "HexNAc-H2O",
        mz: 186.0761,
        mandatory: false,
    },
    OxoniumIon {
        name: "HexNAc-2H2O",
        mz: 168.0655,
        mandatory: false,
    },
    OxoniumIon {
        name: "Hexose",
        mz: 163.0601,
        mandatory: false,
    },
    OxoniumIon {
        name: "HexNAc-fragment",
        mz: 138.0545,
        mandatory: false,
    },
];

/// Configuration for oxonium ion screening
#[derive(Debug, Clone)]
pub struct OxoniumScreeningConfig {
    /// m/z tolerance in ppm (default: 20.0)
    pub mz_tolerance_ppm: f64,
    /// Minimum number of oxonium ions required (default: 2)
    pub min_oxonium_ions: usize,
    /// Top percentage of peaks to consider (default: 0.10 = 10%)
    pub top_peak_fraction: f64,
    /// Require mandatory ion (m/z 204) to be present (default: true)
    pub require_mandatory: bool,
}

impl Default for OxoniumScreeningConfig {
    fn default() -> Self {
        Self {
            mz_tolerance_ppm: 20.0,
            min_oxonium_ions: 2,
            top_peak_fraction: 0.10,
            require_mandatory: true,
        }
    }
}

/// Result of oxonium ion screening for a single spectrum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OxoniumScreeningResult {
    /// Scan number
    pub scan: u32,
    /// Retention time in minutes
    pub rt: f64,
    /// Precursor m/z
    pub precursor_mz: f64,
    /// Precursor charge
    pub precursor_charge: u32,
    /// Whether the spectrum passes glycopeptide screening
    pub is_glycopeptide_candidate: bool,
    /// Number of oxonium ions detected in top peaks
    pub oxonium_ions_detected: usize,
    /// Names of detected oxonium ions
    pub detected_ion_names: Vec<String>,
    /// Whether the mandatory ion (HexNAc, m/z 204) was detected
    pub has_mandatory_ion: bool,
}

/// Summary of oxonium ion screening across all spectra
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OxoniumScreeningSummary {
    /// Total MS2 spectra screened
    pub total_spectra: usize,
    /// Number of glycopeptide candidates
    pub glycopeptide_candidates: usize,
    /// Percentage of spectra that are glycopeptide candidates
    pub glycopeptide_pct: f64,
    /// Distribution of oxonium ion counts
    pub ion_count_distribution: Vec<(usize, usize)>, // (ion_count, spectrum_count)
    /// Individual ion detection rates
    pub ion_detection_rates: Vec<(String, f64)>, // (ion_name, detection_rate)
}

/// Screen a single MS2 spectrum for oxonium ions.
///
/// # Arguments
/// * `spectrum` - MS2 spectrum with peak data
/// * `config` - Screening configuration
///
/// # Returns
/// Screening result for this spectrum
pub fn screen_spectrum(
    spectrum: &Ms2Spectrum,
    config: &OxoniumScreeningConfig,
) -> OxoniumScreeningResult {
    // Find intensity threshold for top peaks
    let intensity_threshold = if spectrum.intensity.is_empty() {
        0.0
    } else {
        let mut sorted_intensities: Vec<f64> = spectrum.intensity.clone();
        sorted_intensities.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

        let top_n = ((spectrum.intensity.len() as f64) * config.top_peak_fraction).ceil() as usize;
        let top_n = top_n.max(1).min(sorted_intensities.len());
        sorted_intensities[top_n - 1]
    };

    // Check for each oxonium ion in top peaks
    let mut detected_ions = Vec::new();
    let mut has_mandatory = false;

    for oxonium in OXONIUM_IONS {
        let tol = config.mz_tolerance_ppm * oxonium.mz / 1_000_000.0;

        // Check if any peak matches this oxonium ion and is in top peaks
        let found = spectrum
            .mz
            .iter()
            .zip(spectrum.intensity.iter())
            .any(|(&mz, &intensity)| {
                (mz - oxonium.mz).abs() <= tol && intensity >= intensity_threshold
            });

        if found {
            detected_ions.push(oxonium.name.to_string());
            if oxonium.mandatory {
                has_mandatory = true;
            }
        }
    }

    // Determine if this is a glycopeptide candidate
    let is_candidate = detected_ions.len() >= config.min_oxonium_ions
        && (!config.require_mandatory || has_mandatory);

    OxoniumScreeningResult {
        scan: spectrum.scan,
        rt: spectrum.rt,
        precursor_mz: spectrum.precursor_mz,
        precursor_charge: spectrum.precursor_charge,
        is_glycopeptide_candidate: is_candidate,
        oxonium_ions_detected: detected_ions.len(),
        detected_ion_names: detected_ions,
        has_mandatory_ion: has_mandatory,
    }
}

/// Screen multiple MS2 spectra for oxonium ions.
///
/// # Arguments
/// * `spectra` - Slice of MS2 spectra
/// * `config` - Screening configuration
///
/// # Returns
/// Vector of screening results, one per spectrum
pub fn screen_spectra(
    spectra: &[Ms2Spectrum],
    config: &OxoniumScreeningConfig,
) -> Vec<OxoniumScreeningResult> {
    spectra.iter().map(|s| screen_spectrum(s, config)).collect()
}

/// Compute summary statistics from screening results.
///
/// # Arguments
/// * `results` - Screening results from `screen_spectra`
///
/// # Returns
/// Summary statistics
pub fn compute_screening_summary(results: &[OxoniumScreeningResult]) -> OxoniumScreeningSummary {
    let total = results.len();
    let candidates: usize = results
        .iter()
        .filter(|r| r.is_glycopeptide_candidate)
        .count();

    // Ion count distribution
    let mut ion_counts: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for r in results {
        *ion_counts.entry(r.oxonium_ions_detected).or_insert(0) += 1;
    }
    let mut ion_count_dist: Vec<_> = ion_counts.into_iter().collect();
    ion_count_dist.sort_by_key(|(count, _)| *count);

    // Individual ion detection rates
    let mut ion_detections: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    for oxonium in OXONIUM_IONS {
        ion_detections.insert(oxonium.name, 0);
    }
    for r in results {
        for name in &r.detected_ion_names {
            if let Some(count) = ion_detections.get_mut(name.as_str()) {
                *count += 1;
            }
        }
    }
    let ion_rates: Vec<_> = OXONIUM_IONS
        .iter()
        .map(|o| {
            let count = ion_detections.get(o.name).copied().unwrap_or(0);
            let rate = if total > 0 {
                (count as f64) / (total as f64) * 100.0
            } else {
                0.0
            };
            (o.name.to_string(), rate)
        })
        .collect();

    OxoniumScreeningSummary {
        total_spectra: total,
        glycopeptide_candidates: candidates,
        glycopeptide_pct: if total > 0 {
            (candidates as f64) / (total as f64) * 100.0
        } else {
            0.0
        },
        ion_count_distribution: ion_count_dist,
        ion_detection_rates: ion_rates,
    }
}

/// Print a summary of oxonium ion screening results
pub fn print_screening_summary(summary: &OxoniumScreeningSummary) {
    println!("=== Oxonium Ion Screening Summary ===");
    println!();
    println!("Total MS2 spectra screened: {}", summary.total_spectra);
    println!(
        "Glycopeptide candidates: {} ({:.1}%)",
        summary.glycopeptide_candidates, summary.glycopeptide_pct
    );
    println!();

    println!("Oxonium Ion Detection Rates:");
    for (name, rate) in &summary.ion_detection_rates {
        println!("  {:<20} {:.1}%", name, rate);
    }
    println!();

    println!("Ion Count Distribution:");
    for (count, spectra) in &summary.ion_count_distribution {
        let pct = if summary.total_spectra > 0 {
            (*spectra as f64) / (summary.total_spectra as f64) * 100.0
        } else {
            0.0
        };
        println!("  {} ions: {} spectra ({:.1}%)", count, spectra, pct);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_spectrum(mz: Vec<f64>, intensity: Vec<f64>) -> Ms2Spectrum {
        Ms2Spectrum {
            scan: 1,
            rt: 10.0,
            precursor_mz: 500.0,
            precursor_charge: 2,
            mz,
            intensity,
        }
    }

    #[test]
    fn test_screen_spectrum_with_glycopeptide() {
        // Spectrum with HexNAc (204.0867) and Hex-HexNAc (366.1395) in top peaks
        let spectrum = make_test_spectrum(
            vec![100.0, 204.0867, 300.0, 366.1395, 500.0],
            vec![1000.0, 5000.0, 500.0, 4000.0, 2000.0], // 204 and 366 are in top 40%
        );

        // Use a higher top_peak_fraction for this test (40% = 2 peaks out of 5)
        let config = OxoniumScreeningConfig {
            top_peak_fraction: 0.40,
            ..Default::default()
        };
        let result = screen_spectrum(&spectrum, &config);

        assert!(result.is_glycopeptide_candidate);
        assert!(result.has_mandatory_ion);
        assert_eq!(result.oxonium_ions_detected, 2);
        assert!(result.detected_ion_names.contains(&"HexNAc".to_string()));
        assert!(result
            .detected_ion_names
            .contains(&"Hex-HexNAc".to_string()));
    }

    #[test]
    fn test_screen_spectrum_without_mandatory() {
        // Spectrum with Hex-HexNAc and NeuAc but NOT HexNAc (mandatory)
        let spectrum = make_test_spectrum(
            vec![100.0, 292.1027, 300.0, 366.1395, 500.0],
            vec![1000.0, 5000.0, 500.0, 4000.0, 2000.0],
        );

        // Use higher top_peak_fraction to include both oxonium ions
        let config = OxoniumScreeningConfig {
            top_peak_fraction: 0.40,
            ..Default::default()
        };
        let result = screen_spectrum(&spectrum, &config);

        // Should NOT be a candidate because mandatory ion is missing
        assert!(!result.is_glycopeptide_candidate);
        assert!(!result.has_mandatory_ion);
        assert_eq!(result.oxonium_ions_detected, 2);
    }

    #[test]
    fn test_screen_spectrum_only_one_ion() {
        // Spectrum with only HexNAc (need at least 2)
        let spectrum = make_test_spectrum(
            vec![100.0, 204.0867, 300.0, 400.0, 500.0],
            vec![1000.0, 5000.0, 500.0, 400.0, 2000.0],
        );

        let config = OxoniumScreeningConfig::default();
        let result = screen_spectrum(&spectrum, &config);

        // Should NOT be a candidate because only 1 ion
        assert!(!result.is_glycopeptide_candidate);
        assert!(result.has_mandatory_ion);
        assert_eq!(result.oxonium_ions_detected, 1);
    }

    #[test]
    fn test_screen_spectrum_ion_not_in_top_peaks() {
        // Spectrum where oxonium ions are present but not in top 10%
        let spectrum = make_test_spectrum(
            vec![
                100.0, 204.0867, 300.0, 366.1395, 500.0, 600.0, 700.0, 800.0, 900.0, 1000.0,
            ],
            vec![
                10000.0, 100.0, 9000.0, 50.0, 8000.0, 7000.0, 6000.0, 5000.0, 4000.0, 3000.0,
            ],
        );

        let config = OxoniumScreeningConfig::default();
        let result = screen_spectrum(&spectrum, &config);

        // Oxonium ions have low intensity, not in top 10%
        assert!(!result.is_glycopeptide_candidate);
    }

    #[test]
    fn test_compute_screening_summary() {
        let results = vec![
            OxoniumScreeningResult {
                scan: 1,
                rt: 10.0,
                precursor_mz: 500.0,
                precursor_charge: 2,
                is_glycopeptide_candidate: true,
                oxonium_ions_detected: 3,
                detected_ion_names: vec![
                    "HexNAc".to_string(),
                    "Hex-HexNAc".to_string(),
                    "NeuAc".to_string(),
                ],
                has_mandatory_ion: true,
            },
            OxoniumScreeningResult {
                scan: 2,
                rt: 11.0,
                precursor_mz: 600.0,
                precursor_charge: 2,
                is_glycopeptide_candidate: false,
                oxonium_ions_detected: 0,
                detected_ion_names: vec![],
                has_mandatory_ion: false,
            },
        ];

        let summary = compute_screening_summary(&results);

        assert_eq!(summary.total_spectra, 2);
        assert_eq!(summary.glycopeptide_candidates, 1);
        assert!((summary.glycopeptide_pct - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_oxonium_ion_mz_values() {
        // Verify key m/z values match reference
        let hexnac = OXONIUM_IONS.iter().find(|o| o.name == "HexNAc").unwrap();
        assert!((hexnac.mz - 204.0867).abs() < 0.0001);
        assert!(hexnac.mandatory);

        let hex_hexnac = OXONIUM_IONS
            .iter()
            .find(|o| o.name == "Hex-HexNAc")
            .unwrap();
        assert!((hex_hexnac.mz - 366.1395).abs() < 0.0001);
        assert!(!hex_hexnac.mandatory);
    }
}
