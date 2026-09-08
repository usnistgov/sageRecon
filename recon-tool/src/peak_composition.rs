//! Peptide-composition readout for discovered peaks — a DIAGNOSTIC, not a gate.
//!
//! **Scope amended 2026-08-25 — this IS now wired into tier assignment.** The header
//! previously read "not wired into annotation, and must not be", citing a NOTES entry
//! that scoped the composition gate out of v0.1.0. That scoping predated any
//! measurement and was deliberately reversed by the user; see NOTES "Annotation has no
//! competing-hypothesis check" and "Step 2 decision rule — route by specificity".
//! It still also reports composition so the two peak-assignment modes can be compared.
//!
//! It asks one question, and only one: do the peptides in this peak *contain* the
//! residues the candidate modification is listed on? That is sequence membership,
//! not site assignment, so it stays compatible with the no-per-residue-localization
//! lock. It cannot say a modification sits on a particular residue, and it is not
//! evidence that it does.
//!
//! The number that carries meaning is ENRICHMENT over the whole run, not the raw
//! fraction. Roughly 9 in 10 tryptic peptides contain at least one K or R, so a
//! raw "92% contain a listed site" says nothing on its own.

use crate::sage_results::Psm;

/// Composition readout for one peak against one candidate's site list.
#[derive(Debug, Clone)]
pub struct SiteSupport {
    /// Residues tested (the candidate's non-hidden Unimod sites, single letters).
    pub sites: Vec<char>,
    /// PSMs in the peak whose peptide contains at least one listed residue.
    pub with_site: usize,
    /// PSMs in the peak.
    pub total: usize,
    /// Same fraction measured over every PSM in the run.
    pub background_fraction: f64,
}

impl SiteSupport {
    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.with_site as f64 / self.total as f64
        }
    }

    /// Peak fraction divided by run-wide fraction. `None` when the background is
    /// zero, which makes the ratio undefined rather than infinite.
    pub fn enrichment(&self) -> Option<f64> {
        if self.background_fraction <= 0.0 {
            None
        } else {
            Some(self.fraction() / self.background_fraction)
        }
    }
}

/// Strip Sage's inline modification brackets so only residues remain.
///
/// Sage writes peptides like `AIETQC[+57.0215]YVVAAAQC[+57.0215]GR`. A naive
/// `contains('C')` is fine, but a naive `contains('N')` would also match the `N`
/// inside a bracketed mod name, so bracket contents are dropped first.
pub fn residues_of(peptide: &str) -> String {
    let mut out = String::with_capacity(peptide.len());
    let mut depth = 0usize;
    for c in peptide.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            c if depth == 0 && c.is_ascii_alphabetic() => out.push(c.to_ascii_uppercase()),
            _ => {}
        }
    }
    out
}

/// Fraction of `psms` whose peptide contains at least one of `sites`.
fn fraction_with_site(psms: &[&Psm], sites: &[char]) -> (usize, f64) {
    if psms.is_empty() {
        return (0, 0.0);
    }
    let n = psms
        .iter()
        .filter(|p| {
            let residues = residues_of(&p.peptide);
            sites.iter().any(|s| residues.contains(*s))
        })
        .count();
    (n, n as f64 / psms.len() as f64)
}

/// Measure one peak's PSMs against a candidate's site list.
///
/// `sites` comes from `UnimodEntry::sites()`, which returns non-hidden sites and
/// may include non-residue tokens such as `N-term`. Those are dropped — a
/// terminus is not a residue and cannot be tested by sequence membership.
pub fn site_support(peak_psms: &[&Psm], all_psms: &[Psm], sites: &[String]) -> SiteSupport {
    let residue_sites: Vec<char> = sites
        .iter()
        .filter(|s| s.len() == 1)
        .filter_map(|s| s.chars().next())
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .collect();

    let all_refs: Vec<&Psm> = all_psms.iter().collect();
    let (_, background_fraction) = fraction_with_site(&all_refs, &residue_sites);
    let (with_site, _) = fraction_with_site(peak_psms, &residue_sites);

    SiteSupport {
        sites: residue_sites,
        with_site,
        total: peak_psms.len(),
        background_fraction,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn residues_strip_inline_mod_brackets() {
        // The bracketed mod text contains no residues of its own.
        assert_eq!(
            residues_of("AIETQC[+57.0215]YVVAAAQC[+57.0215]GR"),
            "AIETQCYVVAAAQCGR"
        );
        assert_eq!(residues_of("PEPTIDE"), "PEPTIDE");
    }

    /// The bracket strip is the whole point: `Deamidated` and `Carbamidomethyl`
    /// both contain letters that are also residue codes, so an unstripped
    /// `contains` would report a site that the peptide does not have.
    #[test]
    fn bracket_text_cannot_fake_a_site() {
        let stripped = residues_of("PEPTIDEK[Carbamidomethyl]");
        assert!(!stripped.contains('C'), "stripped = {stripped}");
        assert!(!stripped.contains('M'), "stripped = {stripped}");
        assert!(stripped.contains('K'));
    }

    #[test]
    fn termini_are_not_residues() {
        // "N-term" must not be read as the residue N.
        let sites = vec!["N-term".to_string(), "K".to_string()];
        let support = site_support(&[], &[], &sites);
        assert_eq!(support.sites, vec!['K']);
    }
}
