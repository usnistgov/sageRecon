// Auto-generated calibration module for recon-tool
// Implements MS1/MS2 mass error statistics and tolerance recommendations
// per _dev/reference-notes/recon-calibration-design-v2.md and temp-flowChart.md

#[derive(Debug, Clone, PartialEq)]
pub struct MassErrorStats {
    pub bias_ppm: f64,
    pub mad_ppm: f64,
    pub n_psms: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ms1UserRecommendation {
    pub bias_ppm: f64,
    pub spread_mad_ppm: f64,
    /// The QUANTIZED recommendation: the smallest rung of
    /// `MS1_TOLERANCE_LADDER_PPM` that is >= `|bias| + K*MAD`. This is the
    /// number a user should type into their own search. MS1 ONLY — a ppm ladder
    /// is meaningless for an ion-trap MS2.
    pub recommended_tolerance_ppm: f64,
    /// The UNQUANTIZED requirement, `|bias| + K*MAD`, reported alongside the
    /// rung so the reader can see how much headroom the rung actually has.
    pub measured_requirement_ppm: f64,
    /// True when `measured_requirement_ppm` exceeds the TOP rung, i.e. the
    /// ladder cannot cover this file. The recommendation is then the top rung
    /// and is KNOWN TO BE TOO NARROW — the containment guarantee below does not
    /// hold, and the report must say so rather than quietly emit 100.
    pub requirement_exceeds_ladder: bool,
    /// Symmetric +/- `recommended_tolerance_ppm`, kept so existing consumers of
    /// the low/high pair keep working. NOT the old asymmetric window.
    ///
    /// **CONTAINMENT GUARANTEE** (asserted in
    /// `symmetric_rung_always_contains_the_bias_centred_window`): whenever
    /// `requirement_exceeds_ladder` is false, `[-rung, +rung]` fully contains
    /// the bias-centred window `[bias - K*MAD, bias + K*MAD]`, because `|bias|`
    /// is folded into the requirement BEFORE quantizing. A symmetric
    /// recommendation therefore never clips a biased instrument — which is what
    /// the superseded asymmetric window existed to prevent.
    pub low_ppm: f64,
    pub high_ppm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ms1Pass2Window {
    pub low_ppm: f64,
    pub high_ppm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ms2Tolerance {
    pub low_ppm: f64,
    pub high_ppm: f64,
    pub median_ppm: f64,
    pub mad_ppm: f64,
    pub tail_95_ppm: f64,
}

#[derive(Debug, Clone)]
pub struct PsmSummary {
    pub q_value: f64,
    pub rank: u8,
    pub corrected_delta_da: f64,
    /// SIGNED precursor mass error, in ppm. RECONSTRUCTED from the mass
    /// columns by `from_psm` — it is NOT a copy of Sage's `precursor_ppm`.
    /// The field is named `_signed` so the distinction cannot be lost: the
    /// v0.14.x reported `precursor_ppm` as `|error|`, so copying
    /// that column made `bias_ppm` a median of absolute errors that could
    /// never be negative. See NOTES "BUG — MS1/MS2 bias was a median of
    /// |error|".
    ///
    /// ⚠ **DO NOT REPLACE THIS WITH SAGE'S COLUMN.** This comment used to
    /// instruct the next session to remove the reconstruction once v0.15 made
    /// `precursor_ppm` signed. v0.15 IS now the pin, the instruction was never
    /// carried out, and it was WRONG. Corrected in place 2026-09-02.
    ///
    /// The two are not the same quantity. This divides by `calcmass`; Sage
    /// divides by a MEAN mass (`crates/sage/src/scoring.rs`). Measured on the
    /// committed v0.15 serum output:
    ///
    /// | population | mean abs diff | sign flips |
    /// |---|---|---|
    /// | clean subset, n=3777 | 0.021 ppm | 0 |
    /// | all rows, n=68817 | 7026 ppm | 0 |
    ///
    /// They agree where the bias is measured, and diverge across the open
    /// window. Swapping in Sage's column would import that divergence into
    /// `bias_ppm`, which sets the user's tolerance recommendation.
    pub precursor_ppm_signed: f64,
    /// Sage's `fragment_ppm`, copied as-is. ⚠ ABSOLUTE, still, in the pinned
    /// v0.15: measured 0 of 68817 values negative. `precursor_ppm` became
    /// signed at v0.15 and this one did not, so the two conventions must not be
    /// merged. No signed MS2 bias can be read from this column.
    pub fragment_ppm: f64,
    pub hyperscore: f64,
    pub spectrum_index: u32,
    pub is_decoy: bool,
}

impl PsmSummary {
    /// Build a `PsmSummary` from a parsed Sage `Psm` row. `rank` is
    /// saturating-cast from u32 to u8 (Sage chimeric rank realistically
    /// never exceeds a couple dozen; anything absurd just won't match
    /// `rank == 1` in `select_clean_subset` and gets excluded, never panics).
    pub fn from_psm(psm: &crate::sage_results::Psm) -> Self {
        PsmSummary {
            q_value: psm.peptide_q,
            rank: psm.rank.min(u8::MAX as u32) as u8,
            corrected_delta_da: psm.delta_mass_corrected,
            precursor_ppm_signed: signed_precursor_ppm(psm),
            fragment_ppm: psm.fragment_ppm,
            hyperscore: psm.hyperscore,
            spectrum_index: psm.scannr,
            is_decoy: psm.is_decoy,
        }
    }
}

/// SIGNED precursor mass error in ppm, reconstructed from Sage's mass columns.
///
/// `delta_mass_corrected / calcmass * 1e6`, where `delta_mass_corrected` is
/// `(expmass - calcmass) - isotope_error * C13_C12_DIFF` (`sage_results.rs`).
/// This is the same quantity Sage's `precursor_ppm` reports, WITH its sign
/// kept. v0.14.x folded that column to `|error|`, which made a
/// median of it read high whenever the true bias is not large compared with
/// the scatter, and negative on no file at all.
///
/// Returns a non-finite value when `calcmass` is zero — a corrupt row that no
/// real Sage TSV produces. `compute_ms1_stats` drops non-finite values rather
/// than folding a fabricated zero into the median.
pub fn signed_precursor_ppm(psm: &crate::sage_results::Psm) -> f64 {
    psm.delta_mass_corrected / psm.calcmass * 1e6
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        Some((values[mid - 1] + values[mid]) / 2.0)
    } else {
        Some(values[mid])
    }
}

fn percentile(values: &mut [f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let rank = ((p / 100.0) * (values.len() as f64 - 1.0)).round() as usize;
    values.get(rank).copied()
}

/// Select the near-zero clean subset used for MS1 calibration measurement.
///
/// Rules (see recon-calibration-design-v2.md / temp-flowChart.md):
/// - rank-1 only (chimeric rank-2 rows inherit rank-1's q-value and would bias
///   the measurement)
/// - q <= q_threshold. ⚠ This read `q <` until 2026-09-01, which made it the one
///   site in the pipeline that excluded the boundary value while
///   `parse_sage_results` and `protein_index` admitted it. See NOTES
///   "`q <= 0.01` IS THE RULE EVERYWHERE". Measured before the change: 0 rows sit
///   at `peptide_q == 0.01` in any of the eight committed pass-1/pass-2 TSVs
///   (542,673 rows), so the change moves nothing on this data — it makes the
///   rule true rather than nearly true.
/// - |corrected_delta_da| < delta_da_threshold (near-zero mass defect)
/// - optional hyperscore guard: keep top ~60% by hyperscore, but only if that
///   still yields >= 200 PSMs AND >= 10% of total spectra; otherwise fall back
///   to the unguarded subset (q-value is the principled primary gate).
pub fn select_clean_subset(
    all_psms: &[PsmSummary],
    delta_da_threshold: f64,
    q_threshold: f64,
    use_hyperscore_guard: bool,
    total_ms2_spectra: usize,
) -> Vec<PsmSummary> {
    let mut subset: Vec<PsmSummary> = all_psms
        .iter()
        .filter(|&psm| {
            !psm.is_decoy
                && psm.rank == 1
                && psm.q_value <= q_threshold
                && psm.corrected_delta_da.abs() < delta_da_threshold
        })
        .cloned()
        .collect();

    if use_hyperscore_guard
        && !subset.is_empty()
        && hyperscore_guard_would_apply(subset.len(), total_ms2_spectra, all_psms.len())
    {
        subset.sort_by(|a, b| {
            b.hyperscore
                .partial_cmp(&a.hyperscore)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let keep = ((subset.len() as f64) * 0.60).round() as usize;
        subset.truncate(keep.max(1));
    }
    // else: guard would shrink an already-marginal subset further (or was
    // not requested); fall back to the unguarded q-value-only subset.

    subset
}

/// Single source of truth for the hyperscore-guard eligibility rule
/// (temp-flowChart.md: "use top 50-70% by hyperscore ... if that
/// yields >= 200 PSMs AND >= 10% of total spectra. Otherwise fall back
/// to no hyperscore filter"). Used by `select_clean_subset` to decide
/// whether to apply the guard, and callable by report-building code
/// afterward to know (without re-deriving the threshold) whether the
/// guard WAS applied to a subset it already has — keeps both call sites
/// reading the same rule.
///
/// ⚠ The wrap above is deliberate: `>=` must not START a line. rustdoc
/// reads a leading `>` as a blockquote, which is what made
/// `clippy::doc_lazy_continuation` fire here. Clippy's suggested fix
/// prefixes the CONTINUATION lines with `>` too, which renders four
/// lines of prose as a quotation and makes the misparse permanent.
/// Reflowing removes the blockquote instead, and keeps the quoted
/// sentence character for character.
///
/// `total_ms2_spectra` should be the true total MS2 spectra in the run
/// (e.g. from mzML stats); pass 0 to fall back to `total_psms_considered`
/// (matches `select_clean_subset`'s own fallback for callers with no better
/// number, such as unit tests).
pub fn hyperscore_guard_would_apply(
    clean_subset_len: usize,
    total_ms2_spectra: usize,
    total_psms_considered: usize,
) -> bool {
    let total_spectra = if total_ms2_spectra > 0 {
        total_ms2_spectra
    } else {
        total_psms_considered
    };
    let min_spectra_fraction = (total_spectra as f64 * 0.10).ceil() as usize;
    clean_subset_len >= 200 && clean_subset_len >= min_spectra_fraction
}

/// Compute robust MS1 precursor mass error statistics from the clean subset.
///
/// `bias_ppm` is a median of SIGNED errors (`PsmSummary::precursor_ppm_signed`),
/// so it can be negative — and on bcell it is. A median of Sage's absolute
/// `precursor_ppm` cannot be, which is the bug this replaced.
pub fn compute_ms1_stats(clean_subset: &[PsmSummary]) -> Option<MassErrorStats> {
    if clean_subset.is_empty() {
        return None;
    }

    // Non-finite values can only come from a zero `calcmass` — a corrupt row.
    // Drop them; a NaN sorts unpredictably and would corrupt the median
    // silently, and a substituted zero would pull the bias toward zero.
    let ppm_values: Vec<f64> = clean_subset
        .iter()
        .map(|psm| psm.precursor_ppm_signed)
        .filter(|v| v.is_finite())
        .collect();
    if ppm_values.is_empty() {
        return None;
    }

    let bias_ppm = median(&mut ppm_values.clone())?;

    let mut deviations: Vec<f64> = ppm_values.iter().map(|v| (v - bias_ppm).abs()).collect();
    let mad_ppm = median(&mut deviations).unwrap_or(0.0);

    Some(MassErrorStats {
        bias_ppm,
        mad_ppm,
        // The count actually measured, which is the subset size unless a
        // corrupt row was dropped above.
        n_psms: ppm_values.len(),
    })
}

/// The ppm rungs a user can realistically type into a search engine.
///
/// A CHOICE informed by field practice, not derived from our data — see
/// `_dev/reference-notes/ms1-tolerance-recommendation-rationale.md` §6, which marks it
/// `[CHOICE]`. The 10 ppm floor is deliberate: recommending below it buys
/// nothing, because narrow tolerances do not reject noise, they select different
/// noise. ⚠ Behaviour above 10 ppm is UNTESTED on real data — all three test
/// files land on the first rung. That limitation ships in the write-up.
pub const MS1_TOLERANCE_LADDER_PPM: [f64; 4] = [10.0, 20.0, 50.0, 100.0];

/// Multiplier on MAD when sizing the tolerance requirement.
///
/// Also a CHOICE (~2.5x IQR). The defence is not that 5 is optimal — it is that
/// the BUCKET is insensitive to it: any k from roughly 3 to 15 yields the same
/// rung on all three test files. That robustness is the whole argument for
/// quantizing, and it is why an arbitrary constant is tolerable here.
pub const MS1_TOLERANCE_MAD_K: f64 = 5.0;

/// Smallest ladder rung that covers `|bias| + K*MAD`, or the top rung if the
/// requirement exceeds every rung (a pathological file still gets a number, and
/// the reported bias/MAD are what reveal the pathology).
pub fn ms1_tolerance_requirement(bias_ppm: f64, mad_ppm: f64) -> f64 {
    bias_ppm.abs() + MS1_TOLERANCE_MAD_K * mad_ppm
}

/// The smallest rung of [`MS1_TOLERANCE_LADDER_PPM`] at or above `required_ppm`,
/// or the top rung when nothing covers it.
///
/// Split out of `quantize_ms1_tolerance` on 2026-09-03 so the MS2 recommendation
/// can land on the SAME ladder without inventing a second copy of it. The MS1
/// requirement (`|bias| + K*MAD`) stays MS1's own; only the quantization step is
/// shared.
pub fn ladder_rung(required_ppm: f64) -> f64 {
    MS1_TOLERANCE_LADDER_PPM
        .iter()
        .copied()
        .find(|&rung| rung >= required_ppm)
        .unwrap_or_else(|| *MS1_TOLERANCE_LADDER_PPM.last().unwrap())
}

pub fn quantize_ms1_tolerance(bias_ppm: f64, mad_ppm: f64) -> f64 {
    ladder_rung(ms1_tolerance_requirement(bias_ppm, mad_ppm))
}

/// User-facing MS1 tolerance recommendation: a QUANTIZED bucket.
///
/// `smallest rung in {10,20,50,100} ppm >= |bias| + 5*MAD`.
///
/// **The user is picking a search setting from an effectively discrete set, so
/// precision below a rung is unusable.** A continuous number forces a precision
/// claim the measurement cannot support; a bucket does not, and a bucket is
/// robust to constants we cannot derive.
///
/// ⚠ SUPERSEDED DESIGN, recorded rather than deleted: this used to return an
/// ASYMMETRIC window, `bias - 2*MAD` to `bias + p95(|dev|)`, on the reasoning
/// that "a biased instrument folded into symmetric tolerance clips real IDs on
/// one side". That reasoning is retired because `|bias|` is now folded INTO the
/// requirement before quantizing, and the rung is generous enough to swallow the
/// bias whole — serum's +2.42 ppm bias sits inside a +/-10 ppm rung with room to
/// spare. The old formula was also 3-5x too tight against three independent
/// lines of evidence.
///
/// The BIAS is still reported separately and must be. Bucketing the tolerance
/// does not discard the bias; they are two numbers from one measurement, at
/// opposite ends (locked).
pub fn ms1_user_recommendation(stats: &MassErrorStats) -> Ms1UserRecommendation {
    let tolerance = quantize_ms1_tolerance(stats.bias_ppm, stats.mad_ppm);
    let requirement = ms1_tolerance_requirement(stats.bias_ppm, stats.mad_ppm);
    let top = *MS1_TOLERANCE_LADDER_PPM.last().unwrap();

    Ms1UserRecommendation {
        bias_ppm: stats.bias_ppm,
        spread_mad_ppm: stats.mad_ppm,
        recommended_tolerance_ppm: tolerance,
        measured_requirement_ppm: requirement,
        requirement_exceeds_ladder: requirement > top,
        low_ppm: -tolerance,
        high_ppm: tolerance,
    }
}

/// ⚠ RETIRED 2026-08-28 — SUBSUMED BY THE LADDER, kept only as documentation.
///
/// This was a real backstop while the Pass 2 half-width was `3*MAD`, which is
/// unbounded: a noisy file could produce an arbitrarily wide window. The window
/// now comes from `MS1_TOLERANCE_LADDER_PPM`, whose TOP RUNG IS ALSO 100, so
/// `min(rung, 100)` was always just the rung and the cap could never bind.
///
/// **Proven, not assumed:** deleting the `.min(...)` left the cap's own test
/// passing, so that test had been asserting the ladder's top rung all along. A
/// guard that cannot fail is worse than no guard — see NOTES "Gate audit".
///
/// The ceiling it argued for is now the ladder's top rung, and the original
/// reasoning still supports that value: a tighter ceiling (±15–20 ppm) would
/// systematically clip TOF instruments, which run 50–80 ppm out of the box, so
/// 100 is TOF-safe and stops only pathological fits. That justification is
/// CURATED, not measured — all three test files land on the FIRST rung, so we
/// have no data on rungs 2–4 at all.
#[deprecated(note = "subsumed by MS1_TOLERANCE_LADDER_PPM's top rung; see NOTES")]
pub const PASS2_HALF_WIDTH_CAP_PPM: f64 = 100.0;

/// Pass 2 `precursor_tol`: the ladder rung, CENTRED ON THE MEASURED BIAS.
///
/// Pass 2 `precursor_tol` window: the MEASURED requirement, centred on the bias.
///
/// **⚠ CHANGED TWICE. Read both, because the second change reverses part of the
/// first.**
///
/// 1. It was `bias ± 3×MAD`. That was MEASURED to cover only 80.82 / 87.46 /
///    80.77 % of confident PSMs — one real peptide in five discarded. No single
///    MAD multiplier transfers between instruments of the same class (99 %
///    coverage needed 18.3× / 12.1× / 11.0×).
/// 2. It then became the LADDER RUNG centred on the bias, which covered
///    99.32 / 99.92 / 99.94 %. **That rule is superseded 2026-08-29.** Its
///    stated justification was that "a generous window costs almost nothing in
///    runtime, because Pass 2 runs against a subset FASTA". Measured on bcell,
///    that is FALSE: Pass 2 took 2437 s against Pass 1's 187 s, because bcell
///    identifies 6485 proteins where serum identifies 472.
///
/// **THE RULE NOW: `bias ± (|bias| + K*MAD)`** — the same unquantized
/// requirement `ms1_tolerance_requirement` computes for the user
/// recommendation, but centred on the bias instead of on zero, and NOT rounded
/// up to a rung.
///
/// **Why the cushion does not belong here.** The ladder rung exists because the
/// USER's next search may be different data on a possibly-drifted instrument, so
/// their recommendation is rounded generously upward. Pass 2 re-searches THE
/// SAME SCANS Pass 1 measured the bias from. There is no drift to allow for, and
/// `K = 5` already carries a five-MAD allowance of its own.
///
/// **Measured cost of dropping the cushion** (post-hoc, on Pass-2 output that
/// was searched at the rung): serum keeps 94.16 % of confident PSMs and the
/// reported semi-tryptic rate moves 32.34 % → 32.00 %; bcell keeps 96.76 % and
/// moves 6.18 % → 5.97 %. Both are LOWER bounds — a search actually run tight
/// has fewer candidates per spectrum and therefore better FDR discrimination,
/// which is the same effect that made pass 1 LOSE identifications when its
/// fragment window was widened.
///
/// Still NOT the same number as the user recommendation: that is symmetric about
/// ZERO and quantized to a rung; this is centred on the BIAS and unrounded.
pub fn ms1_pass2_window(stats: &MassErrorStats) -> Ms1Pass2Window {
    // The unrounded requirement, NOT the rung. The ladder's top rung stays the
    // ceiling so a pathological measurement cannot open the window without
    // limit -- that bound is what `PASS2_HALF_WIDTH_CAP_PPM` used to argue for,
    // and it becomes reachable again now that the width is no longer a rung.
    let top = *MS1_TOLERANCE_LADDER_PPM.last().unwrap();
    let half_width = ms1_tolerance_requirement(stats.bias_ppm, stats.mad_ppm).min(top);
    Ms1Pass2Window {
        low_ppm: stats.bias_ppm - half_width,
        high_ppm: stats.bias_ppm + half_width,
    }
}

/// Multiplier on the measured median MS2 |error| for a ppm-unit analyzer.
///
/// MEASURED on the three closed searches: 2x the median covers only 83.5 / 90.9 /
/// 94.4 % of PSMs, 3x covers 95.5 / 97.7 / 99.5 %, and **5x covers 99.6 / 99.8 /
/// 100.0 %**. Five is the smallest of those that clears 99% on every file.
pub const PASS2_MS2_PPM_MULTIPLIER: f64 = 5.0;

/// Multiplier for a Da-unit analyzer (ion trap, quadrupole).
///
/// ⚠ A CURATED ASSUMPTION, not a measurement — we have no ion-trap file. Ben's
/// rule, 2026-08-28: convert the measured ppm error to Da at a representative
/// fragment m/z and double it. Deliberately NOT the 5x used for ppm analyzers: a
/// trap's error already sits near its resolution limit, where 5x would be absurd.
pub const PASS2_MS2_DA_MULTIPLIER: f64 = 2.0;

/// Representative fragment m/z for the ppm→Da conversion.
///
/// ⚠ ALSO AN ASSUMPTION. Most fragment ions of interest fall in 400–600 m/z as
/// singly-charged species; 500 is the midpoint. The conversion is exact only at
/// that m/z — at 400 it over-estimates the Da width by 25%, at 600 it
/// under-estimates by 17%. We cannot do better without per-fragment m/z, which
/// would need `sage --annotate-matches` (declined — see NOTES "MS2 stays
/// absolute").
pub const PASS2_MS2_REPRESENTATIVE_MZ: f64 = 500.0;

/// Pass 2 `fragment_tol`, in the UNIT the detected analyzer requires.
///
/// `measured MS2 error + padding`, then clamped so it can never exceed the
/// pass-1 window. That clamp is not cosmetic: pass 1 defined the search space, so
/// a wider pass-2 window would claim matches pass 1 could not have made.
///
/// The unit comes from `pass1`, never from the measurement — a ppm number must
/// not be handed to an ion trap. See AGENTS.md "The two conventions".
pub fn ms2_pass2_tolerance(
    measured_median_abs_ppm: f64,
    pass1: crate::mzml::FragmentTolerance,
) -> crate::mzml::FragmentTolerance {
    use crate::mzml::FragmentTolerance;
    match pass1 {
        FragmentTolerance::Ppm(pass1_ppm) => {
            let want = PASS2_MS2_PPM_MULTIPLIER * measured_median_abs_ppm;
            FragmentTolerance::Ppm(want.min(pass1_ppm))
        }
        FragmentTolerance::Da(pass1_da) => {
            let as_da = measured_median_abs_ppm * PASS2_MS2_REPRESENTATIVE_MZ / 1e6;
            let want = PASS2_MS2_DA_MULTIPLIER * as_da;
            FragmentTolerance::Da(want.min(pass1_da))
        }
    }
}

/// The USER-FACING MS2 fragment tolerance recommendation.
///
/// The sibling of [`ms1_user_recommendation`], and it must never be confused with
/// [`ms2_pass2_tolerance`]: that one sizes recon's OWN internal pass-2 window and
/// is clamped by the pass-1 setting; this one is a number the user types into
/// their next search, so nothing clamps it.
///
/// `measured_half_width_ppm` is the measured fragment spread — the half-width
/// `Ms1CalibrationReport::ms2_tolerance_high_ppm` already reports.
///
/// ⚠ **THE UNIT COMES FROM `unit`, NEVER FROM THE MEASUREMENT.** `unit` carries
/// only the unit of [`crate::mzml::bucket_tolerance`] for the detected MS2
/// analyzer class; its magnitude is ignored. A ppm ladder is meaningless on an
/// ion trap or quadrupole at unit resolution, so the ladder MUST NOT be reached
/// on that branch. See AGENTS.md "The three tolerance regimes".
///
/// * **ppm analyzer** — quantized onto [`MS1_TOLERANCE_LADDER_PPM`] via
///   [`ladder_rung`], the same ladder and the same rungs the MS1 recommendation
///   uses. A user picks a search setting from an effectively discrete set, and
///   that argument does not change between MS1 and MS2.
/// * **Da analyzer** — Ben's recorded rule: convert the measured ppm to Da at
///   [`PASS2_MS2_REPRESENTATIVE_MZ`] (500), multiply by
///   [`PASS2_MS2_DA_MULTIPLIER`] (2), then round UP to the nearest tenth of a
///   Dalton via [`round_up_to_tenth_da`]. The two constants are reused rather
///   than restated so the pass-2 rule and the recommendation cannot drift apart.
///
///   The tenth-Da step is the Da regime's LADDER, and it exists for the same
///   reason the ppm ladder does: a user picks a search setting from an
///   effectively discrete set, so precision below the step is unusable. It is
///   NOT applied to [`ms2_pass2_tolerance`], which sizes recon's own internal
///   window and is not a number anyone types — exactly as the ppm branch
///   quantizes the recommendation and not the pass-2 window.
///
///   ⚠ The constants were derived against the median |error|; this feeds them the
///   measured SPREAD, so that both branches start from the one measured number.
///   ⚠ **Quantizing makes that choice matter less, but does not settle it.**
///   Across the documented 0.3-0.8 Da unit-resolution band the step is 0.1 Da
///   wide, so the two candidate inputs have to differ by more than a tenth of a
///   Dalton before the recommendation moves at all.
///   Still a CHOICE, recorded because no ion-trap file exists to test it — all
///   four committed test files are Orbitraps, so this branch is exercised by
///   `ion_trap_recommendation_is_in_daltons_never_ppm` and by nothing else.
pub fn ms2_user_recommendation(
    measured_half_width_ppm: f64,
    unit: crate::mzml::FragmentTolerance,
) -> crate::mzml::FragmentTolerance {
    use crate::mzml::FragmentTolerance;
    match unit {
        FragmentTolerance::Ppm(_) => FragmentTolerance::Ppm(ladder_rung(measured_half_width_ppm)),
        FragmentTolerance::Da(_) => {
            let as_da = measured_half_width_ppm * PASS2_MS2_REPRESENTATIVE_MZ / 1e6;
            FragmentTolerance::Da(round_up_to_tenth_da(PASS2_MS2_DA_MULTIPLIER * as_da))
        }
    }
}

/// Round UP to the nearest tenth of a Dalton — the Da regime's ladder step.
///
/// ⚠ **A naive `(v * 10.0).ceil() / 10.0` IS WRONG HERE, and the failure was
/// measured before this function was written.** Multiplying by 10 can land a
/// value that is already exactly on a tenth a few ULP ABOVE the integer, and
/// `ceil` then promotes it a whole tenth. Sweeping every tenth from 0.0 to 39.9
/// found 20 such values — 2.1 Da became 2.2, 4.9 became 5.0, 6.9 became 7.0.
/// None sits inside the 0.3-0.8 Da band a real ion trap would produce, which is
/// precisely why it would have survived the plausible test cases.
///
/// The guard: if scaling lands within 1e-9 of an integer, treat it AS that
/// integer instead of rounding away from it. Verified over 40 000 inputs on
/// three properties — every exact tenth maps to itself, the result is always
/// `>= v` and `< v + 0.1`, and every result lands exactly on a tenth.
fn round_up_to_tenth_da(v: f64) -> f64 {
    let scaled = v * 10.0;
    let nearest = scaled.round();
    let up = if (scaled - nearest).abs() < 1e-9 {
        nearest
    } else {
        scaled.ceil()
    };
    up / 10.0
}

/// MS2 fragment tolerance recommendation, derived directly from Sage's
/// per-PSM `fragment_ppm` column for confident (q < 0.01, rank-1) PSMs.
/// No recomputation of fragment error from raw spectra - Sage's own
/// fragment_ppm is used as-is, per temp-flowChart.md.
pub fn compute_ms2_tolerance(fragment_ppm_values: &[f64]) -> Option<Ms2Tolerance> {
    if fragment_ppm_values.is_empty() {
        return None;
    }

    let vals = fragment_ppm_values.to_vec();
    let median_ppm = median(&mut vals.clone())?;

    let mut devs: Vec<f64> = vals.iter().map(|v| (v - median_ppm).abs()).collect();
    let mad_ppm = median(&mut devs).unwrap_or(0.0);
    let tail_ppm = percentile(&mut devs.clone(), 95.0).unwrap_or(0.0);

    Some(Ms2Tolerance {
        low_ppm: -tail_ppm,
        high_ppm: tail_ppm,
        median_ppm,
        mad_ppm,
        tail_95_ppm: tail_ppm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_psm(
        q: f64,
        rank: u8,
        delta_da: f64,
        precursor_ppm: f64,
        hyperscore: f64,
        idx: u32,
    ) -> PsmSummary {
        make_psm_decoy(q, rank, delta_da, precursor_ppm, hyperscore, idx, false)
    }

    fn make_psm_decoy(
        q: f64,
        rank: u8,
        delta_da: f64,
        precursor_ppm: f64,
        hyperscore: f64,
        idx: u32,
        is_decoy: bool,
    ) -> PsmSummary {
        PsmSummary {
            q_value: q,
            rank,
            corrected_delta_da: delta_da,
            precursor_ppm_signed: precursor_ppm,
            fragment_ppm: 0.0,
            hyperscore,
            spectrum_index: idx,
            is_decoy,
        }
    }

    #[test]
    fn clean_subset_filters_rank_q_and_delta() {
        let psms = vec![
            make_psm(0.001, 1, 0.001, 1.0, 50.0, 0),
            make_psm(0.001, 2, 0.001, 1.0, 50.0, 0), // wrong rank
            make_psm(0.5, 1, 0.001, 1.0, 50.0, 1),   // fails q
            make_psm(0.001, 1, 5.0, 1.0, 50.0, 2),   // fails delta
            make_psm(0.001, 1, -0.001, 2.0, 60.0, 3),
        ];
        let subset = select_clean_subset(&psms, 0.02, 0.01, false, 0);
        assert_eq!(subset.len(), 2);
    }

    #[test]
    fn clean_subset_excludes_decoys() {
        // Locked (NOTES "MS1 error from the wide search's clean subset"): the
        // clean subset is target-only. A decoy that otherwise clears every
        // gate must still be excluded.
        let psms = vec![
            make_psm_decoy(0.001, 1, 0.001, 1.0, 50.0, 0, false),
            make_psm_decoy(0.001, 1, 0.001, 1.0, 50.0, 1, true), // decoy, otherwise clean
        ];
        let subset = select_clean_subset(&psms, 0.02, 0.01, false, 0);
        assert_eq!(subset.len(), 1);
        assert!(!subset[0].is_decoy);
    }

    #[test]
    fn hyperscore_guard_falls_back_when_subset_too_small() {
        // Only 3 clean PSMs total - guard should not shrink further (< 200).
        let psms: Vec<PsmSummary> = (0..3)
            .map(|i| make_psm(0.001, 1, 0.001, 1.0, i as f64, i))
            .collect();
        let subset = select_clean_subset(&psms, 0.02, 0.01, true, 0);
        assert_eq!(subset.len(), 3);
    }

    #[test]
    fn hyperscore_guard_applies_when_subset_large_enough() {
        let psms: Vec<PsmSummary> = (0..1000)
            .map(|i| make_psm(0.001, 1, 0.001, 1.0, i as f64, i))
            .collect();
        let subset = select_clean_subset(&psms, 0.02, 0.01, true, 0);
        // 60% of 1000 = 600
        assert_eq!(subset.len(), 600);
        // Must keep the highest-hyperscore PSMs
        assert!(subset.iter().all(|p| p.hyperscore >= 400.0));
    }

    #[test]
    fn hyperscore_guard_uses_caller_supplied_total_spectra() {
        // 250 clean PSMs is >=200 and would pass a naive max(idx)+1 guard,
        // but if the run actually had 10,000 MS2 spectra, 250 is only 2.5%
        // of total — below the 10% floor — so the guard must fall back
        // unguarded rather than truncate to 60%.
        let psms: Vec<PsmSummary> = (0..250)
            .map(|i| make_psm(0.001, 1, 0.001, 1.0, i as f64, i))
            .collect();
        let subset = select_clean_subset(&psms, 0.02, 0.01, true, 10_000);
        assert_eq!(subset.len(), 250);
    }

    #[test]
    fn ms1_stats_bias_and_mad_are_correct() {
        let psms = vec![
            make_psm(0.001, 1, 0.0, 1.0, 50.0, 0),
            make_psm(0.001, 1, 0.0, 2.0, 50.0, 1),
            make_psm(0.001, 1, 0.0, 3.0, 50.0, 2),
            make_psm(0.001, 1, 0.0, 4.0, 50.0, 3),
            make_psm(0.001, 1, 0.0, 5.0, 50.0, 4),
        ];
        let stats = compute_ms1_stats(&psms).unwrap();
        assert_eq!(stats.bias_ppm, 3.0);
        assert_eq!(stats.mad_ppm, 1.0);
        assert_eq!(stats.n_psms, 5);
    }

    #[test]
    fn ms1_recommendation_is_a_ladder_rung() {
        // |2.5| + 5*1.2 = 8.5 -> first rung that covers it is 10.
        let stats = MassErrorStats {
            bias_ppm: 2.5,
            mad_ppm: 1.2,
            n_psms: 500,
        };
        let rec = ms1_user_recommendation(&stats);
        assert_eq!(rec.recommended_tolerance_ppm, 10.0);
        assert_eq!(rec.low_ppm, -10.0);
        assert_eq!(rec.high_ppm, 10.0);
        // The bias is still reported, not swallowed by the bucket.
        assert_eq!(rec.bias_ppm, 2.5);
    }

    #[test]
    fn a_negative_bias_uses_its_magnitude() {
        // The requirement is |bias| + k*MAD. A negative bias must widen the
        // window, never narrow it — bcell's true bias is negative.
        let neg = quantize_ms1_tolerance(-9.0, 0.4);
        let pos = quantize_ms1_tolerance(9.0, 0.4);
        assert_eq!(neg, pos);
        assert_eq!(neg, 20.0, "|-9.0| + 2.0 = 11.0 must step up to 20");
    }

    #[test]
    fn the_ladder_steps_up_and_never_exceeds_its_top() {
        assert_eq!(quantize_ms1_tolerance(0.0, 0.0), 10.0, "floor is 10");
        assert_eq!(quantize_ms1_tolerance(0.0, 2.0), 10.0);
        assert_eq!(quantize_ms1_tolerance(0.0, 2.1), 20.0);
        assert_eq!(quantize_ms1_tolerance(0.0, 8.0), 50.0);
        assert_eq!(quantize_ms1_tolerance(0.0, 15.0), 100.0);
        // A pathological file still returns a number rather than panicking.
        assert_eq!(quantize_ms1_tolerance(0.0, 1000.0), 100.0);
    }

    #[test]
    fn symmetric_rung_always_contains_the_bias_centred_window() {
        // THE INVARIANT behind dropping the asymmetric window. A strongly
        // biased instrument (a timsTOF wanting, say, -30..+50 ppm) must never be
        // clipped by a symmetric recommendation. Swept rather than spot-checked,
        // and both signs of bias are covered because a negative bias is the case
        // the folded |error| column could never even produce.
        for bias in [-60.0, -30.0, -9.0, -0.24, 0.0, 0.44, 2.42, 30.0, 60.0] {
            for mad in [0.0, 0.4, 0.68, 2.0, 4.0, 8.0] {
                let rung = quantize_ms1_tolerance(bias, mad);
                let requirement = ms1_tolerance_requirement(bias, mad);
                if requirement > *MS1_TOLERANCE_LADDER_PPM.last().unwrap() {
                    // Off the ladder: the guarantee is explicitly surrendered,
                    // and the caller is told. Skip, do not silently pass.
                    continue;
                }
                let lo = bias - MS1_TOLERANCE_MAD_K * mad;
                let hi = bias + MS1_TOLERANCE_MAD_K * mad;
                assert!(
                    -rung <= lo && hi <= rung,
                    "bias={bias} mad={mad}: bias-centred [{lo}, {hi}] escapes \
                     symmetric [{}, {rung}]",
                    -rung
                );
            }
        }
    }

    #[test]
    fn an_off_the_ladder_file_is_flagged_not_quietly_capped() {
        // The hole in the containment proof, made visible. 40 + 5*20 = 140 ppm,
        // past the 100 ppm top rung: the recommendation is KNOWN too narrow.
        let stats = MassErrorStats {
            bias_ppm: 40.0,
            mad_ppm: 20.0,
            n_psms: 500,
        };
        let rec = ms1_user_recommendation(&stats);
        assert_eq!(rec.recommended_tolerance_ppm, 100.0);
        assert_eq!(rec.measured_requirement_ppm, 140.0);
        assert!(
            rec.requirement_exceeds_ladder,
            "must be flagged, not silently capped"
        );

        // And the normal case must NOT raise the flag.
        let ok = MassErrorStats {
            bias_ppm: 2.4215,
            mad_ppm: 0.4845,
            n_psms: 500,
        };
        assert!(!ms1_user_recommendation(&ok).requirement_exceeds_ladder);
    }

    #[test]
    fn the_bucket_is_insensitive_to_k_which_is_the_whole_argument() {
        // The rationale note's actual claim: any k from ~3 to ~15 gives the same
        // rung on our files. Assert it with serum's corrected numbers rather
        // than trusting the prose.
        let (bias, mad): (f64, f64) = (2.4215, 0.4845);
        for k in [3.0, 5.0, 10.0, 15.0] {
            let required = bias.abs() + k * mad;
            let rung = MS1_TOLERANCE_LADDER_PPM
                .iter()
                .copied()
                .find(|&r| r >= required)
                .unwrap();
            assert_eq!(rung, 10.0, "k={k} moved serum off the 10 ppm rung");
        }
    }

    #[test]
    fn pass2_window_is_the_measured_requirement_centred_on_bias() {
        // |−1| + 5*2 = 11, unrounded. The superseded rule rounded this UP to the
        // 20 ppm rung and gave -21..+19; that is exactly the cushion that made
        // bcell's Pass 2 take 2437 s.
        let stats = MassErrorStats {
            bias_ppm: -1.0,
            mad_ppm: 2.0,
            n_psms: 500,
        };
        let win = ms1_pass2_window(&stats);
        assert!((win.low_ppm - -12.0).abs() < 1e-9, "got {}", win.low_ppm);
        assert!((win.high_ppm - 10.0).abs() < 1e-9, "got {}", win.high_ppm);
        // ...and it must NOT be the rung any more.
        assert_ne!(
            win.high_ppm - win.low_ppm,
            40.0,
            "the window is still a rung"
        );
    }

    #[test]
    fn pass2_window_is_centred_on_bias_where_the_recommendation_is_not() {
        // The two numbers must stay distinct: the recommendation is symmetric
        // about ZERO, the Pass 2 window is centred on the measured bias. serum.
        let stats = MassErrorStats {
            bias_ppm: 2.4215,
            mad_ppm: 0.4845,
            n_psms: 3764,
        };
        let rec = ms1_user_recommendation(&stats);
        let win = ms1_pass2_window(&stats);
        // The recommendation is QUANTIZED and symmetric about zero.
        assert_eq!((rec.low_ppm, rec.high_ppm), (-10.0, 10.0));
        // The Pass 2 window is UNROUNDED and centred on the bias: serum's
        // requirement is 2.4215 + 5*0.4845 = 4.8440 ppm.
        let req = 2.4215 + 5.0 * 0.4845;
        assert!(
            (win.low_ppm - (2.4215 - req)).abs() < 1e-9,
            "got {}",
            win.low_ppm
        );
        assert!(
            (win.high_ppm - (2.4215 + req)).abs() < 1e-9,
            "got {}",
            win.high_ppm
        );
        assert_ne!(
            win.low_ppm, rec.low_ppm,
            "the two must not collapse into one number"
        );
        // The Pass 2 window is now NARROWER than the recommendation, which is
        // the whole point of dropping the cushion.
        assert!(win.high_ppm - win.low_ppm < rec.high_ppm - rec.low_ppm);
    }

    #[test]
    fn mad_sets_the_pass2_width_again() {
        // ⚠ THIS TEST WAS REVERSED 2026-08-29 and the old name is kept in this
        // comment so the change is findable: `mad_no_longer_sets_the_pass2_width`
        // asserted that two files with the same bias and different MAD got the
        // SAME width, because the width was a quantized rung. The width is now
        // the unrounded requirement, so MAD sets it directly -- which is the
        // intended behaviour, not a regression.
        let a = MassErrorStats {
            bias_ppm: 0.5,
            mad_ppm: 0.40,
            n_psms: 500,
        };
        let b = MassErrorStats {
            bias_ppm: 0.5,
            mad_ppm: 0.95,
            n_psms: 500,
        };
        let (wa, wb) = (ms1_pass2_window(&a), ms1_pass2_window(&b));
        assert!(
            wb.high_ppm - wb.low_ppm > wa.high_ppm - wa.low_ppm,
            "a larger MAD must now give a wider Pass 2 window"
        );
        // The USER RECOMMENDATION is still quantized, so it does NOT move here.
        // Both files land on the same rung. Keeping this assertion next to the
        // one above is what stops the two rules being merged again.
        assert_eq!(
            ms1_user_recommendation(&a).recommended_tolerance_ppm,
            ms1_user_recommendation(&b).recommended_tolerance_ppm
        );
    }

    #[test]
    fn ms2_pass2_never_exceeds_the_pass1_window_and_keeps_its_unit() {
        use crate::mzml::FragmentTolerance::{Da, Ppm};
        // Orbitrap: 5 * 1.26 = 6.3 ppm, well inside the ±50 ppm pass-1 window.
        assert_eq!(ms2_pass2_tolerance(1.26, Ppm(50.0)), Ppm(6.3));
        // A pathological measurement must be clamped to pass 1, never exceed it.
        assert_eq!(ms2_pass2_tolerance(400.0, Ppm(50.0)), Ppm(50.0));
        // Ion trap: the unit comes from pass 1, NOT from the measurement.
        // 800 ppm at m/z 500 = 0.4 Da, doubled = 0.8 Da, inside the 1.0 Da window.
        match ms2_pass2_tolerance(800.0, Da(1.0)) {
            Da(v) => assert!((v - 0.8).abs() < 1e-9, "got {v}"),
            other => panic!("an ion trap must keep Da, got {other}"),
        }
        // And clamped there too.
        assert_eq!(ms2_pass2_tolerance(5000.0, Da(1.0)), Da(1.0));
    }

    /// ⚠ **THE ONLY EXERCISE THE DA BRANCH GETS.** All four committed test files
    /// are Orbitraps, so no real input reaches it. That is exactly why it is
    /// asserted here rather than trusted to real data.
    #[test]
    fn ion_trap_recommendation_is_in_daltons_never_ppm() {
        use crate::mzml::FragmentTolerance::{Da, Ppm};
        use crate::mzml::ION_TRAP_MS2_HALF_WIDTH_DA;
        // Whatever the measurement, a Da analyzer must never be handed a rung.
        //
        // ⚠ **THE "value never equals a rung" LOOP THAT USED TO BE HERE IS GONE,
        // AND ITS REMOVAL IS NOT A WEAKENING.** It asserted the Da result never
        // numerically equals one of {10,20,50,100}. That was only ever a proxy
        // for "the Da branch did not call `ladder_rung`", and quantizing to a
        // tenth of a Dalton (2026-09-03) makes those values legitimately
        // reachable: 99999 ppm gives 99.999 Da, which rounds UP to exactly
        // 100.0. Worse, `ladder_rung(99999)` is ALSO 100.0, so at that input no
        // value-based check can tell the two paths apart at all — the proxy
        // cannot do its job, and it now rejects a correct answer.
        //
        // The real claim is about the UNIT, and the match arms below enforce it
        // directly: a Da analyzer must return `Da(_)`, never `Ppm(_)`. The
        // arithmetic itself is pinned by
        // `a_da_analyzer_is_quantized_to_a_tenth_of_a_dalton`.
        for measured in [0.0, 1.67, 9.9, 250.0, 600.0, 99999.0] {
            match ms2_user_recommendation(measured, Da(ION_TRAP_MS2_HALF_WIDTH_DA)) {
                Da(v) => {
                    assert!(v >= 0.0, "measured={measured} gave {v} Da");
                    assert!(v.is_finite(), "measured={measured} gave {v} Da");
                }
                Ppm(v) => panic!("measured={measured} leaked ppm ({v}) into an ion trap"),
            }
        }
        // The recorded rule, worked through: a unit-resolution trap measuring
        // 600 ppm gives 600e-6 * 500 = 0.3 Da, doubled = 0.6 Da. That sits in the
        // 0.3-0.8 Da band `ION_TRAP_MS2_HALF_WIDTH_DA` documents as typical.
        match ms2_user_recommendation(600.0, Da(ION_TRAP_MS2_HALF_WIDTH_DA)) {
            Da(v) => assert!((v - 0.6).abs() < 1e-12, "got {v}"),
            other => panic!("expected Da, got {other}"),
        }
    }

    /// The Da branch is QUANTIZED to a tenth of a Dalton — the Da regime's
    /// ladder step (Ben, 2026-09-03). Convert at m/z 500, double, round UP.
    #[test]
    fn a_da_analyzer_is_quantized_to_a_tenth_of_a_dalton() {
        use crate::mzml::FragmentTolerance::Da;
        use crate::mzml::ION_TRAP_MS2_HALF_WIDTH_DA;

        let rec = |ppm: f64| match ms2_user_recommendation(ppm, Da(ION_TRAP_MS2_HALF_WIDTH_DA)) {
            Da(v) => v,
            other => panic!("expected Da, got {other}"),
        };

        // Worked by hand across the documented 0.3-0.8 Da unit-resolution band:
        // ppm * 500 / 1e6 * 2, then up to the next tenth.
        for (ppm, want) in [
            (300.0, 0.3),
            (400.0, 0.4),
            (600.0, 0.6),
            (800.0, 0.8),
            (1600.0, 1.6),
        ] {
            assert!(
                (rec(ppm) - want).abs() < 1e-12,
                "{ppm} ppm gave {}",
                rec(ppm)
            );
        }

        // Rounding is UP, never to nearest: anything above a step takes the next
        // one. 301 ppm is 301e-6 * 500 = 0.1505 Da, doubled to 0.301, which is
        // barely over the 0.3 step and must therefore report 0.4, not 0.3.
        assert!((rec(301.0) - 0.4).abs() < 1e-12, "got {}", rec(301.0));
        // A well-calibrated measurement still cannot recommend below one step.
        assert!((rec(1.0) - 0.1).abs() < 1e-12, "got {}", rec(1.0));

        // Every result lands exactly on a tenth, and never below the raw value.
        for k in 0..4000 {
            let ppm = k as f64 * 0.5;
            let raw = PASS2_MS2_DA_MULTIPLIER * (ppm * PASS2_MS2_REPRESENTATIVE_MZ / 1e6);
            let got = rec(ppm);
            assert!(got >= raw - 1e-12, "{ppm} ppm: {got} < raw {raw}");
            assert!(
                got < raw + 0.1 + 1e-12,
                "{ppm} ppm: {got} overshot raw {raw}"
            );
            let tenths = got * 10.0;
            assert!(
                (tenths - tenths.round()).abs() < 1e-9,
                "{ppm} ppm gave {got}, which is not a tenth"
            );
        }
    }

    /// ⚠ REGRESSION CASE for the float defect a naive `(v*10).ceil()/10` has.
    ///
    /// These five values are already exactly on a tenth, and scaling by 10 puts
    /// them a few ULP ABOVE the integer, so a bare `ceil` promotes each by a
    /// whole tenth. None lies in the 0.3-0.8 Da band a real trap produces, which
    /// is why plausible test cases would not have caught it. The inputs are the
    /// ppm that produces each Da value through the real conversion.
    #[test]
    fn quantizing_never_promotes_a_value_already_on_a_tenth() {
        use crate::mzml::FragmentTolerance::Da;
        use crate::mzml::ION_TRAP_MS2_HALF_WIDTH_DA;

        for target in [2.1_f64, 4.2, 4.9, 5.9, 6.9, 7.9, 8.4, 9.3] {
            let ppm = target / PASS2_MS2_DA_MULTIPLIER / PASS2_MS2_REPRESENTATIVE_MZ * 1e6;
            match ms2_user_recommendation(ppm, Da(ION_TRAP_MS2_HALF_WIDTH_DA)) {
                Da(v) => assert!(
                    (v - target).abs() < 1e-12,
                    "{target} Da was promoted to {v} Da"
                ),
                other => panic!("expected Da, got {other}"),
            }
        }
    }

    /// The ppm branch lands on the SAME rungs the MS1 recommendation uses.
    #[test]
    fn a_ppm_analyzer_gets_a_ladder_rung() {
        use crate::mzml::FragmentTolerance::{Da, Ppm};
        use crate::mzml::ORBITRAP_MS2_HALF_WIDTH_PPM;
        // serum's measured half-width is 1.67 ppm; the first rung covers it.
        assert_eq!(
            ms2_user_recommendation(1.67, Ppm(ORBITRAP_MS2_HALF_WIDTH_PPM)),
            Ppm(10.0)
        );
        assert_eq!(ms2_user_recommendation(10.0, Ppm(20.0)), Ppm(10.0));
        assert_eq!(ms2_user_recommendation(10.1, Ppm(20.0)), Ppm(20.0));
        assert_eq!(ms2_user_recommendation(75.0, Ppm(20.0)), Ppm(100.0));
        // Off the top of the ladder still yields the top rung, never a Da value.
        assert_eq!(ms2_user_recommendation(1e6, Ppm(20.0)), Ppm(100.0));
        // The magnitude of `unit` is IGNORED; only its unit is read.
        assert_eq!(ms2_user_recommendation(1.67, Ppm(0.001)), Ppm(10.0));
        assert!(matches!(ms2_user_recommendation(1.67, Da(0.5)), Da(_)));
    }

    #[test]
    fn every_analyzer_bucket_routes_to_a_recommendation_in_its_own_unit() {
        use crate::mzml::{bucket_tolerance, AnalyzerClass, FragmentTolerance};
        for class in [
            AnalyzerClass::Orbitrap,
            AnalyzerClass::AstralTof,
            AnalyzerClass::LegacyTof,
            AnalyzerClass::IonTrap,
        ] {
            let unit = bucket_tolerance(class).expect("every bucket above has a number");
            let rec = ms2_user_recommendation(1.67, unit);
            match (unit, rec) {
                (FragmentTolerance::Ppm(_), FragmentTolerance::Ppm(_)) => {}
                (FragmentTolerance::Da(_), FragmentTolerance::Da(_)) => {}
                _ => panic!("{class:?} changed unit: {unit} -> {rec}"),
            }
        }
    }

    #[test]
    fn a_ppm_measurement_never_leaks_into_an_ion_trap_tolerance() {
        use crate::mzml::FragmentTolerance::{Da, Ppm};
        // The unit-safety control. Whatever the measurement, a Da pass-1 window
        // must yield a Da pass-2 window and never a ppm one.
        for measured in [0.0, 1.26, 250.0, 1000.0, 99999.0] {
            match ms2_pass2_tolerance(measured, Da(1.0)) {
                Da(v) => assert!((0.0..=1.0).contains(&v), "measured={measured} gave {v} Da"),
                Ppm(v) => panic!("measured={measured} leaked ppm ({v}) into an ion trap"),
            }
        }
    }

    #[test]
    fn pass2_window_is_bounded_by_the_ladder_top_rung() {
        // The mechanism that ACTUALLY bounds this window. The old version of this
        // test asserted against PASS2_HALF_WIDTH_CAP_PPM and still passed with
        // the cap deleted, because the ladder was doing the work — so it could
        // not fail. Assert against the ladder itself.
        // ⚠ This bound is REACHABLE again as of 2026-08-29. While the width was
        // a rung it was structurally impossible to exceed the top rung, which is
        // why the old PASS2_HALF_WIDTH_CAP_PPM test could not fail. The width is
        // now unrounded, so |bias| + 5*MAD really can exceed 100 and really is
        // clamped here. |5| + 5*200 = 1005 ppm -> clamped to 100.
        let top = *MS1_TOLERANCE_LADDER_PPM.last().unwrap();
        let stats = MassErrorStats {
            bias_ppm: 5.0,
            mad_ppm: 200.0,
            n_psms: 500,
        };
        assert!(
            ms1_tolerance_requirement(5.0, 200.0) > top,
            "fixture must exceed the top rung"
        );
        let win = ms1_pass2_window(&stats);
        assert_eq!(win.high_ppm - win.low_ppm, 2.0 * top);
        assert_eq!(win.low_ppm, 5.0 - top);
        assert_eq!(win.high_ppm, 5.0 + top);
    }

    #[test]
    fn pass2_window_is_tighter_than_the_recommendation_on_a_well_behaved_instrument() {
        // |2| + 5*1 = 7 ppm, against a 10 ppm recommendation rung.
        let stats = MassErrorStats {
            bias_ppm: 2.0,
            mad_ppm: 1.0,
            n_psms: 500,
        };
        let win = ms1_pass2_window(&stats);
        assert!((win.low_ppm - -5.0).abs() < 1e-9, "got {}", win.low_ppm);
        assert!((win.high_ppm - 9.0).abs() < 1e-9, "got {}", win.high_ppm);
    }

    #[test]
    fn ms2_tolerance_none_on_empty_input() {
        assert!(compute_ms2_tolerance(&[]).is_none());
    }

    #[test]
    fn ms2_tolerance_symmetric_around_zero() {
        let vals = vec![-3.0, -1.0, 0.0, 1.0, 3.0, 8.0];
        let tol = compute_ms2_tolerance(&vals).unwrap();
        assert_eq!(tol.low_ppm, -tol.high_ppm);
        assert!(tol.tail_95_ppm > 0.0);
    }

    #[test]
    fn empty_clean_subset_returns_none() {
        assert!(compute_ms1_stats(&[]).is_none());
    }
}
