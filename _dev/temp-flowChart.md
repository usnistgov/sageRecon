# Recon Two-Search Architecture

Working document. Source of truth for the "what and why" of each search.
Feed this into README, methods section, and pub when ready.
Not PLAN or NOTES — this is the architecture narrative.

---

## Overview

One command. Two Sage searches. One report answering three questions:
1. What modifications are present, and which should I fix or vary in my real search?
2. Should I use semi-tryptic search?
3. What precursor and fragment mass tolerances are reasonable?

Everything is derived from the sample's own data. No hardcoded tolerances.
No chemistry assumptions. No re-run loop.

---

## Search 1 — Wide open (alkylation-agnostic discovery search)

### Sage parameters

| Parameter | Value | Why |
|---|---|---|
| `precursor_tol` | `da: [-500, 100]` | Produces a −100..+500 Da delta window. Sage applies tolerance to the *experimental* mass, so the sign is inverted relative to the delta axis (confirmed by Michael Lazear). |
| Enzyme | Fully tryptic, `missed_cleavages: 2` | Wide precursor tolerance + relaxed enzyme specificity is a multiplicative blowup — never combine them. Semi-tryptic assessment gets its own dedicated search (Search 2). |
| `static_mods` | None | Alkylation-agnostic by design (see below). |
| `variable_mods` | None | Same reason. The open window finds everything as a delta. |
| `chimera` | `true` | Captures co-eluting peptides. |
| `report_psms` | `2` | Rank-1 primary + rank-2 secondary per scan. |
| `fragment_tol` | `ppm: [-20, 20]` | Starting value. The output `fragment_ppm` column refines the user recommendation. |
| `min_len` | `7` | Standard tryptic peptide floor. |
| `precursor_charge` | `[2, 4]` | Standard tryptic range. |
| `isotope_errors` | `[0, 0]` | Open search in Da; isotope correction applied post-search via C13−C12 spacing (1.003355 Da), not by Sage. |

### Why no fixed mods

Alkylation chemistry is not known before running recon. +57 Da (iodoacetamide/CAM),
+45 Da (MMTS), or no alkylation at all — whatever was used surfaces as a dominant
delta peak in the open search. Fixing C+57 by default would hide the exact signal the
tool is designed to surface. The user's takeaway is: "your most abundant delta near
+57 suggests CAM alkylation — fix it in your real search."

The fixed-C runs in this repo's history (`open-search-b1906.json`, etc.) were for
apples-to-apples benchmarking against FragPipe's fixed-C PTM-Shepherd run — they are
NOT the product default and should not be used as templates.

### Outputs and what we do with them

**All PSMs (q < 0.01, target, rank-1 and rank-2 per chimera rules):**
- Delta-mass histogram → mod discovery peaks, Unimod annotation, prevalence ranking
- Signal fate: identified vs. unidentified spectra by count and intensity
- Polymer %TIC from MS1 (mzSniffer port)
- Oxonium ion screening from MS2 (glycopeptide flag)
- Missed cleavage distribution (from `missed_cleavages` column, open search only —
  semi-tryptic % is NOT meaningful here because `semi_enzymatic: false`)

**Near-zero clean subset (rank-1 only, q < 0.01, |corrected_delta| < 0.02 Da):**
- Why rank-1 only: chimeric rank-2 rows inherit rank-1's q-value (Sage q-inheritance
  rule). They can pass q < 0.01 without independently clearing FDR at that delta.
  Including them biases the calibration measurement.
- Hyperscore guard: use top 50-70% by hyperscore within this subset *if* that yields
  ≥ 200 PSMs AND ≥ 10% of total spectra. Otherwise fall back to no hyperscore filter
  with a logged warning. q < 0.01 is the principled primary gate; hyperscore is a
  robustness guard against pathologically small subsets on bad samples.
- From this subset: signed median ppm (bias), MAD (core spread), 95th-percentile tail.
- These feed two separate outputs (see "Two numbers, one measurement" below).

**`fragment_ppm` column:**
- Direct from Sage output. No recomputation. Reports the MS2 fragment mass accuracy
  for the confident PSMs. Used as-is for the MS2 tolerance recommendation.

**Identified protein accessions:**
- Extracted from rank-1 PSMs at q < 0.01.
- Used to build the subset FASTA for Search 2 (see below).

### What Search 1 answers

- What modifications are present and at what prevalence?
- What alkylation chemistry was used (and should be fixed in the real search)?
- What is the signal fate — how much MS2 signal is explained vs. unexplained?
- Are there polymer contaminants (%TIC)?
- Is there glycopeptide signal (oxonium ions)?
- What MS2 fragment tolerance is appropriate?
- What is the instrument MS1 bias and spread? (feeds Search 2 and the user report)

---

## Two numbers from one measurement (do not conflate)

The near-zero clean subset from Search 1 yields one set of statistics.
Those statistics are used in two opposite directions for two different audiences.

### User-facing MS1 recommendation (generous, asymmetric)

Purpose: guide the user's real downstream search in any engine (Sage, Comet, Mascot,
SEQUEST, MetaMorpheus, FragPipe — engine-agnostic output).

Reported as separate numbers, not a Sage config string:
- Bias (signed median ppm): the instrument's systematic offset
- Spread (MAD): the typical per-PSM scatter around the bias
- 95th-percentile tail: the generous upper bound

Example output: "MS1 precursor bias: +2.5 ppm. Spread (MAD): 1.2 ppm. 95th percentile: +5.1 ppm."

The user constructs their window: low = bias − 2×MAD, high = bias + 95th_tail.
Asymmetry is intentional and surfaced explicitly — a biased instrument should NOT
be folded into a symmetric ±N window, which would clip real IDs on one side.

**Why not report a ready-to-paste Sage config value:** Sage writes its window as
`ppm: [low, high]` where the sign convention is relative to experimental mass, not
delta mass. Other engines use different sign conventions. Reporting the raw numbers
lets the user apply them correctly regardless of engine.

### Pass 2 precursor_tol (tight, bias-centered)

Purpose: generate a safe precursor window for Search 2 that won't hang the run.

- Center: the measured bias (signed median)
- ⚠ **SUPERSEDED 2026-08-28 — Half-width: MAD.** Measured: a MAD multiple cannot
  do this job. 99% coverage needs 11.0x, 12.1x and 18.3x MAD on three instruments
  of the SAME class, and the superseded `bias ± 3×MAD` covered only 80.8 / 87.5 /
  80.8%. The half-width is now the ladder rung. See NOTES "Pass 2 windows".
- ⚠ **SUPERSEDED 2026-08-28 — Hard cap: ±100 ppm.** Retired: once the half-width
  came from the ladder, whose top rung is also 100, the cap could never bind and
  its own test could not fail. The ceiling is now the ladder's top rung; the TOF
  reasoning below still justifies that VALUE (TOF instruments run 50-80 ppm out of
  the box, so a ±15-20 ceiling would systematically clip them).
- **Round the half-width UP to a clean step, never use the raw computed number
  directly** (2026-08-19). Pass 2's window should always be a bit more generous
  than the measured value — the wide search's number is itself an estimate, not
  a certainty, and semi-tryptic Pass 2 needs the cushion more than it needs
  precision. Example: a computed 2.2 ppm half-width rounds up to 5. Step table
  proposed (open to adjustment): 5, 10, 20, 30 ppm, then by 10s up to the ±100
  cap. ✅ **RESOLVED 2026-08-28: the table is `MS1_TOLERANCE_LADDER_PPM`
  {10, 20, 50, 100} ppm**, not the 5/10/20/30 sketched here. The convention this
  paragraph states — round UP to a clean step, never use the raw number — is
  exactly what shipped, and it is now coverage-measured rather than TBD.

Why bias-centering matters: a +30 ppm drifted instrument with a window centered at 0
misses most real peptides. Bias-centering lets the half-width stay tight while still
catching real peptides. A narrow window is needed because semi-enzymatic search
inflates the candidate database ~×peptide-length; a wide precursor window on top is
a multiplicative blowup (Phase 6B finding, measured at 7× slower on full FASTA).

**Locked: measure once, apply once, no re-run loop.** Pass 2 confirms the window
worked (observability). If Pass 2's observed distribution disagrees with Search 1's
prediction, this is reported as a transparency note — not a trigger to re-run with
adjusted parameters. Re-running would blur what the user is seeing, turn fast recon
into a slow refinement loop, and cross the "parameter auto-configuration engine"
non-goal.

---

### Cross-tool validation band (not a pass/fail gate)

When checking recon's MS1/MS2 numbers against FragPipe/MSFragger and MetaMorpheus
on the same file (2026-08-19): **the bar for "recon agrees" is however much
FragPipe and MetaMorpheus agree with each other, not a fixed ppm tolerance.**
If the two established tools themselves differ by 0.3 ppm on a file, recon
landing within a similar range of either is a pass; expecting recon to land
closer to one of them than they land to each other is holding it to a standard
the reference tools don't meet themselves. Some wiggle beyond that band is
still expected and not automatically a bug — three different engines, three
different PSM-population definitions (see Phase 8.6 in PLAN.md) are not going
to converge to the same decimal. This replaces any earlier framing that treated
a fixed ppm gap as inherently a problem to explain away.

---

## Search 2 — Narrow semi-tryptic (digestion efficiency probe)

### Sage parameters

| Parameter | Value | Why |
|---|---|---|
| `semi_enzymatic` | `true` | The whole point. Allows peptides with one non-tryptic terminus. |
| `missed_cleavages` | `1` | Reduced from 2 (Search 1) — combined with semi-enzymatic, higher MC would further inflate candidates. |
| `min_len` | `8` | Slightly tighter than Search 1 to reduce semi-tryptic noise at short lengths. |
| `precursor_tol` | Bias-centered, MAD half-width, ±100 ppm cap | Derived from Search 1 clean subset (see above). Self-calibrated per run. |
| `fragment_tol` | `ppm: [-20, 20]` | Same as Search 1. We do not use Search 1's fragment_ppm estimate here — semi-enzymatic search needs stability and the fixed ±20 ppm is already tight enough for Orbitrap data. |
| `static_mods` | None | Same as Search 1 — consistency, and semi-tryptic % is a ratio that doesn't require full mod coverage. |
| `variable_mods` | None | Same reason. |
| FASTA | Subset (~1k proteins) | See below. |

### Why subset FASTA

Semi-enzymatic search inflates the candidate database roughly ×peptide-length
(every internal position becomes a possible N-terminal cut site). On the full
20k-protein UniProt human proteome, this made Phase 6B's full-FASTA semi-enzymatic
search 7× slower (1,226 s vs. 172 s). The subset FASTA reduces this to ~3 minutes.

Subset construction:
- Take rank-1 PSMs from Search 1 at q ≤ 0.01
- Extract unique protein accessions
- Filter the full FASTA to those proteins (~1k entries)
- Include both target entry and its `rev_` decoy for each protein
- This step is implemented in Rust inline within the `run` command
  (ported from `testing/scripts/subset_fasta.py`)

Known FDR caveat: a flat `peptide_q ≤ 0.01` on mixed tryptic + semi-tryptic PSMs
inflates semi-tryptic false positives because semi-tryptic PSMs score lower on
average. Class-specific FDR (FragPipe-style) would be more precise. This is a known
limitation — documented, not silently ignored. The ratio (semi/total) is still
useful directionally.

### Terminus annotation

For each PSM from Search 2, classifies its termini:
- Fully tryptic: both termini follow trypsin rules (K/R before N-term unless Pro;
  K/R at C-term unless next residue is Pro; or protein N/C-terminus)
- Semi N-ragged: C-term tryptic, N-term non-tryptic (exopeptidase / degradation)
- Semi C-ragged: N-term tryptic, C-term non-tryptic (incomplete digestion)
- Non-tryptic: neither terminus

Algorithm: for each peptide, find it in its protein sequence (substring search),
then check flanking residues against the trypsin rule. Protein N- and C-termini are
tryptic by definition. Implementation ported to Rust from
`testing/scripts/annotate_termini.py` (Phase 6C; the Python script is now deprecated
in favor of the Rust port).

**Attribution note:** the classification logic (`is_tryptic_nterm`, `is_tryptic_cterm`,
`classify_terminus`) is original — standard trypsin cleavage rules, hand-written for
this repo on 2026-07-08. `pyteomics.fasta` is used only as a FASTA parser in the
Python original (interchangeable with Biopython); no algorithm or code was taken from
pyteomics, OpenMS, or any other package. No external citation is needed for this
component.

### Outputs and what we do with them

**PSMs (q < 0.01, target, rank-1):**
- Semi-tryptic %: semi-tryptic PSMs / total PSMs (a ratio — robust to window width)
- N-ragged vs. C-ragged terminus counts and %
- N:C ratio: >1 suggests exopeptidase / degradation; <1 suggests incomplete digestion

**Precursor and fragment distributions:**
- Compute median + MAD for Search 2 `precursor_ppm` and `fragment_ppm`
- Compare to Search 1 predictions:
  - "Search 1 predicted precursor bias +2.5 ppm; Search 2 observed +2.1 ppm (Δ −0.4 ppm)"
  - "Search 1 spread MAD 1.0 ppm; Search 2 MAD 1.2 ppm"
- If Search 2 median is more than 2×MAD from the Search 1 prediction, print a
  visible note: "Search 2 precursor distribution shifted from Search 1 estimate —
  the MS1 window may have been too tight or the subset FASTA non-representative.
  Results reported as-is." User decides what it means. No re-run.

### What Search 2 answers

- Should I use semi-tryptic search?
- What fraction of my peptides have ragged termini, and in which direction?
- Does this look like normal tryptic digestion, endogenous proteolysis (biofluid),
  or degraded sample?

**Important:** the tool presents numbers, not judgments. High semi-tryptic % in a
biofluid (serum, plasma) is expected biology — endogenous protease activity. The
same number in a cell lysate might indicate poor digestion. The tool cannot know
the sample type, so it does not emit a quality score or label.

---

## Assumptions the methods section must state

1. **Trypsin as primary enzyme.** The two-pass digestion workflow and the
   semi-tryptic classification assume trypsin cleavage rules (cleave at K/R, not
   before P). Other proteases would require different cleavage rules.

2. **Near-zero delta = unmodified peptide.** The MS1 calibration logic depends on
   the population at |Δ| < 0.02 Da representing genuinely unmodified peptides with
   instrument-level mass error. If a sample had a ubiquitous modification shifting
   all peptides away from zero (hypothetically), the clean subset would be sparse
   and the calibration unreliable.

3. **Single-pass open search recovers presence and rank, not full localized magnitude.**
   Compared to a two-stage localization-aware platform (FragPipe + PTM-Shepherd), the
   Sage open search recovers +57 at the correct rank but at ~0.41-0.42× the magnitude
   on our test files. A methods section must state that prevalence estimates are
   single-pass, not localization-corrected.

4. **No fixed alkylation assumed.** This is a feature, not a limitation — but it must
   be stated. A Sage open search with no fixed mods has a larger search space than a
   fixed-C search, so PSM totals are lower. The benchmark showed our totals dropped
   from ~32k (fixed-C) to ~28k (no-fixed-mods) for b1906 — expected from the wider
   search space, not a bug.

5. **One report per file.** MS1 accuracy, calibration, and the mod landscape are
   instrument- and loading-specific. Results from multiple files must not be blended.

6. **Semi-tryptic % uses a flat peptide_q ≤ 0.01 cutoff.** Class-specific FDR
   (separate tryptic and semi-tryptic FDR control) would be more precise. Our ratio
   is directionally useful but may overcount semi-tryptic false positives.

7. **The ±100 ppm hard cap on Pass 2 is a conservative unmeasured backstop.** The
   actual safe upper limit for Pass 2 runtime on a subset FASTA has not been
   measured as a function of ppm. Banked for future calibration.

---

## Design choices and areas for future improvement

| Decision | Current choice | Rationale | Revisit when |
|---|---|---|---|
| Clean-subset hyperscore guard | Percentile + N guard; fall back to q-only when small | q<0.01 is principled; guard prevents pathologically tiny subsets | Validated on more sample types |
| MS1 window output format | Engine-agnostic numbers (bias, MAD, tail) | Different engines use different sign conventions | If we add engine-specific config generation |
| Pass 2 fragment tolerance | Fixed ±20 ppm | Stability; open-search fragment_ppm already reflects good Orbitrap performance | If TOF or low-res instruments are targeted |
| Semi-tryptic FDR | Flat q ≤ 0.01 | Simple, transparent | If false-positive semi-tryptic rate becomes a problem in practice |
| Subset FASTA size | ~1k proteins at q ≤ 0.01 | Balances coverage vs. speed (3 min vs. 20+ min) | If low-ID-rate samples yield too-small subsets |
| Pass 2 ±100 ppm cap | Unmeasured conservative backstop | TOF-safe; stops pathological fits | After measuring Pass 2 runtime vs. window width on a subset FASTA |
| Single open search pass | No localization-aware second pass | Recon mission: fast scout, not platform | If users consistently need magnitude estimates, not just rank |

---

## Planned additions (not yet built)

- MetaMorpheus G-PTM-D approach: vendor and review for potential integration or
  comparison of the PTM discovery step. (See next session.)
- Per-run ranking confidence flag (`ranking_confidence` block in JSON):
  apex_offset, adduct_pct, satellite_pct, n_peaks_above_1pct → `high` / `exploratory`
  tier. Beta, banked in NOTES.
- Self-calibrated MS1 tolerance: the clean-subset math described above — designed,
  not yet wired into `run`.
- Byonic Preview adapter: data-gated, insert when files are available.
