//! Curated modification list — the annotation source for tier assignment.
//!
//! Candidate names and acceptor sites come from MetaMorpheus's hand-curated mod
//! files, NOT from all of Unimod. Searching all of Unimod is what produced the
//! +57 ambiguity (Carbamidomethyl / Carbofuran / Gly) and what handed chemistry
//! names to isotope artifacts. Neither Gly nor Carbofuran is in the curated list,
//! so that ambiguity does not arise.
//!
//! Provenance: `_dev/reference-notes/metaMorpheusMods/`, from
//! `github.com/smith-chem-wisc/MetaMorpheus` under `MetaMorpheus/EngineLayer/Mods/`
//! and `MetaMorpheus/EngineLayer/Data/`. A dated snapshot, like every other
//! reference input here — see NOTES "MetaMorpheus's curated mod list".
//!
//! Format is one record per `//`-terminated block:
//! ```text
//! ID   Carbamidomethyl on C
//! TG   C                     <- target residues, "K or D or E", or X for any
//! PP   Anywhere.             <- position
//! MT   Common Fixed          <- category; drives the fixed/variable label
//! CF   H3 C2 N O             <- chemical formula; the mass comes from this
//! DR   Unimod; 4.
//! //
//! ```
//! The files carry no masses, so each `CF` is summed against Unimod's own element
//! table. Unimod is still needed for that, and for informational names on tail
//! peaks; it no longer drives tiering.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// The `MT` category. `Common Fixed` is the only one that yields a "fixed" label.
pub const CAT_COMMON_FIXED: &str = "Common Fixed";

#[derive(Debug, Clone)]
pub struct CuratedMod {
    /// `ID`, with a small override table applied (see `label_for`).
    pub label: String,
    /// Raw `ID` as written upstream.
    pub id: String,
    /// Acceptor residues from `TG`. Empty when `TG` names no residue (e.g. `X`),
    /// which is what routes a peak to the abundance path.
    pub sites: Vec<char>,
    /// `PP`, e.g. "Anywhere.", "Peptide N-terminal.", "Protein N-terminal.",
    /// or "Protein N-terminal, Met loss."
    pub position: String,
    /// `MT` category.
    pub category: String,
    /// `CF` as written.
    pub formula: String,
    /// Monoisotopic mass summed from `formula`.
    pub mass: f64,
}

impl CuratedMod {
    /// True when the position restricts the mod to a PEPTIDE terminus, which is
    /// the only positional claim testable from a PSM alone.
    ///
    /// Protein-terminal entries are excluded deliberately -- `peptide_hits`
    /// handles those in its own branch, against the protein sequence. Reading
    /// one as a peptide terminus would check the wrong position entirely and
    /// report a confident, wrong odds ratio.
    pub fn is_terminal(&self) -> bool {
        if self.needs_protein_context() {
            return false;
        }
        self.position.contains("N-terminal") || self.position.contains("C-terminal")
    }

    /// True when deciding this candidate needs to know where the peptide sits in
    /// its PROTEIN, which a PSM does not carry -- it needs a FASTA lookup.
    ///
    /// `protein_index::ProteinIndex` supplies that lookup, and `peptide_hits`
    /// has one branch for every candidate this returns true for: the peptide
    /// must start at protein position 0, and residue 1 must be in `TG`.
    ///
    /// WITHOUT an index these candidates are NOT TESTABLE, and `test_candidate`
    /// routes them to abundance rather than computing a statistic against the
    /// wrong residue. The failure that would otherwise happen: "Protein
    /// N-terminal." read as a peptide N-term passes any peptide starting with
    /// the acceptor, protein position unchecked. On bcell that shortcut runs at
    /// a 3.17% background against 1.09% for the real test.
    ///
    /// ⚠ CORRECTED 2026-09-01. This once said the curated entries use `TG=M`, so
    /// residue 1 is the acceptor. They do not: since 2026-08-27 the Met-loss
    /// entries carry `TG=X` (or `TG=G` for myristoylation), and `peptide_hits`
    /// enforces residue 1 = M from `PP` while testing `TG` against residue 2.
    /// The search still matches the Met-RETAINED peptide — that part was right.
    /// See `is_met_loss` below and NOTES "Met-loss encoding".
    pub fn needs_protein_context(&self) -> bool {
        self.position.contains("Protein N-terminal") || self.position.contains("Protein C-terminal")
    }

    /// True when this entry describes an initiator-Met removal, i.e.
    /// `PP = "Protein N-terminal, Met loss."`.
    ///
    /// These entries read `TG` differently, and deliberately: see `peptide_hits`.
    /// The Met that is lost is protein residue 1, and the modification that
    /// follows lands on the residue the removal EXPOSES, which is protein
    /// residue 2. `PP` already states residue 1 is M -- a Met that is not there
    /// cannot be lost -- so `TG` is free to name the acceptor of the following
    /// modification, exactly as it does for every other entry in the file.
    pub fn is_met_loss(&self) -> bool {
        self.position.contains("Met loss")
    }

    /// True when the label should be reported as a fixed modification.
    pub fn is_fixed(&self) -> bool {
        self.category == CAT_COMMON_FIXED
    }
}

/// Parse `TG` into acceptor residues. `X` means "any residue" and yields none,
/// which is the signal that the statistics cannot reach this modification.
fn parse_targets(tg: &str) -> Vec<char> {
    tg.split(" or ")
        .map(str::trim)
        .filter(|p| p.len() == 1 && p.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
        .filter_map(|p| p.chars().next())
        .filter(|c| *c != 'X')
        .collect()
}

/// Sum a `CF` formula such as `H-1 N-1 O1` or `C8H13NO5` against `elements`.
pub fn formula_mass(formula: &str, elements: &HashMap<String, f64>) -> Option<f64> {
    let bytes: Vec<char> = formula.chars().collect();
    let mut total = 0.0;
    let mut i = 0;
    let mut saw_any = false;
    while i < bytes.len() {
        let c = bytes[i];
        if !c.is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        // Element symbol: an uppercase letter, optionally followed by lowercase.
        let start = i;
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_lowercase() {
            i += 1;
        }
        let symbol: String = bytes[start..i].iter().collect();
        // Optional bracketed isotope, e.g. C[13].
        if i < bytes.len() && bytes[i] == '[' {
            while i < bytes.len() && bytes[i] != ']' {
                i += 1;
            }
            i += 1;
        }
        // Optional signed count.
        let num_start = i;
        if i < bytes.len() && bytes[i] == '-' {
            i += 1;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let count: i32 = if i > num_start {
            bytes[num_start..i]
                .iter()
                .collect::<String>()
                .parse()
                .ok()?
        } else {
            1
        };
        let mass = elements.get(&symbol)?;
        total += mass * count as f64;
        saw_any = true;
    }
    if saw_any {
        Some(total)
    } else {
        None
    }
}

/// The curated list, sorted by mass.
#[derive(Debug, Default)]
pub struct CuratedDb {
    entries: Vec<CuratedMod>,
}

impl CuratedDb {
    /// Load every `.txt` in `dir` that parses as a MetaMorpheus mod file.
    ///
    /// `elements` supplies monoisotopic element masses (from Unimod). Entries
    /// whose `CF` references an unknown element are skipped, and the count of
    /// skips is returned so a caller can surface it rather than lose it silently.
    pub fn load(
        dir: &Path,
        files: &[&str],
        elements: &HashMap<String, f64>,
    ) -> Result<(Self, usize)> {
        let mut sources = Vec::with_capacity(files.len());
        for name in files {
            let path = dir.join(name);
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read curated mod file: {}", path.display()))?;
            sources.push((name.to_string(), text));
        }
        let borrowed: Vec<(&str, &str)> = sources
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        Self::load_from_sources(&borrowed, elements)
    }

    /// Parse an already-loaded curated list.
    ///
    /// Exists so the SHIPPED binary can carry the list compiled in
    /// (`crate::defaults::CURATED_MODS`) instead of reading
    /// `_dev/reference-notes/metaMorpheusMods/` relative to the working directory —
    /// a path a user who unzipped a release does not have. [`load`] is the
    /// file-reading wrapper and delegates here, so both routes parse identically
    /// by construction rather than by a second copy of the parser.
    pub fn load_from_sources(
        sources: &[(&str, &str)],
        elements: &HashMap<String, f64>,
    ) -> Result<(Self, usize)> {
        let mut entries = Vec::new();
        let mut skipped = 0usize;
        for (_name, text) in sources {
            let text: &str = text;
            for block in text.split("\n//") {
                let mut f: HashMap<&str, &str> = HashMap::new();
                for line in block.lines() {
                    let line = line.trim_start_matches('\u{feff}');
                    if line.len() > 5 && line.as_bytes()[2] == b' ' {
                        let (k, v) = line.split_at(2);
                        if k.chars().all(|c| c.is_ascii_uppercase()) {
                            f.entry(k).or_insert(v.trim());
                        }
                    }
                }
                let (Some(id), Some(cf)) = (f.get("ID"), f.get("CF")) else {
                    continue;
                };
                let Some(mass) = formula_mass(cf, elements) else {
                    skipped += 1;
                    continue;
                };
                entries.push(CuratedMod {
                    label: id.to_string(),
                    id: id.to_string(),
                    sites: parse_targets(f.get("TG").copied().unwrap_or("")),
                    position: f.get("PP").copied().unwrap_or("").to_string(),
                    category: f.get("MT").copied().unwrap_or("").to_string(),
                    formula: cf.to_string(),
                    mass,
                });
            }
        }
        entries.sort_by(|a, b| {
            a.mass
                .partial_cmp(&b.mass)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok((Self { entries }, skipped))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Read-only view of the loaded entries, for tests that must prove a
    /// specific curated name or entry actually loaded from the file on disk.
    pub fn entries(&self) -> &[CuratedMod] {
        &self.entries
    }

    /// Every curated candidate within `tol` Da of `mass`.
    pub fn candidates(&self, mass: f64, tol: f64) -> Vec<&CuratedMod> {
        self.entries
            .iter()
            .filter(|e| (e.mass - mass).abs() <= tol)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn els() -> HashMap<String, f64> {
        // Monoisotopic masses as Unimod itself states them.
        [
            ("H", 1.007_825_035),
            ("C", 12.0),
            ("N", 14.003_074),
            ("O", 15.994_914_63),
            ("S", 31.972_070_7),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect()
    }

    #[test]
    fn formula_masses_match_unimod() {
        let e = els();
        // Carbamidomethyl, Unimod 4.
        let cam = formula_mass("H3 C2 N O", &e).unwrap();
        assert!((cam - 57.021_464).abs() < 1e-6, "{cam}");
        // Oxidation, Unimod 35.
        let ox = formula_mass("O1", &e).unwrap();
        assert!((ox - 15.994_915).abs() < 1e-6, "{ox}");
        // Deamidation: negative counts must work.
        let deam = formula_mass("H-1 N-1 O1", &e).unwrap();
        assert!((deam - 0.984_016).abs() < 1e-6, "{deam}");
        // Carbamyl, Unimod 5.
        let carb = formula_mass("H1 C1 N1 O1", &e).unwrap();
        assert!((carb - 43.005_814).abs() < 1e-6, "{carb}");
        // No-space form, as glyco.txt writes it.
        let hexnac = formula_mass("C8H13NO5", &e).unwrap();
        assert!((hexnac - 203.079_373).abs() < 1e-5, "{hexnac}");
        // Unknown element yields None rather than a silently wrong mass.
        assert!(formula_mass("Xx2", &e).is_none());
    }

    #[test]
    fn targets_route_correctly() {
        assert_eq!(parse_targets("C"), vec!['C']);
        assert_eq!(parse_targets("K or D or E"), vec!['K', 'D', 'E']);
        // X means any residue: no sites, which routes the peak to abundance.
        assert!(parse_targets("X").is_empty());
        // Motif targets are not single residues and must not be read as any.
        assert!(parse_targets("Nxs or Nxt").is_empty());
    }
}
