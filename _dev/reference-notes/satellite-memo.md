# Decision memo — isotope satellites in step 2 tier stratification
Date: 2026-08-24. Written without the raw Sage TSVs or the upstream clones to hand.
Scope: design + recommendation only. `enable_satellite_folding` NOT changed. No folding code written.
Incorporates the Byonic Preview addendum and `reference-notes/byonic-preview-methodology.md`.

## 0. Recommendation

**Keep `enable_satellite_folding: false`. Handle satellites in tier logic as flag-and-demote
(option B). Pursue option C via a Sage config change as the cheaper structural fix — it is NOT
closed by the locks, which I think the addendum concedes too early.**

The recommendation **survives the ghost hypothesis, and becomes load-bearing under it.** That is
the sharpest result in this memo, so it leads.

## 1. The headline measurement: forest and satellite are orthogonal, and both are required

I simulated each treatment in scratchpad (measurement only, nothing in `recon-tool`) and re-ran
`tier_gates.py`'s own gate-1 function. Forest removal is a **proxy** for the decoy criterion —
the real test needs `results.sage.tsv` and is not mine to run.

**⚠ THIS TABLE IS PRE-FIX AND SUPERSEDED (2026-08-25). Kept for the record; do not cite.**
It was computed before the peak-assignment fix, so its baseline row disagrees with the
committed `tier-gates-2026-08-25.txt`.

| variant | X=5 | X=10 | X=15 | X=20 |
|---|---|---|---|---|
| baseline (reproduced exactly) | 47 | 25 | 31 | 16 |
| satellite fold (simulated) | 17 | 20 | 12 | 7 |
| satellite flag-and-demote | 23 | 12 | 15 | 7 |
| forest removed, satellites left | 32 | 11 | 14 | 7 |
| **forest removed + satellite demote** | 10 | **0 PASS** | 1 | **0 PASS** |
| forest removed (Deamidated kept) + demote | 10 | **0 PASS** | 1 | **0 PASS** |

**Regenerated on the fixed build, 2026-08-25 — this is the live table:**

| variant | X=5 | X=10 | X=15 | X=20 |
|---|---|---|---|---|
| baseline (fixed build) | 55 | 20 | 15 | 8 |
| satellite fold (simulated) | 27 | 2 | 2 | **0 PASS** |
| **satellite flag-and-demote** | 48 | 10 | 2 | **0 PASS** |
| forest removed, satellites left | 37 | 11 | 14 | 7 |
| **forest removed + satellite demote** | 24 | **0 PASS** | 1 | **0 PASS** |
| forest removed (Deamidated kept) + demote | 24 | 1 | 2 | **0 PASS** |

**❌ WITHDRAWN — "neither treatment alone passes at any X."** That was true on the
pre-fix peaks and is FALSE on the fixed build. **Satellite flag-and-demote alone passes
at X=20%** (gate 1 0/0/0, gate 4 0), and reaches 2 violations at X=15%. Forest removal
is no longer required for a pass; it buys a smaller X (10%), not a pass. The
pre-committed "smallest X that passes" rule selects **X=10% if the forest can also be
removed, X=20% if only the satellite is handled** — and the forest has no available
instrument, since the ghost hypothesis was refuted.

**Also superseded: option C2.** `isotope_errors: [-1, 2]` was tested 2026-08-25 and is
CLOSED-NEGATIVE — it destroys +57 and fabricates a Propionyl peak. The memo's
"test C2 before building any tier-logic work" recommendation is spent. Option B stands.

**Strengthened, not weakened: the satellite identification.** Two lines of evidence the
memo did not have — the +58/+59 bands carry the same Cys enrichment as +57
(2.88/9.99/8.96 vs 2.85/9.98/9.41), and their peptides overlap the +57 band at 6.2x /
3.8x / 8.2x above chance. See NOTES "+58/+59 are the +57 satellite".

The two failure modes are independent and need independent instruments:
- The forest is **incorrect matches**. Target/decoy enrichment is the right instrument.
- The satellite is a **correct match with a misassigned precursor**. Decoy enrichment is blind to
  it *by construction*.

NOTES Phase 7C already established the mechanism for the second half, on the neighbouring
discriminator: "**No hyperscore gate** — Monoisotope misassignments score as well as unmodified
(fragments match perfectly, error is on precursor only). Hyperscore is not a discriminator."
The identical argument transfers to decoys. The peptide is real, the fragments match, the PSM is
target-side and high-scoring. Only the precursor is wrong.

Confirmed numerically — after forest removal the satellite does not merely survive, it is large:

| file | +58 after forest removal | % of top surviving peak |
|---|---|---|
| serum | 350 | 31.1% |
| bcell | 938 | 28.3% |
| b1906 | 248 | 19.8% |

It sits above the floor for any X up to ~20%. A decoy criterion promotes it, it does not remove it.

**So, answering the addendum's either/or directly:** it is not "the decoy criterion removes the
forest and leaves the satellite, making a mechanism unnecessary." It is the opposite — removing
the forest leaves the satellite as *the* remaining blocker. What becomes unnecessary is **folding**,
not satellite handling. Folding was never the thing that was needed.

**Item 8, stated plainly as asked.** The recommendation does not depend on the forest being
chemistry. It is unconditional in that direction. If anything it is *strengthened* by the ghost
hypothesis. What IS conditional is the claim that X=10% is selectable — that rests on the forest
proxy and must ship labelled as untested until the decoy test runs.

### Why the count is not monotone in X — resolved

The single residual at X=15% is serum `+31.9907 Dioxidation` (180) placed above
`+209.0205 CarbamidomethylDTT` (138); both references say DTT is larger (PTM-Shepherd 274 vs 314,
Mascot 171 vs 199). Serum floors are 112.5 / 168.8 / 225.0 at X = 10 / 15 / 20. At X=10 both peaks
are above the floor, so gate 1 makes no comparison. At X=15 the floor falls **between** 180 and 138,
creating one cross-floor pair. At X=20 both are below it, and the comparison vanishes again.

**Gate 1 only fires when the floor happens to land between two near-tied peaks.** The violation
count is therefore not a smooth function of X by construction, and "not monotone in X" is a
property of the gate, not evidence of instability in the method. Worth recording in the design.
Separately, this particular pair is the known serum ranking strain already on file (Spearman
ρ=0.284; the deferred `ranking_confidence` entry), not an isotope artifact.

## 2. Option C — the addendum concedes it too early

The addendum expects the answer "the reference tool solves this upstream and we cannot." That is
right for Preview's *method* and wrong for the *architecture*. Split it in two.

**C1 — our own spectrum preprocessing (Preview's actual method): CLOSED.** Preview extracts the
300 most intense peaks per spectrum and downweights isotope peaks. Interposing that between mzML
and Sage means rewriting spectra and feeding Sage modified input — re-implementing part of a
search engine. `mzml.rs` reads spectra today, but only to *measure* (MS1 intensity, polymer,
oxonium). Writing them is a different thing. Correctly closed; belongs in the write-up's
limitations.

**C2 — Sage's own isotope-error handling: NOT CLOSED, and it is cheap.** From
`reference-notes/sage-online-docs.md`, Sage's documentation of the parameter:

> "## Isotope Errors — This parameter essentially runs a multi-notch or mass offset search for
> each integer value in the ( left .. right ) range by multipyling [sic] the integer value by the
> mass difference of one C13 neutron. This can account for incorrect assignment or sequening [sic]
> of the monoisotopic peak."

That is the exact problem, named by the tool we already drive. Recon writes Sage's config
(`main.rs` — search settings come from a template). **Changing a config value recon already
authors does not touch "Sage is a subprocess, not a library."** That lock forbids embedding Sage
as a Rust dependency; it does not forbid configuring the subprocess.

Two things must be checked before calling C2 available, and one is a correction to NOTES.

**Sage's `deisotope` is the wrong axis — it will not help.** Same source:

> "If deisotope is turned on, Sage will attempt to deisotope and deconvolute the charge state of
> fragment ions in an MS2 spectra."

Fragment ions, not precursors. Our `deisotope: true` does nothing for this problem.

**⚠ A NOTES claim needs correcting.** Phase 7C root cause #2 reads: "**Isotope correction was a
no-op on open-search data** — `isotope_error` column is always 0 in Da-tolerance open search."
I verified the observation from a committed artifact rather than the summary —
`testing/recon-output/nofixedmods/*.json`, `isotope_error_view`:

```
serum : [{isotope_error: 0, count: 66771}]     single bin
bcell : [{isotope_error: 0, count: 172081}]    single bin
b1906 : [{isotope_error: 0, count: 80627}]     single bin
```

100% at 0 on all three files. **But our open-search config sets `"isotope_errors": [0, 0]`** — a
notch range containing only zero. A column that is always 0 is exactly what that config predicts.
The observation is therefore consistent with the config and does **not** establish the stated
conclusion that the mechanism is structurally inert in a Da-tolerance open search. Those are two
different claims and NOTES currently runs them together.

Whether `isotope_errors: [-1, 2]` does anything useful when the precursor window is already
`da: [-500, +100]` is **untested and unknown**. It is a one-line config change and one search.
If the notches are subsumed by the wide window it is inert and C2 closes honestly; if Sage
re-reports the corrected notch, +58 and +59 collapse at the search layer and both option A and
option B become unnecessary for this class.

**I recommend testing C2 before building any tier-logic work.** It is the cheapest experiment in
this memo and it can retire the whole question. It needs the raw inputs and a fresh
search run, and it is a change-regenerate: explicit request plus an enumerated
downstream-impact trace.

**Incidental bug, not fixed:** the `pct` field in `isotope_error_view` reads 433.97 / 234.04 /
287.90 percent. The counts are pre-filter PSMs; the denominator is post-filter `total_psms`
(66771 / 15386 = 4.3397). Counts are fine, `pct` is meaningless. Flagging, not touching.

## 3. New finding the prompt did not include: +59 is also a satellite, and it is annotated

`+59.02x`, annotated **AEC-MAEC**, sits on the M+2 rung of `+57` on all three files.
Measured from `testing/recon-output/full-run/*.json`:

| file | +57 | +58 (M+1) | dev from k=1 | +59 (M+2) | dev from k=2 |
|---|---|---|---|---|---|
| serum | 1125 | 350 | -1.1 mDa | 73 | -2.8 mDa |
| bcell | 3311 | 938 | -1.1 mDa | 292 | -0.7 mDa |
| b1906 | 1253 | 248 | -1.5 mDa | 75 | -0.9 mDa |

k=2 tolerance is 16.5 mDa, so all are far inside. This is worse than `+58`: `+58` is UNANNOTATED
and lands in tier 2, whereas `+59` carries a Unimod name and can reach **tier 1** — a named search
parameter recommended to a user. Any treatment must cover k=2, not just k=1.

## 4. The averagine invariant — tested as instructed, and it fails

It should not be built. Three independent results:

**(a) The ratio does not scale with mass.** Across all 49 (parent, M+1-candidate) pairs within the
k=1 window on the three files, Pearson r between count ratio and the parent's `representative_mz`
is **-0.196**. An isotope-envelope intensity ratio rises with carbon count, hence with mass. This
is flat-to-slightly-negative.

**(b) The ratio is out of range for an envelope.** Range 0.099 to 8.606; six of 49 pairs exceed
1.0. One is serum `+14.9840 -> +15.9951` at ratio 8.6, where the "satellite" is real Oxidation
(284 PSMs) and the "parent" is a 33-PSM peak.

**(c) Decisive — the same samples give different ratios under different pipelines.**
+58/+57 count ratio, recon vs PTM-Shepherd on identical raw files:

| file | recon | PTM-Shepherd | factor |
|---|---|---|---|
| serum | 0.311 | 0.086 | 3.6x |
| bcell | 0.283 | 0.041 | 6.9x |
| b1906 | 0.198 | 0.018 | 11.0x |

Same chemistry, same peptides. If this quantity were the isotope envelope the two would agree.
They differ by up to 11x. **The PSM-count ratio is a property of the search pipeline's precursor
handling, not of the peptide's isotope distribution.** An averagine prediction (~0.7-0.8 near
1.5 kDa) matches neither column. A two-sided invariant built on it would fail on correct data for
reasons unrelated to whether a peak is a satellite.

This is the point the prompt flagged as possibly sinking the idea. It sinks it. It is also
independent evidence for C2: the gap between the columns is precursor handling, and precursor
handling is a search-layer setting.

## 5. Where the +58 excess comes from

From committed configs, not recollection:

- Recon, `testing/configs/open-search-serum.json`: `"isotope_errors": [0, 0]`, `"deisotope": true`.
- Reference, `testing/reference-data/ptm-shepherd/reallyOpen/fragger.params` (MSFragger 4.3,
  FragPipe 23.1, per `log_2026-07-21_15-23-17.txt`): `precursor_mass_mode = corrected`,
  `calibrate_mass = 2`, `deisotope = 1`, `isotope_error = 0`.

The reference pipeline corrects the precursor monoisotopic assignment upstream of its histogram.
Recon does not. That is a search-layer gap — which is why C2 is the structurally correct fix and
folding is a downstream patch on an upstream cause.

## 6. External findings — quoted, flagged for verification

**Every quote needs verification against the pinned snapshot before Methods text.** One already
shows drift.

**Byonic Preview** — the addendum's own source. I did not re-verify it; `byonic-preview-methodology.md`
carries its own warning that it is a web read, the PDF is not vendored, and it is a digest rather
than a source. My independent web search for Byonic Preview isotope handling found only precursor
isotope *settings* for low-resolution MS1 ("Too high (narrow)" / "Too high (wide)"), nothing on
delta-mass histogram satellite handling. That is **no finding, not a negative finding** — it does
not corroborate the note. **Vendor the PDF before citing.**

**PTM-Shepherd** — annotates isotope peaks, does not fold them.
- Paper: "Detected peaks are iteratively annotated using entries from the Unimod (retrieved: 2 Oct
  2019) modification database (including single residue insertions and deletions and isotopic
  peaks), supplemented with a user-specified list of mass shifts."
- Paper: "...we find mass shifts of mono-methylation, mono-oxidation, and di-oxidation within the
  top 10 mass shifts (excluding isotopic peaks) for every dataset"
- README, `isotope_error`: "takes a / separated list of isotope states that modify mass offsets to
  check for combinations of a mass shift and isotopic peaks (e.g. 0/1/2). Default is unused."
- **Confirmed against our own committed output**, which outranks the web copies:
  `reallyOpen/global.profile.tsv` carries composite rows — apex 16.99860 →
  `mapped_mass_1 = First isotopic peak`, `mapped_mass_2 = Oxidation or Hydroxylation`;
  apex 59.00160 → `First isotopic peak` + `Iodoacetic acid derivative`. Each keeps its own PSM
  count. Nothing is folded. **No folding option exists.**
- **Worth knowing:** at apex 58.02440 PTM-Shepherd does NOT use the isotope composite. It annotates
  "2,3-dihydro-2,2-dimethyl-7-benzofuranol N-methyl carbamate". It makes our exact mistake on our
  exact peak, despite having the vocabulary. **Reference agreement at +58 is not evidence the peak
  is chemistry** — which matters, because gate 1 is built on reference agreement.
- Our benchmark ran with `isotope_error = 0`, `isotope_states = ` (empty).

**MSFragger / FragPipe** — *DRIFT FLAGGED.*
- Current wiki, `precursor_mass_mode`: "One of isolated/selected/**recalculated**. Isolated uses
  the isolation m/z, selected uses the selected m/z, while recalculated uses a recalculated m/z
  from .ma files within the same directory."
- Our pinned `fragger.params` (4.3) sets `precursor_mass_mode = corrected`; its inline comment
  reads "One of isolated/selected/**corrected**."
- The mode was renamed between our snapshot and current upstream. **Do not cite the wiki as
  describing our run.** What "corrected" does in 4.3 needs 4.3's own docs.
- Current wiki, `isotope_error`: "Isotope correction for MS/MS events triggered on isotopic peaks.
  Should be set to 0 (disabled) for open search or 0/1/2 for correction of narrow window searches."
  Our reference run has 0, consistent.

**Sage** — see §2 for the two quotes (`Isotope Errors`, `deisotope`), both from
`reference-notes/sage-online-docs.md`. Sage pinned at 0.14.7, commit 99407db.

**MetaMorpheus** — labels the misassignment, does not fold it.
- Committed `Task3-SearchTask/MassDifferenceHistogram.tsv` has exactly two rows: MassShift 0.0011 /
  Count 51070 / Mine "Exact match!", and MassShift 1.0040 / Count 3495 / Mine "**1 MM**".
  Ratio 0.068.
- "MM" is probably "missed monoisotopic", matching the MassDiffAcceptor vocabulary ("1 Missed
  Monoisotopic Peak"). **NOT CONFIRMED** — the glossary I fetched defines neither "MM" nor "Mass
  Difference Acceptor". It defines only *notch*: "A narrow mass window in which the is an allowed
  mass difference between the experimentally observed peptide and the best matching theoretical
  peptide." [sic] Treat the expansion as unverified.

**Mascot error-tolerant** — offset test, not folding.
- Mascot help, `#13C`: at 1 it tests "TOL > absolute(exp - calc - 1)"; at 2, "TOL > absolute(exp -
  calc - 2)". Purpose: "you can use a tight mass tolerance and still get a match to a 13C peak."
  Docs note that for high-accuracy instruments the precise shifts are 1.00335 and 2.00670.

**Published treatment.** Rad, Li, Mintseris, O'Connell, Gygi, Schweppe, "Improved Monoisotopic Mass
Estimation for Deeper Proteome Coverage", *J Proteome Res*, 2020: "Misassigned monoisotopic masses
can result in up to a 65% reduction in peptide identifications"; instruments "will often select,
isolate, and record a 13C peak as the precursor". Their fix (Monocle) is applied **before**
database search — the C2/`precursor_mass_mode` layer, not the histogram.
*Year/journal from the PMC record; needs verification.*

**Convergence worth stating in the write-up:** Preview suppresses upstream; MSFragger corrects
upstream; Monocle corrects upstream; Mascot tests offsets at match time; PTM-Shepherd and
MetaMorpheus annotate downstream. **Not one of the six folds satellites in a delta-mass histogram.**
That is the strongest external argument for leaving `enable_satellite_folding` off.

## 7. The invariant

For flag-and-demote. It can fail, and failure is loud. Nothing in it depends on isotope-envelope
intensity, so it survives §4.

Definitions: `C13 = 1.003354835`; `tol(k) = 12 mDa + (|k|-1) * 4.5 mDa`; forest bound 2.5 Da.

**Construction rules**
- R1 **Forest exclusion.** A ladder root must have `|delta| > 2.5 Da`. Under the ghost hypothesis
  this region is a different problem with a different instrument; under NOTES it is a closed
  dead-end. Either way, out of scope for this rule.
- R2 **Root resolution.** Parentage is assigned to a root that is not itself flagged. A flagged
  satellite may not act as a parent.
- R3 **Exclusive closest claim.** For a given root and a given k, only the single closest peak
  within `tol(k)` may be claimed.

**Asserted invariants — these FAIL and raise**
- I1 **Count conservation.** `sum(count) before == sum(count) after`, and the peak-list length is
  unchanged. Structural: flagging moves nothing. Assert it anyway, as fold-to-zero does.
- I2 **Ladder contiguity.** Rung k may exist only if rungs 1..k-1 exist for the same root. A k=3
  claim with no k=1 and no k=2 is not an isotope envelope.
- I3 **Ladder monotonicity.** Strictly decreasing from the root: `count(root) > count(k=1) >
  count(k=2) > count(k=3)` over the rungs present. A rung exceeding the one below it refutes the
  isotope reading — do not flag, raise.
- I4 **Unique parentage.** After R2, a flagged peak has exactly one root. More than one is
  ambiguous — do not flag, raise.

**Measured behaviour.** Run in a form that flags first and checks afterwards, so the checks were
free to fail:
- I3 failures: **0 / 0 / 0** (serum / bcell / b1906).
- I2 caught two bad claims that monotonicity alone allowed: serum `-14.0159 <- -17.0285 at k=3`
  and serum `+17.0275 Ammonium <- +14.0149 at k=3` — both k=3 with no lower rungs, the second
  absorbing an annotated Ammonium peak. **Contiguity is load-bearing.**
- R1 alone removes the entire ±1 Da blow-up. Without it, b1906 `-1.0621` becomes parent to six
  peaks at k=2/k=3 with deviations to 19.9 mDa, and bcell `+1.9818` is claimed by two parents.
  That is the Phase 7C failure reappearing exactly.

Flagged set with all rules on: 11 (serum) / 8 (bcell) / 7 (b1906), including `+58` and `+59` on
all three files.

## 8. What happens to serum Carboxymethyl at 13.2 mDa

**It is not flagged. But the tolerance is not what saves it, and that matters.**

The load-bearing distance is not the 13.2 mDa peak-to-peak separation. It is the distance from
Carboxymethyl to the k=1 fold target of +57:

```
k=1 target      = 57.0237 + 1.003354835 = 58.027055
satellite       @ 58.0260  ->  1.05 mDa from target   INSIDE  12 mDa
Carboxymethyl   @ 58.0128  -> 14.25 mDa from target   OUTSIDE 12 mDa  (by 2.25 mDa)
```

**The 2.25 mDa margin is not safe on its own.** NOTES Phase 7C records m/z-dependent calibration
drift of the same size — "Deamidation apex reads 0.9817 (2.3 mDa low)". Drift of that magnitude in
the other direction puts Carboxymethyl inside the window. A tolerance-only design absorbs a real,
annotated, 93-PSM modification on serum.

What protects it is **R3**: the true satellite is 1.05 mDa from target against Carboxymethyl's
14.25 mDa — a 13.6x margin in distance, which survives drift of a few mDa. R3 is mandatory.

**R2** matters here too. Carboxymethyl otherwise acts as a spurious *parent*: `+59.0276` is
11.45 mDa from Carboxymethyl's k=1 target, inside the 12 mDa window. Root resolution gives `+59`
to the `+57` root at k=2 (2.81 mDa) instead.

And because this is demote rather than fold, the worst case if the rule is wrong is a real peak
losing its recommendation while keeping its count and its row. Under folding, the worst case is a
real peak's PSMs silently absorbed into another peak's count.

## 9. What would falsify this

1. **C2 works.** `isotope_errors: [-1, 2]` on the open search collapses +58 and +59. Then options A
   and B are both unnecessary for this class and the memo's whole apparatus retires. **Cheapest
   decisive test; run it first.** Needs the raw inputs and a fresh search;
   change-regenerate rules apply.
2. **The decoy test refutes the ghost hypothesis** — the ±1 Da forest is target-enriched after all.
   My §1 pairing still holds numerically (the proxy is just peak removal), but the *reason* for
   removing the forest would be wrong, and the X=10% result would need re-deriving under whatever
   criterion replaces it.
3. **A file where the rule flags real chemistry.** One R1–R3-eligible ladder claiming a peak with
   independent evidence of being real, and the rule is unsafe.
4. **I3 failing on a genuine satellite ladder** on a wider panel. Zero failures on three files is
   not validation — NOTES is explicit that one agreeing case is not validation, and three is not
   many more.
5. **A different fold implementation closing the gates** on a panel where the forest is absent. My
   fold simulation is one plausible implementation. The claim I am confident in is narrower:
   *this* fold rule does not close *these* gates, and demote matches or beats it at every X.
6. If `precursor_mass_mode = corrected` in MSFragger 4.3 does not touch the monoisotope, §5's
   mechanism is wrong and the 3.6–11x recon/PTM-Shepherd gap needs another cause.

## 10. Two things I did not do

- **I did not resolve the ghost hypothesis.** Forest removal here is a crude proxy — flat removal of
  everything within ±2.5 Da. I ran a variant keeping annotated Deamidated peaks (real deamidation
  would be decoy-enriched and would survive the real criterion); it gives identical gate counts,
  so the conclusion is robust to that modelling choice. The real test is target:decoy enrichment
  per bin and needs `results.sage.tsv`.
- **I did not touch `enable_satellite_folding`, write folding code, or edit NOTES/PLAN.** The NOTES
  correction in §2 is flagged for the shutdown routine, not applied.

## 11. Reproducing

```
python3 testing/scripts/tier_gates.py          # baseline 47 / 25 / 31 / 16
```
Simulation scripts are in the session scratchpad, not the repo: `breakdown.py` (violations by
offending peak), `simulate.py` (fold vs demote), `rule.py` / `rule2.py` (the invariant, in
flag-first-check-after and construction forms), `ghost.py` (the forest proxy). All import
`tier_gates.py` and `compare_mod_discovery.py` rather than reimplementing loaders.
