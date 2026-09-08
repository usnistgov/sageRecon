//! The MS1 bias must be a SIGNED number, measured on real data.
//!
//! Sage v0.14.7 — the pinned version — reports `precursor_ppm` as `|error|`.
//! `PsmSummary::from_psm` used to copy that column, so `bias_ppm` was a median
//! of absolute errors: it could never be negative, and it read high on any file
//! where the true bias is not large compared with the scatter. See NOTES
//! "BUG — MS1/MS2 bias was a median of |error|".
//!
//! THE INVARIANT THIS FILE EXISTS TO ASSERT: the corrected median MUST be able
//! to come out negative. bcell is the regression case — the reported value was
//! +0.70 ppm and the true value is -0.24 ppm, so the SIGN FLIPS. A fix that
//! only moves the number is not verified.
//!
//! The expected values are not this code's own output. They were produced
//! independently by `_dev/testing/scripts/ms1_bias_sign_check.py`, which
//! reconstructs the signed error straight from the TSV's mass columns in
//! Python, and they are corroborated by two other tools: MetaMorpheus reports
//! -0.295 ppm and MSFragger -0.01 ppm on bcell.

use recon_tool::calibration::{compute_ms1_stats, select_clean_subset, PsmSummary};
use recon_tool::sage_results::{parse_sage_results, FilterOptions};
use std::path::{Path, PathBuf};

fn repo() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn open_tsv(file_key: &str) -> PathBuf {
    repo().join(format!(
        "_dev/testing/search-output/step1-open-{file_key}/results.sage.tsv"
    ))
}

/// The clean subset for one committed file, WITHOUT the hyperscore guard.
///
/// Unguarded on purpose: it is the population the Python check script measured,
/// so the numbers below are comparable with an independent measurement rather
/// than with this code's own. The guard is a separate, already-tested rule.
///
/// Returns `None` when the TSV is not on this machine — the search outputs are
/// gitignored, so every test that needs them must skip cleanly.
fn clean_subset(file_key: &str) -> Option<Vec<PsmSummary>> {
    let tsv = open_tsv(file_key);
    if !tsv.exists() {
        eprintln!("skipping: {} absent", tsv.display());
        return None;
    }
    // Defaults match the script: decoys dropped, peptide_q < 0.01.
    let results = parse_sage_results(&tsv, &FilterOptions::default()).expect("TSV must parse");
    let summaries: Vec<PsmSummary> = results.psms.iter().map(PsmSummary::from_psm).collect();
    Some(select_clean_subset(&summaries, 0.02, 0.01, false, 0))
}

#[test]
fn ms1_bias_is_negative_on_bcell() {
    let Some(subset) = clean_subset("bcell") else {
        return;
    };
    let stats = compute_ms1_stats(&subset).expect("clean subset must not be empty");

    println!(
        "bcell: n={} bias={:+.4} ppm MAD={:.4} ppm",
        stats.n_psms, stats.bias_ppm, stats.mad_ppm
    );

    // THE INVARIANT. The old code could not produce this value at all.
    assert!(
        stats.bias_ppm < 0.0,
        "bcell MS1 bias must be NEGATIVE, got {:+.4} ppm. A non-negative value \
         means the signed reconstruction was lost and the median folded again.",
        stats.bias_ppm
    );

    // The independently-measured values, pinned. Tolerances are loose enough to
    // survive float summation order, tight enough that a convention change trips.
    assert_eq!(stats.n_psms, 32133, "clean-subset size moved");
    assert!(
        (stats.bias_ppm - (-0.2357)).abs() < 0.001,
        "bcell bias {:+.4} ppm does not match the independent measurement -0.2357",
        stats.bias_ppm
    );
    assert!(
        (stats.mad_ppm - 0.6733).abs() < 0.001,
        "bcell MAD {:.4} ppm does not match the independent measurement 0.6733",
        stats.mad_ppm
    );
}

#[test]
fn the_folded_column_could_not_have_produced_that_answer() {
    // The control for the test above: prove the bug was real on this exact
    // file, rather than asserting a negative number in isolation. Sage's own
    // `precursor_ppm` column holds NO negative values, so any median of it is
    // >= 0 by construction and the sign flip is unreachable from it.
    let tsv = open_tsv("bcell");
    if !tsv.exists() {
        eprintln!("skipping: {} absent", tsv.display());
        return;
    }
    let results = parse_sage_results(&tsv, &FilterOptions::default()).expect("TSV must parse");
    let clean: Vec<&recon_tool::sage_results::Psm> = results
        .psms
        .iter()
        .filter(|p| p.rank == 1 && p.delta_mass_corrected.abs() < 0.02)
        .collect();

    let negatives = clean.iter().filter(|p| p.precursor_ppm < 0.0).count();
    println!(
        "bcell raw precursor_ppm: n={} negatives={}",
        clean.len(),
        negatives
    );
    assert_eq!(
        negatives, 0,
        "Sage's precursor_ppm has negative values on this file, so the pinned \
         version is no longer absolute. If Sage was upgraded to v0.15.x, REMOVE \
         the reconstruction in PsmSummary::from_psm — do not stack it."
    );
}

#[test]
fn ms1_bias_is_unchanged_on_serum_where_the_bug_was_invisible() {
    // serum is the file that hid the bug for months: |bias| (2.42) is far larger
    // than the scatter (0.48), so almost every PSM's error already has the same
    // sign and folding about zero changes nothing. It is the control that the
    // fix did NOT break the one file that was always right — and the standing
    // reminder that its agreement never validated the method.
    let Some(subset) = clean_subset("serum") else {
        return;
    };
    let stats = compute_ms1_stats(&subset).expect("clean subset must not be empty");

    println!(
        "serum: n={} bias={:+.4} ppm MAD={:.4} ppm",
        stats.n_psms, stats.bias_ppm, stats.mad_ppm
    );

    assert_eq!(stats.n_psms, 3764, "clean-subset size moved");
    assert!(
        (stats.bias_ppm - 2.4215).abs() < 0.001,
        "serum bias {:+.4} ppm does not match the independent measurement +2.4215",
        stats.bias_ppm
    );
}

#[test]
fn ms1_bias_moves_on_b1906_without_flipping() {
    // The third file: the bug inflated the bias but did not flip it. Included so
    // the regression case is not a single file.
    let Some(subset) = clean_subset("b1906") else {
        return;
    };
    let stats = compute_ms1_stats(&subset).expect("clean subset must not be empty");

    println!(
        "b1906: n={} bias={:+.4} ppm MAD={:.4} ppm",
        stats.n_psms, stats.bias_ppm, stats.mad_ppm
    );

    assert_eq!(stats.n_psms, 10942, "clean-subset size moved");
    assert!(
        (stats.bias_ppm - 0.4403).abs() < 0.001,
        "b1906 bias {:+.4} ppm does not match the independent measurement +0.4403",
        stats.bias_ppm
    );
}

// ---------------------------------------------------------------------------
// The tolerance recommendation, quantized to the {10,20,50,100} ppm ladder.
//
// PLAN's acceptance gate for this change, stated before it was written: all
// three test files must land on the 10 ppm rung. The predicted requirements
// come from `ms1-tolerance-recommendation-rationale.md` §12, which recomputed
// them on |error|-corrected inputs BEFORE this code existed — serum 4.84,
// bcell 3.60, b1906 3.86. So these are a prediction being tested, not a
// snapshot of whatever the code happens to emit.
// ---------------------------------------------------------------------------

use recon_tool::calibration::{ms1_user_recommendation, quantize_ms1_tolerance};

#[test]
fn all_three_files_land_on_the_10_ppm_rung() {
    let expected: [(&str, f64); 3] = [("serum", 4.844), ("bcell", 3.602), ("b1906", 3.862)];

    let mut checked = 0;
    for (file, predicted_requirement) in expected {
        let Some(subset) = clean_subset(file) else {
            continue;
        };
        let stats = compute_ms1_stats(&subset).expect("clean subset must not be empty");
        let rec = ms1_user_recommendation(&stats);
        let requirement = stats.bias_ppm.abs() + 5.0 * stats.mad_ppm;

        println!(
            "{file}: bias={:+.4} MAD={:.4} -> requirement {:.3} ppm -> ±{:.0} ppm",
            stats.bias_ppm, stats.mad_ppm, requirement, rec.recommended_tolerance_ppm
        );

        assert!(
            (requirement - predicted_requirement).abs() < 0.01,
            "{file}: requirement {requirement:.3} does not match the rationale \
             note's pre-computed {predicted_requirement:.3}"
        );
        assert_eq!(
            rec.recommended_tolerance_ppm, 10.0,
            "{file} must land on the 10 ppm rung"
        );
        assert_eq!(rec.low_ppm, -10.0);
        assert_eq!(rec.high_ppm, 10.0);
        checked += 1;
    }
    if checked == 0 {
        eprintln!("skipped: no search outputs on this machine");
    }
}

#[test]
fn the_old_formula_and_the_new_one_disagree_on_every_file() {
    // The control. If the change were cosmetic this test would fail, and the
    // point of quantizing is that it is NOT cosmetic: the superseded
    // asymmetric window was 3-5x too tight.
    for file in ["serum", "bcell", "b1906"] {
        let Some(subset) = clean_subset(file) else {
            continue;
        };
        let stats = compute_ms1_stats(&subset).unwrap();
        // The superseded formula was `bias + p95(|dev|)`. Its p95 term is gone
        // from `MassErrorStats` now that nothing consumes it, so the comparison
        // uses the measured values recorded when the change was made:
        // serum +4.11, bcell +2.09, b1906 +3.89 ppm.
        let old_high = match file {
            "serum" => 4.11,
            "bcell" => 2.09,
            _ => 3.89,
        };
        let new_high = quantize_ms1_tolerance(stats.bias_ppm, stats.mad_ppm);
        println!("{file}: superseded high {old_high:+.2} ppm -> quantized ±{new_high:.0} ppm");
        assert!(
            new_high > old_high * 1.5,
            "{file}: quantized {new_high} should be materially wider than the \
             superseded {old_high}"
        );
    }
}
