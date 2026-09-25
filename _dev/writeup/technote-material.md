# recon tech note: source material

**Status:** working material for a J. Proteome Res. technical note, not
manuscript text. Organized by pipeline phase in execution order. Each phase says
what recon implements, what is distinct about it, how existing tools handle the
same step, which tested assumptions shaped the design, and what evidence
supports it. Nothing is left open. Every point raised while building this is
closed in Appendix B: as a decision (with its commit and measured effect), as a
limitation the note states, or, in one case, as an external expert review
(Appendix B, "External action").

**Updated 2026-09-25** to describe recon v0.2.0 (the code as of `372765c`;
later commits up to `0476f00` change no source code; `bde0652` edits test
messages only). The 2026-09-24 version of this
file described v0.1.3; every change since is listed in Appendix B, and the
numbers it replaced are in Appendix A.

**Scope of evidence.** The note's evidence is the liver file (NIST RM 8461,
`10mg_1_A_1`), because it is the only file with results from all five tools:
recon, Byonic Preview, PTM-Shepherd, MetaMorpheus and Mascot. The other three
files (serum, bcell, b1906) were development test files, chosen because they
were at hand. Numbers from them appear below only as the history behind a
design decision. The inputs of the liver comparison are published in this
repository at `_dev/liver-benchmark/` (with personal paths redacted; its
README lists each file and the script that reads it), so the comparison reruns
from a clone. recon's liver numbers are from `_dev/testing/recon-output/full-run/liver.json`
and `liver_pass2.json` (v0.2.0, `git_commit 372765c`) unless stated.

**Conventions**
- Code is cited by file and symbol, as of `372765c` (v0.2.0). Line numbers
  drift; symbols do not.
- History before the public repository (2026-09-08) is described by date, not
  by commit: it is kept in a private archive (NIST GitLab) as a working record
  and is not cited. Changes made in the public repository
  (github.com/usnistgov/sageRecon) are cited by commit.
- Dev records (`NOTES`, `JOURNAL`, `PLAN`, `reference-notes/`) are cited by
  section heading in `_dev/`.
- Datasets: **serum** (`2019-4-9_909c_0311`), **bcell**
  (`B.naive_01steady-state`), **b1906** (PXD001468), **liver** (RM 8461,
  PXD013608). Triplets such as "4.844 / 3.602 / 3.862" are always in the order
  serum / bcell / b1906.
- Sage version: numbers produced before the v0.15 upgrade (2026-09-01) come from
  Sage v0.14.x builds. The shipped pin is v0.15.0-beta.2 (`df92199`). Each
  number below carries its version where the record states it.
- Verification scope: on 2026-09-24 the development-history numbers were
  re-read from NOTES and PLAN tables, or from the vendored PDFs: +57 146 vs
  3311, the bcell bias, the ladder requirements, the Pass 2 time, the
  MSFragger/recon ratios, the Pass-1 MS2 table, PXD013608, Chick 2015 and the
  Preview floor. Citations were confirmed by lookup. On 2026-09-25 every
  current liver value was re-read from the v0.2.0 JSON, from
  `_dev/liver-benchmark/`, or from a rerun of `liver_mod_rank_comparison.py`
  and `liver_four_tool_digestion.py`. Dated liver measurements (the
  single-change arms of 2026-09-24, such as the prominence and cap effects,
  the Pass 1 and Pass 2 setting tables, the 2.13 % share and the one-TSV
  screen numbers, and OR 1.34 from 2026-09-01) are traced to named NOTES
  entries and were not re-run. Other numbers are traced to NOTES or JOURNAL,
  and each carries its date or version where it predates v0.2.0.

---

## 0. Summary and what is new

`recon run <MZML> <FASTA> --enzyme <ENZYME>` surveys a single raw file before
the real search. It returns search parameters rather than identifications: which
modifications to fix or vary, whether semi-enzymatic search is warranted, and
what precursor and fragment tolerances the data supports. Two Sage searches are
run in process, and each value is measured from the file's own data. The closest
prior art is Byonic Preview (Kil et al. 2011), a commercial survey tool that recon
succeeds in intent as an open, scriptable equivalent.

Candidate claims of novelty, each developed in its phase below:
1. An **alkylation-agnostic open search** in which the alkylation chemistry is
   discovered as a delta peak rather than assumed as a fixed mod (Phase 2).
2. **Self-calibrated tolerances from the open search itself**. Recon uses a
   near-zero clean subset instead of a separate narrow search, and quantizes the
   result onto a ladder that practitioners already use (Phase 4).
3. **Modification recommendations routed by residue specificity**, using a
   Fisher exact test against a per-file background instead of an absolute
   prevalence cut-off (Phase 5).
4. **A digestion measurement defined against a primary source.** It uses
   Preview's peptide-basis denominators and per-class decoy subtraction, runs as a
   cheap second search on a parsimonious protein subset, and was checked against
   four tools on a public reference material (Phase 7).
5. **Sage linked as a pinned library**, so one static binary carries the search
   engine, Unimod and a curated modification list (Cross-cutting §S).

A theme runs through all five: **recon reports measurements and suggests
parameters; it does not judge the sample** (see §R, the composite score).

The headline validation is the **claim test** (§V): on liver, a search guided
by recon's report and filtered by expert judgement identified 125 more stripped
sequences than an expert's vanilla search (15,310 against 15,185, +0.8 %),
with zero run-to-run difference between two vanilla runs, and found 716 PSMs of
chemistry the vanilla search could not see. It cost 5.7× the wall time and
2.2× the peak memory. We frame the result as Ben does: recon says what is in
the sample at a level worth searching for; whether to pay the compute cost is
the user's call for their engine and resources.

---

## Phase 1. Inputs, enzyme and analyzer detection

**Implementation**
- `--enzyme` is required, with no default. There are 14 presets
  (`enzyme.rs` preset table) transcribed from Mascot's enzyme list
  (`reference-notes/mascot-enzymes.md`, snapshot Ben supplied 2026-09-01).
  Buffer-dependent proteases ship as pairs (`glu-c` / `glu-c/de`, `asp-n` /
  `asp-n/ambic`). B, Z, J and X are rejected because Sage's `VALID_AA` would
  abort. A custom-rule form is also accepted.
- The MS analyzer is read from the first 100 MS2 scans (`mzml.rs`
  `DEFAULT_ANALYZER_SAMPLE`). The Thermo filter string (MS:1000512) is
  preferred and the `componentList` CV term is the fallback. Analyzer terms are
  the 12 descendants of MS:1000443 in PSI-MS CV 4.1.259 (vendored, sha256-pinned,
  checked by validation Tier 4).
- The detected class sets the Pass-1 fragment tolerance: Orbitrap and FT-ICR
  20 ppm, Astral 20 ppm, legacy TOF 100 ppm, ion trap and quadrupole 1.0 Da,
  unknown 20 ppm (`mzml.rs` `resolve_ms2_tolerance`).
- The mzML is read for analyzers once per run; the report records that same
  census (`b2a4c07`; it was read twice before).
- Liver: `Orbitrap Fusion Lumos`, Orbitrap on MS1 and MS2, detected (not
  assumed), Pass-1 fragment tolerance ±20 ppm (`liver.json` `analyzers`).

**What is distinct:** the fragment tolerance is chosen from instrument metadata
before any search, not set by the user. The enzyme is an explicit decision the
user must make, not a silent trypsin default.

**Prior art:** Mascot's published enzyme definitions (matrixscience.com
enzyme help). MSFragger instead sweeps the fragment tolerance empirically
(5–50 ppm candidates) after a first search (`msfragger-calibration-tolerances.md`).

**Lessons that shaped it**
- *Assumed:* enzyme rules can be written from general knowledge. *Evidence:*
  the first Asp-N and Glu-C rules lacked Mascot's proline rule, and a B/Z
  residue crashed Sage. *Design:* presets are transcribed from one vendored
  source, and the residue set is checked against Sage's `VALID_AA`.
- *Assumed:* mzML analyzer metadata is authoritative. *Evidence:*
  ThermoRawFileParser before 1.4.4 labels Orbitraps as FT-ICR (bcell, a Q
  Exactive Plus, declares FT-ICR), and it maps its ion trap to the generic
  parent term MS:1000264. *Design:* the per-scan filter string is read first,
  and the CV term is only the fallback.
- *Assumed:* a generous Orbitrap fragment tolerance (±50 ppm, from a curated
  table) is harmless. *Evidence:* on all four files, ±20 ppm gave 12.5–17.4 %
  more PSMs at q ≤ 0.01 and 29–41 % less Pass-1 Sage time (for example bcell
  65,382 → 73,527 PSMs, 171 → 112.5 s; Sage v0.14.x). *Design:* 20 ppm, which is
  documented as the class worst case, not the empirical optimum.
  On liver alone: 27,678 → 32,496 PSMs (+17.4 %), 118 → 82.9 s (measured
  2026-08-31 with Pass 1 at 2 missed cleavages and length 7). Quote the NOTES
  table itself; the prose around it rounds differently (35–41 %, 7–12 %).

**Stated assumptions and limitations**
- The TOF, ion-trap and Astral rows are Ben's working values plus padding, not
  measurements (Appendix E, row 6). The code comment on the TOF constant now
  gives that rationale (about 30 ppm typical, timsTOF up to about 60, padded to
  100; `9417e1d`).
- Limitation for the note: the 13 non-trypsin presets are transcribed from
  Mascot and have not been run on real data.

---

## Phase 2. Pass 1: wide open search

**Implementation** (`defaults/open-search.json`, guarded by `sage_runner.rs`
`write_effective_params_from_text`)
- Settings:
  - `precursor_tol da [-500, 100]`, which gives a delta window of −100..+500 Da;
  - fully enzymatic, `missed_cleavages 1`, length 8–50, the same as Pass 2
    (`395c2f7`; it was 2 and 7–50 until 2026-09-24, see Lessons);
  - charge 2–4;
  - `chimera true`, `report_psms 2`.
- Enforced on every run:
  - **no static or variable mods**, and a leftover static mod is a hard error;
  - `isotope_errors` **must be `[0,0]`**.
- PSMs are kept at `peptide_q <= 0.01` ("q <= 0.01 everywhere", Ben 2026-08-31).
  Rank 1 and rank 2 are both kept.

**What is distinct:** the open search assumes no chemistry. Alkylation
(iodoacetamide +57.02, MMTS +45.99, or none) appears as a delta peak, and the report
then tells the user what to fix.

**Prior art**
- Chick et al. 2015 established the ±500 Da "ultra-tolerant" search, and b1906
  comes from that dataset.
- FragPipe's open workflow runs at about −150/+500 Da with fixed
  carbamidomethyl-C.
- Sage's own PXD001468 example is the template recon started from.

**Lessons that shaped it**
- *Assumed:* fixed carbamidomethyl-C is a safe default, since the starting
  template and FragPipe's open workflow both used it. *Evidence:* on bcell, the +57.02
  peak reads n=146 with `static_mods {C: 57.0215}` and n=3311 without it, where
  it is the top peak in the file (Sage v0.14.x). PTM-Shepherd with C unfixed
  also ranks +57 second (17.92 / 10.68 / 10.77 %). A fixed Cys mod removes the
  largest signal and, with it, the abundance floor derived from that peak.
  *Design:* no mods in Pass 1, enforced by a hard error rather than by the
  template.
- *Assumed:* Sage's isotope-error search helps an open search, as it does a
  narrow one. *Evidence:* `isotope_errors [-1, 2]` cut the b1906 +57 peak from
  1253 to 394 PSMs, and it created a false "Propionyl" peak at +56.018
  (= 57.0219 − 1.003355). *Design:* `[0,0]` is enforced, and isotope handling is
  a post-search fold (Phase 3).
- *Assumed:* the window's signs follow the delta axis. *Evidence:* Sage applies
  the tolerance to the experimental mass, so `da [-500, 100]` yields deltas of
  −100..+500. Michael Lazear confirmed this as a personal communication
  (2026-08-17). On liver it is confirmed twice. The v0.2.0 open search spans
  observed deltas of −99.9997 to +499.949 Da over all 97,835 PSM rows
  (`full-run/liver_search/results.sage.tsv`, a local file, gitignored). A
  direct test with stock Sage v0.15.0-beta.2 and `da [-3.5, 1.25]` gave
  observed deltas of −1.2496 to +3.4987 Da over 44,218 PSM rows
  (`_dev/liver-benchmark/sage-window-check/summary.txt`, with its config and
  script). Development history: a ppm window written `[-30, 5]` gave observed
  errors of −5.047 to +30.007 ppm on serum, so the inversion holds in both
  units. A comparison script built on the wrong reading had missed 6 bcell
  peaks above +100 Da. *Design:* the convention is locked in
  `sage-config-and-gotchas.md`, and the Pass 2 ppm window is written
  sign-inverted (`pass2.rs`; liver writes `ppm [-3.13, +5.95]` for a measured
  window of −5.95..+3.13 ppm, `liver_search/pass2/pass2-effective-params.json`).
- *Assumed:* a second missed cleavage in Pass 1 is worth its cost. *Evidence*
  (liver, `ad7a10e` build, NOTES "Pass 1 digestion settings align with
  Pass 2"): (1, 8) against (2, 7) cut Pass 1 Sage time from 65–72 s to
  43–44 s (1.8× fewer candidate peptides, 2.74 M against 4.94 M) and lost 294
  Pass 1 PSMs (−0.9 %). It changed no recommendation (the same 12 variable
  mods and fixed Carbamidomethyl C), no recommended tolerance (10 / 10 ppm),
  and no Pass 2 rate by more than 0.05 pp. Ben's rule: adopt the fastest
  setting that loses no corroborated recommendation, leaves the tolerances
  unchanged and moves each Pass 2 rate by less than 0.5 pp. *Design:* Pass 1
  at (1, 8) (`395c2f7`), guarded in code
  (`both_passes_share_missed_cleavages_and_min_len`). The cost: the Pass 1
  share of PSMs with 2 or more missed cleavages (2.13 % on liver at (2, 7)) is
  no longer measured.

**Stated choices**
- The window is Sage's own documented open-search setting (Appendix E,
  row 10). The extra −100 to −150 Da that FragPipe searches holds nothing on
  liver (PTM-Shepherd, searching to −150, found no liver peak below −44 Da).
  We state it as a choice with that evidence.
- `chimera true` and `report_psms 2` are also Sage's documented open-search
  example settings. Recon's Pass 1 template is that example (PXD001468 page)
  with mods removed and, since `395c2f7`, 1 missed cleavage and length 8. With `chimera` on, Sage finds the best peptide for a
  spectrum, subtracts the fragment peaks it explains, and searches again.
  `report_psms 2` keeps up to two peptides per spectrum. So co-fragmenting
  peptides contribute their own delta masses to the histogram. Rank 2 is used
  in mod discovery only; calibration and the tier background use rank 1.

---

## Phase 3. Modification discovery (delta-mass histogram)

**Implementation** (`mod_discovery.rs`)
1. **Calibrate:** the Da offset is the intensity-weighted median of |Δ| < 0.1 Da
   (`CalibrationMode::DaScalar`).
2. **Fold isotopes to zero:** Δ = k·1.003355 (k = ±1..±3) is moved to 0, within
   12 mDa + (|k|−1)·4.5 mDa. The constant is the ¹³C–¹²C spacing
   (`sage_results.rs` `C13_C12_DIFF`), not the neutron mass.
3. **Histogram:** 0.01 Da bins; candidate bins have ≥5 PSMs; a candidate is a
   peak when its **topographic prominence** exceeds 0.3 × its height. For each
   bin, recon walks left and right over the whole histogram to the nearest
   strictly higher bin (or the edge), takes the lowest count met on each side,
   including empty bins, and subtracts the higher of the two minima from the
   height (`DenseHistogram`, `topographic_prominence`; `bed06ea`). This is
   PTM-Shepherd's definition. At most **500** peaks (`DEFAULT_MAX_PEAKS`,
   `ad7a10e`; PTM-Shepherd's `peakpicking_topN`). Each PSM is assigned to its
   nearest centre only (`PeakAssignmentMode::Merge`). The settings are
   recorded in the JSON (`mod_discovery.discovery_settings`, including
   `max_peaks`).
4. **Annotate** against Unimod at ±0.01 Da. The classes AA substitution, Other
   glycosylation and Isotopic label are excluded.
5. **Roll up** |Δ| < 0.075 Da into a single "Unmodified" row.
6. **Satellite folding is off** (`enable_satellite_folding: false`).

**What is distinct:** the percentage is presented as a **rank statistic**
(the fraction of PSMs in a delta bin), explicitly not as occupancy. Isotope
handling is a post-search fold, with a conservation invariant asserted in code.

**Prior art**
- **PTM-Shepherd** (Geiszler et al. 2021): fine bins (0.0002–0.001 Da), SNR plus
  apex/shoulder prominence 0.3, Unimod ≤0.01 Da, ≤2-mod decomposition, then
  per-position localization.
- **DeltaMass** (Avtonomov et al. 2019): KDE + GMM, zero-peak recalibration,
  and "annotation is hypothesis".
- **MSFragger**: 2D m/z × RT calibration before the open search.
- **Preview**: suppresses isotope peaks in spectrum preprocessing, which is a
  third architecture for the satellite problem.
- **Crystal-C** (Chang et al. 2020): removes open-search artifacts (missed
  cleavage, chimeras).

**Lessons that shaped it**
- *Assumed:* the isotope spacing is the neutron mass, and Sage's isotope
  correction covers it. *Evidence:* the free-neutron mass over-corrects by
  5.3 mDa per step against the ¹³C–¹²C spacing, and Sage's correction is a no-op
  in a Da-window open search. *Design:* a post-search fold using `C13_C12_DIFF`.
- *Assumed:* isotope satellites can be folded back onto their parent peak.
  *Evidence:* without a conservation check, one run placed 1,770 satellites on
  an 870-PSM parent. *Design:* folding is disabled by design. Satellites are
  instead *demoted* at the recommendation stage (Phase 5), and conservation is
  asserted in code.
- *Assumed:* the peak-assignment mode could be chosen from a gate script's
  output. *Evidence:* that script could not have returned any other answer, and
  an audit found 2 of 6 gate scripts vacuous. A sub-bin histogram then showed
  Split shreds 21 of 27 groups while Merge loses one real doublet (bcell
  −1.030 / −1.020 Da). *Design:* Merge, with the lost doublet as a stated
  limitation. The project rule followed from this: a gate is not done until a
  deliberately wrong input has made it fail.
- *Assumed:* the delta-bin percentage estimates prevalence and can be compared
  across tools. *Evidence:* against MSFragger with variable +57 on the same
  files, MSFragger / recon = 1.23 / 2.98 / 2.22× for +57. The reference tools
  also disagree with each other (serum +57: PTM-Shepherd 17.92 % vs MetaMorpheus
  24.51 %), because each reports a different quantity. *Design:* the report
  presents the percentage as a rank statistic, and the note should compare
  ranks, never percentages.
- *Assumed:* recon's prominence matched PTM-Shepherd's, since the 0.3 ratio was
  taken from it. *Evidence:* the old code took the first higher bin within
  ±0.5 Da in array order (the farthest to the left, not the nearest), saw only
  bins with ≥5 PSMs, and gave a bin adjacent to a taller one its full height as
  prominence. On liver (one open TSV, cap 50, NOTES "Prominence is
  topographic") the prominent centres went 179 → 171; ten flank peaks left the
  list (for example +58.0037, where bins 58.00 / 58.01 / 58.02 hold 40 / 63 /
  125 PSMs, a monotonic rise to the +58.02 peak), and one new peak entered
  (+1.0223, 55 PSMs). The `variable` list lost Carboxymethylation (+58.0037,
  89 PSMs) and gained Water Loss (Glu->pyro-Glu) (−18.0104, 22 PSMs); the
  fixed list and the floor did not move. *Design:* PTM-Shepherd's topographic
  prominence over the whole histogram (`bed06ea`), with two deliberate
  departures: ties do not count as higher (recon must be deterministic, where
  PTM-Shepherd adds random noise), and the sparse histogram is padded with one
  empty bin on each side. Three tests fail under the old rule. Whether the old
  +58.00 call was real Carboxymethylation or the low flank of the +58.025
  Carbamidomethyl ¹³C satellite cannot be decided on 0.01 Da bins; it is the
  same 19 mDa limitation as the deamidation doublet below.
- *Assumed:* 50 peaks are enough. The value had no rationale. *Evidence:* on
  liver, 171 centres passed prominence and the cap cut 121 of them. At 500,
  recon gained four variable recommendations and lost none: Formylation K
  (28 PSMs), Acetylation (14), Kynurenine W (12) and −32.0066 on M (8). The
  first three are seen by PTM-Shepherd, MetaMorpheus or Mascot; no tool sees
  −32.0066. Benjamini–Hochberg then corrects across more tests, so existing
  q values rose up to about 2×, and no decision changed. *Design:* 500
  (`ad7a10e`), PTM-Shepherd's `peakpicking_topN` in the liver run's own
  `shepherd.config`. Liver shows only that the cap must be ≥171; v0.2.0
  reports 183 peaks, well under it.
- *Assumed:* the ±1/±2 Da "carpet" is m/z drift that calibration would remove.
  *Evidence:* three arms were negative.
  - ppm-constant calibration inflated the carpet.
  - Per-stage instrumentation located its origin at peak detection
    (quantization, about 1 % of PSMs).
  - MSFragger-calibrated mzML fed to Sage lost PSMs (−185 / −208 / −1,592) and
    left the bcell deamidation apex unchanged.

  *Design:* a single Da-scalar offset. This is a clean null result, worth
  stating plainly in the note.

- *Assumed:* the open-search histogram can separate deamidation (+0.984) from a
  mis-called ¹³C peak (+1.003). A good deal of development time went here: the
  ±1/±2 Da "carpet", the calibration arms, the ghost hypothesis, and the
  peak-assignment modes all circled this doublet. *Evidence:* the two are
  19 mDa apart. Wilmarth's deamidation guide (the digest in
  `deamidation-wide-search-notes.md`) needs about 120K resolution to
  baseline-resolve them, and it analyzes each charge state separately
  (codeberg.org/pwilmart/Detecting_Deamidation_Guide).
  recon pools all charges into 10 mDa bins. *Design:* recon reports the
  deamidation peak as found, routes it by residue statistics (N/Q), and does
  not try to resolve the doublet. The "carpet" survives only as a diagnostic
  invariant on the abundance path (Appendix E, row 7).

**Evidence it works** (liver; recon v0.2.0, Sage v0.15.0-beta.2,
`full-run/liver.json`; rerun 2026-09-25 with `liver_mod_rank_comparison.py`)
- None of the four alkylation-agnostic tools was told the sample was alkylated.
  recon, PTM-Shepherd, MetaMorpheus and Mascot all rank Oxidation first and
  Carbamidomethyl second among modifications. Unmodified is rank 1 overall;
  recon's +57.0207 is rank 3 with 1,233 of 31,489 PSMs (3.92 %). recon reports
  183 peaks.
- The absolute counts differ by tool (+57: recon 1,233, PTM-Shepherd 2,048,
  MetaMorpheus 1,398, Mascot 2,479; Oxidation: 1,691 / 3,290 / 1,973 / 4,585),
  so the claim is rank agreement only.
- Spearman ρ on shared masses: vs PTM-Shepherd +0.762 (n = 68), vs Mascot
  +0.652 (n = 69), both p = 5e-06, which is the floor of the 200,000-iteration
  permutation test (no permuted ρ reached the observed one); vs MetaMorpheus
  +0.747 (n = 13, p = 0.0047). The MetaMorpheus n is thin by construction
  (G-PTM-D searches a curated list), so never quote that ρ without its n.
- The values moved with the 2026-09-24 changes (Appendix A). Against
  PTM-Shepherd: +0.609 (n = 39, the full-run before the 2026-09-24 changes), +0.676 at cap 50
  with topographic prominence, +0.755 at cap 500, +0.762 with Pass 1 at (1, 8)
  (NOTES "The peak cap is 500" and "Pass 1 digestion settings align with
  Pass 2"). The shared n grew from 39 to 68 because the cap admits many more
  small peaks. A different set of shared masses makes a different test, so the
  sequence is not a before/after measure of recon's accuracy; we quote the
  v0.2.0 values with their n.

**Stated choices**
- Provenance of every histogram and fold parameter is in Appendix E (rows 1–3).
  In short: the 0.3 prominence ratio and its topographic definition are
  PTM-Shepherd's, and so is the 500-peak cap. The bin width, count floor and
  roll-up were set by the coding agent (Cline, 2026-07-07 and 2026-07-14) and
  never compared with alternatives; the note states them as choices.
- There is no two-mod Unimod decomposition. The declared but unused
  `"combination"` source was removed (`b2a4c07`); an unannotated peak is
  reported as unannotated.

---

## Phase 4. MS1/MS2 self-calibration and tolerance recommendation

**Implementation** (`calibration.rs`)
- **Clean subset:** target, rank 1, q ≤ 0.01, |Δcorrected| < 0.02 Da.
- **Hyperscore guard:** keep the top 60 % by hyperscore, but only if the subset
  still has ≥200 PSMs and ≥10 % of MS2 spectra.
- **MS1:** signed ppm is rebuilt as Δcorr / calcmass × 1e6, then bias = median
  and spread = MAD.
- **MS1 recommendation:** the smallest rung of `MS1_TOLERANCE_LADDER_PPM`
  {10, 20, 50, 100} that is ≥ |bias| + 5·MAD (`K`).
- **MS2:** Sage's `fragment_ppm` (absolute): median and MAD on the clean
  subset (`ms2_median_abs_ppm`), and the median over all kept Pass-1 PSMs
  (`ms2_all_psms_median_abs_ppm`). The recommendation is a ppm rung, or for a
  Da analyzer, the ppm converted at **m/z 600**, ×2, rounded up to 0.1 Da
  (`PASS2_MS2_REPRESENTATIVE_MZ`, `cdb970c`; it was m/z 500 until 2026-09-24).
- Liver (`liver.json` `ms1_calibration`): clean subset 7,177 PSMs, bias
  −1.408 ppm, MAD 0.627 ppm, requirement |bias| + 5·MAD = 4.54 ppm, so the
  recommendation is the 10 ppm rung; MS2 median |error| 3.27 ppm (clean
  subset) and 3.38 ppm (all kept PSMs); recommended MS1 / MS2 10 / 10 ppm.

**What is distinct:** one search does both jobs. The open search's own near-zero
population yields the calibration, so no separate narrow search is needed:
"you can't pick a narrow tolerance until you've measured the error". The output
is a rung on a ladder that people already type into search engines, not an
arbitrary decimal.

**Prior art**
- **MetaMorpheus** (Solntsev et al. 2018): three iterative calibrate-search
  rounds; tolerance = round(3·IQR + |median|) for MS1 and 6·IQR for MS2; never
  widens.
- **MSFragger**: a narrow first search, 2D calibration, then an FDR-optimal
  sweep of fragment tolerances.
- **Preview**: calibrate first, then search.
- **Wilmarth** ("Go big or go home?", 2021 blog): PSM yield vs precursor window
  width, and "narrow tolerances select different noise".

**Lessons that shaped it**
- *Assumed:* Sage's `precursor_ppm` column is a signed error, so its median is
  the bias. *Evidence:* the column holds |error|, with zero negative values
  across 3,764 / 32,133 / 10,942 clean-subset PSMs (Sage v0.14.x). bcell's true
  bias is −0.2357 ppm, where the column's median gave +0.7028, and the MAD was
  understated by 37 % (bcell) and 28 % (b1906). Serum masked the error because
  its bias (2.42 ppm) far exceeds its scatter (0.48). *Design:* signed ppm is
  rebuilt from masses and never read from the column. This is the note's
  clearest example of checking a column's convention and not only its name.
- *Assumed:* an asymmetric bias + p95 window is the right user recommendation.
  *Evidence:* it came out 3–5× narrower than field practice (10 ppm) and than
  MSFragger's 20 ppm first search on the same files. *Design:* a ladder rung.
  All three files land on 10 ppm (requirements 4.844 / 3.602 / 3.862 ppm), and a
  test proves that a biased instrument stays inside the symmetric rung. k = 5 and
  the ladder are declared **choices**; any k from about 3 to 15 gives the same
  rung on all three files (`ms1-tolerance-recommendation-rationale.md`).
- *Assumed:* the MS2 recommendation needs a signed bias. *Evidence:* Sage's
  `--annotate-matches` gives serum MS2 +1.0049 ppm, between MSFragger (+0.96) and
  MetaMorpheus (+1.059). The correction moves the requirement by 0.027 ppm
  against a 10 ppm rung. *Design:* MS2 stays absolute and is labelled as such.
- *Assumed:* most fragments fall at 400–600 m/z, so m/z 500 is the right point
  to convert a ppm error to Da (the shipped code comment until 2026-09-24).
  *Evidence:* on liver, stock Sage v0.15.0-beta.2 matched 250,882 fragments
  in 18,865 rank-1 target PSMs at `spectrum_q` ≤ 0.01. Their median m/z is
  605.4, the intensity-weighted median is 726.4, and only 17.27 % fall at
  400–600 (`_dev/liver-benchmark/sage-window-check/summary.txt`, from
  `window_and_fragments.py`). Development history: serum had given 652 / 732 /
  17.6 % (Sage v0.15, 10,511 PSMs, 154,842 fragments, measured by
  `serum_window_and_fragments.py` in the private archive; the code comment in
  `calibration.rs` and NOTES quote that serum measurement and cite this file
  for it). *Design:* convert at m/z 600
  (`cdb970c`), the point the 2026-08-28 design discussion intended. 600 is
  close to the liver median and below the intensity-weighted median; the ×2
  multiplier covers the gap (at 726 m/z a conversion at 600 under-states the
  Da width by about 17 %). Only the Da regime (ion trap, quadrupole) moves; no
  committed report is affected, since all are Orbitrap on MS2. Worked values:
  250 ppm → 0.3 Da, 500 → 0.6, 800 → 1.0.

**Evidence it works**
- **Liver MS1 bias agrees across three tools** (all measured before any
  recalibration, on the same raw file):
  - recon: −1.408 ppm (MAD 0.627, n = 7,177 clean-subset PSMs; v0.2.0);
  - MSFragger first-search calibration: −1.43 ppm (MAD 0.97), from the FragPipe
    log of the PTM-Shepherd liver run in the four-tool comparison;
  - MetaMorpheus Calibrate task, first round: −1.57 ppm (IQR 1.23).

  **Anomaly: Byonic Preview reports 0.0 ppm** (|error| 0.5 ppm, 943 high /
  850 low precursors, pre-recalibration). Preview is therefore the outlier, not
  recon. Possible explanations, none tested: Preview works from its own
  `.mgf` conversion and may re-determine precursor m/z; its population is much
  smaller (~1,800 precursors vs 7,177 PSMs); or its "before recal" figure
  already includes an internal correction. The note should not use Preview as
  the MS1 reference.
- **Liver MS2:** the signed MS2 error agrees across the three tools that report
  a sign: Preview −3.1, MSFragger −3.02, MetaMorpheus −2.99 ppm. recon reports
  only |error|, and its value depends on the population:
  - 3.27 ppm on the calibration clean subset (`ms1_calibration.ms2_median_abs_ppm`,
    shown in the HTML);
  - 3.38 ppm over all kept Pass-1 PSMs
    (`ms1_calibration.ms2_all_psms_median_abs_ppm`; it was
    `mass_accuracy.fragment_median_ppm` until schema 4.0.0).

  Preview's |error| is 3.5 ppm. The note compares Preview with the all-PSM
  value, 3.38 ppm, as NOTES records (the field was kept at schema 4.0.0 for
  this reason), and names the population.

**Stated choices and limitations**
- The clean-subset cut, the 60 % trim and the Da conversion are traced in
  Appendix E (rows 4–5).
- Limitation, locked: the MS1 recommendation is not analyzer-aware. An ion-trap
  MS1 would get a ppm rung. Ben's call is not worth a MAJOR schema change
  without such a file.

---

## Phase 5. Recommendation routing (fixed / variable / not recommended)

**Implementation** (`tier_assignment.rs`, `curated_mods.rs`)
- **Candidates:** the MetaMorpheus curated modification list (99 entries, from
  MetaMorpheus commit `7e453540`, with local corrections itemized in
  THIRD_PARTY_LICENSES). Masses are computed from the formula against Unimod's
  element table.
- **Peaks tested:** |Δ| ≥ 0.1 Da. A peak within ±0.010 Da of a curated entry is
  tested for residue specificity against a background.
  - Background: rank 1, `spectrum_q < 0.01`.
  - Test: one-sided Fisher exact, Benjamini–Hochberg across all tests; pass if
    **OR ≥ 2 and q ≤ 0.05**, with the Haldane–Anscombe correction for zero
    cells.
  - Stats are hand-rolled and pinned to SciPy by test.
- **Unspecific acceptors** (background > 0.95) take an abundance path instead:
  the floor is **20 % of the largest peak with |Δ| ≥ 0.1 Da**
  (`main.rs` `FLOOR_PCT_OF_TOP`). That peak is not necessarily the alkylation
  peak: on liver it is Oxidation (1,691 PSMs), so the floor is 338.2 PSMs.
- **Liver output** (`liver.json` `recommendations`): fixed Carbamidomethyl on C
  (1,233 PSMs, OR 156.9); 12 variable, all decided by statistics: Oxidation M
  (1,691), Deamidation N/Q (544), Gln->pyro-Glu (169), Fe[III] D/E (155),
  Trioxidation C (131), Met-loss+Acetylation (73), Dehydroalanine C (42),
  Formylation K (26), Water Loss (Glu->pyro-Glu) (22), Acetylation protein
  N-term (15), Oxidation to Kynurenine W (12), −32.0074 on M (9).
- **Satellites** (largest peak + n·1.003355, n = 1..2, within ±6 mDa) are
  demoted.
- **Protein-terminal and Met-loss candidates** are tested at protein position 0.
- **Fixed vs variable** is MetaMorpheus's `MT == "Common Fixed"` label,
  inherited and stated as such.

**Where the design came from:** Sage, like any open search, reports a peptide
plus a delta mass, with no localization score (nothing like Ascore). So recon
cannot say which residue carries a mass. When the abundance floor alone gave
unsatisfying results, Ben worked the problem through with Perplexity acting
as an adversarial reviewer (2026-08-24/25). Three things came out of that
back-and-forth:
- a small curated candidate list (MetaMorpheus) instead of all of Unimod;
- a population-level residue test (Fisher's exact test with an odds-ratio
  floor) in place of per-PSM localization;
- detailed rules for how each kind of modification enters the statistics:
  protein termini, Met-loss, saturated backgrounds, satellites.

**What is distinct:** the decision is routed by specificity. A mass that
localizes to a residue must *prove* that localization against the file's own
background. Only mods with no residue specificity face an abundance floor, and
that floor is relative to the file.

**Prior art**
- **Preview**: a decoy-derived per-run floor. `THigh = max{s+1, 23}` and
  `TLow = max{t+1, 15}`, plus decoy subtraction of counts (verified against the
  vendored Kil 2011 PDF).
- **Mascot error-tolerant** (Creasy & Cottrell 2002): pass 2 tests the whole
  Unimod list on a pass-1 protein subset.
- **MetaMorpheus G-PTM-D**: a curated list.
- **PTM-Shepherd**: localization profiles.

**Lessons that shaped it**
- *Assumed:* an "X % of the alkylation peak" floor is established prior art.
  It is not; it is Ben's rule of thumb from years of Mascot error-tolerant
  practice. In an error-tolerant result, the alkylation shows up as the fixed
  mod on C with some count N (say 6,000), and any unsuspected modification
  above about 10 % of N (600) is worth including in the real search. Mascot
  itself publishes no such rule: neither its error-tolerant help page nor its
  blog post on error-tolerant significance gives a threshold (checked
  2026-09-24). An early design note had attributed it to Mascot, which was a
  misreading. *Evidence:* Preview, the closest published method, uses a
  decoy-derived floor instead. A competing hypothesis, that the
  ±1 Da forest below the floor is decoy noise, was refuted: every test region is
  target-enriched (−1 Da: 3.24 / 7.34 / 6.22×). *Design:* the floor stays, is
  relative to the file, and applies only to acceptors with no residue
  specificity. In the code it became 20 % of the largest non-zero peak, not
  10 % of the alkylation count. 20 % was the value at which the step-2 gates
  passed on the three development files, so it is fitted to those files.
- *Assumed:* Unimod's classification separates real modifications from
  artefacts. *Evidence:* Ox(M) is classed "Artefact". *Design:* the candidate
  list is MetaMorpheus's curated list. Residue enrichment then decides identity:
  +57 is Cys alkylation (Cys 2.85 / 9.98 / 9.41× vs Gly ≈1.0).
- *Assumed:* occupancy can decide fixed vs variable. *Evidence:* in an open
  search each PSM carries one delta, so carbamidomethyl occupies only
  23.5 / 53.4 / 50.7 % of Cys PSMs. *Design:* the fixed/variable label is
  inherited from the curated list and stated as inherited.
- *Assumed:* reference tools can adjudicate recon's calls. *Evidence:* a
  pre-committed agreement gate kept failing as real defects were fixed, because
  recon's un-localized population enrichment and the tools' per-PSM
  localization are different quantities. *Design:* the gate was replaced by a
  corroboration rate, 18/21.
- *Assumed:* a 20 mDa window is safe for satellite demotion. *Evidence:* it
  swallowed serum Carboxymethyl. *Design:* ±6 mDa.
- *Assumed:* protein-terminal candidates have no residue to test. *Evidence:*
  that guard labelled them unspecific. *Design:* they are tested at protein
  position 0, and bcell Met-loss+Acetyl (−89.03) is promoted (OR 4230.7).

**Evidence it works:** the claim test (§V) searched liver with the
recommendations an expert keeps from this list and found 716 PSMs carrying
three of them (Fe[III] on D/E, Met-loss+Acetylation, pyro-Glu from E) that a
vanilla search cannot assign. Fe[III] (+52.911 on D/E) is the clearest case:
recon recommends it by residue specificity (155 PSMs, OR 3.30,
q = 0.0048), it is not part of a typical default search, and PTM-Shepherd
(123) and MetaMorpheus (117) also see +52.9105 on this file; Mascot's
error-tolerant list does not.

**Stated limitations**
- Limitation for the note: the 20 % floor was chosen by passing the step-2 gates
  on the three development files, so it is fitted, not derived. The 0.95
  saturation cut has a statistical reason, and the carpet windows are a
  diagnostic only (Appendix E, row 7). On liver no recommendation is decided by
  the floor: all 13 are decided by statistics.
- Known consequence: Hydroxylation-P scores OR 1.34 on liver (measured
  2026-09-01, NOTES "`peptide_hits` is a containment test"), so recon cannot
  recommend Oxidation on P, which all four other tools report. 58.6 % of
  curated entries sit at a contested mass.
- The tier background is fixed at q < 0.01 and ignores the advanced
  `--q-threshold` flag, which the help text warns makes results incomparable.
- Development history: on b1906, recon made 0 protein N-term recommendations
  while MSFragger found 137 N-term acetyl PSMs. On liver, recon recommends
  protein N-terminal Acetylation (15 PSMs) and Met-loss+Acetylation (73).

---

## Phase 6. Contaminants and glycopeptide screen

**Implementation**
- **Polymers** (`polymer.rs`): a port of mzSniffer (W. E. Fondrie, Apache-2.0).
  - 17 series: PEG ×3 charge states, PPG, Triton X-100/X-101 variants,
    polysiloxane, Tween-20/40/60/80, IGEPAL.
  - For each MS1 scan and each expected series m/z, it takes the most intense
    peak within the tolerance, and sums those intensities over all series m/z
    and all MS1 scans.
  - **Tolerance = the measured MS1 requirement, |bias| + 5·MAD** from the
    clean subset, unrounded and symmetric about zero, capped at 100 ppm
    (`calibration::polymer_screen_tolerance`, `615239f`). Fallback when MS1
    was not measured: 10 ppm, mzSniffer's default. Liver: ±4.54 ppm,
    measured.
  - **%TIC = that sum / the sum of every MS1 scan's TIC.** Each scan's TIC comes
    from the mzML (MS:1000285), or the sum of its peaks when the mzML has none
    (`mzml.rs` `get_spectrum_tic`).
- **Oxonium** (`oxonium.rs`): 8 ions. An MS2 scan is a candidate if ≥2
  ions appear in its top 10 % of peaks (by intensity), and HexNAc 204.0867 is
  mandatory.
  - **Tolerance = the Pass-2 fragment tolerance**, in the detected MS2
    analyzer's unit: 5 × the median MS2 |error| in ppm, or for an ion trap or
    quadrupole 2 × that error converted at m/z 600, in Da; clamped to the
    Pass-1 window (`calibration::oxonium_screen_tolerance`, `615239f`).
    Fallback when MS2 was not measured or the analyzer was not detected: 20 ppm.
    Liver: ±16.34 ppm (5 × 3.27 ppm), measured.
  - **% = candidate scans / all MS2 scans in the mzML** (`compute_screening_summary`).
  - Both tolerances are recorded in the JSON (`polymer.tolerance`,
    `oxonium.tolerance`: value, unit, `source` measured or fallback, basis);
    the HTML does not show them. The screens now run after calibration, so a
    FASTA mismatch stops the run before them.
- **Both screens are identification-free.** They read the mzML directly and
  never touch Sage's results, so they would give the same answer with no search
  at all. This defines the denominators:
  - the polymer % is a share of all MS1 ion current, not of unidentified
    signal. A polymer peak that coincides with a peptide m/z is counted anyway;
  - the glycopeptide % is a share of all MS2 scans, identified or not. An
    oxonium-bearing scan that Sage also identified as an unmodified peptide
    still counts.

**What is distinct:** little. These are reused screens that give the user
context in the same report. State that plainly in the note.

**Evidence it works:** the polymer port differs from mzSniffer by 0.00 % on all
16 original polymers (validation harness), at mzSniffer's 10 ppm, which is now
recon's fallback only. On liver (v0.2.0): polymer 0.695 % of TIC, Moderate;
525 glycopeptide candidate scans, 0.92 % of MS2.

**Effect of the measured tolerances** (liver, one open TSV, NOTES "Screen
tolerances come from the measured error"): the polymer tolerance went from
10 ppm to ±4.55 ppm and the polymer share from 0.793 to 0.696 % of TIC, with
the level unchanged (Moderate); every top polymer lost a little. The oxonium
tolerance went from 20 to ±16.37 ppm and the candidate count did not move
(525). The Da branch of the oxonium tolerance is exercised by unit tests only,
since every committed file is Orbitrap.

**Stated assumptions**
- **Oxonium rule, stated as an assumption.** A scan counts if at least 2 of the
  8 ions appear among its top 10 % most intense peaks and one of them is HexNAc
  204.0867. It is a hybrid of two published rules from the Perplexity digest
  `oxonium-ions.md`:
  - "≥2 ions in the top 5 %, 204 mandatory" (a GPQuest-based workflow);
  - "≥2 ions in the top 10 %".

  None of it came from mzSniffer (which screens polymers only). The 20 ppm
  fallback has no source. The measured tolerance comes from peptide
  fragments, while oxonium ions sit at 138–366 m/z, below most of them; their
  own ppm error is not measured.
- **External action (the only one outside the code):** the oxonium rule and
  its tolerance are to be reviewed by a glycoproteomics expert (Nick Riley or
  Chris Ashwood). Until then the note presents the rule as an assumption and the
  glycopeptide share as a screen, not a measurement.
- **Polymer screen.** The Low / Moderate / High cut-offs (< 0.1 / < 1 / < 5 %
  TIC) are recon's own display bands, since mzSniffer reports no levels. They
  have no source, and they conflict with an older note (< 5 / 5–15 / > 15 %);
  the note presents them as recon's own.

---

## Phase 7. Pass 2: semi-enzymatic digestion measurement

**Implementation** (`protein_index.rs`, `pass2.rs`, `digestion.rs`)
1. **Parsimony on Pass 1 proteins:**
   - merge indistinguishable proteins;
   - greedy set cover;
   - drop groups with < 2 peptides (`MIN_PEPTIDES_PER_PROTEIN`).
2. **Subset FASTA:** write it, targets only.
3. **Sage semi-enzymatic search:**
   - precursor window = measured bias ± (|bias| + 5·MAD), not rounded;
   - fragment tolerance = min(5 × median |MS2|, Pass-1 value);
   - `isotope_errors [0,3]`;
   - `missed_cleavages 1`, `min_len 8`, the same as Pass 1 (tested and kept,
     `7a4450e`; see Lessons);
   - no mods, asserted.
   - Liver: 1,700 subset proteins, precursor window −5.95..+3.13 ppm, fragment
     ±16.34 ppm (`liver_pass2.json`).
4. **Classify each distinct peptide** as fully enzymatic, N-ragged, C-ragged or
   non-enzymatic. Initiator-Met excision counts as enzymatic.
5. **Composition:**
   - missed-cleavage and ragged rates over fully + semi peptides (Preview's
     denominators);
   - non-enzymatic peptides reported as a count only;
   - decoys subtracted per class (Kil et al. 2011).

**What is distinct:** an error-tolerant-style two-pass architecture (as in
Mascot and Preview) built on an open engine. The window is sized by the file's
own calibration, and the definition is traceable to one primary source.

**Prior art**
- **Mascot error-tolerant**: pass 2 searches semi-specific on the pass-1 protein
  subset.
- **Preview**: peptide basis, per-class decoy subtraction.
- **Mouchahoir & Schiel 2018** (NISTmAb): XIC-weighted missed-cleavage
  equations, and the reason recon avoids the umbrella term "digestion
  efficiency".
- **Davis et al. 2019**: the liver RM whose Preview report is the ground truth.

**Lessons that shaped it**
- *Assumed:* a semi-enzymatic search can run on the full FASTA. *Evidence:*
  1,226 s vs 172 s for the open search (7.1×, bcell). *Design:* an
  error-tolerant-style second pass on a subset FASTA, which was 6.6× faster.
- *Assumed:* the second pass is cheap once the subset exists. *Evidence:* the
  first end-to-end wiring took 2,437 s on bcell (6,485 proteins) with a wide
  rung-centred window. *Design:* the window is the measured |bias| + 5·MAD,
  which brought bcell to 128 s (PLAN, step 3).
- *Assumed:* Sage's protein column can define the subset. *Evidence:* Sage
  v0.14.x does no protein inference and lists proteins alphabetically.
  *Design:* parsimony (merge, set cover, ≥2 peptides). The saving is file
  dependent (56 % on liver, 3,909 → 1,721 proteins, and 10.7 % on bcell, both
  measured 2026-08-31 with Pass 1 at (2, 7)), so do not quote one figure.
  Sage v0.15 adds its own protein grouping, on by default; recon's templates
  set `protein_grouping: false`, and recon uses its own parsimony. The note
  describes that shipped behaviour.
  Bourgon-style independent filtering was rejected as the justification.
- *Assumed:* a tryptic N-terminus exists only at protein position 0.
  *Evidence:* comparison with MSFragger's `clip_nTerm_M` exposed missing
  initiator-Met excision. *Design:* excision counts as enzymatic. 200 bcell
  PSMs moved to fully tryptic, and liver ragged-N went 7.19 → 6.97 %.
- *Assumed:* PSM-basis rates with a flat FDR match Preview. *Evidence:* the
  Preview `.prv` report counts distinct peptides (319/2008), and the
  semi-tryptic class alone runs at about 10–14 % FDR. *Design:* peptide basis,
  with decoys subtracted per class (Kil et al. 2011).
- *Assumed:* a single "digestion quality" score helps the user. *Evidence and
  design:* see §R. It encoded a sample-type assumption.
- *Assumed:* a third, non-enzymatic search is feasible. *Evidence:* 35.1 M
  peptides and a 363 s index build even on 921 proteins. *Design:*
  non-enzymatic peptides are reported as a count from the semi search only.
- *Assumed:* carrying Pass 1's discovered mods into Pass 2 improves the
  measurement. *Evidence:* Sage was killed after 21 s, the cost broke the
  "answer in minutes" bar, and Met-loss cannot be expressed as a Sage variable
  mod. *Design:* no mods in Pass 2, with a known cost: serum's ragged rate is
  biased by +2.71 pp. The record's supporting figure "liver +57 = 113 PSMs" is
  not cited: it is probably from a fixed-C search, and the agnostic liver value
  is 1,233 (v0.2.0).

- *Assumed:* Pass 2 at 1 missed cleavage and length 8 was a considered
  choice. It was set by the agent (2026-07-08) only to shrink the search
  space, and it means a peptide with ≥2 missed cleavages cannot be identified.
  *Evidence* (liver, one run per arm, NOTES "Pass 2 digestion settings stay at
  1 and 8"): (2, 7) moved the raw missed-cleavage rate by +0.14 pp (17.53 →
  17.67 %) for 4.5× the Pass 2 Sage time (59 s against 13 s; 7.3 M against
  4.2 M candidate peptides). The two changes pull in opposite directions:
  allowing 2 missed cleavages alone adds 124 peptides and +0.98 pp; length 7
  alone adds 753 mostly fully cleaved short peptides and −0.83 pp. Ben's rule
  was to adopt (2, 7) only if it moved the rate by more than 0.5 pp and kept
  Pass 2 under about 2× the time and about 5 minutes; it failed both.
  *Design:* keep (1, 8) (`7a4450e`), and align Pass 1 to it (Phase 2,
  `395c2f7`). **Stated limitation:** the headline missed-cleavage rate counts
  peptides with exactly one missed cleavage; the ≥2 class is not searched in
  either pass. On liver it was 2.13 % of Pass 1 PSMs when Pass 1 allowed 2.
  Preview's rate counts 1 or more, so the cap can only lower recon's rate
  against Preview's; it does not explain why recon reads higher (17.59 against
  15.90 %).

**Evidence it works** (liver. recon row: v0.2.0, `full-run/liver_pass2.json`
`composition`, 10,696 peptides; the Python reclassification of the same
run's Pass 2 TSV (a local, gitignored file) gives the same values. Other rows: each tool's own output,
reclassified by `liver_four_tool_digestion.py`, so they do not depend on the
Sage version. As committed, that script reads the dated 2026-09-24 recon
snapshot in `_dev/liver-benchmark/recon/pass2/` (10,697 peptides, 17.58 /
6.95 / 3.17 %), so a reader who reruns it gets that recon row, one q-draw
away from the one below.)

| source | missed cleavage | ragged-N | ragged-C |
|---|---|---|---|
| recon Pass 2 | 17.59 % | 6.95 % | 3.18 % |
| PTM-Shepherd (fully) | 19.64 % | 1.64 % | 0.51 % |
| MetaMorpheus (fully) | 17.84 % | 0.54 % | 0.45 % |
| Byonic Preview v3.2.0 | 15.90 % | 8.60 % | 1.30 % |

- Acceptance criterion (Ben, NOTES "method spread must be small against sample
  spread"): measured recon vs MSFragger on one file against recon across four
  files. The between-sample spread is 32× the method difference for ragged-N
  (24.7 vs 0.77 pp) and 6.6× for missed cleavage (15.9 vs 2.4 pp). Missed
  cleavage is the weakest link, and it is the headline number.
- Across the four tools on liver, missed cleavage spans 3.74 pp
  (15.90–19.64 %), still about 4× below the 15.9 pp between samples. recon sits
  inside that range. Earlier recon rows (v0.14 17.12 / 6.75 / 3.01 %; v0.1.3
  17.53 / 6.94 / 3.14 %) are superseded (Appendix A).
- PTM-Shepherd and MetaMorpheus searched fully tryptic, so their ragged rates
  are a control near zero, not a measurement; only Preview and recon measure
  ragged ends.
- The 32× and 6.6× ratios above come from development files at Sage v0.14. The
  note uses the liver four-tool spread instead.
- Supporting evidence from sample type (development history): MSFragger's
  semi-tryptic run gives serum 38.0 % semi, consistent with biofluid biology.
- recon reports one digestion measurement: Pass 2 `composition` (distinct
  peptides, per-class decoy correction). The PSM-basis `digestion`,
  `terminus` and `comparison` blocks of the Pass 2 JSON, which put 9.20 % and
  9.48 % semi-enzymatic beside it on liver, and the Pass 1 `digestion` block
  were removed (schema 4.0.0 / 2.0.0, `6d1104b`). The HTML N:C ratio now uses
  the same peptide counts as the printed rates (liver 743 / 340 = 2.19).

**Stated choices**
- The Preview comparison uses Preview's own liver report
  (`_dev/liver-benchmark/preview/10mg_1_A_1/result_summary.html`), not Davis
  2019 Table 3.
- The note quotes one semi-enzymatic class FDR: 9.23 % on liver (fully
  enzymatic 0.15 %), from `full-run/liver_pass2.json` v0.2.0, as README does.
  Other values in the record (9.58, 9.15, 10.10, 9.42, 10.32 %, and 9.14 % in
  `examples/liver_pass2.json`) come from earlier builds or another draw.

---

## Phase 8. Report

- `<base>.json` (schema 4.0.0) and `<base>_pass2.json` (2.0.0), plus an HTML
  report. Main JSON blocks: `input`, `mod_discovery`, `polymer`, `oxonium`,
  `ms1_calibration`, `recommendations`, `analyzers`. Pass 2 JSON: the subset,
  the windows it searched with, and `composition`.
- HTML sections: meta, Detectors, Mass accuracy, Contamination, Glycopeptides,
  Digestion, Recommended search modifications.
- JSON only: the full peak list, `not_recommended` and `notable_unannotated`,
  the screen tolerances, and the all-PSM MS2 median. Every JSON block now
  backs a claim the tool stands behind: the alkylation check, `mass_accuracy`,
  `signal_fate` and the Pass 1 `digestion` block were removed at 4.0.0 (§R).
- Provenance recorded in the report: recon version, git commit (the v0.2.0
  reports carry `372765c`, no `-dirty`), Sage version, enzyme, FASTA,
  discovery settings including the peak cap, and a per-decision audit trail
  (`decided_by`, odds ratio, q value).
- The CLI surface is `recon run <MZML> <FASTA> --enzyme <ENZYME>
  [--output NAME]`, plus advanced flags. One hidden development subcommand
  remains, `discover`, which the validation harness calls; ten others were
  removed (`b2a4c07`, §R).

---

## §V. Validation: the claim test

The question an expert user asks: on an unfamiliar file, does running recon
first and searching with what it recommends identify more than a search with
the expert's own vanilla settings? Source: `_dev/liver-benchmark/claim-test/`
(`README.md`, `results.md`, configs and run scripts); NOTES "Claim test result
on liver (2026-09-25)".

**Design** (Ben, 2026-09-25)
- Two searches of liver with stock Sage v0.15.0-beta.2 at the pinned rev,
  fully tryptic, 1 missed cleavage, length 8–50 (recon's settings), charge
  2–4, at most 2 variable mods per peptide, decoys generated. Only the mods
  and tolerances differ.
- **Vanilla** (Ben's usual settings): fixed Carbamidomethyl C; variable
  Oxidation M, pyro-Glu from peptide N-term Q, Deamidation N/Q, protein
  N-term Acetyl; MS1 / MS2 20 / 20 ppm.
- **recon-guided:** the same, plus pyro-Glu from peptide N-term E, Fe[III] on
  D/E and Met-loss+Acetylation (protein N-term); MS1 / MS2 10 / 10 ppm, recon's
  recommendation.
- The guided list is recon's 12 variable recommendations filtered by expert
  judgement, applied by hand: Ben's rule of thumb keeps a mod at about 10 % or
  more of the alkylation count (+57 on C, 1,233 PSMs). This is his manual
  rule, not the code's 20 %-of-top floor. The line falls at Trioxidation
  (131), which is also impossible alongside a fixed +57 on C. pyro-Glu from E
  (22) and Met-loss+Acetylation (73) were kept as common and cheap.
- A second vanilla run measures run-to-run noise. Success criterion, set
  before the results: more stripped sequences at peptide q ≤ 0.01 than
  vanilla, by more than the vanilla repeat differs, at a cost an expert would
  accept. A null or negative result would be reported as found.

**Results** (run by Ben with `run.ps1` on a 32 GB Windows laptop)

| | vanilla | recon-guided | change |
|---|---|---|---|
| PSMs (spectrum q ≤ 0.01) | 24,005 | 24,160 | +155 (+0.6 %) |
| peptides (peptide q ≤ 0.01) | 18,320 | 18,574 | +254 (+1.4 %) |
| stripped sequences | 15,185 | 15,310 | +125 (+0.8 %) |
| protein groups | 1,396 | 1,432 | +36 (+2.6 %) |
| wall time | 63 s | 355 s | 5.7× |
| peak memory | 10.0 GB | 22.3 GB | 2.2× |

- **Run-to-run noise was zero:** the vanilla repeat gave identical counts,
  and 0 stripped sequences differ.
- Overlap: 14,796 sequences shared, 514 found only with guidance, 389 only
  with vanilla. The net gain of 125 is the difference of two larger sets.
- **716 PSMs carry chemistry the vanilla search cannot assign:** Fe[III] on E
  216 and on D 191, Met-loss+Acetylation 273, pyro-Glu from E 36
  (`results.md`). The 716 is the sum of per-modification PSM counts
  (`summarize.py` counts each modification once per PSM), so a PSM with two
  added mods, such as Fe[III] on both D and E, counts under each. These are
  Sage PSM counts in a closed search, not recon delta-peak counts, so they are
  not comparable with recon's 155 Fe[III] PSMs.

**Verdict, in Ben's framing.** The criterion's first half is met: the guided
search identifies more sequences than vanilla, beyond the (zero) run-to-run
noise. The cost half is not recon's call. recon reports that a modification
is present in the sample at a level worth searching for, sometimes one the
user did not expect (Fe[III] here). Whether to pay the compute cost is the
user's decision, weighed against their search engine and resources. On liver
the guidance gave a modest identification gain (0.6–2.6 % across the four
measures) and 716 PSMs of chemistry the vanilla search missed, for 5.7× the
time and 2.2× the memory.

**Caveats the note states**
- The arms differ in both mods and tolerances. The gain belongs to the
  guidance as a whole; the test does not separate the two.
- Fe[III] on the common residues D and E probably accounts for much of the
  added cost, since it enlarges the candidate space most. No arm drops it, so
  this is an inference, not a measurement.
- One file, one machine, one repeat.
- **Anomaly: zero noise here, jitter elsewhere.** The vanilla repeat was
  identical, yet §S records that Sage q-derived counts jitter, and recon's own
  two v0.2.0 liver runs from one build differ in Pass 2 (missed cleavage
  1,881 against 1,880 of 10,696 peptides; semi-enzymatic class FDR 9.23
  against 9.14 %) while their Pass 1 counts are identical (31,489 PSMs, 183
  peaks). Possible explanations, none tested: the jitter may depend on the
  search (a semi-enzymatic subset search against a fully tryptic full-FASTA
  one); the stock Sage CLI on Windows may order work differently from Sage
  embedded in recon on macOS; or ties near the q threshold may simply not
  occur in this search. Caution: one identical repeat shows that noise was
  small in this run, not that it is always zero, so the note compares the
  +125 gain with that one repeat and says so.

---

## Cross-cutting

### §S. Sage as a pinned library
- Sage (Lazear 2023) is linked through Cargo git dependencies at `df92199`
  (v0.15.0-beta.2), with `Cargo.lock` committed (415 packages).
- Caveat for Methods: a git rev pins Sage's source, not its dependency graph;
  155 of 335 shared packages resolve differently than in Sage's own lock.
- *"Subprocess, not library" was an early lock, not a finding.* It was fixed at
  Phase 0 ("already decided, don't relitigate"). Ben lifted it on 2026-09-01.
  - A serum replay through the embedded library gave 68,817 rows with 0
    differences across 21 columns.
- *The v0.15 upgrade re-baselined every pinned number.*
  - Oxidation moved +4.09 %, accepted as the new benchmark.
  - A predicted effect of removing fragment m/z bounds was falsified on liver.
  - "Wait for v0.15 final" was an agent-invented policy, not Ben's.
- Sage **telemetry** was on in every dev-era search. It is disabled, and
  embedding makes it silent by construction.
- Sage is not bit-deterministic: bcell Pass 1 PSMs were 72,801–72,803 over
  n = 7 (development history). On liver, two v0.2.0 runs of one build gave
  identical Pass 1 results (31,489 PSMs, 183 peaks) and Pass 2 results one
  q-draw apart (10,696 peptides both times; missed cleavage 1,881 against
  1,880; `full-run/liver_pass2.json` against `examples/liver_pass2.json`).
  recon's own code is deterministic. Tests assert bands, not equality. See §V
  for the claim test's identical repeat.

### §D. Datasets and comparison tools

| file | sample | instrument | accession |
|---|---|---|---|
| serum | NIST SRM 909c (frozen human serum), trypsin/Lys-C digest; one QC run from a much larger forthcoming dataset (Ben's lab) | Orbitrap Fusion Lumos, Orbitrap MS1 and MS2, DDA | none (forthcoming) |
| bcell | human naive B cells, steady state (Rieckmann et al. 2017) | Q Exactive Plus (from the mzML filter string) | PXD004352. Chosen because Ben had already been using Sage on this study (talk materials, Zenodo 11221393) |
| b1906 | 293T (Chick et al. 2015) | Q Exactive | PXD001468 |
| liver | NIST RM 8461 (Davis et al. 2019), Ben's own study | Orbitrap Fusion Lumos (the lab's only Lumos) | PXD013608 (confirmed in the Davis PDF) |

- FASTA: UniProt human canonical 2023_05 for dev runs; liver uses
  `uniprot_sprot_iso_human-2018_06`, matching Davis 2019.
- Comparison tools, all run by Ben:
  - FragPipe 23.1 / MSFragger with PTM-Shepherd, in "open" (fixed C),
    "reallyOpen" and tight variants;
  - MetaMorpheus;
  - Mascot error-tolerant;
  - Byonic Preview v3.2.0 (liver only, run 2019-03-14).
- Versions, read from the run logs in the private repo's
  `_dev/testing/reference-data/`: liver PTM-Shepherd run on FragPipe 23.1,
  MSFragger 4.4.1, PTM-Shepherd 3.0.2; MetaMorpheus 1.1.7; Byonic Preview
  v3.2.0; Mascot 2.6.0, error-tolerant, with its bundled Unimod (circa 2018;
  Unimod changes slowly).
- The liver inputs of every comparison tool, and the liver window-sign and
  fragment check, are published in `_dev/liver-benchmark/` (`827466b`,
  `655c1b0`), with the user part of each personal path redacted and the
  redaction rule stated in its README. The development files' reference
  outputs stay in the withheld `_dev/testing/reference-data/`.
- Evidence base: one tryptic Orbitrap file (liver) for the note, and three
  more tryptic Orbitrap files in development. Nothing has been run on a
  non-tryptic digest or an ion trap. This is the headline limitation.

### §P. Development process and AI use
- The work was done by AI coding agents under continuous human review. The
  protocol lives in `_dev/dev_AGENTS.md`:
  - the prime directive is "never assume, always check";
  - a summary is not a source;
  - a contradicting result outranks the hypothesis;
  - one agreeing case is not validation.
- Tripwires: a numerical validation harness (14 → 17 gates; 17/17 pass after
  the v0.2.0 regeneration, with 208 of 208 `cargo test` tests passing with all
  data present, NOTES "Version 0.2.0") against committed reference outputs and
  cross-tool results. A gate is not done until a
  deliberately wrong input has made it fail. A structured debrief closes every
  session.
- Several episodes above are *caught* errors, which is the note's evidence that
  the process works:
  - the |error| median;
  - the fixed-C regeneration;
  - the vacuous gates;
  - the fabricated v0.15 policy.
- AI tools used, as recorded in `docs/AI_USAGE.md` (updated 2026-09-24):
  - Cline (VS Code) with Claude Opus (planning) and Sonnet (implementation),
    2026-07-06 to 2026-07-15;
  - Claude Code from 2026-07-15 onward;
  - Perplexity, in three roles:
    - a coding-session host running Claude models on 2026-08-19;
    - an adversarial reviewer in Ben's design discussions: the 2026-08-17
      benchmark critique, and the Phase 5 statistics;
    - research and drafts of several reference notes (Sonar). These are
      secondary digests, and some still carry `[cite:N]` placeholders.
- The adversarial review is part of the method, not only of the tooling. The
  2026-08-17 critique of the benchmark (eight points, all adopted) found the
  comparison script was reading −150..+100 Da instead of the true −100..+500 Da
  overlap, which had hidden every peak above +100 Da.

---

## §R. Design alternatives considered and rejected

| Alternative | Why rejected | Decided |
|---|---|---|
| Composite digestion score (0–100) | Scored normal serum biology (31.8 % semi) as "64.2/100 Acceptable" because the rubric assumed cell culture; recon cannot know sample type | 2026-07-15 |
| Signal fate / three-layer MS1 ("where the signal goes") | Four measured defects: counted Pass-1 IDs only; "peptide-like" was an m/z window; contradicted polymer %TIC; headline moved with a default tolerance (6.99 vs 7.7 %). Code preserved in `_dev/extracted/`; the remaining `signal_fate` JSON block and its MS1 precursor-intensity code were removed at schema 4.0.0 (`6d1104b`, `b2a4c07`) | 2026-09-01 (report), 2026-09-02 (code moved), 2026-09-24 (JSON block) |
| Separate closed reference search for MS1 bias | Redundant once the open search's clean subset gives the bias; it served only as a cross-check (it matched FragPipe on b1906, +0.53 vs +0.53 ppm) | 2026-09-01 |
| Satellite folding | Violates conservation; replaced by demotion | 2026-07-14 |
| A single search only | Tolerances cannot be chosen before they are measured; two searches, no re-run loop | 2026-08-17 |
| Isotope-error search in Pass 1 | Fabricates modifications (Propionyl) | NOTES, CLOSED-NEGATIVE |
| m/z calibration before discovery (C1/C2) | Three negative arms | 2026-07-16 |
| Mascot paired target-decoy carry-forward | +33 proteins (+7.3 %) drawn from the false-positive tail; Sage already generates one decoy per target | 2026-08-28 |
| Unique-peptide histogram basis | Erases modifications; loses Formylation and 45 % of the carpet margin | NOTES, histogram basis |
| Alkylation check (−57 Da Cys search, "Alkylation appears complete") | Assumed a fixed Carbamidomethyl that Pass 1 never applies, so it contradicted the alkylation-agnostic design. Removed at schema 4.0.0 (`6d1104b`) | 2026-09-24 |
| `mass_accuracy` block (Sage `precursor_ppm` over all open-search PSMs) | Meaningless in a ±500 Da open search (liver p95 76,782 ppm); superseded by `ms1_calibration`. Its fragment median moved to `ms1_calibration.ms2_all_psms_median_abs_ppm` (`6d1104b`) | 2026-09-24 |
| Pass 1 `digestion` block, and the PSM-basis `digestion`, `terminus` and `comparison` blocks of Pass 2 | Pass 1 ragged ends and ≥2 missed cleavages are 0 by construction; the Pass 2 blocks put a second and third semi-enzymatic rate beside the defined one, where a reader could quote the wrong one. One measurement kept, `composition` (`6d1104b`) | 2026-09-24 |
| Ten hidden development subcommands (`parse`, `detect-analyzer`, `compare-peak-assignment`, `signal-fate`, `mzml-stats`, `polymer-stats`, `oxonium-screen`, `digestion-stats`, `qc-stats`, `analyze`) | No committed script, test or workflow called them; hidden code must still compile and stay correct. `discover` stays for the harness (`b2a4c07`, `33d52c6`) | 2026-09-24 |
| First-higher-bin prominence within ±0.5 Da | Not PTM-Shepherd's definition, although its 0.3 ratio was; replaced by topographic prominence (`bed06ea`, Phase 3) | 2026-09-24 |
| 50-peak cap | No rationale; cut 121 of 171 prominent liver centres. Replaced by 500 (`ad7a10e`) | 2026-09-24 |
| (2 missed cleavages, length 7) in either pass | Pass 2: +0.14 pp for 4.5× the time. Pass 1: 1.6× the time for no change in any recommendation or tolerance (`7a4450e`, `395c2f7`) | 2026-09-24 |
| ppm-to-Da conversion at m/z 500 | Below where fragments fall (liver median 605, intensity-weighted 726); replaced by m/z 600 (`cdb970c`) | 2026-09-24 |
| Fixed screen tolerances (polymer 10 ppm, oxonium 20 ppm) | The file's own measured error is available; the fixed values remain only as fallbacks (`615239f`) | 2026-09-24 |

---

## §C. Sources

Verified means confirmed against a vendored PDF or by lookup on 2026-09-24
(Benjamini–Hochberg on 2026-09-25).

| Source | Informed | Status |
|---|---|---|
| Lazear MR. Sage. *J Proteome Res* 2023, 22(11):3652–3659. doi:10.1021/acs.jproteome.3c00486 | engine | verified |
| Kil YJ, Becker C, Sandoval W, Goldberg D, Bern M. Preview. *Anal Chem* 2011, 83(13):5259–5267. doi:10.1021/ac200609a | prior art; Phases 4, 5, 7 | verified (PDF) |
| Chick JM et al. *Nat Biotechnol* 2015, 33(7):743–749. doi:10.1038/nbt.3267 | ±500 Da precedent; b1906 | verified. Cite the published title, "A mass-tolerant database search identifies a large proportion of unassigned spectra in shotgun proteomics as modified peptides"; the vendored author manuscript carries an earlier title. |
| Davis WC, Kilpatrick LE, Ellisor DL, Neely BA. *Sci Data* 2019, 6:324. doi:10.1038/s41597-019-0336-7 | liver RM | verified (PDF) |
| Mouchahoir T, Schiel JE. *Anal Bioanal Chem* 2018, 410:2111–2126. doi:10.1007/s00216-018-0848-6 | digestion metrics | verified (PDF) |
| Creasy DM, Cottrell JS. Error tolerant searching… *Proteomics* 2002, 2:1426–1434 | Mascot ET | verified. It corrects the note that says "no peer-reviewed methods paper". |
| Creasy DM, Cottrell JS. Unimod. *Proteomics* 2004, 4:1534–1536 | annotation | verified (cited in Kil 2011) |
| Kong AT et al. MSFragger. *Nat Methods* 2017, 14:513–520. doi:10.1038/nmeth.4256 | comparison | verified |
| Geiszler DJ et al. PTM-Shepherd. *Mol Cell Proteomics* 2021, 20:100018 | Phase 3 prior art | verified. The note wrongly says "Kong et al." |
| Avtonomov DM, Kong AT, Nesvizhskii AI. DeltaMass. *J Proteome Res* 2019, 18(2):715–720. doi:10.1021/acs.jproteome.8b00728 | Phase 3 | verified. The note labels a PMCID as a PMID. |
| Chang HY et al. Crystal-C. *J Proteome Res* 2020, 19(6):2511. doi:10.1021/acs.jproteome.0c00119 | artifact context | verified |
| Solntsev SK, Shortreed MR, Frey BL, Smith LM. *J Proteome Res* 2018, 17(5):1844–1851. doi:10.1021/acs.jproteome.7b00873 | calibration prior art; curated mods | verified. The note wrongly says "JASMS". |
| Rad R et al. Monocle. *J Proteome Res* 2021, 20:591–598. doi:10.1021/acs.jproteome.0c00563 | satellite folding kept off | verified |
| Mayer G et al. PSI-MS CV. *Database* 2013, bat009. doi:10.1093/database/bat009 | analyzer classes | verified |
| Hulstaert N et al. ThermoRawFileParser. *J Proteome Res* 2020, 19(1):537–542. doi:10.1021/acs.jproteome.9b00328 | analyzer mislabel | verified |
| Keller BO et al. *Anal Chim Acta* 2008, 627:71–81 | contaminant background (not in shipped code) | verified |
| Müller T, Winter D. *Mol Cell Proteomics* 2017 (PMID 28539326) | over-alkylation (reference note only) | not cited by the note: the alkylation check it supported was removed at schema 4.0.0 |
| Benjamini Y, Hochberg Y. Controlling the false discovery rate: a practical and powerful approach to multiple testing. *J R Stat Soc B* 1995, 57(1):289–300. doi:10.1111/j.2517-6161.1995.tb02031.x | Phase 5 | verified (publisher page) |
| mzSniffer (Fondrie), github.com/wfondrie/mzsniffer | polymer port | verified. Upstream's last commit is `e6c3317d` (2023-03-13), so the July 2026 port is of that commit |
| MetaMorpheus @ `7e453540` | curated mods | verified (THIRD_PARTY_LICENSES) |
| mzdata crate v0.65.5 (J. Klein), github.com/mobiusklein/mzdata | mzML reading | credited in `THIRD_PARTY_LICENSES.md` and the HTML footer (`9417e1d`); Apache-2.0 from its registry `Cargo.toml`; the author name is from the owner's GitHub profile, as the crate names none |
| Wilmarth P. "Go big or go home?" blog, 2021-04-22 | MS1 ladder | URL |
| Wilmarth P. *Detecting Deamidation Guide*, 2026-08-18. codeberg.org/pwilmart/Detecting_Deamidation_Guide (MIT; commit `0a1cab5b`) | deamidation lesson; ghost hypothesis | verified against the public README |
| Lazear M., personal communication, 2026-08-17 | window sign | also confirmed empirically (Phase 2) |
| Rieckmann JC et al. Social network architecture of human immune cells unveiled by quantitative proteomics. *Nat Immunol* 2017, 18(5):583–593. doi:10.1038/ni.3693 | bcell (PXD004352) | verified |

---

## Appendix A. Figures not to cite

| Figure | Reason |
|---|---|
| 35–58× (+57 recon vs PTM-Shepherd) | wrong denominator; replaced by 1.23 / 2.98 / 2.22× |
| Any MS1/MS2 bias or MAD dated before 2026-08-24 (e.g. bcell +0.70) | median of \|error\| |
| liverFragger 16.15 / 16.16 % | archived intermediate check; Ben excluded it |
| Merge "0 of 24", "46 PSMs" | from a gate script that could not fail |
| "~25–50× faster than FragPipe" | one log vs n runs; withdrawn 2026-09-04 |
| "±50 ppm fragment tolerance is free" | refuted; it costs 7–12 % of IDs |
| liver +57 = 113 PSMs | probably a fixed-C artifact |
| Spearman ρ +0.616 / +1.000 / +0.502 (PTM-Shepherd / MetaMorpheus / Mascot) | v0.14 |
| Spearman ρ +0.609 (n 39) / +0.943 (n 6) / +0.472 (n 28) | full-run before the 2026-09-24 changes; replaced by +0.762 (n 68) / +0.747 (n 13) / +0.652 (n 69) at v0.2.0 |
| Intermediate Spearman values 0.676, 0.755 (PTM-Shepherd), 0.496, 0.591 (Mascot), 0.964, 0.771 (MetaMorpheus) | single-change measurements on 2026-09-24 (prominence, cap); history only |
| Liver recon digestion 17.12 / 6.75 / 3.01 % | v0.14 |
| Liver recon digestion 17.53 / 6.94 / 3.14 % (10,773 or 10,771 peptides) | Pass 1 at (2, 7), before `395c2f7`; replaced by 17.59 / 6.95 / 3.18 % (10,696) at v0.2.0 |
| Liver recon digestion 17.58 / 6.95 / 3.17 % (10,697) | the `ba4d30e` draw, still the dated snapshot the committed digestion script reads; one q-draw from v0.2.0 |
| Liver semi-enzymatic class FDR 9.58 %, 9.15 % | earlier builds; v0.2.0 is 9.23 % |
| Liver +57 1,279 of 31,793 PSMs (4.02 %); Oxidation 1,709; floor 341.8; Fe[III] 149 PSMs | before the 2026-09-24 changes; v0.2.0 is 1,233 of 31,489 (3.92 %), 1,691, 338.2, 155 |
| Liver MS1 bias −1.417 ppm, MAD 0.63, clean subset 7,574 | before Pass 1 at (1, 8); v0.2.0 is −1.408, 0.627, 7,177 |
| Liver polymer ±4.55 ppm, 0.696 % TIC; oxonium ±16.37 ppm | the one-TSV measurement for `615239f`; v0.2.0 is ±4.54 ppm, 0.695 %, ±16.34 ppm |
| Liver unmodified 51.28 %, 49 or 50 peaks | earlier builds (cap 50, Pass 1 at (2, 7)); v0.2.0 is 48.65 %, 183 peaks |
| Liver Pass 1 PSMs at ≥2 missed cleavages 2.1 % (`examples/liver.json` `digestion`) | the block was removed at 4.0.0 and the class is no longer searched; the measured value, 2.13 % at (2, 7), is in NOTES |
| Serum fragment m/z 652 / 732 / 17.6 % | development file; the note uses liver 605 / 726 / 17.27 % |
| Liver runtime 135.3 s (v0.2.0 full-run) | a claim-test Sage search ran at the same time; runtime is not a measurement there. The last clean liver run was 85.9 s at `ba4d30e` |
| "Do not present recon's MS1 bias as corroborated" (NOTES) | superseded: MSFragger −1.43 and MetaMorpheus −1.57 agree with recon −1.41; Preview is the outlier |
| Serum runtime 95.2 s (README) vs 93.2 s (NOTES) | superseded: README now quotes 46.8 s at `ba4d30e`; a development file, not cited by the note |
| Liver parsimony saving "56 %" as a general figure | file-dependent (bcell 10.7 %), and measured with Pass 1 at (2, 7) |

## Appendix B. Closing ledger (settled with Ben, 2026-09-24; updated 2026-09-25)

Every point raised while building this material ends in one of three places:
decided, stated as a limitation, or the one external action.

**Decided**
- The note's evidence is the liver file; the other three files are
  development history.
- Commit hashes: history before the public repository (2026-09-08) is a
  private working record and is not cited; public usnistgov commits are cited.
- AI use is disclosed in `docs/AI_USAGE.md`: Cline (Claude Opus and Sonnet),
  Claude Code, and Perplexity.
- Datasets:
  - bcell is PXD004352 (Rieckmann et al. 2017);
  - serum is a NIST SRM 909c QC run from a forthcoming dataset, with no
    reference yet;
  - liver and serum were acquired on the lab's Orbitrap Fusion Lumos.
- The abundance floor is Ben's Mascot-practice rule of thumb (10 % of the
  alkylation count), which became 20 % of the largest non-zero peak in code.
  The claim test applied the 10 % rule by hand to filter recon's list (§V).
- The window sign is confirmed by Lazear (personal communication) and, on
  liver, by a direct test (`da [-3.5, 1.25]` → deltas −1.2496 to +3.4987 Da;
  `_dev/liver-benchmark/sage-window-check/`).
- The Pass-1 window, `chimera` and `report_psms 2` are Sage's documented
  open-search example.
- Liver MS1 bias: recon agrees with MSFragger and MetaMorpheus; Preview's 0.0 is
  stated as an anomaly.
- Wilmarth's guide is cited from its public Codeberg repository.
- The Preview comparison uses Preview's own liver report, not Davis 2019
  Table 3.
- Tool versions are recorded: Mascot 2.6.0, and the rest in §D.
- The four-tool liver spread replaces the development-file acceptance ratios.
- One semi-enzymatic class FDR is quoted: 9.23 % (liver, v0.2.0).
- The polymer level bands are presented as recon's own display choice.
- The serum trypsin/Lys-C digest searched as trypsin: no action, since the
  file was only a test.
- The pre-repo "spec" in the first plan was Ben's own early ideas. It needs no
  citation.
- The 2026-08-17 benchmark critique, recorded as coming from a "collaborator",
  was Ben working with Perplexity as an adversarial reviewer. The Phase 5
  statistics came out of the same kind of exchange. Both are disclosed in
  `docs/AI_USAGE.md`.
- Sage v0.15 protein grouping stays off (`protein_grouping: false` in both
  templates); recon uses its own parsimony, and the note describes that.

**Decided and done, 2026-09-24/25** (the former "Planned for the next release"
list; each item with its commit and measured effect on liver)
1. **ppm-to-Da conversion at m/z 600, not 500** (`cdb970c`). Liver fragments:
   median 605.4 m/z, intensity-weighted 726.4, 17.27 % at 400–600. No
   committed report moves, since all are Orbitrap on MS2; only an ion-trap or
   quadrupole Da recommendation changes (250 ppm → 0.3 Da).
2. **Pass 2 kept at 1 missed cleavage and length 8** (`7a4450e`), after
   measuring (2, 7), (2, 8) and (1, 7): (2, 7) moved missed cleavage by
   +0.14 pp for 4.5× the Pass 2 time, failing Ben's 0.5 pp rule. **Pass 1
   aligned to (1, 8)** (`395c2f7`): about 1.6× faster Pass 1, 0.9 % fewer
   Pass 1 PSMs, no recommendation or tolerance change, every Pass 2 rate
   within 0.05 pp. Stated limitation: ≥2 missed cleavages are not searched.
3. **Topographic prominence over the whole histogram**, as PTM-Shepherd
   (`bed06ea`). At cap 50: prominent centres 179 → 171; `variable` lost the
   +58.0037 Carboxymethylation flank and gained Water Loss (Glu->pyro-Glu);
   fixed list and floor unchanged. **Peak cap 50 → 500** (`ad7a10e`): four
   variable gains (Formylation K, Acetylation, Kynurenine W, −32.0066 on M),
   no loss; v0.2.0 reports 183 peaks.
4. **Screen tolerances from the measured error** (`615239f`): polymer
   |bias| + 5·MAD (liver ±4.54 ppm, fallback 10 ppm), oxonium the Pass-2
   fragment tolerance (liver ±16.34 ppm, fallback 20 ppm). Liver polymer
   0.793 → 0.696 % TIC, level unchanged; glycopeptide candidates unchanged
   (525).
5. **Vestigial code and JSON removed** (`b2a4c07`, `6d1104b`, `33d52c6`):
   main schema 4.0.0, Pass 2 schema 2.0.0; the alkylation check,
   `mass_accuracy`, `signal_fate`, Pass 1 `digestion` and the PSM-basis Pass 2
   blocks; ten hidden subcommands; the analyzers are read once per run. The
   HTML N:C ratio now uses peptide counts (liver 2.19).
6. **Documentation defects fixed** (`9417e1d`): the list formerly in
   Appendix C, including mzdata in the third-party credits.
7. **Regeneration and version 0.2.0** (`0f0c3ac`, `0a3c9b1`, `372765c`,
   `ee5d34d`): every committed report rebuilt from a clean tree (no
   `-dirty`); the protein-context test now asserts the mechanism rather than a
   count of moved decisions; liver and `examples/` regenerated at v0.2.0.
   Gates: `run_validation.py` 17/17, `cargo test` 208/208.
8. **Liver benchmark published** (`827466b`, `655c1b0`) in
   `_dev/liver-benchmark/`, with redacted paths, so the five-tool comparison
   and the window/fragment check rerun from a clone.
9. **The claim test** (`884e156` design, `0476f00` result): recon-guided
   against vanilla on liver, +125 stripped sequences (+0.8 %), zero
   run-to-run difference, 716 PSMs of chemistry vanilla misses, for 5.7× the
   time and 2.2× the memory (§V). It replaced the earlier plan to re-search
   with and without Fe[III] alone.

**External action (outside the code; it changes nothing in recon)**
- Review of the oxonium rule (≥2 of 8 ions in the top 10 %, HexNAc 204.0867
  required) and its tolerance by a glycoproteomics expert (Nick Riley or Chris
  Ashwood). The note presents the rule as an assumption until then (Phase 6).

**Stated as limitations in the note**
- The evidence base is tryptic Orbitrap data. The 13 other enzyme presets and
  the Da regime are unexercised on real files.
- The floor (20 %) was fitted on the development files.
- Several parameters were set by the coding agent and never compared with
  alternatives. They are listed in Appendix E and stated as choices:
  - 0.01 Da bins and the ≥5 count;
  - the 60 % hyperscore trim and the 10 % spectra rule;
  - the 4.5 mDa fold step.
- recon reports un-localized delta-bin fractions (a rank statistic), not
  occupancy.
- The MS1 recommendation is not analyzer-aware.
- Deamidation (+0.984) and a mis-called ¹³C peak (+1.003) are not resolved by
  a pooled open-search histogram; the same holds for Carboxymethyl (+58.005)
  beside the Carbamidomethyl ¹³C satellite (+58.025).
- Peptides with ≥2 missed cleavages are not searched in either pass.
- The oxonium tolerance is measured on peptide fragments, not on the oxonium
  ions themselves.
- The claim test is one file, one machine and one repeat, and its arms differ
  in both mods and tolerances.

## Appendix C. Documentation defects

All fixed on 2026-09-24 (`9417e1d`, list in NOTES "Vestigial output and code
removed"); the dirty-tree provenance of the committed reports was fixed by the
regeneration (`0f0c3ac`, `ee5d34d`: every report carries a clean
`git_commit`). One standing caveat, not a defect: reference notes produced
with Perplexity are secondary digests, and their links are leads, not sources
(`VENDOR-CHECKLIST.md` names them). Two keep unresolved `[cite:N]` placeholders
(`mass-error-reporting.md`, `incomplete-alkylation-detection.md`); the note
cites neither.

## Appendix D. Vestigial code and JSON

All removed on 2026-09-24 (§R and Appendix B item 5). Kept on purpose, with
the reason:
- the hidden `discover` subcommand, which `run_validation.py` calls for the
  Tier 3 snapshots;
- `CalibrationMode::None` / `PpmConstant` and `PeakAssignmentMode::Split`,
  which unit tests and `discover` use;
- satellite folding (`enable_satellite_folding: false`), disabled by design,
  not vestigial.

## Appendix E. Threshold provenance (traced 2026-09-24, values updated to v0.2.0 on 2026-09-25)

Sources: git history, JOURNAL, NOTES, reference notes, upstream code, and Ben's
answers (2026-09-24). "Agent" means the value was set by the coding agent,
Cline before 2026-07-15 and Claude Code after, with no recorded external source.

| # | Value | Origin | Evidence or status |
|---|---|---|---|
| 1 | Histogram bin 0.01 Da | Agent, Phase 3 (2026-07-07). PTM-Shepherd uses 0.0002 Da bins with smoothing (`histo_bindivs` 5000); Preview bins to integers | Never compared with alternatives. DeltaMass argues against fixed bins (KDE) |
| 1 | Count floor ≥5 PSMs per bin | The agent's 2026-07-06 plan says "e.g., count > 5". It was 10 at Phase 3 and set to 5 at Phase 7B (2026-07-14) | Only `min_peak_count` moved the prominence values (NOTES "Config-provenance gap CLOSED") |
| 1 | Peak merge 0.01 Da (one bin) | Phase 7B: tightened from 0.02 once calibration removed the offset, so that deamidation is not bridged (`mod-discovery-calibration.md`) | Reasoned, not measured |
| 1 | At most 500 peaks (was 50) | PTM-Shepherd `peakpicking_topN = 500`, in the liver run's own `shepherd.config` (`ad7a10e`). The 50 was the agent's, with no rationale | Liver: 171 prominent centres at the time, 183 peaks at v0.2.0, so the cap does not bind. Recorded as `discovery_settings.max_peaks` |
| 2 | Prominence ratio 0.3, topographic, whole histogram | PTM-Shepherd `peakpicking_promRatio = 0.3` (`shepherd.config`), with the same definition: topographic prominence / height > 0.3 (`Prominence.java`, `PeakPicker.java`, master `61eebcb`) | Same concept, value and implementation since `bed06ea`, except ties (deterministic, no random noise) and one padding bin on each side |
| 3 | Near-zero population \|Δ\| < 0.1 Da (calibration) | Phase 7B. The zero peak is smeared ±0.05–0.1 Da (`mod-discovery-calibration.md`); DeltaMass and PTM-Shepherd "zero-peak correction" | Reasoned |
| 3 | "Unmodified" roll-up \|Δ\| < 0.075 Da | Phase 7B: "capture the full zero smear (~0.06 Da) but stay under 0.1 Da" (`mod_discovery.rs`) | The 0.075–0.1 band is a known leak (`ptm-stratification-design.md`). The ~0.06 smear is not sourced |
| 3 | Isotope fold 12 mDa + 4.5 mDa per step | Phase 7B. The 12 mDa keeps deamidation clear of the k=1 window; the per-step widening "catches satellites that drifted further at higher k" (JOURNAL Phase 7B). The plan had said 10 mDa | Decision: the note uses the arithmetic, 1.003355 − 0.984016 = 19.3 mDa, so a 12 mDa window leaves 7.3 mDa after calibration. The record's other figures (11.2, 9.65 mDa) are not used. The 4.5 mDa step is stated as a choice |
| 4 | Clean subset \|Δ\| < 0.02 Da | Design lock 2026-08-17 (NOTES "MS1 error from the wide search's clean subset") | It imposes a mass-dependent ppm ceiling (4.1 ppm at 5000 Da, 31.9 ppm at 500 Da), so it measures a centre, not a tail (NOTES) |
| 4 | Hyperscore trim 60 %, if ≥200 PSMs and ≥10 % of MS2 | The 50–70 % comes from the 2026-08-19 architecture sketch; 60 % is its midpoint, picked at implementation (2026-08-19) | The 200 is documented and not load-bearing (bootstrap; NOTES "Clean-subset PSM floor"). 60 % and 10 % are agent choices. JOURNAL 2026-08-19 Q1 calls them "reasonable guesses, not validated" |
| 5 | MS2 Da = ppm at m/z 600, ×2, rounded up to 0.1 Da (was m/z 500) | Ben and the agent, in chat, 2026-08-28; 600 was the intended point. Resolution is quoted at m/z 200, but searches take Da while Sage reports ppm, and ppm maps to different Da across m/z | Changed to 600 in `cdb970c`. Liver fragments: median 605.4 m/z, intensity-weighted 726.4, 17.27 % at 400–600 (Phase 4) |
| 6 | Pass-1 MS2: TOF 100 ppm, ion trap 1.0 Da, Astral 20 ppm, Orbitrap 20 ppm | Ben's working values plus padding: TOF ~30 ppm in his experience (timsTOF up to ~60) → 100; ion trap 0.6 Da typical → 1.0; Astral ~10 ppm (no direct experience) → 20 | Experience, not measurement. Only Orbitrap is measured (50 → 20 ppm; Phase 1) |
| 6 | Analyzer read from the first 100 MS2 scans | Ben: enough to catch analyzer switching within a method. MS2 scans, not all scans, because a run can begin with ~10 min of MS1 only. Chosen empirically for speed vs coverage | Empirical |
| 7 | Background saturation 0.95; carpet windows ±0.85–1.15 and ±1.85–2.15 Da | Agent, during the step-2 work on the ±1 Da "forest" (2026-08-24/25). 0.95 has a stated statistical reason: above it the 2×2 table saturates (Carbamyl's K/R/C/M sit in 99.6 % of tryptic peptides; `tier_assignment.rs`) | The carpet feeds no decision. It is computed and written to the JSON (`carpet_margin_psms`) as an invariant check: the floor must sit above the tallest ±1/±2 Da peak it governs. Liver v0.2.0: floor 338.2 PSMs, tallest carpet peak 161, margin 177.2 |
| 8 | Oxonium: top 10 %, ≥2 ions, 204 required; tolerance = Pass-2 fragment tolerance (5 × median MS2 \|error\|, clamped to Pass 1), fallback 20 ppm | The rule is a hybrid of two published rules (Phase 6). The tolerance is measured since `615239f`; the 20 ppm fallback is unsourced | Liver ±16.34 ppm. External action: review of the rule by Nick Riley or Chris Ashwood (Appendix B) |
| 9 | Polymer: tolerance = \|bias\| + 5·MAD, capped at 100 ppm, fallback 10 ppm; levels < 0.1 / < 1 / < 5 % TIC | Measured since `615239f`; 10 ppm = mzSniffer default. The levels are recon's own | Liver ±4.54 ppm. Decision: the note presents the levels as recon's own display bands |
| 10 | Pass-1 window −100..+500 Da | Sage's documented open-search example (`da [-500, 100]`, docs pages "Example: PXD001468 (Open-Search)" and "Search Tolerances"). FragPipe uses −150 | Across all four files in PTM-Shepherd (which searched to −150), one peak lies below −100: an unannotated −130.07 with 11 of 97,500 PSMs (0.011 %), and liver has none (lowest peak −44). The extra 50 Da holds almost nothing here |
| 11 | Both passes `missed_cleavages 1`, `min_len 8` (Pass 1 was 2 and 7) | Pass 2: agent, Phase 6C design (2026-07-08), to shrink the semi-enzymatic search space. Tested and kept 2026-09-24 (`7a4450e`); Pass 1 aligned (`395c2f7`) | Measured on liver by Ben's rules (Phase 2 and Phase 7 Lessons). Stated limitation: ≥2 missed cleavages are not searched |
