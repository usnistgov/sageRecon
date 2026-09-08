//! NEITHER PASS EVER SEARCHES WITH A MODIFICATION.
//!
//! This is the product claim, not a preference: an open search that ASSUMES
//! carbamidomethyl cannot report whether the sample was alkylated, or with
//! what. MEASURED on bcell 2026-08-31 — with `static_mods {C: 57.0215}` the
//! +57.02 peak reads n=146; without it, n=3311, and it is the TOP peak in the
//! file. A fixed Cys mod does not hide a peak, it deletes the largest signal
//! recon exists to surface.
//!
//! `write_effective_params_from_text` and `write_pass2_params_from_text` both
//! strip the mods and then assert the result — these are the functions the
//! shipped binary calls. **Until 2026-09-01 nothing tested either guard.**
//! A guard that has never been seen to fire is not a guard, and the failure it
//! protects against is silent: a fixed-C search does not error, it reports
//! "no alkylation" on an alkylated sample.
//!
//! THE TEST IS BUILT SO IT CANNOT PASS ON A CLEAN TEMPLATE. It feeds a REAL
//! committed template that carries `static_mods {C: 57.0215}`, and asserts the
//! provenance block recorded that dirty input. If someone cleans the template,
//! this test fails and says so, rather than passing vacuously.
//!
//! It also covers `variable_mods`, which BOTH guards strip but NEITHER asserts
//! (found 2026-09-01). That asymmetry is the live hole: deleting the
//! `variable_mods` insert would leave both asserts green.
//!
//! ⚠ IT NOW DRIVES THE PRODUCTION ROUTE, 2026-09-02. Until then every test here
//! called `write_effective_params`, the path-taking wrapper, which NOTHING in
//! the shipped binary calls. `main.rs` reads the template to text, applies the
//! enzyme to that text (`main.rs:2501`), and only then calls
//! `write_effective_params_from_text` (`main.rs:2502`). So the guards were
//! proved against params production never writes — the same split that shipped
//! v0.1.0 with no recommendations. `write_pass1` below is that exact sequence.
//! The guard is not weakened by the change: every assertion is kept, and the
//! enzyme block the extra step writes is now asserted too.

use recon_tool::mzml::{FragmentTolerance, Ms2TolDecision, ToleranceBasis};
use std::path::{Path, PathBuf};

/// A real committed template that carries a fixed Cys mod. Chosen on purpose:
/// a synthetic fixture would inherit the assumptions of the code it tests.
/// Real committed config with a fixed Cys mod and a VALID open isotope window.
const DIRTY_STATIC_TEMPLATE: &str = "_dev/testing/configs/open-search-b1906.json";
/// Real committed config carrying a non-zero isotope window.
const BAD_ISOTOPE_TEMPLATE: &str = "_dev/testing/configs/closed-search-reference.json";
/// The one synthetic fixture — see its own `_fixture_note`.
const DIRTY_MODS_FIXTURE: &str = "recon-tool/tests/fixtures/dirty-mods-open-template.json";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("recon-tool has a parent")
        .to_path_buf()
}

fn decision() -> Ms2TolDecision {
    Ms2TolDecision {
        tolerance: FragmentTolerance::Ppm(20.0),
        basis: ToleranceBasis::Detected,
        detected: None,
        ms2_analyzers: vec!["orbitrap".to_string()],
        ms1_analyzers: vec!["orbitrap".to_string()],
        assumed: false,
        explanation: "test".to_string(),
    }
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("effective params written"))
        .expect("effective params parse")
}

/// Write the pass-1 effective params THE WAY `main.rs` DOES.
///
/// Three steps, in this order, copied from `main.rs:2437-2506`:
///   1. resolve the template to TEXT (a `--params` file here; the bundled
///      default in the no-flag case, which `bundled_defaults_integration.rs`
///      covers),
///   2. `enzyme::apply_to_params_text`,
///   3. `sage_runner::write_effective_params_from_text`.
///
/// Step 2 is the one the old wrapper route skipped. `--enzyme` is required and
/// has no default in production, so a test has to pick one: trypsin, which is
/// what every pinned number in this repo was produced with.
fn write_pass1(
    template: &Path,
    decision: &Ms2TolDecision,
    out_dir: &Path,
    fasta: &Path,
    mzml_paths: &[PathBuf],
) -> anyhow::Result<PathBuf> {
    let text = std::fs::read_to_string(template)
        .unwrap_or_else(|e| panic!("failed to read template {}: {e}", template.display()));
    let enzyme = recon_tool::enzyme::parse("trypsin").expect("the trypsin preset must parse");
    let text = recon_tool::enzyme::apply_to_params_text(&text, &enzyme)?;
    recon_tool::sage_runner::write_effective_params_from_text(
        &text,
        &template.display().to_string(),
        decision,
        out_dir,
        fasta,
        mzml_paths,
    )
}

/// The enzyme step must be VISIBLE in the artifact, or `write_pass1` is not
/// really the production route and the tests here drift back to the old one.
///
/// ⚠ `c_terminal` IS THE FALSIFYING FIELD. Every committed template already
/// carries `cleave_at: "KR"` and `restrict: "P"`, so asserting those would pass
/// with the enzyme step deleted. NO template carries `c_terminal` —
/// `apply_to_params_text` is the only thing that writes it. Check that this
/// stays true before trusting this helper.
fn assert_enzyme_was_applied(template: &serde_json::Value, effective: &serde_json::Value) {
    assert!(
        template["database"]["enzyme"]["c_terminal"].is_null(),
        "the template now carries c_terminal, so this check is vacuous. Pick a \
         field only the enzyme step writes."
    );
    let enz = &effective["database"]["enzyme"];
    assert_eq!(
        enz["c_terminal"], true,
        "the enzyme step did not reach the effective params; got {enz}"
    );
    assert_eq!(enz["cleave_at"], "KR", "trypsin cuts at K and R: {enz}");
    assert_eq!(enz["restrict"], "P", "the proline rule is missing: {enz}");
}

#[test]
fn pass1_strips_a_fixed_cys_mod_from_a_real_dirty_template() {
    let root = repo_root();
    let template = root.join(DIRTY_STATIC_TEMPLATE);

    // Guard the guard: if this template ever loses its fixed mod, the rest of
    // this test proves nothing. Fail loudly instead of passing vacuously.
    let before = read_json(&template);
    let before_static = before["database"]["static_mods"]
        .as_object()
        .expect("template has a static_mods object");
    let before_variable = before["database"]["variable_mods"]
        .as_object()
        .expect("template has a variable_mods object");
    assert!(
        !before_static.is_empty(),
        "{DIRTY_STATIC_TEMPLATE} no longer carries a fixed mod, so this test \
         cannot demonstrate anything. Point it at a template that still does."
    );
    let _ = &before_variable;

    let out = tempfile::tempdir().expect("tempdir");
    let real_fasta = PathBuf::from("/tmp/the-database-actually-searched.fasta");
    let real_mzml = vec![PathBuf::from("/tmp/actually-searched.mzML.gz")];
    let written = write_pass1(&template, &decision(), out.path(), &real_fasta, &real_mzml)
        .expect("pass 1 params written");
    let effective = read_json(&written);

    let static_mods = effective["database"]["static_mods"]
        .as_object()
        .expect("static_mods present");
    let variable_mods = effective["database"]["variable_mods"]
        .as_object()
        .expect("variable_mods present");

    assert!(
        static_mods.is_empty(),
        "pass 1 would search with static mods {static_mods:?} — the alkylation-\
         agnostic claim is broken"
    );
    // NOT covered by either guard's assert. This is the point of the test.
    assert!(
        variable_mods.is_empty(),
        "pass 1 would search with variable mods {variable_mods:?}"
    );
    assert_eq!(
        effective["database"]["max_variable_mods"], 0,
        "max_variable_mods must be 0"
    );

    // Proves the dirty input actually reached the function.
    let recorded = &effective["_recon_fragment_tol_provenance"]["template_static_mods"];
    assert_eq!(
        recorded["C"], 57.0215,
        "provenance must record the fixed mod that was stripped; got {recorded:?}"
    );

    // And proves the enzyme step production runs first was not skipped. Applying
    // an enzyme must not put a mod back either — that is a new claim this route
    // can make and the old one could not.
    assert_enzyme_was_applied(&before, &effective);
}

/// The effective config must name the database that WAS searched.
///
/// ⚠ REGRESSION CASE, 2026-09-01. This file used to copy the template's
/// `database.fasta`, while Sage took the real one from `-f`. A committed liver
/// artifact therefore claimed the 2023 canonical FASTA for a search run against
/// the 2018 one, and a session believed the claim. The template used here names
/// a DIFFERENT database from the one passed, so the old behaviour fails this.
#[test]
fn pass1_records_the_fasta_that_was_actually_searched() {
    let root = repo_root();
    let template = root.join(DIRTY_STATIC_TEMPLATE);
    let template_fasta = read_json(&template)["database"]["fasta"]
        .as_str()
        .expect("template names a fasta")
        .to_string();

    let real_fasta = PathBuf::from("/tmp/the-database-actually-searched.fasta");
    assert_ne!(
        template_fasta,
        real_fasta.display().to_string(),
        "the test is only meaningful if the template names a DIFFERENT database"
    );

    let out = tempfile::tempdir().expect("tempdir");
    let written = write_pass1(
        &template,
        &decision(),
        out.path(),
        &real_fasta,
        &[PathBuf::from("/tmp/actually-searched.mzML.gz")],
    )
    .expect("params written");
    let effective = read_json(&written);

    assert_eq!(
        effective["database"]["fasta"], "/tmp/the-database-actually-searched.fasta",
        "effective config must name the SEARCHED database, not the template's"
    );
    assert_eq!(
        effective["mzml_paths"][0], "/tmp/actually-searched.mzML.gz",
        "effective config must name the SEARCHED spectra"
    );
    // The template's claim is kept, but clearly as the template's.
    assert_eq!(
        effective["_recon_fragment_tol_provenance"]["template_fasta"], template_fasta,
        "the template's own fasta must still be recorded, separately"
    );
}

/// An open search must not inherit a non-zero isotope window from its template.
///
/// ⚠ Pass 1 overrides `fragment_tol`, the mods, the FASTA and the mzML, but it
/// CANNOT override `isotope_errors` — that value is part of what makes the
/// search an open one, so it has to come from the template. That leaves it as a
/// live ghost, and NOTES records the measured damage: `[-1, 2]` cut the +57 peak
/// from 1253 to 394 and invented a Propionyl peak at +56.018.
///
/// `closed-search-reference.json` carries `[-1, 3]`, so it is a real committed
/// example of the bad value — not a synthetic one.
#[test]
fn pass1_refuses_a_template_with_a_non_zero_isotope_window() {
    let root = repo_root();
    let template = root.join(BAD_ISOTOPE_TEMPLATE);
    let iso = read_json(&template)["isotope_errors"].clone();
    assert_ne!(
        iso,
        serde_json::json!([0, 0]),
        "{BAD_ISOTOPE_TEMPLATE} no longer carries a non-zero isotope window, so \
         this test proves nothing. Point it at one that does."
    );

    let out = tempfile::tempdir().expect("tempdir");
    let err = write_pass1(
        &template,
        &decision(),
        out.path(),
        &PathBuf::from("/tmp/db.fasta"),
        &[PathBuf::from("/tmp/x.mzML.gz")],
    )
    .expect_err("a non-zero isotope window must be REFUSED, not searched");
    let msg = err.to_string();
    assert!(
        msg.contains("isotope_errors") && msg.contains("[0, 0]"),
        "the refusal must name the field and the required value; got: {msg}"
    );
}

/// The shipped default template must satisfy that guard.
#[test]
fn the_default_open_search_template_is_clean() {
    let root = repo_root();
    let template = root.join("_dev/testing/configs/open-search-params.json");
    let out = tempfile::tempdir().expect("tempdir");
    let written = write_pass1(
        &template,
        &decision(),
        out.path(),
        &PathBuf::from("/tmp/db.fasta"),
        &[PathBuf::from("/tmp/x.mzML.gz")],
    )
    .expect("the DEFAULT template must pass every pass-1 guard");
    let e = read_json(&written);
    assert_eq!(e["isotope_errors"], serde_json::json!([0, 0]));
    assert!(e["database"]["static_mods"].as_object().unwrap().is_empty());
    assert!(e["database"]["variable_mods"]
        .as_object()
        .unwrap()
        .is_empty());
    assert_enzyme_was_applied(&read_json(&template), &e);
}

/// The variable-mods half of the strip, which NO committed template can test.
///
/// ⚠ FALSIFIED 2026-09-01 and it mattered: the first version of the static-mods
/// test asserted variable_mods too, but used a template whose variable_mods were
/// already empty — so it passed with the strip deleted. This test uses a fixture
/// that actually carries them. Neither guard in the code asserts variable_mods;
/// this test is the only thing covering that half.
#[test]
fn pass1_strips_variable_mods_too() {
    let root = repo_root();
    let template = root.join(DIRTY_MODS_FIXTURE);
    let before = read_json(&template);
    assert!(
        !before["database"]["variable_mods"]
            .as_object()
            .unwrap()
            .is_empty(),
        "the fixture must carry variable mods or this test is vacuous"
    );

    let out = tempfile::tempdir().expect("tempdir");
    let written = write_pass1(
        &template,
        &decision(),
        out.path(),
        &PathBuf::from("/tmp/db.fasta"),
        &[PathBuf::from("/tmp/x.mzML.gz")],
    )
    .expect("params written");
    let e = read_json(&written);
    assert!(
        e["database"]["variable_mods"]
            .as_object()
            .unwrap()
            .is_empty(),
        "pass 1 would search with variable mods {:?}",
        e["database"]["variable_mods"]
    );
    assert_eq!(e["database"]["max_variable_mods"], 0);
}
