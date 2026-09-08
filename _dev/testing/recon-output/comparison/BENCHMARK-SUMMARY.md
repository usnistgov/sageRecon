# Mod-discovery benchmark summary — Sage-Recon vs PTM-Shepherd

**Sage-Recon version:** numbers below REGENERATED 2026-08-25 on the fixed build, after
the peak-assignment defect (one PSM could be claimed by two peaks) was corrected. Any
figure quoted from this file before that date is superseded. Earlier provenance:
commit `46b74d8` (initial); comparison script updated 2026-08-17 (window fix +
tolerance + Spearman scoring).

**This file is hand-written prose, not script output.** `compare_4way.py` mentions it
but does not write it. When the tables are regenerated, this file does NOT follow —
update it by hand or it silently rots.
**Reference tool:** FragPipe 23.1 + MSFragger 4.3 + PTM-Shepherd 3.0.2, open PTM
workflow. Full per-file tables in `recon_vs_ptmshepherd.md`; methodology deltas in
`../../reference-data/ptm-shepherd/README.md`.

Objective tool-vs-tool benchmark — NOT a gate, NOT a comparison to the tool's author.

## Comparison window (updated 2026-08-17)

Prior version used window [−150, +100] Da — both bounds were wrong. The true delta
range for our Sage open search `da:[-500,100]` is **−100..+500** (Lazear confirmed
2026-08-17: Sage applies tolerance to experimental mass, so −500 means theoretical
500 Da lighter than observed = +500 delta). PTM-Shepherd uses −150..+500. True
overlap is **−100..+500**. Updated: `WINDOW_LO = -100.0`, `WINDOW_HI = 500.0`.
This was a tracked latent bug (NOTES 2026-08-17); the old window silently dropped
every peak from +100..+500 — glycans, large adducts, and the masses the recon
tool is specifically meant to flag. Match tolerance updated 0.01 → 0.015 Da to
add headroom for residual m/z-dependent drift after scalar apex_offset correction.

**Spot-check confirms the fix:** window-coverage lines in each table show Sage-Recon
peaks above +100 Da genuinely appearing in the comparison (e.g. bcell open: 6 peaks
>+100 in window, serum: 5). Had these read 0 despite >+100 data existing, the bug
would still be present.

## Headline: agree on the biology; divergences are understood

Matched mods (both tools, ±0.015 Da, shared window −100..+500, per file).
**Regenerated 2026-08-25; superseded values in brackets.**
- **vs PTM-Shepherd open:** bcell **22** [32], serum **25** [24], b1906 **22** [22]
- **vs PTM-Shepherd reallyOpen:** bcell **24** [25], serum **31** [21], b1906 **25** [21]
- **nofixedmods vs reallyOpen:** bcell **24** [30], serum **31** [23], b1906 **25** [23]

**The last two rows are now identical, and that is correct, not a copy error.** Step 1
moved `full-run/` to an alkylation-agnostic search, so `full-run/` and `nofixedmods/`
are the same search family and differ only in peak floor (`min_peak_count` 5 vs 10).
Inside the comparison window the matched sets coincide. **The old "fixed-C" labels on
these rows were retired for the same reason — `full-run/` is no longer a fixed-C
search.**

Every biologically/analytically real mod is recovered by BOTH tools at the same mass:
Oxidation, Deamidation, pyro-Glu(Q), Carbamidomethyl (off-site), Acetyl, Phospho,
Carbamyl, Formyl, Dioxidation, and the Fe/Al/Na/Ca/Ni metal adducts.

## Spearman rank correlation (2026-08-17; REGENERATED 2026-08-25)

Spearman ρ on `percent_PSMs` within the matched set, per file. A high ρ means the
tools not only find the same mods but rank them similarly by prevalence. **This is the
evidence for the whole rank-not-magnitude design, so it is quoted with its superseded
values rather than overwritten.**

vs PTM-Shepherd **reallyOpen** (the like-for-like comparison — both searches
alkylation-agnostic):

| file | ρ | p | n |
|---|---|---|---|
| serum | **0.798** | 7.6e-08 | 31 |
| b1906 | **0.700** | 9.76e-05 | 25 |
| bcell | **0.402** | 0.0517 | 24 |

vs PTM-Shepherd **open** (their fixed-C search; less comparable):

| file | ρ | p | n |
|---|---|---|---|
| serum | **0.562** | 0.00347 | 25 |
| b1906 | **0.533** | 0.0107 | 22 |
| bcell | **0.237** | 0.289 | 22 |

**SUPERSEDED — the pre-2026-08-25 table, kept so the change is legible:**
bcell 0.648 (p 6e-5, n 32), b1906 0.625 (p 0.002, n 22), serum 0.284 (p 0.18, n 24).

**The per-file story INVERTED, and that matters.** The old table read "bcell/b1906
strong, serum weak", and explained serum's weakness by its calibration offset and
sample-prep artifacts. On the fixed build and the like-for-like comparison the
opposite holds: **serum is the strongest (0.798) and bcell the weakest (0.402,
p=0.0517, marginal).** That old explanation is therefore withdrawn — it was reasoning
built on a number the peak defect produced. **Do not re-use it.** No replacement
explanation is offered here, because none has been tested.

What survives is the design claim itself: two of three files show strong, significant
rank agreement, and the third is marginal rather than absent.

Top-10 overlap (mods in both tools' top 10 by % in matched set): 3–4/10 across files.
The low overlap reflects the prevalence asymmetry (their two-pass + recal assigns more
PSMs to high-ranking mods than our single-pass), not a presence miss — the mods ARE
matched, just differently ranked. This is the expected "presence + rank recovered,
magnitude differs" result.

## +57 corroboration from PTM-Shepherd profile.tsv (point 7, 2026-08-17)

The `reallyOpen/global.profile.tsv` provides per-residue enrichment for each peak. For
the +57.021 apex:
- **AA1 = C, enrichment score = 21.0, PSM count = 8372** out of 10694 total localized
  PSMs (78% on Cys).
- Only one residue listed — the signal is overwhelmingly Cys.

This is the external site-enrichment corroboration the collaborator asked for. Our tool
surfaces +57 as a dominant delta (rank 2, 4.5–7.3% in the no-fixed-mods run); PTM-Shepherd
independently confirms the signal localizes to Cys at 21× background enrichment.
**We structurally cannot self-verify site enrichment** (no-per-residue-localization lock)
— we rely on PTM-Shepherd here, which is the correct design: the recon answer
("consider Carbamidomethyl(C) as a fixed mod") is actionable without site confirmation,
and the external tool provides the site evidence when needed. The +57 demo does NOT
rest on `modsummary` alone.

## Speed vs. what it cost (the recon-vs-platform tradeoff, quantified)

- **Their MSFragger open search: ~50.9 min** (2-stage, localization-aware, recalibrated;
  total FragPipe pipeline 57.1 min). **Our open Sage search: 56–123 s per file → ~25–50×
  faster** at the search step.
- **What the speed cost us, per this benchmark: essentially nothing on the biology.**
  The only divergences are cosmetic/methodological, not missed PTMs (below). Strong
  "fast scout" result: order-of-magnitude faster, same modification landscape.

## Two findings the benchmark surfaced about our tool

1. **"PTM-Shepherd only" is dominated by isotope peaks we deliberately fold — NOT
   misses.** Every file's PTM-Shepherd-only list is led by First/Second/Third isotopic
   peak (+1.002/+2.005/+3.007) and Isotopic peak error (−1.002), 3–5% each. We fold
   these to Δ=0 (Phase 7C); PTM-Shepherd keeps them separate. This validates our folding
   as a deliberate, correct choice — the divergence is by design.

2. **A real weakness: our ±1/±2 Da UNANNOTATED residual carpet.** Our "Sage-Recon only"
   lists are full of small UNANNOTATED peaks at +0.93/+0.97/+1.98/−1.06/−1.96 Da
   (0.2–0.8% each). These are isotope-region satellites just outside our k-scaled fold
   window that our 0.01 Da binning + scalar-only calibration leaves as many small bins;
   PTM-Shepherd's 0.0002 Da bins + recalibration collapse them. **This is external,
   across-all-three-files evidence for the deferred m/z-dependent calibration (NOTES C1
   discussion / C2 enhancement)** — the uncorrected drift smears the ±1/±2 regions. Not
   fixed now; recorded as benchmark-confirmed motivation for C1/C2.

## Expert interpretation of b1906 artifacts (Ben's domain read — commentary, not tool output)

The tools objectively established these are present; this is the *why*, from ~19 years
experience + the PXD001468 methods paper (Chick et al. 2015, PMC4515955):

- **Carbamylation (+43, rank 3, 2.6%)** — driven by the paper's **8 M urea at pH 8.5,
  56 °C reduction, and prolonged digestion in 4 M urea** (urea → cyanate → carbamylation
  of N-termini/Lys). A sample-prep artifact, verified against the methods.
- **Formylation (+28)** — non-enzymatic; plausibly repeated 1% formic acid handling +
  0.125% FA during long LC gradients, preferentially on exposed N-termini/Lys.
- **deam, pyro-Glu (Q and E), dihydroxy** — all unsurprising.
- **Fe/Al replacement-of-protons adducts** — noted; not previously seen by Ben, no
  explanation asserted.

This division is the point: the tools agree on presence; the expert explains cause. It
is NOT a validation criterion.

## Verified along the way

- **C+57 was FIXED in their open search** (`add_C_cysteine = 57.02146`, fix-mods table),
  same as ours — which is why +57 appears only small (as off-site over-alkylation) in
  both tools, not as a large delta. Resolves the "why so little +57" question.
- The −1.5/+3.5 MSFragger exclusion does NOT carve a hole in the comparable window
  (verified from profile peak apexes; see ptm-shepherd README delta #4).
- **Enzyme settings match:** our Sage config is `missed_cleavages:2, cleave_at:KR,
  restrict:P, semi_enzymatic:false` — identical to FragPipe's `num_enzyme_termini=2,
  stricttrypsin, missed_cleavages:2`. Not a source of divergence.

## reallyOpen (2026-07-24): PTM-Shepherd re-run with C+57 UNFIXED — the "predict fix-mods" view

Ben re-ran FragPipe with **`add_C_cysteine = 0.0` and every variable mod commented out**
(`reallyOpen/fragger.params` lines 67–82, 125 — even Acetyl protein-N-term `42.0106 [^ 1` was
not used). Full output in `testing/reference-data/ptm-shepherd/reallyOpen/`. This is the
behaviour the recon tool is *meant* to have: find everything as a delta, so the tool can
recommend appropriate fixed/variable mods for a real search. Detail table:
`recon_vs_ptmshepherd_reallyOpen.md`.

- **+57.021464 leaps to rank 2 on every file** (from tiny in the fixed-C open run):
  serum 17.92% (2697 PSMs), bcell 10.68% (6884), b1906 10.77% (2845) — vs old open
  0.79 / 0.09 / 0.16%. **+114.042927** (double-CAM / GlyGly) newly appears (serum 3.27%,
  bcell 1.27%, b1906 1.05%). Unmodified drops (bcell 77.3%→63.0%) as those PSMs move into
  the +57 delta. Oxidation +15.99 rises correspondingly.
- **Our matched-mod agreement is UNCHANGED** — we compare our *existing fixed-C* recon JSONs
  against reallyOpen, so our +57 stays small (0.19–0.45%, rank ~14–15) while theirs is rank 2.
  That is the expected, correct asymmetry: they unfixed C, we did not. It is external
  confirmation of the recon-side counterpart now queued (see below): **run our own open search
  with no fixed mods** and our +57 should surface at reallyOpen magnitude.
- Post-churn regression check (the original NEXT ACTION): re-running the **existing open**
  comparison reproduced the committed `recon_vs_ptmshepherd.md` **byte-identically** — real-PTM
  agreement held through the C1/C2 + Unified-MS1 churn. Confirmed through the script, not
  JSON-alone; carpet unchanged as context.

## Mascot error-tolerant (2026-07-24): third tool, name+site → mass adapter

`testing/reference-data/mascot/error-tolerant/` — `Human_ertol.par` (no mods, 10/20 ppm MS1/MS2,
`ERRORTOLERANT=1`) + three hand-copied full mod summaries. Detail: `recon_vs_mascot.md`.

- **Mascot localizes each mod to a site and splits one mass across site rows** (Carbamidomethyl
  C 1172 + N-term 189 + Y 16 + D 13 + E 11 + H 6 on b1906; plus Gly K/S/T at the same +57.0215).
  Our tool and PTM-Shepherd report +57 as a **single un-localized mass peak** (the
  no-per-residue-localization lock). So the adapter **rolls up all site rows sharing a mass into
  one row** (name→mass via `unimod.xml`, ET summed, sites preserved in the label). Without the
  roll-up, Mascot's C-only row would understate the true +57 population — apples-to-oranges.
  The multi-site spread IS the over-alkylation signal (C intended, N-term/Y/D/E/H off-site);
  it is demoted to the label, not discarded. Conservation asserted (rolled-up == Σ site ET).
- **Agreement on the biology:** matched 25 (bcell) / 29 (serum) / 25 (b1906). Oxidation, Carbamyl,
  Cation:Fe, Deamidated, Formyl, pyro-Glu(Q) all present in both. Rolled-up +57 is Mascot's
  rank-1 mass row (25.64% of resolved ET on b1906) — same story as reallyOpen.
- **Expected divergence = the AA-substitution + Label:15N long tail.** "Mascot only" is dominated
  by hundreds of low-count substitution rows (Xle→His, Cys→Dha, …) and `Label:15N(1)` (9% —
  error-tolerant's isotope-label search space) that we fold/suppress. Same discipline as the
  isotope-peak divergence with PTM-Shepherd — by design, not a miss.
- **Skipped, not silently dropped:** rows with no clean Unimod mass ("Non-specific cleavage",
  unresolved substitutions) are logged with their ET count (bcell 1137, serum 1611, b1906 644 ET).
- **Denominator:** percentages are Mascot's own resolved-ET fraction (NOT PSM count) — stated so
  no one forces PSM-count equality across tools.

## Deferred / queued

- **Recon no-fixed-mods run — ✅ DONE (2026-07-24).** Ran our open Sage search with all mods
  removed (`open-search-*-nofixedmods.json`) → `discover` → `testing/recon-output/nofixedmods/`,
  compared vs reallyOpen in `recon_nofixedmods_vs_reallyOpen.md`. **Result: +57 surfaces at rank 2
  on all three files** — b1906 4.47% (1253 PSMs), bcell 4.50% (3311), serum 7.31% (1125), up from
  ~0.2–0.45% when C was fixed. Matches PTM-Shepherd reallyOpen's rank-2 +57. **The tool now
  discovers the alkylation population as a dominant delta → it would correctly recommend
  Carbamidomethyl(C) as a fixed mod** (the recon "predict search settings" mission, demonstrated).
  - **Honest caveat — magnitude is ~0.4× theirs, systematically** (b1906 4.47 vs 10.77%, ratio
    0.41–0.42× across all three; totals are comparable, so NOT a denominator effect). MSFragger's
    localization-aware two-pass + recalibration assigns more PSMs to the +57 population than our
    single-pass Sage open — the same "lost at search" recon-vs-platform tradeoff quantified in the
    original benchmark. We recover the *presence and rank* (what a recon tool needs to recommend the
    fixed mod), not the platform's full localized magnitude. Stated plainly, not spun.
  - Conservation: no-fixed-mods totals (b1906 28005 / bcell 73527 / serum 15386) differ from the
    fixed-C run (31682 / 81966 / 19307) by the search-space change — fixing C+57 lets more spectra
    match, so removing it drops totals, as expected.
- **Byonic Preview** adapter — data-gated, not on disk. Aligner is N-tool; add when it arrives.

*Note on the artifacts (all four kept on purpose): `recon_vs_ptmshepherd_reallyOpen.md` and
`recon_vs_mascot.md` compare our FIXED-C JSONs (the "why our fixed +57 reads small" record);
`recon_nofixedmods_vs_reallyOpen.md` is the apples-to-apples no-fixed-mods comparison the
reallyOpen dataset was made for.*

## MetaMorpheus (2026-08-21): fourth tool, GPTMD-confirmed reallyOpen tier

`testing/reference-data/metamorpheus/2026-08-21-10-29-48/Task3-SearchTask/AllPSMs.psmtsv` —
MetaMorpheus 1.1.7, three-task pipeline (Calibrate → GPTMD → Search), **all fixed/variable
mods cleared in every task** (`ListOfModsFixed = ""` throughout), GPTMD's candidate list
populated with Common Fixed + Common Variable + Common Biological + Common Artifact + Metal
(including Carbamidomethyl(C) as a proposed, not assumed, candidate) — the MetaMorpheus
equivalent of PTM-Shepherd's `reallyOpen` config (`add_C_cysteine = 0.0`, no fixed mods) and
recon's `nofixedmods` run. Detail table: `recon_nofixedmods_vs_metamorpheus_reallyOpen.md`.
Adapter: `load_metamorpheus()` in `compare_mod_discovery.py`.

### Getting the right MetaMorpheus run — a real methodology finding, not just plumbing

Three prior attempts were wrong for three different, instructive reasons, each caught by
checking real output against a known-good result rather than trusting the pipeline:

1. **First MetaMorpheus run had fixed Carbamidomethyl(C)** (`ListOfModsFixed =
   "Common Fixed\tCarbamidomethyl on C..."` in `Task3-SearchTaskconfig.toml`) — confirmed
   directly from the config file. This was the wrong tier entirely (matches PTM-Shepherd's
   `open/`, not `reallyOpen`) and had to be re-run with all three tasks' fixed/variable
   mod lists cleared.
2. **`Mass Diff (Da)` is NOT the modification mass once GPTMD is in play.** GPTMD bakes
   candidate mods directly into the theoretical protein database at specific sites; a PSM
   matching a GPTMD-modified entry shows near-zero `Mass Diff (Da)` (ordinary calibration
   residual), not the mod's mass. Binning on this column reported MetaMorpheus as having
   essentially no Carbamidomethyl population — directly contradicted by MetaMorpheus's own
   `results.txt` (`"Localized mods seen below q-value 0.01: Carbamidomethyl on C 5448"`).
3. **Unimod name-lookup (the Mascot adapter's approach) does not transfer.** MetaMorpheus's
   internal mod dictionary uses different spellings than Unimod's `title` field
   (`"Deamidation"` vs Unimod's `"Deamidated"`, `"Acetylation"` vs `"Acetyl"`). Name-matching
   silently failed to resolve ~31 distinct mod names per file (thousands of PSMs), inflating
   the "Unmodified" bin to 77–83% (vs. the correct ~57–78%).

**Fix that worked:** MetaMorpheus's own `Mods Combined Chemical Formula` column (e.g.
`C2H3NO` for Carbamidomethyl, `H-2Ca` for a Calcium adduct) gives the exact elemental
composition MetaMorpheus itself computed per PSM. Summing monoisotopic atomic masses from
this formula — no name-matching, no external reference file — reconstructs the true
modification mass directly. `Full Sequence` bracket tags are still used for descriptive
labels only; mass and label are now sourced independently, which is what makes the mass
axis trustworthy regardless of label-parsing edge cases.

**A structurally distinct point, not a methodology bug:** MetaMorpheus's `Task2-GPTMDTask`
produces its own results.txt with mod counts (e.g. `Carbamidomethyl on C: 6565`), and it can
look like a ready-made comparison table. It is not — those are deduplicated protein-site
placements (capped by isoform limits), not PSM/spectrum counts, and the file pools all three
input files with no per-file breakdown. It is useful only as a fast, rank-order sanity check
("is Carbamidomethyl > Oxidation > Deamidation the dominant order, roughly") — not as a
percentage-of-PSMs source. `Task3-SearchTask/AllPSMs.psmtsv` is the PSM-backed, per-file
tier that plays the same role as PTM-Shepherd's `global.modsummary.tsv` and Mascot's
error-tolerant summary.

### Population filter (same evidence-based approach as the ambiguity investigation)

Filtered to `Decoy/Contaminant/Target == "T"`, `QValue < 0.01`, and `Full Sequence` containing
no `|` (MetaMorpheus's multi-candidate marker). Verified by direct measurement that this is
equivalent to, but more precise than, filtering on `Ambiguity Level`: levels `1` and `2D` are
0% pipe-bearing (`2D` is protein-paralog mapping ambiguity only — e.g. same peptide in
`TUBA1A`/`TUBA1C` — not mass/mod ambiguity); levels `2A/2B/2C/3/4/5` are 100% pipe-bearing
(genuine multi-candidate mass ambiguity, e.g. a Calcium-vs-Potassium adduct call on one
spectrum). Exclusion rate: 1.86–3.30% per file — smaller and more defensible than the
~8–10% that filtering on Ambiguity Level alone would have produced.

### Headline: agreement on presence and rank, MetaMorpheus reports higher magnitude

Matched mods (both tools, ±0.015 Da, shared window −100..+500): **bcell 14, serum 17,
b1906 17.**

| file | ρ (matched %) | p | n |
|---|---|---|---|
| bcell | 0.491 | 0.075 | 14 |
| serum | 0.820 | 5.65e-05 | 17 |
| b1906 | 0.630 | 0.0067 | 17 |

The three dominant real PTMs (Carbamidomethyl, Oxidation, Deamidation) are recovered by both
tools on every file, at matching mass and consistent relative rank — the same "agree on the
biology" result already established against PTM-Shepherd and Mascot. Also matched: Fe(II)/
Fe(III) adducts, Formyl, Carbamyl, pyro-Glu(Q), GG, Dioxidation, Methyl, Water Loss, Sodium/
Calcium adducts.

**MetaMorpheus's percentages run systematically higher than recon's** for the three dominant
mods (e.g. b1906 Carbamidomethyl: recon 4.47% vs. MetaMorpheus 11.58%; Oxidation: 2.67% vs.
7.98%). This is the same "recon-vs-platform" pattern already characterized against
PTM-Shepherd's reallyOpen (there: recon ~0.4× PTM-Shepherd's magnitude on +57, attributed to
MSFragger's localization-aware two-pass + recalibration assigning more PSMs to the dominant
delta than recon's single-pass Sage open). MetaMorpheus's GPTMD workflow is also a two-stage,
database-expansion approach (propose candidates → re-search against an expanded database),
so the same explanation plausibly applies here — not independently verified stage-by-stage,
consistent with how the PTM-Shepherd magnitude gap was left as a characterized-not-chased
difference rather than fully attributed to one stage.

### Multi-mod PSMs are visible and labeled correctly

MetaMorpheus's `Full Sequence` can carry multiple simultaneous mod tags on one PSM (e.g. two
Carbamidomethyl-modified cysteines plus an oxidized methionine). These sum to one combined
mass and are labeled with the full comma-joined tag list (e.g. `"Carbamidomethyl on C,
Carbamidomethyl on C"`), rather than being split or attributed to only one mod — consistent
with recon's and PTM-Shepherd's "one row per observed total PSM delta" convention, not
"one row per named mod."

### Expected divergence — MetaMorpheus-only tail

Each file's "MetaMorpheus only" list includes real biological mods recon's open search does
not surface as separate matched peaks at this tolerance/window (Phosphorylation, Acetylation,
Glutarylation, Dimethylation, Nitrosylation, HexNAc) — these are in GPTMD's candidate list and
genuinely confirmed at the PSM level by MetaMorpheus's targeted database-expansion approach,
but are low-abundance enough (dozens to low hundreds of PSMs, <1% each) that recon's untargeted
delta-mass histogram may not isolate them as distinct peaks from the surrounding noise floor.
Not yet checked stage-by-stage whether recon's histogram has a corresponding sub-threshold
peak at these masses — a candidate follow-up, same category as the still-open PTM-Shepherd
"UNANNOTATED carpet" question, not resolved here.

### Verified along the way

- Config confirmed clean (`ListOfModsFixed = ""` in Calibrate, GPTMD, and Search tasks) before
  trusting the run — same "verify against the actual file" discipline as the Mascot/PTM-Shepherd
  config checks.
- Mass-error reporting (`Mass Diff (ppm)`) is unaffected by the mass-source fix above — that
  column measures precursor mass accuracy, a different, correctly-read quantity from
  "what does this PSM's modification weigh." Reported both ways (all confident targets vs.
  unmodified-only) per the Phase 8.6 population-definition question; not otherwise analyzed
  in this pass — flagged as a separate topic requiring its own discussion (see conversation
  log), given an observed but not yet explained systematic offset against FragPipe's own
  post-calibration numbers.
