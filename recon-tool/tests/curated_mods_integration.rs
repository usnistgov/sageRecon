//! Integration tripwire: the curated list must load from the REAL committed files
//! and reproduce the masses and counts the Python prototype pinned.
//!
//! `_dev/testing/scripts/tier_report_prototype.py` measures 94 entries with a computable
//! mass across the four files it reads. If the Rust loader disagrees, one of the two
//! is wrong and the report's candidate set has silently moved.
//!
//! ⚠ "The REAL committed files" now reach this test the way they reach the
//! SHIPPED BINARY: through `defaults::CURATED_MODS`, which `include_str!`s those
//! same four files and is what `main.rs` passes to
//! `CuratedDb::load_from_sources`. Before 2026-09-02 this test called
//! `CuratedDb::load` and read the directory from disk — a route production does
//! not take, which is the v0.1.0 bug shape. `defaults.rs` proves the embedded
//! text is byte-identical to the committed files, and
//! `bundled_defaults_integration.rs` proves the two parse to the same database,
//! so nothing about the provenance claim is weakened.

use recon_tool::curated_mods::CuratedDb;
use recon_tool::unimod::UnimodDb;
use std::path::Path;

fn repo() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

#[test]
fn loads_the_committed_curated_list() {
    let unimod = UnimodDb::from_xml(&repo().join("recon-tool/resources/unimod.xml"))
        .expect("unimod.xml must load");
    let elements = unimod.elements();
    assert!(
        elements.len() >= 40,
        "expected the full element table, got {}",
        elements.len()
    );
    let h = elements.get("H").copied().unwrap_or_default();
    assert!((h - 1.007_825_035).abs() < 1e-9, "H mass {h}");

    let (db, skipped) = CuratedDb::load_from_sources(recon_tool::defaults::CURATED_MODS, elements)
        .expect("curated mod files must load");

    // Pinned against the Python prototype, which reported 94.
    //
    // RE-PINNED 94 -> 98 on 2026-08-26, a deliberate recorded edit. FOUR entries
    // were ADDED to Mods.txt -- one Met-loss pair for each protein-N-term mod:
    // Met-loss+{Acetylation, Methylation, Succinylation, Myristoylation}, at
    // -89.02992 / -117.02483 / -31.02444 / +79.15788. The prototype predates them. Nothing was removed and no mass or
    // acceptor changed on any existing entry -- the same-day corrections touched
    // only names, one DR cross-reference, and five PP position strings. See NOTES
    // "Met-loss encoding" and the correction header in Mods.txt.
    //
    // RE-PINNED 98 -> 99 on 2026-08-27, a deliberate recorded edit. ONE entry was
    // ADDED: "Met-loss" (Unimod 765, -131.040485), the initiator Met removed with
    // no following modification. It was missing while all four "+mod" forms were
    // present. The same edit changed TG on the four Met-loss entries -- M -> X on
    // acetyl/methyl/succinyl, M -> G on myristoyl -- which moves no mass and adds
    // no entry. See the numbered header block in Mods.txt.
    assert_eq!(
        db.len(),
        99,
        "curated entry count moved (skipped {skipped}) — the candidate set has changed"
    );

    // The two 2026-08-26 name corrections must be what actually LOADS. They live
    // in Mods.txt itself now, not in a code-side override table -- one mechanism,
    // not two -- so this is the only place that proves they took effect.
    assert!(
        db.entries().iter().any(|e| e.label == "Gln->pyro-Glu"),
        "corrected Gln->pyro-Glu name did not load"
    );
    assert!(
        db.entries()
            .iter()
            .any(|e| e.label == "Water Loss (Glu->pyro-Glu)"),
        "corrected -18.01 N-term E name did not load"
    );
    // All four protein-N-term mods must have a paired Met-loss entry.
    for m in [
        "Acetylation",
        "Methylation",
        "Succinylation",
        "Myristoylation",
    ] {
        let want = format!("Met-loss+{m}");
        assert!(
            db.entries().iter().any(|e| e.label == want),
            "missing curated entry: {want}"
        );
    }

    // The +57 candidate set is the whole reason for using a curated list:
    // Gly and Carbofuran must NOT be in it.
    let at57 = db.candidates(57.021_464, 0.01);
    assert!(!at57.is_empty(), "no curated candidate at +57.0215");
    let labels: Vec<&str> = at57.iter().map(|c| c.label.as_str()).collect();
    assert!(
        labels.iter().any(|l| l.starts_with("Carbamidomethyl")),
        "Carbamidomethyl missing at +57: {labels:?}"
    );
    assert!(
        !labels.iter().any(|l| *l == "Gly" || *l == "Carbofuran"),
        "the +57 ambiguity is back: {labels:?}"
    );

    // Carbamidomethyl on C is the only Common Fixed entry that matters here, and
    // it must carry Cys as a testable site.
    let cam_on_c = at57
        .iter()
        .find(|c| c.id == "Carbamidomethyl on C")
        .expect("Carbamidomethyl on C missing");
    assert!(cam_on_c.is_fixed(), "CAM on C should read as fixed");
    assert_eq!(cam_on_c.sites, vec!['C']);

    // Carbamyl is the routing case: TG=X at the N-terminus yields no testable
    // residue, which is what sends it to the abundance path.
    let carbamyl_nterm = db
        .candidates(43.005_814, 0.01)
        .into_iter()
        .find(|c| c.is_terminal() && c.sites.is_empty());
    assert!(
        carbamyl_nterm.is_some(),
        "Carbamyl N-terminal entry must have no testable residue"
    );

    // The label override must survive the real parse.
    let pyro = db.candidates(-17.026_549, 0.01);
    assert!(
        pyro.iter().any(|c| c.label.contains("Gln->pyro-Glu")),
        "label override not applied: {:?}",
        pyro.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}
