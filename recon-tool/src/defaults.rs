//! Resources compiled into the binary, so `recon` does not depend on being
//! launched from the repo root.
//!
//! WHY THIS EXISTS. Before this module, `recon run` with no `--params` looked for
//! `_dev/testing/configs/open-search-params.json` RELATIVE TO THE WORKING DIRECTORY,
//! the pass-2 default likewise, and the curated mod list at
//! `_dev/reference-notes/metaMorpheusMods`. A user who unzipped a release and ran it
//! on their own data got an error naming a repo path they do not have. That is a
//! packaging defect, not a science one — the searches were always right when they
//! ran at all.
//!
//! WHAT IS BUNDLED, AND WHAT IS NOT.
//!
//! * **Bundled:** the two search templates and the four curated mod files. Around
//!   20 KB in total, all of it text this project authors or curates.
//! * **NOT bundled:** `unimod.xml` (2.4 MB) and any FASTA. Those stay CLI
//!   arguments. Size is only half the reason; the other half is that they are
//!   third-party data whose redistribution terms are not settled in
//!   `THIRD_PARTY_LICENSES.md`. Do not embed them without settling that first.
//!
//! THE DIVERGENCE PROBLEM, AND THE GUARD.
//!
//! A copy of a config is a config that can go stale. The committed templates
//! under `_dev/testing/configs/` remain the source of truth for the test suite and for
//! reproducing any recorded run, so a bundled copy that quietly drifts from them
//! would make the shipped tool and the tested tool two different tools.
//! [`tests::bundled_default_matches_committed_template`] forbids that: it parses
//! both and asserts they agree on EVERY field, allowing exactly two documented
//! differences — `database.fasta` and `mzml_paths`, which the bundled copies omit
//! because both are always supplied at runtime. Edit one file and not the other
//! and the suite fails.

/// Pass-1 alkylation-agnostic open search template.
pub const OPEN_SEARCH: &str = include_str!("defaults/open-search.json");

/// Pass-2 semi-enzymatic template, for the identified-subset search.
pub const PASS2: &str = include_str!("defaults/pass2.json");

/// The Unimod database, compiled in so `--unimod` is an override rather than a
/// requirement.
///
/// ⚠ **THIS IS REDISTRIBUTION AND IT IS LICENSED.** Unimod is published under the
/// Design Science License, whose Section 3 requires a copy of the License to
/// travel with the work AND the Source Data to accompany the Object Form. So
/// `THIRD_PARTY_LICENSES.md` (which carries the full DSL text) and `unimod.xml`
/// itself MUST both ship inside any release archive. See NOTES and
/// `_dev/reference-notes/mascot-enzymes.md`'s sibling entry for the reasoning.
///
/// About 2.4 MB, which is most of the binary's size. That is the price of the
/// tool running without a second file to hand it.
pub const UNIMOD: &str = include_str!("../resources/unimod.xml");

/// Label used in provenance records and error messages when a bundled default is
/// used instead of a file. It names the version so an `effective-params.json`
/// says which build produced it.
pub fn bundled_label(which: &str) -> String {
    format!(
        "<bundled {} default, recon {}>",
        which,
        env!("CARGO_PKG_VERSION")
    )
}

/// The curated modification list, as (filename, contents) pairs.
///
/// Provenance is unchanged from the on-disk list: the MetaMorpheus curated mod
/// list, a dated snapshot, recorded in `curated_mods.rs`. These are the four
/// files the tool ships, held in `recon-tool/resources/mods/` because they are
/// product data, not reference material. The other files in the upstream
/// snapshot were never loaded and are not bundled.
pub const CURATED_MODS: &[(&str, &str)] = &[
    ("Mods.txt", include_str!("../resources/mods/Mods.txt")),
    (
        "aListOfmods.txt",
        include_str!("../resources/mods/aListOfmods.txt"),
    ),
    (
        "ProteaseMods.txt",
        include_str!("../resources/mods/ProteaseMods.txt"),
    ),
    (
        "surfactants.txt",
        include_str!("../resources/mods/surfactants.txt"),
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The keys a bundled default deliberately drops, because the run always
    /// supplies them and a value here would be a ghost path.
    const OMITTED: [&str; 2] = ["fasta", "mzml_paths"];

    fn committed(rel: &str) -> serde_json::Value {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(rel);
        let text = std::fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("cannot read committed template {}: {e}", p.display()));
        serde_json::from_str(&text).unwrap()
    }

    /// Compare two configs field by field, ignoring `_`-prefixed notes and the
    /// two deliberately omitted keys. Returns the disagreements.
    fn diff(bundled: &serde_json::Value, committed: &serde_json::Value) -> Vec<String> {
        let mut out = Vec::new();
        let (b, c) = (bundled.as_object().unwrap(), committed.as_object().unwrap());
        // Every committed key must be present and equal in the bundled copy,
        // except the omitted ones and the commentary.
        for (k, cv) in c {
            if k.starts_with('_') || OMITTED.contains(&k.as_str()) {
                continue;
            }
            match b.get(k) {
                None => out.push(format!("bundled is MISSING `{k}`")),
                Some(bv) if k == "database" => {
                    let (bd, cd) = (bv.as_object().unwrap(), cv.as_object().unwrap());
                    for (dk, dcv) in cd {
                        if dk.starts_with('_') || OMITTED.contains(&dk.as_str()) {
                            continue;
                        }
                        match bd.get(dk) {
                            None => out.push(format!("bundled database is MISSING `{dk}`")),
                            Some(dbv) if dbv != dcv => {
                                out.push(format!("database.{dk}: bundled {dbv} != committed {dcv}"))
                            }
                            _ => {}
                        }
                    }
                    for dk in bd.keys() {
                        if !dk.starts_with('_') && !cd.contains_key(dk) {
                            out.push(format!("bundled database has EXTRA `{dk}`"));
                        }
                    }
                }
                Some(bv) if bv != cv => out.push(format!("{k}: bundled {bv} != committed {cv}")),
                _ => {}
            }
        }
        for k in b.keys() {
            if !k.starts_with('_') && !c.contains_key(k) {
                out.push(format!("bundled has EXTRA `{k}`"));
            }
        }
        out
    }

    /// THE GUARD. A bundled config that drifts from the committed one makes the
    /// shipped tool and the tested tool different tools, silently. This fails the
    /// moment either side is edited alone.
    #[test]
    fn bundled_default_matches_committed_template() {
        for (name, bundled, rel) in [
            (
                "pass 1",
                OPEN_SEARCH,
                "_dev/testing/configs/open-search-params.json",
            ),
            (
                "pass 2",
                PASS2,
                "_dev/testing/configs/digestion-efficiency-pass2.json",
            ),
        ] {
            let b: serde_json::Value = serde_json::from_str(bundled)
                .unwrap_or_else(|e| panic!("bundled {name} default is not valid JSON: {e}"));
            let d = diff(&b, &committed(rel));
            assert!(
                d.is_empty(),
                "bundled {name} default has drifted from {rel}:\n  {}",
                d.join("\n  ")
            );
        }
    }

    /// The bundled defaults must carry NO paths at all. This is the property that
    /// makes them safe to ship: a path baked into the binary is a ghost that
    /// cannot be corrected without a rebuild.
    #[test]
    fn bundled_defaults_carry_no_paths() {
        for (name, bundled) in [("pass 1", OPEN_SEARCH), ("pass 2", PASS2)] {
            let v: serde_json::Value = serde_json::from_str(bundled).unwrap();
            let o = v.as_object().unwrap();
            assert!(
                o.get("mzml_paths").is_none(),
                "bundled {name} default declares mzml_paths"
            );
            assert!(
                o["database"].as_object().unwrap().get("fasta").is_none(),
                "bundled {name} default declares database.fasta"
            );
        }
    }

    /// Pass 1 refuses a non-`[0,0]` isotope window at runtime. The bundled
    /// default must not be the thing that trips it.
    #[test]
    fn the_bundled_pass1_default_satisfies_the_isotope_guard() {
        let v: serde_json::Value = serde_json::from_str(OPEN_SEARCH).unwrap();
        assert_eq!(v["isotope_errors"], serde_json::json!([0, 0]));
    }

    /// The four curated files must arrive non-empty and in the order `main.rs`
    /// used to read them. An empty embed would silently produce a curated list
    /// with no entries, which routes every decision to abundance.
    #[test]
    fn the_curated_mod_list_is_embedded_and_non_empty() {
        assert_eq!(CURATED_MODS.len(), 4);
        let names: Vec<&str> = CURATED_MODS.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            names,
            [
                "Mods.txt",
                "aListOfmods.txt",
                "ProteaseMods.txt",
                "surfactants.txt"
            ]
        );
        for (n, text) in CURATED_MODS {
            assert!(
                !text.trim().is_empty(),
                "embedded curated file {n} is empty"
            );
        }
    }

    /// The embedded curated text must be byte-identical to the committed files.
    /// `include_str!` makes drift impossible at compile time on this machine, but
    /// this states the property so a future refactor that copies the text by hand
    /// cannot pass.
    #[test]
    fn embedded_curated_text_matches_the_committed_files() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("recon-tool/resources/mods");
        for (name, text) in CURATED_MODS {
            let on_disk = std::fs::read_to_string(dir.join(name)).unwrap();
            assert_eq!(
                *text, on_disk,
                "embedded {name} differs from the committed file"
            );
        }
    }
}
