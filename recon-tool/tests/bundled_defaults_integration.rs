//! THE PACKAGING INVARIANT: nothing `recon` needs at runtime lives at a path
//! relative to the working directory.
//!
//! Before the bundling, three resources were read from repo-relative paths —
//! `_dev/testing/configs/open-search-params.json`, `_dev/testing/configs/digestion-efficiency-pass2.json`
//! and `recon-tool/resources/mods/`. A user who unzipped a release and ran
//! it on their own data got an error naming a repo directory they do not have.
//!
//! WHAT THESE TESTS PROVE, PRECISELY. Each of the three resources now comes from
//! the compiled-in copy, and the compiled-in copy produces the SAME RESULT as the
//! on-disk one it replaced. That is an equivalence claim, and equivalence is what
//! makes the bundling safe: the shipped tool and the tested tool stay one tool.
//!
//! ⚠ WHAT THEY DO NOT PROVE. They do not run a search. A full end-to-end run from
//! a foreign working directory needs Sage, an mzML and minutes, and it is covered
//! by the normal `run` path. These are the unit-level property behind it, not a
//! substitute for it. Stated so nobody reads a green tick as more than it is.

use recon_tool::calibration::Ms1Pass2Window;
use recon_tool::curated_mods::CuratedDb;
use recon_tool::defaults;
use recon_tool::mzml::{FragmentTolerance, Ms2TolDecision, ToleranceBasis};
use recon_tool::pass2::Pass2Plan;
use recon_tool::unimod::UnimodDb;
use std::path::{Path, PathBuf};

fn decision() -> Ms2TolDecision {
    Ms2TolDecision {
        tolerance: FragmentTolerance::Ppm(20.0),
        basis: ToleranceBasis::Detected,
        detected: None,
        ms2_analyzers: vec!["orbitrap".to_string()],
        ms1_analyzers: vec!["orbitrap".to_string()],
        assumed: false,
        explanation: "bundled-default packaging gate".to_string(),
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// The element table, from the COMMITTED `unimod.xml`.
///
/// ⚠ THIS MUST NOT SKIP. Until 2026-09-02 it returned `None` and the equivalence
/// test below returned early — silently green — when the file was absent. That
/// is the correct behaviour for the gitignored data (mzML, FASTA,
/// `_dev/testing/search-output`), which is genuinely not on every machine. It is the
/// WRONG behaviour here: `recon-tool/resources/unimod.xml` is committed, and
/// `.gitattributes` marks it `-text` precisely because a Windows checkout once
/// corrupted it. An absent or unreadable copy is a real error, and the test that
/// self-skips on it is the project's most important equivalence gate.
fn unimod() -> UnimodDb {
    let p = repo_root().join("recon-tool/resources/unimod.xml");
    assert!(
        p.exists(),
        "{} is COMMITTED and must be present. Its absence is a broken checkout, \
         not a machine difference — this test must not skip.",
        p.display()
    );
    UnimodDb::from_xml(&p)
        .unwrap_or_else(|e| panic!("the committed {} must parse: {e}", p.display()))
}

/// The curated list, loaded from the EMBEDDED text, must be identical to the same
/// list loaded from the directory `main.rs` used to read.
///
/// This is the equivalence that matters most. The curated list decides which
/// modifications can be recommended at all; an embed that silently parsed to
/// nothing would route every decision to abundance and still look like a working
/// tool.
///
/// ⚠ THIS IS THE ONLY REMAINING CALLER OF `CuratedDb::load`, and it is the
/// reason that function still exists. Every other test now goes through
/// `load_from_sources` on the embedded text, which is the route the shipped
/// binary takes. Comparing the two is the whole job of this test; anywhere else,
/// reading the directory would test a path production never runs.
#[test]
fn embedded_curated_list_equals_the_on_disk_list() {
    let u = unimod();
    const FILES: [&str; 4] = [
        "Mods.txt",
        "aListOfmods.txt",
        "ProteaseMods.txt",
        "surfactants.txt",
    ];

    let dir = repo_root().join("recon-tool/resources/mods");
    let (from_disk, skipped_disk) = CuratedDb::load(&dir, &FILES, u.elements()).unwrap();
    let (from_embed, skipped_embed) =
        CuratedDb::load_from_sources(defaults::CURATED_MODS, u.elements()).unwrap();

    assert_eq!(
        from_disk.len(),
        from_embed.len(),
        "embedded curated list has {} entries, the on-disk list has {}",
        from_embed.len(),
        from_disk.len()
    );
    assert_eq!(skipped_disk, skipped_embed, "skip counts differ");
    assert!(
        !from_embed.is_empty(),
        "the embedded curated list parsed to NOTHING"
    );

    // Same entries, not merely the same count. A reordering or a dropped field
    // would keep the count and change the decisions.
    let key = |d: &CuratedDb| -> Vec<String> {
        let mut v: Vec<String> = d
            .entries()
            .iter()
            .map(|e| format!("{}|{}|{}|{:.6}", e.id, e.position, e.category, e.mass))
            .collect();
        v.sort();
        v
    };
    assert_eq!(
        key(&from_disk),
        key(&from_embed),
        "embedded and on-disk entries differ"
    );
    println!(
        "curated list: {} entries, {} skipped — identical from both sources",
        from_embed.len(),
        skipped_embed
    );
}

/// The bundled pass-1 default must survive the real production path — every
/// guard it enforces, with no file on disk anywhere.
///
/// The path is `enzyme::apply_to_params_text` then
/// `write_effective_params_from_text`, which is what `main.rs:2501-2506` runs.
/// The enzyme half was added 2026-09-02; before that this gate skipped it and so
/// proved the guards against text production never writes.
#[test]
fn the_bundled_pass1_default_passes_every_runtime_guard() {
    let out = tempfile::tempdir().unwrap();
    let decision = decision();

    // The enzyme is applied to the template TEXT before the writer runs, exactly
    // as `main.rs:2501` does. Without this the gate would prove the bundled
    // default passes guards on text production never actually writes.
    let enzyme = recon_tool::enzyme::parse("trypsin").expect("the trypsin preset must parse");
    let text = recon_tool::enzyme::apply_to_params_text(defaults::OPEN_SEARCH, &enzyme)
        .expect("the enzyme must apply to the bundled pass-1 default");
    let written = recon_tool::sage_runner::write_effective_params_from_text(
        &text,
        &defaults::bundled_label("open-search"),
        &decision,
        out.path(),
        Path::new("/nowhere/db.fasta"),
        &[PathBuf::from("/nowhere/sample.mzML.gz")],
    )
    .expect("the bundled pass-1 default must pass the no-mods and isotope guards");

    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&written).unwrap()).unwrap();

    // The enzyme step landed. `c_terminal` falsifies: the bundled template
    // carries cleave_at and restrict already, but not c_terminal.
    assert_eq!(v["database"]["enzyme"]["c_terminal"], true);
    assert_eq!(v["database"]["enzyme"]["cleave_at"], "KR");
    // The mods guard.
    assert_eq!(v["database"]["static_mods"], serde_json::json!({}));
    assert_eq!(v["database"]["variable_mods"], serde_json::json!({}));
    // The isotope guard — the bundled default must not be what trips it.
    assert_eq!(v["isotope_errors"], serde_json::json!([0, 0]));
    // The run's real inputs are written, not the template's (it declares none).
    assert_eq!(v["database"]["fasta"], "/nowhere/db.fasta");
    assert_eq!(v["mzml_paths"][0], "/nowhere/sample.mzML.gz");
    // Provenance names the bundled default rather than a path that does not exist.
    let template = v["_recon_fragment_tol_provenance"]["template"]
        .as_str()
        .unwrap();
    assert!(
        template.contains("bundled"),
        "provenance should name the bundled default, got {template}"
    );
    println!(
        "effective params written from the bundled default: {}",
        written.display()
    );
}

/// Same for pass 2. Its extra guard is that the enzyme block must be
/// semi-enzymatic — a non-semi template would report "0 % semi-tryptic" as a
/// finding rather than as a misconfiguration.
#[test]
fn the_bundled_pass2_default_passes_every_runtime_guard() {
    let out = tempfile::tempdir().unwrap();
    let subset = out.path().join("subset.fasta");
    std::fs::write(&subset, ">sp|X|X\nPEPTIDEK\n").unwrap();

    let plan = Pass2Plan {
        ms1_window: Ms1Pass2Window {
            low_ppm: -7.5785,
            high_ppm: 12.4215,
        },
        ms2_tolerance: Some(FragmentTolerance::Ppm(5.6)),
        pass1_fragment_tol: FragmentTolerance::Ppm(20.0),
        subset_fasta: subset.clone(),
        subset_proteins: 1,
    };
    let enzyme = recon_tool::enzyme::parse("trypsin").expect("the trypsin preset must parse");
    let text = recon_tool::enzyme::apply_to_params_text(defaults::PASS2, &enzyme)
        .expect("the enzyme must apply to the bundled pass-2 default");
    let written = recon_tool::pass2::write_pass2_params_from_text(
        &text,
        &defaults::bundled_label("pass-2"),
        &plan,
        out.path(),
    )
    .expect("the bundled pass-2 default must pass the semi-enzymatic and no-mods guards");

    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&written).unwrap()).unwrap();
    assert_eq!(
        v["database"]["enzyme"]["semi_enzymatic"],
        serde_json::json!(true),
        "applying the enzyme must not drop semi_enzymatic"
    );
    assert_eq!(v["database"]["enzyme"]["c_terminal"], true);
    assert_eq!(v["database"]["static_mods"], serde_json::json!({}));
    assert_eq!(v["database"]["variable_mods"], serde_json::json!({}));
    assert_eq!(v["database"]["fasta"], subset.display().to_string());
    println!(
        "pass-2 effective params written from the bundled default: {}",
        written.display()
    );
}
