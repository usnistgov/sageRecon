//! Exclusive PSM assignment: a PSM belongs to at most one peak.
//!
//! This is the invariant behind the serum +1 Da conservation violation recorded
//! in NOTES.md. Two earlier check scripts tried to settle that by comparing PSM
//! counts inside a mass region, and both returned a false answer. The defect is
//! not a region-boundary effect, so these tests do not count a region. They run
//! the pipeline and ask the structural question directly: does any PSM index
//! land in two peaks?
//!
//! The invariant is not a mass-spec convention. It holds or fails independently
//! of what this crate believes about mass, units, or sign, so a fixture cannot
//! make it agree by accident. The gate lives in
//! `mod_discovery::detect_peaks_with_prominence` and runs in both modes.
//!
//! Input is `tests/fixtures/determinism_psms.tsv` — a real slice of the b1906
//! open search (header + 2000 PSMs), not synthesised deltas.

use recon_tool::mod_discovery::{
    run_mod_discovery, ModDiscoveryConfig, ModDiscoveryResult, PeakAssignmentMode,
};
use recon_tool::sage_results::{parse_sage_results, FilterOptions};
use recon_tool::unimod::UnimodDb;
use std::path::PathBuf;

fn manifest(parts: &[&str]) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for part in parts {
        p.push(part);
    }
    p
}

fn run_with(mode: PeakAssignmentMode) -> ModDiscoveryResult {
    let results = parse_sage_results(
        &manifest(&["tests", "fixtures", "determinism_psms.tsv"]),
        &FilterOptions::default(),
    )
    .expect("parse fixture");
    let unimod = UnimodDb::from_xml(&manifest(&["resources", "unimod.xml"])).expect("load unimod");
    let config = ModDiscoveryConfig {
        peak_assignment_mode: mode,
        ..Default::default()
    };
    run_mod_discovery(&results, &unimod, &config)
}

/// Both modes must satisfy the invariant. The gate is an assert inside
/// `detect_peaks_with_prominence`, so reaching a result at all means it held.
#[test]
fn both_modes_assign_each_psm_to_one_peak() {
    for mode in [PeakAssignmentMode::Merge, PeakAssignmentMode::Split] {
        let result = run_with(mode);
        assert!(
            !result.peaks.is_empty(),
            "{mode:?} produced no peaks; the fixture should yield some"
        );
    }
}

/// Independent restatement of the invariant, checked from OUTSIDE the module.
///
/// The in-module gate proves no PSM index was claimed twice. This proves the
/// consequence that actually matters downstream: peak counts cannot sum to more
/// PSMs than the run holds. If the gate were ever weakened, this still fails.
///
/// The Unmodified roll-up merges the near-zero peaks and recounts them over a
/// wider window than peak detection uses, so it is excluded here — this checks
/// the peaks as detection built them.
#[test]
fn peak_counts_never_exceed_total_psms() {
    for mode in [PeakAssignmentMode::Merge, PeakAssignmentMode::Split] {
        let result = run_with(mode);
        let claimed: usize = result
            .peaks
            .iter()
            .filter(|p| !p.annotations.iter().any(|a| a.name == "Unmodified"))
            .map(|p| p.count)
            .sum();
        assert!(
            claimed <= result.summary.total_psms,
            "{mode:?}: peaks claim {} PSMs but the run holds {}",
            claimed,
            result.summary.total_psms
        );
    }
}

/// The two modes must actually differ, or the comparison is meaningless.
///
/// Split keeps adjacent bins as separate peaks; Merge collapses them. Split must
/// therefore report at least as many peaks as Merge, and on real data strictly
/// more — the b1906 fixture has adjacent prominent bins around +1 Da.
#[test]
fn split_resolves_more_peaks_than_merge() {
    let merge = run_with(PeakAssignmentMode::Merge);
    let split = run_with(PeakAssignmentMode::Split);
    assert!(
        split.peaks.len() > merge.peaks.len(),
        "split {} peaks vs merge {} — the modes are not behaving differently",
        split.peaks.len(),
        merge.peaks.len()
    );
}
