# recon tech note: source material

**Status:** working material for a J. Proteome Res. technical note, not
manuscript text. Organized by pipeline phase in execution order. Each phase says
what recon implements, what is distinct about it, how existing tools handle the
same step, which tested assumptions shaped the design, and what evidence
supports it. Nothing is left open. Every point raised while building this is
closed in Appendix B: as a decision, as a change planned for the next release,
or as a limitation the note states.

**Scope of evidence.** The note's evidence is the liver file (NIST RM 8461,
`10mg_1_A_1`), because it is the only file with results from all five tools:
recon, Byonic Preview, PTM-Shepherd, MetaMorpheus and Mascot. The other three
files (serum, bcell, b1906) were development test files, chosen because they
were at hand. Numbers from them appear below only as the history behind a
design decision. The full liver results are in the private repo, at
`_dev/testing/reference-data/liver-full-results/`.

**Conventions**
- Code is cited by file and symbol, as of usnistgov `e942455`. Line numbers
  drift; symbols do not.
- History is described by date, not by commit. The full development history is
  kept in a private archive (NIST GitLab) as a working record; it is not cited.
- Dev records (`NOTES`, `JOURNAL`, `PLAN`, `reference-notes/`) are cited by
  section heading in `_dev/`.
- Datasets: **serum** (`2019-4-9_909c_0311`), **bcell**
  (`B.naive_01steady-state`), **b1906** (PXD001468), **liver** (RM 8461,
  PXD013608). Triplets such as "4.844 / 3.602 / 3.862" are always in the order
  serum / bcell / b1906.
- Sage version: numbers produced before the v0.15 upgrade (2026-09-01) come from
  Sage v0.14.x builds. The shipped pin is v0.15.0-beta.2 (`df92199`). Each
  number below carries its version where the record states it.
- Verification scope (2026-09-24): the headline numbers were re-read from
  NOTES and PLAN tables, or from the vendored PDFs: +57 146 vs 3311, the bcell
  bias, the ladder requirements, the Pass 2 time, the MSFragger/recon ratios,
  the Pass-1 MS2 table, the liver four-tool tables, PXD013608, Chick 2015 and the
  Preview floor. Citations were confirmed by lookup. Every other number is traced
  to NOTES or JOURNAL but was not re-derived from raw output.

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
  On liver alone: 27,678 → 32,496 PSMs (+17.4 %), 118 → 82.9 s. Quote the NOTES
  table itself; the prose around it rounds differently (35–41 %, 7–12 %).

**Open / flags**
- The TOF, ion-trap and Astral rows are Ben's working values plus padding, not
  measurements (Appendix E, row 6). The code comment on the TOF constant cites
  a "30 ppm default" and should be updated to that rationale.
- Limitation for the note: the 13 non-trypsin presets are transcribed from
  Mascot and have not been run on real data.

---

## Phase 2. Pass 1: wide open search

**Implementation** (`defaults/open-search.json`, guarded by `sage_runner.rs`
`write_effective_params_from_text`)
- Settings:
  - `precursor_tol da [-500, 100]`, which gives a delta window of −100..+500 Da;
  - fully enzymatic, `missed_cleavages 2`, length 7–50;
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
  (2026-08-17), and it is confirmed empirically: every open-search TSV spans
  deltas of exactly −100.00 to +500.00 Da (serum), and a ppm window written
  `[-30, 5]` produced observed errors of −5.047 to +30.007 ppm (serum). A
  direct test was also run: Sage v0.15.0-beta.2 on serum with `da [-3.5, 1.25]`
  gave observed deltas of −1.250 to +3.499 Da (32,221 PSMs;
  `~/Documents/proteomicsTesting/2026-9-9-serum/`). The inversion therefore
  holds in both units. A comparison script built on the wrong reading
  had missed 6 bcell peaks above +100 Da. *Design:* the convention is locked in
  `sage-config-and-gotchas.md`, and the Pass 2 ppm window is written
  sign-inverted (`pass2.rs`).

**Open / flags**
- The window is Sage's own documented open-search setting (Appendix E,
  row 10). The extra −100 to −150 Da that FragPipe searches holds almost
  nothing on these files. This is a candidate for improvement, not a known
  loss.
- `chimera true` and `report_psms 2` are also Sage's documented open-search
  example settings. Recon's Pass 1 template is that example (PXD001468 page)
  with mods removed. With `chimera` on, Sage finds the best peptide for a
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
3. **Histogram:** 0.01 Da bins, bins with ≥5 PSMs, prominence > 0.3. At most 50
   peaks. Each PSM is assigned to its nearest centre only (`PeakAssignmentMode::Merge`).
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

**Evidence it works** (liver; Sage v0.15.0-beta.2, `full-run/liver.json`;
re-derived 2026-09-24 with `liver_mod_rank_comparison.py`)
- None of the four alkylation-agnostic tools was told the sample was alkylated.
  recon, PTM-Shepherd, MetaMorpheus and Mascot all rank Oxidation first and
  Carbamidomethyl second among modifications. Unmodified is rank 1 overall;
  recon's +57.0207 is rank 3 with 1,279 of 31,793 PSMs (4.02 %).
- The absolute counts differ by tool (+57: recon 1,279, PTM-Shepherd 2,048,
  MetaMorpheus 1,398, Mascot 2,479), so the claim is rank agreement only.
- Spearman ρ on shared masses: vs PTM-Shepherd +0.609 (n = 39, p = 5e-05), vs
  Mascot +0.472 (n = 28, p = 0.012), vs MetaMorpheus +0.943 (n = 6, p = 0.017).
  The MetaMorpheus n is thin by construction (G-PTM-D searches a curated list),
  so never quote that ρ without its n. The v0.14 values (+0.616 / +0.502 /
  +1.000) are superseded.

**Open / flags**
- Provenance of every histogram and fold parameter is in Appendix E (rows 1–3).
  In short: the 0.3 prominence ratio is PTM-Shepherd's and means the same thing
  there. The bin width, count floor, peak cap and roll-up were set by the coding
  agent (Cline, 2026-07-07 and 2026-07-14) and never compared with alternatives.
- Planned for the next release: recon's prominence search takes the *first*
  higher bin within ±0.5 Da in array order (the farthest to the left, not the
  nearest), and it ignores empty bins, because only bins with ≥5 PSMs are
  considered. Both differ from PTM-Shepherd's topographic prominence. Change it
  to the nearest higher bin over the full histogram, and report which liver
  peaks move.
- The two-mod Unimod decomposition is declared (`"combination"` source) but not
  implemented.

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
- **MS2:** Sage's `fragment_ppm` (absolute): median, MAD, p95. The recommendation
  is a ppm rung, or for a Da analyzer, the ppm at m/z 500 ×2 rounded up to 0.1 Da.

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

**Evidence it works**
- The corrected MS1 bias falls between MetaMorpheus and MSFragger on bcell and
  b1906.
- **Liver MS1 bias agrees across three tools** (all measured before any
  recalibration, on the same raw file):
  - recon: −1.417 ppm (MAD 0.63, n = 7,574 clean-subset PSMs; v0.15);
  - MSFragger first-search calibration: −1.43 ppm (MAD 0.97), from the FragPipe
    log of the PTM-Shepherd liver run in the four-tool comparison;
  - MetaMorpheus Calibrate task, first round: −1.57 ppm (IQR 1.23).

  **Anomaly: Byonic Preview reports 0.0 ppm** (|error| 0.5 ppm, 943 high /
  850 low precursors, pre-recalibration). Preview is therefore the outlier, not
  recon. Possible explanations, none tested: Preview works from its own
  `.mgf` conversion and may re-determine precursor m/z; its population is much
  smaller (~1,800 precursors vs 7,574 PSMs); or its "before recal" figure
  already includes an internal correction. The note should not use Preview as
  the MS1 reference.
- **Liver MS2:** the signed MS2 error agrees across the three tools that report
  a sign: Preview −3.1, MSFragger −3.02, MetaMorpheus −2.99 ppm. recon reports
  only |error|, and its value depends on the population:
  - 3.27 ppm on the calibration clean subset (`ms1_calibration.ms2_median_abs_ppm`);
  - 3.38 ppm over all kept PSMs (`mass_accuracy.fragment_median_ppm`).

  Preview's |error| is 3.5 ppm. NOTES quotes the 3.38 figure as the Preview
  match; say which population is used.

**Open / flags**
- The clean-subset cut, the 60 % trim and the Da conversion are traced in
  Appendix E (rows 4–5).
- **Planned for the next release: the ppm-to-Da conversion moves from m/z 500
  to m/z 600.** The shipped code converts at 500, justified by a comment that
  most fragments fall at 400–600 m/z. The data contradicts that: on serum,
  matched fragments (Sage v0.15, 10,511 PSMs at q ≤ 0.01, 154,842 fragments)
  have a median m/z of 652, an intensity-weighted median of 732, and only
  17.6 % fall in 400–600 (reproduced by `serum_window_and_fragments.py` in the
  private repo). The intended point, from the 2026-08-28 design discussion,
  was 600. It affects only the Da recommendation for ion-trap or quadrupole
  MS2. The note describes the shipped behaviour and states the correction.
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
  peak: on liver it is Oxidation (1,709 PSMs), so the floor is 341.8 PSMs.
- **Satellites** (largest peak + n·1.003355, n = 1..2, within ±6 mDa) are
  demoted.
- **Protein-terminal and Met-loss candidates** are tested at protein position 0.
- **Fixed vs variable** is MetaMorpheus's `MT == "Common Fixed"` label,
  inherited and stated as such.

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

**Open / flags**
- Limitation for the note: the 20 % floor was chosen by passing the step-2 gates
  on the three development files, so it is fitted, not derived. The 0.95
  saturation cut has a statistical reason, and the carpet windows are a
  diagnostic only (Appendix E, row 7).
- Known consequence: Hydroxylation-P scores OR 1.34, so recon cannot recommend
  Oxidation on P, which all four other tools report. 58.6 % of curated entries
  sit at a contested mass.
- The tier background ignores `--q-threshold` (fixed at 0.01).
- Future work, the "claim test": show that a recommendation changes a real
  search. A natural first case is Fe[III] (+52.911 on D/E). recon recommends it
  as variable on liver (149 PSMs, routed by residue specificity), and it is not
  part of a typical default search. PTM-Shepherd (123) and MetaMorpheus (117)
  also see +52.9105 on this file; Mascot's error-tolerant list does not. Re-search liver with and without it, and
  compare IDs at q ≤ 0.01.
- b1906: 0 protein N-term recommendations, while MSFragger finds 137 N-term
  acetyl PSMs.

---

## Phase 6. Contaminants and glycopeptide screen

**Implementation**
- **Polymers** (`polymer.rs`): a port of mzSniffer (W. E. Fondrie, Apache-2.0).
  - 17 series: PEG ×3 charge states, PPG, Triton X-100/X-101 variants,
    polysiloxane, Tween-20/40/60/80, IGEPAL.
  - For each MS1 scan and each expected series m/z, it takes the most intense
    peak within 10 ppm, and sums those intensities over all series m/z and
    all MS1 scans.
  - **%TIC = that sum / the sum of every MS1 scan's TIC.** Each scan's TIC comes
    from the mzML (MS:1000285), or the sum of its peaks when the mzML has none
    (`mzml.rs` `get_spectrum_tic`).
- **Oxonium** (`oxonium.rs`): 8 ions, 20 ppm. An MS2 scan is a candidate if ≥2
  ions appear in its top 10 % of peaks (by intensity), and HexNAc 204.0867 is
  mandatory.
  - **% = candidate scans / all MS2 scans in the mzML** (`compute_screening_summary`).
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
16 original polymers (Phase 8 validation).

**Open / flags**
- **Oxonium rule, stated as an assumption.** A scan counts if at least 2 of the
  8 ions appear among its top 10 % most intense peaks, one of them is HexNAc
  204.0867, and each lies within 20 ppm. It is a hybrid of two published rules
  from the Perplexity digest `oxonium-ions.md`:
  - "≥2 ions in the top 5 %, 204 mandatory" (a GPQuest-based workflow);
  - "≥2 ions in the top 10 %".

  The 20 ppm has no source, and none of it came from mzSniffer (which screens
  polymers only). Planned: review by a glycoproteomics expert (Nick Riley or
  Chris Ashwood).
- **Polymer screen.** The 10 ppm tolerance is mzSniffer's default (confirmed on
  its GitHub). The Low / Moderate / High cut-offs (< 0.1 / < 1 / < 5 % TIC) are
  recon's own, since mzSniffer reports no levels. They have no source, and they
  conflict with an older note (< 5 / 5–15 / > 15 %).
- **Improvement for both screens:** use the measured Pass-1 MS1 error for the
  polymer tolerance and the measured MS2 error for the oxonium tolerance,
  instead of fixed 10 and 20 ppm.

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
   - `missed_cleavages 1`, `min_len 8`;
   - no mods, asserted.
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
  dependent (56 % on liver, 10.7 % on bcell), so do not quote one figure.
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
  is 1,279.

- *Found while tracing thresholds (2026-09-24):* **Pass 2 searches with
  `missed_cleavages 1`, so a peptide with two missed cleavages cannot be
  identified in Pass 2.** recon's headline missed-cleavage rate therefore
  counts peptides with exactly one, and those with ≥2 are excluded by
  construction. On liver, Pass 1 (which allows 2) puts 2.1 % of PSMs at ≥2
  missed cleavages (`examples/liver.json` `digestion`). The note states this.
  Planned for the next release: compare Pass 2 at (2 missed cleavages, length 7)
  with (1, 8) on liver, then make the two passes consistent (Appendix E,
  row 11).

**Evidence it works** (liver. recon row: Sage v0.15.0-beta.2, from
`examples/liver_pass2.json`, 10,773 peptides. Other rows: each tool's own
output, reclassified by `liver_four_tool_digestion.py`, so they do not depend
on the Sage version.)

| source | missed cleavage | ragged-N | ragged-C |
|---|---|---|---|
| recon Pass 2 | 17.53 % | 6.94 % | 3.14 % |
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
  inside that range. The v0.14 recon row (17.12 / 6.75 / 3.01 %) is superseded.
- The 32× and 6.6× ratios above come from development files at Sage v0.14. The
  note uses the liver four-tool spread instead.
- Supporting evidence from sample type: MSFragger's semi-tryptic run gives serum
  38.0 % semi, consistent with biofluid biology.

**Open / flags**
- The Preview comparison uses Preview's own liver report
  (`liver-full-results/Preview/`), not Davis 2019 Table 3.
- The note quotes one semi-enzymatic class FDR: 9.58 % on liver, from the
  shipped v0.15 example. Other values in the record (10.10, 9.42, 10.32 %) come
  from earlier builds.
- Sage v0.15 enables protein grouping by default. It is held off, and adoption is
  undecided.

---

## Phase 8. Report

- `<base>.json` (schema 3.2.0) and `<base>_pass2.json` (1.1.0), plus an HTML
  report.
- HTML sections: meta, Detectors, Mass accuracy, Contamination, Glycopeptides,
  Digestion, Recommendations.
- JSON only: signal fate, alkylation check, peak histogram, QC mass accuracy.
- Provenance recorded in the report: recon version, Sage version and commit,
  enzyme, FASTA, discovery settings, and a per-decision audit trail.

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
- Sage is not bit-deterministic: bcell PSMs were 72,801–72,803 over n = 7.
  recon's own code is deterministic. Tests assert bands, not equality.

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
- The full liver results of every tool, and the serum window-sign and fragment
  data, are in the private repo under `_dev/testing/reference-data/`
  (`liver-full-results/`, `serum-window-and-fragments/`, added 2026-09-24).
- Evidence base: four tryptic Orbitrap files. Nothing has been run on a
  non-tryptic digest or an ion trap. This is the headline limitation.

### §P. Development process and AI use
- The work was done by AI coding agents under continuous human review. The
  protocol lives in `_dev/dev_AGENTS.md`:
  - the prime directive is "never assume, always check";
  - a summary is not a source;
  - a contradicting result outranks the hypothesis;
  - one agreeing case is not validation.
- Tripwires: a numerical validation harness (14 → 17 gates) against committed
  reference outputs and cross-tool results. A gate is not done until a
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
  - Perplexity/Sonar to research and draft several reference notes. These are
    secondary digests, and some still carry `[cite:N]` placeholders.

---

## §R. Design alternatives considered and rejected

| Alternative | Why rejected | Decided |
|---|---|---|
| Composite digestion score (0–100) | Scored normal serum biology (31.8 % semi) as "64.2/100 Acceptable" because the rubric assumed cell culture; recon cannot know sample type | 2026-07-15 |
| Signal fate / three-layer MS1 ("where the signal goes") | Four measured defects: counted Pass-1 IDs only; "peptide-like" was an m/z window; contradicted polymer %TIC; headline moved with a default tolerance (6.99 vs 7.7 %). Code preserved in `_dev/extracted/` | 2026-09-01 (report), 2026-09-02 (code moved) |
| Separate closed reference search for MS1 bias | Redundant once the open search's clean subset gives the bias; it served only as a cross-check (it matched FragPipe on b1906, +0.53 vs +0.53 ppm) | 2026-09-01 |
| Satellite folding | Violates conservation; replaced by demotion | 2026-07-14 |
| A single search only | Tolerances cannot be chosen before they are measured; two searches, no re-run loop | 2026-08-17 |
| Isotope-error search in Pass 1 | Fabricates modifications (Propionyl) | NOTES, CLOSED-NEGATIVE |
| m/z calibration before discovery (C1/C2) | Three negative arms | 2026-07-16 |
| Mascot paired target-decoy carry-forward | +33 proteins (+7.3 %) drawn from the false-positive tail; Sage already generates one decoy per target | 2026-08-28 |
| Unique-peptide histogram basis | Erases modifications; loses Formylation and 45 % of the carpet margin | NOTES, histogram basis |
| **Still shipped, should go:** the alkylation check and other JSON-only blocks | Leftovers from earlier designs; see Appendix D | open |

---

## §C. Sources

Verified means confirmed against a vendored PDF or by lookup on 2026-09-24.

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
| Müller T, Winter D. *Mol Cell Proteomics* 2017 (PMID 28539326) | over-alkylation | from note; not re-checked |
| Benjamini Y, Hochberg Y. *J R Stat Soc B* 1995 | Phase 5 | standard; add |
| mzSniffer (Fondrie), github.com/wfondrie/mzsniffer | polymer port | verified. Upstream's last commit is `e6c3317d` (2023-03-13), so the July 2026 port is of that commit |
| MetaMorpheus @ `7e453540` | curated mods | verified (THIRD_PARTY_LICENSES) |
| mzdata crate v0.65.5 (J. Klein) | mzML reading | planned: add to the third-party credits |
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
| Spearman ρ +0.616 / +1.000 / +0.502 | v0.14; replaced by +0.609 / +0.943 / +0.472 at v0.15 |
| Liver recon digestion 17.12 / 6.75 / 3.01 % | v0.14; replaced by 17.53 / 6.94 / 3.14 % |
| "Do not present recon's MS1 bias as corroborated" (NOTES) | superseded: MSFragger −1.43 and MetaMorpheus −1.57 agree with recon −1.42; Preview is the outlier |
| Serum runtime 95.2 s (README) vs 93.2 s (NOTES) | reconcile first |
| Liver parsimony saving "56 %" as a general figure | file-dependent (bcell 10.7 %) |

## Appendix B. Closing ledger (settled with Ben, 2026-09-24)

Every point raised while building this material ends in one of three places.

**Decided**
- The note's evidence is the liver file; the other three files are
  development history.
- Commit hashes are not cited. The development history is a private working
  record (private repo and NIST GitLab archive).
- AI use is disclosed in `docs/AI_USAGE.md`: Cline (Claude Opus and Sonnet),
  Claude Code, and Perplexity.
- Datasets:
  - bcell is PXD004352 (Rieckmann et al. 2017);
  - serum is a NIST SRM 909c QC run from a forthcoming dataset, with no
    reference yet;
  - liver and serum were acquired on the lab's Orbitrap Fusion Lumos.
- The abundance floor is Ben's Mascot-practice rule of thumb (10 % of the
  alkylation count), which became 20 % of the largest non-zero peak in code.
- The window sign is confirmed by Lazear (personal communication) and by a
  direct test (`da [-3.5, 1.25]` → deltas −1.250 to +3.499 Da).
- The Pass-1 window, `chimera` and `report_psms 2` are Sage's documented
  open-search example.
- Liver MS1 bias: recon agrees with MSFragger and MetaMorpheus; Preview's 0.0 is
  stated as an anomaly.
- Wilmarth's guide is cited from its public Codeberg repository.
- The Preview comparison uses Preview's own liver report, not Davis 2019
  Table 3.
- Tool versions are recorded: Mascot 2.6.0, and the rest in §D.
- The four-tool liver spread replaces the development-file acceptance ratios.
- One semi-enzymatic class FDR is quoted: 9.58 % (liver, v0.15).
- The polymer level bands are presented as recon's own display choice.
- The serum trypsin/Lys-C digest searched as trypsin: no action, since the
  file was only a test.
- The pre-repo "spec" in the first plan was Ben's own early ideas. It needs no
  citation.

**Planned for the next release** (the note says these are coming)
1. **ppm-to-Da conversion at m/z 600, not 500**, for ion-trap and quadrupole
   MS2 recommendations.
2. **Pass 2 missed cleavages and length.** Compare (2, 7) with (1, 8) on liver,
   then make the passes consistent. Until then the note states that ≥2 missed
   cleavages are excluded from the headline rate.
3. **Prominence:** use the nearest higher bin over the full histogram, as
   PTM-Shepherd does, and report which liver peaks move.
4. **Screens:** tie the polymer tolerance to the measured MS1 error and the
   oxonium tolerance to the measured MS2 error. Have the oxonium rule reviewed
   by a glycoproteomics expert (Nick Riley or Chris Ashwood).
5. **Remove vestigial code and JSON** (Appendix D). This is a MAJOR schema
   change.
6. **Fix the documentation defects** in Appendix C, including adding mzdata to
   the third-party credits.
7. **The claim test:** re-search liver with and without the recommended
   Fe[III], and report the change in identifications.

**Stated as limitations in the note**
- The evidence base is tryptic Orbitrap data. The 13 other enzyme presets and
  the Da regime are unexercised on real files.
- The floor (20 %) was fitted on the development files.
- Several parameters were set by the coding agent and never compared with
  alternatives. They are listed in Appendix E and stated as choices:
  - 0.01 Da bins, the ≥5 count and the 50-peak cap;
  - the 60 % hyperscore trim and the 10 % spectra rule;
  - the 4.5 mDa fold step.
- recon reports un-localized delta-bin fractions (a rank statistic), not
  occupancy.
- The MS1 recommendation is not analyzer-aware.
- Deamidation (+0.984) and a mis-called ¹³C peak (+1.003) are not resolved by
  a pooled open-search histogram.

## Appendix C. Documentation defects to fix in the next release

- `THIRD_PARTY_LICENSES.md`:
  - the Sage entry still says recon "invokes an unmodified Sage binary";
  - mzdata is missing;
  - the HTML footer credits Pyteomics, which has no entry.
- Pass-2 window described three different ways: README ("bias-centred rung"),
  the console line in `main.rs` ("±100 ppm cap"), and the `main.rs` docstring
  ("Pass 2 does NOT yet run").
- `defaults.rs`: the header says Unimod is "NOT bundled", but it is embedded.
- `mzml.rs` comments say 50 ppm where the constants are 20.
- The neutron mass is used as the isotope spacing in `glossary.md`,
  `domain-primer.md`, `fallback-mod-table.md` and `sage-config-and-gotchas.md`.
  The last also carries a wrong "52.91 Da = triply ¹³C" claim.
- `result-schema.md` body is v1.0.0; only its changelog is current.
- README `--params` / `--pass2-params` understate what the code overrides.
- The HTML digestion section mixes peptide-basis rates with a PSM-basis N:C
  ratio.
- `byonic-preview-methodology.md` still says the PDF is not vendored.
- `liver_mod_rank_comparison.py` still reads `reference-data/unimod.xml`, which
  moved to `recon-tool/resources/`; it fails as committed. (It was run on
  2026-09-24 with the path patched in memory only.)
- The committed example and full-run reports were built from dirty trees
  (`270a357-dirty`, `b62c331-dirty`), so `git_commit` does not pin an exact
  source state.
- `tier_assignment.rs` module header says `q < 0.05`, but the code tests
  `q <= Q_MAX` (fixed 2026-09-01; the header was not updated).
- Reference notes produced with Perplexity are secondary digests. Treat their
  links as leads. `VENDOR-CHECKLIST.md` names six:
  - sage-config-and-gotchas, unimod-decomposition, ptm-shepherd-methodology,
    oxonium-ions, polymer-contaminant-ions, mgf-mzml-intensity-differences;
  - `unimod-classification-filtering` states it too.

  Several others have the same web-digest form and unresolved `[cite:N]`
  placeholders (mass-error-reporting, incomplete-alkylation-detection).

## Appendix D. Vestigial code and JSON to remove in the next release (checked against code, 2026-09-24)

README "Future work" item 8 already commits to this: "a field in the output
implies a claim we are making". The HTML report (`generate_html_report`) reads
only `input`, `mod_discovery`, `ms1_calibration`, `polymer`, `oxonium`,
`recommendations` and `analyzers`. Everything else in the main JSON is JSON-only
(and often console-only).

**Main report JSON (`<base>.json`), blocks with no HTML use**
- `alkylation`: the fixed-CAM check (−57 Da Cys search). On liver it reads
  `"fixed_mod_assumed": "Carbamidomethyl (+57.02 Da) on C"` and "✓ Alkylation
  appears complete", which contradicts the alkylation-agnostic design. README
  line ~387 describes it as "observed cysteine chemistry". This is the most
  misleading leftover.
- `mass_accuracy` (`qc.rs`): summarises Sage `precursor_ppm` over all kept
  open-search PSMs. On liver, `precursor_p95_ppm` is 76,782 ppm, which is
  meaningless in an open search (`domain-primer.md` says so). Superseded by
  `ms1_calibration`. **Its fragment fields are not useless:**
  `fragment_median_ppm` (3.38 on liver) is the all-PSM MS2 |error| that NOTES
  compares with Preview. Decide which MS2 population the report states before
  removing the block.
- `digestion` (Pass 1): missed cleavage on the fully enzymatic Pass 1, plus
  `ragged_ends_pct`, which is 0 by construction. The defined measurement is
  Pass 2 `composition`.
- `signal_fate`: already named in README Future work 8.

**Pass 2 JSON (`<base>_pass2.json`), overlapping digestion measures**
- `composition` (distinct peptides, per-class decoy correction) is the defined
  measurement.
- `digestion` (PSM basis) also reports a semi-enzymatic rate (liver 9.48 %).
- `terminus` and `comparison` report a third (9.21 %).
- Keep one measurement, or label the others as supporting, so a reader cannot
  quote the wrong one. The HTML's N:C ratio also comes from `terminus` (PSM
  basis) while its rates come from `composition` (peptide basis).

**Code**
- 11 hidden development subcommands in `main.rs`: `parse`, `detect-analyzer`,
  `compare-peak-assignment`, `discover`, `signal-fate`, `mzml-stats`,
  `polymer-stats`, `oxonium-screen`, `digestion-stats`, `qc-stats`, `analyze`.
  Some are useful for the validation harness. Decide which the harness needs.
- Alternative modes reachable only through hidden `discover`:
  `CalibrationMode::None` / `PpmConstant` (the negative C1 arm) and
  `PeakAssignmentMode::Split`.
- `mod_discovery.rs`: the `"combination"` source is declared, but nothing
  produces it (two-mod decomposition was never built).
- `calibration.rs`: `PASS2_HALF_WIDTH_CAP_PPM` is `#[deprecated]` with no users.
- `detect_analyzers` runs twice per `run` (`main.rs`, before Pass 1 and again
  for the report). The second reads the mzML again to record the first
  decision.
- Satellite folding (`enable_satellite_folding: false`) is **disabled by
  design** and protected by `dev_AGENTS.md`. Do not remove it without an
  explicit decision.

Removing JSON blocks is a MAJOR schema change (currently 3.2.0), and the
committed example reports and regression snapshots must be regenerated.

## Appendix E. Threshold provenance (traced 2026-09-24)

Sources: git history, JOURNAL, NOTES, reference notes, upstream code, and Ben's
answers (2026-09-24). "Agent" means the value was set by the coding agent,
Cline before 2026-07-15 and Claude Code after, with no recorded external source.

| # | Value | Origin | Evidence or status |
|---|---|---|---|
| 1 | Histogram bin 0.01 Da | Agent, Phase 3 (2026-07-07). PTM-Shepherd uses 0.0002 Da bins with smoothing (`histo_bindivs` 5000); Preview bins to integers | Never compared with alternatives. DeltaMass argues against fixed bins (KDE) |
| 1 | Count floor ≥5 PSMs per bin | The agent's 2026-07-06 plan says "e.g., count > 5". It was 10 at Phase 3 and set to 5 at Phase 7B (2026-07-14) | Only `min_peak_count` moved the prominence values (NOTES "Config-provenance gap CLOSED") |
| 1 | Peak merge 0.01 Da (one bin) | Phase 7B: tightened from 0.02 once calibration removed the offset, so that deamidation is not bridged (`mod-discovery-calibration.md`) | Reasoned, not measured |
| 1 | At most 50 peaks | Agent, Phase 3. PTM-Shepherd reports 500 | No rationale |
| 2 | Prominence ratio 0.3 | PTM-Shepherd `peakpicking_promRatio` = 0.3 (README), with the same definition: topographic prominence / height > 0.3 (`Prominence.java`, `PeakPicker.java`, master `61eebcb`) | Same concept and value. Implementation differs (Phase 3 flag) |
| 3 | Near-zero population \|Δ\| < 0.1 Da (calibration) | Phase 7B. The zero peak is smeared ±0.05–0.1 Da (`mod-discovery-calibration.md`); DeltaMass and PTM-Shepherd "zero-peak correction" | Reasoned |
| 3 | "Unmodified" roll-up \|Δ\| < 0.075 Da | Phase 7B: "capture the full zero smear (~0.06 Da) but stay under 0.1 Da" (`mod_discovery.rs`) | The 0.075–0.1 band is a known leak (`ptm-stratification-design.md`). The ~0.06 smear is not sourced |
| 3 | Isotope fold 12 mDa + 4.5 mDa per step | Phase 7B. The 12 mDa keeps deamidation clear of the k=1 window; the per-step widening "catches satellites that drifted further at higher k" (JOURNAL Phase 7B). The plan had said 10 mDa | Decision: the note uses the arithmetic, 1.003355 − 0.984016 = 19.3 mDa, so a 12 mDa window leaves 7.3 mDa after calibration. The record's other figures (11.2, 9.65 mDa) are not used. The 4.5 mDa step is stated as a choice |
| 4 | Clean subset \|Δ\| < 0.02 Da | Design lock 2026-08-17 (NOTES "MS1 error from the wide search's clean subset") | It imposes a mass-dependent ppm ceiling (4.1 ppm at 5000 Da, 31.9 ppm at 500 Da), so it measures a centre, not a tail (NOTES) |
| 4 | Hyperscore trim 60 %, if ≥200 PSMs and ≥10 % of MS2 | The 50–70 % comes from the 2026-08-19 architecture sketch; 60 % is its midpoint, picked at implementation (`c47f624`) | The 200 is documented and not load-bearing (bootstrap; NOTES "Clean-subset PSM floor"). 60 % and 10 % are agent choices. JOURNAL 2026-08-19 Q1 calls them "reasonable guesses, not validated" |
| 5 | MS2 Da = ppm at m/z 500, ×2, rounded up to 0.1 Da | Ben and the agent, in chat, 2026-08-28. Resolution is quoted at m/z 200, but searches take Da while Sage reports ppm, and ppm maps to different Da across m/z. A table of Da error at several m/z was made | Planned for the next release: change to m/z 600, the intended point. Measured serum fragments: median 652 m/z (Phase 4) |
| 6 | Pass-1 MS2: TOF 100 ppm, ion trap 1.0 Da, Astral 20 ppm, Orbitrap 20 ppm | Ben's working values plus padding: TOF ~30 ppm in his experience (timsTOF up to ~60) → 100; ion trap 0.6 Da typical → 1.0; Astral ~10 ppm (no direct experience) → 20 | Experience, not measurement. Only Orbitrap is measured (50 → 20 ppm; Phase 1) |
| 6 | Analyzer read from the first 100 MS2 scans | Ben: enough to catch analyzer switching within a method. MS2 scans, not all scans, because a run can begin with ~10 min of MS1 only. Chosen empirically for speed vs coverage | Empirical |
| 7 | Background saturation 0.95; carpet windows ±0.85–1.15 and ±1.85–2.15 Da | Agent, during the step-2 work on the ±1 Da "forest" (2026-08-24/25). 0.95 has a stated statistical reason: above it the 2×2 table saturates (Carbamyl's K/R/C/M sit in 99.6 % of tryptic peptides; `tier_assignment.rs`) | The carpet feeds no decision. It is computed and written to the JSON (`carpet_margin_psms`) as an invariant check: the floor must sit above the tallest ±1/±2 Da peak it governs |
| 8 | Oxonium: 20 ppm, top 10 %, ≥2 ions, 204 required | A hybrid of two published rules (Phase 6); 20 ppm unsourced | Planned: review by Nick Riley or Chris Ashwood; tie the tolerance to the measured MS2 error |
| 9 | Polymer: 10 ppm; levels < 0.1 / < 1 / < 5 % TIC | 10 ppm = mzSniffer default. The levels are recon's own | Decision: the note presents the levels as recon's own display bands. Planned: tie the tolerance to the measured MS1 error |
| 10 | Pass-1 window −100..+500 Da | Sage's documented open-search example (`da [-500, 100]`, docs pages "Example: PXD001468 (Open-Search)" and "Search Tolerances"). FragPipe uses −150 | Across all four files in PTM-Shepherd (which searched to −150), one peak lies below −100: an unannotated −130.07 with 11 of 97,500 PSMs (0.011 %), and liver has none (lowest peak −44). The extra 50 Da holds almost nothing here |
| 11 | Pass 2 `missed_cleavages 1`, `min_len 8` (Pass 1: 2 and 7) | Agent, Phase 6C design (2026-07-08). It was meant to shrink the semi-enzymatic search space, which was the reason Pass 2 moved to a subset. No other rationale is recorded | Ben's reading (one fewer missed cleavage, one more residue for a semi peptide) is plausible but untested. It caps the missed-cleavage measurement (Phase 7). Planned for the next release: compare runtime and composition at (1, 8) and (2, 7) on liver, then make the passes consistent |
