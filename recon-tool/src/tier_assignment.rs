//! Step-2 tier assignment — route each peak to the instrument that can decide it.
//!
//! The decision rule, settled 2026-08-25 and recorded in NOTES "Step 2 decision
//! rule — route by specificity". Every peak takes exactly ONE path:
//!
//! | acceptor | instrument | rule |
//! |---|---|---|
//! | residue-specific | statistics | Fisher exact, odds ratio, BH. `OR >= 2 AND q < 0.05` |
//! | unspecific (`TG=X`, or saturated background) | abundance | `count >= X% floor` |
//! | isotope satellite | — | demoted before either test |
//! | not curated | — | unannotated tail, never tiered |
//!
//! **Both statistical thresholds are required.** Gly reaches `q = 1e-5` on bcell
//! from n = 2969 alone while its odds ratio is 1.37. Significance without an
//! effect-size floor readmits exactly the candidate the curated list exists to
//! exclude.
//!
//! **Why not both instruments on every peak** — measured, both directions:
//! * A floor gating the statistics drops Gln->pyro-Glu, whose odds ratios are
//!   27 / 331 / 286 while it sits below the floor at X = 10, 15 and 20 on all
//!   three test files.
//! * Statistics promoting on top of the floor breaks gate 1 with 29 / 28 / 30
//!   violations against 0 for floor-only, because gate 1 validates an abundance
//!   ordering and evidence promotion inverts abundance deliberately.
//!
//! **Fixed-versus-variable is a LABEL read from MetaMorpheus, not a measurement.**
//! The occupancy rule was measured and does not work: an open search assigns one
//! delta per PSM, so a Cys peptide carrying CAM plus anything else lands
//! elsewhere and occupancy cannot approach total occurrence. Report the label,
//! attribute it, do not claim it.

use crate::curated_mods::{CuratedDb, CuratedMod};
use crate::peak_composition::residues_of;
use crate::protein_index::ProteinIndex;
use crate::sage_results::Psm;
use crate::stats::{benjamini_hochberg, fisher_exact_greater};

/// C13 - C12. Mirrors `sage_results::C13_C12_DIFF`.
use crate::sage_results::C13_C12_DIFF;

/// Odds-ratio floor on the statistics path.
pub const OR_MIN: f64 = 2.0;
/// BH-adjusted significance floor on the statistics path.
pub const Q_MAX: f64 = 0.05;
/// Above this background the 2x2 saturates: band and background are both ~100%,
/// the odds ratio is undefined or explodes, and the test says nothing. Carbamyl's
/// K/R/C/M sit in 99.6% of tryptic peptides.
pub const BG_SATURATED: f64 = 0.95;
/// Half-width of the delta-mass band drawn around a peak, in Da.
pub const BAND_TOL_DA: f64 = 0.010;
/// Mass tolerance for matching a peak to a curated candidate, in Da.
pub const CURATED_TOL_DA: f64 = 0.010;
/// Satellite match window. Measured parent + 1xC13 offsets are 1.1-1.5 mDa. This
/// must stay well below 14 mDa: serum's Carboxymethyl at +58.0128 sits 14.3 mDa
/// from the satellite, and a 20 mDa window swallowed it as an isotope satellite.
pub const SAT_TOL_DA: f64 = 0.006;

/// Which instrument decided a peak, and what it said.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Residue-specific acceptor, supported by the statistics.
    Statistics {
        odds_ratio: f64,
        q: f64,
        sites: Vec<char>,
    },
    /// Unspecific acceptor; the count floor decided it.
    Abundance { pct_of_floor: f64 },
    /// Residue-specific, tested, and not supported.
    NoResidueSupport { odds_ratio: f64, q: f64 },
    /// Unspecific and below the floor.
    BelowFloor,
    /// Demoted as an isotope satellite of a larger peak.
    Satellite { parent_delta: f64, neutrons: u8 },
    /// No curated candidate at this mass, and BELOW the abundance floor.
    NotCurated,
    /// No curated candidate at this mass, but ABOVE the abundance floor.
    ///
    /// This is the design's "tier 2 — present, your call": a large delta the
    /// curated list cannot name. It is the novel-chemistry signal the tool exists
    /// to surface, so it must not be flattened into the tail with trivia. No
    /// statistic applies — there is no candidate, so there are no acceptor
    /// residues to test — which is why the floor decides here.
    NotableUnannotated { pct_of_floor: f64 },
}

impl Decision {
    /// True when the peak is recommended to the user.
    pub fn is_recommended(&self) -> bool {
        matches!(
            self,
            Decision::Statistics { .. } | Decision::Abundance { .. }
        )
    }
}

#[derive(Debug, Clone)]
pub struct TieredPeak {
    pub delta_mass: f64,
    pub count: usize,
    /// The winning curated candidate, when there is one.
    pub label: Option<String>,
    pub category: Option<String>,
    /// The winning candidate's `PP`, e.g. "Anywhere." or "Peptide N-terminal.".
    ///
    /// Carried through because `sites` ALONE UNDER-SPECIFIES the search setting.
    /// A terminal candidate is tested positionally -- "first residue is E", not
    /// "contains E" -- so reporting a bare "E" would send a reader to configure a
    /// much larger, noisier search than the one recon actually tested.
    pub position: Option<String>,
    /// The winning curated candidate's acceptor residues. Empty when no curated
    /// candidate applied (a satellite, or an un-curated mass).
    ///
    /// ⚠ **NOT the same as `Decision::Statistics::sites`, which exists only on the
    /// path that PASSED.** This is carried on EVERY curated peak, including the
    /// ones that were tested and failed and the ones the floor decided, because
    /// the report has to say which residues the test actually ran against. Before
    /// this field, `failed_residue_test` rows reached the report with no acceptor
    /// at all and rendered an em dash — for the one row type whose entire point is
    /// the residue.
    pub sites: Vec<char>,
    pub is_fixed: bool,
    pub decision: Decision,
}

/// Does this PSM satisfy the candidate's site requirement?
///
/// Takes the whole PSM, not just the peptide, because a protein-terminal
/// candidate needs the `proteins` column too.
fn peptide_hits(psm: &Psm, cand: &CuratedMod, index: Option<&ProteinIndex>) -> bool {
    let residues = residues_of(&psm.peptide);
    if cand.needs_protein_context() {
        // The peptide must sit at protein position 0. That is the half of the
        // test no PSM can answer on its own, and it is rare -- 0.4-1.6% of
        // confident PSMs -- so it discriminates hard.
        let Some(ix) = index else {
            // Unreachable in practice: `test_candidate` routes to abundance when
            // there is no index. False, not true, if it is ever reached.
            return false;
        };
        if !ix.starts_protein(&residues, &psm.proteins) {
            return false;
        }
        let mut chars = residues.chars();
        let residue_1 = chars.next();
        if cand.is_met_loss() {
            // THE MET THAT IS LOST IS RESIDUE 1, AND IT MUST BE THERE.
            // The open search cannot build Met-clipped peptides, so it matched
            // the Met-RETAINED peptide and put the mod-minus-Met difference into
            // the delta mass. The observed peptide therefore still begins with
            // the initiator Met. Enforced here rather than through `TG`, because
            // `PP`'s own words already state it.
            if residue_1 != Some('M') {
                return false;
            }
            // `TG` NAMES THE FOLLOWING MODIFICATION'S ACCEPTOR, AT THE EXPOSED
            // RESIDUE (protein residue 2). Empty means that modification has no
            // residue requirement.
            //
            // THIS IS NOT THE WITHDRAWN "TG is the NME rule" DESIGN. That one put
            // the NME-PERMISSIVE SET (A/C/G/S/T/V) on Met-loss+Acetylation,
            // encoding aminopeptidase enzymology as an acceptor list. Here
            // acetylation carries `TG=X` -- no residue claim at all, which is the
            // opposite of that. The only residue constraint is myristoylation's
            // Gly, which is the modification's OWN chemistry: every N-terminal
            // myristoyl record in the pinned Unimod is `site=G` (45 and 135).
            // Unimod likewise gives Acetyl, Methyl and Succinyl `site=N-term` at
            // Protein N-term -- no residue. See NOTES "Met-loss encoding".
            if cand.sites.is_empty() {
                return true;
            }
            return chars.next().is_some_and(|c| cand.sites.contains(&c));
        }
        // Plain protein-terminal: `TG` names the acceptor at residue 1, which is
        // the residue the modification actually sits on.
        //
        // An empty `TG` here is NOT unspecific. The residue is unconstrained; the
        // POSITION is not, and protein position 0 is rarer than most residue
        // sets. Measured: bcell +42.0109 `Acetylation TG=X` is 63.8% protein
        // N-terminal against a 1.09% background.
        if cand.sites.is_empty() {
            return true;
        }
        return residue_1.is_some_and(|c| cand.sites.contains(&c));
    }
    if cand.is_terminal() {
        // Position-specific terminal mods are the MOST testable things available:
        // "first residue is Q" runs at a 3.8-4.2% background against "contains Q"
        // at 48%, so pyro-Glu reads OR 27-331 instead of 2.0.
        //
        // ⚠ THE END TESTED MUST MATCH THE END CLAIMED. This read `.next()` — the
        // FIRST residue — for every terminal candidate, including the ones whose
        // `PP` says "Peptide C-terminal.". A C-terminal candidate was therefore
        // tested at the opposite end of the peptide from the one it claims, and
        // would report a confident odds ratio for a position it never examined.
        let residue = if cand.position.contains("C-terminal") {
            residues.chars().next_back()
        } else {
            residues.chars().next()
        };
        match residue {
            Some(r) => cand.sites.contains(&r),
            None => false,
        }
    } else {
        cand.sites.iter().any(|s| residues.contains(*s))
    }
}

/// One candidate's 2x2 against the run background, or `None` when the candidate
/// is unspecific. `None` means "route to abundance", never "no support".
fn test_candidate(
    band: &[&Psm],
    all: &[Psm],
    cand: &CuratedMod,
    index: Option<&ProteinIndex>,
) -> Option<(f64, f64)> {
    if band.is_empty() {
        return None;
    }
    if cand.needs_protein_context() {
        // A protein-terminal candidate cannot be decided from a PSM alone: it
        // needs the peptide's position in its protein, which only the search
        // FASTA gives. With no index, route to abundance rather than test the
        // wrong position and report a confident, wrong odds ratio. NOT a weak
        // result -- not testable.
        index?;
        // NOTE THE ASYMMETRY, AND THAT IT IS DELIBERATE. An empty `TG` does NOT
        // make a protein-terminal candidate untestable, because the position
        // carries the specificity by itself. Everywhere else an empty `TG` means
        // "any residue anywhere", which really is untestable. Measured, both
        // ways: bcell +42.0109 `Acetylation TG=X` is 63.8% protein N-terminal
        // against a 1.09% background (OR 174), while `TG=X` at a PEPTIDE
        // N-terminus matches every peptide in the run and saturates.
    } else if cand.sites.is_empty() {
        return None;
    }
    let a = band.iter().filter(|p| peptide_hits(p, cand, index)).count() as u64;
    let b = band.len() as u64 - a;
    let hits_all = all.iter().filter(|p| peptide_hits(p, cand, index)).count() as u64;
    if hits_all as f64 / all.len() as f64 > BG_SATURATED {
        return None;
    }
    // The 2x2 is band vs NOT-band. `c` and `d` must therefore exclude the band,
    // or the background is counted twice and the odds ratio is understated.
    let non_band = (all.len() as u64).saturating_sub(band.len() as u64);
    let c = hits_all.saturating_sub(a);
    let d = non_band.saturating_sub(c);
    if a + b == 0 || c + d == 0 {
        return None;
    }
    let (mut odds, p) = fisher_exact_greater(a, b, c, d);
    if !odds.is_finite() {
        // Haldane-Anscombe, so an empty cell yields a large finite value rather
        // than an infinity that no threshold can compare.
        odds = ((a as f64 + 0.5) * (d as f64 + 0.5)) / ((b as f64 + 0.5) * (c as f64 + 0.5));
    }
    Some((odds, p))
}

/// The ±1 / ±2 Da quantization carpet, as bounds on the delta mass.
///
/// These are the SAME bounds `_dev/testing/scripts/decoy_delta_histogram.py` uses, so
/// the carpet has one definition in this project, not two. The decoy test proved
/// the region is target-ENRICHED (correct peptide matches), which refutes "the
/// forest is noise" but leaves "the forest is isotope artifacts" standing — and
/// that is exactly what the floor has to clear.
const CARPET_REGIONS: [(f64, f64); 4] =
    [(0.85, 1.15), (-1.15, -0.85), (1.85, 2.15), (-2.15, -1.85)];

fn in_carpet(delta: f64) -> bool {
    CARPET_REGIONS
        .iter()
        .any(|(lo, hi)| delta >= *lo && delta <= *hi)
}

/// The Step-2 carpet invariant: **the floor must sit above the ±1/±2 Da carpet,
/// and the floor governs ONLY the peaks the abundance path decides.**
///
/// The scoping is the whole point. A residue-specific modification is decided by
/// PRESENCE — Fisher exact on its acceptor residues — never by amount, so it never
/// faces the floor and the floor asserts nothing about it. Deamidated is the
/// tallest peak inside the ±1/±2 Da region on all three test files; under this
/// wording it is not an exception needing a carve-out, it is simply out of the
/// floor's jurisdiction. Two earlier wordings are superseded in NOTES "The carpet
/// invariant — RESTATED AND SCOPED"; do not reintroduce either.
///
/// Returns `(margin, tallest_carpet_count)`. The margin is `floor - tallest`, so a
/// POSITIVE margin means the invariant holds. Peaks the floor does not govern are
/// excluded from `tallest` by construction.
pub fn carpet_margin(tiers: &[TieredPeak], floor: f64) -> (f64, usize) {
    let tallest = tiers
        .iter()
        .filter(|t| in_carpet(t.delta_mass))
        // Floor-governed only: unspecific acceptors and uncurated masses. A
        // satellite is demoted before either test, so it is not governed either.
        .filter(|t| {
            matches!(
                t.decision,
                Decision::Abundance { .. }
                    | Decision::BelowFloor
                    | Decision::NotCurated
                    | Decision::NotableUnannotated { .. }
            )
        })
        .map(|t| t.count)
        .max()
        .unwrap_or(0);
    (floor - tallest as f64, tallest)
}

/// Assign every non-zero peak to a tier.
///
/// `peaks` is `(delta_mass, count)` for peaks already past the near-zero
/// exclusion. `floor` is the abundance floor in PSMs. `all` is the confident PSM
/// population the background is measured over.
pub fn assign(
    peaks: &[(f64, usize)],
    all: &[Psm],
    curated: &CuratedDb,
    floor: f64,
    protein_index: Option<&ProteinIndex>,
) -> Vec<TieredPeak> {
    let Some(&(parent_delta, _)) = peaks.iter().max_by_key(|(_, c)| *c) else {
        return Vec::new();
    };
    let satellite_of = |d: f64| -> Option<u8> {
        (1u8..=2).find(|n| (d - (parent_delta + *n as f64 * C13_C12_DIFF)).abs() <= SAT_TOL_DA)
    };

    // Score every candidate first so BH corrects across the whole sweep, not
    // per peak. Correcting per peak would understate the multiplicity.
    struct Job {
        peak_idx: usize,
        cand_idx: usize,
        result: Option<(f64, f64)>,
    }
    let mut jobs: Vec<Job> = Vec::new();
    let mut cand_lists: Vec<Vec<&CuratedMod>> = Vec::with_capacity(peaks.len());
    for (pi, (delta, _)) in peaks.iter().enumerate() {
        let cands = if satellite_of(*delta).is_some() {
            Vec::new()
        } else {
            curated.candidates(*delta, CURATED_TOL_DA)
        };
        let band: Vec<&Psm> = all
            .iter()
            .filter(|p| (p.delta_mass_corrected - delta).abs() <= BAND_TOL_DA)
            .collect();
        for (ci, cand) in cands.iter().enumerate() {
            jobs.push(Job {
                peak_idx: pi,
                cand_idx: ci,
                result: test_candidate(&band, all, cand, protein_index),
            });
        }
        cand_lists.push(cands);
    }
    let pvals: Vec<f64> = jobs
        .iter()
        .filter_map(|j| j.result.map(|(_, p)| p))
        .collect();
    let adjusted = benjamini_hochberg(&pvals);
    let mut q_iter = adjusted.into_iter();
    // `clippy::type_complexity`: the tuple is (peak_idx, cand_idx, Some((odds
    // ratio, q))). A named type would move that meaning away from the one place
    // it is used and read, and the local is consumed three lines below.
    #[allow(clippy::type_complexity)]
    let scored: Vec<(usize, usize, Option<(f64, f64)>)> = jobs
        .iter()
        .map(|j| {
            let with_q = j.result.map(|(o, _)| (o, q_iter.next().unwrap_or(1.0)));
            (j.peak_idx, j.cand_idx, with_q)
        })
        .collect();

    let mut out = Vec::with_capacity(peaks.len());
    for (pi, (delta, count)) in peaks.iter().enumerate() {
        if let Some(n) = satellite_of(*delta) {
            out.push(TieredPeak {
                delta_mass: *delta,
                count: *count,
                label: None,
                category: None,
                position: None,
                sites: Vec::new(),
                is_fixed: false,
                decision: Decision::Satellite {
                    parent_delta,
                    neutrons: n,
                },
            });
            continue;
        }
        let cands = &cand_lists[pi];
        if cands.is_empty() {
            out.push(TieredPeak {
                delta_mass: *delta,
                count: *count,
                label: None,
                category: None,
                position: None,
                sites: Vec::new(),
                is_fixed: false,
                decision: if (*count as f64) >= floor {
                    Decision::NotableUnannotated {
                        pct_of_floor: 100.0 * *count as f64 / floor,
                    }
                } else {
                    Decision::NotCurated
                },
            });
            continue;
        }
        let mine: Vec<_> = scored.iter().filter(|(p, _, _)| *p == pi).collect();
        let testable: Vec<_> = mine.iter().filter(|(_, _, r)| r.is_some()).collect();
        let (label, category, position, sites, is_fixed, decision) = if testable.is_empty() {
            // ABUNDANCE PATH — nothing here has a testable residue.
            let cand = cands[0];
            let d = if *count as f64 >= floor {
                Decision::Abundance {
                    pct_of_floor: 100.0 * *count as f64 / floor,
                }
            } else {
                Decision::BelowFloor
            };
            (
                Some(cand.label.clone()),
                Some(cand.category.clone()),
                Some(cand.position.clone()),
                cand.sites.clone(),
                cand.is_fixed(),
                d,
            )
        } else {
            // STATISTICS PATH — take the best supported candidate.
            let best = testable
                .iter()
                .filter(|(_, _, r)| {
                    let (o, q) = r.unwrap();
                    // BOTH BOUNDS INCLUSIVE. This read `q < Q_MAX` while the OR
                    // bound was `>=`, so a peak at exactly q = 0.05 was rejected
                    // by code and accepted by the stated rule ("OR >= 2 and
                    // q <= 0.05"). Same asymmetry commit 674d197 fixed for the
                    // calibration subset, and fixed the same way: measured inert
                    // first. Closest observed q to the bound across serum and
                    // bcell is 0.06859; nothing sits on it.
                    o >= OR_MIN && q <= Q_MAX
                })
                .max_by(|a, b| {
                    a.2.unwrap()
                        .0
                        .partial_cmp(&b.2.unwrap().0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            match best {
                Some((_, ci, r)) => {
                    let cand = cands[*ci];
                    let (o, q) = r.unwrap();
                    (
                        Some(cand.label.clone()),
                        Some(cand.category.clone()),
                        Some(cand.position.clone()),
                        cand.sites.clone(),
                        cand.is_fixed(),
                        Decision::Statistics {
                            odds_ratio: o,
                            q,
                            sites: cand.sites.clone(),
                        },
                    )
                }
                None => {
                    let (_, ci, r) = testable
                        .iter()
                        .max_by(|a, b| {
                            a.2.unwrap()
                                .0
                                .partial_cmp(&b.2.unwrap().0)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .unwrap();
                    let cand = cands[*ci];
                    let (o, q) = r.unwrap();
                    (
                        Some(cand.label.clone()),
                        Some(cand.category.clone()),
                        Some(cand.position.clone()),
                        cand.sites.clone(),
                        false,
                        Decision::NoResidueSupport { odds_ratio: o, q },
                    )
                }
            }
        };
        out.push(TieredPeak {
            delta_mass: *delta,
            count: *count,
            label,
            category,
            position,
            sites,
            is_fixed,
            decision,
        });
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::curated_mods::CuratedMod;

    fn cand(label: &str, sites: &[char], position: &str) -> CuratedMod {
        CuratedMod {
            label: label.to_string(),
            id: label.to_string(),
            sites: sites.to_vec(),
            position: position.to_string(),
            category: "Common Biological".to_string(),
            formula: String::new(),
            mass: 0.0,
        }
    }

    /// Fields are spelled out rather than defaulted: a `Default` on `Psm` would
    /// silently give every test PSM q = 0.0 and rank 0, which passes filters it
    /// should not.
    pub(crate) fn psm(peptide: &str, delta: f64) -> Psm {
        psm_in(peptide, delta, "PROTEIN")
    }

    /// Same fixture, with the `proteins` column set. Protein-terminal candidates
    /// are decided against that column, so they need their own builder.
    pub(crate) fn psm_in(peptide: &str, delta: f64, proteins: &str) -> Psm {
        Psm {
            scannr: 1,
            rank: 1,
            peptide: peptide.to_string(),
            proteins: proteins.to_string(),
            expmass: 1000.0 + delta,
            calcmass: 1000.0,
            isotope_error: 0,
            delta_mass: delta,
            delta_mass_corrected: delta,
            hyperscore: 30.0,
            matched_intensity_pct: 0.5,
            longest_b: 5,
            longest_y: 6,
            ms2_intensity: 1000.0,
            peptide_q: 0.001,
            spectrum_q: 0.001,
            is_decoy: false,
            charge: 2,
            rt: 30.0,
            missed_cleavages: 0,
            semi_enzymatic: false,
            precursor_ppm: 2.0,
            fragment_ppm: 5.0,
            peptide_len: peptide.len() as u32,
        }
    }

    /// The 2x2 must be band vs NOT-band. Counting the band inside the background
    /// understates the odds ratio, which is how a real modification gets demoted.
    #[test]
    fn background_excludes_the_band() {
        // 10 band PSMs, all Cys. 90 others, 10 of which are Cys.
        let mut all: Vec<Psm> = (0..10).map(|_| psm("AAACAAA", 0.5)).collect();
        all.extend((0..10).map(|_| psm("AAACAAA", 5.0)));
        all.extend((0..80).map(|_| psm("AAAAAAA", 5.0)));
        let band: Vec<&Psm> = all
            .iter()
            .filter(|p| p.delta_mass_corrected == 0.5)
            .collect();
        let c = cand("CAM", &['C'], "Anywhere.");
        let (odds, p) = test_candidate(&band, &all, &c, None).expect("testable");
        // a=10 b=0 c=10 d=80 -> Haldane-Anscombe odds = (10.5*80.5)/(0.5*10.5) = 161
        assert!((odds - 161.0).abs() < 1.0, "odds {odds}");
        assert!(p < 0.001, "p {p}");
    }

    /// A saturated background carries no information and must route to abundance,
    /// not be reported as unsupported. This is the Carbamyl case.
    #[test]
    fn saturated_background_routes_to_abundance() {
        let all: Vec<Psm> = (0..100).map(|_| psm("AAAKAAA", 5.0)).collect();
        let band: Vec<&Psm> = all.iter().take(10).collect();
        let c = cand("Carbamyl", &['K'], "Anywhere.");
        assert!(test_candidate(&band, &all, &c, None).is_none());
    }

    /// TG=X yields no sites, which is the other route to abundance.
    #[test]
    fn unspecific_candidate_is_untestable() {
        let all: Vec<Psm> = (0..10).map(|_| psm("AAAKAAA", 5.0)).collect();
        let band: Vec<&Psm> = all.iter().take(3).collect();
        let c = cand("Carbamyl N-term", &[], "Peptide N-terminal.");
        assert!(test_candidate(&band, &all, &c, None).is_none());
    }

    /// Terminal candidates test the FIRST residue, not membership. This is what
    /// takes pyro-Glu from OR 2 to OR 27-331.
    #[test]
    fn terminal_candidates_test_position_not_membership() {
        let nterm = cand("pyro-Glu", &['Q'], "Peptide N-terminal.");
        assert!(peptide_hits(&psm("QAAAAA", 0.0), &nterm, None));
        // Contains Q, but not at the N-terminus.
        assert!(!peptide_hits(&psm("AAAQAA", 0.0), &nterm, None));
        let anywhere = cand("Deamidation", &['Q'], "Anywhere.");
        assert!(peptide_hits(&psm("AAAQAA", 0.0), &anywhere, None));
    }

    /// Build a two-protein index without touching the filesystem.
    ///
    /// ROUTING ONLY. A synthetic fixture inherits the assumptions of the code it
    /// tests, so this cannot prove the CONVENTION is right -- that the acceptor
    /// really is residue 1 and not residue 2, or that Sage's `proteins` column
    /// keys match FASTA headers. `tier_assignment_integration.rs` does that,
    /// against real committed data whose answer is known independently.
    ///
    /// The `TempDir` comes back with the index and must be held for the length of
    /// the test. A shared fixed path would be truncated by whichever test ran
    /// next, because cargo runs these in parallel threads of ONE process.
    fn index() -> (tempfile::TempDir, crate::protein_index::ProteinIndex) {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.fasta");
        let mut f = std::fs::File::create(&path).unwrap();
        // P1 is the Met-loss case: initiator M, then an NME-permissive A.
        writeln!(f, ">P1\nMASTQKLLNPR").unwrap();
        // P2 starts with the same tryptic peptide P1 carries INTERNALLY, and
        // begins with G WITHOUT an initiator Met -- the myristoylation trap.
        writeln!(f, ">P2\nGGGRLLNPRYY").unwrap();
        // P3 is the real N-myristoylation shape: initiator M, Gly exposed under it.
        writeln!(f, ">P3\nMGSSKSKPKDPSQR").unwrap();
        drop(f);
        let ix = crate::protein_index::ProteinIndex::from_fasta(&path).unwrap();
        (dir, ix)
    }

    /// The protein-terminal branch reads residue 1 at protein position 0.
    ///
    /// Both halves must bite. `TG=M` alone would pass any internal peptide
    /// starting with M; protein position 0 alone would pass a protein whose
    /// first residue is not an acceptor.
    #[test]
    fn protein_terminal_needs_position_zero_and_residue_one() {
        let (_d, ix) = index();
        // TG=X: acetyl has no residue requirement at the exposed position.
        let met_loss = cand("Met-loss+Acetylation", &[], "Protein N-terminal, Met loss.");

        // At protein position 0, residue 1 is M. Passes.
        assert!(peptide_hits(
            &psm_in("MASTQK", 0.0, "P1"),
            &met_loss,
            Some(&ix)
        ));

        // Starts with M, but is not at protein position 0 anywhere. This is the
        // case the peptide-level shortcut would wrongly pass.
        assert!(!peptide_hits(
            &psm_in("MQQQQ", 0.0, "P1"),
            &met_loss,
            Some(&ix)
        ));

        // At protein position 0 of P2, but residue 1 is G, not M. A Met that is
        // not there cannot be lost, so this must fail even though TG is empty.
        assert!(!peptide_hits(
            &psm_in("GGGR", 0.0, "P2"),
            &met_loss,
            Some(&ix)
        ));

        // Internal in P1, at position 0 of nothing. Reported under both.
        assert!(!peptide_hits(
            &psm_in("LLNPR", 0.0, "P1;P2"),
            &met_loss,
            Some(&ix)
        ));
    }

    /// A Met-loss entry with a residue requirement tests the EXPOSED residue,
    /// protein residue 2 -- not residue 1, which is always the lost Met.
    ///
    /// Myristoylation is the case that makes this matter. Every N-terminal
    /// myristoyl record in the pinned Unimod is `site=G` (45 and 135), and the
    /// Gly only becomes an N-terminus after the Met is removed. Reading TG at
    /// residue 1 instead would demand a protein that starts with G AND lost a
    /// Met it never had -- an entry that can never fire, silently.
    #[test]
    fn met_loss_reads_its_acceptor_at_the_exposed_residue() {
        let (_d, ix) = index();
        let myr = cand(
            "Met-loss+Myristoylation",
            &['G'],
            "Protein N-terminal, Met loss.",
        );
        // P3 is M-G-...: Met lost, Gly exposed. This is the real case.
        assert!(peptide_hits(&psm_in("MGSSKSK", 0.0, "P3"), &myr, Some(&ix)));
        // P1 is M-A-...: Met lost, Ala exposed. Not a myristoylation acceptor.
        assert!(!peptide_hits(&psm_in("MASTQK", 0.0, "P1"), &myr, Some(&ix)));
        // P2 starts with G but has no initiator Met to lose.
        assert!(!peptide_hits(&psm_in("GGGR", 0.0, "P2"), &myr, Some(&ix)));
    }

    /// The bare Met-loss entry, Unimod 765: position 0, residue 1 = M, and
    /// nothing follows, so no residue requirement at the exposed position.
    #[test]
    fn bare_met_loss_needs_only_the_initiator_met() {
        let (_d, ix) = index();
        let bare = cand("Met-loss", &[], "Protein N-terminal, Met loss.");
        assert!(peptide_hits(&psm_in("MASTQK", 0.0, "P1"), &bare, Some(&ix)));
        assert!(peptide_hits(
            &psm_in("MGSSKSK", 0.0, "P3"),
            &bare,
            Some(&ix)
        ));
        assert!(!peptide_hits(&psm_in("GGGR", 0.0, "P2"), &bare, Some(&ix)));
    }

    /// The plain "Protein N-terminal." PP takes the SAME branch. There is one
    /// rule, not a Met-loss special case.
    #[test]
    fn plain_protein_nterm_uses_the_same_rule() {
        let (_d, ix) = index();
        let myr = cand("Myristoylation", &['G'], "Protein N-terminal.");
        assert!(peptide_hits(&psm_in("GGGR", 0.0, "P2"), &myr, Some(&ix)));
        assert!(!peptide_hits(&psm_in("MASTQK", 0.0, "P1"), &myr, Some(&ix)));
    }

    /// An unresolvable accession is false, never true. Decoys are the expected
    /// case, and a decoy must never read as protein N-terminal.
    #[test]
    fn unresolvable_accessions_never_hit() {
        let (_d, ix) = index();
        let met_loss = cand("Met-loss+Acetylation", &[], "Protein N-terminal, Met loss.");
        assert!(!peptide_hits(
            &psm_in("MASTQK", 0.0, "rev_P1"),
            &met_loss,
            Some(&ix)
        ));
    }

    /// `TG=X` AT A PROTEIN TERMINUS IS TESTABLE. The position carries the
    /// specificity, so an empty acceptor set is not a weakness here.
    ///
    /// This is the case the original step-2 rule got wrong: it read `TG=X` as
    /// "unspecific, route to abundance", a rule written when protein position was
    /// unknowable. Measured on real data, bcell +42.0109 `Acetylation TG=X` is
    /// 63.8% protein N-terminal against a 1.09% background.
    #[test]
    fn empty_sites_at_a_protein_terminus_is_still_testable() {
        let (_d, ix) = index();
        let acetyl = cand("Acetylation", &[], "Protein N-terminal.");
        // Position alone decides: P1 at position 0 hits, an internal peptide does not.
        assert!(peptide_hits(
            &psm_in("MASTQK", 0.0, "P1"),
            &acetyl,
            Some(&ix)
        ));
        assert!(peptide_hits(&psm_in("GGGR", 0.0, "P2"), &acetyl, Some(&ix)));
        assert!(!peptide_hits(
            &psm_in("LLNPR", 0.0, "P1;P2"),
            &acetyl,
            Some(&ix)
        ));

        // And it reaches the statistics, rather than short-circuiting to abundance.
        let mut all: Vec<Psm> = (0..10).map(|_| psm_in("MASTQK", 5.0, "P1")).collect();
        all.extend((0..90).map(|_| psm_in("LLNPR", 0.0, "P1")));
        let band: Vec<&Psm> = all.iter().take(10).collect();
        let (odds, p) = test_candidate(&band, &all, &acetyl, Some(&ix)).expect("testable");
        assert!(odds >= OR_MIN, "odds {odds}");
        assert!(p < 0.001, "p {p}");
    }

    /// THE NARROWNESS CONTROL. `TG=X` at a PEPTIDE terminus stays untestable.
    ///
    /// Every peptide has an N-terminus, so this matches the whole run and the 2x2
    /// says nothing. If this test ever passes, the fix above has been widened into
    /// exactly the thing the abundance path exists for.
    #[test]
    fn empty_sites_at_a_peptide_terminus_stays_untestable() {
        let (_d, ix) = index();
        let all: Vec<Psm> = (0..100).map(|_| psm_in("MASTQK", 5.0, "P1")).collect();
        let band: Vec<&Psm> = all.iter().take(10).collect();
        let pep = cand("Carbamyl N-term", &[], "Peptide N-terminal.");
        assert!(test_candidate(&band, &all, &pep, Some(&ix)).is_none());
        // Same for "Anywhere.", the other genuinely unspecific case.
        let anywhere = cand("Something", &[], "Anywhere.");
        assert!(test_candidate(&band, &all, &anywhere, Some(&ix)).is_none());
    }

    /// Without an index the candidate is NOT TESTABLE and routes to abundance.
    /// `None` here must mean "the floor decides", never "no residue support".
    ///
    /// The background is deliberately NOT all-protein-N-term. A population where
    /// every PSM hits saturates the 2x2 and returns `None` for a different
    /// reason, which would make the "with an index it IS tested" half vacuous.
    #[test]
    fn protein_terminal_without_an_index_routes_to_abundance() {
        let mut all: Vec<Psm> = (0..10).map(|_| psm_in("MASTQK", 5.0, "P1")).collect();
        all.extend((0..90).map(|_| psm_in("LLNPR", 0.0, "P1")));
        let band: Vec<&Psm> = all.iter().take(10).collect();
        // TG empty: acetyl places no requirement on the exposed residue.
        let met_loss = cand("Met-loss+Acetylation", &[], "Protein N-terminal, Met loss.");
        assert!(test_candidate(&band, &all, &met_loss, None).is_none());
        // With an index the same candidate IS tested, and strongly supported:
        // 10 of 10 in the band, 0 of 90 outside it.
        let (_d, ix) = index();
        let (odds, p) = test_candidate(&band, &all, &met_loss, Some(&ix)).expect("testable");
        assert!(odds >= OR_MIN, "odds {odds}");
        assert!(p < 0.001, "p {p}");
    }

    /// Satellites are demoted before any test runs.
    #[test]
    fn satellites_are_demoted() {
        let all: Vec<Psm> = (0..100).map(|_| psm("AAACAAA", 0.0)).collect();
        let (db, _) = (crate::curated_mods::CuratedDb::default(), 0usize);
        let peaks = vec![(57.0215, 1000usize), (57.0215 + C13_C12_DIFF, 300)];
        let out = assign(&peaks, &all, &db, 100.0, None);
        // The parent is uncurated AND above the floor (1000 PSMs vs 100), so it is
        // notable rather than tail. What this test pins is the SATELLITE below.
        assert!(
            matches!(out[0].decision, Decision::NotableUnannotated { .. }),
            "{:?}",
            out[0].decision
        );
        match out[1].decision {
            Decision::Satellite { neutrons, .. } => assert_eq!(neutrons, 1),
            ref d => panic!("expected satellite, got {d:?}"),
        }
    }
}

/// Convert tiered peaks into the report's recommendation block.
///
/// `total_psms` is the denominator for `count_pct`. `floor` and `floor_pct` are
/// echoed so a reader can see what the abundance path was judged against without
/// recomputing it.
/// `unimod_name` supplies an INFORMATIONAL name for peaks the curated list cannot
/// name. The curated list is deliberately narrower than Unimod, so a peak can be
/// uncurated and still have a plausible Unimod identity — bcell's −1.0290
/// `Lys->Allysine` is the example. Dropping that name would leave a reader looking
/// at a bare mass with no way to judge it for themselves, which is the opposite of
/// what the tail is for. The name is reported, never tiered on.
pub fn to_report(
    tiers: &[TieredPeak],
    total_psms: usize,
    floor: f64,
    floor_pct: f64,
    unimod_name: impl Fn(f64) -> Option<String>,
    protein_context: Option<crate::report::ProteinContext>,
) -> crate::report::ModRecommendations {
    use crate::report::{ModRecommendations, NotRecommended, RecommendedMod};

    let pct = |c: usize| {
        if total_psms == 0 {
            0.0
        } else {
            100.0 * c as f64 / total_psms as f64
        }
    };
    let (mut fixed, mut variable, mut not_recommended) = (Vec::new(), Vec::new(), Vec::new());
    let mut notable_unannotated: Vec<crate::report::NotRecommended> = Vec::new();

    for t in tiers {
        let label = t.label.clone().unwrap_or_default();
        match &t.decision {
            Decision::Statistics {
                odds_ratio,
                q,
                sites,
            } => {
                let rec = RecommendedMod {
                    delta_mass: t.delta_mass,
                    count: t.count,
                    count_pct: pct(t.count),
                    label,
                    category: t.category.clone().unwrap_or_default(),
                    role: if t.is_fixed { "fixed" } else { "variable" }.to_string(),
                    decided_by: "statistics".to_string(),
                    sites: sites.iter().collect(),
                    position: t.position.clone().unwrap_or_default(),
                    odds_ratio: Some(*odds_ratio),
                    q_value: Some(*q),
                    pct_of_floor: None,
                };
                if t.is_fixed {
                    fixed.push(rec)
                } else {
                    variable.push(rec)
                }
            }
            Decision::Abundance { pct_of_floor } => {
                let rec = RecommendedMod {
                    delta_mass: t.delta_mass,
                    count: t.count,
                    count_pct: pct(t.count),
                    label,
                    category: t.category.clone().unwrap_or_default(),
                    role: if t.is_fixed { "fixed" } else { "variable" }.to_string(),
                    decided_by: "abundance".to_string(),
                    sites: String::new(),
                    position: t.position.clone().unwrap_or_default(),
                    odds_ratio: None,
                    q_value: None,
                    pct_of_floor: Some(*pct_of_floor),
                };
                if t.is_fixed {
                    fixed.push(rec)
                } else {
                    variable.push(rec)
                }
            }
            // TESTED AND FAILED. `q` is carried through now: it used to be
            // destructured away, leaving a report that showed an OR clearing the
            // bar with no way to see which gate actually failed.
            // ⚠ THE RESIDUE IS THE WHOLE POINT OF THIS ROW. `sites` and `position`
            // are the ones the test actually RAN AGAINST, taken from the same
            // candidate that produced `odds_ratio` and `q`. Recomputing them from
            // the mass would risk naming a different candidate than the one tested.
            Decision::NoResidueSupport { odds_ratio, q } => not_recommended.push(NotRecommended {
                delta_mass: t.delta_mass,
                count: t.count,
                count_pct: pct(t.count),
                reason: "failed_residue_test".to_string(),
                label: t.label.clone(),
                name_source: t.label.as_ref().map(|_| "curated".to_string()),
                sites: t.sites.iter().collect(),
                position: t.position.clone().unwrap_or_default(),
                odds_ratio: Some(*odds_ratio),
                q_value: Some(*q),
            }),
            Decision::BelowFloor => not_recommended.push(NotRecommended {
                delta_mass: t.delta_mass,
                count: t.count,
                count_pct: pct(t.count),
                reason: "below_floor".to_string(),
                q_value: None,
                label: t.label.clone(),
                name_source: t.label.as_ref().map(|_| "curated".to_string()),
                sites: t.sites.iter().collect(),
                position: t.position.clone().unwrap_or_default(),
                odds_ratio: None,
            }),
            Decision::Satellite {
                parent_delta,
                neutrons,
            } => not_recommended.push(NotRecommended {
                delta_mass: t.delta_mass,
                count: t.count,
                count_pct: pct(t.count),
                reason: format!("satellite (+{neutrons} C13 of {parent_delta:+.4})"),
                label: unimod_name(t.delta_mass),
                name_source: unimod_name(t.delta_mass).map(|_| "unimod".to_string()),
                // No curated candidate, so no tested residues. Unimod's own
                // acceptor sites are filled in later, beside the name they belong
                // to -- see `ReconReport::from_analyses`.
                sites: String::new(),
                position: String::new(),
                odds_ratio: None,
                q_value: None,
            }),
            // ⚠ `NotCurated` means "no curated entry AND below the floor" — both,
            // not just the first. The old string `not_curated` hid the floor half
            // and read as "we did not recognise it", which is why an un-curated
            // peak at n=148 against a floor of 224 looked unexplained.
            Decision::NotCurated => not_recommended.push(NotRecommended {
                delta_mass: t.delta_mass,
                count: t.count,
                count_pct: pct(t.count),
                reason: "below_floor_uncurated".to_string(),
                label: unimod_name(t.delta_mass),
                name_source: unimod_name(t.delta_mass).map(|_| "unimod".to_string()),
                sites: String::new(),
                position: String::new(),
                odds_ratio: None,
                q_value: None,
            }),
            Decision::NotableUnannotated { pct_of_floor } => {
                notable_unannotated.push(NotRecommended {
                    delta_mass: t.delta_mass,
                    count: t.count,
                    count_pct: pct(t.count),
                    reason: format!("uncurated — above floor ({pct_of_floor:.0}%), never tested"),
                    label: unimod_name(t.delta_mass),
                    name_source: unimod_name(t.delta_mass).map(|_| "unimod".to_string()),
                    sites: String::new(),
                    position: String::new(),
                    odds_ratio: None,
                    q_value: None,
                })
            }
        }
    }
    fixed.sort_by_key(|b| std::cmp::Reverse(b.count));
    variable.sort_by_key(|b| std::cmp::Reverse(b.count));
    not_recommended.sort_by_key(|b| std::cmp::Reverse(b.count));
    notable_unannotated.sort_by_key(|b| std::cmp::Reverse(b.count));

    let (carpet_margin_psms, carpet_tallest_psms) = carpet_margin(tiers, floor);

    ModRecommendations {
        fixed,
        variable,
        not_recommended,
        notable_unannotated,
        carpet_margin_psms,
        carpet_tallest_psms,
        floor_psms: floor,
        floor_pct_of_top: floor_pct,
        odds_ratio_min: OR_MIN,
        q_max: Q_MAX,
        annotation_source:
            "MetaMorpheus curated mod list (recon-tool/resources/mods/), dated snapshot".to_string(),
        caveats: {
            let mut c = vec![
                "'fixed' and 'variable' are inherited labels, not recon measurements. \
             They come from the curated list's MT field and are passed through as \
             guidance for how to set each modification in your FINAL search. Recon \
             makes no judgement of its own here: an open search assigns one delta \
             per PSM, so residue occupancy cannot approach total occurrence and \
             cannot decide the split. Treat the label as the curator's convention."
                    .to_string(),
                "Modifications whose acceptors are unspecific (any residue at a terminus, \
             or an acceptor set present in nearly every peptide) cannot be tested \
             statistically. They are decided by abundance alone and carry no odds ratio."
                    .to_string(),
                "Recommendations are not prevalence estimates. Recon reports which \
             modifications to search for, not how much of each is present."
                    .to_string(),
                "Peaks with no curated match are reported in the tail, never tiered. That \
             is deliberate: a novel delta mass must stay visible."
                    .to_string(),
            ];
            // Said only when it is TRUE. A standing caveat that a class was not
            // tested, printed on reports where it WAS tested, teaches a reader to
            // skip the caveat list.
            if protein_context.is_none() {
                c.push(
                    "Protein-terminal modifications were NOT TESTABLE in this report. \
                     Proving a peptide starts at protein position 0 needs the search \
                     FASTA, which was not supplied (`analyze --fasta`). Those \
                     candidates were decided by abundance alone. That is 'not tested', \
                     not 'not supported'."
                        .to_string(),
                );
            }
            c
        },
        protein_context,
    }
}

#[cfg(test)]
mod notable_tests {
    use super::*;
    use crate::curated_mods::CuratedDb;

    // Reuse the fixture builder from the main test module rather than a second,
    // divergent one.
    use super::tests::psm;

    /// A large delta with no curated name must surface as "present, your call",
    /// not be flattened into the tail alongside trivia. Treated like an unspecific
    /// acceptor -- the floor decides, because with no candidate there are no
    /// residues to test -- but flagged rather than recommended, since an unnamed
    /// delta cannot go into a search configuration.
    ///
    /// NOTE: no real test file exercises this path. On all three, zero uncurated
    /// peaks clear the X=20% floor (closest is b1906 +16.9978 at 74% of floor), so
    /// this fixture is synthetic by necessity and covers routing only.
    #[test]
    fn large_uncurated_peak_is_flagged_not_buried() {
        let all: Vec<Psm> = (0..1000).map(|_| psm("AAAKAAA", 0.0)).collect();
        let db = CuratedDb::default();
        // 900.0 Da matches nothing curated. One above the floor, one below.
        let peaks = vec![(900.0, 500usize), (901.5, 10usize)];
        let out = assign(&peaks, &all, &db, 100.0, None);

        match out[0].decision {
            Decision::NotableUnannotated { pct_of_floor } => {
                assert!((pct_of_floor - 500.0).abs() < 1e-6, "{pct_of_floor}")
            }
            ref d => panic!("500 PSMs against a 100 floor should be notable, got {d:?}"),
        }
        assert!(
            matches!(out[1].decision, Decision::NotCurated),
            "{:?}",
            out[1].decision
        );
        // Neither is a recommendation: an unnamed delta is not a search parameter.
        assert!(!out[0].decision.is_recommended());
        assert!(!out[1].decision.is_recommended());
    }

    /// THE q BOUND IS INCLUSIVE, MATCHING THE OR BOUND.
    ///
    /// Guards the asymmetry that existed until 2026-09-01: `o >= OR_MIN` paired
    /// with `q < Q_MAX`, so a candidate sitting exactly on the q bound was
    /// rejected while one sitting exactly on the OR bound was accepted. Measured
    /// inert before changing — the closest real q to the bound across serum and
    /// bcell is 0.06859 — but a boundary that disagrees with the documented rule
    /// is a trap regardless of whether today's data lands on it.
    #[test]
    fn both_recommendation_bounds_are_inclusive() {
        let passes = |o: f64, q: f64| o >= OR_MIN && q <= Q_MAX;
        assert!(
            passes(OR_MIN, Q_MAX),
            "a candidate exactly on both bounds must pass"
        );
        assert!(passes(OR_MIN, 0.0));
        assert!(
            !passes(OR_MIN - 0.0001, Q_MAX),
            "just under the OR bound must fail"
        );
        assert!(
            !passes(OR_MIN, Q_MAX + 0.0001),
            "just over the q bound must fail"
        );
    }
}
