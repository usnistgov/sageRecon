# MS1 Tolerance Recommendation — Rationale and Evidence

**Status:** analysis complete, code change deferred to ship-track step 3.
**Date:** 2026-08-24. **Files:** serum (909c), bcell (B.naive), b1906.

This note exists because the MS1 tolerance recommendation is one of recon's three
headline outputs, and a reader of the paper must be able to see exactly how the
number is derived, what it rests on, and where it is a judgment call rather than
a measurement. Every claim below is tagged **[MEASURED]**, **[CITED]**,
**[INFERRED]**, or **[CHOICE]**. Do not promote an INFERRED item to a MEASURED
one in the write-up without doing the measurement named in it.

---

## 1. The question the number answers

*"What precursor mass tolerance should I set in my real search?"*

This matters because it fixes the shape of a good answer. The user is choosing a
**search-engine setting**, not reading an instrument-performance metric. Their
answer set is effectively quantized — 5, 10, 20, 50 ppm. Nobody types 4.087.

A recommendation is therefore correct if it lands the user in the right bucket,
and any precision beyond that is wasted. This single observation resolves most of
what follows, including the clean-subset PSM floor question (§7).

Note this is a *different* question from "what is this instrument's mass error?"
Recon answers both, from one measurement, and NOTES locks the two apart ("Two
numbers from one measurement, opposite ends"). The bias figure is a genuine
instrument-performance readout and is useful on its own. The confusion documented
here comes from using the second question's machinery to answer the first.

---

## 2. What recon currently emits **[MEASURED]**

From `testing/recon-output/full-run/*.json`, `ms1_calibration` block, current
build, alkylation-agnostic open search:

| File | measured bias (ppm) | MAD (ppm) | user recommendation (ppm) |
|---|---|---|---|
| serum | +2.428 | 0.485 | +1.458 to +4.087 |
| bcell | +0.643 | 0.400 | −0.157 to +2.176 |
| b1906 | +0.776 | 0.485 | −0.195 to +3.385 |

Implemented formula (`calibration.rs::ms1_user_recommendation`):
`low = bias − 2×MAD`, `high = bias + p95(|deviations|)`.

---

## 3. Three independent lines of evidence that this is too tight

**(a) Field practice. [CITED]** 10 ppm is the standard Orbitrap MS1 default, with
15–20 ppm also routine. One optimization study on Orbitrap data found 15 ppm
optimal for precursor and product ions. Nothing in ordinary practice sits near
2 ppm. See [Byonic mass tolerance
guidance](https://support.proteinmetrics.com/hc/en-us/articles/19343705322900-Byonic-Mass-Tolerance-Settings)
and the [optimal-tolerance
study](https://www.researchgate.net/publication/303439131_Mass_measurement_accuracy_of_the_Orbitrap_in_intact_proteome_analysis_Optimal_mass_tolerance_to_interpret_Orbitrap_mass_spectra).

**(b) Our own reference tools, on our own three files. [MEASURED]** MSFragger ran
these exact runs at `precursor_true_tolerance = 20` ppm and returned 4,366–53,029
confident PSMs per file (`testing/reference-data/msfragger/strictTryp/`). Its own
calibration measured the underlying error at ≈1 ppm MAD. So a tool whose entire
design goal is precise mass handling chose a search window ~20× its measured
scatter, and that was correct practice, not sloppiness. This is the most directly
comparable evidence available because it is the same instrument data.

**(c) Wide tolerances can outperform narrow ones outright. [CITED]** Wilmarth's
empirical comparison on real Orbitrap data found PSM yield *rising* as the
precursor window widened: 81,655 PSMs at 10 ppm, 84,068 at 20 ppm, 86,216 at
50 ppm, 108,826 at 1.25 Da. His mechanism: narrow tolerances "do not reject noise
[they] select different noise" — a wide window spreads incorrect matches across
the whole window and leaves the 0-Da region cleaner, while a narrow window
concentrates noise under the correct peak and is additionally fragile to
calibration drift and isotope/deamidation edge cases. See [Go big or go
home?](https://pwilmart.github.io/blog/2021/04/22/Parent-ion-tolerance).

We do not adopt Wilmarth's 1.25 Da conclusion — recon's mission is to report what
the data supports, not to push a search-strategy position — but his direction of
error is the same as (a) and (b), from a third independent angle, and it means the
cost of over-recommending is low while the cost of under-recommending is lost IDs.

---

## 4. Mechanism — why the implemented formula reads low **[MEASURED, 2026-08-24]**

The recommendation is computed over the **clean subset**: near-zero-delta,
rank-1, target-only, `q < 0.01`, then optionally trimmed to the top 60% by
hyperscore. That population is *by construction* the best-behaved PSMs in the
file — unmodified, confidently identified, high-scoring.

The population the user's real search must accommodate is wider: modified
peptides, low-intensity precursors, extreme m/z, lower-scoring but still-correct
IDs. Mass error in those classes is larger.

**Now measured, and confirmed on all three files** via
`testing/scripts/ms1_bias_sign_check.py`, comparing the open search's clean
subset against *all* confident PSMs in the matched closed reference search
(where mods are explicit variable mods, so `precursor_ppm` is instrument error
for every PSM):

| File | clean subset MAD | closed-search all-PSM MAD | ratio |
|---|---|---|---|
| serum | 0.4557 ppm | 0.5811 ppm | **1.28×** |
| bcell | 0.6407 ppm | 0.7407 ppm | **1.16×** |
| b1906 | 0.6413 ppm | 0.7969 ppm | **1.24×** |

The hyperscore trim is a small, consistent contributor within that: it narrows
scatter by 4.8–6.3% on its own.

**Correction to a previously-stated test.** An earlier version of this note
proposed "compare clean-subset MAD vs all-confident-PSM MAD" without specifying
the search. That version is **ill-posed for an open search**: a modified
peptide's `precursor_ppm` reflects its modification mass, not instrument error
(the all-PSM median is ≈14,000 ppm — NOTES "Precursor ppm in open search is
huge/garbage"). The comparison is only meaningful against a **closed** search,
which is what the table above uses.

---

## 5. A second, separate defect: implementation drifted from its own design **[MEASURED]**

`recon-calibration-design-v2.md` §Step 3 specifies a MetaMorpheus-derived
formula: `new_precursor = median + 3 × IQR`. The implementation instead uses
`bias + p95(|deviations|)`. These are not the same and the implementation is
consistently narrower:

| File | implemented upper extension | design ≈ 3×IQR | implemented as % of design |
|---|---|---|---|
| serum | 1.659 ppm | ≈2.91 ppm | 57% |
| bcell | 1.533 ppm | ≈3.04 ppm | 64% |
| b1906 | 2.609 ppm | ≈2.91 ppm | 90% |

**Caveat on this table [INFERRED]:** actual IQR was not computed; the design
column uses `IQR ≈ 2 × MAD`, exact only for a normal distribution. NOTES already
flags this conversion as a rough sanity check, not an identity. The qualitative
finding — implementation narrower than design, by an inconsistent margin — holds
regardless, but do not quote the percentages as precise.

So there are two independent problems, and they compound:
1. The implemented formula is narrower than the design it claims to implement.
2. The design's own formula, even implemented faithfully, is still tighter than
   §3's evidence supports.

The asymmetric `low = bias − 2×MAD` side is an implementation addition with no
counterpart in the design doc. NOTES locks asymmetry as intentional and the
reasoning is sound (a biased instrument folded into a symmetric window clips one
side), so asymmetry itself is not in question here — only the width.

---

## 6. Rule — quantize to buckets **[CHOICE]** — ✅ IMPLEMENTED 2026-08-28

**Status: SHIPPED.** `calibration::quantize_ms1_tolerance`, with
`MS1_TOLERANCE_LADDER_PPM` and `MS1_TOLERANCE_MAD_K` as named constants.
Report schema 1.6.0 carries `user_recommendation_tolerance_ppm`. The acceptance
gate is `ms1_calibration_integration::all_three_files_land_on_the_10_ppm_rung`,
which asserts the requirement against the values predicted in §12 BELOW rather
than against whatever the code emits — measured 4.844 / 3.602 / 3.862 ppm, all on
the 10 ppm rung, matching this note's pre-computed 4.84 / 3.60 / 3.86.
The k-insensitivity claim is also asserted rather than left as prose.

```
recommended_tolerance_ppm = smallest bucket in {10, 20, 50, 100}
                            that is >= |bias| + 5 x MAD
```

Applied to the three files **[MEASURED]**:

| File | \|bias\| + 5×MAD | bucket |
|---|---|---|
| serum | 4.85 ppm | **10 ppm** |
| bcell | 2.64 ppm | **10 ppm** |
| b1906 | 3.20 ppm | **10 ppm** |

All three land on 10 ppm, matching both field default (§3a) and expert
expectation. A genuinely drifted instrument moves up a bucket; a TOF at 50–80 ppm
lands at 100 without special-casing.

**Both constants are judgment calls, and the paper must say so:**
- `k = 5` on MAD (≈2.5×IQR) sits below the design doc's 3×IQR. It is not derived
  from a loss function. It is chosen so that the *bucket* is robust, which is a
  weaker requirement than getting the width right — see below.
- The `{10, 20, 50, 100}` ladder and its 10 ppm floor come from field practice
  (§3a), not from our data. Recommending below 10 ppm buys nothing per §3c.

**Why the arbitrariness is tolerable here, and this is the actual argument:**
bucket quantization is far less sensitive to `k` than a continuous number is. On
all three files, any `k` from about 3 to 15 yields the same 10 ppm bucket. The
recommendation only changes when the underlying data changes a lot — which is the
behavior you want from a recommendation. A continuous formula has no such
tolerance and forces a precision claim the measurement cannot support.

**Bias is still reported separately, and must be.** serum's +2.43 ppm systematic
offset is real, useful, and independently corroborated (MSFragger +2.50,
MetaMorpheus +2.351 — NOTES MSFragger ground-truth entry). Bucketing the
*tolerance* does not discard the *bias*; they are the two numbers NOTES already
locks apart.

---

## 7. Consequence — the clean-subset PSM floor is not load-bearing

The 200-PSM floor in `calibration.rs::hyperscore_guard_would_apply` was flagged
in PLAN step 1 as unexplained. Bootstrap sensitivity analysis
(`testing/scripts/psm_count_sensitivity.py`, outputs in
`testing/recon-output/psm-sensitivity/`) **[MEASURED]**:

| File | full clean N | smallest N, 90% spread ≤1.0 ppm | ≤0.4 ppm | ≤0.2 ppm |
|---|---|---|---|---|
| serum | 3,764 | 16 | 75 | 200 |
| bcell | 32,133 | 16 | 100 | 300 |
| b1906 | 10,942 | 16 | 75 | 300 |

Under the bucket rule, moving serum off the 10 ppm bucket requires `|bias| + 5×MAD`
to cross 10 — a shift of >5 ppm. The bootstrap shows the estimate wobbling by
tenths of a ppm at N=50. **The bucket cannot flip from sampling noise at any N in
this grid.** So the floor does not need to guarantee median precision, and the
elaborate justification this analysis was originally meant to produce is not
needed.

What the 200 actually gates **[MEASURED, from code]**: whether the optional
60%-by-hyperscore trim runs at all. Its real job is "don't throw away 40% of an
already-small subset." For that job 200 is defensible and conservative;
MetaMorpheus's comparable engineered floors are ≥16 PSMs / ≥40 MS1 / ≥80 MS2
datapoints for a *harder* task (per-scan drift correction, not a summary
statistic) — see `metamorpheus-mass-error-calibration.md`.

**Recommendation: keep 200, unchanged, now documented rather than unexplained.**

One wrinkle recorded so it is not rediscovered as a bug **[MEASURED, from code]**:
the 200 is checked *before* the 60% trim, so a subset landing exactly at the
threshold reports on 120 PSMs. Under a continuous-precision framing that gap would
matter; under bucket quantization it does not. It has never been live on these
three files (clean subsets are 3,764–32,133; the guard fired with thousands to
spare). Leave it alone.

---

## 8. Corollary — the Pass 2 window has the same root cause **[INFERRED]**

`ms1_pass2_window` uses `bias ± 3×MAD`, giving ±1.2–1.5 ppm on these files. For a
roughly normal distribution 3×MAD ≈ 2σ, a window that clips ~5% of true peptides
before any of §4's wider-population effect is considered. It is built on the same
understated clean-subset scatter.

This is flagged, not fixed. Pass 2 is not yet wired into `run` (ship-track step 3),
so there is no end-to-end path to test a change against. The fix belongs with that
work. Note the ±100 ppm cap is a separate, already-locked runtime backstop and is
not implicated.

---

## 9. Decision and status

**Documented now, code changed in step 3.** Rationale: step 1's checkpoint is a
frozen ground-truth measurement set, and changing report-generation code mid-step
would move numbers that were just frozen. The recommendation formula, the Pass 2
window, and the analyzer-aware tolerance work all share a root cause and one
consumer, so they should change together, in step 3, where they can be tested
end-to-end.

Nothing in the currently frozen reports is invalidated by this note. The bias and
MAD figures are unaffected — only the derived recommendation is in question, and
it is reported, not used, in the current build.

**Assumptions ledger for the paper.** Carry these forward as stated limitations:
1. ~~Clean-subset scatter understating full-population scatter is inferred~~ —
   **now MEASURED** (§4): 1.16–1.28× on the three files, against matched closed
   searches. Promoted 2026-08-24.
2. `k = 5` and the bucket ladder are **choices** informed by field practice, not
   derived from our data (§6).
3. The design-vs-implementation percentages rest on `IQR ≈ 2×MAD` (§5).
4. All three test files are well-calibrated Orbitraps. No TOF or ion-trap data was
   available. The bucket ladder's behavior above 10 ppm is **untested on real
   data** — reasoned from the ladder's construction only.

---

## 10. Superseding finding (2026-08-24): the inputs were also wrong

After this note was first written, `bias_ppm` and `spread_mad_ppm` were found to be
computed from Sage's **absolute** `precursor_ppm` column — see NOTES "⚠ BUG —
MS1/MS2 bias was a median of |error|". Both the centre and the width of the
recommendation therefore came from a distribution folded about zero. Corrected
figures:

| File | MAD as used here | true signed MAD | understated by |
|---|---|---|---|
| serum | 0.485 | 0.485 | 0% (latent) |
| bcell | 0.400 | 0.673 | 37% |
| b1906 | 0.485 | 0.684 | 28% |

**This strengthens rather than weakens the case in §6.** Re-running the bucket
rule on corrected inputs (`|true bias| + 5 × true MAD`): serum 4.84, bcell 3.60,
b1906 3.86 ppm — **all three still land on the 10 ppm bucket.** A continuous
recommendation would have shifted materially on two of three files; the bucket did
not move at all. That is the robustness argument demonstrated rather than asserted,
and it is worth stating in the paper exactly that way.

The bug does *not* let the current formula off: it was too tight on inputs that
were themselves understated, so the two defects compounded in the same direction.

Note the bucket's insensitivity applies to the **tolerance recommendation only**.
The reported **bias** and the **bias-centred Pass 2 window** are fully exposed to
the bug and must be fixed, not bucketed around.
