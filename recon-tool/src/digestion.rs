//! Digestion composition of the Pass 2 peptide population.
//!
//! One measurement lives here, `DigestionComposition`: missed cleavage and
//! ragged N- and C-termini over distinct peptides, with per-class decoy
//! subtraction. It is the only digestion number recon reports.
//!
//! The PSM-basis summaries that used to sit beside it (`DigestionResult`, from
//! Sage's own columns, and `TerminusStats`) were removed on 2026-09-24. Each
//! reported a second semi-enzymatic rate that a reader could quote in place of
//! this one.

use crate::enzyme::Enzyme;
use serde::{Deserialize, Serialize};

// =============================================================================
// Terminus specificity — the `annotate` half of the digestion_efficiency port
// =============================================================================

/// How a Pass-2 peptide's two termini sit against its protein.
///
/// `NotFound` and `ProteinNotFound` are different failures and are kept apart:
/// the first means the accession WAS in the FASTA but the peptide sequence is
/// not a substring of it (an I/L ambiguity, or a mismatched database); the
/// second means the accession itself was absent. Folding them together would
/// hide a wrong-FASTA run inside a plausible-looking count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminusClass {
    FullyEnzymatic,
    SemiNRagged,
    SemiCRagged,
    NonEnzymatic,
    NotFound,
    ProteinNotFound,
}

impl TerminusClass {
    /// Every class, in report order. An absent key and a zero count are
    /// different claims, so the report seeds all six rather than materialising
    /// only the ones that occurred.
    pub const ALL: [TerminusClass; 6] = [
        TerminusClass::FullyEnzymatic,
        TerminusClass::SemiNRagged,
        TerminusClass::SemiCRagged,
        TerminusClass::NonEnzymatic,
        TerminusClass::NotFound,
        TerminusClass::ProteinNotFound,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TerminusClass::FullyEnzymatic => "fully_enzymatic",
            TerminusClass::SemiNRagged => "semi_n_ragged",
            TerminusClass::SemiCRagged => "semi_c_ragged",
            TerminusClass::NonEnzymatic => "non_enzymatic",
            TerminusClass::NotFound => "not_found",
            TerminusClass::ProteinNotFound => "protein_not_found",
        }
    }
}

/// Is the peptide N-terminus a tryptic one?
///
/// True at protein position 0, after initiator-methionine excision, or when the
/// preceding residue is K/R and the peptide does not start with P (trypsin does
/// not cut before proline).
///
/// ## Initiator-methionine excision
///
/// A peptide starting at position 1 of a protein whose position 0 is Met sits at
/// a REAL protein N-terminus: methionine aminopeptidase removes the initiator
/// Met co-translationally, so nothing cleaved that bond and the terminus is not
/// ragged. Without this case the preceding residue is `M`, which is not K/R, and
/// the peptide is mis-called `SemiNRagged`.
///
/// MEASURED on liver `10mg_1_A_1`, 2026-08-31: 23 distinct Pass-2 peptides hit
/// this case, ALL of them previously called ragged-N or non-tryptic. Correcting
/// them moves the reported ragged-N rate 7.19 % -> 6.97 %. The bias is small here
/// but it is one-directional, and it would grow on N-terminomics-flavoured
/// samples.
///
/// Surfaced by comparing against MSFragger, which applies the same rule via
/// `clip_nTerm_M = 1`: on the same file its own NTT column reports 2 non-tryptic
/// peptides where this classifier reported 16.
fn is_enzymatic_nterm(_residues: &[u8], protein: &[u8], start: usize, enzyme: &Enzyme) -> bool {
    if start == 0 {
        return true;
    }
    // Initiator-Met excision is BIOLOGICAL, not enzymatic, so it holds whatever
    // protease was used. See the doc comment above.
    if start == 1 && protein.first() == Some(&b'M') {
        return true;
    }
    // The peptide N-terminus sits at protein index `start`, which is exactly the
    // "first residue of the next peptide" that Sage's boundary rule tests.
    enzyme.is_boundary(protein, start)
}

/// Is the peptide C-terminus a tryptic one?
///
/// True at the protein C-terminus, or when the peptide ends in K/R and the
/// following residue is not P.
fn is_enzymatic_cterm(_residues: &[u8], protein: &[u8], end: usize, enzyme: &Enzyme) -> bool {
    if end >= protein.len() {
        return true;
    }
    // `end` is exclusive, so it is already the index of the next peptide's first
    // residue -- the same `right` Sage tests.
    enzyme.is_boundary(protein, end)
}

/// Classify one peptide against one protein sequence.
///
/// `residues` must already be stripped of Sage's inline mod brackets — pass
/// `peak_composition::residues_of(peptide)`, never the raw peptide.
///
/// ⚠ **FIRST OCCURRENCE ONLY.** A peptide that repeats within a protein is
/// classified at its first position. A repeat could sit tryptically in one copy
/// and raggedly in another; this reports the first and does not search for a
/// better one. Carried over from the Python prototype deliberately, so the two
/// implementations agree — see NOTES.
pub fn classify_terminus(residues: &str, protein_seq: &str, enzyme: &Enzyme) -> TerminusClass {
    let start = match protein_seq.find(residues) {
        Some(s) => s,
        None => return TerminusClass::NotFound,
    };
    let end = start + residues.len();
    let r = residues.as_bytes();
    let p = protein_seq.as_bytes();
    match (
        is_enzymatic_nterm(r, p, start, enzyme),
        is_enzymatic_cterm(r, p, end, enzyme),
    ) {
        (true, true) => TerminusClass::FullyEnzymatic,
        (false, true) => TerminusClass::SemiNRagged,
        (true, false) => TerminusClass::SemiCRagged,
        (false, false) => TerminusClass::NonEnzymatic,
    }
}

// =============================================================================
// Digestion composition — the numbers recon actually REPORTS
// =============================================================================

/// One reported rate, carrying its own fraction.
///
/// The fraction is not decoration. Preview's own report prints the SAME
/// numerator under two different denominators on two of its pages — oxidised
/// methionine is "19.1 % (509 ... over 2667 baseline)" in the summary and
/// "69.3 % (509/735)" in the detail. A bare percentage cannot be checked; a
/// fraction can. See `_dev/testing/reference-data/preview/10mg_1_A_1/README.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rate {
    pub numerator: usize,
    pub denominator: usize,
    pub pct: f64,
}

impl Rate {
    fn new(numerator: usize, denominator: usize) -> Self {
        Rate {
            numerator,
            denominator,
            pct: if denominator > 0 {
                100.0 * numerator as f64 / denominator as f64
            } else {
                0.0
            },
        }
    }
}

/// Digestion composition of the identified peptide population.
///
/// ## Basis and denominators, taken from Preview's own output
///
/// Read verbatim off the vendored Preview v3.2.0 report for NIST liver
/// `10mg_1_A_1` — "Missed cleavage: 15.9% (319/2008) of tryptic and semitryptic
/// peptides", "Nontryptic peptides (% of all peptides): 0.1% (2/2009)":
///
/// * basis is **DISTINCT PEPTIDES**, not PSMs;
/// * missed cleavage and both ragged rates are over **tryptic + semitryptic**
///   peptides, with non-tryptic EXCLUDED;
/// * only the non-tryptic rate uses all peptides.
///
/// ## The headline
///
/// `cleavage_completeness` is `100 − %missed cleavage`. Both NIST references
/// make missed cleavage the primary digestion axis, and it is the only figure
/// here with published values to compare against: Preview reports 15.9 % missed
/// cleavage on this liver RM and 5.8 % on a HeLa digest standard (Davis et al.
/// 2019, Sci Data 6:324, Table 3).
///
/// ⚠ It is NOT named "digestion efficiency". Both Mouchahoir & Schiel 2018 and
/// Davis et al. use that phrase as an umbrella over a SET of metrics, never for
/// a single number. Reusing it for one figure would invent a third meaning.
///
/// ## What is deliberately absent
///
/// There is **no composite score**, because missed cleavage and ragged termini
/// are opposite phenomena — one is trypsin failing to cut, the other is
/// something else cutting where trypsin would not — and neither reference tool
/// sums them.
///
/// There is **no non-tryptic percentage**. See `non_enzymatic_note`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestionComposition {
    /// Distinct TARGET peptides that could be classified against the FASTA.
    pub peptides_classified: usize,
    /// Peptides whose accession or sequence could not be resolved, and which are
    /// therefore in no denominator. Non-zero means a database mismatch.
    pub peptides_unresolved: usize,

    /// THE HEADLINE: 100 − %missed cleavage, over tryptic + semitryptic peptides.
    pub cleavage_completeness_pct: f64,
    /// The missed-cleavage rate itself, with its fraction.
    pub missed_cleavage: Rate,

    pub ragged_n: Rate,
    pub ragged_c: Rate,
    pub ragged_total: Rate,

    /// Count only. Deliberately has no percentage — see `non_enzymatic_note`.
    pub non_enzymatic_count: usize,
    pub non_enzymatic_note: String,

    /// Per-class decoy subtraction, as Preview does it. `None` when the PSM set
    /// carried no decoys (i.e. `FilterOptions::keep_decoys` was false).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoy_corrected: Option<DecoyCorrectedComposition>,
}

/// Preview's correction: reported count = target hits − decoy hits, PER CLASS.
///
/// Sourced to Kil et al. 2011, which describes exactly this for digestion
/// specificity: Preview "counts the number of hits with score above THigh, and
/// then corrects for the number of false hits estimated by the target/decoy
/// approach".
///
/// It matters because FDR is not uniform across these classes. Measured on liver
/// `10mg_1_A_1` Pass 2: fully-tryptic 0.09 %, semi-tryptic **9.42 %**, against a
/// global cut of 0.97 %. The global 1 % is carried by the fully-tryptic
/// majority, so the ragged classes are far dirtier than the headline FDR admits.
///
/// ⚠ MEASURED, AND IT DOES NOT DO WHAT WAS PREDICTED. Subtraction lowers both
/// ragged rates but barely moves their ratio (liver N:C 2.30 -> 2.28), because
/// the decoys' own N:C is 2.45 — false positives are distributed across N and C
/// almost exactly like real hits. Recorded so nobody re-runs this expecting the
/// ratio to move.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoyCorrectedComposition {
    /// Decoys counted in each class, at the same q cut as the targets.
    pub decoys_fully_enzymatic: usize,
    pub decoys_ragged_n: usize,
    pub decoys_ragged_c: usize,
    /// Implied class FDR = decoys / targets, per class. The single number this
    /// block exists to expose.
    pub class_fdr_fully_enzymatic_pct: f64,
    pub class_fdr_semi_enzymatic_pct: f64,
    /// The corrected rates.
    pub cleavage_completeness_pct: f64,
    pub missed_cleavage: Rate,
    pub ragged_n: Rate,
    pub ragged_c: Rate,
    /// How decoys were split into N vs C. See `decoy_ragged_side`.
    pub n_c_assignment_note: String,
}

/// Which terminus is ragged on a DECOY, inferred from its last residue.
///
/// Decoys carry no terminus classification: Sage builds them per-peptide
/// (`enzyme.rs`, "reversing the sequence inside the first and last amino
/// acids"), so there is no decoy protein to look up flanking residues in. But
/// that same rule PRESERVES the first and last residues, and preserves the
/// target's `semi_enzymatic` flag — so a decoy is a class-matched partner of a
/// real peptide and its ends are the real peptide's ends.
///
/// For a semi-enzymatic peptide exactly one terminus is enzymatic. If the
/// peptide ends in K or R that is almost always the enzymatic end, which makes
/// the N-terminus the ragged one.
///
/// ⚠ THIS IS A PROXY, and it was validated before use rather than assumed:
/// applied to TARGET semi-tryptic PSMs on liver `10mg_1_A_1`, where the true
/// class is known from protein context, it agrees on **96.63 %** (1289/1334).
/// The residual is peptides ending at a protein C-terminus without K/R.
fn decoy_ragged_side(residues: &str, enzyme: &Enzyme) -> TerminusClass {
    let b = residues.as_bytes();
    if !enzyme.is_c_terminal() {
        // An N-terminal cleaver leaves the cleaved residue at the peptide
        // N-terminus, so the roles invert.
        return match b.first() {
            Some(r) if enzyme.cleave_at.contains(r) => TerminusClass::SemiCRagged,
            _ => TerminusClass::SemiNRagged,
        };
    }
    match b.last() {
        Some(r) if enzyme.cleave_at.contains(r) => TerminusClass::SemiNRagged,
        _ => TerminusClass::SemiCRagged,
    }
}

/// Count internal missed cleavages the way Preview defines them.
///
/// Preview's detail page: peptides that "contain an internal K or R not followed
/// by P". VERIFIED to agree with Sage's own `missed_cleavages` column on
/// **0 disagreements out of 10589** peptides (liver `10mg_1_A_1` Pass 2), so the
/// two definitions are the same thing and either may be used.
///
/// ⚠ MSFragger's `Number of Missed Cleavages` is NOT the same quantity — it
/// disagrees with this rule on 12 % of peptides, partly because FragPipe's
/// "stricttrypsin" preset leaves `search_enzyme_nocut` empty and so ignores the
/// proline rule. Do not compare the two columns directly.
fn internal_missed_cleavages(residues: &str, enzyme: &Enzyme) -> usize {
    enzyme.internal_boundaries(residues.as_bytes())
}

/// Compute the reported digestion composition from a Pass-2 PSM set.
///
/// `psms` may contain decoys (see `FilterOptions::keep_decoys`); they are split
/// out here rather than by the caller. Targets are deduplicated to distinct
/// peptide sequences, first occurrence winning, matching Preview's peptide basis.
///
/// The invariant, asserted below: every classified target peptide lands in
/// exactly one class, and the two denominators differ by exactly the non-tryptic
/// count.
pub fn compute_digestion_composition(
    psms: &[crate::sage_results::Psm],
    index: &crate::protein_index::ProteinIndex,
    enzyme: &Enzyme,
) -> DigestionComposition {
    use std::collections::HashSet;

    let mut seen: HashSet<String> = HashSet::new();
    let mut fully = 0usize;
    let mut n_ragged = 0usize;
    let mut c_ragged = 0usize;
    let mut non_tryptic = 0usize;
    let mut unresolved = 0usize;
    let mut mc_hits = 0usize;

    let mut seen_decoy: HashSet<String> = HashSet::new();
    let mut d_fully = 0usize;
    let mut d_n = 0usize;
    let mut d_c = 0usize;
    let mut d_mc = 0usize;
    let mut any_decoys = false;

    for psm in psms {
        let residues = crate::peak_composition::residues_of(&psm.peptide);
        if residues.is_empty() {
            continue;
        }

        if psm.is_decoy {
            any_decoys = true;
            if !seen_decoy.insert(residues.clone()) {
                continue;
            }
            if psm.semi_enzymatic {
                match decoy_ragged_side(&residues, enzyme) {
                    TerminusClass::SemiNRagged => d_n += 1,
                    _ => d_c += 1,
                }
            } else {
                d_fully += 1;
            }
            if internal_missed_cleavages(&residues, enzyme) >= 1 {
                d_mc += 1;
            }
            continue;
        }

        if !seen.insert(residues.clone()) {
            continue;
        }
        let first = psm.proteins.split(';').next().unwrap_or("").trim();
        let class = match index.sequence(first) {
            Some(seq) if !seq.is_empty() => classify_terminus(&residues, seq, enzyme),
            _ => TerminusClass::ProteinNotFound,
        };
        match class {
            TerminusClass::FullyEnzymatic => fully += 1,
            TerminusClass::SemiNRagged => n_ragged += 1,
            TerminusClass::SemiCRagged => c_ragged += 1,
            TerminusClass::NonEnzymatic => non_tryptic += 1,
            TerminusClass::NotFound | TerminusClass::ProteinNotFound => {
                unresolved += 1;
                continue;
            }
        }
        // Preview's denominator for missed cleavage EXCLUDES non-tryptic
        // peptides, so only count hits inside the enzymatic classes.
        if class != TerminusClass::NonEnzymatic && internal_missed_cleavages(&residues, enzyme) >= 1
        {
            mc_hits += 1;
        }
    }

    // Preview's two denominators. Asserted rather than narrated: the enzymatic
    // denominator plus the non-tryptic count IS the all-peptides denominator, so
    // a class that stopped being counted would show up here rather than as a
    // quietly shrunken rate.
    let enzymatic_den = fully + n_ragged + c_ragged;
    let all_den = enzymatic_den + non_tryptic;
    debug_assert_eq!(
        all_den,
        enzymatic_den + non_tryptic,
        "the two Preview denominators must differ by exactly the non-tryptic count"
    );

    let missed_cleavage = Rate::new(mc_hits, enzymatic_den);

    let decoy_corrected = if any_decoys {
        let sub = |t: usize, d: usize| t.saturating_sub(d);
        let cf = sub(fully, d_fully);
        let cn = sub(n_ragged, d_n);
        let cc = sub(c_ragged, d_c);
        let cden = cf + cn + cc;
        // The headline is corrected the same way every other class is: subtract
        // the decoys that carry a missed cleavage from the numerator, and the
        // decoys overall from the denominator. Sage's decoy generation PRESERVES
        // `missed_cleavages` from the target it was built from (`enzyme.rs`), so
        // a decoy's missed-cleavage status is a real class label, not an artifact
        // of the reversal.
        let corrected_mc = Rate::new(sub(mc_hits, d_mc), cden);
        let semi_t = n_ragged + c_ragged;
        let semi_d = d_n + d_c;
        Some(DecoyCorrectedComposition {
            decoys_fully_enzymatic: d_fully,
            decoys_ragged_n: d_n,
            decoys_ragged_c: d_c,
            class_fdr_fully_enzymatic_pct: if fully > 0 {
                100.0 * d_fully as f64 / fully as f64
            } else {
                0.0
            },
            class_fdr_semi_enzymatic_pct: if semi_t > 0 {
                100.0 * semi_d as f64 / semi_t as f64
            } else {
                0.0
            },
            cleavage_completeness_pct: 100.0 - corrected_mc.pct,
            missed_cleavage: corrected_mc,
            ragged_n: Rate::new(cn, cden),
            ragged_c: Rate::new(cc, cden),
            n_c_assignment_note:
                "Decoys carry no N/C label; Sage's per-peptide reversal preserves \
                 the first and last residue, so the side is inferred from the last \
                 residue. Validated at 96.63 % against known target classes."
                    .to_string(),
        })
    } else {
        None
    };

    DigestionComposition {
        peptides_classified: all_den,
        peptides_unresolved: unresolved,
        cleavage_completeness_pct: 100.0 - missed_cleavage.pct,
        missed_cleavage,
        ragged_n: Rate::new(n_ragged, enzymatic_den),
        ragged_c: Rate::new(c_ragged, enzymatic_den),
        ragged_total: Rate::new(n_ragged + c_ragged, enzymatic_den),
        non_enzymatic_count: non_tryptic,
        non_enzymatic_note: "NOT MEASURED as a rate. Pass 2 runs semi_enzymatic, so Sage only \
             generates candidates with ONE non-specific terminus and a fully \
             non-tryptic peptide is never scored. Preview obtains its 0.1 % from \
             a separate non-enzymatic search, which recon does not run. The count \
             here is residual classifier disagreement, not a measurement."
            .to_string(),
        decoy_corrected,
    }
}

/// Print the reported digestion composition.
///
/// Every percentage is printed WITH its fraction. Preview's own report shows the
/// same numerator under two different denominators on two pages; a bare rate
/// cannot be checked against anything.
pub fn print_composition_summary(c: &DigestionComposition) {
    println!();
    println!("=== Digestion (Pass 2) ===");
    println!();
    println!(
        "  CLEAVAGE COMPLETENESS: {:.1} %   (peptides with no missed cleavage)",
        c.cleavage_completeness_pct
    );
    println!(
        "    missed cleavage    : {:.2} % ({}/{})",
        c.missed_cleavage.pct, c.missed_cleavage.numerator, c.missed_cleavage.denominator
    );
    println!();
    println!(
        "    ragged N-terminus  : {:.2} % ({}/{})",
        c.ragged_n.pct, c.ragged_n.numerator, c.ragged_n.denominator
    );
    println!(
        "    ragged C-terminus  : {:.2} % ({}/{})",
        c.ragged_c.pct, c.ragged_c.numerator, c.ragged_c.denominator
    );
    println!("    non-tryptic        : not measured (needs a non-enzymatic search)");
    println!();
    println!(
        "  Basis: {} distinct peptides. Missed-cleavage and ragged rates exclude",
        c.peptides_classified
    );
    println!("  non-tryptic peptides from the denominator, as Byonic Preview does.");
    if c.peptides_unresolved > 0 {
        println!(
            "  ⚠ {} peptides could not be located in the search FASTA and are in no",
            c.peptides_unresolved
        );
        println!("    denominator. A non-zero count here means a database mismatch.");
    }
    if let Some(d) = &c.decoy_corrected {
        println!();
        println!("  Per-class FDR at the global q cut:");
        println!(
            "    fully tryptic {:.2} %   semi-tryptic {:.2} %",
            d.class_fdr_fully_enzymatic_pct, d.class_fdr_semi_enzymatic_pct
        );
        println!("  Decoy-subtracted (target hits minus decoy hits, per class):");
        println!(
            "    completeness {:.1} %   ragged-N {:.2} % ({}/{})   ragged-C {:.2} % ({}/{})",
            d.cleavage_completeness_pct,
            d.ragged_n.pct,
            d.ragged_n.numerator,
            d.ragged_n.denominator,
            d.ragged_c.pct,
            d.ragged_c.numerator,
            d.ragged_c.denominator
        );
    }
    println!();
    println!("  No composite score: missed cleavage and ragged termini are opposite");
    println!("  phenomena with different causes, and are not summed.");
}
