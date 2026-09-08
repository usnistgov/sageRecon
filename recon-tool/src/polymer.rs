//! Polymer contamination detection.
//!
//! This file includes code adapted from wfondrie/mzsniffer (Apache 2.0).
//! Modified by Benjamin A. Neely (NIST) on 2026-07-07: ported polymer
//! definitions, formula masses, and m/z matching logic from Python to Rust;
//! restructured as a library module with typed structs.
//! Source: https://github.com/wfondrie/mzsniffer

use std::collections::HashMap;

// Proton mass for m/z calculation. `mzml` holds the only definition.
// This module had a local copy of 1.007276466879, which was 6.7e-11 Da above the
// value the rest of the crate used. No source in the repo justified the local
// value, so the shared one replaces it.
use crate::mzml::PROTON_MASS;

/// A polymer definition with core formula and repeat unit
#[derive(Debug, Clone)]
pub struct Polymer {
    /// Name of the polymer
    pub name: String,
    /// Mass of the non-repeating core
    core_mass: f64,
    /// Mass of the repeating unit
    rep_mass: f64,
    /// Charge state
    charge: i32,
    /// Whether to add protons for charge
    protonate: bool,
}

impl Polymer {
    /// Create a new polymer definition
    pub fn new(name: &str, core_mass: f64, rep_mass: f64, charge: i32, protonate: bool) -> Self {
        Self {
            name: name.to_string(),
            core_mass,
            rep_mass,
            charge,
            protonate,
        }
    }

    /// Calculate m/z for a given number of repeat units
    fn mz_for_repeats(&self, n: usize) -> f64 {
        let mass = self.core_mass + (n as f64) * self.rep_mass;
        if self.protonate {
            (mass + (self.charge as f64) * PROTON_MASS) / (self.charge as f64).abs()
        } else {
            mass / (self.charge as f64).abs()
        }
    }

    /// Generate all m/z values up to max_mz
    pub fn mz_series(&self, max_mz: f64) -> Vec<f64> {
        let mut mz_values = Vec::new();
        for n in 0.. {
            let mz = self.mz_for_repeats(n);
            if mz > max_mz {
                break;
            }
            if mz > 0.0 {
                mz_values.push(mz);
            }
            // Handle non-polymers (rep_mass == 0)
            if self.rep_mass == 0.0 {
                break;
            }
        }
        mz_values
    }
}

// Monoisotopic element masses, as the mzSniffer port carried them.
//
// Unimod is the project's authority for element masses (see `unimod.rs`,
// `UnimodDb::elements`), but this module is NOT wired to Unimod, because that
// would move a numeric path. The values sit at module scope so the test
// `element_masses_agree_with_unimod` can compare them and report a divergence.
const H: f64 = 1.007825;
const C: f64 = 12.0;
const O: f64 = 15.994915;
const NA: f64 = 22.989769;
const SI: f64 = 27.976927;

/// Formula masses (pre-calculated for common formulas)
/// These match mzSniffer's formula_mass() function
fn formula_mass(formula: &str) -> f64 {
    // Pre-calculated formula masses for mzSniffer defaults
    match formula {
        "H2O" => 2.0 * H + O,                               // 18.0106
        "C2H4O" => 2.0 * C + 4.0 * H + O,                   // 44.0262 (PEG repeat)
        "C3H6O" => 3.0 * C + 6.0 * H + O,                   // 58.0419 (PPG repeat)
        "C2H6SiO" => 2.0 * C + 6.0 * H + SI + O,            // 74.0188 (Polysiloxane repeat)
        "C14H22O" => 14.0 * C + 22.0 * H + O,               // 206.1671 (Triton X-100 core)
        "C14H28O" => 14.0 * C + 28.0 * H + O,               // 212.2140 (Triton X-100 reduced core)
        "C14H22ONa" => 14.0 * C + 22.0 * H + O + NA,        // 229.1563 (Triton X-100 Na)
        "C14H28ONa" => 14.0 * C + 28.0 * H + O + NA,        // 235.2032 (Triton X-100 reduced Na)
        "C15H24O" => 15.0 * C + 24.0 * H + O,               // 220.1827 (Triton X-101 / IGEPAL)
        "C15H30O" => 15.0 * C + 30.0 * H + O,               // 226.2297 (Triton X-101 reduced)
        "C18H34O6Na" => 18.0 * C + 34.0 * H + 6.0 * O + NA, // 369.2253 (Tween-20)
        "C22H42O6Na" => 22.0 * C + 42.0 * H + 6.0 * O + NA, // 425.2879 (Tween-40)
        "C24H46O6Na" => 24.0 * C + 46.0 * H + 6.0 * O + NA, // 453.3192 (Tween-60)
        "C24H44O6Na" => 24.0 * C + 44.0 * H + 6.0 * O + NA, // 451.3035 (Tween-80)
        "" => 0.0,
        _ => {
            // For unknown formulas, return 0 and log warning
            log::warn!("Unknown formula: {}", formula);
            0.0
        }
    }
}

/// Default polymer definitions (from mzSniffer)
pub fn default_polymers() -> Vec<Polymer> {
    vec![
        // PEG (polyethylene glycol) at different charge states
        Polymer::new(
            "PEG+1H",
            formula_mass("H2O"),
            formula_mass("C2H4O"),
            1,
            true,
        ),
        Polymer::new(
            "PEG+2H",
            formula_mass("H2O"),
            formula_mass("C2H4O"),
            2,
            true,
        ),
        Polymer::new(
            "PEG+3H",
            formula_mass("H2O"),
            formula_mass("C2H4O"),
            3,
            true,
        ),
        // PPG (polypropylene glycol)
        Polymer::new("PPG", formula_mass("H2O"), formula_mass("C3H6O"), 1, true),
        // Triton X-100 variants
        Polymer::new(
            "Triton X-100",
            formula_mass("C14H22O"),
            formula_mass("C2H4O"),
            1,
            true,
        ),
        Polymer::new(
            "Triton X-100 (Reduced)",
            formula_mass("C14H28O"),
            formula_mass("C2H4O"),
            1,
            true,
        ),
        Polymer::new(
            "Triton X-100 (Na)",
            formula_mass("C14H22ONa"),
            formula_mass("C2H4O"),
            1,
            false,
        ),
        Polymer::new(
            "Triton X-100 (Reduced, Na)",
            formula_mass("C14H28ONa"),
            formula_mass("C2H4O"),
            1,
            false,
        ),
        // Triton X-101
        Polymer::new(
            "Triton X-101",
            formula_mass("C15H24O"),
            formula_mass("C2H4O"),
            1,
            true,
        ),
        Polymer::new(
            "Triton X-101 (Reduced)",
            formula_mass("C15H30O"),
            formula_mass("C2H4O"),
            1,
            true,
        ),
        // Polysiloxane
        Polymer::new("Polysiloxane", 0.0, formula_mass("C2H6SiO"), 1, true),
        // Tween variants
        Polymer::new(
            "Tween-20",
            formula_mass("C18H34O6Na"),
            formula_mass("C2H4O"),
            1,
            false,
        ),
        Polymer::new(
            "Tween-40",
            formula_mass("C22H42O6Na"),
            formula_mass("C2H4O"),
            1,
            false,
        ),
        Polymer::new(
            "Tween-60",
            formula_mass("C24H46O6Na"),
            formula_mass("C2H4O"),
            1,
            false,
        ),
        Polymer::new(
            "Tween-80",
            formula_mass("C24H44O6Na"),
            formula_mass("C2H4O"),
            1,
            false,
        ),
        // IGEPAL CA-630 (NP-40)
        Polymer::new(
            "IGEPAL CA-630 (NP-40)",
            formula_mass("C15H24O"),
            formula_mass("C2H4O"),
            1,
            true,
        ),
    ]
}

/// Result for a single polymer search
#[derive(Debug, Clone)]
pub struct PolymerResult {
    /// Polymer name
    pub name: String,
    /// Total intensity across all MS1 scans
    pub total_intensity: f64,
    /// Intensity per scan (XIC-like)
    pub xic: Vec<f64>,
}

/// Results for all polymer searches
#[derive(Debug, Clone)]
pub struct PolymerSearchResults {
    /// Results for each polymer type
    pub polymers: Vec<PolymerResult>,
    /// Retention times for each MS1 scan
    pub ret_times: Vec<f64>,
    /// TIC for each MS1 scan
    pub tic: Vec<f64>,
    /// Total TIC across all MS1 scans
    pub total_tic: f64,
}

impl PolymerSearchResults {
    /// Calculate %TIC for each polymer
    pub fn pct_tic(&self) -> HashMap<String, f64> {
        let mut result = HashMap::new();
        for poly in &self.polymers {
            let pct = if self.total_tic > 0.0 {
                100.0 * poly.total_intensity / self.total_tic
            } else {
                0.0
            };
            result.insert(poly.name.clone(), pct);
        }
        result
    }

    /// Get total polymer %TIC (sum of all polymers)
    pub fn total_polymer_pct_tic(&self) -> f64 {
        self.pct_tic().values().sum()
    }

    /// Polymers sorted by %TIC, descending, ties broken by NAME.
    ///
    /// ⚠ The name tie-break is not cosmetic. `pct_tic` returns a `HashMap`, so
    /// `into_iter` yields an arbitrary order, and sorting on the value alone
    /// left equal values in whatever order the map produced. Surfactants that
    /// share a repeat unit have identical mass series and therefore identical
    /// %TIC, so the LABEL moved between runs while the number stayed the same.
    ///
    /// Observed 2026-09-03 regenerating bcell: rank 9 read `Triton X-101` in one
    /// run and `IGEPAL CA-630 (NP-40)` in the next, both at 0.002936 %TIC. A
    /// reader would take that as a change of sample. Same data must give the
    /// same name.
    pub fn polymers_by_pct_tic(&self) -> Vec<(String, f64)> {
        let mut pcts: Vec<_> = self.pct_tic().into_iter().collect();
        pcts.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        pcts
    }
}

/// Find peaks matching query m/z values in a spectrum
/// Returns the sum of max intensities at each query m/z
fn find_peaks(
    query_mz: &[f64],
    tol_ppm: f64,
    spectrum_mz: &[f64],
    spectrum_intensity: &[f64],
) -> f64 {
    let mut total_intensity = 0.0;

    for &qmz in query_mz {
        let tol = tol_ppm * qmz / 1_000_000.0;
        let mut biggest = 0.0;

        for (&mz, &intensity) in spectrum_mz.iter().zip(spectrum_intensity.iter()) {
            if (mz - qmz).abs() <= tol && intensity > biggest {
                biggest = intensity;
            }
        }
        total_intensity += biggest;
    }

    total_intensity
}

/// Search for polymers in MS1 spectra
///
/// # Arguments
/// * `ms1_spectra` - Iterator of (rt, tic, mz_array, intensity_array) tuples
/// * `max_mz` - Maximum m/z to search (from scan range)
/// * `tol_ppm` - Mass tolerance in ppm (default: 10.0)
///
/// # Returns
/// PolymerSearchResults with %TIC for each polymer type
pub fn search_polymers<'a, I>(ms1_spectra: I, max_mz: f64, tol_ppm: f64) -> PolymerSearchResults
where
    I: Iterator<Item = (f64, f64, &'a [f64], &'a [f64])>,
{
    let polymers = default_polymers();

    // Pre-compute m/z series for each polymer
    let polymer_mz_series: Vec<Vec<f64>> = polymers.iter().map(|p| p.mz_series(max_mz)).collect();

    // Initialize results
    let mut polymer_results: Vec<PolymerResult> = polymers
        .iter()
        .map(|p| PolymerResult {
            name: p.name.clone(),
            total_intensity: 0.0,
            xic: Vec::new(),
        })
        .collect();

    let mut ret_times = Vec::new();
    let mut tic_values = Vec::new();
    let mut total_tic = 0.0;

    // Process each MS1 spectrum
    for (rt, tic, mz_array, intensity_array) in ms1_spectra {
        ret_times.push(rt);
        tic_values.push(tic);
        total_tic += tic;

        // Search for each polymer
        for (i, mz_series) in polymer_mz_series.iter().enumerate() {
            let intensity = find_peaks(mz_series, tol_ppm, mz_array, intensity_array);
            polymer_results[i].xic.push(intensity);
            polymer_results[i].total_intensity += intensity;
        }
    }

    PolymerSearchResults {
        polymers: polymer_results,
        ret_times,
        tic: tic_values,
        total_tic,
    }
}

#[cfg(test)]
mod tests {
    /// Equal %TIC must always come back in the same order. Without the name
    /// tie-break this passed or failed depending on HashMap iteration order.
    #[test]
    fn tied_polymers_sort_by_name_deterministically() {
        let mut prev: Option<Vec<String>> = None;
        for _ in 0..40 {
            let mut v: Vec<(String, f64)> = vec![
                ("Triton X-101".into(), 0.002936),
                ("IGEPAL CA-630 (NP-40)".into(), 0.002936),
                ("Triton X-100".into(), 0.004339),
            ];
            v.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            });
            let names: Vec<String> = v.into_iter().map(|(n, _)| n).collect();
            if let Some(ref p) = prev {
                assert_eq!(p, &names, "tied polymers reordered between runs");
            }
            prev = Some(names);
        }
        assert_eq!(
            prev.unwrap(),
            vec![
                "Triton X-100".to_string(),
                "IGEPAL CA-630 (NP-40)".to_string(),
                "Triton X-101".to_string()
            ]
        );
    }

    use super::*;

    #[test]
    fn test_peg_mz_series() {
        let peg = Polymer::new(
            "PEG+1H",
            formula_mass("H2O"),
            formula_mass("C2H4O"),
            1,
            true,
        );
        let series = peg.mz_series(500.0);

        // First few PEG m/z values (H2O + n*C2H4O + H)
        // n=0: 18.0106 + 1.0078 = 19.0184
        // n=1: 18.0106 + 44.0262 + 1.0078 = 63.0446
        // n=2: 18.0106 + 88.0524 + 1.0078 = 107.0708
        assert!(!series.is_empty());
        assert!(series[0] > 18.0 && series[0] < 20.0);

        // Check spacing is ~44 Da
        if series.len() > 1 {
            let spacing = series[1] - series[0];
            assert!((spacing - 44.0262).abs() < 0.01);
        }
    }

    #[test]
    fn test_find_peaks() {
        let query_mz = vec![100.0, 200.0, 300.0];
        // Use values within 10 ppm tolerance:
        // 10 ppm at 100 = 0.001 Da, at 200 = 0.002 Da, at 300 = 0.003 Da
        let spectrum_mz = vec![100.0005, 150.0, 200.001, 300.0];
        let spectrum_intensity = vec![1000.0, 500.0, 2000.0, 3000.0];

        let total = find_peaks(&query_mz, 10.0, &spectrum_mz, &spectrum_intensity);

        // Should match 100.0 -> 100.0005 (1000), 200.0 -> 200.001 (2000), 300.0 -> 300.0 (3000)
        assert!((total - 6000.0).abs() < 0.1);
    }

    #[test]
    fn test_default_polymers() {
        let polymers = default_polymers();
        assert!(polymers.len() >= 16); // mzSniffer has 16 default polymers

        // Check PEG is present
        assert!(polymers.iter().any(|p| p.name.contains("PEG")));

        // Check Triton is present
        assert!(polymers.iter().any(|p| p.name.contains("Triton")));
    }

    #[test]
    fn test_pct_tic_calculation() {
        let results = PolymerSearchResults {
            polymers: vec![
                PolymerResult {
                    name: "PEG".to_string(),
                    total_intensity: 1000.0,
                    xic: vec![500.0, 500.0],
                },
                PolymerResult {
                    name: "PPG".to_string(),
                    total_intensity: 500.0,
                    xic: vec![250.0, 250.0],
                },
            ],
            ret_times: vec![1.0, 2.0],
            tic: vec![5000.0, 5000.0],
            total_tic: 10000.0,
        };

        let pct = results.pct_tic();
        assert!((pct["PEG"] - 10.0).abs() < 0.01); // 1000/10000 = 10%
        assert!((pct["PPG"] - 5.0).abs() < 0.01); // 500/10000 = 5%
        assert!((results.total_polymer_pct_tic() - 15.0).abs() < 0.01);
    }

    /// This module keeps its own element masses, because they came with the
    /// mzSniffer port. Unimod is the authority for the same five elements.
    /// This test reports a divergence. It does not correct one: a correction
    /// would move the polymer m/z values, which is a deliberate decision.
    ///
    /// The largest difference measured on 2026-09-02 was 1.3e-6 Da, on Na.
    /// The tolerance gives that a small margin. It is far below the 10 ppm
    /// match window that `find_peaks` uses (1e-3 Da at m/z 100).
    #[test]
    fn element_masses_agree_with_unimod() {
        const TOLERANCE_DA: f64 = 5e-6;

        let db = crate::unimod::UnimodDb::from_embedded().expect("embedded Unimod must parse");
        let elements = db.elements();

        for (title, local) in [("H", H), ("C", C), ("O", O), ("Na", NA), ("Si", SI)] {
            let unimod = *elements
                .get(title)
                .unwrap_or_else(|| panic!("Unimod must give a mass for element {title}"));
            let diff = (local - unimod).abs();
            assert!(
                diff <= TOLERANCE_DA,
                "{title}: polymer.rs has {local}, Unimod has {unimod}, difference {diff:e} Da"
            );
        }
    }
}
