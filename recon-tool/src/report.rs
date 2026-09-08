//! Unified report generation for proteomics reconnaissance.
//!
//! This module provides:
//! - `ReconReport` struct that consolidates all analysis results
//! - JSON serialization for machine-readable output
//! - Text summary for console output
//! - HTML report for shareable/printable output
//!
//! Phase 7 implementation per PLAN.md.

use crate::digestion::DigestionResult;
use crate::mod_discovery::{DiscoverySettings, ModDiscoveryResult, Peak};
use crate::mzml::MzmlStats;
use crate::oxonium::OxoniumScreeningSummary;
use crate::polymer::PolymerSearchResults;
use crate::qc::QcResult;
use crate::signal_fate::SignalFateResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Schema version for the report format
/// Bumped to 1.1.0 on 2026-08-25: peak counts changed because peak assignment was
/// fixed, and `discovery_settings` was added to `discover` output. Additive for
/// readers — no field was removed or renamed — but a 1.0.0 artifact and a 1.1.0
/// artifact are NOT comparable on peak counts. Anything still at 1.0.0 predates the
/// fix.
///
/// Bumped to 1.2.0 on 2026-08-25: `mod_discovery.discovery_settings` added to the
/// ANALYZE report too. 1.1.0 claimed this and was wrong — the field existed only on
/// `discover` output, so a shipped report did not record the peak-assignment mode
/// that made its peaks. Additive; peak values are unchanged between 1.1.0 and 1.2.0.
///
/// Bumped to 1.3.0 on 2026-08-26: the `recommendations` block added — step 2's tier
/// assignment. Additive; peak values unchanged.
///
/// Bumped to 1.4.0 on 2026-08-27: `recommendations.protein_context` added — step
/// 2.5. Additive, and peak values are again unchanged, but ONE recommendation moves
/// between 1.3.0 and 1.4.0: with a FASTA supplied, protein-terminal candidates become
/// testable, and bcell's -89.0289 Met-loss+Acetylation goes from `below_floor` to the
/// statistics path. A 1.3.0 report's `recommendations` is therefore not comparable to
/// a 1.4.0 one for that class of modification.
///
/// Bumped to 1.6.0 on 2026-08-28: `ms1_calibration.user_recommendation_tolerance_ppm`
/// ADDED, and `user_recommendation_low/high_ppm` CHANGE MEANING. They were an
/// asymmetric `bias-2*MAD .. bias+p95(|dev|)` window; they are now symmetric
/// -/+ the quantized ladder rung. On all three test files that is -/+10 ppm,
/// where 1.5.0 reported e.g. serum +1.46 to +4.09. A 1.5.0 report's
/// recommendation is NOT comparable with a 1.6.0 one. Bias and MAD are
/// unaffected and still reported separately.
///
/// Bumped to 1.5.0 on 2026-08-28: no field was added, removed or renamed, but
/// `ms1_calibration.bias_ppm` and `ms1_calibration.spread_mad_ppm` — and the
/// user recommendation and Pass 2 window derived from them — CHANGE VALUE. They
/// were medians of Sage's ABSOLUTE `precursor_ppm`; they are now computed from a
/// signed error reconstructed from the mass columns. A 1.4.0 report's
/// calibration block is therefore NOT comparable with a 1.5.0 one: on bcell the
/// bias goes from +0.70 to -0.24 ppm, a sign flip. Everything outside the
/// calibration block is unchanged. This follows the 1.1.0 precedent — a value
/// correction on an existing field is a MINOR bump carrying an explicit
/// non-comparability note.
///
/// The four `mass_accuracy.*_ppm` fields are NOT affected BY THAT CHANGE.
///
/// ⚠ They were affected by a LATER one. See the pending-version entry below.
/// Bumped to 1.7.0 on 2026-08-31, carrying THREE changes that land together
/// because they are regenerated in one pass:
///
/// * `analyzers` ADDED — the detected MS1/MS2 mass analyzers and the pass-1 MS2
///   `fragment_tol` they imply. Additive. It records a decision `recon run`
///   already made before the pass-1 search; it does not change one.
/// * **`q <= threshold` is now the rule EVERYWHERE.** `parse_sage_results` used
///   `peptide_q >= threshold` to EXCLUDE — strictly LESS than 1 % — while
///   `protein_index` used `q >`, which is `<= 1 %`. The two halves of the
///   pipeline disagreed on exactly one boundary. Ben settled it: `q <= 0.01`,
///   which is what "1 % FDR" conventionally means.
///   ⚠ **MEASURED: this moves NOTHING on our data.** Rows sitting exactly on
///   `peptide_q == 0.01`: **0 of 100470 on liver, 0 of 77086 on bcell.** An
///   earlier draft of this note claimed counts "can move"; that was asserted,
///   not measured, and it is wrong on these files. It is a consistency fix.
/// * **The Pass 2 subset is now PARSIMONIOUS.** Only in `<output>_pass2.json`,
///   but recorded here because the two are regenerated together. Measured on two
///   files: no digestion rate moves more than 0.14 pp.
///
/// ⚠ **3.0.0, 2026-09-02 — BREAKING. A MEANING CHANGE WITH NO FIELD CHANGE.**
///
/// Ben's call. `result-schema.md` says a field "with changed semantics" is a
/// MAJOR bump, and this is one. No key was added, renamed or removed.
///
/// The two `mass_accuracy.precursor_*_ppm` fields became SIGNED when the Sage
/// pin moved to v0.15.0-beta.2 on 2026-09-01. No recon code changed, so nothing
/// announced it. The fields simply started carrying a different quantity.
///
/// ⚠ Output made between 2026-09-01 and this bump is labelled 2.1.0 and already
/// carries SIGNED values. That window cannot be closed: the change came from a
/// dependency, not from recon. The committed `full-run/` artifacts are 2.0.0 and
/// predate it.
///
/// A consumer that treats `precursor_median_ppm` as a magnitude will read a
/// sign flip as a zero crossing, or will `abs()` it and destroy the bias.
/// `fragment_*_ppm` are unaffected and stay absolute.
///
/// The v0.15 upgrade checklist called for a manual audit of column semantics. It
/// was not done, and this is the entry it would have produced.
///
/// Bumped to 3.1.0 on 2026-09-03: `input.fasta_file` and `input.enzyme` ADDED.
/// The report did not record which database was searched, nor which protease the
/// digestion numbers were counted with. Both had to be recovered from Sage's own
/// `results.json` next to the search output. Additive: both fields are optional
/// and are absent when the run did not have them, so a 3.0.0 consumer keeps
/// working. NO EXISTING VALUE MOVES — verified by diffing a full `run` before and
/// after the change with the two new keys and the timestamps excluded.
///
/// `input.enzyme` is IDENTITY ONLY — `name`, `cleave_at`, `restrict`,
/// `c_terminal`. The tuning fields (`missed_cleavages`, `min_len`, `max_len`,
/// `semi_enzymatic`) are deliberately NOT here: they belong to the search config,
/// they differ between Pass 1 and Pass 2, and recording them beside the identity
/// would invite a reader to take them as the enzyme's definition. See
/// `enzyme::apply_to_params_text`, which writes the same three fields and no more.
/// Bumped to 3.2.0 on 2026-09-03: `recommendations.not_recommended[]` and
/// `recommendations.notable_unannotated[]` gained `count_pct`, `sites` and
/// `position`. Additive — every one is `#[serde(default)]` and the two strings
/// are omitted when empty — so a 3.1.0 consumer keeps working. NO EXISTING VALUE
/// MOVES; verified by diffing a full `run` before and after with only the new
/// keys and the timestamps excluded.
///
/// The "did not make the cut" table rendered an em dash in both the Residue and
/// the % column while its Count was populated. The data existed in both cases and
/// was being dropped:
/// * `sites` / `position` — for a curated row these are the acceptors the residue
///   test ACTUALLY RAN AGAINST, carried through from the tested candidate by
///   `tier_assignment::to_report`. A `failed_residue_test` row is the one row type
///   whose entire point is the residue. For a Unimod-named row they come from the
///   Unimod entry the annotation already chose, looked up by its `unimod_id` and
///   guarded on `mono_mass`, with Unimod's position vocabulary mapped onto the
///   curated one.
///   ⚠ **CORRECTED 2026-09-03, same day.** This entry first said "Unimod
///   contributes no `position`" and described the sites as coming off
///   `PeakAnnotation.sites`. Both were wrong, and the second was a BUG: that field
///   is `UnimodEntry::sites`, which drops every `hidden="1"` specificity with no
///   fallback, so it filled 1 row of 10 on serum — `Pyro-carbamidomethyl` was the
///   only one of the eleven with a visible specificity. `unimod_acceptor` reads
///   the entry's own `specificities` with the all-hidden fallback
///   `UnimodEntry::classification` already documents, and does supply a position.
///   ⚠ 3.2.0 output made BEFORE that correction carries `sites` on the non-hidden
///   subset only, and no `position` on a Unimod-named row. The field set is
///   identical either way, so no version distinguishes them; regenerate to get the
///   full set.
/// * `count_pct` — the SAME denominator `RecommendedMod.count_pct` uses
///   (`mod_discovery.summary.total_psms`). A second denominator would have made
///   the three tables incomparable while they sit next to each other.
///
/// The MS1/MS2 tolerance recommendation was merged into one presented stat in the
/// same pass. That is HTML only and adds no field: the MS2 rung is derived at
/// render time from `ms1_calibration.ms2_tolerance_high_ppm`, which is already in
/// the JSON.
pub const SCHEMA_VERSION: &str = "3.2.0";

/// Schema version of the SEPARATE `<output>_pass2.json` artifact.
///
/// Versioned apart from `SCHEMA_VERSION` on purpose: Pass 2 is produced by
/// `recon run`, the analyze report by `recon analyze`, and the frozen
/// `full-run/` set contains only the latter. One file moving must not force a
/// version bump on the other.
/// **1.1.0** (2026-08-31): added `composition` — the digestion numbers recon
/// actually reports, on Preview's peptide basis and denominators, with per-class
/// decoy subtraction. Additive, so 1.0.0 consumers keep working.
pub const PASS2_SCHEMA_VERSION: &str = "1.1.0";

/// Input file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputInfo {
    /// Path to mzML file
    pub mzml_file: String,
    /// Path to Sage TSV file
    pub sage_tsv: String,
    /// Path to Unimod XML file
    pub unimod_file: String,
    /// Number of MS1 spectra
    pub ms1_spectra: usize,
    /// Number of MS2 spectra
    pub ms2_spectra: usize,
    /// The protein FASTA, as the path was given on the command line.
    ///
    /// Stored verbatim like `mzml_file`, NOT canonicalised: the report records
    /// what the user passed. `None` for `recon analyze` without `--fasta`, which
    /// is a supported call — the protein context is then simply not available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fasta_file: Option<String>,
    /// The protease the search and the digestion report used.
    ///
    /// `None` for `recon analyze`, which has no `--enzyme`: it reads a TSV some
    /// other command produced, so it cannot know the protease and must not guess
    /// one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enzyme: Option<EnzymeInfo>,
}

/// The protease IDENTITY, as recorded in the report.
///
/// ⚠ **IDENTITY ONLY, AND THAT IS A LOCK.** Exactly the three rule fields plus
/// the name. The tuning fields — `missed_cleavages`, `min_len`, `max_len`,
/// `semi_enzymatic` — belong to the search config, not to the enzyme, and Pass 1
/// and Pass 2 set them differently on purpose. Putting them here would read as
/// part of the enzyme's definition. This mirrors `enzyme::apply_to_params_text`,
/// which writes the same three fields into Sage's config and no more.
///
/// This is a separate struct from `enzyme::Enzyme` for ONE reason: `Enzyme` holds
/// the residues as `Vec<u8>`, which serde writes as arrays of byte NUMBERS
/// (`[75, 82]`). The report writes them as the residue letters a reader expects
/// (`"KR"`), which is also how Sage's own config spells them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnzymeInfo {
    /// Name as the user gave it: a preset name such as `trypsin`, or an explicit
    /// rule such as `KR/P`.
    pub name: String,
    /// Residues cleaved at. Sage's `database.enzyme.cleave_at`.
    pub cleave_at: String,
    /// Residues that suppress a cleavage at the boundary. Sage's `restrict`.
    /// Empty string means no restriction.
    pub restrict: String,
    /// True when cleavage is at the C-terminus of the matched residue (trypsin),
    /// false for an N-terminal cleaver (Asp-N). Sage's `c_terminal`.
    pub c_terminal: bool,
}

impl From<&crate::enzyme::Enzyme> for EnzymeInfo {
    fn from(e: &crate::enzyme::Enzyme) -> Self {
        EnzymeInfo {
            name: e.name.clone(),
            cleave_at: String::from_utf8_lossy(&e.cleave_at).into_owned(),
            restrict: String::from_utf8_lossy(&e.restrict).into_owned(),
            c_terminal: e.c_terminal,
        }
    }
}

impl EnzymeInfo {
    /// One-line identity for the HTML meta grid, e.g.
    /// `trypsin — KR, not before P, C-term`.
    ///
    /// The restriction clause is worded by cleavage direction, because Sage tests
    /// the residue AT the boundary in both cases: for a C-terminal enzyme that
    /// residue FOLLOWS the cleaved one ("not before P"), for an N-terminal enzyme
    /// it IS the cleaved one ("not at X"). See `enzyme::Enzyme::is_boundary`.
    pub fn one_line(&self) -> String {
        let term = if self.c_terminal { "C-term" } else { "N-term" };
        if self.restrict.is_empty() {
            format!("{} — {}, {}", self.name, self.cleave_at, term)
        } else if self.c_terminal {
            format!(
                "{} — {}, not before {}, {}",
                self.name, self.cleave_at, self.restrict, term
            )
        } else {
            format!(
                "{} — {}, not at {}, {}",
                self.name, self.cleave_at, self.restrict, term
            )
        }
    }
}

/// Alkylation check results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlkylationCheck {
    /// Fixed modification assumed (e.g., "Carbamidomethyl +57.02 Da on C")
    pub fixed_mod_assumed: String,
    /// Number of Cys-containing PSMs
    pub cys_psm_count: usize,
    /// Percentage of total PSMs that contain Cys
    pub cys_psm_pct: f64,
    /// Number of Cys PSMs showing unalkylated signal (delta ~-57 Da)
    pub unalkylated_count: usize,
    /// Percentage of Cys PSMs that appear unalkylated
    pub unalkylated_pct: f64,
    /// Status message (e.g., "Alkylation appears complete")
    pub status: String,
}

/// One recommended search modification.
///
/// `decided_by` is the load-bearing field. A recommendation reached by statistics
/// carries an odds ratio and a BH-adjusted q; one reached by abundance does not,
/// because no statistic could reach it. Do not read a missing odds ratio as a weak
/// result — read it as "not testable", and see `sites`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedMod {
    pub delta_mass: f64,
    pub count: usize,
    pub count_pct: f64,
    /// Curated name. Source is MetaMorpheus's mod list, not Unimod.
    pub label: String,
    /// MetaMorpheus `MT` category, e.g. "Common Fixed", "Metal", "Common Artifact".
    pub category: String,
    /// "fixed" or "variable". **An INHERITED LABEL read from `category`, not a
    /// recon measurement.** Passed through from the curator as guidance for the
    /// user's FINAL search; recon makes no judgement of its own. The occupancy
    /// rule was tested and does not work: an open search assigns one delta per
    /// PSM, so occupancy cannot approach total residue occurrence. See NOTES
    /// "Step 2 decision rule".
    pub role: String,
    /// "statistics" or "abundance".
    pub decided_by: String,
    /// Acceptor residues tested. Empty on the abundance path.
    ///
    /// **Read this WITH `position`.** A terminal candidate is tested positionally
    /// -- "first residue is E", not "contains E" -- so `sites` alone
    /// under-specifies the setting to carry into a search.
    pub sites: String,
    /// The candidate's positional restriction, from the curated list's `PP`:
    /// "Anywhere.", "Peptide N-terminal.", "Protein N-terminal.", or
    /// "Protein N-terminal, Met loss.". Empty when no curated candidate applied.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub position: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub odds_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q_value: Option<f64>,
    /// Only on the abundance path: the count as a percentage of the floor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pct_of_floor: Option<f64>,
}

/// Why a detected peak is not recommended.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotRecommended {
    pub delta_mass: f64,
    pub count: usize,
    /// `count` as a percentage of the same denominator `RecommendedMod.count_pct`
    /// uses — `mod_discovery.summary.total_psms`. Added at schema 3.2.0.
    ///
    /// It was missing, so the report showed a count with no scale on exactly the
    /// rows a reader most needs to size: the ones that were tested and failed.
    /// The two tables sit next to each other, so a second denominator here would
    /// have made the three tables incomparable.
    #[serde(default)]
    pub count_pct: f64,
    /// Why this delta mass was not recommended.
    ///
    /// ⚠ **THESE NAME THE OUTCOME, NOT THE CODE BRANCH.** The previous strings
    /// named the branch a peak fell out of, which read as something else
    /// entirely: `not_curated` actually meant "un-curated AND below the floor",
    /// and `no_residue_support` meant "tested at the residue level and FAILED",
    /// not "has no residue annotation". Both misled a reader of the report.
    ///
    /// One of: `"satellite"`, `"failed_residue_test"`, `"below_floor"`,
    /// `"below_floor_uncurated"`, `"uncurated"`.
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Where `label` came from: "curated" or "unimod".
    ///
    /// A curated name is one recon can DECIDE on -- it carries acceptor residues
    /// and a position. A Unimod name is reached only when the curated list has no
    /// entry at this mass, and is offered FOR EXPLORATION ONLY: it names a
    /// possibility, nothing was tested, and it is never recommended. Keeping the
    /// distinction as a field rather than baking it into the string keeps the
    /// JSON machine-readable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_source: Option<String>,
    /// Acceptor residues, read WITH `position` exactly as `RecommendedMod.sites`
    /// is. Added at schema 3.2.0. Empty when nothing is known, which is not the
    /// same statement as "unspecific" — see `acceptor_cell`.
    ///
    /// Where it comes from depends on `name_source`:
    /// * `"curated"` — the residues the test ACTUALLY RAN AGAINST, carried
    ///   through from the tested candidate by `tier_assignment::to_report`. A
    ///   `failed_residue_test` row without them is unreadable: the row exists to
    ///   say which residues failed.
    /// * `"unimod"` — the acceptor residues of the Unimod entry the annotation
    ///   already chose, via `unimod_acceptor`. If the report reaches into the XML
    ///   for a name it takes the residues too, rather than naming a modification
    ///   and then refusing to say where it goes.
    ///   ⚠ It is read from the entry's `specificities`, NOT from
    ///   `PeakAnnotation.sites` / `UnimodEntry::sites`. Those drop `hidden="1"`
    ///   specificities, which on serum is 10 of the 11 named entries.
    ///
    /// ⚠ **A SATELLITE ROW IS DELIBERATELY LEFT EMPTY**, even when it carries a
    /// Unimod name. It stops at step 1 of the routing chart with `decided_by:
    /// null` and never reached a residue test; printing acceptors would imply a
    /// test that never ran.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sites: String,
    /// The acceptor's positional restriction, e.g. "Anywhere." or
    /// "Protein N-terminal, Met loss.". Same field and same words as
    /// `RecommendedMod.position` — for a curated row it IS the tested candidate's.
    ///
    /// A Unimod-named row carries Unimod's own position, mapped onto that same
    /// vocabulary ("Any N-term" becomes "Peptide N-terminal.", and so on). It is
    /// set only when every residue-bearing specificity agrees on one; a Unimod
    /// entry may carry several and this field has room for one.
    ///
    /// An empty `position` AND an empty `sites` mean "not known", which is NOT the
    /// same statement as "unspecific" — see `acceptor_cell`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub position: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub odds_ratio: Option<f64>,
    /// The q-value from the same residue test as `odds_ratio`.
    ///
    /// ⚠ **THIS WAS COMPUTED AND THEN THROWN AWAY.** `Decision::NoResidueSupport`
    /// carries both `odds_ratio` and `q`, but the serialiser destructured
    /// `{ odds_ratio, .. }` and dropped q on the floor. The report therefore
    /// showed an odds ratio that CLEARED the bar with no way to see which gate
    /// actually failed — serum's `Water Loss (Glu->pyro-Glu)` shows OR 2.12
    /// against a minimum of 2.0. Without q the decision cannot be audited from
    /// the artifact at all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q_value: Option<f64>,
}

/// Whether the protein-terminal candidates could be tested at all, and against
/// what.
///
/// Present when `analyze --fasta` supplied the search database. Absent means the
/// protein-terminal class was NOT TESTABLE and went to the abundance path — a
/// different statement from "not supported", and the reason it is recorded as a
/// block rather than left to be inferred from a missing recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinContext {
    /// The FASTA as it was given on the command line.
    pub fasta: String,
    /// Sequences read from it.
    pub proteins_indexed: usize,
    /// Target PSMs whose `proteins` column resolved at least one accession.
    pub psms_resolved: usize,
    /// Target PSMs considered. Decoys are excluded: `rev_` accessions cannot
    /// resolve by construction, so counting them would drag the fraction down by
    /// the decoy rate and make the guard threshold meaningless.
    pub psms_total: usize,
    /// `psms_resolved / psms_total`, as a percentage.
    pub resolved_pct: f64,
    /// Target PSMs whose peptide starts at protein position 0.
    ///
    /// This is the number that makes the positional test discriminating. It runs
    /// at 0.4-1.6% of confident PSMs on the three test files. A value near 100%
    /// would mean the lookup is matching everything and the test is worthless.
    pub protein_nterm_psms: usize,
}

/// The step-2 search-parameter recommendation.
///
/// Recon's claim here is NOT a prevalence estimate and NOT a full ordering. It is
/// "use these in your next search". Peaks are routed to one instrument each:
/// residue-specific acceptors are decided by Fisher exact with an odds ratio and
/// BH correction; unspecific acceptors are decided by the abundance floor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModRecommendations {
    pub fixed: Vec<RecommendedMod>,
    pub variable: Vec<RecommendedMod>,
    pub not_recommended: Vec<NotRecommended>,
    /// Peaks ABOVE the abundance floor that the curated list cannot name.
    ///
    /// The design's "present, your call". A large delta with no curated match is
    /// the novel-chemistry signal recon exists to surface, so it gets its own list
    /// rather than being flattened into `not_recommended`. Never recommended, and
    /// no statistic applies: with no candidate there are no acceptor residues to
    /// test, so the floor is the only instrument.
    pub notable_unannotated: Vec<NotRecommended>,
    /// Step-2 carpet invariant: `floor_psms` minus the tallest FLOOR-GOVERNED peak
    /// inside the ±1/±2 Da quantization carpet. **Positive means the invariant
    /// holds** — the floor sits above the carpet.
    ///
    /// Only floor-governed peaks count: unspecific acceptors and uncurated masses.
    /// A residue-specific modification is decided by presence, never by amount, so
    /// it never faces the floor. That is why Deamidated — the tallest peak in the
    /// region on all three test files — is not a violation here.
    pub carpet_margin_psms: f64,
    /// The tallest floor-governed peak inside the carpet region, in PSMs. 0 when
    /// the region holds none.
    pub carpet_tallest_psms: usize,
    /// Abundance floor in PSMs. Applies ONLY to the abundance path.
    pub floor_psms: f64,
    /// The floor as a percentage of the top non-zero peak.
    pub floor_pct_of_top: f64,
    pub odds_ratio_min: f64,
    pub q_max: f64,
    /// Where the candidate names and acceptor sites came from.
    pub annotation_source: String,
    /// The protein-sequence context, when a FASTA was supplied. `None` means
    /// protein-terminal candidates were not testable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_context: Option<ProteinContext>,
    /// Stated limitations a reader must carry with these numbers.
    pub caveats: Vec<String>,
}

/// Modification discovery summary for report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDiscoverySummaryReport {
    /// Total PSMs analyzed
    pub total_psms: usize,
    /// Percentage of PSMs that are unmodified (delta ~0)
    pub unmodified_pct: f64,
    /// All detected peaks (full list)
    pub peaks: Vec<Peak>,
    /// The mod-discovery config that produced `peaks`.
    ///
    /// Added 2026-08-25. `discover` output carried this from schema 1.1.0 but the
    /// `analyze` report did not, so a shipped report could not say which
    /// `peak_assignment_mode`, bin width, peak floor or calibration mode made its
    /// peaks. A peak table whose grouping rule is unrecorded is not reproducible
    /// from the file alone.
    pub discovery_settings: DiscoverySettings,
}

/// Signal fate summary for report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalFateSummaryReport {
    /// Identification rate by MS2 spectrum count
    pub id_rate_by_count_pct: f64,
    /// Identification rate by MS2 TIC
    pub id_rate_by_tic_pct: f64,
    /// Number of identified spectra
    pub identified_spectra: usize,
    /// Total MS2 spectra
    pub total_ms2_spectra: usize,
    /// Chimeric scan percentage
    pub chimera_rate_pct: f64,
}

/// Polymer contamination summary for report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymerSummaryReport {
    /// Total polymer %TIC
    pub total_pct_tic: f64,
    /// Contamination level description
    pub contamination_level: String,
    /// Top polymers by %TIC
    pub top_polymers: Vec<PolymerEntry>,
}

/// Single polymer entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymerEntry {
    /// Polymer name
    pub name: String,
    /// Percentage of TIC
    pub pct_tic: f64,
}

/// Oxonium screening summary for report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OxoniumSummaryReport {
    /// Number of glycopeptide candidate spectra
    pub glycopeptide_candidates: usize,
    /// Percentage of MS2 spectra that are glycopeptide candidates
    pub glycopeptide_pct: f64,
}

/// Digestion summary for report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestionSummaryReport {
    /// Percentage with 0 missed cleavages
    pub missed_cleavage_0_pct: f64,
    /// Percentage with 1 missed cleavage
    pub missed_cleavage_1_pct: f64,
    /// Percentage with 2+ missed cleavages
    pub missed_cleavage_2plus_pct: f64,
    /// Percentage of semi-tryptic (ragged ends)
    pub ragged_ends_pct: f64,
}

/// Mass accuracy summary for report
///
/// ⚠ **THE TWO PRECURSOR FIELDS ARE SIGNED. THE TWO FRAGMENT FIELDS ARE NOT.**
/// This block used to say all four were medians of |error| and that none could
/// be negative. That described v0.14.x and became FALSE at the v0.15 pin.
/// Corrected in place 2026-09-02; the committed `full-run/liver.json` already
/// carried `precursor_median_ppm: -0.2771486`, a negative value in a field the
/// doc said could not be negative.
///
/// Measured on the committed v0.15 serum output: 17591 of 68817 `precursor_ppm`
/// values negative, 0 of 68817 `fragment_ppm` values negative.
///
/// None of the four measures instrument bias. **The signed MS1 number lives in
/// `ms1_calibration.bias_ppm`**, reconstructed by
/// `calibration::signed_precursor_ppm` from the OPEN search's own near-zero
/// clean subset. No closed search is needed, or used, anywhere in recon.
///
/// The KEYS keep their names deliberately. Renaming them to `*_abs_ppm` would be
/// a MAJOR schema bump under `_dev/reference-notes/result-schema.md` "Schema
/// Versioning Policy", which is not warranted by a labelling fix. The rename is
/// recorded in NOTES as a candidate for a future 2.0.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassAccuracySummaryReport {
    /// Precursor (MS1) median of Sage's SIGNED `precursor_ppm`. Can be negative.
    /// In an open search this is dominated by the delta mass, the PTM shift, not
    /// by instrument calibration. It is not a mass accuracy and is not
    /// comparable with `bias_ppm`.
    pub precursor_median_ppm: f64,
    /// Precursor (MS1) 95th percentile of the SIGNED distribution. Under v0.14
    /// this was a coverage bound, "95 % of PSMs within X ppm". It is not that
    /// any more, because the values carry a sign.
    pub precursor_p95_ppm: f64,
    /// Fragment (MS2) median ABSOLUTE ppm. `fragment_ppm` stayed absolute at
    /// v0.15, so no signed MS2 bias can be read from it.
    pub fragment_median_ppm: f64,
    /// Fragment (MS2) 95th percentile ABSOLUTE ppm.
    pub fragment_p95_ppm: f64,
}

/// Self-calibrated MS1/MS2 tolerance recommendation, measured from the open
/// search's own clean subset (near-zero-delta, rank-1, target, q<0.01 PSMs —
/// see NOTES "MS1 error from the wide search's clean subset"). Two MS1 numbers
/// are reported deliberately (locked, do NOT conflate): `user_recommendation`
/// is generous/asymmetric for the user's own next search; `pass2_window` is
/// tight/bias-centered and hard-capped at +/-100 ppm, sized for an internal
/// Pass 2 semi-tryptic search only. Present whenever the clean subset is
/// non-empty; `None` when there were no qualifying PSMs (e.g. every PSM had a
/// large delta mass, or too few target rank-1 hits cleared q<threshold).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ms1CalibrationReport {
    /// PSMs in the clean subset the stats below were measured from.
    pub clean_subset_n_psms: usize,
    /// Whether the hyperscore guard (top 50-70% by hyperscore) was applied,
    /// or skipped because the subset was too small (< 200 or < 10% of total
    /// MS2 spectra) and fell back to the unguarded q-value-only subset.
    pub hyperscore_guard_applied: bool,
    /// Median signed MS1 ppm error of the clean subset (instrument bias).
    pub bias_ppm: f64,
    /// Median absolute deviation of the clean subset's ppm errors (spread).
    pub spread_mad_ppm: f64,
    /// The user-facing MS1 tolerance recommendation, QUANTIZED to the
    /// {10, 20, 50, 100} ppm ladder: the smallest rung >= |bias| + 5*MAD.
    /// This is the number to type into your own search. MS1 only — a ppm ladder
    /// is meaningless for an ion-trap MS2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_recommendation_tolerance_ppm: Option<f64>,
    /// The UNQUANTIZED requirement `|bias| + 5*MAD`, so a reader can see how
    /// much headroom the recommended rung actually has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_recommendation_requirement_ppm: Option<f64>,
    /// True when the requirement exceeds the TOP rung (100 ppm). The
    /// recommendation is then the top rung and is KNOWN TO BE TOO NARROW.
    /// A badly mis-calibrated instrument must not be handed a quiet "100 ppm"
    /// that looks like every other answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_recommendation_exceeds_ladder: Option<bool>,
    /// Symmetric -/+ `user_recommendation_tolerance_ppm`.
    ///
    /// ⚠ CHANGED 2026-08-28. These were an ASYMMETRIC window
    /// (`bias-2*MAD` .. `bias+p95`), which was 3-5x too tight against three
    /// independent lines of evidence and, before the |error| fix, was computed
    /// from a folded distribution as well. A 1.5.0-or-earlier report's values
    /// here are NOT comparable with a 1.6.0 one.
    pub user_recommendation_low_ppm: f64,
    pub user_recommendation_high_ppm: f64,
    /// Bias-centred Pass 2 `precursor_tol` window, in DELTA space.
    ///
    /// ⚠ CHANGED 2026-08-28. The half-width was `3*MAD` with a `±100 ppm` cap;
    /// both are gone. `3*MAD` was MEASURED to cover only 80.82 / 87.46 / 80.77 %
    /// of confident PSMs, and the cap became unreachable once the width came
    /// from the ladder (whose top rung is also 100). The half-width is now the
    /// ladder rung, and measured coverage is 99.32 / 99.92 / 99.94 %.
    ///
    /// NOT the same number as the user recommendation above: that is symmetric
    /// about ZERO, this is centred on the measured BIAS.
    ///
    /// ⚠ These are DELTA-space bounds. Sage's `precursor_tol` config is
    /// sign-INVERTED relative to them — see `pass2::precursor_tol_json`, which
    /// is the only place that conversion is allowed to happen.
    pub pass2_window_low_ppm: f64,
    pub pass2_window_high_ppm: f64,
    /// MS2 fragment tolerance, derived directly from Sage's own fragment_ppm
    /// column on the clean subset (no recomputation from raw spectra).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ms2_tolerance_low_ppm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ms2_tolerance_high_ppm: Option<f64>,
    /// ⚠ NOT A BIAS, despite the field name. This is the MEDIAN over the clean
    /// subset of Sage's `fragment_ppm`, and in the pinned Sage that column is
    /// an **intensity-weighted MEAN of |error|** per PSM
    /// (`crates/sage/src/scoring.rs:607` at the pinned commit). It cannot be
    /// negative and it reads high. Measured on serum 2026-08-28 against a real
    /// `--annotate-matches` run: this field gives +1.2602 ppm where the true
    /// signed value is +0.9496, against MSFragger +0.96 and MetaMorpheus +1.059.
    ///
    /// The doc here previously claimed "median SIGNED MS2 fragment ppm error";
    /// that was wrong and is withdrawn. **Do NOT compare this field with another
    /// engine's signed MS2 error** — that is the use it was added for in the
    /// 2026-08-19 review, and it does not support that use.
    ///
    /// Recon deliberately does NOT reconstruct the signed value. It would need
    /// `sage --annotate-matches`, a second output file (8.5 MB / 210k rows for
    /// ONE file), a `psm_id` join, and a schema change — and it cannot change
    /// what recon actually recommends: the MS2 tolerance below is symmetric
    /// about ZERO and never reads this field, and on all three test files the
    /// bucket requirement is 2.9-4.4 ppm against a 10 ppm ladder step, where the
    /// signed correction moves it by 0.027 ppm. See NOTES "MS2 stays absolute".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Median |fragment error|, NOT a bias.
    ///
    /// ⚠ **RENAMED 2026-09-01 because the old name `ms2_bias_ppm` was a lie.**
    /// Sage's `fragment_ppm` is ABSOLUTE — v0.15 made only `precursor_ppm`
    /// signed — so a value derived from it can never be negative and cannot
    /// express a direction. Reporting it as a "bias" invited the reader to
    /// interpret a magnitude as a systematic offset. MS1 is signed; MS2 is not.
    pub ms2_median_abs_ppm: Option<f64>,
    /// Spread of the same ABSOLUTE quantity. Understated by folding — measured
    /// ~15% low on serum (0.4440 here vs 0.5114 signed, like-for-like per-PSM).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ms2_spread_mad_ppm: Option<f64>,
}

/// Pass 2 — the semi-enzymatic subset search, written to `<output>_pass2.json`.
///
/// Deliberately a SEPARATE artifact rather than a block inside `ReconReport`.
/// Pass 2 is produced by `recon run`; `ReconReport` is produced by `recon
/// analyze`, and `_dev/testing/recon-output/full-run/` is a frozen set of ANALYZE
/// outputs. Folding Pass 2 into that schema would move the regeneration on one
/// more axis at the same time as the others, which is exactly what the
/// regeneration impact trace exists to prevent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pass2Report {
    pub schema_version: String,
    pub generated_at: DateTime<Utc>,
    pub tool_version: String,
    pub git_commit: String,

    /// The mzML Pass 2 searched — the same raw file Pass 1 used.
    pub source_file: String,
    /// Effective Pass 2 config, which carries its own provenance block.
    pub effective_params: String,
    /// Subset FASTA Pass 2 searched, and how many proteins it held.
    pub subset_fasta: String,
    pub subset_proteins: usize,

    /// MS1 window applied, in DELTA space (not the inverted config values).
    pub ms1_window_low_ppm: f64,
    pub ms1_window_high_ppm: f64,
    /// MS2 fragment tolerance applied, rendered with its unit
    /// (`"±5.6 ppm"` / `"±0.5 Da"`), or `None` when Pass 1 measured no MS2
    /// error and the template's own value was kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ms2_tolerance: Option<String>,
    /// The Pass-1 fragment tolerance the MS2 number is clamped against.
    pub pass1_fragment_tol: String,

    /// Terminus breakdown over Pass 2's confident target PSMs.
    pub terminus: crate::digestion::TerminusStats,
    /// Missed cleavages and semi-tryptic rate as Pass 2 saw them.
    ///
    /// ⚠ PSM-basis, decoys already dropped, no per-class correction. Retained
    /// because it is what 1.0.0 consumers read. `composition` is the reported
    /// quantity; this is the raw material.
    pub digestion: crate::digestion::DigestionResult,

    /// THE REPORTED DIGESTION NUMBERS.
    ///
    /// Distinct peptides, Preview's denominators, per-class decoy subtraction.
    /// See `digestion::DigestionComposition` for the derivation and for the
    /// reference values this was validated against.
    pub composition: crate::digestion::DigestionComposition,

    /// Pass-1 prediction against Pass-2 observation. See `Pass2Comparison`.
    pub comparison: Pass2Comparison,

    /// Wall-clock seconds for the Pass 2 Sage search alone.
    pub pass2_search_seconds: f64,
}

/// What Pass 1 predicted about digestion, next to what Pass 2 measured.
///
/// The point of the comparison is that Pass 1 CANNOT see ragged termini: it is a
/// fully-tryptic search, so every semi-tryptic peptide in the sample is either
/// missed or mis-assigned. Pass 1's `semi_enzymatic_pct` therefore reads from
/// Sage's own `semi_enzymatic` column on a search that could not generate one,
/// and is expected to be ~0. Pass 2 is the measurement; Pass 1 is the control
/// that shows the measurement was necessary.
///
/// ⚠ The two rates have DIFFERENT DENOMINATORS (each pass's own confident PSM
/// count) and different search spaces (whole database vs subset). They are not
/// a before/after of one quantity, and the difference must not be read as a
/// delta. Both counts are carried so a reader can see that.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pass2Comparison {
    pub pass1_psms: usize,
    pub pass1_semi_enzymatic_pct: f64,
    pub pass2_psms: usize,
    pub pass2_semi_enzymatic_pct: f64,
    /// Semi-tryptic PSMs Pass 2 found that Pass 1's search space could not
    /// have contained at all.
    pub pass2_only_semi_enzymatic: usize,
    /// Plain-language reading, generated so the JSON explains itself.
    pub note: String,
}

/// Complete reconnaissance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconReport {
    /// Schema version
    pub schema_version: String,
    /// Wall-clock seconds for the WHOLE run, filled in once every stage is done.
    ///
    /// `None` from `analyze`, which is one stage of a run rather than the run.
    /// `run` writes the report before Pass 2 (Pass 2 consumes it), then rewrites
    /// both files once the total is known — so this is genuinely the total and
    /// not the time to first output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_seconds: Option<f64>,
    /// Generation timestamp
    pub generated_at: DateTime<Utc>,
    /// Tool version
    pub tool_version: String,
    /// Git commit the binary was built from (short SHA; may carry `-dirty` if
    /// the build had uncommitted tracked changes; `"unknown"` if git absent).
    /// Together with `tool_version` + inputs, makes each report self-describing
    /// and provably tied to a code state — testing artifacts stay unique across
    /// code updates. See `provenance.rs`.
    #[serde(default)]
    pub git_commit: String,

    /// Input file information
    pub input: InputInfo,

    /// Modification discovery results
    pub mod_discovery: ModDiscoverySummaryReport,

    /// Signal fate results
    pub signal_fate: SignalFateSummaryReport,

    /// Polymer contamination results
    pub polymer: PolymerSummaryReport,

    /// Oxonium screening results
    pub oxonium: OxoniumSummaryReport,

    /// Digestion efficiency results
    pub digestion: DigestionSummaryReport,

    /// Mass accuracy results
    pub mass_accuracy: MassAccuracySummaryReport,

    /// Alkylation check results
    pub alkylation: AlkylationCheck,

    /// Self-calibrated MS1/MS2 tolerance recommendation from the open
    /// search's clean subset (see `Ms1CalibrationReport` docs). `None` when
    /// the clean subset was empty (no qualifying near-zero-delta PSMs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ms1_calibration: Option<Ms1CalibrationReport>,

    /// Step-2 search-parameter recommendation. `None` when the curated mod list
    /// could not be loaded, which is a configuration problem, not an empty result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommendations: Option<ModRecommendations>,

    /// Which mass analyzer acquired the MS1 and MS2 scans, and the pass-1 MS2
    /// tolerance that implies.
    ///
    /// This is a RECORD of a decision the run already made, not a new one:
    /// `recon run` detects the analyzer from the mzML and overrides the
    /// template's fixed `fragment_tol` with it BEFORE the pass-1 search. Without
    /// this block the report shows a peak list and a tolerance recommendation
    /// with no way to tell what instrument they came from — and a ppm ladder is
    /// meaningless for an ion trap, which needs ~0.5-1.0 Da.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analyzers: Option<AnalyzerReport>,
}

/// The detected analyzers and the pass-1 MS2 tolerance they imply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerReport {
    /// Instrument model CV term, when the mzML declares one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instrument_model: Option<String>,
    /// Analyzer labels seen at MS1.
    pub ms1_analyzers: Vec<String>,
    /// Analyzer labels seen at MS2.
    pub ms2_analyzers: Vec<String>,
    /// True when the sampled MS2 scans do not agree on one analyzer. A mixed
    /// file cannot have one right fragment tolerance.
    pub ms2_switched: bool,
    /// The pass-1 MS2 `fragment_tol` actually applied, formatted as written.
    pub pass1_fragment_tol: String,
    /// What decided it.
    pub basis: String,
    /// True when recon substituted an assumption for a real detection. The
    /// tolerance is then a DEFAULT, not a measurement, and must be read as one.
    pub assumed: bool,
    pub explanation: String,
}

impl ReconReport {
    /// Create a new report from analysis results
    ///
    /// ⚠ `clippy::too_many_arguments` fires here (16/7) and is ALLOWED, not
    /// fixed. Collapsing these into a struct is a signature change across the
    /// whole report-building path, and every argument is a measured quantity.
    /// The failure mode of that refactor is a silently swapped field, which is
    /// exactly the class of defect this project cannot detect cheaply, against
    /// no functional gain. Revisit only with a reason beyond the lint.
    #[allow(clippy::too_many_arguments)]
    pub fn from_analyses(
        mzml_file: &str,
        sage_tsv: &str,
        unimod_file: &str,
        fasta_file: Option<&str>,
        enzyme: Option<&crate::enzyme::Enzyme>,
        mzml_stats: &MzmlStats,
        mod_discovery: &ModDiscoveryResult,
        signal_fate: &SignalFateResult,
        polymer: &PolymerSearchResults,
        oxonium: &OxoniumScreeningSummary,
        digestion: &DigestionResult,
        qc: &QcResult,
        alkylation: AlkylationCheck,
        ms1_calibration: Option<Ms1CalibrationReport>,
        recommendations: Option<ModRecommendations>,
        analyzers: Option<AnalyzerReport>,
    ) -> Self {
        // Build input info
        let input = InputInfo {
            mzml_file: mzml_file.to_string(),
            sage_tsv: sage_tsv.to_string(),
            unimod_file: unimod_file.to_string(),
            ms1_spectra: mzml_stats.ms1_spectra,
            ms2_spectra: mzml_stats.ms2_spectra,
            fasta_file: fasta_file.map(|f| f.to_string()),
            enzyme: enzyme.map(EnzymeInfo::from),
        };

        // A Unimod-named row gets Unimod's acceptor sites and position too.
        //
        // ⚠ **IT MUST NOT READ `PeakAnnotation.sites`.** That was the first
        // attempt and it filled 1 row of 10. MEASURED cause, not a guess:
        // `PeakAnnotation.sites` is `UnimodEntry::sites()`, which drops every
        // specificity marked `hidden="1"` and has NO fallback when they all are.
        // In the pinned `unimod.xml`, `Pyro-carbamidomethyl` is the only one of
        // the eleven with a `hidden="0"` specificity; `CarbamidomethylDTT`,
        // `Lys->Allysine`, `Arg->Npo`, `Gly+O(2)`, `Ammonium`, `Cation:Al[III]`,
        // `Cation:Ni[II]`, `Xlink:SMCC[219]` and `Unknown:210` are ALL-HIDDEN and
        // came back empty. Specificity COUNT is not the discriminator --
        // `Cation:Al[III]` has three and failed, `Pyro-carbamidomethyl` has one
        // and worked.
        //
        // The entry's own `specificities` are read instead, with the same
        // all-hidden fallback `UnimodEntry::classification` already documents for
        // amino-acid substitutions. `unimod_acceptor` does that.
        //
        // The lookup key is `unimod_id` -- the record the ANNOTATION already
        // chose -- never the title, so a name and its residues cannot come from
        // two different entries. `mono_mass` is then asserted against the
        // annotation before anything is written: if a `--unimod` override put a
        // different entry at that id, the row keeps an empty acceptor rather than
        // naming the wrong residues.
        let recommendations = recommendations.map(|mut rec| {
            // ⚠ A SATELLITE IS SKIPPED. It stops at step 1 of the flow chart with
            // `decided_by: null` and never reached a residue test; printing
            // acceptors there would imply a test that never ran.
            let wants = |n: &NotRecommended| {
                n.name_source.as_deref() == Some("unimod")
                    && n.sites.is_empty()
                    && !n.reason.starts_with("satellite")
            };
            let needed = rec
                .not_recommended
                .iter()
                .chain(rec.notable_unannotated.iter())
                .any(wants);
            // Parsed only when a row actually needs it, and only the copy
            // compiled into this binary. A failure to parse is not fatal: the
            // rows keep their names and lose only the acceptor.
            let db = if needed {
                match crate::unimod::UnimodDb::from_embedded() {
                    Ok(db) => Some(db),
                    Err(e) => {
                        log::warn!("Unimod acceptor sites unavailable: {e:#}");
                        None
                    }
                }
            } else {
                None
            };

            if let Some(db) = db {
                let acceptor = |delta: f64| -> Option<(String, String)> {
                    let a = mod_discovery
                        .peaks
                        .iter()
                        .find(|p| (p.delta_mass - delta).abs() < 1e-9)?
                        .annotations
                        .first()?;
                    let entry = db.get_by_id(a.unimod_id?)?;
                    // The guard. Same id AND same mass, or nothing is written.
                    if (entry.mono_mass - a.delta_mass).abs() > 1e-9 {
                        return None;
                    }
                    Some(unimod_acceptor(entry))
                };
                for n in rec
                    .not_recommended
                    .iter_mut()
                    .chain(rec.notable_unannotated.iter_mut())
                {
                    if !wants(n) {
                        continue;
                    }
                    if let Some((sites, position)) = acceptor(n.delta_mass) {
                        n.sites = sites;
                        if n.position.is_empty() {
                            n.position = position;
                        }
                    }
                }
            }
            rec
        });

        // Build mod discovery summary
        let mod_discovery_summary = ModDiscoverySummaryReport {
            total_psms: mod_discovery.summary.total_psms,
            unmodified_pct: mod_discovery.summary.psms_near_zero_pct,
            peaks: mod_discovery.peaks.clone(),
            discovery_settings: mod_discovery.discovery_settings.clone(),
        };

        // Build signal fate summary
        let signal_fate_summary = SignalFateSummaryReport {
            id_rate_by_count_pct: signal_fate.by_count.identified_pct.unwrap_or(0.0),
            id_rate_by_tic_pct: signal_fate.by_intensity.identified_pct.unwrap_or(0.0),
            identified_spectra: signal_fate.by_count.identified_spectra,
            total_ms2_spectra: signal_fate
                .unidentified
                .as_ref()
                .map(|u| u.total_ms2_spectra)
                .unwrap_or(mzml_stats.ms2_spectra),
            chimera_rate_pct: signal_fate.chimera_stats.chimera_rate_pct,
        };

        // Build polymer summary
        let total_pct_tic = polymer.total_polymer_pct_tic();
        let contamination_level = if total_pct_tic < 0.1 {
            "Low (< 0.1%)"
        } else if total_pct_tic < 1.0 {
            "Moderate (0.1-1%)"
        } else if total_pct_tic < 5.0 {
            "High (1-5%)"
        } else {
            "Very High (> 5%)"
        }
        .to_string();

        let top_polymers: Vec<PolymerEntry> = polymer
            .polymers_by_pct_tic()
            .into_iter()
            .filter(|(_, pct)| *pct > 0.0)
            .take(10)
            .map(|(name, pct)| PolymerEntry { name, pct_tic: pct })
            .collect();

        let polymer_summary = PolymerSummaryReport {
            total_pct_tic,
            contamination_level,
            top_polymers,
        };

        // Build oxonium summary
        let oxonium_summary = OxoniumSummaryReport {
            glycopeptide_candidates: oxonium.glycopeptide_candidates,
            glycopeptide_pct: oxonium.glycopeptide_pct,
        };

        // Build digestion summary
        let mc0 = digestion
            .missed_cleavages
            .distribution
            .iter()
            .find(|m| m.missed == 0)
            .map(|m| m.pct)
            .unwrap_or(0.0);
        let mc1 = digestion
            .missed_cleavages
            .distribution
            .iter()
            .find(|m| m.missed == 1)
            .map(|m| m.pct)
            .unwrap_or(0.0);
        let mc2plus: f64 = digestion
            .missed_cleavages
            .distribution
            .iter()
            .filter(|m| m.missed >= 2)
            .map(|m| m.pct)
            .sum();

        let digestion_summary = DigestionSummaryReport {
            missed_cleavage_0_pct: mc0,
            missed_cleavage_1_pct: mc1,
            missed_cleavage_2plus_pct: mc2plus,
            ragged_ends_pct: digestion.semi_enzymatic.semi_enzymatic_pct,
        };

        // Build mass accuracy summary
        let mass_accuracy_summary = MassAccuracySummaryReport {
            precursor_median_ppm: qc.precursor_ppm.median,
            precursor_p95_ppm: qc.precursor_ppm.percentile_95,
            fragment_median_ppm: qc.fragment_ppm.median,
            fragment_p95_ppm: qc.fragment_ppm.percentile_95,
        };

        ReconReport {
            schema_version: SCHEMA_VERSION.to_string(),
            runtime_seconds: None,
            generated_at: Utc::now(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            git_commit: crate::provenance::GIT_COMMIT.to_string(),
            input,
            mod_discovery: mod_discovery_summary,
            signal_fate: signal_fate_summary,
            polymer: polymer_summary,
            oxonium: oxonium_summary,
            digestion: digestion_summary,
            mass_accuracy: mass_accuracy_summary,
            alkylation,
            ms1_calibration,
            recommendations,
            analyzers,
        }
    }
}

/// Compute alkylation check from PSMs
///
/// Looks for Cys-containing peptides with delta mass near -57 Da,
/// which indicates unalkylated cysteine.
pub fn compute_alkylation_check(psms: &[crate::sage_results::Psm]) -> AlkylationCheck {
    const CARBAMIDOMETHYL_MASS: f64 = 57.021464;
    const TOLERANCE_DA: f64 = 0.5;

    // Filter to Cys-containing PSMs
    let cys_psms: Vec<_> = psms.iter().filter(|p| p.peptide.contains('C')).collect();

    let cys_psm_count = cys_psms.len();
    let total_psms = psms.len();

    let cys_psm_pct = if total_psms > 0 {
        100.0 * (cys_psm_count as f64) / (total_psms as f64)
    } else {
        0.0
    };

    // Count PSMs with delta mass near -57 Da (unalkylated Cys)
    let unalkylated_count = cys_psms
        .iter()
        .filter(|p| (p.delta_mass_corrected + CARBAMIDOMETHYL_MASS).abs() < TOLERANCE_DA)
        .count();

    let unalkylated_pct = if cys_psm_count > 0 {
        100.0 * (unalkylated_count as f64) / (cys_psm_count as f64)
    } else {
        0.0
    };

    // Determine status
    let status = if cys_psm_count == 0 {
        "No Cys-containing peptides found".to_string()
    } else if unalkylated_pct < 1.0 {
        "✓ Alkylation appears complete".to_string()
    } else if unalkylated_pct < 5.0 {
        format!("⚠ Minor incomplete alkylation ({:.1}%)", unalkylated_pct)
    } else {
        format!(
            "✗ Significant incomplete alkylation ({:.1}%)",
            unalkylated_pct
        )
    };

    AlkylationCheck {
        fixed_mod_assumed: format!("Carbamidomethyl (+{:.2} Da) on C", CARBAMIDOMETHYL_MASS),
        cys_psm_count,
        cys_psm_pct,
        unalkylated_count,
        unalkylated_pct,
        status,
    }
}

/// Print text summary to console
pub fn print_report_summary(report: &ReconReport) {
    println!("================================================================================");
    println!("                    PROTEOMICS RECONNAISSANCE REPORT");
    println!("================================================================================");
    println!("File: {}", report.input.mzml_file);
    println!(
        "Generated: {}",
        report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!(
        "Tool: recon-tool v{} (commit {})",
        report.tool_version, report.git_commit
    );
    println!();

    // Signal Fate
    println!("--- SIGNAL FATE ---");
    println!(
        "ID Rate (% of {} MS2 spectra):  {:.1}% ({} identified)",
        report.signal_fate.total_ms2_spectra,
        report.signal_fate.id_rate_by_count_pct,
        report.signal_fate.identified_spectra
    );
    println!(
        "ID Rate (% of MS2 TIC):         {:.1}%",
        report.signal_fate.id_rate_by_tic_pct
    );
    println!(
        "Chimeric scans:                 {:.1}%",
        report.signal_fate.chimera_rate_pct
    );
    println!();

    // Modification Landscape
    println!("--- MODIFICATION LANDSCAPE ---");
    println!(
        "Total PSMs:              {}",
        report.mod_discovery.total_psms
    );
    println!(
        "Unmodified:              {:.1}%",
        report.mod_discovery.unmodified_pct
    );
    println!();
    println!("Top 10 Modifications:");
    println!(
        "  {:<6} {:<12} {:<10} {:<8} Annotation",
        "Rank", "Delta (Da)", "Count", "%"
    );
    println!("  {}", "-".repeat(60));

    for peak in report.mod_discovery.peaks.iter().take(10) {
        let annotation = if peak.unannotated {
            "UNANNOTATED".to_string()
        } else if let Some(ann) = peak.annotations.first() {
            ann.name.clone()
        } else {
            "?".to_string()
        };

        println!(
            "  {:<6} {:<12.4} {:<10} {:<8.1} {}",
            peak.rank, peak.delta_mass, peak.count, peak.count_pct, annotation
        );
    }

    if report.mod_discovery.peaks.len() > 10 {
        println!(
            "  ... and {} more peaks (see JSON for full list)",
            report.mod_discovery.peaks.len() - 10
        );
    }
    println!();

    // Contamination
    println!("--- CONTAMINATION ---");
    println!(
        "Polymer (% of MS1 TIC):  {:.2}% ({})",
        report.polymer.total_pct_tic, report.polymer.contamination_level
    );
    if !report.polymer.top_polymers.is_empty() {
        for poly in report.polymer.top_polymers.iter().take(5) {
            println!("  {}: {:.3}%", poly.name, poly.pct_tic);
        }
    }
    println!();

    // Glycopeptides
    println!("--- GLYCOPEPTIDES ---");
    println!(
        "Oxonium-positive:        {} spectra ({:.1}% of MS2)",
        report.oxonium.glycopeptide_candidates, report.oxonium.glycopeptide_pct
    );
    println!();

    // Digestion
    println!("--- DIGESTION ---");
    println!(
        "Missed Cleavages (% of PSMs):  0: {:.1}% | 1: {:.1}% | 2+: {:.1}%",
        report.digestion.missed_cleavage_0_pct,
        report.digestion.missed_cleavage_1_pct,
        report.digestion.missed_cleavage_2plus_pct
    );
    println!(
        "Ragged Ends (% of PSMs):       {:.1}%",
        report.digestion.ragged_ends_pct
    );
    println!();

    // Mass Accuracy
    println!("--- MASS ACCURACY ---");
    println!(
        "Fragment (MS2):          median |error| {:.2} ppm, 95th percentile {:.2} ppm",
        report.mass_accuracy.fragment_median_ppm, report.mass_accuracy.fragment_p95_ppm
    );
    println!(
        "                         (a MAGNITUDE, not a bias: Sage {} reports fragment_ppm as |error|)",
        crate::sage_runner::SAGE_VERSION
    );
    println!("Precursor (MS1):         NOT a mass accuracy in an open search — see below.");
    println!(
        "                         raw signed precursor_ppm over ALL PSMs: median {:.2} ppm, 95th pct {:.2} ppm.",
        report.mass_accuracy.precursor_median_ppm, report.mass_accuracy.precursor_p95_ppm
    );
    println!(
        "                         This spans the whole open window because the precursor delta"
    );
    println!(
        "                         carries the MODIFICATION mass. The usable MS1 number is the"
    );
    println!("                         self-calibrated bias in the next block.");
    println!();

    // Self-calibrated MS1/MS2 recommendation (open-search clean subset)
    if let Some(ref cal) = report.ms1_calibration {
        println!("--- SELF-CALIBRATED TOLERANCE RECOMMENDATION (open-search clean subset) ---");
        println!(
            "Clean subset: {} PSMs (near-zero delta, rank-1, target, q<0.01){}",
            cal.clean_subset_n_psms,
            if cal.hyperscore_guard_applied {
                ", hyperscore-guarded"
            } else {
                ", unguarded (subset too small for guard)"
            }
        );
        println!(
            "MS1 bias: {:+.2} ppm, spread (MAD): {:.2} ppm",
            cal.bias_ppm, cal.spread_mad_ppm
        );
        match cal.user_recommendation_tolerance_ppm {
            Some(t) => {
                println!(
                    "  User recommendation for your NEXT search: +/-{t:.0} ppm \
                     (quantized; measured requirement |bias|+5*MAD = {:.2} ppm)",
                    cal.user_recommendation_requirement_ppm
                        .unwrap_or(cal.bias_ppm.abs() + 5.0 * cal.spread_mad_ppm)
                );
                if cal.user_recommendation_exceeds_ladder == Some(true) {
                    println!(
                        "  ⚠ MEASURED REQUIREMENT EXCEEDS THE LADDER — +/-{t:.0} ppm is \
                         KNOWN TOO NARROW for this file. The instrument looks \
                         mis-calibrated; recalibrate rather than widen."
                    );
                }
            }
            None => println!(
                "  User recommendation: {:+.2} to {:+.2} ppm",
                cal.user_recommendation_low_ppm, cal.user_recommendation_high_ppm
            ),
        }
        println!(
            "  Pass 2 precursor_tol (tight, internal use only, capped ±100 ppm): {:+.2} to {:+.2} ppm",
            cal.pass2_window_low_ppm, cal.pass2_window_high_ppm
        );
        if let (Some(bias), Some(mad)) = (cal.ms2_median_abs_ppm, cal.ms2_spread_mad_ppm) {
            println!(
                "  MS2 |error|: {:.2} ppm, spread (MAD): {:.2} ppm \
                 (Sage fragment_ppm — an intensity-weighted mean of |error|, NOT a signed bias)",
                bias, mad
            );
        }
        if let (Some(lo), Some(hi)) = (cal.ms2_tolerance_low_ppm, cal.ms2_tolerance_high_ppm) {
            println!(
                "  MS2 fragment tolerance (+/-95th-pct tail around zero): {:+.2} to {:+.2} ppm",
                lo, hi
            );
        }
        println!();
    }

    // Alkylation Check
    println!("--- ALKYLATION CHECK ---");
    println!(
        "Fixed mod assumed:       {}",
        report.alkylation.fixed_mod_assumed
    );
    println!(
        "Cys-containing PSMs:     {} / {} ({:.1}%)",
        report.alkylation.cys_psm_count,
        report.mod_discovery.total_psms,
        report.alkylation.cys_psm_pct
    );
    println!(
        "Unalkylated Cys signal:  {} PSMs ({:.1}% of Cys PSMs)",
        report.alkylation.unalkylated_count, report.alkylation.unalkylated_pct
    );
    println!("Status:                  {}", report.alkylation.status);
    println!();

    println!("================================================================================");
}

/// Escape text taken from data before it goes into HTML.
///
/// Curated and Unimod names contain `>` (e.g. `Gln->pyro-Glu`), which is inert
/// here but must not be able to open a tag.
fn esc(v: &str) -> String {
    v.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// "E · peptide N-term" — the acceptor, with the positional restriction that
/// makes it a usable search setting.
///
/// `sites` alone under-specifies: a terminal candidate is tested positionally
/// ("first residue is E"), so a bare "E" would send a reader to configure a
/// much larger, noisier search than the one recon actually tested.
fn acceptor_cell(sites: &str, position: &str) -> String {
    let protein_terminal =
        position.starts_with("Protein N-terminal") || position.starts_with("Protein C-terminal");
    let residues = if sites.is_empty() && protein_terminal {
        // NOT "unspecific". An empty acceptor set at a PROTEIN terminus means
        // any residue is accepted AT A POSITION that holds ~1% of PSMs, which is
        // more specific than most residue sets, not less. Calling it unspecific
        // would tell a reader the opposite of what the statistics measured.
        //
        // ⚠ The layout mockup collapsed BOTH empty-acceptor cases to "any
        // residue". That is not ported: the distinction is a measurement, not a
        // layout choice.
        "<strong>any residue</strong>".to_string()
    } else if sites.is_empty() {
        "<span class=\"mut\">unspecific</span>".to_string()
    } else {
        let joined: Vec<String> = sites.chars().map(|c| c.to_string()).collect();
        format!("<strong>{}</strong>", esc(&joined.join(", ")))
    };
    let pos = position.trim_end_matches('.');
    let pos_note = match pos {
        "" | "Anywhere" => String::new(),
        "Peptide N-terminal" => " · peptide N-term".to_string(),
        "Protein N-terminal" => " · protein N-term".to_string(),
        "Protein N-terminal, Met loss" => " · protein N-term, Met loss".to_string(),
        other => format!(" · {}", esc(other)),
    };
    format!("{residues}<span class=\"mut\">{pos_note}</span>")
}

/// One Unimod entry's acceptor residues and position, in the CURATED vocabulary.
///
/// Returns `(sites, position)` shaped exactly like `RecommendedMod`'s pair, so
/// `acceptor_cell` renders a Unimod-named row and a curated one identically.
///
/// **The all-hidden fallback is the point of this function.** `UnimodEntry::sites`
/// keeps only `hidden="0"` specificities and returns nothing when every one is
/// hidden — which is the common case, not an edge case: 10 of the 11 named rows
/// on serum are all-hidden entries. `UnimodEntry::classification` already carries
/// the same fallback, with the same reasoning recorded on it ("important for AA
/// substitutions which are all marked hidden=1 in Unimod"). This mirrors it
/// rather than inventing a second rule.
///
/// ⚠ `UnimodEntry::sites` is deliberately NOT changed. It feeds
/// `mod_discovery.peaks[].annotations[].sites`, an existing published field, and
/// widening it there would move values that nothing asked to move.
///
/// Only single-letter sites are residues. Unimod writes exactly two multi-letter
/// pseudo-sites — `N-term` and `C-term` — and those are POSITIONS, so they are
/// read through `position` and never printed as an acceptor.
///
/// The position is taken only when every RESIDUE-bearing specificity agrees on
/// one. A Unimod entry may carry several (`Ammonium` is D and E `Anywhere` plus a
/// `C-term`), and the curated vocabulary has room for exactly one. Disagreement
/// yields an empty position — the residues still stand, and no restriction is
/// claimed that the entry does not support.
fn unimod_acceptor(entry: &crate::unimod::UnimodEntry) -> (String, String) {
    let visible: Vec<&crate::unimod::ModSpecificity> =
        entry.specificities.iter().filter(|s| !s.hidden).collect();
    let specs: Vec<&crate::unimod::ModSpecificity> = if visible.is_empty() {
        entry.specificities.iter().collect()
    } else {
        visible
    };

    let residue_specs: Vec<&&crate::unimod::ModSpecificity> = specs
        .iter()
        .filter(|s| s.site.chars().count() == 1)
        .collect();

    let mut sites: Vec<char> = residue_specs
        .iter()
        .filter_map(|s| s.site.chars().next())
        .collect();
    sites.sort_unstable();
    sites.dedup();

    // Terminal-only entries still have a position worth reporting.
    let positional: Vec<&str> = if residue_specs.is_empty() {
        specs.iter().map(|s| s.position.as_str()).collect()
    } else {
        residue_specs.iter().map(|s| s.position.as_str()).collect()
    };
    let agreed = positional.first().copied().filter(|first| {
        positional
            .iter()
            .all(|p| p.eq_ignore_ascii_case(first) || p == first)
    });

    // Unimod's five position strings, mapped onto the words the curated list
    // uses. `acceptor_cell` keys off these exactly — "Protein N-terminal" is what
    // makes an empty acceptor read "any residue" instead of "unspecific" — so an
    // unrecognised string yields NO position rather than a near-miss.
    let position = match agreed {
        Some("Anywhere") => "Anywhere.",
        Some("Any N-term") => "Peptide N-terminal.",
        Some("Any C-term") => "Peptide C-terminal.",
        Some("Protein N-term") => "Protein N-terminal.",
        Some("Protein C-term") => "Protein C-terminal.",
        _ => "",
    };

    (sites.into_iter().collect(), position.to_string())
}

/// Every analyzer class, for a label lookup. Kept beside the lookup that uses it
/// and pinned by `every_analyzer_label_maps_back_to_its_class`, so a new variant
/// cannot be added upstream without this list failing a test.
const ALL_ANALYZER_CLASSES: [crate::mzml::AnalyzerClass; 5] = [
    crate::mzml::AnalyzerClass::Orbitrap,
    crate::mzml::AnalyzerClass::AstralTof,
    crate::mzml::AnalyzerClass::LegacyTof,
    crate::mzml::AnalyzerClass::IonTrap,
    crate::mzml::AnalyzerClass::Unclassified,
];

/// The UNIT the MS2 tolerance recommendation must be expressed in.
///
/// ⚠ **THIS IS THE GUARD THAT KEEPS A PPM RUNG AWAY FROM AN ION TRAP.** The
/// ladder is ppm-only. A trap or quadrupole at unit resolution needs ~0.3-0.8 Da
/// (`ION_TRAP_MS2_HALF_WIDTH_DA`), and a rung would be meaningless there.
///
/// It does not detect anything. `AnalyzerReport` records the analyzers recon
/// already detected before the pass-1 search, as the human labels
/// `AnalyzerClass::label` produces; this maps those labels back onto their class
/// and asks `bucket_tolerance` — the one model of what unit a class needs — which
/// unit that class takes.
///
/// **Any Da-unit analyzer in the MS2 census wins.** A run that switches detectors
/// has no single right answer, and a ppm number would be wrong for the trap half
/// of it. Falling to daltons is the harmless direction: too wide on an Orbitrap
/// is a worse search, too narrow on a trap finds nothing.
///
/// With no analyzer block at all the fallback is ppm, matching
/// `UNKNOWN_MS2_FALLBACK_PPM` — recon already searches such a file as if it were
/// an Orbitrap, and the recommendation must not disagree with the search.
fn ms2_recommendation_unit(analyzers: Option<&AnalyzerReport>) -> crate::mzml::FragmentTolerance {
    use crate::mzml::{bucket_tolerance, FragmentTolerance};
    let ppm = FragmentTolerance::Ppm(crate::mzml::UNKNOWN_MS2_FALLBACK_PPM);
    let Some(a) = analyzers else { return ppm };
    for label in &a.ms2_analyzers {
        let class = ALL_ANALYZER_CLASSES.iter().find(|c| c.label() == label);
        if let Some(tol @ FragmentTolerance::Da(_)) = class.and_then(|c| bucket_tolerance(*c)) {
            return tol;
        }
    }
    ppm
}

/// Group an integer with thousands separators, as the mockup's `{:,}` does.
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let first = digits.len() % 3;
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && i % 3 == first {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A q value, written the way a reader compares it against `Q_MAX` (0.05).
///
/// ⚠ **This was `{q:.1e}` and it made a correct report unreadable.** The cut
/// table shows the q that failed the gate beside the bound it failed, so the
/// closest call in the committed serum report printed as
/// `q 6.9e-2 (needs <= 0.05)`. Deciding that 6.9e-2 exceeds 0.05 takes a mental
/// conversion, and four of serum's six cut rows pass the OR gate — so the row
/// showed a passing number next to a failing one that did not LOOK like it was
/// failing. Ben read the table and could not tell why those rows were cut.
///
/// The rule, plain: below a thousandth say so, otherwise three decimals.
/// `0.001` is fine as the floor because the only comparison that matters here is
/// against 0.05, and everything below 0.001 clears it decisively.
///
/// This subsumes the old `q == 0.0` branch, which existed because a serialised
/// zero is underflow rather than certainty and printing `0` would "claim
/// infinite evidence". `< 0.001` keeps exactly that honesty and drops the
/// `1e-300` magic number.
///
/// ⚠ Known edge, deliberately not special-cased: a q of exactly `0.0500` prints
/// `0.050` against an INCLUSIVE `<= 0.05` bound, so a pass and a fail look
/// alike. No such value exists in any of the four committed reports (checked
/// across all 49 q values), and a special case would cost more clarity than the
/// edge is worth.
fn fmt_q(q: f64) -> String {
    if q < 0.001 {
        "&lt; 0.001".to_string()
    } else {
        format!("{q:.3}")
    }
}

/// The evidence behind one recommendation, in the terms of the path that decided it.
fn evidence_cell(m: &RecommendedMod) -> String {
    match (m.odds_ratio, m.q_value, m.pct_of_floor) {
        (Some(or), Some(q), _) => {
            // Large odds ratios do not need decimals; small ones do.
            let or_txt = if or >= 100.0 {
                format!("{or:.0}")
            } else {
                format!("{or:.2}")
            };
            format!("OR {or_txt}, q {}", fmt_q(q))
        }
        (None, None, Some(pf)) => format!("{pf:.0}% of floor"),
        _ => "&mdash;".to_string(),
    }
}

/// One row of the Fixed or Variable table.
///
/// Column order is the mockup's: delta mass first, then the name, then residue
/// and position sharing ONE column.
fn rec_row(m: &RecommendedMod) -> String {
    format!(
        "<tr><td class=\"num\">{delta:+.4}</td><td>{label}</td><td>{acceptor}</td>\
         <td class=\"num\">{count}</td><td class=\"num\">{pct:.2}%</td>\
         <td>{by}</td><td class=\"why\">{ev}</td></tr>",
        delta = m.delta_mass,
        label = esc(&m.label),
        acceptor = acceptor_cell(&m.sites, &m.position),
        count = thousands(m.count),
        pct = m.count_pct,
        by = esc(&m.decided_by),
        ev = evidence_cell(m),
    )
}

/// One row of "Detected but did not make the cut".
///
/// ⚠ **THE MOCKUP'S REASON STRINGS ARE SCHEMA 2.0.0 AND NO LONGER EXIST.** It
/// matched `no_residue_support`, `not_curated` and `satellite…`. The emitter in
/// `tier_assignment.rs` now writes `failed_residue_test`, `below_floor`,
/// `below_floor_uncurated`, `satellite (…)` and `uncurated — above floor (…)`.
/// Ported verbatim, every non-satellite row would have fallen through to the
/// mockup's else branch and rendered "Decided by: —". Every current string is
/// mapped explicitly below.
fn cut_row(n: &NotRecommended, rec: &ModRecommendations) -> String {
    let reason = n.reason.as_str();
    let (decided, why): (&str, String) = if reason.starts_with("satellite") {
        // ⚠ NOT "floor". A satellite stops at step 1 of the flow chart and
        // never reaches the abundance floor, and `decided_by` is null for these
        // rows in the JSON. Rendering "floor" told the reader the row took a
        // route it did not take, contradicting the chart printed directly below
        // the table. Found 2026-09-03 by reading the rendered report.
        (
            "satellite",
            format!("isotope satellite &mdash; {}", esc(reason)),
        )
    } else if reason == "failed_residue_test" {
        // Tested at the residue level and FAILED. Both gates are shown, because
        // an odds ratio that clears the bar with a failing q reads as unexplained
        // otherwise.
        let or = n
            .odds_ratio
            .map(|o| format!("{o:.2}"))
            .unwrap_or_else(|| "not recorded".to_string());
        let q = n
            .q_value
            .map(fmt_q)
            .unwrap_or_else(|| "not recorded".to_string());
        (
            "statistics",
            format!(
                "failed the residue test &mdash; OR {or} (needs &ge; {or_min:.1}), \
                 q {q} (needs &le; {q_max})",
                or_min = rec.odds_ratio_min,
                q_max = rec.q_max,
            ),
        )
    } else if reason.starts_with("uncurated") {
        // Above the floor, no curated entry, so nothing could be tested.
        ("floor", format!("no curated entry &mdash; {}", esc(reason)))
    } else if reason == "below_floor_uncurated" {
        (
            "floor",
            "no curated entry, and below the abundance floor".to_string(),
        )
    } else if reason == "below_floor" {
        (
            "floor",
            format!(
                "below the abundance floor &mdash; {} against {:.0} PSMs",
                thousands(n.count),
                rec.floor_psms
            ),
        )
    } else {
        // Unreachable for the strings the emitter writes today. Kept so a new
        // reason string shows up as itself rather than as a blank cell.
        ("&mdash;", esc(reason))
    };

    let label = match (&n.label, n.name_source.as_deref()) {
        (Some(l), Some("unimod")) => {
            format!("{} <span class=\"mut\">named from Unimod</span>", esc(l))
        }
        (Some(l), _) => esc(l),
        (None, _) => "<span class=\"mut\">un-curated</span>".to_string(),
    };

    // The SAME merged residue/position cell the Fixed and Variable tables use.
    // A `failed_residue_test` row is the one row type whose whole point is the
    // residue, and it used to render an em dash.
    //
    // The dash survives ONLY for a row where nothing is known — an un-named
    // satellite, say. `acceptor_cell` is not called there, because with both
    // fields empty it would print "unspecific", and "we tested nothing" and "the
    // acceptor is unspecific" are different statements.
    let acceptor = if n.sites.is_empty() && n.position.is_empty() {
        "<span class=\"mut\">&mdash;</span>".to_string()
    } else {
        acceptor_cell(&n.sites, &n.position)
    };

    format!(
        "<tr><td class=\"num\">{delta:+.4}</td><td>{label}</td>\
         <td>{acceptor}</td><td class=\"num\">{count}</td>\
         <td class=\"num\">{pct:.2}%</td><td>{decided}</td><td class=\"why\">{why}</td></tr>",
        delta = n.delta_mass,
        count = thousands(n.count),
        pct = n.count_pct,
    )
}

/// The recommendation block: Fixed, Variable, and what did not make the cut.
///
/// Returns the empty string when the curated list could not be loaded, which is
/// a configuration problem rather than an empty result.
fn recommendations_section(report: &ReconReport) -> String {
    let Some(rec) = report.recommendations.as_ref() else {
        return String::new();
    };

    let head = "<tr><th class=\"num\">Delta mass</th><th>Modification</th><th>Residue</th>\
                <th class=\"num\">Count</th><th class=\"num\">%</th><th>Decided by</th>\
                <th>Evidence</th></tr>";
    let head_why = head.replace("<th>Evidence</th>", "<th>Why not</th>");

    let table = |id: &str, header: &str, rows: String| -> String {
        format!("<div class=\"scroll\"><table id=\"{id}\">{header}{rows}</table></div>")
    };

    let fixed_rows: String = rec.fixed.iter().map(rec_row).collect();
    let variable_rows: String = rec.variable.iter().map(rec_row).collect();

    // What belongs in the cut table, per the mockup's CORRECTED 2026-09-01 note:
    // the floor is not gate 1, so this holds anything that was actually TESTED
    // and failed, plus un-curated and satellite deltas that cleared the floor.
    // Untested things below the floor are simply not shown.
    //
    // `notable_unannotated` is joined in because it is, by construction, the
    // above-floor un-curated class. At schema 2.0.0 — which the mockup read —
    // that list was empty on every test file and the same peaks appeared in
    // `not_recommended`.
    let mut cut: Vec<&NotRecommended> = rec
        .not_recommended
        .iter()
        .chain(rec.notable_unannotated.iter())
        .filter(|n| n.reason == "failed_residue_test" || (n.count as f64) >= rec.floor_psms)
        .collect();
    cut.sort_by_key(|b| std::cmp::Reverse(b.count));
    let cut_rows: String = cut.iter().map(|n| cut_row(n, rec)).collect();

    let n_peaks = report.mod_discovery.peaks.len();

    format!(
        r##"
<section><div class="hd"><h2>Recommended search modifications</h2>
<button onclick="reconCsv()">Copy as CSV</button></div><div class="bd">
<div class="tbl-title">Fixed</div>
{fixed}
<div class="tbl-title">Variable</div>
{variable}
<div class="tbl-title">Detected but did not make the cut</div>
{cut}

<details id="csv-box"><summary>CSV of the three tables above</summary><div class="bd">
<p class="why" id="csv-status">Press <b>Copy as CSV</b> above to fill this box.</p>
<textarea id="csv-out" rows="10" spellcheck="false"
 aria-label="CSV of the three tables"></textarea>
</div></details>

<div class="note"><b>{n_peaks} distinct delta masses were detected and assessed in this file.</b>
Each one walked this chart from the top and stopped at the first answer that applied.

<div class="flow">
<div class="step"><b>1. Is it an isotope satellite of a larger peak?</b>
  <div class="yes"><b>Yes</b> &rarr; shown as a satellite. Never recommended.</div>
  <div class="no"><b>No</b> &rarr; go to 2.</div></div>

<div class="step"><b>2. Is there a curated entry for this delta mass?</b>
  <div class="no"><b>No</b> &rarr; judged on <b>abundance</b> against the floor of <b>{floor:.0} PSMs</b>
    ({floor_pct:.0}% of the tallest peak).<br>
    Above the floor: listed as an un-curated delta mass. Never recommended.<br>
    Below the floor: not shown at all.</div>
  <div class="yes"><b>Yes</b> &rarr; go to 3.</div></div>

<div class="step"><b>3. Does the curated entry name specific acceptor residues?</b>
  <div class="no"><b>No</b> &rarr; judged on <b>abundance</b> against the same floor.</div>
  <div class="yes"><b>Yes</b> &rarr; judged on <b>statistics</b>. Recommended only if
    <b>OR &ge; {or_min:.1}</b> AND <b>q &le; {q_max}</b>.<br>
    Otherwise it appears in &ldquo;did not make the cut&rdquo; with both numbers.</div></div>
</div>

<b>Decided by</b> tells you which route each row took.</div>

<div class="note"><b>These counts are lower than your next search will report, and that is
expected.</b> This is one open search: each spectrum is assigned a single delta mass, so occupancy is
split across every form a peptide takes. <b>Use the ranking, not the magnitude.</b> A targeted search
with these modifications set will report higher numbers for the same chemistry.</div>

<p class="why">Modification names and acceptor sites come from: {source}</p>
</div></section>
"##,
        fixed = if rec.fixed.is_empty() {
            "<p class=\"why\">None on this file.</p>".to_string()
        } else {
            table("t1", head, fixed_rows)
        },
        variable = if rec.variable.is_empty() {
            "<p class=\"why\">None on this file.</p>".to_string()
        } else {
            table("t2", head, variable_rows)
        },
        cut = if cut.is_empty() {
            "<p class=\"why\">None.</p>".to_string()
        } else {
            table("t3", &head_why, cut_rows)
        },
        n_peaks = thousands(n_peaks),
        or_min = rec.odds_ratio_min,
        q_max = rec.q_max,
        floor = rec.floor_psms,
        floor_pct = rec.floor_pct_of_top,
        source = esc(&rec.annotation_source),
    )
}

/// The Digestion section.
///
/// ⚠ **THE MOCKUP READS ONLY PASS-2 VALUES HERE**, from `<output>_pass2.json`.
/// `ReconReport` does not and must not carry them (see `Pass2Report`'s own doc),
/// so Pass 2 is passed in beside the report. `recon analyze` never produces one,
/// and `recon run --no-pass2` produces none either. When it is absent the
/// section says so rather than showing zeros or vanishing.
fn digestion_section(pass2: Option<&Pass2Report>) -> String {
    let Some(p) = pass2 else {
        return r#"
<section><div class="hd"><h2>Digestion</h2></div><div class="bd">
<p class="why">The digestion figures are measured by the semi-enzymatic second search, which did not
run for this report. Nothing about cleavage completeness or ragged termini can be read from the
first search alone: it is a fully enzymatic search and cannot generate a ragged peptide.</p>
</div></section>
"#
        .to_string();
    };

    let ratio = match p.terminus.n_c_ratio {
        Some(r) => format!("{r:.2}"),
        // No C-ragged peptides. A fabricated 0 or an infinity would both read as
        // a measurement.
        None => "<span class=\"mut\">not computed</span>".to_string(),
    };

    format!(
        r#"
<section><div class="hd"><h2>Digestion</h2></div><div class="bd"><div class="stats">
  <div class="stat"><span>Missed cleavage</span><b>{mc:.2}%</b></div>
  <div class="stat"><span>Ragged N</span><b>{rn:.2}%</b></div>
  <div class="stat"><span>Ragged C</span><b>{rc:.2}%</b></div>
  <div class="stat"><span>N : C ratio</span><b>{ratio}</b></div>
</div></div></section>
"#,
        mc = p.composition.missed_cleavage.pct,
        rn = p.composition.ragged_n.pct,
        rc = p.composition.ragged_c.pct,
    )
}

/// Stylesheet for the report. Kept out of `format!` so the CSS braces need no
/// doubling.
const REPORT_CSS: &str = r##"<style>
:root{--fg:#1a1a1a;--mut:#767676;--line:#e3e3e3;--card:#fafafa;--acc:#0b5fa5;--warn:#a35b00}
body{margin:0;font:14px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;color:var(--fg);background:#fff}
.wrap{max-width:1000px;margin:0 auto;padding:24px}
h1{font-size:20px;margin:0 0 4px} h2{font-size:15px;margin:0}
.sub{color:var(--mut);font-size:13px;margin-bottom:18px}
.meta{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:10px 20px;background:var(--card);border:1px solid var(--line);border-radius:6px;padding:14px 16px;margin-bottom:22px}
.meta div span{display:block;color:var(--mut);font-size:11px;text-transform:uppercase;letter-spacing:.05em}
.meta div b{font-weight:600;font-size:13px;word-break:break-all}
section{border:1px solid var(--line);border-radius:6px;margin-bottom:16px;overflow:hidden}
.hd{background:var(--card);padding:10px 14px;border-bottom:1px solid var(--line);display:flex;justify-content:space-between;align-items:center;gap:12px}
.bd{padding:14px}
.stats{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:14px}
.stat b{display:block;font-size:19px} .stat span{color:var(--mut);font-size:11px;text-transform:uppercase;letter-spacing:.04em}
table{width:100%;border-collapse:collapse;font-size:13px}
th{text-align:left;font-size:11px;text-transform:uppercase;letter-spacing:.04em;color:var(--mut);border-bottom:1px solid var(--line);padding:6px 8px}
td{padding:6px 8px;border-bottom:1px solid #f0f0f0;vertical-align:top}
.num{text-align:right;font-variant-numeric:tabular-nums;white-space:nowrap}
.mut{color:var(--mut);font-weight:400}
.tbl-title{font-size:12px;font-weight:600;text-transform:uppercase;letter-spacing:.05em;color:var(--acc);margin:16px 0 4px}
.tbl-title:first-child{margin-top:0}
.why{color:var(--mut)}
/* Wide tables scroll inside their own box; the page never scrolls sideways. */
.scroll{overflow-x:auto}
.note{background:#fbfbfb;border-left:3px solid var(--line);padding:10px 12px;font-size:12.5px;color:#555;margin-top:14px}
/* The routing explainer reads as a decision chart, not as prose. */
.flow{margin:10px 0}
.step{border:1px solid var(--line);border-left:3px solid var(--acc);border-radius:4px;background:#fff;padding:8px 10px;margin-bottom:6px}
.step+.step{margin-top:0}
.step>b{display:block;margin-bottom:5px;color:var(--fg)}
.step .yes,.step .no{padding:3px 0 3px 12px;border-left:2px solid var(--line);margin-left:2px}
.step .yes+.no,.step .no+.yes{margin-top:3px}
details{border-top:1px solid var(--line);margin-top:14px}
details summary{cursor:pointer;padding:9px 14px;font-size:13px;background:var(--card)}
textarea{width:100%;box-sizing:border-box;font:12px/1.4 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
footer{margin-top:26px;border-top:2px solid var(--fg);padding-top:14px;font-size:12.5px;color:var(--mut)}
footer a{color:var(--acc)}
button{font:inherit;font-size:12px;border:1px solid var(--line);background:#fff;border-radius:4px;padding:4px 10px;cursor:pointer}
@media print{.wrap{max-width:none} button{display:none} section{break-inside:avoid}}
</style>"##;

/// CSV export.
///
/// ⚠ The mockup called `navigator.clipboard.writeText` and nothing else. That
/// call REJECTS, or is absent entirely, on a `file://` page in several browsers
/// — and this report is always opened from disk — so the mockup's `alert`
/// claimed success on a copy that never happened. The textarea below is the path
/// that always works offline; the clipboard write is attempted as a bonus and
/// its outcome is reported either way.
const REPORT_JS: &str = r##"<script>
function reconCsv(){
  const rows=[];
  for(const id of ['t1','t2','t3']){
    const t=document.getElementById(id);
    if(!t){continue;}
    for(const r of t.rows){
      rows.push([...r.cells].map(c=>'"'+c.innerText.replace(/"/g,'""')+'"').join(','));
    }
    rows.push('');
  }
  const text=rows.join('\n');
  const box=document.getElementById('csv-box');
  const out=document.getElementById('csv-out');
  const st=document.getElementById('csv-status');
  out.value=text;
  box.open=true;
  out.focus();
  out.select();
  if(navigator.clipboard&&navigator.clipboard.writeText){
    navigator.clipboard.writeText(text).then(
      function(){st.textContent='Copied to the clipboard. It is also in the box below.';},
      function(){st.textContent='The browser blocked the clipboard. Select the text below and copy it.';}
    );
  }else{
    st.textContent='This browser gives a local page no clipboard. Select the text below and copy it.';
  }
}
</script>"##;

/// Render the HTML report.
///
/// Layout is `_dev/testing/scripts/report_layout_mockup.py`, which IS the agreed
/// design. Section order: header, detectors, mass accuracy, contamination,
/// glycopeptides, digestion, recommended modifications, footer. Signal Fate, the
/// Modification Landscape table and the Alkylation section are deliberately
/// gone; they stay in the JSON and in the console summary.
///
/// `pass2` supplies the Digestion section and is `None` for `recon analyze` and
/// for `recon run --no-pass2`.
pub fn generate_html_report(report: &ReconReport, pass2: Option<&Pass2Report>) -> String {
    let file_name = report
        .input
        .mzml_file
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&report.input.mzml_file)
        .to_string();

    // `input.fasta_file` is the record of what was passed (schema 3.1.0). The
    // `protein_context` copy is the fallback, so a 3.0.0 report still renders.
    let fasta = report
        .input
        .fasta_file
        .as_deref()
        .or_else(|| {
            report
                .recommendations
                .as_ref()
                .and_then(|r| r.protein_context.as_ref())
                .map(|c| c.fasta.as_str())
        })
        .map(esc)
        .unwrap_or_else(|| "<span class=\"mut\">not recorded</span>".to_string());

    // `recon analyze` has no `--enzyme` and cannot know the protease, so the
    // "not recorded" fallback stays. It is never a guess at trypsin.
    let enzyme = match report.input.enzyme.as_ref() {
        Some(e) => format!("<b>{}</b>", esc(&e.one_line())),
        None => "<b class=\"mut\">not recorded in this report</b>".to_string(),
    };

    let runtime = match report.runtime_seconds {
        Some(s) => format!("{s:.1} s"),
        None => "<span class=\"mut\">not recorded</span>".to_string(),
    };

    // Detectors.
    let detectors = match report.analyzers.as_ref() {
        Some(a) => format!(
            r#"  <div class="stat"><span>Instrument</span><b style="font-size:14px">{model}</b></div>
  <div class="stat"><span>MS1 analyzer</span><b style="font-size:14px">{ms1}</b></div>
  <div class="stat"><span>MS2 analyzer</span><b style="font-size:14px">{ms2}</b></div>
  <div class="stat"><span>MS2 scans</span><b>{scans}</b></div>"#,
            model = a
                .instrument_model
                .as_deref()
                .map(esc)
                .unwrap_or_else(|| "<span class=\"mut\">not declared</span>".to_string()),
            ms1 = a
                .ms1_analyzers
                .first()
                .map(|s| esc(s))
                .unwrap_or_else(|| "<span class=\"mut\">none seen</span>".to_string()),
            ms2 = a
                .ms2_analyzers
                .first()
                .map(|s| esc(s))
                .unwrap_or_else(|| "<span class=\"mut\">none seen</span>".to_string()),
            scans = thousands(report.input.ms2_spectra),
        ),
        None => format!(
            r#"  <div class="stat"><span>Instrument</span><b style="font-size:14px" class="mut">not detected</b></div>
  <div class="stat"><span>MS2 scans</span><b>{scans}</b></div>"#,
            scans = thousands(report.input.ms2_spectra),
        ),
    };

    // Mass accuracy. The MEASURED values first, then ONE recommendation stat
    // carrying both tolerances — the mockup's arrangement.
    //
    // ⚠ THE MS2 RECOMMENDATION IS LADDERED ONLY WHEN THE MS2 ANALYZER IS
    // PPM-BASED. A ppm rung handed to an ion trap or quadrupole at unit
    // resolution is meaningless; that class gets DALTONS. The routing is
    // `ms2_recommendation_unit`, which reads the analyzer class recon already
    // detected, and the arithmetic is `calibration::ms2_user_recommendation`.
    // Neither re-detects anything.
    let mass_accuracy = match report.ms1_calibration.as_ref() {
        Some(cal) => {
            let ms1_rec = match cal.user_recommendation_tolerance_ppm {
                Some(t) => format!("{t:.0}"),
                None => format!(
                    "{:+.2} to {:+.2} ppm",
                    cal.user_recommendation_low_ppm, cal.user_recommendation_high_ppm
                ),
            };
            let ms2_abs = match cal.ms2_median_abs_ppm {
                Some(v) => format!("{v:.2} ppm"),
                None => "<span class=\"mut\">not measured</span>".to_string(),
            };
            let ms2_tol = match (cal.ms2_tolerance_low_ppm, cal.ms2_tolerance_high_ppm) {
                (Some(lo), Some(hi)) => format!("{lo:+.2} to {hi:+.2} ppm"),
                _ => "<span class=\"mut\">not measured</span>".to_string(),
            };
            // Both rungs on one line when both are ppm, which is the common case
            // and the string the mockup shows. When the MS2 analyzer needs
            // daltons the two units are spelled out, because "10 / 0.6" would
            // read as two ppm numbers.
            let unit = ms2_recommendation_unit(report.analyzers.as_ref());
            let ms2_rec = cal
                .ms2_tolerance_high_ppm
                .map(|half| crate::calibration::ms2_user_recommendation(half, unit));
            let combined = match (cal.user_recommendation_tolerance_ppm, ms2_rec) {
                (Some(_), Some(crate::mzml::FragmentTolerance::Ppm(p))) => {
                    format!("{ms1_rec} / {p:.0} ppm")
                }
                // ⚠ ONE decimal, not three. The Da recommendation is quantized
                // to a tenth of a Dalton by `ms2_user_recommendation`, so `.1`
                // is LOSSLESS here — and `.3` actively lied, rendering 0.6 as
                // "0.600" and claiming precision the number does not carry.
                // The guarantee this relies on is asserted by
                // `calibration::tests::a_da_analyzer_is_quantized_to_a_tenth_of_a_dalton`.
                (Some(_), Some(crate::mzml::FragmentTolerance::Da(d))) => {
                    format!("{ms1_rec} ppm / {d:.1} Da")
                }
                (Some(_), None) => {
                    format!("{ms1_rec} ppm / <span class=\"mut\">not measured</span>")
                }
                (None, Some(crate::mzml::FragmentTolerance::Ppm(p))) => {
                    format!("{ms1_rec} / {p:.0} ppm")
                }
                // One decimal, for the same reason as the arm above.
                (None, Some(crate::mzml::FragmentTolerance::Da(d))) => {
                    format!("{ms1_rec} / {d:.1} Da")
                }
                (None, None) => ms1_rec.clone(),
            };
            format!(
                r#"  <div class="stat"><span>Precursor / MS1 &mdash; signed median</span><b>{bias:+.2} ppm</b></div>
  <div class="stat"><span>Fragment / MS2 &mdash; |median|</span><b>{ms2_abs}</b></div>
  <div class="stat"><span>Measured MS2 tolerance</span><b style="font-size:15px">{ms2_tol}</b></div>
  <div class="stat"><span>Recommended MS1 / MS2</span><b>{combined}</b></div>"#,
                bias = cal.bias_ppm,
            )
        }
        None => "  <div class=\"stat\"><span>Mass accuracy</span>\
                 <b style=\"font-size:14px\" class=\"mut\">no clean subset &mdash; not measured</b>\
                 </div>"
            .to_string(),
    };

    let ladder_note = match report
        .ms1_calibration
        .as_ref()
        .and_then(|c| c.user_recommendation_exceeds_ladder)
    {
        Some(true) => "<p class=\"why\" style=\"margin-top:10px\">The measured MS1 requirement is \
             wider than the largest recommended tolerance, so the value above is known to be too \
             narrow for this file. The instrument looks mis-calibrated. Recalibrate it rather than \
             widen the search.</p>"
            .to_string(),
        _ => String::new(),
    };

    // Contamination.
    let polymer_rows: String = report
        .polymer
        .top_polymers
        .iter()
        .take(5)
        .map(|p| {
            format!(
                "<tr><td>{}</td><td class=\"num\">{:.3}%</td></tr>",
                esc(&p.name),
                p.pct_tic
            )
        })
        .collect();

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{file_name} &mdash; recon</title>
{css}
</head>
<body>
<div class="wrap">

<h1>{file_name}</h1>
<div class="sub">Proteomics reconnaissance report</div>

<div class="meta">
  <div><span>FASTA</span><b>{fasta}</b></div>
  <div><span>Enzyme</span>{enzyme}</div>
  <div><span>Generated</span><b>{generated_at} UTC</b></div>
  <div><span>recon</span><b>v{tool_version}</b></div>
  <div><span>Total runtime</span><b>{runtime}</b></div>
</div>

<section><div class="hd"><h2>Detectors</h2></div><div class="bd"><div class="stats">
{detectors}
</div></div></section>

<section><div class="hd"><h2>Mass accuracy</h2></div><div class="bd"><div class="stats">
{mass_accuracy}
</div>{ladder_note}</div></section>

<section><div class="hd"><h2>Contamination</h2></div><div class="bd">
<div class="stats">
  <div class="stat"><span>Polymer % of TIC</span><b>{polymer_pct:.2}%</b></div>
  <div class="stat"><span>Level</span><b style="font-size:14px">{polymer_level}</b></div>
</div>
<div class="scroll"><table style="margin-top:12px"><tr><th>Polymer</th><th class="num">% TIC</th></tr>
{polymer_rows}
</table></div></div></section>

<section><div class="hd"><h2>Glycopeptides</h2></div><div class="bd"><div class="stats">
  <div class="stat"><span>Candidate spectra</span><b>{glyco_count}</b></div>
  <div class="stat"><span>% of MS2</span><b>{glyco_pct:.2}%</b></div>
</div></div></section>
{digestion}{recommendations}
<footer>
<b>sageRecon</b> v{tool_version} &middot; using Sage v{sage_version} &middot; maintained by
Benjamin Neely (NIST), <a href="mailto:benjamin.neely@nist.gov">benjamin.neely@nist.gov</a><br>
<!-- Provenance line KEPT, against the mockup, which dropped both. A report that
     cannot say which code state made it is not traceable. -->
Built from commit {git_commit} &middot; report schema v{schema_version}
<details><summary>Acknowledgements</summary><div class="bd">
<b>Sage</b> &mdash; <a href="https://github.com/lazear/sage">github.com/lazear/sage</a> &mdash; Lazear, M.R.
<i>J. Proteome Res.</i> 2023, 22(11), 3652&ndash;3659. doi:10.1021/acs.jproteome.3c00486<br>
<b>MetaMorpheus</b> (curated modification list) &mdash;
<a href="https://github.com/smith-chem-wisc/metamorpheus">github.com/smith-chem-wisc/metamorpheus</a> &mdash;
Solntsev, Shortreed, Frey, Smith. <i>J. Proteome Res.</i> 2018, 17(5), 1844&ndash;1851. doi:10.1021/acs.jproteome.7b00873<br>
<b>mzSniffer</b> (polymer detection) &mdash; <a href="https://github.com/wfondrie/mzsniffer">github.com/wfondrie/mzsniffer</a> &mdash; no publication.<br>
<b>Pyteomics</b> (code ideas) &mdash; <a href="https://github.com/levitsky/pyteomics">github.com/levitsky/pyteomics</a> &mdash;
Levitsky, Klein, Ivanov, Gorshkov. <i>J. Proteome Res.</i> 2019, 18(2), 709&ndash;714. doi:10.1021/acs.jproteome.8b00717<br>
<b>Unimod</b> &mdash; <a href="https://www.unimod.org/">unimod.org</a> &mdash; Design Science License.
</div></details>
<details><summary>Licence &mdash; NIST</summary><div class="bd">
NIST Software Licensing Statement. The full text is in <code>LICENSE.md</code>, shipped beside this tool.
</div></details>
<details><summary>Licence &mdash; third party</summary><div class="bd">
Sage (MIT), mzSniffer (Apache 2.0), MetaMorpheus (MIT), Unimod (Design Science License).
The full texts are in <code>THIRD_PARTY_LICENSES.md</code>, shipped in the release archive.
</div></details>
</footer>
</div>
{js}
</body>
</html>
"##,
        css = REPORT_CSS,
        js = REPORT_JS,
        generated_at = report.generated_at.format("%Y-%m-%d %H:%M:%S"),
        tool_version = esc(&report.tool_version),
        sage_version = crate::sage_runner::SAGE_VERSION,
        git_commit = esc(&report.git_commit),
        schema_version = esc(&report.schema_version),
        polymer_pct = report.polymer.total_pct_tic,
        polymer_level = esc(&report.polymer.contamination_level),
        glyco_count = thousands(report.oxonium.glycopeptide_candidates),
        glyco_pct = report.oxonium.glycopeptide_pct,
        file_name = esc(&file_name),
        digestion = digestion_section(pass2),
        recommendations = recommendations_section(report),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sage_results::Psm;

    fn make_test_psm(peptide: &str, delta: f64) -> Psm {
        Psm {
            scannr: 1,
            rank: 1,
            peptide: peptide.to_string(),
            proteins: "PROTEIN".to_string(),
            expmass: 1000.0 + delta,
            calcmass: 1000.0,
            isotope_error: 0,
            delta_mass: delta,
            delta_mass_corrected: delta,
            hyperscore: 40.0,
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

    #[test]
    fn test_alkylation_check_complete() {
        let psms = vec![
            make_test_psm("PEPTCIDE", 0.0),  // Cys, alkylated (delta ~0)
            make_test_psm("PEPTCIDE", 0.01), // Cys, alkylated
            make_test_psm("PEPTIDE", 0.0),   // No Cys
        ];

        let check = compute_alkylation_check(&psms);

        assert_eq!(check.cys_psm_count, 2);
        assert_eq!(check.unalkylated_count, 0);
        assert!(check.status.contains("complete"));
    }

    #[test]
    fn test_alkylation_check_incomplete() {
        let psms = vec![
            make_test_psm("PEPTCIDE", 0.0),    // Cys, alkylated
            make_test_psm("PEPTCIDE", -57.02), // Cys, unalkylated!
            make_test_psm("PEPTIDE", 0.0),     // No Cys
        ];

        let check = compute_alkylation_check(&psms);

        assert_eq!(check.cys_psm_count, 2);
        assert_eq!(check.unalkylated_count, 1);
        assert!((check.unalkylated_pct - 50.0).abs() < 0.1);
    }

    /// THE IDENTITY-ONLY LOCK, asserted rather than described.
    ///
    /// `input.enzyme` must serialise EXACTLY four keys. A tuning field arriving
    /// here — `missed_cleavages`, `min_len`, `max_len`, `semi_enzymatic` — would
    /// read as part of the enzyme's definition, and Pass 1 and Pass 2 set those
    /// differently on purpose. The residues must be LETTERS, not the byte
    /// numbers `Enzyme`'s `Vec<u8>` would otherwise write.
    #[test]
    fn the_recorded_enzyme_is_identity_only() {
        let info = EnzymeInfo::from(&crate::enzyme::trypsin_for_tests());
        let v = serde_json::to_value(&info).unwrap();
        let obj = v.as_object().unwrap();

        let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
        keys.sort();
        assert_eq!(keys, vec!["c_terminal", "cleave_at", "name", "restrict"]);

        assert_eq!(obj["name"], "trypsin");
        assert_eq!(obj["cleave_at"], "KR");
        assert_eq!(obj["restrict"], "P");
        assert_eq!(obj["c_terminal"], true);
    }

    /// Both new fields are absent, not null, when the run did not have them.
    /// That is what keeps the addition backward compatible for a 3.0.0 reader.
    #[test]
    fn the_new_input_fields_are_omitted_when_absent() {
        let input = InputInfo {
            mzml_file: "a.mzML".to_string(),
            sage_tsv: "b.tsv".to_string(),
            unimod_file: "unimod.xml".to_string(),
            ms1_spectra: 1,
            ms2_spectra: 2,
            fasta_file: None,
            enzyme: None,
        };
        let v = serde_json::to_value(&input).unwrap();
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("fasta_file"));
        assert!(!obj.contains_key("enzyme"));
    }

    /// The meta-grid wording, as the layout mockup spells it.
    #[test]
    fn the_enzyme_one_liner_matches_the_mockup() {
        let trypsin = EnzymeInfo::from(&crate::enzyme::trypsin_for_tests());
        assert_eq!(trypsin.one_line(), "trypsin — KR, not before P, C-term");

        // No restriction: the clause disappears rather than reading "not before".
        let lys_n = EnzymeInfo::from(&crate::enzyme::parse("lys-n").unwrap());
        assert_eq!(lys_n.one_line(), "lys-n — K, N-term");
    }

    fn analyzer_report(ms2: &[&str]) -> AnalyzerReport {
        AnalyzerReport {
            instrument_model: None,
            ms1_analyzers: vec![],
            ms2_analyzers: ms2.iter().map(|s| s.to_string()).collect(),
            ms2_switched: ms2.len() > 1,
            pass1_fragment_tol: String::new(),
            basis: "Detected".to_string(),
            assumed: false,
            explanation: String::new(),
        }
    }

    /// The label lookup is the only join between `AnalyzerReport`'s strings and
    /// the class enum. If a variant is added upstream and not listed here, the
    /// MS2 recommendation would silently fall back to ppm for it.
    #[test]
    fn every_analyzer_label_maps_back_to_its_class() {
        for class in ALL_ANALYZER_CLASSES {
            let found = ALL_ANALYZER_CLASSES
                .iter()
                .find(|c| c.label() == class.label());
            assert_eq!(
                found.map(|c| c.label()),
                Some(class.label()),
                "{class:?} did not round-trip through its label"
            );
        }
        // Distinct labels, or the lookup would resolve to the wrong class.
        let mut labels: Vec<&str> = ALL_ANALYZER_CLASSES.iter().map(|c| c.label()).collect();
        labels.sort_unstable();
        let n = labels.len();
        labels.dedup();
        assert_eq!(labels.len(), n, "two analyzer classes share one label");
    }

    /// ⚠ **NO COMMITTED TEST FILE REACHES THE DA BRANCH** — all four are
    /// Orbitraps — so the trap routing is asserted here and nowhere else.
    #[test]
    fn an_ion_trap_never_gets_a_ppm_recommendation() {
        use crate::mzml::{AnalyzerClass, FragmentTolerance};

        let trap = AnalyzerClass::IonTrap.label();
        assert!(matches!(
            ms2_recommendation_unit(Some(&analyzer_report(&[trap]))),
            FragmentTolerance::Da(_)
        ));
        // A run that switches detectors falls to Da if ANY analyzer needs it.
        assert!(matches!(
            ms2_recommendation_unit(Some(&analyzer_report(&[
                AnalyzerClass::Orbitrap.label(),
                trap
            ]))),
            FragmentTolerance::Da(_)
        ));
        // The ppm classes stay ppm, and so does "no analyzer block at all".
        for label in [
            AnalyzerClass::Orbitrap.label(),
            AnalyzerClass::AstralTof.label(),
            AnalyzerClass::LegacyTof.label(),
            AnalyzerClass::Unclassified.label(),
        ] {
            assert!(
                matches!(
                    ms2_recommendation_unit(Some(&analyzer_report(&[label]))),
                    FragmentTolerance::Ppm(_)
                ),
                "{label} must stay on the ppm ladder"
            );
        }
        assert!(matches!(
            ms2_recommendation_unit(None),
            FragmentTolerance::Ppm(_)
        ));

        // End to end: the same measured spread that gives serum a 10 ppm rung
        // must give a trap a value in daltons.
        let measured = 1.6670475500000004_f64;
        let trap_unit = ms2_recommendation_unit(Some(&analyzer_report(&[trap])));
        match crate::calibration::ms2_user_recommendation(measured, trap_unit) {
            FragmentTolerance::Da(_) => {}
            FragmentTolerance::Ppm(p) => panic!("a trap was handed {p} ppm"),
        }
        let orbi =
            ms2_recommendation_unit(Some(&analyzer_report(&[AnalyzerClass::Orbitrap.label()])));
        assert_eq!(
            crate::calibration::ms2_user_recommendation(measured, orbi),
            FragmentTolerance::Ppm(10.0)
        );
    }

    fn cut(reason: &str, sites: &str, position: &str) -> NotRecommended {
        NotRecommended {
            delta_mass: 114.0457,
            count: 141,
            count_pct: 0.9021,
            reason: reason.to_string(),
            label: Some("GG (Ubiquitination Site)".to_string()),
            name_source: Some("curated".to_string()),
            sites: sites.to_string(),
            position: position.to_string(),
            odds_ratio: Some(1.2915),
            q_value: Some(0.1887),
        }
    }

    fn recs() -> ModRecommendations {
        ModRecommendations {
            fixed: vec![],
            variable: vec![],
            not_recommended: vec![],
            notable_unannotated: vec![],
            carpet_margin_psms: 1.0,
            carpet_tallest_psms: 0,
            floor_psms: 224.0,
            floor_pct_of_top: 20.0,
            odds_ratio_min: crate::tier_assignment::OR_MIN,
            q_max: crate::tier_assignment::Q_MAX,
            annotation_source: "test".to_string(),
            protein_context: None,
            caveats: vec![],
        }
    }

    /// Table 3 used to render an em dash in BOTH the Residue and the % column
    /// while the count was populated.
    #[test]
    fn the_cut_table_shows_the_residue_and_the_percentage() {
        let r = recs();

        // A tested-and-failed row shows the residues the test ran against, in
        // the same merged cell the other two tables use.
        let row = cut_row(&cut("failed_residue_test", "K", "Anywhere."), &r);
        assert!(row.contains("<strong>K</strong>"), "{row}");
        assert!(row.contains("0.90%"), "{row}");
        // "Anywhere" carries no position note.
        assert!(!row.contains("&middot;"), "{row}");
        assert!(!row.contains(" · "), "{row}");

        // A position that is NOT "Anywhere" is shown, in grey, after the residue.
        let row = cut_row(&cut("below_floor", "E", "Peptide N-terminal."), &r);
        assert!(row.contains("<strong>E</strong>"), "{row}");
        assert!(row.contains("peptide N-term"), "{row}");

        // ⚠ The "any residue" / "unspecific" distinction is a MEASUREMENT and
        // must survive into this table too.
        let row = cut_row(&cut("below_floor", "", "Protein N-terminal."), &r);
        assert!(row.contains("any residue"), "{row}");
        assert!(!row.contains("unspecific"), "{row}");
        let row = cut_row(&cut("below_floor", "", "Anywhere."), &r);
        assert!(row.contains("unspecific"), "{row}");

        // Nothing known at all still renders a dash, NOT "unspecific": no
        // acceptor was tested and none may be claimed.
        let mut bare = cut("satellite (+1 C13 of +57.0238)", "", "");
        bare.label = None;
        bare.name_source = None;
        let row = cut_row(&bare, &r);
        assert!(row.contains("&mdash;"), "{row}");
        assert!(!row.contains("unspecific"), "{row}");
        assert!(!row.contains("any residue"), "{row}");
        // The percentage is still shown on that row.
        assert!(row.contains("0.90%"), "{row}");
    }

    /// A satellite row must not claim the floor decided it.
    ///
    /// The JSON carries `decided_by: null` for satellites, and the flow chart
    /// printed below the table says they stop at step 1. The HTML rendered
    /// "floor", contradicting both, on the same page. Found 2026-09-03 by
    /// reading the rendered report.
    ///
    /// ⚠ **This assertion existed before and DID NOT RUN.** It was written as a
    /// `mod` nested inside `cut_row`'s body, which the test harness cannot
    /// reach, so it never appeared in `cargo test` output — only as a
    /// `never used` warning. It also grepped this file's own source text
    /// through a fixed 400-byte window, the same brittle pattern that produced
    /// a wrong measurement in the Unimod work the same day. It is now a
    /// behavioural test on the RENDERED ROW, which is the artifact a reader
    /// actually sees.
    #[test]
    fn a_satellite_is_not_attributed_to_the_floor() {
        let r = recs();
        let row = cut_row(&cut("satellite (+1 C13 of +57.0238)", "", ""), &r);

        assert!(
            row.contains("<td>satellite</td>"),
            "a satellite must render \"satellite\" as its route: {row}"
        );
        assert!(
            !row.contains("<td>floor</td>"),
            "a satellite must never be attributed to the floor: {row}"
        );
        // The floor is not merely absent from the route cell — the row must not
        // cite a floor value anywhere, which is how the contradiction read.
        assert!(
            !row.contains("abundance floor"),
            "a satellite row must not mention the abundance floor: {row}"
        );

        // Control: a row that IS decided by the floor still says so, so the
        // assertion above cannot pass by the label having been removed for
        // everyone.
        let floored = cut_row(&cut("below_floor", "K", "Anywhere."), &r);
        assert!(floored.contains("<td>floor</td>"), "{floored}");
        assert!(floored.contains("abundance floor"), "{floored}");
    }

    /// The regression case, against the REAL pinned `unimod.xml` — not a
    /// synthetic fixture, which could not have caught this. The first attempt read
    /// `PeakAnnotation.sites` (i.e. `UnimodEntry::sites`), and that drops every
    /// `hidden="1"` specificity with no fallback. All ten of these entries are
    /// all-hidden in the pinned file, so all ten came back empty.
    #[test]
    fn all_hidden_unimod_entries_still_yield_their_acceptor_residues() {
        let db = crate::unimod::UnimodDb::from_embedded().expect("compiled-in Unimod");

        // (record_id, title, expected sites, expected position). Read off the
        // pinned XML. Keyed by RECORD ID, the same key the production path uses,
        // so the test pins the id -> entry mapping as well as the acceptor.
        let cases = [
            (893, "CarbamidomethylDTT", "C", "Anywhere."),
            (352, "Lys->Allysine", "K", "Anywhere."),
            (837, "Arg->Npo", "R", "Anywhere."),
            (2034, "Gly+O(2)", "H", "Anywhere."),
            (989, "Ammonium", "DE", "Anywhere."),
            (1910, "Cation:Al[III]", "DE", "Anywhere."),
            (953, "Cation:Ni[II]", "DE", "Anywhere."),
            (1903, "Xlink:SMCC[219]", "CK", "Anywhere."),
            (1972, "Unknown:210", "DE", "Anywhere."),
            // The one that already worked: a single NON-hidden specificity, and
            // the only one of the set whose position is not "Anywhere".
            (26, "Pyro-carbamidomethyl", "C", "Peptide N-terminal."),
        ];
        for (id, title, sites, position) in cases {
            let entry = db
                .get_by_id(id)
                .unwrap_or_else(|| panic!("record {id} is not in the pinned unimod.xml"));
            assert_eq!(entry.title, title, "record {id} is no longer {title}");
            assert_eq!(
                unimod_acceptor(entry),
                (sites.to_string(), position.to_string()),
                "{title}"
            );
            // The bug, stated as itself: `sites()` is empty for nine of these.
            // If upstream ever un-hides them this assertion is what says so.
            if title != "Pyro-carbamidomethyl" {
                assert!(
                    entry.sites().is_empty(),
                    "{title} is no longer all-hidden; re-read the fallback"
                );
            }
        }
    }

    /// Multi-letter Unimod sites are POSITIONS, never acceptors, and a position is
    /// claimed only when the residue-bearing specificities agree on one.
    #[test]
    fn unimod_terminal_pseudo_sites_never_become_residues() {
        use crate::unimod::{ModSpecificity, UnimodEntry};
        let spec = |site: &str, position: &str, hidden: bool| ModSpecificity {
            site: site.to_string(),
            position: position.to_string(),
            classification: "Artefact".to_string(),
            hidden,
        };
        let entry = |specs: Vec<ModSpecificity>| UnimodEntry {
            record_id: 1,
            title: "t".to_string(),
            full_name: "t".to_string(),
            mono_mass: 1.0,
            avge_mass: 1.0,
            composition: String::new(),
            specificities: specs,
        };

        // "C-term" is not an acceptor. D and E are, and they agree on Anywhere.
        assert_eq!(
            unimod_acceptor(&entry(vec![
                spec("E", "Anywhere", true),
                spec("D", "Anywhere", true),
                spec("C-term", "Any C-term", true),
            ])),
            ("DE".to_string(), "Anywhere.".to_string())
        );

        // Residues that DISAGREE on position keep their residues and claim no
        // position, rather than picking one.
        assert_eq!(
            unimod_acceptor(&entry(vec![
                spec("K", "Anywhere", false),
                spec("C", "Protein N-term", false),
            ])),
            ("CK".to_string(), String::new())
        );

        // ⚠ A terminal-only entry keeps the protein-terminal wording, because
        // `acceptor_cell` reads exactly that to say "any residue" instead of
        // "unspecific". Collapsing the two is what this preserves.
        let (sites, position) =
            unimod_acceptor(&entry(vec![spec("N-term", "Protein N-term", false)]));
        assert_eq!(sites, "");
        assert_eq!(position, "Protein N-terminal.");
        assert!(acceptor_cell(&sites, &position).contains("any residue"));

        // A peptide terminus with no residue IS unspecific, and says so.
        let (sites, position) = unimod_acceptor(&entry(vec![spec("N-term", "Any N-term", false)]));
        assert!(acceptor_cell(&sites, &position).contains("unspecific"));

        // A non-hidden specificity wins; the hidden one beside it is ignored.
        assert_eq!(
            unimod_acceptor(&entry(vec![
                spec("M", "Anywhere", false),
                spec("C", "Anywhere", true),
            ])),
            ("M".to_string(), "Anywhere.".to_string())
        );

        // An unrecognised position yields NO position, never a near-miss.
        assert_eq!(
            unimod_acceptor(&entry(vec![spec("K", "Somewhere else", false)])),
            ("K".to_string(), String::new())
        );
    }

    /// The same cell renderer serves all three tables, so a change to one cannot
    /// silently diverge in another.
    #[test]
    fn the_cut_table_reuses_the_recommendation_acceptor_cell() {
        let r = recs();
        let expected = acceptor_cell("K", "Anywhere.");
        assert!(cut_row(&cut("failed_residue_test", "K", "Anywhere."), &r).contains(&expected));
    }
}
