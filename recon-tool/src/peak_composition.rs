//! Peptide residue helper: `residues_of` strips Sage's inline modification
//! brackets so only residues remain. `tier_assignment`, `digestion`,
//! `protein_index` and the report builder use it.
//!
//! This module once also held `site_support`, a per-peak composition readout
//! (do the peak's peptides contain the candidate's residues, and how enriched
//! is that over the run). Its only caller was the removed
//! `compare-peak-assignment` subcommand, and it was removed with it
//! (2026-09-24). Tier assignment tests residues with its own 2x2 counts.

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
}
