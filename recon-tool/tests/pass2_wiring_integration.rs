//! The Pass 2 config must carry Pass 1's MEASURED window, correctly signed.
//!
//! The unit tests in `pass2.rs` prove the conversion on hand-written numbers.
//! This file runs the same path on the committed open searches, so the numbers
//! under test are the ones the tool would really produce.
//!
//! THE INVARIANT: Sage's `precursor_tol` is sign-INVERTED relative to the delta
//! mass it produces (measured on serum 2026-08-29 with `ppm [-30, 5]`, which
//! yielded delta -5.047 .. +30.007). So the config's `[a, b]` must satisfy
//! `[-b, -a] == [window.low, window.high]`, and the MEASURED BIAS must fall
//! inside the resulting delta window. On a biased instrument a sign error puts
//! the bias near one wall or outside it, which is the failure this catches and
//! a symmetric example cannot.

use recon_tool::calibration::{
    compute_ms1_stats, ms1_pass2_window, ms2_pass2_tolerance, select_clean_subset, PsmSummary,
};
use recon_tool::mzml::FragmentTolerance;
use recon_tool::pass2::Pass2Plan;
use recon_tool::sage_results::{parse_sage_results, FilterOptions};
use std::path::{Path, PathBuf};

fn repo() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn open_tsv(key: &str) -> PathBuf {
    repo().join(format!(
        "_dev/testing/search-output/step1-open-{key}/results.sage.tsv"
    ))
}

/// `(bias, window)` measured from one committed open search, unguarded — the
/// same population the canonical NOTES figures come from.
fn measured(key: &str) -> Option<(f64, recon_tool::calibration::Ms1Pass2Window)> {
    let tsv = open_tsv(key);
    if !tsv.exists() {
        eprintln!("skipping: {} absent", tsv.display());
        return None;
    }
    let results = parse_sage_results(&tsv, &FilterOptions::default()).expect("TSV must parse");
    let summaries: Vec<PsmSummary> = results.psms.iter().map(PsmSummary::from_psm).collect();
    let subset = select_clean_subset(&summaries, 0.02, 0.01, false, 0);
    let stats = compute_ms1_stats(&subset)?;
    Some((stats.bias_ppm, ms1_pass2_window(&stats)))
}

/// The Pass 2 template, as TEXT. Production never has this on disk in the
/// default case — `defaults::PASS2` is compiled in — so a test that writes it
/// to a file and reads it back is testing a step production does not take.
fn template_text() -> String {
    let j = serde_json::json!({
        "database": {
            "enzyme": { "semi_enzymatic": true, "cleave_at": "KR" },
            "fasta": "PLACEHOLDER"
        },
        "precursor_tol": { "ppm": [-10.0, 10.0] },
        "fragment_tol": { "ppm": [-20.0, 20.0] }
    });
    serde_json::to_string_pretty(&j).unwrap()
}

/// Write the Pass 2 effective params THE WAY `main.rs` DOES: template TEXT,
/// then `enzyme::apply_to_params_text`, then `write_pass2_params_from_text`
/// (`main.rs:2223-2228`).
///
/// ⚠ Until 2026-09-02 this gate called `write_pass2_params`, the path-taking
/// wrapper, which NOTHING in the shipped binary calls — and which skips the
/// enzyme step that sits between the template and the writer. The sign
/// invariant below was therefore proved on a config production never writes.
fn write_pass2(text: &str, plan: &Pass2Plan, out_dir: &Path) -> PathBuf {
    let enzyme = recon_tool::enzyme::parse("trypsin").expect("the trypsin preset must parse");
    let text = recon_tool::enzyme::apply_to_params_text(text, &enzyme)
        .expect("the enzyme must apply to the Pass 2 template");
    recon_tool::pass2::write_pass2_params_from_text(&text, "pass2-wiring-gate", plan, out_dir)
        .expect("Pass 2 params must write")
}

#[test]
fn the_pass2_config_encodes_the_measured_window_inverted_on_all_three_files() {
    let mut checked = 0;
    for key in ["serum", "bcell", "b1906"] {
        let Some((bias, window)) = measured(key) else {
            continue;
        };
        checked += 1;

        let dir = tempfile::tempdir().unwrap();
        let plan = Pass2Plan {
            ms1_window: window.clone(),
            ms2_tolerance: Some(FragmentTolerance::Ppm(6.0)),
            pass1_fragment_tol: FragmentTolerance::Ppm(50.0),
            subset_fasta: PathBuf::from("/tmp/subset.fasta"),
            subset_proteins: 1,
        };
        let out = write_pass2(&template_text(), &plan, dir.path());
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        // The enzyme step production runs first must land, and must NOT clobber
        // `semi_enzymatic` — it writes into the same `database.enzyme` object.
        // A fully-tryptic Pass 2 would report 0 % semi-tryptic as a finding.
        // `c_terminal` falsifies: the template carries cleave_at already, but
        // only `apply_to_params_text` writes c_terminal.
        assert_eq!(
            v["database"]["enzyme"]["c_terminal"], true,
            "{key}: the enzyme step did not reach the Pass 2 config"
        );
        assert_eq!(
            v["database"]["enzyme"]["semi_enzymatic"], true,
            "{key}: applying the enzyme dropped semi_enzymatic"
        );

        let a = v["precursor_tol"]["ppm"][0].as_f64().unwrap();
        let b = v["precursor_tol"]["ppm"][1].as_f64().unwrap();

        println!(
            "{key}: bias {bias:+.4} ppm | delta window {:+.4}..{:+.4} | config ppm [{a:.4}, {b:.4}]",
            window.low_ppm, window.high_ppm
        );

        // The inversion, on real measured numbers.
        assert!(
            (-b - window.low_ppm).abs() < 1e-9 && (-a - window.high_ppm).abs() < 1e-9,
            "{key}: config [{a}, {b}] does not invert to the measured window \
             [{}, {}]",
            window.low_ppm,
            window.high_ppm
        );

        // The bias must sit INSIDE the delta window the config produces. This is
        // the assertion a symmetric fixture cannot make: on bcell the bias is
        // negative and on serum strongly positive, so a sign flip moves the
        // window off the bias by 2x the bias.
        let (delta_lo, delta_hi) = (-b, -a);
        assert!(
            delta_lo < bias && bias < delta_hi,
            "{key}: measured bias {bias:+.4} ppm is not inside the delta window \
             {delta_lo:+.4}..{delta_hi:+.4} the config produces"
        );

        // ...and it must not be hugging a wall. The window is a rung centred on
        // the bias, so the bias sits at the MIDPOINT to within float noise.
        let midpoint = (delta_lo + delta_hi) / 2.0;
        assert!(
            (midpoint - bias).abs() < 1e-9,
            "{key}: window midpoint {midpoint:+.6} is not the measured bias \
             {bias:+.6} — the window is no longer bias-centred"
        );
    }
    if checked == 0 {
        eprintln!("skipped: no committed open searches on this machine");
    }
}

/// The Pass-2 MS2 tolerance must never widen past what Pass 1 actually searched.
///
/// Pass 1 defined the search space. A wider Pass-2 fragment window would claim
/// matches Pass 1 could not have made, which would make Pass 2's digestion rate
/// depend on a window nothing measured.
#[test]
fn the_pass2_ms2_tolerance_never_exceeds_pass1_on_real_measurements() {
    let mut checked = 0;
    for key in ["serum", "bcell", "b1906"] {
        let tsv = open_tsv(key);
        if !tsv.exists() {
            continue;
        }
        checked += 1;
        let results = parse_sage_results(&tsv, &FilterOptions::default()).unwrap();
        let summaries: Vec<PsmSummary> = results.psms.iter().map(PsmSummary::from_psm).collect();
        let subset = select_clean_subset(&summaries, 0.02, 0.01, false, 0);
        let mut fp: Vec<f64> = subset.iter().map(|p| p.fragment_ppm).collect();
        fp.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = fp[fp.len() / 2];

        // The real Orbitrap pass-1 window for these files.
        let pass1 = FragmentTolerance::Ppm(50.0);
        let got = ms2_pass2_tolerance(median, pass1);
        let FragmentTolerance::Ppm(w) = got else {
            panic!("{key}: a ppm pass-1 must yield a ppm pass-2, got {got}");
        };
        println!("{key}: MS2 median {median:.4} ppm -> pass-2 {got} (pass-1 ±50 ppm)");
        assert!(
            w <= 50.0,
            "{key}: pass-2 MS2 window {w} ppm exceeds the pass-1 window it was clamped to"
        );
        assert!(w > 0.0, "{key}: pass-2 MS2 window collapsed to zero");
    }
    if checked == 0 {
        eprintln!("skipped: no committed open searches on this machine");
    }
}
