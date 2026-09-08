//! Acceptance test: MS2 analyzer detection, the tolerance bucket it selects, and
//! the override that puts that tolerance into the search.
//!
//! THE REFERENCE IS NOT THIS TOOL. The expected analyzer was produced by
//! MSFragger, which read the original Thermo RAW files — not the mzML this code
//! parses. Its log states, per file:
//!
//!   2019-4-9_909c_0311.raw:     Scans = 41788; MS2 ITMS = false; MS2 FTMS = true
//!   b1906_..._122212.raw:       Scans = 41820; MS2 ITMS = false; MS2 FTMS = true
//!   B_naive_01steady-state.raw: Scans = 92304; MS2 ITMS = false; MS2 FTMS = true
//!
//! Source: _dev/testing/reference-data/msfragger/strictTryp/log_2026-08-24_13-19-52.txt
//! That makes these real inputs whose correct answer is known independently,
//! which is what AGENTS.md requires of a regression case — a synthetic fixture
//! could not catch a wrong convention here.
//!
//! Detection SAMPLES the head of each run rather than counting every scan, so the
//! MSFragger scan totals are not asserted here; they are already tripwired against
//! `get_mzml_stats` by the committed full-run output and by `run_validation.py`.
//!
//! ⚠ WHAT THIS FILE CANNOT TEST. All three reference files are FTMS at both MS
//! levels. NOTHING here exercises the ion-trap, TOF or Astral branches on real
//! data. Those tests are bucket-level only: they pin the UNIT and the bucket
//! boundaries, which are properties of the code, not claims about a measured
//! instrument.

use recon_tool::{
    bucket_tolerance, class_from_accession, class_from_filter_string, detect_analyzers,
    resolve_ms2_tolerance, AnalyzerClass, FragmentTolerance, ToleranceBasis,
    ION_TRAP_MS2_HALF_WIDTH_DA, LEGACY_TOF_MS2_HALF_WIDTH_PPM, ORBITRAP_MS2_HALF_WIDTH_PPM,
    UNKNOWN_MS2_FALLBACK_PPM,
};
use std::path::{Path, PathBuf};

fn repo() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

/// The mzML files are gitignored, so every test that needs one skips cleanly
/// when it is absent rather than failing on a machine without the data.
fn mzml(name: &str) -> PathBuf {
    repo().join("_dev/testing/inputs").join(name)
}

const REFERENCE_FILES: [(&str, &str); 3] = [
    ("2019-4-9_909c_0311.mzML.gz", "Orbitrap Fusion Lumos"),
    ("b1906_293T_proteinID_01A_QE3_122212.mzML.gz", "Q Exactive"),
    ("B.naive_01steady-state.mzML.gz", "Q Exactive Plus"),
];

/// A census with no scans counted, for exercising the decision logic.
fn stub_census() -> recon_tool::AnalyzerCensus {
    recon_tool::AnalyzerCensus {
        file_path: "<stub>".to_string(),
        instrument_model: None,
        declared_analyzers: vec![],
        detection_source: "<stub>".to_string(),
        ms1: Default::default(),
        ms2: Default::default(),
        sample_limit: 100,
        spectra_examined: 0,
    }
}

fn census_with_ms2(class: AnalyzerClass) -> recon_tool::AnalyzerCensus {
    let mut c = stub_census();
    match class {
        AnalyzerClass::Orbitrap => c.ms2.orbitrap = 100,
        AnalyzerClass::AstralTof => c.ms2.astral_tof = 100,
        AnalyzerClass::LegacyTof => c.ms2.legacy_tof = 100,
        AnalyzerClass::IonTrap => c.ms2.ion_trap = 100,
        AnalyzerClass::Unclassified => c.ms2.unclassified = 100,
    }
    c
}

// ---------------------------------------------------------------------------
// Real data
// ---------------------------------------------------------------------------

#[test]
fn ms2_analyzer_agrees_with_msfragger_on_every_reference_file() {
    let mut checked = 0;
    for (name, model) in REFERENCE_FILES {
        let path = mzml(name);
        if !path.exists() {
            eprintln!("skipping: {} absent", path.display());
            continue;
        }
        let census = detect_analyzers(&path).expect("mzML must parse");

        assert_eq!(
            census.instrument_model.as_deref(),
            Some(model),
            "{name}: model"
        );
        assert_eq!(
            census.ms2.dominant().map(|(c, _)| c),
            Some(AnalyzerClass::Orbitrap),
            "{name}: MSFragger says MS2 FTMS = true, MS2 ITMS = false"
        );
        assert_eq!(
            census.ms2.ion_trap, 0,
            "{name}: MSFragger says MS2 ITMS = false, so no sampled scan may be ion trap"
        );
        assert_eq!(census.ms2.unknown, 0, "{name}: unclassified MS2 scans");
        assert_eq!(census.ms1.unknown, 0, "{name}: unclassified MS1 scans");
        assert!(
            !census.ms2_switched(),
            "{name}: MS2 detector must not switch"
        );
        assert_eq!(
            census.ms2.total(),
            census.sample_limit,
            "{name}: MS2 sample should fill"
        );

        // Detection must stop early. The bound is deliberately loose rather than
        // a tight multiple of the sample: a method may sit on MS1 for a long
        // stretch at the head of a run and detection must keep reading until it
        // has its MS2 scans. In 2019-4-9_909c_0311 the first MS2 is at spectrum
        // 140 and 2063 MS1 scans precede the 100th MS2. These files hold
        // 56k-105k spectra, so under a few thousand proves the early exit.
        assert!(
            census.spectra_examined < 5_000,
            "{name}: read {} scans; should stop once the MS2 sample is full",
            census.spectra_examined
        );

        let d = census.ms2_decision();
        assert_eq!(
            d.basis,
            ToleranceBasis::Detected,
            "{name}: analyzer was detected"
        );
        assert!(!d.assumed, "{name}: nothing should be assumed here");
        assert_eq!(
            d.tolerance,
            FragmentTolerance::Ppm(ORBITRAP_MS2_HALF_WIDTH_PPM),
            "{name}: Orbitrap bucket"
        );
        checked += 1;
    }
    eprintln!(
        "checked {checked} of {} reference files",
        REFERENCE_FILES.len()
    );
}

/// THE INVARIANT the Orbitrap number has to satisfy: the pass-1 window must
/// contain the MS2 error actually measured on this data, with room to spare.
///
/// Values are MSFragger's PRE-calibration ("Old") MS2 median and MAD, in ppm.
/// Pre-calibration is the right column: pass 1 has not been calibrated yet, and
/// the whole point of a loose pass-1 window is to tolerate a poorly calibrated
/// instrument.
#[test]
fn orbitrap_tolerance_contains_the_measured_ms2_error() {
    let measured = [
        ("serum (run 001)", 0.96_f64, 1.33_f64),
        ("bcell (run 002)", 0.96, 2.49),
        ("b1906 (run 003)", -0.05, 1.62),
    ];
    let mut worst = 0.0_f64;
    for (label, median, mad) in measured {
        let needed = median.abs() + 3.0 * mad;
        assert!(
            needed < ORBITRAP_MS2_HALF_WIDTH_PPM,
            "{label}: needs ±{needed:.2} ppm, window is ±{ORBITRAP_MS2_HALF_WIDTH_PPM} ppm"
        );
        worst = worst.max(needed);
    }
    assert!((worst - 8.43).abs() < 0.01, "worst case moved: {worst:.2}");
    eprintln!(
        "worst measured MS2 need = {worst:.2} ppm; window ±{ORBITRAP_MS2_HALF_WIDTH_PPM} ppm; \
         headroom {:.1}x",
        ORBITRAP_MS2_HALF_WIDTH_PPM / worst
    );
}

// ---------------------------------------------------------------------------
// Buckets
// ---------------------------------------------------------------------------

/// The single rule this feature exists to enforce: an ion trap never gets a ppm
/// fragment tolerance.
#[test]
fn ion_trap_never_gets_a_ppm_tolerance() {
    match bucket_tolerance(AnalyzerClass::IonTrap) {
        Some(FragmentTolerance::Da(v)) => {
            assert!((v - ION_TRAP_MS2_HALF_WIDTH_DA).abs() < f64::EPSILON);
            assert!(v > 0.0);
        }
        other => panic!("ion trap must take daltons, got {other:?}"),
    }
}

/// Astral is a CV CHILD of time-of-flight (MS:1003379 is_a MS:1000084), but it
/// performs like an Orbitrap, not like a legacy QTOF. Bucketing by CV parentage
/// alone would give it the much looser legacy-TOF window. Most-specific wins.
#[test]
fn astral_is_not_bucketed_with_legacy_tof() {
    let astral = class_from_accession("MS:1003379").expect("Astral is in the table");
    let tof = class_from_accession("MS:1000084").expect("TOF is in the table");
    assert_ne!(astral, tof, "Astral must have its own bucket");
    assert_eq!(
        bucket_tolerance(astral),
        Some(FragmentTolerance::Ppm(ORBITRAP_MS2_HALF_WIDTH_PPM)),
        "Astral takes the Orbitrap-class window"
    );
    assert_eq!(
        bucket_tolerance(tof),
        Some(FragmentTolerance::Ppm(LEGACY_TOF_MS2_HALF_WIDTH_PPM)),
        "legacy TOF is looser"
    );
    // A `const` block, so the ordering of the two curated constants is checked
    // when the crate COMPILES rather than when this test runs. Same claim,
    // enforced earlier and unconditionally: it can no longer be skipped by
    // filtering tests, and it cannot silently stop being collected.
    const {
        assert!(LEGACY_TOF_MS2_HALF_WIDTH_PPM > ORBITRAP_MS2_HALF_WIDTH_PPM);
    }
}

/// The MS1 recommendation ladder is {10,20,50,100} ppm and is MS1-only. A
/// low-resolution MS2 must never receive a ppm rung — the exact merge AGENTS.md
/// forbids.
#[test]
fn the_ms1_ppm_ladder_never_reaches_a_low_res_ms2() {
    assert!(matches!(
        bucket_tolerance(AnalyzerClass::IonTrap),
        Some(FragmentTolerance::Da(_))
    ));
}

/// Every ion-trap term in the PSI-MS CV must classify as an ion trap — including
/// the generic parent MS:1000264, which is the ONLY one ThermoRawFileParser
/// writes for `MassAnalyzerITMS`, and MS:1000291, which the curated CSV also
/// omits. A table listing just the specific children would miss every
/// Thermo-converted ion-trap file.
#[test]
fn every_cv_ion_trap_term_classifies_as_ion_trap() {
    for acc in [
        "MS:1000264", // ion trap — what ThermoRawFileParser actually emits
        "MS:1000082", // quadrupole ion trap
        "MS:1000291", // linear ion trap
        "MS:1000078", // axial ejection linear ion trap
        "MS:1000083", // radial ejection linear ion trap
    ] {
        assert_eq!(
            class_from_accession(acc),
            Some(AnalyzerClass::IonTrap),
            "{acc} must be an ion trap"
        );
    }
}

/// Both high-res FT terms share a bucket. This is what makes the
/// ThermoRawFileParser <1.4.4 FTICR mislabel harmless: bcell is a Q Exactive Plus
/// whose mzML declares MS:1000079, and the tolerance must come out the same as if
/// it had correctly declared MS:1000484.
#[test]
fn the_thermo_fticr_mislabel_cannot_change_the_tolerance() {
    let orbitrap = class_from_accession("MS:1000484").expect("orbitrap");
    let fticr = class_from_accession("MS:1000079").expect("FT-ICR");
    assert_eq!(orbitrap, fticr);
    assert_eq!(bucket_tolerance(orbitrap), bucket_tolerance(fticr));
}

#[test]
fn thermo_filter_tokens_map_to_the_right_bucket() {
    let cases = [
        (
            "FTMS + p NSI Full ms [375.0000-1500.0000]",
            AnalyzerClass::Orbitrap,
        ),
        (
            "ITMS + c NSI d Full ms2 415.04@cid35.00 [110.0000-841.0000]",
            AnalyzerClass::IonTrap,
        ),
        ("TOFMS + p ESI Full ms", AnalyzerClass::LegacyTof),
        ("ASTMS + p NSI Full ms2", AnalyzerClass::AstralTof),
    ];
    for (filter, expect) in cases {
        assert_eq!(class_from_filter_string(filter), Some(expect), "{filter:?}");
    }
    assert_eq!(class_from_filter_string(""), None);
    assert_eq!(class_from_filter_string("NOTANANALYZER + p"), None);
}

// ---------------------------------------------------------------------------
// Recon never refuses to search
// ---------------------------------------------------------------------------

/// A run that swaps MS2 detector part-way has no single correct tolerance. recon
/// does NOT halt: it falls back, flags the assumption, and reports both detectors
/// so the reader can see what it did.
#[test]
fn a_switching_ms2_detector_falls_back_and_reports() {
    let mut census = census_with_ms2(AnalyzerClass::Orbitrap);
    census.ms2.ion_trap = 40;
    assert!(census.ms2_switched());

    let d = resolve_ms2_tolerance(&census);
    assert_eq!(d.basis, ToleranceBasis::DetectorSwitched);
    assert_eq!(
        d.tolerance,
        FragmentTolerance::Ppm(UNKNOWN_MS2_FALLBACK_PPM)
    );
    assert!(d.assumed, "a switch must be flagged as an assumption");
    assert!(d.explanation.contains("CHANGES"), "{}", d.explanation);
    // Both detectors must be named in the decision, for the report.
    assert_eq!(
        d.ms2_analyzers.len(),
        2,
        "both detectors reported: {:?}",
        d.ms2_analyzers
    );
}

/// An analyzer with no bucket, and a file with no readable analyzer, both fall
/// back rather than halting.
#[test]
fn an_unknown_analyzer_falls_back_and_reports() {
    let d = resolve_ms2_tolerance(&census_with_ms2(AnalyzerClass::Unclassified));
    assert_eq!(d.basis, ToleranceBasis::Unknown);
    assert_eq!(
        d.tolerance,
        FragmentTolerance::Ppm(UNKNOWN_MS2_FALLBACK_PPM)
    );
    assert!(d.assumed);

    let empty = resolve_ms2_tolerance(&stub_census());
    assert_eq!(empty.basis, ToleranceBasis::Unknown);
    assert_eq!(
        empty.tolerance,
        FragmentTolerance::Ppm(UNKNOWN_MS2_FALLBACK_PPM)
    );
    assert!(empty.assumed);
}

/// The blanket guarantee: whatever the census, a tolerance comes out. Analyzer
/// detection can never be the reason a search does not run.
#[test]
fn recon_never_refuses_to_search() {
    for class in [
        AnalyzerClass::Orbitrap,
        AnalyzerClass::AstralTof,
        AnalyzerClass::LegacyTof,
        AnalyzerClass::IonTrap,
        AnalyzerClass::Unclassified,
    ] {
        let d = resolve_ms2_tolerance(&census_with_ms2(class));
        match d.tolerance {
            FragmentTolerance::Ppm(v) | FragmentTolerance::Da(v) => {
                assert!(v > 0.0, "{class:?} yielded a non-positive tolerance");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The override
// ---------------------------------------------------------------------------

/// Write the pass-1 effective params THE WAY `main.rs` DOES: template TEXT,
/// then `enzyme::apply_to_params_text`, then `write_effective_params_from_text`
/// (`main.rs:2501-2506`).
///
/// ⚠ Until 2026-09-02 these tests called `write_effective_params`, the
/// path-taking wrapper, which NOTHING in the shipped binary calls — so the
/// fragment-tolerance override was proved on params production never writes.
/// The enzyme step sits BETWEEN reading the template and writing the effective
/// config, so it is part of the route or the route is not the real one. Trypsin
/// is the enzyme every pinned number in this repo was produced with.
fn write_pass1(
    template: &Path,
    decision: &recon_tool::mzml::Ms2TolDecision,
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

fn template_with(dir: &Path, unit: &str, lo: f64, hi: f64, tag: &str) -> PathBuf {
    let path = dir.join(format!("recon_tmpl_{tag}.json"));
    std::fs::write(
        &path,
        // isotope_errors [0,0] is REQUIRED of a pass-1 template since 2026-09-01
        // — an open search must not let Sage match a neighbouring isotope. See
        // no_mods_guard.rs.
        format!(
            r#"{{"database":{{"fasta":"x.fasta"}},"fragment_tol":{{"{unit}":[{lo},{hi}]}},"isotope_errors":[0,0]}}"#
        ),
    )
    .expect("temp template must write");
    path
}

/// THE CASE THIS FEATURE EXISTS FOR. Every pass-1 template hardcodes
/// ±20 ppm regardless of instrument. Pointed at ion-trap MS2 that is ~30x too
/// tight, so the search matches almost nothing. The override must replace it with
/// daltons BEFORE the search — not report it afterwards.
#[test]
fn a_ppm_template_is_overridden_to_daltons_for_ion_trap_ms2() {
    let dir = tempfile::tempdir().expect("tempdir");
    let tmpl = template_with(dir.path(), "ppm", -20.0, 20.0, "trap");
    let out = dir.path().join("recon_eff_trap");
    let decision = resolve_ms2_tolerance(&census_with_ms2(AnalyzerClass::IonTrap));
    let eff = write_pass1(
        &tmpl,
        &decision,
        &out,
        &PathBuf::from("/tmp/db.fasta"),
        &[PathBuf::from("/tmp/x.mzML.gz")],
    )
    .expect("override must write");

    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&eff).unwrap()).unwrap();
    let frag = &json["fragment_tol"];
    assert!(frag.get("da").is_some(), "must be daltons, got {frag}");
    assert!(
        frag.get("ppm").is_none(),
        "the ppm key must be gone, got {frag}"
    );
    assert_eq!(frag["da"][1].as_f64().unwrap(), ION_TRAP_MS2_HALF_WIDTH_DA);

    // The artifact must explain itself.
    let prov = &json["_recon_fragment_tol_provenance"];
    assert_eq!(
        prov["template_fragment_tol"]["ppm"][1].as_f64().unwrap(),
        20.0
    );
    assert_eq!(prov["basis"], "Detected");
}

/// And the Orbitrap case must be widened from the template's 20 ppm to 50 ppm —
/// the override is not a no-op just because the unit already matched.
#[test]
fn a_ppm_template_is_widened_for_orbitrap_ms2() {
    let dir = tempfile::tempdir().expect("tempdir");
    let tmpl = template_with(dir.path(), "ppm", -20.0, 20.0, "orb");
    let out = dir.path().join("recon_eff_orb");
    let decision = resolve_ms2_tolerance(&census_with_ms2(AnalyzerClass::Orbitrap));
    let eff = write_pass1(
        &tmpl,
        &decision,
        &out,
        &PathBuf::from("/tmp/db.fasta"),
        &[PathBuf::from("/tmp/x.mzML.gz")],
    )
    .expect("override must write");

    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&eff).unwrap()).unwrap();
    assert_eq!(
        json["fragment_tol"]["ppm"][1].as_f64().unwrap(),
        ORBITRAP_MS2_HALF_WIDTH_PPM,
        "the hardcoded 20 ppm must be replaced, not kept"
    );
}

/// The override applied to the REAL templates and the REAL files. Every
/// open-search template carries ppm ±20; all three files are Orbitrap MS2; so
/// every pair must come out at the Orbitrap window.
#[test]
fn every_real_template_is_overridden_on_every_real_file() {
    let configs = repo().join("_dev/testing/configs");
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join("recon_eff_real");
    let mut pairs = 0;
    for (name, _) in REFERENCE_FILES {
        let path = mzml(name);
        if !path.exists() {
            eprintln!("skipping: {} absent", path.display());
            continue;
        }
        let decision = detect_analyzers(&path)
            .expect("mzML must parse")
            .ms2_decision();
        for entry in std::fs::read_dir(&configs).expect("configs dir") {
            let cfg = entry.expect("dir entry").path();
            let fname = cfg.file_name().unwrap().to_string_lossy().to_string();
            if !fname.starts_with("open-search-") || !fname.ends_with(".json") {
                continue;
            }
            let eff = write_pass1(
                &cfg,
                &decision,
                &out,
                &PathBuf::from("/tmp/db.fasta"),
                std::slice::from_ref(&path),
            )
            .unwrap_or_else(|e| panic!("{fname} vs {name}: {e}"));
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&eff).unwrap()).unwrap();
            assert_eq!(
                json["fragment_tol"]["ppm"][1].as_f64().unwrap(),
                ORBITRAP_MS2_HALF_WIDTH_PPM,
                "{fname} vs {name}"
            );
            // The enzyme step runs before this writer in production. Assert it
            // landed, so the route cannot silently revert to the wrapper.
            // `c_terminal` is the falsifying field: these templates already carry
            // cleave_at and restrict, but none of them carries c_terminal.
            assert_eq!(
                json["database"]["enzyme"]["c_terminal"], true,
                "{fname} vs {name}: the enzyme step did not reach the effective params"
            );
            pairs += 1;
        }
    }
    eprintln!("{pairs} template/file pairs overridden");
}
