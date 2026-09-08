# PTM Stratification Design: Tiered Mod Recommendations

**Status:** Design note — not yet implemented. Evidence-vector table corrected
2026-08-24 against the committed artifacts; see the correction block in Step 1.
**Purpose:** Turn the mod-discovery peak list into a search-parameter recommendation.
Replaces the attempt to match other tools' prevalence percentages.

---

## Why this exists

Recon's open-search peak percentage is a discovery-rank statistic. Other tools report a
post-localization PSM fraction. These are different quantities. Matching them requires
either a second confirmation search per delta (too slow) or a calibration factor
(rejected — see JOURNAL 2026-08-24; the reference platforms disagree with each other by
1.37x on the same file).

Rank is different. Recon agrees with the platforms on rank. A tier is ordinal. It never
needs a calibrated magnitude. So the report moves to the axis the evidence supports.

**⚠ The rho values that stood here — MetaMorpheus 0.820 / 0.630 / 0.491, PTM-Shepherd
0.648 / 0.625 / 0.284 — are PRE-FIX and WITHDRAWN (2026-08-25).** On the fixed build,
against PTM-Shepherd reallyOpen: **serum 0.798, b1906 0.700, bcell 0.402** — the
per-file ordering INVERTED. The argument survives (two of three strongly significant,
one marginal); only the numbers moved. See NOTES "the rank-not-magnitude evidence
survives — but the PER-FILE story INVERTED".

This also matches what the commercial recon tools deliver. Mascot's first pass emits a
protein list. Byonic Preview emits a parameter file. Neither emits a prevalence table.

---

## Guiding decisions

1. **A tier is a search-parameter suggestion, not a quality verdict.** The output reads
   "for your next search," never "this sample is good." This is the line that keeps the
   digestion-score failure from repeating (see NOTES: the composite score misread normal
   serum biology as a mediocre digest, because it assumed cell culture).
2. **Tiers derive from the file's own distribution.** No absolute percentage thresholds.
   A cutoff like "above 4%" overfits at n=3 (same risk already recorded against the
   per-run ranking confidence flag).
3. **No tuning against the expert mod set.** Locked in NOTES: the tool is never compared
   to its author. If thresholds get adjusted until +57, Ox(M), and deamidation land in
   tier 1, that is the same unfalsifiable comparison in a new form. Inputs must be
   intrinsic to the run.
4. **Two tiers first, not five.** Resolution can increase when the file panel grows past
   three. Two tiers that hold beat five that need tuning.
5. **The unannotated tail is listed, never tiered.** Recon's ability to report deltas
   with no Unimod match is the capability the commercial tools lack. Suppressing it to
   make a clean recommendation would discard the differentiator.
6. **User knowledge enters as an override, never as a default.** An expert will want to
   add modifications they know are biologically real for their sample — acetyl protein
   N-term is the standard example. That belongs in the workflow, not in the tool's
   claims. The tier-2 listing already gives a knowledgeable user what they need to
   promote a peak by eye. If an explicit mechanism is wanted later, it is a documented
   flag, the same resolution as the parked `--sample-type` idea from the dropped
   digestion score. Promoting a peak because the author knows it is real would make the
   tier unfalsifiable — the exact comparison NOTES locks out.

---

## Step 1 — Define the evidence vector per peak

Every discovered peak carries these, all measurable from one file with no reference run:

| Field | Source | Already computed? |
|---|---|---|
| `rank` | position in the peak list by PSM count | yes |
| `count` / `count_pct` | PSM count and fraction | yes |
| `prominence` | existing prominence-based peak detection | yes |
| `unimod_match` | annotation status and name | yes |
| `mass_error_ppm` | `ppm_at_mz` on the peak's representative m/z | yes — but **only in `full-run/*.json`**; `representative_mz` is null on every peak in `nofixedmods/*.json`, which pre-dates the field |
| `folded_in` | whether fold-to-zero drained satellites into this peak | **no — corrected 2026-08-24.** See below |
| `satellite_fraction` | neighbouring peaks plausibly belonging to this population | partial — designed for the ranking-confidence flag, not built |
| `residue_degeneracy` | mass equals an amino acid residue mass | no — Phase 7D, deferred by evidence |

**Correction, 2026-08-24 (measured, replaces the "yes (fold log)" this table used to
claim for `folded_in`).** `folded_count` is **0 on every peak of all three files**, and
**0 histogram bins carry `folded: true`**. There is no per-peak fold provenance in any
committed artifact — only the aggregate `folding.folded_to_zero_count` and
`folding.fold_by_k`. Two separate reasons, both structural:

1. **Fold-to-zero runs at the PSM level, before the histogram is built.** It rewrites the
   PSM's delta to exactly 0.0, so those PSMs are already inside the Δ=0 peak and can never
   appear as a separate non-zero peak. **This is why Step 2's "exclusion must cover the fold
   target window" clause needs no separate test — it is satisfied by construction.**
2. `folded_count` is the *satellite* fold counter, and satellite folding is
   `enable_satellite_folding: false` — disabled by design, no conservation guard. See NOTES
   "Disabled-by-design". It is 0 because the feature is off, and it stays off.

**The real near-zero leak is a different one, and it is what the exclusion must handle.**
Roll-up into the zero peak uses `UNMODIFIED_ROLLUP_THRESHOLD_DA = 0.075`, but the summary
counts "near zero" at `NEAR_ZERO_THRESHOLD_DA = 0.1`. Peaks in that 0.075–0.100 Da band
survive as separately ranked peaks while being counted as unmodified:

```
serum:  psms_near_zero=5008   peaks |d|<0.1: (0.0, 4960)                                  unaccounted 48
bcell:  psms_near_zero=43263  peaks |d|<0.1: (0.0,42861) (-0.0788,226) (-0.0995,164)       unaccounted 12
b1906:  psms_near_zero=14480  peaks |d|<0.1: (0.0,14330) (-0.0997,75)                      unaccounted 75
```

Excluding at `|delta_mass| < NEAR_ZERO_THRESHOLD_DA` covers both this band and the fold
targets, and reuses the constant the rest of the pipeline already treats as unmodified.

**Source rule for Step 2.** Peaks and `representative_mz` come from `full-run/*.json`
(`analyze`, 2026-08-24, current build). Histogram and folding stats come from
`nofixedmods/*.json` (`discover`, 2026-07-24) — the only committed artifact that carries
them. The two agree bit-for-bit on `rank`, `delta_mass`, `count`, `count_pct`,
`unannotated`, `ambiguous`, and annotation names; they differ only in `prominence` (6 tail
peaks) and `representative_mz`. Neither differing field is read by any script. See NOTES
"Third confirmation, free: mod discovery is stable across builds".

Only `satellite_fraction` needs new work, and its design already exists. Nothing here
requires a second search.

---

## Step 2 — Define the within-file noise floor

The floor is a fraction of the **top-ranked non-zero delta peak** in the same file:

```
floor = X% of the count of the highest-counted non-zero delta peak
```

**Origin.** This generalizes a working heuristic from Mascot error-tolerant practice:
take the alkylation count, keep anything above 10% of it. The alkylation peak is a good
yardstick because sample preparation is the one thing about an unfamiliar file that is
not unfamiliar — the analyst performed the alkylation, so that peak represents a known,
essentially complete chemical event.

**Why not detect the alkylation peak specifically.** Recon takes no user input about
sample prep. Asking which alkylating agent was used would break the pure-reconnaissance
design. Identifying the alkylation peak automatically is possible in a normal
reduced/alkylated tryptic digest, but it fails on samples that were not alkylated and
adds a chemistry assumption the tool does not need. Using the top non-zero peak instead
requires no assumption at all. In a typical digest it selects the alkylation peak anyway.
In an unusual sample it selects whatever dominant chemistry is actually present, which is
the correct behaviour for a recon tool.

**Two implementation details that must be settled before this is a rule:**

1. **"Non-zero" must be defined explicitly.** The highest-counted delta in any open
   search is Δ≈0 (unmodified). Validation Gate 1 requires Δ≈0 to be dominant at ≥35%. So
   the reference peak is the top peak *after* excluding the Δ≈0 population and everything
   fold-to-zero drained into it. Excluding by "the zero bin" is not sufficient — the
   exclusion must cover the fold target window.
2. **X is not yet chosen.** Mascot practice suggests 10%, but that was applied to a
   localized per-residue count from a different engine. Recon reports an un-localized
   delta-bin count. The number must be tested, not inherited.

**Test before adopting.** Compute the floor at several candidate X values (5, 10, 15, 20)
on all three files from committed JSONs. No re-run needed. Check what each floor admits
and excludes per file, and whether the resulting tier-1 set is stable.

**Invariant to assert (RESTATED 2026-08-26 — two earlier wordings are superseded
below):** the floor must sit above the ±1/±2 Da quantization carpet, **and the floor
governs ONLY the peaks the abundance path decides.** That carpet is characterized as a
peak-detection artifact of roughly 1% of PSMs (JOURNAL 2026-07-17, closed as a
dead-end). If the floor lands below it, the tool reports detector noise as a
recommendation. Assert this in code — do not check it by reading the output.

**The scoping is the point, and it follows from the routing rule.** Recon decides a
residue-specific mod by PRESENCE — Fisher exact on its acceptor residues — not by
amount. Such a peak never faces the floor, so the floor makes no claim about it. The
floor's only job is to separate real signal from carpet for peaks that have no residue
to test: unspecific acceptors, and masses the curated list cannot name. The invariant is
scoped to exactly those.

**⚠ TWO SUPERSEDED WORDINGS — do not reintroduce either.**
1. *Original:* "the floor must sit above the ±1/±2 Da carpet", unscoped. **Can never
   pass.** The tallest peak inside the ±1/±2 Da region is **Deamidated** on all three
   files (184 / 760 / 228) — real chemistry in the carpet's mass region. Satisfying it
   literally would require excluding deamidation.
2. *2026-08-25 patch:* "carpet EXCLUDES annotated real chemistry inside the region."
   Right outcome, wrong reasoning — it carved deamidation out by hand. Under the
   restatement Deamidated is not an exception at all: it is residue-specific (N/Q), the
   statistics path decides it, and the floor has no jurisdiction over it. The carve-out
   disappears instead of being maintained.
   That patch's margins (+115.8 / +113.6 / +75.9 PSM at X≥15%) were measured under the
   patched definition, over a different peak set than the restatement scopes. **They are
   NOT carried forward. Re-measure when the assert is written.**

**Known risk, to measure not assume.** Recon's +57 magnitude runs about 0.4x the
platforms' (JOURNAL 2026-07-24). If the reference peak's count is systematically low,
the floor is proportionally low too. Numerator and denominator both come from the same
open search, so the effect may cancel. That is an empirical question. The three-file test
above answers it.

---

## Step 3 — Assign tiers

**Tier 1 — recommended.** Peak is above the floor, has a Unimod annotation, and is not
explained as a residue-mass or isotope artifact. Report with the suggested search role:

**2026-08-25 — the "not an isotope artifact" clause is the load-bearing one.** It is
what promotes +57 (resolved to Carbamidomethyl by Cys enrichment, 2.85/9.98/9.41x, with
Gly at background) and what demotes +58/+59. It is not new scope; it is this clause.
Two candidate implementations are being prototyped in parallel — curated annotation
(`reference-notes/metaMorpheusMods/`) vs statistical support (Fisher/OR/BH + power
guard). See PLAN's status block.

- Dominant, single-site chemistry (e.g. +57.0215) → suggest **fixed**.
- Present but sub-dominant, or multi-site → suggest **variable**.

The fixed-versus-variable split needs its own rule. Starting proposal: fixed if the
peak's count approaches the count of the residue's total occurrence in confident PSMs;
variable otherwise. This needs testing before it is a claim.

**Tier 2 — present, your call.** Above the floor, but one of: no Unimod annotation,
ambiguous annotation (multiple candidates within tolerance), or flagged as possible
artifact. Listed with the reason for the demotion.

**Unranked tail.** Everything detected below the floor. Listed, counted, explicitly not
recommended. This is where a new-species or new-tissue signal would appear, so it is
never suppressed.

---

## Step 4 — Report shape

The peak percentage stops being a headline. It becomes an evidence line under a
recommendation:

```
RECOMMENDED SEARCH MODIFICATIONS

  Fixed
    Carbamidomethyl (C)   +57.0215   rank 2   4.50% of PSMs

  Variable
    Oxidation (M)         +15.9949   rank 3   2.7% of PSMs

PRESENT — YOUR CALL
    +43.0058              rank 5   1.2%   Carbamyl / Trimethyl (ambiguous)
    +28.0313              rank 8   0.6%   Formyl / Dimethyl (ambiguous)

DETECTED BELOW RECOMMENDATION FLOOR — 34 peaks, 12 unannotated
    (full list in JSON)

NOTE: percentages are open-search delta-bin fractions, not post-localization
PSM prevalence. They rank modifications; they do not measure site occupancy.
Confirm with a targeted search using the parameters above.
```

The NOTE is the currency statement. It carries the whole undercount explanation in two
sentences and needs no calibration factor.

---

## Step 5 — Validation

The tier is a claim about ordering, so it is validated against ordering.

1. **Rank agreement, already measured.** Spearman rho against PTM-Shepherd,
   MetaMorpheus, and Mascot is in `testing/recon-output/comparison/`. Tier assignment
   must not invert any pair the platforms agree on. This is a gate, computable from
   committed artifacts, no re-run.
2. **Tier stability across the three files.** The same chemistry should not land in tier
   1 on one file and the unranked tail on another without a stated reason (different
   sample, different instrument). Serum is a biofluid; bcell and b1906 are not. Expect
   real differences; require them to be explainable.
3. **Floor sanity.** Assert the floor sits above the carpet (Step 2 invariant), on all
   three files.
4. **Negative control.** The +57 residue-mass degeneracy case (Phase 8 Gate 3, serum)
   was proven to be over-alkylation, not added glycine, by a flanking check with 0 of 26
   peptides showing Gly context. A tier that recommends adding Gly as a modification on
   that file is a failure. This is a concrete, already-adjudicated test case.

**Acceptance is numeric.** Print the tier assignment for all three files against the
rank-agreement gate. Do not substitute a pass mark.

---

## What this does not do

- It does not measure site occupancy. The report says so.
- It does not localize. The no-per-residue-localization lock stands.
- It does not re-search. The measure-once / apply-once / report-both lock stands. A tier
  is derived from Search 1's output, not from a confirmation pass.
- It does not judge sample quality.

---

## Relation to the open queue

- Folds into JOURNAL 2026-08-24 item 4 (decompose the gap in the report). The
  decomposition and the tier are the same report section.
- Independent of item 6 (wide x fully tryptic x subset FASTA timing). If that
  measurement comes back cheap, a confirmation pass could later promote tier 2 peaks
  into tier 1 with a real prevalence number. The tier design does not depend on it.
- Absorbs the July "suggested fixed/variable mods block" suggestion (JOURNAL 2026-07-24
  Q5), which is the same idea with the percentage demoted from a claim to an input.
- Supersedes the per-run ranking confidence flag as the immediate consumer of
  `satellite_fraction`. The flag stays banked as beta.

---

## What is borrowed versus original

| Element | Origin |
|---|---|
| Recon pass emits parameters, not prevalence | Byonic Preview's stated design; Protein Metrics says outright it is not a substitute for a full search |
| Restricting recommendations to evidence above a significance floor | Mascot ET's pass-1 selection rule, applied to modifications rather than proteins |
| Floor as a percentage of a reference peak | Mascot error-tolerant working practice (keep anything above ~10% of the alkylation count) |
| Using the top non-zero peak as the reference instead of the alkylation peak | Original — removes the sample-prep assumption and needs no user input |
| Reporting the unannotated tail | Original, and the differentiator; both vendors are limited to named modifications (Mascot to Unimod, Preview to ~60 common mods) |
| Refusing to score sample quality | Carried from the dropped digestion score (NOTES, known permanent limitations) |
