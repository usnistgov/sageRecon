//! Integration tests for sage_results parsing

use recon_tool::sage_results::{parse_sage_results, FilterOptions, C13_C12_DIFF};
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test_psms.tsv")
}

#[test]
fn test_parse_fixture_basic() {
    let path = fixture_path();
    let options = FilterOptions::default();
    let results = parse_sage_results(&path, &options).expect("Failed to parse fixture");

    // Fixture has 5 rows:
    // - Row 1: target, q=0.001, isotope=0 -> PASS
    // - Row 2: target, q=0.002, isotope=1 -> PASS
    // - Row 3: target, q=0.003, isotope=2 -> PASS
    // - Row 4: decoy (rev_), q=0.05 -> FILTERED (decoy)
    // - Row 5: target, q=0.02 -> FILTERED (q > 0.01)

    assert_eq!(results.filter_stats.total_before_filter, 5);
    assert_eq!(results.filter_stats.decoys_removed, 1);
    assert_eq!(results.filter_stats.q_filtered, 1);
    assert_eq!(results.filter_stats.total_after_filter, 3);
    assert_eq!(results.psms.len(), 3);
}

#[test]
fn test_isotope_error_distribution() {
    let path = fixture_path();
    let options = FilterOptions::default();
    let results = parse_sage_results(&path, &options).expect("Failed to parse fixture");

    // Check isotope distribution (before filtering)
    assert_eq!(results.isotope_error_distribution.get(&0), Some(&3)); // rows 1, 4, 5
    assert_eq!(results.isotope_error_distribution.get(&1), Some(&1)); // row 2
    assert_eq!(results.isotope_error_distribution.get(&2), Some(&1)); // row 3
}

#[test]
fn test_isotope_correction() {
    let path = fixture_path();
    let options = FilterOptions::default();
    let results = parse_sage_results(&path, &options).expect("Failed to parse fixture");

    // Row 1: expmass=1000.5, calcmass=1000.5, isotope=0
    // delta = 0.0, corrected = 0.0 - (0 * NEUTRON) = 0.0
    let psm1 = results.psms.iter().find(|p| p.scannr == 100).unwrap();
    assert!((psm1.delta_mass - 0.0).abs() < 0.001);
    assert!((psm1.delta_mass_corrected - 0.0).abs() < 0.001);

    // Row 2: expmass=1501.6, calcmass=1500.6, isotope=1
    // delta = 1.0, corrected = 1.0 - (1 * 1.003354835) = -0.0033548
    // The constant is the C13-C12 spacing, NOT the free neutron mass. These two
    // comments used to say 1.0086649, the neutron mass, which is the convention
    // the code was corrected AWAY from. Wrong by 0.0053101 per isotope step.
    let psm2 = results.psms.iter().find(|p| p.scannr == 200).unwrap();
    assert!((psm2.delta_mass - 1.0).abs() < 0.001);
    let expected_corrected = 1.0 - C13_C12_DIFF;
    assert!((psm2.delta_mass_corrected - expected_corrected).abs() < 0.0001);

    // Row 3: expmass=1702.8, calcmass=1700.8, isotope=2
    // delta = 2.0, corrected = 2.0 - (2 * 1.003354835) = -0.0067097
    let psm3 = results.psms.iter().find(|p| p.scannr == 300).unwrap();
    assert!((psm3.delta_mass - 2.0).abs() < 0.001);
    let expected_corrected3 = 2.0 - (2.0 * C13_C12_DIFF);
    assert!((psm3.delta_mass_corrected - expected_corrected3).abs() < 0.0001);
}

#[test]
fn test_scan_number_extraction() {
    let path = fixture_path();
    let options = FilterOptions::default();
    let results = parse_sage_results(&path, &options).expect("Failed to parse fixture");

    // Verify scan numbers were extracted correctly
    let scan_numbers: Vec<u32> = results.psms.iter().map(|p| p.scannr).collect();
    assert!(scan_numbers.contains(&100));
    assert!(scan_numbers.contains(&200));
    assert!(scan_numbers.contains(&300));
}

#[test]
fn test_isotope_zero_filter() {
    let path = fixture_path();
    let options = FilterOptions {
        isotope_error_zero_only: true,
        ..FilterOptions::default()
    };
    let results = parse_sage_results(&path, &options).expect("Failed to parse fixture");

    // Only row 1 should pass (isotope=0, target, q<0.01)
    // Row 2 (isotope=1) and Row 3 (isotope=2) filtered
    assert_eq!(results.psms.len(), 1);
    assert_eq!(results.psms[0].scannr, 100);
    assert_eq!(results.filter_stats.isotope_filtered, 2);
}
