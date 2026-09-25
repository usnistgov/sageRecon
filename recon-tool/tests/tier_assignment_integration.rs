//! Acceptance test: the Rust must reproduce the Python prototype's recommendations
//! on real committed data.
//!
//! `_dev/testing/scripts/tier_report_prototype.py` is the reference. If these two
//! disagree, one of them is wrong and the report has silently moved. Values below
//! were produced by that script on b1906 and are pinned here deliberately.
//!
//! b1906 is used because it is the file that exercises BOTH paths: seven mods
//! decided by statistics, and Carbamyl decided by abundance because its acceptors
//! are unspecific.
//!
//! ⚠ THE CURATED LIST COMES FROM `defaults::CURATED_MODS`, THE EMBEDDED TEXT THE
//! SHIPPED BINARY USES (`main.rs` calls `CuratedDb::load_from_sources` on it).
//! Until 2026-09-02 these tests read `recon-tool/resources/mods/` from
//! disk through `CuratedDb::load`, a route production does not take — the exact
//! shape of the v0.1.0 bug, where the embedded list was tested and the disk list
//! was shipped, or the other way round. A test that does not exercise the
//! production route proves nothing. The disk-vs-embed equivalence itself is
//! still proved, once, in `bundled_defaults_integration.rs`.

use recon_tool::curated_mods::CuratedDb;
use recon_tool::protein_index::ProteinIndex;
use recon_tool::sage_results::{parse_sage_results, FilterOptions};
use recon_tool::tier_assignment::{assign, Decision};
use recon_tool::unimod::UnimodDb;
use std::path::Path;

fn repo() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

/// The search FASTA. Gitignored, so every test that needs it must skip cleanly
/// when it is absent rather than fail on a machine that does not carry the data.
fn fasta() -> std::path::PathBuf {
    repo().join("_dev/testing/inputs/UniProt-Human-UP000005640_canonical-2023_05.fasta")
}

/// The protein context, or `None` when the FASTA is not on this machine.
fn protein_index() -> Option<ProteinIndex> {
    let p = fasta();
    if !p.exists() {
        eprintln!("skipping protein context: {} absent", p.display());
        return None;
    }
    Some(ProteinIndex::from_fasta(&p).expect("FASTA must parse"))
}

/// Run the routing rule on one committed file. Returns the tiered peaks and the
/// rank-1 spectrum_q<0.01 population size, so each test can pin both.
fn run(file_key: &str) -> Option<(Vec<recon_tool::tier_assignment::TieredPeak>, usize)> {
    let tsv = repo().join(format!(
        "_dev/testing/search-output/step1-open-{file_key}/results.sage.tsv"
    ));
    if !tsv.exists() {
        eprintln!("skipping: {} absent", tsv.display());
        return None;
    }
    let opts = FilterOptions {
        q_threshold: 1.0,
        ..Default::default()
    };
    let results = parse_sage_results(&tsv, &opts).expect("TSV must parse");
    let psms: Vec<_> = results
        .psms
        .into_iter()
        .filter(|p| p.rank == 1 && p.spectrum_q < 0.01)
        .collect();
    let unimod = UnimodDb::from_xml(&repo().join("recon-tool/resources/unimod.xml")).unwrap();
    let (curated, _) =
        CuratedDb::load_from_sources(recon_tool::defaults::CURATED_MODS, unimod.elements())
            .unwrap();
    let report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo().join(format!(
            "_dev/testing/recon-output/full-run/{file_key}.json"
        )))
        .unwrap(),
    )
    .unwrap();
    let peaks: Vec<(f64, usize)> = report["mod_discovery"]["peaks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["delta_mass"].as_f64().unwrap(),
                p["count"].as_u64().unwrap() as usize,
            )
        })
        .filter(|(d, _)| d.abs() >= 0.1)
        .collect();
    let top = peaks.iter().map(|(_, c)| *c).max().unwrap() as f64;
    let n = psms.len();
    let index = protein_index();
    Some((
        assign(&peaks, &psms, &curated, top * 0.20, index.as_ref()),
        n,
    ))
}

/// Assert a q-derived population is close to its recorded value.
///
/// ⚠ AGENTS.md: "Never `assert_eq!` a q-derived count -- band it, and assert
/// the claim the number exists to support." These three counts were pinned with
/// `assert_eq!` until 2026-09-03 and violated that rule.
///
/// The jitter is measured, not theoretical. NOTES records bcell pass-1 PSMs
/// reading 72801, 72802 five times and 72803 over seven runs. Regenerating
/// `full-run/` on 2026-09-03 moved liver's pass-1 PSMs from 31782 to 31793.
/// Sage's rescoring is not bit-reproducible, so the q threshold lands on a
/// slightly different set each run.
///
/// The band is 0.5 %, comfortably above the ~0.1 % drift NOTES documents and far
/// below any change that would matter. The point of these counts is that the
/// POPULATION is the one the prototype measured, not that a specific integer
/// recurs.
fn assert_population_near(actual: usize, recorded: usize, label: &str) {
    let tol = (recorded as f64 * 0.005).ceil() as usize;
    let lo = recorded.saturating_sub(tol);
    let hi = recorded + tol;
    assert!(
        actual >= lo && actual <= hi,
        "{label}: population {actual} is outside {lo}..={hi} (recorded {recorded}, band 0.5 %). \
         Small drift is expected and documented; this is larger than drift."
    );
}

fn counts(tiers: &[recon_tool::tier_assignment::TieredPeak]) -> (usize, usize) {
    (
        tiers
            .iter()
            .filter(|t| matches!(t.decision, Decision::Statistics { .. }))
            .count(),
        tiers
            .iter()
            .filter(|t| matches!(t.decision, Decision::Abundance { .. }))
            .count(),
    )
}

/// bcell is the file where the STATISTICS OVERRIDE the curated list's own category
/// preference. Deamidation (Common Artifact, TG=N or Q) and Citrullination
/// (Common Biological, TG=R) are both +0.98402 with the same formula. Category
/// ordering alone picks Citrullination, which is wrong for these samples; only the
/// odds ratios separate them (2.9 vs 1.19). Without this pin a regression in
/// candidate selection silently relabels deamidation and nothing fails.
#[test]
fn statistics_override_category_on_bcell() {
    let Some((tiers, n)) = run("bcell") else {
        return;
    };
    assert_population_near(n, 55_419, "bcell rank-1 spectrum_q<0.01");
    let deam = tiers
        .iter()
        .find(|t| (t.delta_mass - 0.9821).abs() < 0.005)
        .expect("no peak near +0.9821");
    let label = deam.label.clone().unwrap_or_default();
    assert!(
        label.contains("Deamidation"),
        "+0.9821 should resolve to Deamidation, got {label:?}"
    );
    assert!(
        !label.contains("Citrullination"),
        "category preference beat the statistics: {label:?}"
    );
    match deam.decision {
        Decision::Statistics { odds_ratio, .. } => {
            assert!(
                odds_ratio > 2.0 && odds_ratio < 5.0,
                "OR {odds_ratio} (prototype: 2.9)"
            )
        }
        ref d => panic!("expected statistics, got {d:?}"),
    }
    // 10, not 8, since step 2.5. Two protein-terminal candidates become testable
    // once the FASTA is supplied: -89.0289 Met-loss+Acetylation (OR 4231) and
    // +42.0109 Acetylation at a protein N-terminus (OR 174, which REPLACES the
    // `TG=K anywhere` reading that scored OR 1.19 and failed). See
    // `protein_context_moves_only_explained_decisions` below.
    // This count is 8 when the FASTA is absent and the test skips the index.
    // ⚠ REPINNED 2026-09-24, a recorded edit: 10 -> 13 with the FASTA, 8 -> 10
    // without. The peaks come from the regenerated full-run/bcell.json, which
    // holds 268 peaks under the 500 cap and topographic prominence (47 before).
    // Three recommendations enter, each on a peak absent from the old list:
    // +14.0151 Methylation (n=26, protein N-term, needs the FASTA), +47.9861
    // Trioxidation (n=21) and -32.0047 oxidized M side-chain loss (n=22).
    // None leaves. Measured with a scratch probe against both peak lists.
    let expected = if fasta().exists() { 13 } else { 10 };
    assert_eq!(
        counts(&tiers),
        (expected, 0),
        "bcell recommendation set moved"
    );
}

/// serum is the file where a curated candidate is DROPPED by its own statistics:
/// Fe[III] is in the list and clears no threshold (OR 1.72), so it does not appear.
/// Pinned so a regression cannot quietly readmit it.
#[test]
fn statistics_drop_unsupported_candidates_on_serum() {
    let Some((tiers, n)) = run("serum") else {
        return;
    };
    assert_population_near(n, 12_438, "serum rank-1 spectrum_q<0.01");
    let fe = tiers
        .iter()
        .find(|t| (t.delta_mass - 52.9130).abs() < 0.005)
        .expect("no peak near +52.9130");
    match fe.decision {
        Decision::NoResidueSupport { odds_ratio, .. } => {
            assert!(
                odds_ratio < 2.0,
                "Fe[III] OR {odds_ratio} should fail the floor"
            )
        }
        ref d => panic!("serum Fe[III] should read as unsupported, got {d:?}"),
    }
    assert!(
        !fe.decision.is_recommended(),
        "serum Fe[III] must not be recommended"
    );
    // 8, not 7, since the Sage v0.15 upgrade re-baselined the PSM set
    // (2026-09-01). Was 7 under v0.14.7, and 6 when the FASTA is absent because
    // +14.0149 Methylation at a protein N-terminus is only testable with it.
    // NEW BASELINE, not a regression: recon's math is unchanged.
    // ⚠ REPINNED 2026-09-24, a recorded edit: 8 -> 7 with the FASTA (6 without,
    // unchanged). The regenerated full-run/serum.json no longer carries the
    // +58.0128 Carboxymethylation flank (n=97), which topographic prominence
    // removes (NOTES "Prominence is topographic" predicted this loss). Nothing
    // enters.
    let expected = if fasta().exists() { 7 } else { 6 };
    assert_eq!(
        counts(&tiers),
        (expected, 0),
        "serum recommendation set moved"
    );
}

#[test]
fn reproduces_the_prototype_on_b1906() {
    let tsv = repo().join("_dev/testing/search-output/step1-open-b1906/results.sage.tsv");
    if !tsv.exists() {
        eprintln!("skipping: {} absent", tsv.display());
        return;
    }
    // Loose q here; the delta-band population is then taken at spectrum_q < 0.01,
    // which is the basis NOTES standardises on.
    let opts = FilterOptions {
        q_threshold: 1.0,
        ..Default::default()
    };
    let results = parse_sage_results(&tsv, &opts).expect("TSV must parse");
    let psms: Vec<_> = results
        .psms
        .into_iter()
        .filter(|p| p.rank == 1 && p.spectrum_q < 0.01)
        .collect();
    assert_population_near(
        psms.len(),
        22_298,
        "b1906 rank-1 spectrum_q<0.01 (prototype: 22298)",
    );

    let unimod = UnimodDb::from_xml(&repo().join("recon-tool/resources/unimod.xml")).unwrap();
    let (curated, _) =
        CuratedDb::load_from_sources(recon_tool::defaults::CURATED_MODS, unimod.elements())
            .unwrap();

    let report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo().join("_dev/testing/recon-output/full-run/b1906.json"))
            .unwrap(),
    )
    .unwrap();
    let peaks: Vec<(f64, usize)> = report["mod_discovery"]["peaks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["delta_mass"].as_f64().unwrap(),
                p["count"].as_u64().unwrap() as usize,
            )
        })
        .filter(|(d, _)| d.abs() >= 0.1)
        .collect();
    let top = peaks.iter().map(|(_, c)| *c).max().unwrap() as f64;
    let floor = top * 0.20;

    // b1906 has NO protein-context candidate on any peak, so the prototype pin
    // holds with or without an index. Passed anyway, so this test exercises the
    // production configuration rather than a reduced one.
    let index = protein_index();
    let tiers = assign(&peaks, &psms, &curated, floor, index.as_ref());
    let find = |mass: f64| {
        tiers
            .iter()
            .find(|t| (t.delta_mass - mass).abs() < 0.005)
            .unwrap_or_else(|| panic!("no peak near {mass:+.4}"))
    };

    // --- the statistics path -------------------------------------------------
    // (mass, label fragment, minimum odds ratio the prototype reported)
    let by_stats: [(f64, &str, f64); 7] = [
        (57.0219, "Carbamidomethyl", 400.0),
        (15.9949, "Oxidation", 100.0),
        (53.9186, "Fe[II]", 5.0),
        (0.9818, "Deamidation", 3.0),
        (27.9946, "Formylation", 30.0),
        (-18.0113, "Water Loss", 15.0),
        (-17.0266, "pyro-Glu", 200.0),
    ];
    for (mass, want_label, min_or) in by_stats {
        let t = find(mass);
        let label = t.label.clone().unwrap_or_default();
        assert!(
            label.contains(want_label),
            "{mass:+.4}: expected {want_label:?}, got {label:?}"
        );
        match t.decision {
            Decision::Statistics { odds_ratio, q, .. } => {
                assert!(
                    odds_ratio >= min_or,
                    "{label}: OR {odds_ratio} below {min_or}"
                );
                assert!(q < 0.05, "{label}: q {q}");
            }
            ref d => panic!("{label} should be decided by statistics, got {d:?}"),
        }
    }

    // Carbamidomethyl is the only fixed label, and it comes from MetaMorpheus.
    assert!(find(57.0219).is_fixed, "+57 should carry the fixed label");
    assert!(!find(15.9949).is_fixed, "Oxidation is variable");

    // --- the abundance path --------------------------------------------------
    // Carbamyl's acceptors sit in ~100% of tryptic peptides, so no statistic can
    // reach it. It is recommended here because 530 PSMs clears the 259 floor.
    // ⚠ REPINNED 2026-09-24, a recorded edit: 217 % -> 205 %. The peaks come
    // from the regenerated full-run/b1906.json: Carbamyl 545 -> 530 PSMs and
    // the floor 251.0 -> 259.0 (the tallest peak grew). Band unchanged.
    let carbamyl = find(43.0058);
    assert!(carbamyl.label.as_deref().unwrap_or("").contains("Carbamyl"));
    match carbamyl.decision {
        Decision::Abundance { pct_of_floor } => {
            assert!(
                (pct_of_floor - 205.0).abs() < 2.0,
                "Carbamyl should sit at ~205% of floor, got {pct_of_floor:.0}%"
            );
        }
        ref d => panic!("Carbamyl should be decided by abundance, got {d:?}"),
    }

    // --- satellites ----------------------------------------------------------
    for mass in [58.0237, 59.0277] {
        match find(mass).decision {
            Decision::Satellite { .. } => {}
            ref d => panic!("{mass:+.4} should be a satellite, got {d:?}"),
        }
    }

    // The recommendation set is 7 by statistics plus 1 by abundance.
    let n_stats = tiers
        .iter()
        .filter(|t| matches!(t.decision, Decision::Statistics { .. }))
        .count();
    let n_abund = tiers
        .iter()
        .filter(|t| matches!(t.decision, Decision::Abundance { .. }))
        .count();
    // Sage v0.15 baseline (2026-09-01): was (7, 1) under v0.14.7.
    // ⚠ REPINNED 2026-09-24, a recorded edit: (9, 1) -> (11, 1). The regenerated
    // full-run/b1906.json holds 155 peaks (48 before). Two statistics
    // recommendations enter, each on a peak absent from the old list: +47.9825
    // Trioxidation (n=5) and -89.0296 Met-loss+Acetylation (n=17, protein
    // N-term). None leaves. Without the FASTA it is (9, 1), measured the same
    // day; this pin had no FASTA branch before and would have failed there.
    let expected_stats = if fasta().exists() { 11 } else { 9 };
    assert_eq!(
        (n_stats, n_abund),
        (expected_stats, 1),
        "recommendation set moved"
    );
}

/// The Step-2 carpet invariant, asserted on REAL committed data with the actual
/// margins printed. Acceptance is numeric: no pass mark without values.
///
/// The invariant is scoped — the floor governs only peaks the abundance path
/// decides. A synthetic fixture could not test this honestly, because it would
/// inherit whatever carpet the fixture author invented. See NOTES "The carpet
/// invariant — RESTATED AND SCOPED".
#[test]
fn floor_sits_above_the_carpet_on_all_three_files() {
    let mut checked = 0;
    for f in ["serum", "bcell", "b1906"] {
        let path = repo().join(format!("_dev/testing/recon-output/full-run/{f}.json"));
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("report must exist"))
                .expect("report must parse");
        let rec = &v["recommendations"];
        assert!(!rec.is_null(), "{f}: no recommendations block");

        let floor = rec["floor_psms"].as_f64().expect("floor_psms");
        let margin = rec["carpet_margin_psms"]
            .as_f64()
            .expect("carpet_margin_psms");
        let tallest = rec["carpet_tallest_psms"]
            .as_u64()
            .expect("carpet_tallest_psms");

        println!(
            "{f:<7} floor {floor:>7.1} PSMs | tallest floor-governed carpet peak {tallest:>4} \
             | margin {margin:>+8.1}"
        );
        assert!(
            margin > 0.0,
            "{f}: floor {floor:.1} does NOT sit above the carpet — tallest floor-governed \
             carpet peak is {tallest} PSMs, margin {margin:+.1}"
        );
        checked += 1;
    }
    assert_eq!(checked, 3, "all three files must be checked");
}

// ---------------------------------------------------------------------------
// Step 2.5 — the protein-position lookup. Its OWN validation; it does not ride
// on step 2's gates.
// ---------------------------------------------------------------------------

/// The decision's variant, ignoring its payload. Two `Statistics` outcomes are
/// the SAME tier even when their q values differ by BH re-normalisation.
fn variant(d: &Decision) -> &'static str {
    match d {
        Decision::Statistics { .. } => "statistics",
        Decision::Abundance { .. } => "abundance",
        Decision::NoResidueSupport { .. } => "no_residue_support",
        Decision::BelowFloor => "below_floor",
        Decision::Satellite { .. } => "satellite",
        Decision::NotCurated => "not_curated",
        Decision::NotableUnannotated { .. } => "notable_unannotated",
    }
}

/// `(odds_ratio, q)` for the decisions that carry them.
fn evidence(d: &Decision) -> Option<(f64, f64)> {
    match d {
        Decision::Statistics { odds_ratio, q, .. }
        | Decision::NoResidueSupport { odds_ratio, q } => Some((*odds_ratio, *q)),
        _ => None,
    }
}

/// Why one peak's decision may differ between the run without the protein
/// index and the run with it. `Err` names a change that has no explanation.
///
/// Two mechanisms are allowed, and nothing else:
///
/// 1. PROTEIN CONTEXT. The peak has a curated candidate that needs the
///    protein position (protein-terminal or Met-loss). Without the index that
///    candidate is untestable; with it, it is tested. Any change is allowed.
/// 2. BH FAMILY GROWTH. The peak has NO such candidate, so its candidates and
///    their p-values are the same in both runs. Only the BH family changed: the
///    index adds the protein-context p-values to the sweep, so every other q can
///    move, in either direction (an added p near 1, such as serum +42.0032 or
///    b1906 +14.0153, can raise the rest). The odds ratio cannot move, because
///    it is computed per candidate. So a change here is allowed only between
///    `statistics` and `no_residue_support`, each side must satisfy the rule
///    (OR >= OR_MIN and q <= Q_MAX, or not) with its own numbers, and when the
///    same candidate won both times, its odds ratio must be identical and its q
///    must have crossed Q_MAX.
///
/// A peak whose decision did not change must also keep its odds ratio, unless
/// mechanism 1 applies.
fn explain_change(
    has_protein_context: bool,
    a: &recon_tool::tier_assignment::TieredPeak,
    b: &recon_tool::tier_assignment::TieredPeak,
) -> Result<bool, String> {
    use recon_tool::tier_assignment::{OR_MIN, Q_MAX};
    let changed = variant(&a.decision) != variant(&b.decision) || a.label != b.label;
    if has_protein_context {
        return Ok(changed);
    }
    let passes = |d: &Decision| evidence(d).map(|(o, q)| o >= OR_MIN && q <= Q_MAX);
    if !changed {
        if let (Some((oa, _)), Some((ob, _))) = (evidence(&a.decision), evidence(&b.decision)) {
            if (oa - ob).abs() >= 1e-9 {
                return Err(format!(
                    "odds ratio moved {oa} -> {ob} with no protein context"
                ));
            }
        }
        return Ok(false);
    }
    let consistent = |d: &Decision| match d {
        Decision::Statistics { .. } => passes(d) == Some(true),
        Decision::NoResidueSupport { .. } => passes(d) == Some(false),
        _ => false,
    };
    if !consistent(&a.decision) || !consistent(&b.decision) {
        return Err(format!(
            "decision changed with no protein-context candidate, and not by a q \
             crossing Q_MAX: {:?} {:?} -> {:?} {:?}",
            a.label, a.decision, b.label, b.decision
        ));
    }
    if a.label == b.label {
        let (oa, qa) = evidence(&a.decision).unwrap();
        let (ob, qb) = evidence(&b.decision).unwrap();
        if (oa - ob).abs() >= 1e-9 || (qa <= Q_MAX) == (qb <= Q_MAX) {
            return Err(format!(
                "same candidate, but not a pure q shift across Q_MAX: \
                 OR {oa} -> {ob}, q {qa:.6e} -> {qb:.6e}"
            ));
        }
    }
    Ok(true)
}

/// Run both arms on one file. Returns `(peak, has_protein_context, without,
/// with)` per peak, or `None` when the data is absent.
#[allow(clippy::type_complexity)]
fn both_arms(
    key: &str,
    index: &ProteinIndex,
) -> Option<
    Vec<(
        bool,
        recon_tool::tier_assignment::TieredPeak,
        recon_tool::tier_assignment::TieredPeak,
    )>,
> {
    let tsv = repo().join(format!(
        "_dev/testing/search-output/step1-open-{key}/results.sage.tsv"
    ));
    if !tsv.exists() {
        eprintln!("skipping: {} absent", tsv.display());
        return None;
    }
    let opts = FilterOptions {
        q_threshold: 1.0,
        ..Default::default()
    };
    let psms: Vec<_> = parse_sage_results(&tsv, &opts)
        .expect("TSV must parse")
        .psms
        .into_iter()
        .filter(|p| p.rank == 1 && p.spectrum_q < 0.01)
        .collect();
    let unimod = UnimodDb::from_xml(&repo().join("recon-tool/resources/unimod.xml")).unwrap();
    let (curated, _) =
        CuratedDb::load_from_sources(recon_tool::defaults::CURATED_MODS, unimod.elements())
            .unwrap();
    let report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            repo().join(format!("_dev/testing/recon-output/full-run/{key}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    let peaks: Vec<(f64, usize)> = report["mod_discovery"]["peaks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["delta_mass"].as_f64().unwrap(),
                p["count"].as_u64().unwrap() as usize,
            )
        })
        .filter(|(d, _)| d.abs() >= 0.1)
        .collect();
    let floor = peaks.iter().map(|(_, c)| *c).max().unwrap() as f64 * 0.20;

    // I2 / I3: the two structural guards, printed per file.
    let (resolved, total) = index.resolution(&psms);
    let frac = resolved as f64 / total as f64;
    let nterm = psms
        .iter()
        .filter(|p| {
            index.starts_protein(
                &recon_tool::peak_composition::residues_of(&p.peptide),
                &p.proteins,
            )
        })
        .count();
    let nterm_pct = 100.0 * nterm as f64 / total as f64;
    println!(
        "{key}: {resolved}/{total} accessions resolved ({:.2}%), \
         {nterm} at protein position 0 ({nterm_pct:.3}%)",
        100.0 * frac
    );
    assert!(
        frac >= recon_tool::protein_index::MIN_RESOLVED_FRACTION,
        "{key}: only {:.2}% of target PSMs resolve; wrong FASTA?",
        100.0 * frac
    );
    assert!(
        nterm_pct < 5.0,
        "{key}: {nterm_pct:.3}% at protein position 0; the lookup is matching \
         too much and the positional test would carry no information"
    );

    let without = assign(&peaks, &psms, &curated, floor, None);
    let with = assign(&peaks, &psms, &curated, floor, Some(index));
    assert_eq!(without.len(), with.len(), "{key}: peak count changed");
    Some(
        without
            .into_iter()
            .zip(with)
            .map(|(a, b)| {
                let ctx = curated
                    .candidates(a.delta_mass, recon_tool::tier_assignment::CURATED_TOL_DA)
                    .iter()
                    .any(|c| c.needs_protein_context());
                (ctx, a, b)
            })
            .collect(),
    )
}

/// I1: supplying the protein index changes only EXPLAINED decisions.
///
/// Run the same three files with and without the index and diff the decisions.
/// Every changed decision must be explained by `explain_change`: either the
/// peak has a protein-terminal or Met-loss candidate, or the change is a q
/// crossing Q_MAX because the BH family grew. Nothing else may change.
///
/// ⚠ REWRITTEN 2026-09-25, a recorded edit. This test pinned the moved SET
/// (three decisions under Sage v0.14.7, four at v0.15). With the 500-peak cap
/// (ad7a10e) the sweep holds more tests and the index adds p-values near 1,
/// so the set became seven and the old "q can only fall" premise became false.
/// A count is a property of the peak list; the mechanism is the claim. See
/// NOTES "The protein-context test asserts the mechanism".
///
/// The CONVENTION evidence is unchanged and independent of this code: the band
/// is 98.9% NME-permissive at residue 2 against a 55.3% background.
/// `_dev/testing/scripts/protein_nterm_evidence.py` produces those numbers.
#[test]
fn protein_context_moves_only_explained_decisions() {
    let Some(index) = protein_index() else { return };
    let mut moved: Vec<String> = Vec::new();
    for key in ["serum", "bcell", "b1906"] {
        let Some(rows) = both_arms(key, &index) else {
            return;
        };
        for (ctx, a, b) in &rows {
            match explain_change(*ctx, a, b) {
                Ok(true) => {
                    let why = if *ctx { "protein context" } else { "BH family" };
                    moved.push(format!("{key} {:+.4} n={} ({why})", a.delta_mass, a.count));
                    println!(
                        "  MOVED {key} {:+.4} n={} [{why}] {:?} {:?}\n            ->  {:?} {:?}",
                        a.delta_mass, a.count, a.label, a.decision, b.label, b.decision
                    );
                }
                Ok(false) => {}
                Err(e) => panic!("{key} {:+.4} n={}: {e}", a.delta_mass, a.count),
            }
        }
    }
    println!("\ndecisions that moved: {moved:#?}");
    // The index must do something, or the test proves nothing about it.
    assert!(
        moved.iter().any(|m| m.ends_with("(protein context)")),
        "no protein-context decision moved; the index had no effect"
    );
}

/// The check in I1 must FAIL on an unexplained change. Feed it deliberately
/// wrong inputs, built from real bcell data, and require an `Err` each time.
#[test]
fn unexplained_decision_change_is_rejected() {
    let Some(index) = protein_index() else { return };
    let Some(rows) = both_arms("bcell", &index) else {
        return;
    };
    // A peak decided by statistics with no protein-context candidate.
    let (_, a, b) = rows
        .iter()
        .find(|(ctx, a, b)| {
            !ctx && matches!(a.decision, Decision::Statistics { .. })
                && variant(&a.decision) == variant(&b.decision)
        })
        .expect("bcell has a plain statistics peak");
    assert_eq!(
        explain_change(false, a, b),
        Ok(false),
        "the real pair is explained"
    );

    // 1. The tier moves to one no q shift can produce.
    let mut wrong = b.clone();
    wrong.decision = Decision::BelowFloor;
    assert!(
        explain_change(false, a, &wrong).is_err(),
        "tier jump accepted"
    );

    // 2. The tier flips to no_residue_support while the numbers still pass.
    let (o, q) = evidence(&b.decision).unwrap();
    let mut wrong = b.clone();
    wrong.decision = Decision::NoResidueSupport { odds_ratio: o, q };
    assert!(
        explain_change(false, a, &wrong).is_err(),
        "inconsistent flip accepted"
    );

    // 3. The odds ratio moves while the tier holds.
    let mut wrong = b.clone();
    if let Decision::Statistics { odds_ratio, .. } = &mut wrong.decision {
        *odds_ratio *= 1.01;
    }
    assert!(
        explain_change(false, a, &wrong).is_err(),
        "odds-ratio move accepted"
    );

    // The same wrong change IS allowed when a protein-context candidate is present.
    let mut wrong = b.clone();
    wrong.decision = Decision::BelowFloor;
    assert_eq!(explain_change(true, a, &wrong), Ok(true));
}

/// The promoted peak, pinned with its actual 2x2 outcome.
///
/// Separate from I1 so a regression says WHICH half broke: "the set of moves
/// changed" and "the move is still there but its evidence collapsed" are
/// different faults.
#[test]
fn met_loss_evidence_on_bcell() {
    let Some((tiers, n)) = run("bcell") else {
        return;
    };
    if !fasta().exists() {
        return;
    }
    assert_population_near(n, 55_419, "bcell background");
    let t = tiers
        .iter()
        .find(|t| (t.delta_mass + 89.0289).abs() < 0.005)
        .expect("no peak near -89.0289");
    let label = t.label.clone().unwrap_or_default();
    println!(
        "bcell {:+.4} n={} label={label:?} -> {:?}",
        t.delta_mass, t.count, t.decision
    );
    assert!(label.contains("Met-loss+Acetylation"), "got {label:?}");
    match &t.decision {
        Decision::Statistics {
            odds_ratio,
            q,
            sites,
        } => {
            println!("  OR {odds_ratio:.1}  q {q:.3e}  sites {sites:?}");
            // Pinned LOOSE deliberately. The claim is "overwhelming", not a
            // specific value: the odds ratio depends on the exact band and would
            // move for reasons that are not a defect. A collapse below 100 is.
            assert!(*odds_ratio > 100.0, "OR {odds_ratio} — evidence collapsed");
            assert!(*q < 1e-10, "q {q} — evidence collapsed");
            // TG=X on this entry: acetylation places NO requirement on the
            // residue the Met removal exposes (Unimod 1 gives Acetyl
            // `site=N-term` at Protein N-term). The initiator-Met requirement is
            // carried by PP and enforced in `peptide_hits`, not by TG.
            //
            // If this ever reads the NME-permissive set (A/C/G/S/T/V), the
            // withdrawn "TG is the NME rule" design is back. A single 'G' would
            // be Met-loss+Myristoylation's acceptor, not acetylation's.
            assert!(
                sites.is_empty(),
                "acetyl must place no residue requirement, got {sites:?}"
            );
        }
        d => panic!("expected the statistics path, got {d:?}"),
    }
    assert_eq!(
        t.position.as_deref(),
        Some("Protein N-terminal, Met loss."),
        "the position carries the acceptor test and must reach the report"
    );
    assert!(t.decision.is_recommended());
}

/// I4 — the negative control. A decoy can never be protein N-terminal, because
/// `rev_` accessions are not in a target FASTA.
///
/// Free, and it is the only control here that would fire if the lookup started
/// matching on something other than the accession.
#[test]
fn decoys_are_never_protein_n_terminal() {
    let Some(index) = protein_index() else { return };
    let tsv = repo().join("_dev/testing/search-output/step1-open-bcell/results.sage.tsv");
    if !tsv.exists() {
        eprintln!("skipping: {} absent", tsv.display());
        return;
    }
    // q_threshold 1.0 still drops decoys, so read the raw TSV rows instead.
    let text = std::fs::read_to_string(&tsv).expect("TSV must read");
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().expect("header").split('\t').collect();
    let col = |name: &str| header.iter().position(|h| *h == name).expect(name);
    let (c_pep, c_prot) = (col("peptide"), col("proteins"));
    let mut decoys = 0usize;
    let mut hits = 0usize;
    for line in lines {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() <= c_prot || !f[c_prot].starts_with("rev_") {
            continue;
        }
        decoys += 1;
        if index.starts_protein(
            &recon_tool::peak_composition::residues_of(f[c_pep]),
            f[c_prot],
        ) {
            hits += 1;
        }
    }
    println!("bcell decoy PSMs: {decoys}, classified protein N-terminal: {hits}");
    assert!(decoys > 0, "no decoy rows found — the control did not run");
    assert_eq!(hits, 0, "a decoy resolved to protein position 0");
}
