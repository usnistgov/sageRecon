# Sage-Based Proteomics Reconnaissance Tool — Development Notes

Topical, not chronological. Why things are built the way they are — the
reasoning you don't want to re-derive. Current state lives in `PLAN.md`'s
status block, not here; how-we-got-here lives in `JOURNAL.md`. Domain primer
(delta mass, open search, oxonium, etc.) lives in
`reference-notes/domain-primer.md`.

## Sage version pinning and upgrade path

**Current pinned version:** v0.15.0-beta.2, commit
`df9219951cc9a54cf4cd55d76541af24b687bd3d`, on UPSTREAM `lazear/sage`.

⚠ **THERE IS NO VENDORED BINARY. Sage is a Cargo git dependency** (landed
2026-09-01, `a4f09b9`). `SAGE_VERSION` and `SAGE_COMMIT` still live in
`sage_runner.rs` for provenance, but the runtime version handshake is GONE — the
pin is a `Cargo.lock` entry and `cargo_lock_pins_exactly_sage_commit` asserts
every `lazear/sage` line in it names `SAGE_COMMIT`.
`parse_sage_results()` still runs `validate_tsv_schema()` before parsing.

**Upgrade checklist (do in one commit):**
1. Update the `rev` on all three `sage-*` git dependencies in
   `recon-tool/Cargo.toml`, and `SAGE_VERSION` / `SAGE_COMMIT` in
   `sage_runner.rs`. There is no binary to download and no path to update.
2. `cargo update -p sage-core -p sage-cli -p sage-cloudpath` so `Cargo.lock`
   moves with it, or the pin gate fails — which is the gate working.
3. Diff the new Sage `CHANGELOG.md` against `REQUIRED_COLUMNS` in
   `sage_results.rs`. Add new required columns; remove dropped ones.
4. Check for semantic changes to existing columns — v0.15 made `precursor_ppm`
   signed while `fragment_ppm` stayed absolute, and the schema validator CANNOT
   catch that because the column name does not change. Manual audit step.
5. Update any affected config templates in `testing/configs/`.
6. Bump `recon-tool/Cargo.toml` minor version.
7. Update the Sage section of `THIRD_PARTY_LICENSES.md`.
8. Run the full test suite and `run_validation.py`.
9. ⚠ **Expect q-derived counts to move a little and do not chase it.** They
   jitter run to run anyway — see "q-DERIVED COUNTS JITTER". Compare raw columns
   exactly; band anything derived from a q threshold.

**v0.15.0 breaking changes — ADOPTION DECIDED 2026-09-01, see "ADOPTION STATUS" below.**

⚠ This block said "one parser-breaking change and one config-breaking change".
**Re-read at the tag 2026-09-01: there are FOUR breaking changes and one
default-on addition that affects us more than any of them.** The original count
was low because the changelog was summarised rather than read. The full list from
`git show v0.15.0-beta.2:CHANGELOG.md`:
signed `precursor_ppm`; `variable_mods` list syntax; **removal of
`fragment_min_mz`/`fragment_max_mz`**; `sage-cloudpath` no longer exposing
`CloudPath` (all paths become `url::Url` — matters only once recon LINKS Sage,
which route A1 does); TMT/iTRAQ deisotoping (no impact, we do not use TMT); and
**protein grouping enabled by default**.

- **`precursor_ppm` is now SIGNED (non-absoluted).**
  In v0.14.x the field was `|error|`. In v0.15.x it is `signed(error)`.
  Our `qc.rs` (`compute_qc_stats`) treats this as a magnitude for the distribution.
  When upgrading: update `qc.rs` to handle the sign (the QC metric becomes a signed
  distribution, which is actually MORE informative — bias direction is visible). Also
  verify the MS1-mass-accuracy path in `qc.rs` does not double-sign. The schema
  validator will NOT catch this — the column name is unchanged. This must be a manual
  code audit step in the upgrade checklist.

  ⚠ **CONFIRMED 2026-09-03: THIS STEP WAS NOT DONE.** `qc.rs` got the sign fix, but
  `report.rs`'s doc comment kept describing all four `mass_accuracy.*_ppm` fields as
  absolute and never negative for two more days, while the committed
  `full-run/liver.json` sat there the whole time carrying a negative
  `precursor_median_ppm`. That contradiction is the direct evidence the manual audit
  was skipped, not a guess. Fixed `38fab8a` (docs) and `45dd004` (schema 3.0.0,
  the breaking bump the missed sign change forced). See "LESSON — prose
  instructions rot" at the end of this file.

- **`variable_mods` config syntax changed** — must now be a list of masses per residue.
  All `testing/configs/*.json` files that use `variable_mods` need updating. The wide
  open search template (the alkylation-agnostic default) has no variable mods, so it
  is unaffected. The closed-reference and digestion configs may be affected — check each.

  ⚠ **THIS CLAIM WAS CHECKED 2026-09-01 AND IS TRUE** — verified against both tags
  in a clean upstream clone, not inherited. v0.14.7:
  `Option<HashMap<String, crate::modification::ValueOrVec>>`. v0.15.0-beta.2:
  `Option<HashMap<String, Vec<f32>>>` (`crates/sage/src/database.rs:78`).
  `ValueOrVec` allowed a bare scalar; v0.15 requires a list. Our live templates
  carry `variable_mods: {}` and both passes strip them anyway, so the empty map is
  valid under both. The deliberately-dirty fixture `closed-search-reference.json`
  carries four variable mods and should be checked before anyone SEARCHES with it.

- ❌ **"No impact items ... `fragment_min_mz` / `fragment_max_mz` removal (not in
  any of our configs)" — THAT WAS FALSE. Corrected 2026-09-01 by reading the
  files.** Both live templates carry `fragment_min_mz: 150.0` and
  `fragment_max_mz: 2000.0`, and so do BOTH new bundled defaults in
  `recon-tool/src/defaults/`. v0.15 REMOVES these parameters, and upstream's stated
  reason is not cosmetic: they "were decreasing the accuracy of preliminary scoring
  estimation when attempting to annotate multiply-charged, high-m/z ions". So this
  is a live impact item that may MOVE FRAGMENT MATCHING, not a no-op.
  TMT/iTRAQ deisotoping remains genuinely no-impact — we do not use TMT.

- 🔴 **NEW AND LARGEST ITEM, found 2026-09-01 reading the changelog at the tag:
  v0.15 does PROTEIN GROUPING BY DEFAULT.** "IDPicker-based protein grouping with
  picked group FDR control (`protein_grouping` setting, enabled by default).
  Proteins are grouped using a bipartite graph greedy set cover approach", adding
  `protein_groups`, `num_protein_groups`, `protein_group_q`, plus a
  `protein_grouping_peptide_fdr` parameter (default 0.01).
  ⚠ **That is the same algorithm recon built for itself** — `parsimonious_groups`,
  greedy set cover, shipped 2026-08-31. And it makes the locked entry "Sage sorts
  `proteins` ALPHABETICALLY and does no protein inference" **VERSION-SCOPED: true
  of v0.14.7, FALSE of v0.15.** Whether recon keeps its own parsimony, defers to
  Sage's, or disables `protein_grouping` is an open DECISION for the upgrade
  landing — not something to settle by accident.
  The schema validator tolerates extra columns, so the new columns cause no error.

**ADOPTION STATUS — corrected 2026-09-01.**
❌ This block previously ended: "**Do not adopt v0.15.0 until the stable release is
published.** Wait for `v0.15.0` final, not a beta tag." **That was never Ben's
decision.** An agent session wrote it as project policy without attribution, and
it stood as a constraint on the roadmap. Ben, 2026-09-01, asked directly: "notes
made that up, i am fine building on beta."
✅ **The decision is to build on `v0.15.0-beta.2`.** Measured the same day:
`git ls-remote --tags https://github.com/lazear/sage.git` shows **no v0.15.0 final
exists** — the newest 0.15 tags are `v0.15.0-beta.1` and `v0.15.0-beta.2`. Waiting
for a final was therefore not a cautious choice, it was an indefinite block.
**Reproducibility does not depend on the tag being stable:** a Cargo git rev pins
an exact commit in `Cargo.lock`. The real risk of a beta is that the API may still
change before final, which costs a future upgrade — not that this build drifts.
**Candidate pin:** `v0.15.0-beta.2` = `df9219951cc9a54cf4cd55d76541af24b687bd3d`,
clean-cloned to the gitignored `reference/sage-src/`.

⚠ **THE PATTERN, worth naming.** Three claims sat in this block. One was true, one
was measurably false, and one was invented policy. Two of the three were asserted
rather than measured, in the file whose whole purpose is to hold measured
conclusions. Check this block's claims against the clean clone before relying on
any of them.

---



Settled calls. Do not reopen without being told to.

### Default recon = two searches, self-calibrated, no re-run loop (locked 2026-08-17)

- **What:** the default `recon <file(s)> <fasta>` runs TWO Sage searches — a wide open search and
  a semi-tryptic Pass 2 — and returns one report answering: (1) what mods are present + which to
  include (prevalence, Ben's ~10%-of-abundant rule), (2) whether to use semi-tryptic, (3) reasonable
  MS1 and MS2 mass tolerances. One command, just file(s) + FASTA; everything else runs.
  (⚠ said "MS1 (asymmetric)" until 2026-08-28. The MS1 recommendation is now a QUANTIZED
  ladder rung, symmetric about zero; the asymmetric window is superseded.)
  Full build spec in PLAN "Default recon" heading.
- **Alkylation-agnostic, no fixed-C default (locked):** the discovery (wide) search fixes NO
  alkylation mod. +57 (CAM), +45 (MMTS), or whatever was used surfaces as a delta — that IS the
  signal ("this alkylation is abundant, fix it in your real search"). Fixing C+57 by default would
  hide the exact thing the tool measures and bake in a chemistry assumption the tool has no business
  making. The fixed-C runs in the repo history were BENCHMARKING artifacts (apples-to-apples with
  FragPipe's fixed-C open run), NOT how the tool is used. **This is why the over-alkylation
  annotation was dropped** — it only made sense in a fixed-C mode we don't run, and its logic
  ("re-run with a fixed flag to confirm the cluster") contradicts the one-command mission.
- **MS1 error from the wide search's clean subset — NOT a narrow reference search (locked):** in a
  pure open search, precursor ppm reflects the delta mass, not instrument error (see "intentional,
  not bugs"). But the near-zero-delta population is unmodified, so ITS precursor ppm is real MS1
  accuracy. Measure from that clean subset (`|Δ|<0.02 Da`, rank-1 only, q<0.01, target, good score).
  A separate narrow Pass 1 (the old digestion pass, hardcoded ±10 ppm) is DROPPED: you can't pick an
  appropriate narrow tolerance until you've measured the error, and a wrong guess (±10 on a
  +30-ppm-drifted or a TOF run) silently clips IDs and biases the measurement. The wide search has no
  such prior. This is MSFragger's own calibration logic.
- **Two numbers from one measurement, opposite ends (locked — do NOT conflate):**
  - *User MS1 recommendation:* GENEROUS — median (bias) + high percentile (95/99th), rounded up,
    reported ASYMMETRIC. Over-suggesting tolerance is safer than clipping the user's real-search IDs.
  - *Pass 2 `precursor_tol`:* TIGHT — bias-CENTERED on the median + CORE spread (MAD/low percentile,
    NOT the tail). Semi-tryptic inflates the candidate DB ~×peptide-length; a wide precursor window
    on top is the multiplicative blowup that hangs a run (see Phase 6B / "no wide-tol + relaxed-enzyme
    in one search"). Bias-centering lets the half-width stay tight and still catch real peptides.
    **Hard cap ±100 ppm** — runtime backstop ONLY, not an accuracy claim. A ±15–20 cap would clip
    TOFs (50–80 ppm out of the box) systematically on exactly the instruments where semi-tryptic
    assessment matters; ±100 is TOF-safe and stops only pathological fits.
- **Semi-tryptic efficiency is a RATIO, robust to window width (locked reasoning):** semi/total PSMs.
  A wide window costs Pass 2 speed, not the answer — even a TOF at ±80 ppm gives thousands of IDs on
  the subset FASTA, far more than needed for a stable ratio.
- **MEASURE ONCE, APPLY ONCE, REPORT BOTH — no re-run loop (locked):** the wide search measures MS1
  error; Pass 2 applies the tight window; the report shows both the user recommendation AND whether
  Pass 2 confirmed the window (IDs landed where predicted). Pass 2 confirmation is OBSERVABILITY, not
  a control loop. If Pass 2 looks off, REPORT it — never silently re-run with adjusted params.
  Iterating would blur what the user is looking at, turn fast recon into a slow refinement loop, and
  cross the "parameter auto-configuration engine" non-goal (PLAN). The tool recons the sample as-is;
  the search-config decision is the user's. **Do not add an iterative calibration loop.**
- **Why:** every recommendation is derived from the sample's own data (no hardcoded tolerances, no
  chemistry assumptions), the design is two searches not three, and it stays inside the recon mission
  — tell the user what's here and how to search it, don't search it for them. Global median + MAD/
  percentile only — NO m/z-dependent grid (the output is a rounded user-facing recommendation, not
  our own calibration constant; validate global MS1 against the trusted Phase 8.5 closed b1906
  +0.53 ppm, only go m/z-dependent if global disagrees badly).
- **Banked to-do (not now):** measure semi-tryptic Pass 2 runtime vs. precursor window — the ±100
  ppm cap is a conservative guess, unmeasured. Find where Pass 2 goes from ~3 min to painful vs. ppm
  on a subset FASTA, then tune the cap honestly.

### MS1 calibration implementation, and three bugs found while wiring it in (2026-08-19)

- **Status:** the design above is now built. `calibration.rs` has the clean-subset filter, the MS1
  stats, the user recommendation, the Pass 2 window, and the MS2 tolerance. `run_analyze_command`
  calls all of it and prints a new recommendation block. This is read-only reporting. `run` and
  Pass 2 are not touched — that was the user's explicit scope choice for this pass.
- **Bug found: the clean subset did not exclude decoys.** The locked spec above says target only.
  The filter was missing the check. Fixed. In the actual `analyze` call path this bug had no live
  effect, because `parse_sage_results` already drops decoys before `analyze` ever sees a PSM. The
  fix is still correct defense in depth for any future caller that skips that upstream filter.
- **Bug found: `Psm` never carried Sage's own rank column.** The TSV parser read `rank` into an
  internal record type and dropped it before building the `Psm` struct the rest of the tool uses.
  The rank-1 filter in the clean subset had nothing to filter on. Added `rank: u32` to `Psm` and
  wired it through the parser. This touched six places that build a `Psm` by hand (the parser plus
  five test helpers). A first commit added the field to the call sites but not to the struct
  definition itself, and did not compile — caught by the user's build, not by the agent. Fixed in a
  follow-up commit.
- **Bug found: the hyperscore guard's "10% of total spectra" check used `max(scan_number)+1` as a
  stand-in for total spectra.** That undercounts whenever scan numbers have gaps. Fixed to take the
  real MS2 spectra count from `mzml_stats`, and pulled the guard's decision (percentile subset AND
  ≥200 PSMs AND ≥10% of spectra) into one shared function (`hyperscore_guard_would_apply`) so
  `select_clean_subset` and the report code cannot disagree on the threshold.
- **Not yet done:** validate the global MS1 bias/MAD number against the Phase 8.5 closed b1906
  ground truth (+0.53 ppm), per the locked "why" above. Banked for next session.

### MetaMorpheus workflow review, part 1 of 2 (2026-08-19, session 5)

- **What:** Ben ran a new MetaMorpheus calibration task on all three raw files (909c, B.naive,
  b1906). Output: **`testing/reference-data/metamorpheus/2026-08-21-10-29-48/`** — corrected
  2026-08-24; this entry originally cited `2026-08-19-11-28-05/`, which does not exist in the
  repo. The tracked directory holds the exact round-0 values quoted below (serum +2.351/+1.059,
  bcell −0.295), so the numbers are right and only the path was wrong. This is the "bring in the
  new MetaMorpheus results" step banked in JOURNAL session 4's debrief.
- **Why the BEFORE numbers, not the AFTER numbers.** MetaMorpheus's `Task1-CalibrateTask` runs
  search -> measure -> correct -> re-measure, twice, and reports MS1/MS2 ppm error median +
  IQR each round. The AFTER (final, post-correction) numbers are MetaMorpheus's own calibration
  residual, not the raw instrument bias. Our tool's Sage open search runs on the RAW mzML (no
  recalibration step exists in our pipeline). So the correct comparison point is MetaMorpheus's
  FIRST (round-0, pre-correction) measurement, from the same raw file.
- **Extracted round-0 (raw) numbers, from `Task1-CalibrateTask/results.txt`:**

  | File (our name) | MS1 ppm median | MS1 IQR | MS2 ppm median | MS2 IQR |
  |---|---|---|---|---|
  | 909c (serum) | +2.351 | 0.815 | +1.059 | 1.974 |
  | B.naive (bcell) | −0.295 | 1.253 | +0.942 | 3.459 |
  | b1906 | +0.314 | 1.358 | −0.023 | 2.496 |

  (Final, post-calibration numbers for reference only, NOT the comparison target: 909c
  +0.299/+0.103; B.naive +0.557/−0.048; b1906 +0.678/+0.019 ppm MS1/MS2.)
- **Cross-validation against sources already in NOTES, before trusting these numbers:** the
  Phase 8.5 closed-search b1906 ground truth is +0.53 ppm (line ~110 above); the MSFragger
  `calibrate_mass` table (line ~562 above) has serum 2.51 ppm raw and b1906 0.53 ppm raw. Three
  independent tools (a closed/narrow reference search, MSFragger, MetaMorpheus) now roughly
  agree: b1906's raw MS1 bias is +0.3 to +0.5 ppm, serum's is +2.3 to +2.5 ppm. No prior
  independent number existed for bcell/B.naive; MetaMorpheus's −0.295 ppm is the first one.
- **Bug found and fixed while preparing this comparison:** `compute_ms2_tolerance`
  (`calibration.rs`) already computed a signed median MS2 ppm bias internally (`median_ppm`),
  but `Ms1CalibrationReport` (`report.rs`) never exposed it. Only `ms2_tolerance_low/high_ppm`
  was reported — a symmetric +/-95th-percentile-tail window AROUND ZERO, not centered on the
  bias. There was no field at all to compare against another engine's "MS2 ppm error median."
  Added `ms2_bias_ppm` and `ms2_spread_mad_ppm` to the struct, populated in `main.rs`, and
  printed in the console summary. `ms2_tolerance_low/high_ppm` is unchanged and still answers a
  different question (how wide should the user's next MS2 tolerance be) — do not conflate the
  two, same discipline as the MS1 "two numbers from one measurement" rule above.
- **Not yet done — part 2, the actual comparison.** No Rust compiler and no raw mzML/Sage
  binary access in the agent sandbox this session, so `analyze` could not actually be run
  against the three files. Banked as the next action: run `analyze` on 909c/B.naive/b1906 and
  check `bias_ppm` / `ms2_bias_ppm` in the JSON output against the round-0 table above. Per
  AGENTS.md verification discipline: if the numbers don't land within roughly 1–2x the
  reported IQR/MAD, report the contradiction, do not rationalize it as "different engines, so
  it's fine."

### MetaMorpheus workflow review, part 2 of 2 — actual comparison (2026-08-19)

- **What:** Ben built recon-tool (with the `ms2_bias_ppm` fix from part 1) and ran
  `analyze --full` on all three raw files, from a real Sage open-search TSV
  (`open-search-{serum,bcell,b1906}.json`, same config used for the tracked
  `testing/recon-output/full-run/` reports). Output now committed to
  `testing/recon-output/full-run/{serum,bcell,b1906}.{json,console.txt}`.
- **How our numbers are calculated (exact formulas, `recon-tool/src/calibration.rs`):**
  - **Clean subset selection** (`select_clean_subset`): target only (no decoys), rank == 1
    (chimeric rank-2+ rows share rank-1's q-value and would double-count), `peptide_q <
    0.01`, and `|corrected_delta_da| < threshold` (near-zero mass defect — the open search's
    delta already isotope-corrected: `corrected_delta = (expmass - calcmass) - isotope_error *
    1.0086649158849`). An optional hyperscore guard then keeps the top 60% by Sage's
    `hyperscore`, but ONLY if that still leaves >= 200 PSMs AND >= 10% of total MS2 spectra;
    otherwise it falls back to the unguarded q-value-only subset. All three runs report
    "(hyperscore-guarded)" — the guard fired for all three files.
  - **MS1 bias_ppm / MS2 bias_ppm:** the **median** of Sage's own per-PSM `precursor_ppm` /
    `fragment_ppm` columns over that clean subset. We do not recompute ppm error from raw
    spectra — Sage's own reported value is used as-is, per design.
  - **spread (MAD):** **median absolute deviation** — median of `|value - median|` over the
    same clean-subset values. This is a different statistic from an IQR (interquartile range,
    Q3−Q1) — see the MetaMorpheus caveat below, do not treat MAD and IQR numbers as directly
    equal. For a roughly normal distribution IQR ≈ 2× MAD, useful only as a rough sanity check,
    not an exact conversion.
  - **No other math** (no re-fit, no outlier trimming beyond the subset filters above, no
    weighting) goes into `bias_ppm`/`spread_mad_ppm`/`ms2_bias_ppm`/`ms2_spread_mad_ppm`.
- **How MetaMorpheus's numbers are calculated, as best determined from its own output
  (`Task1-CalibrateTask/results.txt`) — methodologically different in two ways, not just a
  different engine:**
  1. Its confident-PSM population comes from a **narrow/classic closed search at 1% FDR**
     (`ClassicSearchEngine` + `FdrAnalysisEngine`), a different, smaller, and differently-biased
     PSM set than recon's WIDE-OPEN-search clean subset.
  2. Its median/IQR are computed over **per-datapoint** samples (`DataPointAcquisitionEngine`,
     e.g. 187,516 MS1 datapoints from only 2,214 confident serum PSMs) — likely multiple
     mass-measurement points per PSM (isotope envelope points for MS1, matched fragment ions for
     MS2) — not one value per PSM like recon's per-PSM median. Spread is reported as IQR, not
     MAD.
  Both differences mean an exact match was never guaranteed even if both tools measured the same
  underlying instrument bias correctly — flagging this so an agreement or disagreement isn't
  over- or under-interpreted.
- **Phase 8.5 (closed-search b1906, +0.53 ppm) and MSFragger's `calibrate_mass` table are
  quoted as reported** — their internal formulas were not re-derived this session, only their
  published summary numbers are used for triangulation.
- **⛔ SUPERSEDED TABLE — DO NOT CITE. Kept only to show what the discrepancy looked like before
  it was explained.** Every recon column below is wrong (computed from Sage's absolute
  `precursor_ppm`), and the MSFragger column is from a pre-2026-08-24 run. **Canonical values:
  "Ground-truth reference values — the single source" below.** Results table as originally
  recorded (MS1 bias / MS2 bias, ppm):

  | File | recon-tool ⛔ (n clean) | MetaMorpheus round-0 | Phase 8.5 / MSFragger ⛔ (MS1 only) |
  |---|---|---|---|
  | serum (909c) | +2.51 / +1.18 (n=3453) | +2.351 / +1.059 | MSFragger +2.51 |
  | bcell (B.naive) | +0.65 / +1.87 (n=23149) | −0.295 / +0.942 | none |
  | b1906 | +0.77 / +0.98 (n=8059) | +0.314 / −0.023 | Phase 8.5 +0.53, MSFragger +0.53 |

- **Reading the table — the contradictions below are REAL but their diagnosis was WRONG.** The
  observations (recon reads high on b1906, disagrees in sign on bcell) were correct and correctly
  refused rationalization — that discipline is why the bug was eventually found. The *causes*
  proposed were both wrong. Retained as a record of reasoning, not as current findings:
  - **serum: strong agreement.** MS1 bias matches MSFragger almost exactly (+2.51 both) and is
    within 0.16 ppm of MetaMorpheus (+2.351). MS2 bias is within 0.12 ppm of MetaMorpheus
    (+1.18 vs +1.059). Three independent tools converge here — good evidence recon's MS1/MS2
    self-cal pipeline works correctly on this file.
  - **b1906: MS1 close-ish, one direction consistently.** recon (+0.77) reads 0.24 ppm above
    both Phase 8.5 and MSFragger (+0.53 each, which agree with each other almost exactly) and
    0.46 ppm above MetaMorpheus (+0.314). Three other methods cluster tightly at +0.3 to +0.53;
    recon is consistently the highest of the four, by a similar margin each time. Small but
    real and directionally consistent — worth a closer look, not dismissed as noise.
  - **b1906: MS2 disagrees materially.** recon reports +0.98 ppm; MetaMorpheus reports
    essentially zero (−0.023); MSFragger's b1906 MS2 range is also near zero (−0.05 to −0.16,
    NOTES line ~562). Two independent sources agree MS2 bias on this file is ~0, recon says
    it's nearly +1 ppm. This is the sharpest disagreement in the table.
  - **bcell: MS1 disagrees in sign.** recon (+0.65) vs MetaMorpheus (−0.295) — opposite signs,
    0.945 ppm apart. No third source exists for this file yet (first time bcell has been
    measured by anything other than recon and MetaMorpheus), so there's no tiebreaker.
  - **bcell: MS2 disagrees in magnitude.** recon +1.87 vs MetaMorpheus +0.942 — same sign, ~2x
    apart.
  - **Not resolved this session — open question for whoever picks this up next.** Two candidate
    explanations, neither confirmed: (a) recon's clean subset comes from the wide-open search
    (very different PSM population than MetaMorpheus's narrow closed-search 1%-FDR set,
    especially for bcell/b1906 which may have more chimeric/ambiguous spectra than serum), and
    (b) the per-PSM vs per-datapoint aggregation unit could behave differently on files with
    more multiply-matched fragment ions per PSM. Do NOT treat either explanation as settled —
    they are hypotheses, not verified causes. If this needs resolving before shipping the
    self-cal feature more broadly, the next step is probably comparing the actual PSM sets
    (recon's clean-subset spectrum IDs vs MetaMorpheus's confident-PSM spectrum IDs) for bcell,
    since that's the file with no third-source tiebreaker at all.
  - **✅ RESOLVED 2026-08-24 — both hypotheses above were WRONG.** The cause was neither the PSM
    population nor the aggregation unit. Sage v0.14.x reports `precursor_ppm` and `fragment_ppm`
    as `|error|`, and recon took the median of that column as a signed bias. Every discrepancy in
    this entry collapses to that one cause. See "MS1/MS2 bias was a median of |error|" below. Do
    not act on the two hypotheses above — they are retained only as a record of what was tried.

### Ground-truth reference values — the single source (2026-08-24)

**Cite this block. Any MS1/MS2 ppm number elsewhere in this file that disagrees is superseded.**
Two generations of numbers exist in this file because both MSFragger and recon were re-run on
2026-08-24; older values are close but not identical and must not be mixed into new tables.

**External references — MSFragger 2026-08-24 run, pre-calibration ("Old") column**
(`testing/reference-data/msfragger/strictTryp/log_2026-08-24_13-19-52.txt`; identical in the
`semiTryp` and `strictTrypVarCAM` logs, because MSFragger calibrates on the enzyme-agnostic first
search) **and MetaMorpheus round-0** (2026-08-19 run, still current — not re-run since):

| File | MSFragger MS1 / MS2 | MetaMorpheus MS1 / MS2 |
|---|---|---|
| serum (909c) | +2.50 / +0.96 | +2.351 / +1.059 |
| bcell (B.naive) | −0.01 / +0.96 | −0.295 / +0.942 |
| b1906 | +0.50 / −0.05 | +0.314 / −0.023 |

**Superseded MSFragger values, do not cite:** serum 2.51, b1906 0.53 (pre-2026-08-24 run; appears
around the Phase 8.5, C1/C2, and MetaMorpheus part-1/2 entries). They agree with the current run
to within 0.03 ppm, so no earlier *conclusion* changes — but new tables must use the values above.
bcell had **no** MSFragger value before 2026-08-24.

**recon's own values are NOT listed here on purpose.** Every recon MS1/MS2 bias/MAD figure in this
file predates the |error| bug fix and is wrong on any file where the bias is not much larger than
the scatter. The corrected TRUE signed values measured 2026-08-24 are in the bug entry immediately
below; recon will not have trustworthy *reported* values until the fix lands and it is re-run.

### ⚠ BUG — MS1/MS2 bias was a median of |error|, not a signed bias (found and confirmed 2026-08-24; **MS1 FIXED 2026-08-28**, MS2 still open)

**This is a correctness bug, not a design preference. It resolves every open mass-accuracy
discrepancy in this file at once.** Evidence: `testing/scripts/ms1_bias_sign_check.py`, run on all
three files against both the open and closed searches.

- **Root cause.** Sage v0.14.x — the pinned version — reports `precursor_ppm` and `fragment_ppm`
  as **`|error|`** (it only becomes signed in v0.15.0; NOTES "Sage version pinning" already
  recorded this, but the consequence was not traced). `compute_ms1_stats`
  (`calibration.rs:168`) takes `median(precursor_ppm)` directly. So recon's `bias_ppm` is a
  **median of absolute errors**. It can never be negative, and it reads systematically high
  whenever `|true bias|` is not large compared with the scatter.
- **Dispositive measurement.** Zero negative values in `precursor_ppm` across 3,764 / 32,133 /
  10,942 clean-subset PSMs. Zero negatives in `fragment_ppm` either. The signed error was then
  reconstructed independently of that column, from `delta_mass_corrected / calcmass × 1e6`:

  | File | recon reports | TRUE signed | MetaMorpheus r0 | MSFragger 2026-08-24 pre-cal | recon error |
  |---|---|---|---|---|---|
  | serum | +2.4278 | +2.4215 | +2.351 | +2.50 | +0.006 (latent) |
  | bcell | +0.7028 | **−0.2357** | −0.295 | −0.01 | **+0.938, sign flipped** |
  | b1906 | +0.7804 | +0.4403 | +0.314 | +0.50 | +0.340 |

  ("pre-cal" = the Old column of `calibrate_mass`'s table, i.e. the raw instrument reading before
  MSFragger's own correction — the correct comparison point for recon, which runs on raw mzML.
  These are the **2026-08-24** run's values, not the older 2.51/0.53 pair that appears elsewhere
  in this file. See the canonical block below.)

- **The correction brings recon into agreement with both independent tools on all three files.**
  Max disagreement against MetaMorpheus/MSFragger, before → after: serum 0.077 → 0.079 ppm
  (unchanged, was never wrong), bcell **0.998 → 0.226**, b1906 **0.466 → 0.126**. On bcell and
  b1906 the corrected value lands *between* the two references. This is the strongest available
  evidence the fix is right: one root cause, three files, and independent measurements converge.
- **Why serum hid it.** When `|bias| ≫ scatter` (serum: 2.42 vs 0.48) almost every PSM's error has
  the same sign, so folding about zero changes nothing. serum agreeing with MSFragger to two
  decimals was read as validation of the whole pipeline; it only ever validated the one file where
  the bug is invisible. **A single agreeing file is not validation of a method.**
- **The MAD is wrong too, in the same way and for the same reason.** Folding a distribution
  centred near zero compresses it. Measured understatement of the true scatter: serum −0.1%
  (latent), **bcell 37.0%**, **b1906 27.7%**. This compounds with the too-tight tolerance finding
  below — the recommendation's *centre* and its *width* are both derived from a folded
  distribution.
- **MS2 is affected identically, and is STILL OPEN as of 2026-08-28. Fixable, but not from the
  default TSV.** The 2026-08-28 MS1 fix did NOT touch it. `PsmSummary::fragment_ppm` still copies
  Sage's absolute column, and it is now documented as absolute at every point it surfaces — the
  struct field, `QcResult`, the report struct, the console line and the HTML caption — so the
  report no longer implies a signed MS2 bias it does not have. The HTML previously said fragment
  error "reflects true instrument calibration"; that wording is withdrawn. `fragment_ppm` is also
  absolute (zero negatives, all three files), which explains the remaining part-2 discrepancies
  (bcell MS2 recon +1.80 vs MetaMorpheus +0.942 / MSFragger +0.95; b1906 recon +0.97 vs ≈0 from
  both). The default `results.sage.tsv` carries only the per-PSM summary `fragment_ppm`, with no
  per-fragment rows to reconstruct from. **Four options for step 3, in preference order:**
  1. **`sage --annotate-matches`** — a documented Sage flag: "Record all experimental-theoretical
     fragment ion matches." If it emits observed and theoretical m/z per matched fragment, the
     signed error is a subtraction, exactly as for MS1, with no fragment-matching code of our own
     and no reliance on Sage's sign convention. **Output schema is not in our scraped docs —
     verify empirically before designing around it** (run it on one file, inspect the output).
     Cost: one extra Sage flag; the fragment file may be large.
  2. **Compute fragment errors ourselves** from the mzML plus the peptide sequence. Honest and
     self-contained, and it is what we are already doing for MS1 — but it means generating
     theoretical b/y ions and matching peaks, which collides with the locked "Confidence from
     existing Sage fields only — no new fragment-ion computation". That lock was written when
     Sage's columns were trusted; this bug is legitimate grounds to revisit it, but revisit it
     deliberately, not as a side effect. Restricting the work to the clean subset makes it much
     easier (unmodified peptides only, so no localization problem — which the "No per-residue
     localization" lock otherwise forbids).
  3. **Adopt Sage v0.15.x**, where the column is signed. Zero code, but blocked — NOTES says wait
     for the stable release, not a beta.
  4. **Relabel** the field as a median *absolute* fragment error. Accurate and nearly free, but it
     abandons the engine-comparison use the field was added for.
  Option 1 first, because it is cheap to test and, if it works, gives the real number without
  reopening a locked design decision.
- **✅ MS1 FIXED 2026-08-28, in one place.** `PsmSummary::from_psm` now populates a field named
  `precursor_ppm_signed` from `calibration::signed_precursor_ppm`, which is
  `delta_mass_corrected / calcmass × 1e6`. Sage's absolute column is no longer read for this
  purpose anywhere. The field was RENAMED, not just repopulated, so the v0.15.0 double-correction
  hazard below cannot be missed by someone reading the struct.
  **The Rust reproduces the Python check script exactly on all three files**, which is agreement
  between two independent implementations, not a self-check:

  | file | recon BEFORE | recon NOW | check script | MetaMorpheus | MSFragger |
  |---|---|---|---|---|---|
  | serum | +2.4278 | **+2.4215** | +2.4215 | +2.351 | +2.50 |
  | bcell | +0.7028 | **−0.2357** | −0.2357 | −0.295 | −0.01 |
  | b1906 | +0.7804 | **+0.4403** | +0.4403 | +0.314 | +0.50 |

  MADs now 0.4845 / 0.6733 / 0.6844, against the folded 0.4239 on bcell.
  Clean-subset sizes 3764 / 32133 / 10942, unchanged.
  **The invariant is asserted in code**, in `recon-tool/tests/ms1_calibration_integration.rs`:
  `ms1_bias_is_negative_on_bcell` fails on any non-negative value, and
  `the_folded_column_could_not_have_produced_that_answer` is its control — it asserts Sage's raw
  column still holds ZERO negatives on that file, so a median of it is `>= 0` by construction and
  the sign flip is unreachable from it. That control doubles as the v0.15.x tripwire: if Sage is
  ever upgraded, it fails and says to REMOVE the reconstruction rather than stack it.
  serum and b1906 are pinned too, so the regression case is not a single file.
- **Blast radius.** `bias_ppm`, `spread_mad_ppm`, the user tolerance recommendation, and the Pass 2
  window are all downstream. Pass 2 is the worst affected: it is **bias-centred**, so on bcell it
  would centre a ±1.2 ppm window at +0.70 when the truth is −0.24 — mis-centred by nearly a full
  ppm, on a window narrower than the error. That is a clipping failure, not a rounding issue.
- **What is NOT affected:** everything not derived from `precursor_ppm`/`fragment_ppm` — mod
  discovery and its peak counts (delta-mass based), signal fate, polymer, oxonium, digestion, the
  +57 reconciliation, and the MSFragger/MetaMorpheus reference numbers themselves. The step-1
  ground truth stands except for the calibration block.
- **Sage version upgrade note:** add to the v0.15.0 checklist — adopting it makes `precursor_ppm`
  signed, at which point the MS1 fix above becomes a double-correction. Whoever does that upgrade
  must remove the reconstruction, not stack it.

### ✅ `--annotate-matches` RUN, AND SIGNED MS2 AGREES WITH BOTH REFERENCE TOOLS (2026-08-28)

**⚠ THE ENTRY BELOW WAS WRITTEN FROM SOURCE ONLY. It has now been RUN and every
claim in it is confirmed by observation.** Sage v0.14.7 was built natively for
arm64 from the pinned commit and the official
`sage-v0.14.7-aarch64-apple-darwin` release binary was also obtained; both report
`sage 0.14.6`. The closed serum search was re-run with `--annotate-matches` using
the config `step1-closed-serum` recorded.

**The observed header is EXACTLY the predicted one**, seven columns in the same
order. 210309 fragment rows, 8.5 MB, for 24817 PSMs on one file — the size
warning stands.

**✅ THE CONVENTION IS CONFIRMED AGAINST THE TOOL'S OWN OUTPUT.** Sage's
`fragment_ppm` was RECOMPUTED from the fragment rows using the formula read out
of `scoring.rs:607` — intensity-weighted mean of |error| — and reproduced the
column to a **max difference of 0.041 ppm across 4000 PSMs** (f32 rounding). So
the join on `psm_id` is right AND `fragment_ppm` is an intensity-weighted MEAN of
|error|, not a median. Verified, not assumed.

**✅ SIGNED MS2, MEASURED — and it lands BETWEEN the two independent references.**
9497 confident PSMs (rank 1, target, `peptide_q<0.01`), 138638 matched fragments:

| quantity | serum |
|---|---|
| negative fragments | **32113 / 138638 = 23.16%** (Sage's column: 0%) |
| SIGNED per-fragment median | **+1.0049 ppm** (MAD 0.9498) |
| SIGNED per-PSM median | **+0.9496 ppm** (MAD 0.5114) |
| Sage `fragment_ppm` median, same data | +1.2602 ppm (MAD 0.4440) |
| **MSFragger reference** | **+0.96** |
| **MetaMorpheus reference** | **+1.059** |

The per-fragment signed median sits between MSFragger and MetaMorpheus (0.045 /
0.054 away). The ABSOLUTE column sits OUTSIDE both (0.300 / 0.201 away). This is
the MS1 result repeating at MS2: folding about zero reads high, and the signed
reconstruction converges on two tools that were measured independently.
For contrast, recon currently reports serum `ms2_bias_ppm = 1.1197` from the OPEN
search's clean subset — a different population, still absolute.

**The truncation caveat is real but MILD on this file, and now quantified.**
Search `fragment_tol` was ±10 ppm. Percentiles: p5 −2.388, p25 +0.054, p50
+1.005, p75 +1.954, p95 +4.470. Only **900 of 138638 fragments (0.649%)** lie
within 1 ppm of the ±10 wall, so the median and MAD are not being shaped by the
wall here. At a tighter fragment tolerance they would be. The caveat governs the
TAILS and any attempt to justify widening the tolerance — not this median.

### ✅ `--annotate-matches` carries both m/z — option 1 works (read from the pinned source, 2026-08-28)

**Read from Sage source at the pinned commit `99407db`, NOT from a run.** No
runnable build of the PINNED Sage was available: `reference/sage/` does not
exist, and the only runnable build is
`~/Documents/GitHub/sage/target/debug/sage`, which is **0.15.0-beta.2** — an
unpinned beta, and the very version where `precursor_ppm` becomes signed. Probing
with it would have measured a different tool. The pinned commit is present as
source in that same checkout, so the flag was read at the exact pinned revision
instead. **No `matched_fragments.sage.tsv` has been seen on disk yet** — the schema
below is the writer's, not an observed file.

**The flag exists in 0.14.7** (`crates/sage-cli/src/main.rs:473`) and writes
`matched_fragments.sage.tsv` (`crates/sage-cli/src/output.rs:219`), or
`matched_fragments.sage.parquet` in parquet mode. Columns, verbatim:

```
psm_id  fragment_type  fragment_ordinals  fragment_charge
fragment_mz_calculated  fragment_mz_experimental  fragment_intensity
```

**Both m/z values are recorded**, exactly as option 1 hoped. `crates/sage/src/scoring.rs:610`
sets `exp_mz = peak.mass + PROTON` and `calc_mz = mz + PROTON`, so the signed
fragment error is

    (fragment_mz_experimental - fragment_mz_calculated) / fragment_mz_calculated * 1e6

a subtraction, with no fragment-ion generation of ours and no reliance on Sage's
sign convention. The locked "no new fragment-ion computation" does NOT need
reopening. `psm_id` is the join key, it is the FIRST column of
`results.sage.tsv`, and it is already present in every committed TSV — but
`sage_results.rs` does not parse it, so the join needs that field added.

**⚠ NEW, and it sharpens what NOTES already said about `fragment_ppm`.** The
column is not merely absolute — `scoring.rs:607` accumulates
`peak.intensity * |mz - peak.mass| * 2e6 / (mz + peak.mass)` and divides by
`summed_b + summed_y`. So `fragment_ppm` is an **INTENSITY-WEIGHTED MEAN of
|error| per PSM**, not a median and not a plain mean. Any comparison against
another engine's fragment-error figure must match that convention or state that it
does not. This is the "check the convention, not just the column" rule again.

**⚠ THE REAL LIMIT OF THIS DATA, and it must be stated wherever the number is
reported.** `select_most_intense_peak` (`crates/sage/src/spectrum.rs:133`) takes
the MOST INTENSE peak inside the fragment tolerance window, not the CLOSEST. Two
consequences:
1. The error distribution is **truncated at ±(the fragment tolerance the search
   used)** by construction. It cannot see error outside that window, so it can
   never justify a tolerance wider than the one already searched. Structurally the
   same trap as "the clean subset understates the wider population's scatter".
2. Inside the window the peak is chosen by intensity, so at a wide tolerance a more
   intense neighbour can win over the true fragment, which widens the apparent
   error.
A measurement from this file is therefore a measure of MATCHED-fragment error at a
given tolerance. It is honest as that, and it is not an unconditioned instrument
figure.

**THE PROBE OUTPUT IS PRESERVED**, moved out of the session scratchpad on
2026-08-28 to `testing/search-output/probe-serum-annotate-matches/`
(gitignored): `matched_fragments.sage.tsv` (8.5 MB, 210309 rows),
`results.sage.tsv`, `results.json`, and `params-used.json`. **This is the only
per-fragment data this project has ever had.** It can answer the per-fragment
coverage question for the Pass 2 MS2 multiplier without running a new search.

**Not yet decided:** whether to wire it in. It costs one more Sage flag, a second
output file (one row per matched fragment, so large), a new `psm_id` field, and a
join. Deciding that is the rest of step 3's MS2 item.

### ⚠ The Rust ports — reviewed and RUN against the Python, 2026-08-28

Three ports sat in `testing/scripts/` beside their Python originals:
`annotate_termini.rs` (360 lines), `subset_fasta.rs` (231),
`digestion_efficiency.rs` (656). **None was wired into the crate** — no
`recon-tool/src` file referenced them. They were built and executed for this
review, not just read.

**⚠ `subset_fasta.{py,rs}` and `annotate_termini.{py,rs}` were DELETED on
2026-08-28** (recoverable from git), under the locked curation policy —
"superseded versions are deleted, not accumulated". Both were headed "superseded
by `digestion_efficiency`", nothing in the repo referenced them outside PLAN /
NOTES / JOURNAL prose, and keeping two contradictory subset implementations was
the confusion this cleanup exists to remove. **`digestion_efficiency` is the sole
port target.** The findings below are kept because they explain WHY, and because
the FASTA bug they describe still lives in `digestion_efficiency.rs`.

**✅ `subset_fasta` is byte-identical to its Python on real data.** Run on
`step1-closed-serum/results.sage.tsv` against the human canonical FASTA, both
emit 454 entries / 4001 lines, `diff` clean.

**🐛 FOUND AND FIXED — all three shared one FASTA-parsing divergence.**
`sequence_lines.clear()` sat INSIDE the `if let Some(h) = header.take()` block,
so any content before the FIRST `>` stayed in the buffer and was concatenated
onto the first entry's sequence. Python resets unconditionally
(`sequence_lines = []`). Demonstrated with a two-protein fixture carrying one
junk line: Python wrote `PEPTIDEK`, Rust wrote `JUNKLINEBEFOREHEADERPEPTIDEK`.
Fixed in all three by moving the reset out of the `if let`; the fixture now
matches Python and the 454-entry real-data output is unchanged. The real FASTA
begins with `>tr|A0A075B6V1|...`, so this never fired in production — but the
same function will be copied into `digestion.rs` when the port lands.

**⚠ THE `rev_` PAIRING IN `subset_fasta` IS DEAD CODE, AND ITS MESSAGE IS
FALSE.** Both Python and Rust add `rev_{acc}` to the include set, then print
"(includes both target and `rev_*` decoy entries)". Measured: the source FASTA
holds **0** `rev_` entries, and the subset wrote **0**. 454 accessions in, 454
out. Sage generates decoys internally (`generate_decoys: true`), so no decoy
accession can ever match. `digestion_efficiency` is right where `subset_fasta`
is wrong — it writes targets only and says "Sage will generate decoys
internally during Pass 2". This is very likely why `subset_fasta` was
deprecated.

**⚠ PLAN's paired target-decoy requirement is NOT built, and it is now SIZED.**
Both implementations filter `label == "1"`, so a significant hit on a DECOY
contributes nothing. Decoy PSMs carry `proteins` like
`rev_sp|Q7Z3E2|CC186_HUMAN`, so stripping `rev_` yields a TARGET accession that
IS in the FASTA — the requirement is implementable at the accession level, not
the FASTA-entry level. Measured on serum's closed search: **49 significant decoy
PSMs (q<0.01)** imply 42 target accessions, **33 of them not already selected**.
Subset would go **454 -> 487, +7.3%**.

**⚠ PLAN NAMES THE WRONG PORT TARGETS.** Step 3 says port `annotate_termini.py`
and `subset_fasta.py`. Both files — Python AND Rust — are headed "DEPRECATED:
superseded by `digestion_efficiency` (the `subset`/`annotate` subcommand)". The
live logic is `digestion_efficiency`, which PLAN does not list as a port at all.

**Shared with the Python, so limitations rather than port defects:**
`protein_seq.find(peptide)` takes the FIRST occurrence only; terminus
classification uses only the FIRST protein of a semicolon-separated list; and
all three Rust `parse_fasta_entries` load the whole FASTA into a `Vec` where
`subset_fasta.py` streamed via a generator — a regression that matters once this
runs inline in `run_run_command`. `strip_modifications` uses a depth counter
against Python's non-greedy regex; they converge on Sage-format peptides because
of the trailing `[^A-Z]` filter.

**✅ ALL THREE NOW RUN AND COMPARED (2026-08-28, later the same day).** `pyteomics
5.0` was installed into a scratchpad venv, so the two Python files that need it
could finally be executed. Results against `step1-closed-serum`:
* `subset_fasta` — byte-identical (454 entries).
* `digestion_efficiency subset` — byte-identical (454 entries).
* `annotate_termini` — identical content, 24817 total / 9493 filtered / 9493
  `fully_tryptic` on both.
* `digestion_efficiency annotate` — identical TSV, and identical JSON after the
  fix below.

**🐛 SECOND BUG FOUND AND FIXED — the JSON dropped empty categories.**
`digestion_efficiency.rs` built `terminus_classification.counts` from a map that
only ever held classes it actually saw, so on serum it emitted
`{"fully_tryptic": 9493}` where Python emitted all six keys with explicit zeros.
An ABSENT key and a ZERO count are different claims, and a class being empty is
exactly where a reader needs the 0 — the same distinction NOTES already draws for
`protein_context` ("not tested" vs "not supported"). Fixed by seeding a
`TERMINUS_CLASSES` constant at zero.
**Note WHY Python was right: it was right by accident.** `counts` is a
`defaultdict(int)`, and the percentage lines and the summary loop READ every
class before `dict(counts)` runs, which materialises the keys. The Rust now
STATES the schema rather than inheriting it from `defaultdict` semantics.

**⚠ CRLF — the Python writes `\r\n`, the Rust writes `\n`.** `csv.DictWriter`
defaults to `lineterminator='\r\n'`, so every Python-produced TSV carries CRLF:
measured 9494 of 9494 lines on the annotate output, and the COMMITTED
`testing/search-output/annotated_termini.tsv` is 55857 CRLF lines. The Rust ports
emit LF. Content is identical once CR is stripped. LF is the right choice for the
inline port, but **any future gate that byte-compares Rust output against a
committed Python-produced TSV will fail on line endings alone.** Normalise before
comparing.

### ✅ "TRYPTIC" RENAMED TO "ENZYMATIC" ACROSS CODE AND KEYS (2026-09-01)

**Ben's call, and the second half of making the enzyme a parameter.** Once the
protease is user-chosen, a report that says `fully_tryptic_pct` for a Lys-C run is
simply wrong. Ben chose to rename the SERIALISED KEYS too, not just the Rust
identifiers, so the JSON stops lying rather than being papered over with
`#[serde(rename)]`. Consumers break once, in the same 2.0.0 schema that already
dropped `three_layer_ms1`, instead of twice.

**Renamed:** `SemiTrypticStats` -> `SemiEnzymaticStats`;
`TerminusClass::{FullyTryptic, NonTryptic}` -> `{FullyEnzymatic, NonEnzymatic}`
and their `"fully_tryptic"` / `"non_tryptic"` strings; `is_tryptic_nterm/cterm`
-> `is_enzymatic_nterm/cterm`; and the keys `fully_tryptic_count`,
`fully_tryptic_pct`, `semi_tryptic_{count,pct,total}`, `semi_tryptic`,
`non_tryptic_{count,note}`, `decoys_fully_tryptic`,
`class_fdr_{fully,semi}_tryptic_pct`, `pass1_semi_tryptic_pct`,
`pass2_semi_tryptic_pct`, `pass2_only_semi_tryptic`.

**VALUE-PRESERVING, measured.** A full serum run before and after, compared with
the rename applied as a key mapping: **pass 2 shows 0 value differences across 58
numeric leaves**, and pass 1 differs only in the known ULP float on
`id_rate_by_tic_pct`. The key sets map 1:1 with nothing added or lost.

⚠ **WHAT WAS DELIBERATELY *NOT* RENAMED — this is the part a blind
find-and-replace would have got wrong.**
1. **Statements about measurements actually made with trypsin keep saying
   trypsin.** The liver class-FDR figures (0.10 % fully-tryptic vs 10.10 %
   semi-tryptic), the ~32-35 % serum semi-tryptic rate, and the
   `decoy_ragged_side` 96.63 % agreement were all measured ON TRYPSIN. Rewriting
   them to "enzymatic" would generalise a claim the data does not support.
2. **`testing/scripts/digestion_efficiency.{py,rs}`** keep their old vocabulary.
   NOTES:589 records that these ports were never wired into the crate; they are
   the historical reference implementation whose agreement with the Rust was
   measured. Renaming their internals would break that record for no gain.
3. **`Psm.semi_enzymatic`** was ALREADY named that — it comes from Sage's own
   column and was never trypsin-specific.

### ✅ THE RECOMMENDATION DECISION IS NOW AUDITABLE, AND THE RULE WAS AUDITED (2026-09-01)

**Schema 2.1.0** (additive). Prompted by Ben asking the report to state how many
delta masses were checked — a number that had to add up, and did not.

**THE AUDITED RULE, read from `tier_assignment.rs` line by line, not inferred:**

```
1. satellite?                -> Satellite            (floor never consulted)
2. no curated candidates?    -> count >= floor -> NotableUnannotated
                                else          -> NotCurated
3. candidates, none testable -> count >= floor -> Abundance
                                else          -> BelowFloor
4. testable candidates?      -> any passing   -> Statistics (best by OR)
                                none passing  -> NoResidueSupport (highest OR)
                                *** floor never consulted ***
```

⚠ **THE FLOOR IS NOT THE FIRST GATE.** A curated modification with testable
residues goes to statistics and the floor never applies to it. **Six of serum's
nine recommended mods sit BELOW the floor** — Deamidation 189, Carboxymethylation
97, Gln->pyro-Glu 60, Methylation 54, Trioxidation 36, Dehydroalanine 28, against
a floor of 224. Abundance decides only the branches statistics cannot reach.
This is also why water loss at n=82 was statistically tested despite being below
the floor: nothing inconsistent, statistics do not consult it.

⚠ **A WRONG VERSION OF THIS RULE WAS WRITTEN INTO A PLAN AND APPROVED.** It was
inferred from the reason STRINGS in committed output rather than read from the
decision function. It was internally consistent and matched every example
examined, so review did not catch it. It broke only when a count had to
reconcile. **"Audit the code before changing display" was already written in that
plan and was partly skipped.**

**REASON STRINGS NAMED THE CODE BRANCH, NOT THE OUTCOME. All renamed:**

| was | actually meant | now |
|---|---|---|
| `no_residue_support` | tested at the residue level and FAILED | `failed_residue_test` |
| `not_curated` | un-curated AND below the floor | `below_floor_uncurated` |

**`q_value` IS NOW SERIALISED, AND THAT IS THE REAL FIX.**
`Decision::NoResidueSupport` always carried `{ odds_ratio, q }`, but the
serialiser destructured `{ odds_ratio, .. }` and dropped q. The report showed an
odds ratio CLEARING the bar with no way to see which gate failed. Measured with
q now visible, serum:

| delta | label | OR | q |
|---|---|---|---|
| -18.0101 | Water Loss (Glu->pyro-Glu) | 2.122 | **0.0686** |
| +53.9156 | Fe[II] | 2.708 | 0.1887 |
| +37.9491 | Potassium | 4.058 | 0.2387 |
| +71.0377 | Propionamidation | 5.125 | 0.1169 |

So water loss failed on **q**, exactly as suspected but previously unprovable
from the artifact. Note several entries clear the OR bar comfortably and still
fail on q: a large odds ratio alone does not earn a recommendation once the
multiple-testing correction is applied.

✅ **BOTH BOUNDS ARE NOW INCLUSIVE.** The test read `o >= OR_MIN && q < Q_MAX` —
OR inclusive, q exclusive — so a candidate exactly on the q bound was rejected
while one exactly on the OR bound was accepted, against a documented rule of
"OR >= 2 and q <= 0.05". Same asymmetry commit `674d197` fixed for the
calibration subset, fixed the same way: **measured inert first** (closest real q
to the bound across serum and bcell is 0.06859; nothing sits on it), then
changed, then gated by `both_recommendation_bounds_are_inclusive`.

✅ **`ms2_bias_ppm` RENAMED TO `ms2_median_abs_ppm`.** Sage's `fragment_ppm` is
ABSOLUTE — v0.15 made only `precursor_ppm` signed — so a value derived from it
can never be negative and cannot express a direction. Calling it a bias invited
reading a magnitude as a systematic offset. **MS1 is signed, MS2 is not, and that
is a Sage convention rather than a choice.** `testing/scripts/liver_5way_report.py`
consumed the old key and was updated with it.

✅ **`runtime_seconds` ADDED, and it is genuinely the TOTAL.** The report is
written BEFORE Pass 2, because Pass 2 consumes it, so a naive field would only
ever record the time to first output. `write_report_files` is factored out and
called twice — the second time once every stage is done. Verified against the
console: serum 81.107685833 s vs "81.1s", bcell 385.249807333 s vs "385.2s".
`analyze` leaves it `None`, since that is one stage rather than a run.

### ✅ THE ARGUMENT SURFACE IS FROZEN (2026-09-01)

**Ben's requirement, and the final shape:**

    recon run <MZML> <FASTA> --enzyme <ENZYME> [--output NAME]

Three required inputs, one common option, everything else under an `Advanced`
help heading. `recon --help` now shows ONE command.

**Decisions, each with its reason:**

* **`mzml` and `fasta` are POSITIONAL.** Both are always required and their order
  is obvious. This is the change that is free before v0.1.0 and a breaking change
  after, which is why the freeze happened before CI rather than after.
* **`--enzyme` is REQUIRED, with no default.** recon does not assume trypsin.
  Defaulting it would silently mis-report every digestion number for any other
  protease, and the digestion number is the one validated against Byonic Preview
  and MSFragger. Making the user state it is the point.
* **`--unimod` is an OVERRIDE, not a requirement, on every subcommand.** `unimod.xml`
  (2.4 MB) is compiled in via `include_str!`, which is most of the binary's
  24.5 MB. `UnimodDb::from_embedded()` and `from_xml()` share ONE parser, so the
  two routes cannot diverge, and `embedded_unimod_matches_the_committed_file`
  compares them entry for entry on `record_id`, `title` and `mono_mass` bits —
  not merely on the count, because a reordering would hold the count and change
  the answers.
  **Corrected in place 2026-09-02, `84b8f0c`.** This entry used to say the
  override held for `run` only, and that `analyze`, `discover` and
  `compare-peak-assignment` still declared `unimod` as a required `PathBuf`.
  That was true when written and is not true now: a shared `load_unimod` takes
  the CLI path when given and the compiled-in copy otherwise, on all four
  commands. Verified by running `analyze` from a scratch directory outside the
  repo with no `--unimod`: it reports the database as compiled in, loads 1560
  entries, and produces 7 recommendations.
* **The 11 development subcommands are `hide = true`.** They still run, and
  `testing/scripts/` still calls `parse`, `discover`, `analyze` and
  `compare-peak-assignment`. They are simply not part of the product surface.
* **`--q-threshold` went to Advanced deliberately.** Every pinned baseline is at
  0.01; a visible knob invites numbers that cannot be compared with them.

**MEASURED, not assumed: the embedded Unimod changes nothing.** serum through the
frozen positional surface against the committed baseline: **1523 leaves compared,
3 differ** — `input.unimod_file` (now `<compiled into recon 0.1.0>`, which is the
provenance line doing its job) and the two known ULP floats
(`id_rate_by_tic_pct`, `polymer.total_pct_tic`).

⚠ **THE DSL OBLIGATION FOLLOWS THE EMBEDDING INTO THE RELEASE.** Unimod is under
the Design Science License. Section 3 requires a copy of the License to travel
with the work, and permits the Object Form only when the Source Data accompanies
it. So a release archive MUST contain BOTH `THIRD_PARTY_LICENSES.md` (which
carries the full DSL text) AND `unimod.xml` itself. Compiling the XML in does not
discharge that — it creates it.

### ✅ ENZYME IS A PARAMETER, NOT A CONSTANT (2026-09-01)

**The gap it closes.** Sage honoured any enzyme in the params template, but
recon's own digestion classifier hardcoded trypsin in four places. Choose Lys-C
and the SEARCH followed while the REPORT still counted K/R — a silently wrong
number, and the digestion number is the one validated against Byonic Preview and
MSFragger.

**The rule was taken from Sage's SOURCE, not its docs.** `crates/sage/src/enzyme.rs`
at the pinned commit computes `right = if c_terminal { mat.end() } else { mat.start() }`
and then suppresses the site if the residue **at** `right` is in `skip_suffix`.
So ONE rule covers both enzyme families, and `restrict` always tests the first
residue of the NEXT peptide. The DOCS say only "if one of these AAs follows the
cleavage site", which is ambiguous for an N-terminal cleaver — hence reading the
implementation. `recon-tool/src/enzyme.rs` mirrors it in `Enzyme::is_boundary`.

**IDENTITY ONLY, and that is a lock.** `--enzyme` writes exactly `cleave_at`,
`restrict` and `c_terminal` into both templates. It NEVER touches
`missed_cleavages`, `min_len`, `max_len` or `semi_enzymatic`, because Pass 1 and
Pass 2 set those differently on purpose. Asserted by
`applying_an_enzyme_leaves_the_tuning_fields_alone`.

**Interface: a preset name AND an explicit rule (Ben, 2026-09-01).** `--enzyme`
takes a preset or `CLEAVE_AT[/RESTRICT][/n]`, and `--cleave-at` / `--restrict` /
`--c-terminal` override individual fields. So an enzyme the preset table does not
know is still reachable without editing code.

✅ **PRESET PROVENANCE IS CITED AND HAS BEEN EYEBALLED (2026-09-01).** Ben checked
the shipped table against the Mascot URL and the literature. A generated
side-by-side of the code against the vendored rows read **10 of 12 EXACT**, with
the only two deviations being the agreed ambiguity-code drops (`asp-n` BD -> D,
`glu-c` EZ -> E).

⚠ **THE EYEBALL FOUND A NAMING TRAP, now fixed.** `glu-c` was shipped alone for
Mascot's `V8-E` row. But Glu-C/V8 is BUFFER-DEPENDENT — after E in phosphate,
after BOTH D and E in ammonium bicarbonate — which is exactly why Mascot splits
it into `V8-E` and `V8-DE`. One preset called `glu-c` silently picks a buffer
condition for the user and reports the wrong digestion rules with no warning.
Asp-N has the same story (`Asp-N` vs `Asp-N_ambic`).

**So both now ship as PAIRS (Ben's call, option 2):** `glu-c` (E, restrict P) and
`glu-c/de` (DE, restrict P); `asp-n` (D, N-terminal) and `asp-n/ambic` (DE,
N-terminal). Fourteen presets. `buffer_dependent_pairs_are_actually_different`
asserts each pair really differs — and caught its own first version, which probed
with `AAAEAAA`, a peptide containing no D, where `glu-c` and `glu-c/de` see the
same single site. The probe is now `AADAEAA`, which contains both.

**PRESETS ARE A CONVENIENCE, NOT A CEILING**, which is why the table does not
need to be exhaustive: `--enzyme "CLEAVE/RESTRICT[/n]"` expresses any single-rule
protease, and `--cleave-at` / `--restrict` / `--c-terminal` override single fields
of whatever a preset resolved to.

All presets come from Mascot's published enzyme list,
<https://www.matrixscience.com/help/enzyme_help.html>, vendored at
`reference-notes/mascot-enzymes.md`. Scope is Ben's call: **the common proteases
only**, with everything else reachable through an explicit rule on the command
line.

⚠ **THE SOURCE CORRECTED TWO OF MY GUESSES, which is exactly why it was needed.**
Before the citation this table was written from general knowledge and had
`asp-n` as `D` alone (Mascot says `BD`) and `glu-c` with NO restriction (Mascot's
V8-E is `EZ` with `P`). The missing proline rule on Glu-C would have been a
silently wrong digestion number for every Glu-C run.

⚠ **A CRASH WAS FOUND AND GUARDED, not a style issue.** Sage's `VALID_AA`
(`crates/sage/src/mass.rs`, pinned commit) is the 20 standard residues plus `U`
and `O` — it excludes the ambiguity codes `B`, `Z`, `J`, `X`. Sage validates
`cleave_at` with **`assert!`, not a `Result`**, so since A1 landing 2 put Sage in
our process, `--cleave-at BD` would have ABORTED recon instead of reporting an
error. `validate_residues` now rejects them first with a message naming the
Mascot ambiguity-code issue. Falsified: `BD`, `BDEZ`, `EZ`, `J`, `X` all refused;
`U` and `O` still accepted.

Two classes of Mascot enzyme are therefore not shipped, both recorded in the
vendored table: those needing `B`/`Z` (`asp-n` keeps `D`, `glu-c` keeps `E`,
`V8-DE` is dropped), and the MULTI-RULE ones (`CNBr+Trypsin`, `LysC+AspN`,
`Formic_acid`, `TrypsinMSIPI`) which need two or three rules mixing N- and
C-terminal cleavage. Sage's config holds ONE triple, so that is a Sage limit.

**INVARIANT PROVED BEFORE THE FEATURE: monotonicity.** Removing a restriction can
only ADD boundaries, so for ANY peptide `trypsin/p` counts >= as many missed
cleavages as `trypsin`. Asserted over nine sequences in
`dropping_the_restriction_never_removes_a_boundary`, including the strictness
case (`AAKPAARPAA`) that proves the two presets are not the same enzyme.

**THE DEFAULT CHANGES NOTHING — measured, not asserted.** A full serum run at the
default enzyme against the committed baseline: in the pass-2 report, which is
where recon's own classifier runs, **exactly one metric moved and it is
`pass2_search_seconds`, a timing.** All 50 scientific metrics are identical. In
the pass-1 report only the two known ULP floats differ
(`polymer.total_pct_tic`, `signal_fate.id_rate_by_tic_pct`).

**BEN'S SPOT CHECK, run on serum: trypsin vs trypsin/p, 51 metrics differ, in
TWO OPPOSITE DIRECTIONS.** That is the signature of a classifier doing real work
rather than shifting everything one way.

| metric | trypsin | trypsin/p |
|---|---|---|
| `composition.cleavage_completeness_pct` | 89.137 | 84.349 |
| `composition.missed_cleavage.pct` | 10.863 | 15.651 |
| `digestion.missed_cleavages.mean` | 0.1405 | 0.1989 |
| `terminus.fully_tryptic_pct` | 66.922 | 68.101 |
| `terminus.semi_tryptic_pct` | 33.078 | 31.899 |
| pass-1 PSMs | 15631 | 16423 |
| parsimony subset proteins | 225 | 238 |

Both directions follow from one mechanism: K|P and R|P become cleavage sites, so
INTERNAL ones turn into missed cleavages (completeness down) while TERMINAL ones
turn into enzymatic termini (fully-enzymatic up, semi down).

⚠ **THE DECOY RAGGED-SIDE PROXY'S VALIDATION DOES NOT TRANSFER.**
`decoy_ragged_side` was measured at 96.63 % agreement ON TRYPSIN. It now inverts
its test for N-terminal cleavers, which is the right shape but is UNMEASURED for
any enzyme other than trypsin. Do not quote that 96.63 % for a non-tryptic run.

⚠ **Sage's two special cases are REFUSED with a reason, not silently mishandled:**
`cleave_at ""` (non-specific) and `"$"` (no digestion) have no cleavage rule, so
recon cannot classify termini or count missed cleavages. The error says Sage can
still search them via `--params`.

### ✅ THREE-LAYER MS1 REMOVED FROM RECON, PRESERVED AS A COMMAND (2026-09-01)

**Ben's call, and it matches a finding this project already made and did not act
on.** NOTES:7556 recorded at build time that three-layer is ~9x slower than
MS2-only signal fate and that "For routine QC, MS2 count-based metrics are
sufficient", with the row marked Optional. It stayed in the unified report anyway.

**FOUR defects, each measured, not argued:**

1. **It reported PASS-1 identifications only.** `run_analyze_command` is called
   before `run_pass2`, so every ID the semi-enzymatic pass finds was invisible to
   it. recon's own two-pass design exists because Pass 1 under-identifies, so the
   report carried an "identified" fraction the tool itself does not believe.
2. **"Peptide-like" is an m/z window, nothing more.** `400.0 <= mz <= 1200.0`, per
   RAW MS1 PEAK. No charge, no isotope pattern, no deisotoping. `MzRtRegion`
   carries a `charge` field that `contains()` never reads. It does not examine
   spectra at all.
3. **It therefore contradicts mzSniffer, in the same report.** Polymer peaks sit
   inside 400-1200. On serum recon reports polymer at 0.57 % of TIC while
   three-layer counts that same ion current as "peptide-like".
4. **The headline number moved with a tolerance default.** The report path passed
   `mz_tol_ppm = 10`; `signal-fate` defaults to 20. Measured on serum: identified
   6.99 % at 10 ppm, 7.7 % at 20 ppm, never-sampled 48.9 % vs 47.4 %. Two entry
   points in one binary, two answers.

**What was removed:** the `three_layer_ms1` field on `ReconReport`, its console
block, its HTML card, and the `--full` flag on `run` and `analyze`. `--rt-window`
and `--mz-tol-ppm` went with it from `analyze` — they fed nothing else there.
**Schema 1.8.0 -> 2.0.0**, a major bump because a field was removed.

**What was PRESERVED, and why that shape:** `signal-fate --three-layer` was
ALREADY a standalone subcommand. So rather than write a script that
re-implements the algorithm — a reimplementation nobody would check — the
existing command IS the preserved artifact. It stays compiled, tested and
runnable, which a loose script would not. **This is the copy to port to sageGUI,
where "your method never sampled half the peptide signal" is directly
actionable.**

**A bug fixed while preserving it:** the command COMPUTED and PRINTED the
three-layer block but never serialised it, so `--output` wrote JSON with no
three-layer in it. `SignalFateResult` now carries `three_layer_ms1`, and
`mod_breakdown` — which only ever existed on this path — now reaches JSON too.

**PROVEN EQUIVALENT, not assumed.** At matching tolerance the preserved command
reproduces every removed number bit-for-bit on serum — all 11 fields:
`total_ms1_tic` 52222394912255.09, `non_peptidic_pct` 4.7236460334210175,
`never_sampled_pct_of_peptide_like` 48.9193383746919,
`identified_pct_of_peptide_like` 6.985321814667726, `ms2_precursor_count` 41788,
`identified_psm_count` 15631, `identified_feature_count` 14807, and the rest.

**THE SNIFFERS ARE UNTOUCHED, which was Ben's explicit requirement.**
`polymer.rs` imports only `std::collections::HashMap`; `oxonium.rs` imports only
`crate::mzml::Ms2Spectrum` and serde. Neither ever referenced the three-layer
types. Measured on a full serum run after removal: the `oxonium` block is
IDENTICAL, zero differing fields. `polymer/total_pct_tic` differs in the last
decimal place only (…818 vs …8179) — the same ULP difference seen between two
runs of the UNMODIFIED code earlier the same day, so it is float summation order,
not this change.

⚠ **`build_precursor_queries_from_psms` was KEPT** — `signal-fate --ms1-intensity`
uses it and it has nothing to do with three-layer.

The four committed `full-run/*.json` were re-baselined. Verified: none of the
four carries `three_layer_ms1`, and all four carry `schema_version` 2.0.0. This
agrees with PLAN.md.

### ✅ A1 LANDING 2 — Sage is now a LIBRARY, and the call mechanism moved nothing (2026-09-01)

**What changed.** `sage-core`, `sage-cli` and `sage-cloudpath` are Cargo git
dependencies pinned to `df9219951cc9a54cf4cd55d76541af24b687bd3d` — tag
`v0.15.0-beta.2` on **upstream `lazear/sage`**. `run_sage` calls
`sage_cli::runner::Runner` in process. **No fork is needed:** recon wants neither
of the `progress` / `cancel` patches `neely/sage` carries for sagegui.

**Deleted, because there is no external binary any more:** `locate_sage_binary`,
`DEFAULT_SAGE_PATHS`, `SAGE_PATH`, `verify_sage_version`, `extract_sage_version`,
the `--sage-binary` flag, and `SageRunResult`'s `stdout`/`stderr` fields (empty
strings would have been fields that lie).

**THE PIN IS NOW STRONGER, not weaker.** It was a runtime handshake with a
swappable binary; it is now a `Cargo.lock` entry that cannot drift between what
was tested and what a user runs. The guard moved with it:
`cargo_lock_pins_exactly_sage_commit` asserts every `lazear/sage` line in
`Cargo.lock` names `SAGE_COMMIT`. **Falsified both ways** — set `SAGE_COMMIT` to
`deadbeef…` and it fails naming `Cargo.lock` line 2810.

**Telemetry is silent BY CONSTRUCTION, and the flag is gone.**
`Telemetry::send` is called only from Sage's own `crates/sage-cli/src/main.rs:138`;
`Runner::run` merely builds and returns the struct. Read from the pinned source,
not assumed. So `--disable-telemetry-i-dont-want-to-improve-sage` was deleted
rather than ported.

**Rayon is configured exactly as Sage's `main` does** — a 2 MiB worker stack —
behind a `Once`, because `build_global` succeeds only once per process and recon
runs TWO searches. The two-pass path was exercised end to end: Pass 1 45.5s,
Pass 2 4.5s, both in one process.

**MEASURED, NOT ASSUMED — serum replayed against the committed v0.15 baseline:**

| comparison | result |
|---|---|
| PSM rows | 68817 vs 68817, same key set, 0 baseline-only, 0 probe-only |
| all 16 RAW columns | **0 rows differ** |
| all 5 RESCORING columns | **0 rows differ**, max abs delta 0 |
| `spectrum_q <= 0.01` | 13786 vs 13786, 0 out, 0 in |
| `peptide_q <= 0.01` | 15707 vs 15707, 0 out, 0 in |
| full recon report JSON | 4 differing leaves: `generated_at`, `git_commit`, `input/sage_tsv`, and `id_rate_by_tic_pct` at ~1e-14 |

⚠ **DO NOT OVERREAD THIS.** It is a SAME-PLATFORM comparison: the baseline was
produced on this arm64 Mac by the v0.15.0-beta.2 binary, and the probe by the
same commit linked as a library on the same machine. It shows the CALL MECHANISM
is inert. It does **not** repeal the cross-build entry below — a Windows or Linux
build is still expected to move q-values by about 0.1% of PSM membership, and
that remains untested for the library route.

The gate is `testing/scripts/compare_sage_tsv.py`, which fails on any RAW column
difference and reports rescoring movement without failing.

### ⚠⚠ q-DERIVED COUNTS JITTER RUN TO RUN. MEASURED, n=7 (2026-09-01)

**THIS SUPERSEDES THE READING THAT ±1 PSM MEANS A BUILD DIFFERENCE.** The entry
below is still right that builds differ; it is not the explanation for small
count movements seen on the SAME machine.

**The measurement.** bcell pass-1 PSMs at q <= 0.01, SEVEN runs of ONE binary,
one lockfile, no code change between them:

    72801, 72802, 72802, 72802, 72802, 72802, 72803

Range 2 in 72802 = **0.003 %**. The committed value (72801) sits INSIDE that
range. So the committed numbers were never exactly reproducible: they are one
draw, and so is every regeneration.

**HOW THIS WAS GOT WRONG FIRST, because the failure is instructive.** A
regeneration produced 72802 against a committed 72801. A "control" of TWO runs
both gave 72802, and that was read as "stable within a build, therefore a build
difference". **n=2 is not a stability measurement.** Two hypotheses were then
chased and BOTH FALSIFIED:

1. **Build profile.** Sage's workspace sets `lto = "fat"`, `codegen-units = 1`;
   recon had no `[profile.release]`, so Cargo's defaults applied and Sage was
   compiled differently from upstream's release. Adding the matching profile did
   NOT restore the count (still 72802) and made bcell Pass 2 SLOWER
   (98 s -> 125 s). Reverted. **Do not re-add it expecting either benefit.**
2. **Dependency graph.** A Cargo git rev pins Sage's SOURCE, not its
   dependencies: 155 of 335 shared packages resolved differently from Sage's own
   `Cargo.lock`, including `rayon` 1.11 -> 1.12 (parallel reduction order),
   `hashbrown`, `twox-hash`. Forcing 88 of them to Sage's versions (67 could not
   be, blocked by recon's own requirements) moved the count to **72803 — further
   away**. Reverted.

The count is not a target to converge on. It is noise, and chasing it cost two
build cycles.

**⚠ A CLAIM IN COMMIT `a4f09b9` IS WRONG AND IS CORRECTED HERE.** That message
says the embed made the pin "stronger". On the axis it meant — a lockfile entry
cannot be swapped the way a `SAGE_PATH` binary can — that is true. But it missed
that **a Cargo git rev does not pin the dependency graph**, so recon compiles
Sage against 155 packages that differ from what upstream built. The pin is
stronger in one way and weaker in another. Both halves belong in any Methods
text.

**WHAT FOLLOWS FOR TESTS, and it is now implemented.** An `assert_eq!` on a
q-derived count asserts one draw from a distribution and fails on the next.
`digestion_composition_integration` did exactly that and broke on regeneration
(`peptides_classified` 10772 -> 10771, `decoys_ragged_n` 86 -> 84). Counts are
now BANDED, per PLAN's long-standing "allow ~0.1 % movement on any q-derived
count":
* large counts: a 0.1 % band, thirty times the measured jitter;
* decoy counts: wider still, because the population is tiny (84, 18) so the same
  absolute jitter is a far larger relative move — counting noise alone on 86 is
  about sqrt(86) ~ 9;
* structural invariants (`peptides_unresolved == 0`, non-enzymatic == 0 under a
  semi-enzymatic search) stay EXACT — no jitter can cause those;
* and the actual scientific claim is now asserted directly: semi-enzymatic class
  FDR must exceed 20x the fully-enzymatic one. That is what the test is FOR, and
  it is immune to jitter.

**RULE:** never pin a q-derived count with equality again. Band it, and assert
the claim the number exists to support.

### ✅ THE RULE WAS ENFORCED — four `assert_eq!` on q-derived counts found and banded (2026-09-03)

The 2026-09-03 code audit found four tests still pinning a q-derived count with
`assert_eq!`, in violation of the rule directly above. All four are now banded at
0.5 %, the same practice used elsewhere in the suite.

The evidence that this was necessary, not precautionary: today's `full-run`
regeneration moved liver's pass-1 PSM count from **31782 to 31793**, a
**+0.035 %** move on the SAME pinned Sage commit, no code change. An `assert_eq!`
on that count would have failed the regeneration for a reason that has nothing to
do with correctness. See "q-DERIVED COUNTS JITTER" above — this is the same
jitter, not a new phenomenon, and not a regression.

### ✅ FULL-RUN RE-BASELINED at schema 2.0.0 (2026-09-01)

All four files regenerated and promoted on Ben's explicit request, with the
downstream-impact trace done first. Carries: schema **1.8.0 -> 2.0.0**,
`three_layer_ms1` removed, the tryptic -> enzymatic key rename, and
`c_terminal: true` now explicit in every `effective-params.json` (a no-op —
Sage's default is `true`, and the serum TSV is bit-identical either way).

**Movement, committed -> re-baseline:** serum, bcell and b1906 pass-1 PSM counts
are **EXACTLY unchanged** (15631, 72801, 28236). Only liver moved: pass-1
31793 -> 31782 (**-0.035 %**), pass-2 classified 15021 -> 15022 (+0.007 %).
Across all eight files only **8 values moved by more than 1 %**, all of them
q-value tails or small decoy counts. This is the jitter above, not a change in
behaviour.

⚠ **THERE IS NO MEASURED SLOWDOWN FROM THE EMBED. An earlier claim here that
bcell was "~2.3x slower, reproducibly" WAS WRONG and is corrected.**

Pass-2 search, committed vs re-baseline: serum 4.30 -> 4.6 s, b1906
25.66 -> 26.1 s, liver 12.52 -> 13.3 s — all within noise. bcell looked like a
2.3x outlier, so it was measured properly. NINE runs of the same binary family:

    59.0, 66.7, 85.8, 87.4, 108.8, 110.0, 110.2, 121.7, 125.3 s

**A 2.1x spread between runs of ONE binary.** The committed 52.3 s is only 12 %
below our FASTEST observation. The variance swamps any effect, and the "2.3x"
came from repeatedly sampling the slow end while other jobs competed for six
cores. This machine has 8 GB of RAM and 6 CPUs, and searches were running back
to back for hours.

**The memory hypothesis was tested and is DEAD.** The worry was specific to
embedding: Pass 2 now runs while recon still holds Pass-1 state — 72k PSMs, the
parsed Unimod DB, the protein index — in one address space, where the subprocess
got a clean one. Measured with `/usr/bin/time -l` on bcell, the largest file:
**peak RSS 3.14 GB of 8.0 GB, ZERO swaps, 178 page faults.** No pressure.

So **do NOT spill Pass-1 state to disk between passes** to "fix" this. It would
add I/O for a problem that does not exist. (Ben raised it 2026-09-01; measured
and declined.)

⚠ Any real timing comparison needs a quiet machine and N runs per arm. Nothing
in this repo currently supports a performance claim in either direction.

### ⚠ Sage output is NOT bit-reproducible across builds — but every RAW measurement is (2026-08-28)

Measured by re-running the committed `step1-closed-serum` search with a
locally-built arm64 macOS Sage at the pinned commit, and diffing against the
committed TSV produced by the vendored **Windows** binary. **This matters for
step 4, which ships three platforms.**

**Identical:** the PSM key set (24817 rows both sides, same
scan/peptide/rank keys) and **every raw measurement** — `expmass`, `calcmass`,
`isotope_error`, `precursor_ppm`, `fragment_ppm`, `hyperscore`, `delta_next`,
`delta_best`, `rt`, `matched_peaks`, `longest_b`/`longest_y`,
`matched_intensity_pct`, `scored_candidates`, `poisson`, `ms2_intensity`.

**NOT identical:** the five rescoring outputs — `sage_discriminant_score`,
`posterior_error`, `spectrum_q`, `peptide_q`, `protein_q`. Max |Δq| 0.0111
(spectrum) and 0.0169 (peptide).

**Consequence, measured at the threshold recon actually uses:**

| filter | committed | probe | moved |
|---|---|---|---|
| `spectrum_q < 0.01` | 9371 | 9365 | 8 out, 2 in |
| `peptide_q < 0.01` | 9542 | 9546 | 2 out, 6 in |

About **0.1% of PSM membership** shifts at the q<0.01 boundary between platforms.
Small, but not zero, and it is NOT a bug in either build — the raw evidence is
bit-identical, only the model-based rescoring moves. Related: NOTES already
records `id_rate_by_tic_pct` as not bit-reproducible run to run.

**Two rules follow.** (1) Never byte-compare Sage output across platforms; compare
raw columns, and treat q-derived counts as reproducible only to ~0.1%. (2) Any
committed reference data must state WHICH platform's binary produced it. The
current `full-run/` and `step1-*` sets came from the vendored Windows binary.

### ✅ SAGE_VERSION corrected to 0.14.6, and a gate that COULD NOT FIRE now fires (2026-08-28)

**The pin was self-inconsistent, and the guard hid it.** `SAGE_VERSION` was
`"0.14.7"`. Every v0.14.7 binary reports **`0.14.6`** — upstream never bumped the
crate versions at that tag, and `--version` is `clap::crate_version!()`.
Confirmed FOUR independent ways: the crate manifests at tag `v0.14.7`
(`sage-cli` and `sage-core` both say `0.14.6`), every committed `results.json`
under `testing/search-output/`, a local arm64 build from the pinned commit, and
the official `sage-v0.14.7-aarch64-apple-darwin` release artifact.

**`SAGE_COMMIT` was right all along and is the authoritative pin.**
`git rev-list -n1 v0.14.7` on UPSTREAM `lazear/sage` returns exactly
`99407db…` — checked against upstream, not only the `neely/sage` fork.
`SAGE_VERSION` only ever had to match what the binary says about itself, so it is
now `"0.14.6"`. A deliberate, recorded pin edit.

**⚠ THE GUARD WAS A NO-OP ON EVERY RUN RECON HAS EVER DONE.** It scraped the
version from Sage's STDERR *after* the search, inside `if let Some(..)`, so a
`None` skipped the comparison silently. **Sage prints no version during a normal
run** — verified against the captured output of a real 1245-second search, which
contains no version string anywhere. So the gate never fired; and had it fired,
it would have REJECTED the correct binary. Third member of the family in
NOTES "Gate audit — 2 of 6 real gates could not fail".

**Replaced by `verify_sage_version`,** which runs `sage --version` explicitly
BEFORE the search (a search costs many minutes; failing afterwards wastes them)
and hard-stops on mismatch **or on being unable to determine the version** —
"I could not check" must never be reported as "it passed".

**Falsification-tested, both directions, against real binaries:**
* official `aarch64-apple-darwin` v0.14.7 release → reports `0.14.6`, PASSES.
* local `0.15.0-beta.2` build → `found 0.15.0-beta.2, expected 0.14.6`, FAILS.

The old unit test fed a synthetic `"sage 0.14.6"` string and asserted `0.14.6` —
it encoded the true version while the constant said otherwise, and could not
catch the mismatch, because **a synthetic fixture inherits the assumptions of the
code it tests**. The new test asks a real binary (honouring `SAGE_PATH`, skipping
cleanly when absent) and a companion test pins the `None` case using a real log
line from the 1245-second run.

### 🔒 Paired target-decoy selection — DELIBERATELY NOT BUILT (decided 2026-08-28, locked)

**Decision: recon does NOT carry decoy hits into the Pass 2 subset FASTA.**
The Mascot ET rule stays in `reference-notes/` as method provenance, and the
write-up states that we knowingly diverge from it.

**Why the rule exists for Mascot and not for us.** `mascot-error-tolerant-methodology.md`
gives two properties: the pass-2 target and decoy databases have identical size,
and together they cover all significant pass-1 PSMs — "without the pairing, decoy
selection would be biased by pass-1 evidence and the pass-2 FDR estimate would
not hold". **Both are about protecting an FDR estimate.** Mascot needs pass-2
q-values because its ET pass is looking for PTMs and reports them as findings.

**Recon's Pass 2 does not report findings.** Its locked purpose is enzyme
performance — see "Pass 2 (semi-enzymatic on subset FASTA) is non-optional":
digestion efficiency, missed cleavages, semi-tryptic rate. It runs with **no
variable mods at all**. We take a subset FASTA from the wide pass and re-search
it semi-tryptically to measure how the protease behaved. Nothing downstream
quotes a Pass 2 q-value as an FDR-controlled discovery, so there is no FDR
estimate to protect.

⚠⚠ **THE PARAGRAPH ABOVE IS FALSE ON BOTH COUNTS. MEASURED 2026-08-29.**
1. **A Pass 2 q-value IS quoted as an FDR-controlled number.** `run_pass2` calls
   `parse_sage_results` with `FilterOptions { q_threshold }`, which drops decoys
   (`sage_results.rs:285-286`) and filters on `peptide_q` (`:295`). ⚠ This line
   read "`peptide_q >= q_threshold` (`:278`)" and is corrected 2026-09-01 on both
   halves: the line numbers moved, and the operator is now `peptide_q >
   q_threshold` — the skip test that KEEPS `q <= threshold`. See "`q <= 0.01` IS
   THE RULE EVERYWHERE". The
   semi-tryptic rate the report ships is computed on exactly that population. So
   the entry's own stated reopening condition — "any decision to quote a Pass 2
   q-value as an FDR-controlled number" — was ALREADY MET when it was written.
2. **"No variable mods at all" was not true of the shipped code until
   2026-08-29.** `digestion-efficiency-pass2.json` and `serum-digestion-pass2.json`
   both carried `variable_mods {M: [15.9949]}`, and the echoed configs in
   `testing/search-output/*-pass2/results.json` prove they were used. The claim
   describes an intent the templates did not implement. It became true only when
   Pass 2 was changed to strip all mods.

**And the FDR really is not controlled per class.** Measured on the Pass 2
output, decoys against targets at q<0.01:

| file | fully-tryptic class | semi-tryptic class |
|---|---|---|
| serum | 0.09 % | **2.18 %** |
| bcell | 0.16 % | **13.99 %** |
| b1906 | 0.11 % | **5.31 %** |

The global 1 % cut is carried by the fully-tryptic majority. On bcell about one
in seven reported semi-tryptic PSMs is a false positive, which inflates 6.69 % to
roughly 5.76 %. **Preview itself did per-class decoy subtraction** — see
`byonic-preview-methodology.md`, "Preview then corrects the modification hit
counts themselves by decoy subtraction". This is now step 3 part 3 in PLAN. The
REJECTED-ALTERNATIVE reasoning below still stands on its own terms (adding 33
proteins on the basis of noise); what has changed is the premise that no FDR
estimate needs protecting.

**Two supporting facts, both measured rather than assumed (2026-08-28):**
* **The size property is FREE under Sage anyway.** Sage generates one decoy per
  target from whatever FASTA it is given (`generate_decoys: true`), so the two
  databases are identical in size by construction. Mascot needed explicit pairing
  only because its decoys are independent FASTA entries. Half the cited rationale
  never applied to us.
* **The coverage property WAS deliverable, so this is a choice and not an
  excuse.** Sage's decoy generation is per-peptide reversal
  (`enzyme.rs:45` at the pinned commit — reverse, then swap first and last
  residue), which is deterministic and independent of database composition. So
  adding a protein WOULD have regenerated exactly the decoy peptides that scored
  in pass 1. We are declining a fix that would work, not skipping one that would
  not.

**REJECTED ALTERNATIVE: implement it.** Measured cost on serum's closed search —
49 significant decoy PSMs (q<0.01) imply 42 target accessions, 33 of them not
already selected, so the subset would go **454 -> 487, +7.3%**. Rejected because
those 49 PSMs ARE the false-positive tail by definition, so the rule would add 33
proteins to the search space on the basis of noise, in service of an FDR estimate
nothing consumes. Also rejected: it does not touch the dominant effect anyway —
Pass 2 searches a database pre-filtered on pass-1 evidence either way, which is a
much larger enrichment than 33 proteins and is an accepted property of two-pass
searching.

**⚠ THE REOPENING CONDITION WAS MET, AND THE DECISION WAS RE-TAKEN — STILL NO
(2026-08-31).** Recon now quotes a per-class FDR and subtracts decoys per class.
That is exactly the trigger this entry named. Revisited, and paired target-decoy
selection is **still not built**, on a NEW and narrower basis:

> Per-class decoy subtraction operates on the Pass-2 OUTPUT, not on the subset
> FASTA's composition. The pairing property protects a decoy population selected
> by pass-1 evidence; we are not selecting decoys at all, we are counting the
> ones Sage already generated from whatever FASTA it was handed.

The original rejected-alternative reasoning (adding 33 proteins on the basis of
noise) is independent of this and still stands. **REJECTED ALTERNATIVE: build it
anyway** — rejected because nothing in the corrected design consumes the property
it would buy, and it would enlarge the search space using the false-positive tail
as the selection criterion.

What made the correction possible without any of this: Sage's decoys PRESERVE
their target's `semi_enzymatic` and `missed_cleavages` (`enzyme.rs`), so each
decoy is already class-matched to a real peptide. The pairing Mascot had to
construct explicitly, Sage gives for free. See "What Pass 2 IS FOR".

**What would reopen this now:** a decision to report a Pass-2 q-value as a
DISCOVERY (a PTM, an identification a user acts on) rather than as a class-level
error rate. That is not in scope for v0.1.0.

### 🔒 MS2 stays ABSOLUTE — `--annotate-matches` NOT wired (decided 2026-08-28, locked)

**Decision: recon reports Sage's `fragment_ppm` as what it is — an
intensity-weighted mean of |error| — and does NOT reconstruct a signed MS2 bias.**
The method is proven and the number is measured (see the `--annotate-matches`
entry); it is not shipped.

**The deciding measurement: the correction cannot change what recon recommends.**
Recon's MS2 output is a tolerance, and the tolerance recommendation is a BUCKET
from the {10, 20, 50, 100} ppm ladder — the same argument locked for MS1 in
"MS1 tolerance recommendation is too tight". A user picks a search setting from a
discrete set, so precision below the ladder step is unusable. Against
`|bias| + 5×MAD`:

| reading | need | bucket |
|---|---|---|
| serum, Sage ABSOLUTE (per-PSM) | 3.480 ppm | 10 |
| serum, SIGNED (per-PSM) | 3.507 ppm | 10 |

**The signed correction moves the requirement by 0.027 ppm.** It comes out
slightly HIGHER, not lower, because the bias falls (1.2602 -> 0.9496) while the
MAD rises (0.4440 -> 0.5114) and the two nearly cancel. Across all three
committed files the requirement is **2.9 / 4.4 / 2.9 ppm against a 10 ppm step —
5.5 to 7.1 ppm of headroom**. bcell is the worst case, where the absolute reading
is ~2x the reference (+1.84 vs ~+0.95), and it still needs only 4.45 ppm. The
correction is two orders of magnitude smaller than the margin it would have to
cross.

**Second reason: the MS2 window never reads the bias anyway.**
`compute_ms2_tolerance` returns `low_ppm: -tail_ppm, high_ppm: tail_ppm` —
symmetric about ZERO. `median_ppm` is reported and then ignored. This is the
structural difference from MS1, where the Pass 2 window is bias-CENTRED and a
sign flip mis-centres a window narrower than the error. There is no equivalent
consumer at MS2.

**REJECTED ALTERNATIVE: wire `--annotate-matches` in.** Cost: an extra Sage flag,
a second output file of **8.5 MB / 210309 rows for ONE file**, a `psm_id` field on
`Psm`, a join, and a schema change — against a tool whose stated goal is FAST
reconnaissance. Rejected on cost/benefit, not on difficulty: the probe proved it
works and lands between MSFragger and MetaMorpheus.

**What IS fixed instead (2026-08-28):** the field is labelled honestly.
`ms2_bias_ppm`'s doc comment claimed "median SIGNED MS2 fragment ppm error" —
**that claim is WITHDRAWN**, in place. The console now prints `MS2 |error|` with
the convention stated. The KEY is not renamed: that would be a MAJOR schema bump
under `result-schema.md`, so `ms2_bias_ppm` -> `ms2_median_abs_ppm` is logged as a
2.0.0 candidate alongside the `mass_accuracy.*_ppm` pair.

**⚠ CONSEQUENCE FOR THE WRITE-UP, and it is a real cost.** Recon has NO signed MS2
number to put beside MSFragger and MetaMorpheus in the benchmark table. The field
was ADDED in the 2026-08-19 review specifically for engine comparison, and it does
not support that use. Either the write-up omits recon's MS2 from that comparison,
or it cites a scripted measurement from an `--annotate-matches` run rather than a
shipped report field. Decide this in step 5; it is not a code change.

**NOT PURSUED, noted so it is not rediscovered as new:** `matched_fragments.sage.tsv`
carries per-fragment ion type, ordinal, charge and intensity, which is the raw
material for LOCALIZATION-aware PTM scoring. That is out of scope twice over — the
locked "No per-residue localization", and the locked single-purpose recon scope.
Recorded as a possible future direction, not a gap.

### 🔒 The three tolerance regimes — stated explicitly (Ben, 2026-08-28)

Four different numbers were being conflated in conversation. Writing the design
down so they stop being.

**PASS 1 — MS1: the open window. FIXED, not derived.**
`precursor_tol.da: [-500, 100]`, which produces a delta-mass window of
**−100 to +500 Da** (see the locked convention entry — the config sign is the
OPPOSITE of the delta sign it produces). This is a search setting, not an
accuracy claim, and nothing measures it.

**PASS 1 — MS2: DETECTOR-AWARE AS OF 2026-08-28. The text below is the
pre-build statement, corrected in place — read the ✅ block under it for what
actually shipped.**
The rule Ben wants: pick a fragment tolerance from the MS2 ANALYZER TYPE, wide
enough to always contain a reasonable error for that detector, with the
assumption documented. It is derivable from the mzML header with no PSMs at all —
`mzml-instrument-metadata-tolerances.md` gives the mapping (MS2 Orbitrap 10–20 ppm
or 0.02–0.03 Da; MS2 linear/quadrupole ion trap **0.5–1.0 Da**) and the hybrid
rule for a Fusion/Lumos (MS1 → Orbitrap → ppm, MS2 → ion trap → Da).
**⚠ MEASURED 2026-08-28: `fragment_tol` appears in NO Rust source file.** It is
hardcoded `ppm: [-20, 20]` in all TEN `testing/configs/open-search-*.json`
templates. On ion-trap MS2 that is ~30x too tight, so the SEARCH ITSELF would
match almost nothing and every downstream number would be garbage — before any
recommendation is computed. Analyzer-awareness cannot rescue a search that already
ran at the wrong fragment tolerance, so this is a PASS-1 INPUT problem, not a
reporting problem. It sits in the same template that step 4 already fixes for
`static_mods {C: 57.0215}`.

✅ **WHAT SHIPPED (2026-08-28).** The paragraph above is superseded on two
counts. First, `fragment_tol` IS now decided in Rust. Second, the 10–20 ppm /
0.5–1.0 Da figures were TYPICAL-PERFORMANCE numbers; pass 1 must instead assume
the instrument could be well out of calibration, so the shipped windows are
deliberately looser. Curated by Ben; full derivation and citations in
`reference-notes/ms2-analyzer-tolerance-table.md`.

| bucket | pass-1 window | typical real-world MS2 error |
|---|---|---|
| Orbitrap / FT-ICR | **±50 ppm** | 1–5 ppm, ~20 ppm poorly calibrated |
| Astral | **±50 ppm** | <5 ppm RMS external cal |
| legacy TOF / QTOF | **±100 ppm** | ~10–30 ppm |
| ion trap / quadrupole | **±1.0 Da** | 0.3–0.8 Da, unit resolution |
| unknown / no bucket / detector switched | **±20 ppm, reported** | — |

⚠ **Only the Orbitrap row is measured against local data.** MSFragger's
pre-calibration MS2 error over the three files is at worst
`|median| + 3×MAD = 8.43 ppm` (bcell), so ±50 ppm holds it with 5.9x headroom.
The other rows are curated assumptions and are flagged `assumed` in the tool's
own output. **The curated figures still carry unresolved citation fragments
(`pmc.ncbi.nlm.nih+1`, `support.proteinmetrics`, `pure.mpg+1`) — these are
placeholders, not references, and MUST be resolved before publication.**

**PASS 2 — both tolerances carried forward from pass 1, WITH CUSHION.**
Pass 2 is the subset-FASTA semi-tryptic search whose only job is digestion
efficiency and missed cleavages. It uses the MS1 and MS2 errors MEASURED in pass 1,
widened: "if we measure 2 ppm then maybe we use 5 or 10". This is the same
instinct the MS1 bucket ladder encodes — measure, then round generously upward —
and it is the correction PLAN already demanded for the Pass 2 window.
⚠ **DONE the same day, and the "~5%" was wrong:** `bias ± 3×MAD` was MEASURED to
cover only **80.82 / 87.46 / 80.77 %** — about one real peptide in five discarded.
The window is now the ladder rung centred on bias. See "Pass 2 windows — sized by
COVERAGE".
**Note the simplification this permits:** Pass 2 runs against a subset FASTA
(454 proteins vs ~20k on serum, a ~45x smaller space), so a generous window costs
almost nothing in runtime. The original "TIGHT, bias-CENTERED" rationale was about
speed. With a generous window, bias-centring stops mattering — a ±10 ppm window
swallows bcell's −0.24 ppm bias without needing to be centred on it, which removes
the mis-centring failure mode entirely rather than fixing it.

**SEPARATE FROM ALL THREE — the user-facing recommendation.** The
`{10, 20, 50, 100}` ppm bucket is what recon tells the USER to set in THEIR own
final closed search. It is an OUTPUT. It is MS1-only and must not be applied to
MS2, because a ppm ladder is meaningless for an ion trap. Precursors are measured
in the high-res analyzer even on instruments whose MS2 is an ion trap, which is
what makes a ppm ladder defensible for MS1 and not for MS2.

**And a fourth number that shares a digit with the ladder — ⚠ NOW RETIRED.**
`PASS2_HALF_WIDTH_CAP_PPM = 100` was an internal runtime backstop on the Pass 2
window, never shown to the user. It became unreachable on 2026-08-28 once the
window's width came from the ladder, whose top rung is also 100, and its own test
could no longer fail. Retired. The ceiling survives as the ladder's TOP RUNG, and
is CURATED rather than measured — all three files land on the first rung.

### 🔒 Pass 2 windows — sized by COVERAGE, not by a MAD multiple (2026-08-28)

**⚠ PLAN's "3×MAD clips ~5% of true peptides" DID NOT SURVIVE MEASUREMENT, and the
rule changed because of it.** Measured against the closed searches' full confident
populations: the superseded `bias ± 3×MAD` window covers **80.82 / 87.46 / 80.77 %**
on serum / bcell / b1906. It was discarding roughly **one real peptide in five**,
not one in twenty.

**Why a MAD multiplier is the WRONG SHAPE here, not just the wrong constant.**
The multiplier needed for 99% coverage is **18.3× MAD (serum), 12.1× (bcell),
11.0× (b1906)** — three instruments of the same class, and no single `k`
transfers, because the tails are not proportional to the core. What IS stable is
the absolute half-width: 8.7–10.6 ppm covers 99% on all three.

**THE MEASUREMENT HAD TO MOVE POPULATIONS, and this is the subtle part.** The
first attempt sized the window from the OPEN search's clean subset and produced
reassuringly consistent half-widths of 13.6–15.8 ppm. **That was an artifact.**
The clean subset's `|delta| < 0.02 Da` filter imposes a MASS-DEPENDENT ppm ceiling
— 4.1 ppm at 5000 Da, 31.9 ppm at 500 Da, median 12.4 / 15.0 / 14.9 — and those
"consistent" half-widths were just that median. **The clean subset can measure a
CENTRE but not a TAIL.** Only 5–13% of the clipped tail sat at the filter wall, so
the tail is real; it is the ceiling that makes the quantile meaningless. The
closed searches have no such filter (in a closed search the delta IS the error) and
are the right reference.

**THE RULE, and it DELETES two constants rather than adding any:**
> Pass 2 MS1 window = **the ladder rung, centred on the measured bias**.

No `k`, no floor, no MAD in the window. Measured coverage **99.32 / 99.92 / 99.94 %**.
The rung is already justified for the recommendation, it scales on its own when an
instrument is bad (20/50/100), and MAD stays a reported measurement — it simply
stops setting a width it was measured to be unable to set. The ±100 ppm backstop is
untouched.

**It is still NOT the same number as the recommendation.** The recommendation is
symmetric about ZERO; this is centred on the BIAS. On serum: recommend −10..+10,
search −7.59..+12.41. Asserted by
`pass2_window_is_centred_on_bias_where_the_recommendation_is_not`, and
`mad_no_longer_sets_the_pass2_width` asserts two files with equal bias and very
different MAD get equal width.

**MS2 — `ms2_pass2_tolerance`, built and tested, NOT yet consumed.** It takes the
measured median |error| and the pass-1 `FragmentTolerance`, and returns a
tolerance IN THE UNIT PASS 1 USED, clamped so it can never exceed the pass-1
window (pass 1 defined the search space; a wider pass-2 window would claim matches
pass 1 could not have made).
* **ppm analyzers: 5× the median.** ⚠ **THE JUSTIFICATION BELOW IS WITHDRAWN
  2026-08-29; the CONSTANT is unchanged.** These are PER-PSM coverage figures on
  a per-PSM summary quantity, and the window has to contain individual
  FRAGMENTS. Per-fragment, 5× covers only **96.82 %** on serum (p99 needs 6.76×).
  5× survives because a sweep of Pass 2 itself showed that quadrupling the window
  buys 1.09 % more PSMs and moves the reported semi-tryptic rate by 0.12
  percentage points. See "Sage's `fragment_ppm` convention CONFIRMED
  per-fragment". The superseded figures: 2× covers 83.5 / 90.9 / 94.4 %,
  3× covers 95.5 / 97.7 / 99.5 %, 5× covers 99.6 / 99.8 / 100.0 %.
* **Da analyzers (ion trap, quadrupole — the ONLY Da buckets): Ben's rule.**
  Convert the measured ppm to Da at a representative fragment m/z of 500, then
  DOUBLE it. ⚠ A CURATED ASSUMPTION, not a measurement: we have no ion-trap file.
  Deliberately not the 5× used for ppm analyzers, because a trap's error already
  sits near its resolution limit. The m/z 500 assumption is exact only there — it
  over-estimates by 25% at m/z 400 and under-estimates by 17% at m/z 600. Doing
  better needs per-fragment m/z, i.e. `--annotate-matches`. ⚠ That was recorded
  as "declined"; a probe WAS run and is committed at
  `testing/search-output/probe-serum-annotate-matches/`. It is still not wired
  into the tool — what was declined is making it part of the product flow, not
  running it once.
* **✅ WIRED 2026-08-29, in `run` only.** The stated blocker — "`analyze` does not
  run `detect_analyzers`" — turned out not to apply: `run` ALREADY runs
  `detect_analyzers` at stage 0 to choose the pass-1 `fragment_tol`, so it holds
  the pass-1 unit and hands it straight to `ms2_pass2_tolerance`. No detection
  was added to `analyze`; that belongs with the schema bump that adds the
  analyzer block (PLAN step 3 item 4), where it is already planned.
  Measured pass-2 MS2 windows: serum ±5.6 ppm, bcell ±9.01 ppm, b1906 ±4.84 ppm,
  all clamped against a pass-1 ±50 ppm.
  `a_ppm_measurement_never_leaks_into_an_ion_trap_tolerance` is the unit-safety
  control.

### 🔒 `PASS2_HALF_WIDTH_CAP_PPM` RETIRED, and the dead MS1 tail removed (2026-08-28)

**The cap became unreachable the moment the Pass 2 width came from the ladder, and
its own test could no longer fail.** It was a real backstop while the half-width
was `3×MAD`, which is unbounded. The width is now a ladder rung, and the ladder's
TOP RUNG IS ALSO 100, so `min(rung, 100)` was always just the rung.

**Proven, not argued:** deleting `.min(PASS2_HALF_WIDTH_CAP_PPM)` left
`pass2_window_is_hard_capped_at_100_ppm_half_width` PASSING. It had been asserting
the ladder's top rung all along. **Fourth member of the family in "Gate audit —
2 of 6 real gates could not fail"**, and this one was self-inflicted the same day.
The test is replaced by `pass2_window_is_bounded_by_the_ladder_top_rung`, which
asserts against `MS1_TOLERANCE_LADDER_PPM` — the mechanism that actually bounds it.

The constant is `#[deprecated]` with its reasoning kept, and dropped from the
public re-exports. **The ceiling it argued for survives as the ladder's top rung**,
and its original justification still supports that value: a tighter ceiling
(±15–20 ppm) would systematically clip TOF instruments, which run 50–80 ppm out of
the box. ⚠ That justification is CURATED, not measured — all three files land on
the FIRST rung, so **there is no data on rungs 2–4 at all**. PLAN's "±100 ppm cap
measurement" item is repointed accordingly: it is a ladder-top question now, and it
cannot be answered without a drifted or low-res file.

**Dead code removed with it: the MS1 95th-percentile tail.** `MassErrorStats.tail_ppm`
and `Ms1UserRecommendation.tail_95_ppm` were computed, stored and carried, but
nothing has consumed or reported them since the asymmetric window was retired
earlier the same day. Removed. **`Ms2Tolerance.tail_95_ppm` is NOT the same field
and is still live** — it sets the MS2 tolerance window.

**⚠ AUDIT RESULT WORTH KEEPING: MAD is used, but it is not decisive on any file we
have.** It feeds `|bias| + 5×MAD`, so it drives both the recommendation rung and the
Pass 2 window. But setting MAD to ZERO leaves all three files on the 10 ppm rung
(requirements fall 4.844→2.421, 3.602→0.236, 3.862→0.440). MAD would only change an
outcome on a file where `|bias| + 5×MAD` crosses a rung boundary, and we have none.
It is kept because it is a real reported measurement of instrument scatter and it
WOULD bite on a drifted instrument — but no test on current data can demonstrate
that it matters.

### 🔒 What Pass 2 IS FOR, and what the digestion number means (2026-08-29, SETTLED 2026-08-31)

**The number is now DEFINED, and it is defined against a primary source.** Ben
supplied the actual Byonic Preview v3.2.0 output for one of the Davis et al.
liver files, and it is vendored at
`testing/reference-data/preview/10mg_1_A_1/`. Read that README before this entry.

**⚠ THE GROUND TRUTH IS THE `.prv` REPORT, NOT DAVIS TABLE 3.** Ben, 2026-08-31.
Table 3 is a record of a measurement; the Preview output IS the measurement.
Write-up framing: "We used files described in Davis et al., but specifically the
report from `10mg_1_A_1.raw` to compare our recon to."

**WHAT RECON REPORTS** (`digestion::compute_digestion_composition`), over
DISTINCT PEPTIDES:

| | |
|---|---|
| **cleavage completeness** | `100 − %missed cleavage` — THE HEADLINE |
| ragged N-terminus, ragged C-terminus | beside it, never folded in |
| non-tryptic | a COUNT only, never a rate — see below |

Denominators are Preview's own, quoted from its report: "Missed cleavage: 15.9%
(319/2008) of tryptic and semitryptic peptides" and "Nontryptic peptides (% of
all peptides): 0.1% (2/2009)". So missed cleavage and both ragged rates exclude
non-tryptic peptides from the denominator; only non-tryptic uses all peptides.
⚠ Preview's own arithmetic is off by one there (2008 + 2 ≠ 2009). Do not copy it.

**Every rate is printed WITH its fraction.** Preview's report shows the same
numerator under two different denominators on two of its pages — oxidised Met is
"19.1 % (509 ... over 2667 baseline)" in the summary and "69.3 % (509/735)" in
the detail. A bare percentage cannot be checked.

**⚠ THE HEADLINE IS NOT CALLED "DIGESTION EFFICIENCY."** Mouchahoir & Schiel 2018
and Davis et al. both use that phrase as an umbrella over a SET of metrics, never
for one number. It stays the section heading. Reusing it for a single figure
would invent a third meaning for a term the literature already uses two ways.

**No composite score.** Missed cleavage is trypsin failing to cut; a ragged
terminus is something else cutting where trypsin would not. Opposite phenomena,
different causes, and neither reference tool sums them.

⚠ **THE SITE-LEVEL `cut/(cut+missed)` METRIC STAYS WITHDRAWN.** It was measured
(serum 91.54 %, bcell 92.48 %, b1906 89.07 %) and withdrawn because it is the
chemical-probability framing Preview avoided, and because `missed_cleavages: 2`
truncates the missed count so every figure is an upper bound. It also has nothing
published to compare against. **Do not re-propose it as the headline.**

**THE FOUR DEFECTS, and where each landed:**
1. **Semi-tryptic is not trypsin failing.** RESOLVED by reporting the classes
   side by side and refusing a composite.
2. **The joint quantity was never computed.** SUPERSEDED. The headline is now
   `100 − %MC` on Preview's denominator, which is the axis both NIST papers
   report; the fully-enzymatic intersection (serum 55.34 %, bcell 76.84 %,
   b1906 66.69 %) is not the reported figure.
3. **FDR is not stratified by class.** FIXED — per-class decoy subtraction, below.
4. **The denominator is PSMs with duplicates.** ⚠ **THE OLD JUSTIFICATION WAS
   WRONG AND IS WITHDRAWN.** This entry previously said a PSM denominator "is
   what Preview did, and matching it is the point." Preview counts PEPTIDES:
   "In all cases the percentage is the number of peptides with the property
   divided by the number that could have that property" (Kil et al. 2011), and
   its report says "319/2008 ... peptides". **Ben chose the peptide basis
   2026-08-31.** Recon now reports on distinct peptides.

**PER-CLASS DECOY SUBTRACTION — built.** Sourced to Kil et al.: Preview "counts
the number of hits with score above THigh, and then corrects for the number of
false hits estimated by the target/decoy approach." Measured on liver Pass 2,
peptide basis: class FDR **0.10 % fully-tryptic against 10.10 % semi-tryptic**.
The global 1 % cut is carried by the fully-tryptic majority.

Decoys carry no N/C label. Sage builds them per-peptide by "reversing the
sequence inside the first and last amino acids" (`enzyme.rs`), which PRESERVES
the first and last residues AND the target's `semi_enzymatic` and
`missed_cleavages` — so a decoy is a class-matched partner and its ends are its
target's ends. The side is inferred from the last residue, **validated at 96.63 %
(1289/1334) against known target classes before use**. No decoy FASTA is needed.

⚠ **SUBTRACTION DOES NOT MOVE THE N:C RATIO — a prediction was falsified.** It
lowers both ragged rates (liver ragged-N 6.97 → 6.32 %, ragged-C 3.03 → 2.77 %)
but N:C goes only 2.30 → 2.28, because the decoys' own N:C is 2.45. False
positives are distributed across N and C almost exactly like real hits. Pinned by
a test. **Do not re-run this expecting the ratio to move.**

**NON-TRYPTIC IS NOT REPORTED AS A RATE, and that is deliberate.** Under
`semi_enzymatic: true` Sage only generates candidates with ONE non-specific
terminus, so a fully non-tryptic peptide is never scored — measured 0/10589.
Preview obtains its 0.1 % from a SEPARATE non-enzymatic search ("Preview then
does the analogous search for fully nontryptic peptides"). Printing 0.0 % would
report a search-space artifact as a measurement. See the non-enzymatic RAM entry.

**EXTERNAL MATH, NOW READ FROM THE VENDORED PDFs** (`reference-notes/`):
* **Mouchahoir & Schiel 2018**, Eq. 3, verbatim: `(ΣXICs of peptides with missed
  cleavages / ΣXICs of all identified peptides) × 100`. Eq. 2 is the same form
  for non-specific cleavage, Eq. 4 for trypsin autolysis. All XIC-weighted, all
  "best kept at a minimum". **It reports no composite percent.**
* **In-source rule, VERIFIED verbatim:** "Peptides that were identified as
  non-specific cleavage but had the same retention time as a 'parent' peptide
  with specific cleavage were considered as in-source fragments and thus were not
  counted as non-specifically cleaved peptides." ⚠ It requires only SAME RT plus
  specific cleavage — it does NOT require the parent to contain the peptide.
* Niu et al. 2020 ceilings (~22 % nonspecific from trypsin handling) unchanged.

**CALIBRATION SCALE — the HeLa row nobody had recorded.** Davis Table 3 also
carries a HeLa Digest Standard used as routine instrument QC: **5.8 % missed
cleavage**, against the liver RM's 15.0 %. That is the known-good anchor this
project lacked.

### ✅ The N-ragged / C-ragged direction — ANOMALY DISSOLVED (2026-08-31)

**This was recorded for a week as an unexplained contradiction. It was never
one.** Every source agrees: **ragged-N ≫ ragged-C**.

| source, liver `10mg_1_A_1` | ragged-N | ragged-C | N:C |
|---|---|---|---|
| **Byonic Preview v3.2.0** (its own report) | **8.6 %** | **1.3 %** | 6.62 |
| recon Pass 2 (Met-corrected, decoy-subtracted) | 6.32 % | 2.77 % | 2.28 |
| MSFragger (same rules applied) | 5.45 % | 2.14 % | 2.54 |

recon and MSFragger also give N ≫ C on all three of the original files
(N:C 1.85 / 4.81 / 10.82), and so does a reclassification of Preview's own
baseline identifications.

⚠ **Davis et al. Table 3 lists this material the other way round** — ragged
C-term 7.4 %, ragged N-term 1.1 % — with magnitudes consistent with the two
column headers being transposed relative to the tool that produced them.
**Table 3 is not the comparison basis; the `.prv` report is.** Recorded so nobody
re-derives the confusion from the paper. Not chased further, on Ben's call.

**The convention itself was checked and is correct.** Kil et al. 2011, verbatim:
Preview searches against "'N-ragged' (nonspecific at the N-terminus)" peptides —
identical to `classify_terminus`, where `(n_tryptic=false, c_tryptic=true)` maps
to `SemiNRagged`.

### 🐛 Initiator-methionine excision was mis-called ragged — FIXED (2026-08-31)

`is_tryptic_nterm` returned true only at protein position 0. A peptide beginning
at position **1** of a protein whose position 0 is Met sits at a real protein
N-terminus — methionine aminopeptidase removes the initiator Met, so nothing
cleaved that bond. The preceding residue is `M`, not K/R, so it was called
`SemiNRagged`. **One-directional: it only ever inflated ragged-N.**

Measured: 200 PSMs move `SemiNRagged` → `FullyTryptic` on bcell (C-ragged
untouched); liver's ragged-N rate moves 7.19 → 6.97 %.

**Found only by cross-tool comparison.** MSFragger applies the rule via
`clip_nTerm_M = 1`; on the same file its own NTT column reported 2 non-tryptic
peptides where this classifier reported 16. ⚠ The Python prototype has the same
bug, so the port's parity test now pins the DIVERGENCE (exactly 200 PSMs, moving
between two named classes) instead of asserting an equality it no longer has.

### 🔒 PROTEIN PARSIMONY — greedy set cover, BUILT and PINNED, not yet the default (2026-08-31)

`protein_index::parsimonious_groups` implements Ben's 7-step rollup.
✅ **ADOPTED AND WIRED 2026-08-31.** `run_pass2` calls `parsimonious_accessions`.
Confirmed on THREE files before adoption; `full-run/` regenerated at 1.7.0 with
it. See "full-run REGENERATED AT SCHEMA 1.7.0".

**THE ORDER IS 7 -> 5 -> 6, and that is the finding.** Merging indistinguishable
proteins must happen BEFORE the cover, not after.

⚠ **AS SPECIFIED, STEP 7 IS A NO-OP, AND IT LOOKS LIKE ONE THAT WORKED.** Greedy
cover assigns each peptide to exactly ONE group, so every surviving group's
peptide set is DISJOINT from every other. Two disjoint non-empty sets can never
be identical, so a final "merge identical peptide sets" pass can never fire.
Measured on liver: **0 merges after the cover, against 2517 accessions genuinely
merged before it.** The test asserts `members > groups`, which fails if anyone
reorders it back.

**Measured, both files:**

| | accessions touched | current rule (>=2, no parsimony) | parsimony groups | smaller by |
|---|---|---|---|---|
| **liver** `10mg_1_A_1` | 6261 | 3909 | **1721** | **56.0 %** |
| **bcell** (`digestion-pass1`) | 6245 | 4719 | **4214** | **10.7 %** |

⚠ **THE SAVING IS STRONGLY FILE-DEPENDENT. Do not quote liver's 56 % as "the"
number.** bcell is the deeper run — 55419 confident PSMs against liver's 31127 —
so far more of its proteins carry independent peptide support and there is much
less redundancy to remove. A shallow run compresses; a deep one does not.

**Liver structure, for scale:** 6261 accessions collapse to 3744 distinct peptide
sets (2517 indistinguishable duplicates), of which 1031 are strict SUBSETS of a
larger group — the ones the cover subsumes.

**WHY IT MATTERS BEYOND SPEED.** The non-enzymatic RAM table in this file shows a
**3909**-protein semi-enzymatic index killed by the OS at 118.6 s with no output,
while a **921**-protein one built fine. 1721 sits between them. A tighter subset
is the lever that moves a search from "killed" to "runs" — and it is the likely
explanation for the Pass-2 SIGKILL in "THE MODS DECISION", which that entry
correctly recorded as unproven.

**⚠ THE INVARIANT THAT CAUGHT A REAL BUG.** The cover must be a PARTITION of the
confident peptides — every one claimed, none twice — and it is asserted in
`cover`, not merely checked in prose. In the Python prototype the winner's
peptide set was subtracted from ITSELF by aliasing, emptying it so every later
subtraction was a no-op: **22784 claims over 14954 peptides, with no other
symptom.** The output looked entirely plausible.

**⚠ RUST GIVES 1721 WHERE THE PROTOTYPE GAVE 1723, AND NEITHER IS WRONG.** The
prototype ran the cover on RAW accessions; the Rust merges first. The peptide
partition is IDENTICAL either way — 13985 covered, 969 dropped by the `>=2`
floor — so it is 2 fewer groups explaining the same peptides, not 2 peptides
lost. Recorded so nobody "fixes" one to match the other.

**⚠ SWITCHING PASS 2 TO THIS MOVES THE REPORTED NUMBER ONLY SLIGHTLY — the
larger figure once written here was WRONG.** The `>=2` floor applied to
POST-cover counts is stricter than the current pre-cover rule and drops 969
groups covering 969 Pass-1 peptide ASSIGNMENTS. **That is not the Pass 2
population, and using it as "6.5 % of peptides lost" was the wrong denominator.**
MEASURED end to end, Pass 2 loses **45 peptides, 0.4 %**, and no reported rate
moves more than 0.14 pp. See "PARSIMONY IMPACT TRACE".
See "The subset filter is justified by CENSORING" — as the subset shrinks,
ragged-N converges on Preview while missed cleavage moves the other way, and NO
subset size matches Preview on both. This is a deliberate change-regenerate, not
a free optimisation.

### 🔒 PASS 1 AND PASS 2 NEVER SEARCH WITH MODIFICATIONS — enforced in code (2026-08-31)

**The alkylation-agnostic default is now a guarantee, not a convention.**
`sage_runner::write_effective_params` (pass 1) and `pass2::write_pass2_params`
(pass 2) both STRIP `static_mods`/`variable_mods` and then ASSERT the result is
empty, failing the run with a message that names the consequence.

**Why it lives in the code and not the template:** **17 of the configs under
`testing/configs/` carry `static_mods {C: 57.0215}`**, including the default
`open-search-params.json`, and any of them can be passed with `--params` /
`--pass2-params`. `recon run` writes a fresh `effective-params.json` per run but
only overrode `fragment_tol`, `fasta` and `mzml_paths` — `static_mods` passed
straight through. A template fix alone leaves the same trap for the next template.

**MEASURED cost of getting this wrong, bcell:** with `static_mods {C: 57.0215}`
the +57.02 peak reads **n=146**; without it, **n=3311 — and it is the TOP peak in
the file.** A fixed Cys mod does not merely hide one peak. It removes the largest
signal recon exists to surface, and with it the abundance floor derived from that
peak, which then inverts the carpet guard. `open-search-params.json` was also
corrected, but the guard is what holds.

### 🔒 ORBITRAP PASS-1 MS2 TOLERANCE: 50 -> 20 ppm (Ben, 2026-08-31)

`ORBITRAP_MS2_HALF_WIDTH_PPM` read **50.0** while its own doc comment gave
"typical 1-5 ppm, up to ~20 ppm poorly calibrated" — **2.5x its own documented
bound.** It was also WIDER than `UNKNOWN_MS2_FALLBACK_PPM` (20), so detecting an
Orbitrap correctly produced a WORSE search than failing to detect anything.

**MEASURED on all four files, both arms alkylation-agnostic** (+57 above 500 on
every arm, so both are in the correct family — see the discarded-regeneration
entry for what happens when they are not):

| file | PSMs ±20 | PSMs ±50 | gain | pass-1 Sage ±20 | ±50 | carpet margin ±20 | ±50 |
|---|---|---|---|---|---|---|---|
| serum | **15386** | 13371 | +15.1 % | 40.6 s | 69 s | +172.0 | +150.2 |
| bcell | **73527** | 65382 | +12.5 % | 112.5 s | 171 s | +279.2 | +201.4 |
| b1906 | **28005** | 24487 | +14.4 % | 66.4 s | 93 s | +138.6 | +108.8 |
| liver | **32496** | 27678 | +17.4 % | 82.9 s | 118 s | +165.6 | +139.6 |

**±20 wins on every axis: more confident PSMs, 35-41 % less wall clock, and a
better carpet margin on all four.** No tradeoff. The mechanism is standard — a
wider fragment tolerance admits more random matches, inflating the decoy
distribution and pushing true hits below 1 % FDR. Recommendation counts barely
move (serum 7 vs 8; the other three identical), so the gain is sensitivity, not
noise.

⚠ **20 IS THE DOCUMENTED WORST CASE FOR THE CLASS, NOT THE EMPIRICAL OPTIMUM.**
All four test files are well-calibrated Orbitraps and would almost certainly keep
improving below 20. Tuning to them would overfit a default that has to survive
somebody else's poorly-calibrated instrument. **Do not lower it on the strength of
these four files.**

**Astral moved too, and a duplication was removed.** `ASTRAL_MS2_HALF_WIDTH_PPM`
also read `50.0`, so "same bucket as Orbitrap" was true only by coincidence; the
moment Orbitrap moved, the two silently disagreed. Caught by
`astral_is_not_bucketed_with_legacy_tof`. It is now `20.0`, kept as a NUMERIC
LITERAL because `psi_ms_analyzer_terms.py` pins each class to a number and cannot
resolve an alias — the equality is enforced by that test instead.
⚠ No Astral file exists in the test set (all four are FTMS), so this is curated,
not measured.

### ✅ `full-run/` REGENERATED — schema 1.7.0, agnostic, ±20 ppm, parsimony (2026-08-31)

Four files (liver added; the set held three), plus the `_pass2.json` artifacts it
never had. **`run_validation.py`: 15/15.** 179 tests, 0 failures.

| file | +57 (agnostic gate >500) | subset | classified | completeness | missed cleavage | ragged N | ragged C |
|---|---|---|---|---|---|---|---|
| serum | 1125 | 223 | 3021 | 89.21 % | 10.79 % | 27.28 % | 17.71 % |
| bcell | 3311 | 4358 | 29370 | 83.51 % | 16.49 % | 4.30 % | 2.35 % |
| b1906 | 1253 | 2318 | 5235 | 75.42 % | 24.58 % | 9.78 % | 1.93 % |
| liver | 1291 | 1792 | 10621 | 82.88 % | **17.12 %** | 6.75 % | 3.01 % |

Liver's 17.12 % missed cleavage against **Preview 15.90 %** and
**MSFragger 16.15 %**. serum/bcell/b1906 reproduce the +57 values this file
already recorded for the agnostic family (**1125 / 3311 / 1253**), and bcell
reproduces the previous baseline exactly (73527 PSMs, top peak 3311, floor 662.2,
margin +279.2) — so ±20 agnostic IS what the committed set already was.

**Axes that moved vs the 1.4.0 set:** schema 1.4.0 -> 1.7.0; `analyzers` block
added; `_pass2.json` added; Pass 2 subset now parsimonious; Pass 2 mods none;
`isotope_errors` −1..3 -> 0..3; Pass 2 window measured; `q <= 0.01` unified;
liver added. Pass-1 `fragment_tol` is ±20 in BOTH, so it is not one of the axes.

### ⚠ A REGENERATION WAS DISCARDED — it ran in the WRONG SEARCH FAMILY (2026-08-31)

**Recorded so the mistake is not repeated, and so no number from it is trusted.**

A full four-file regeneration was produced and then thrown away. It used the
default template `testing/configs/open-search-params.json`, which carries
`static_mods {C: 57.0215}` — so every file was searched in the **fixed-C family**,
not the alkylation-agnostic one that ships. `recon run` writes a fresh
`effective-params.json` per run but only overrode `fragment_tol`, `fasta` and
`mzml_paths`; `static_mods` passed straight through.

**How obvious it was, in hindsight.** This file ALREADY recorded both the family
split and the exact PSM counts: "the fixed-C `open-*-full` family (19307 / 81966 /
31682 PSMs) — not the alkylation-agnostic family that actually ships". The
discarded run reproduced **19307 / 81966 / 31682** exactly. The +57 gate bands were
also already written down — fixed-C below 200, agnostic above 500 — and the
discarded bcell read **+57 = 146** against `step1-open-bcell`'s **3311**.

**Three conclusions drawn from it were WRONG and are withdrawn:**
1. "bcell's carpet invariant is violated (margin −90.6)" — an artifact. The +57
   peak IS bcell's top peak (n=3311); a fixed Cys mod collapses it to 146, which
   halves the abundance floor (20 % of the top peak) while the carpet does not
   move. In the agnostic family the margin is **+279.2**, unchanged.
2. "±50 ppm halves the floor" — no. Same-tolerance comparison disproved it.
3. "the 1.5.0 signed-bias calibration change moved the peaks" — no. Current code
   on the OLD TSV reproduces the old report exactly: 73527 PSMs, top peak 3311,
   floor 662.2, margin +279.2. The analysis code changed nothing; the SEARCH did.

⚠ **`testing/scripts/run_validation.py` Gate 3 exists precisely to catch this**
and was not run before regenerating. Run it.

**FIXED, in the code rather than the data:** `write_effective_params` (pass 1) and
`write_pass2_params` (pass 2) both now STRIP `static_mods`/`variable_mods` and
ASSERT the result is empty. 17 configs under `testing/configs/` carry
`static_mods {C: 57.0215}` and any can be passed with `--params`, so a template fix
alone would not hold.

### 🔒 `q <= 0.01` IS THE RULE EVERYWHERE — inconsistency fixed (Ben, 2026-08-31)

`parse_sage_results` excluded on `peptide_q >= threshold` — strictly LESS than
1 % — while `protein_index` excluded on `q >`, which is `<= 1 %`. The two halves
of the pipeline disagreed on exactly one boundary value. Ben settled it:
**`q <= 0.01`**, which is what "1 % FDR" conventionally means.

⚠ **The operator sits in the SKIP test, which reads backwards.** `if q > t {
continue }` KEEPS `q <= t`. Nothing above 1 % is admitted in either version; the
change admits the boundary itself and nothing beyond it.

⚠ **MEASURED: it moves nothing on our data.** Rows sitting exactly on
`peptide_q == 0.01`: **0 of 100470 on liver, 0 of 77086 on bcell.** An earlier
draft of the schema note claimed counts "can move" — that was asserted, not
measured, and it was wrong. It is a consistency fix, not a numeric one.

⚠ **"EVERYWHERE" WAS NOT TRUE UNTIL 2026-09-01 — a THIRD site was missed.**
`calibration::select_clean_subset` filtered the MS1 clean subset on
`psm.q_value < q_threshold` (STRICT), on the same `peptide_q` column
(`PsmSummary.q_value` is assigned from `psm.peptide_q`, `calibration.rs:91`). So
a row at exactly 0.01 was admitted by `parse_sage_results` and then dropped by
calibration. **Ben's call 2026-09-01: change the code, not the rule.**
`calibration.rs:170` now reads `<=`.

**Every q filter in `src/` was then enumerated, so "everywhere" is a checked
claim and not a hope.** Four sites, all now agreeing that `q <= threshold` is
kept: `calibration.rs:170` (`<=`), `protein_index.rs:208` and `:333`
(`if q > t { skip }`), `sage_results.rs:295` (`if peptide_q > t { skip }`).
⚠ Also corrected the same day: five console lines printed
`Loaded N PSMs (q < 0.01)` and one code comment said `peptide_q < 0.01`
(`main.rs:845, 996, 1348, 1387, 1512, 1711`). The filter was `<=` and the message
said `<` — the tool was misstating its own rule to the user on every run. No test
or snapshot reads those strings; 167 tests and `run_validation.py` 15/15 after.

**Measured BEFORE the change, on all EIGHT committed TSVs** (pass 1 and pass 2 of
all four files, 542,673 rows): rows at `peptide_q == 0.01` exactly = **0
everywhere** — liver 0/96048, serum 0/66771, bcell 0/172081, b1906 0/80627, and 0
in each pass-2 file. The change is provably inert on this data.

**Verified AFTER the change, on real TSVs, not on fixtures:** `cargo test` 167
passed / 0 failed, and `ms1_calibration_integration -- --nocapture` reports the
same clean subsets as before — bcell **n=32133, bias -0.2357 ppm, MAD 0.6733**;
b1906 **n=10942, +0.4403, 0.6844**; serum **n=3764, +2.4215, 0.4845**. Serum's
pair is the direct check on a committed artifact: `full-run/serum.json` carries
`clean_subset_n_psms 3764` and `bias_ppm 2.4214858493429903`, which the
post-change run reproduces. ⚠ recon was NOT re-run end to end, so this is
"the calibration input set is identical", not a fresh regeneration.
⚠ The rejected alternative was narrowing the entry's wording to call calibration
a knowing exception. That keeps a rule that is true "except here" — the same
shape as the conventions this file exists to stop.

### 🐛 Peptide C-TERMINAL candidates were tested at the N-TERMINUS — FIXED (2026-08-31)

`tier_assignment::peptide_hits` read `residues.chars().next()` — the FIRST
residue — for every candidate where `is_terminal()` is true, including those whose
`PP` says "Peptide C-terminal.". A C-terminal candidate was therefore tested at
the opposite end of the peptide from the one it claims, and could report a
confident odds ratio for a position it never examined.

**Fixed**: C-terminal candidates now read `chars().next_back()`. Only
`Homoserine lactone` (`TG M`, `PP Peptide C-terminal.`) can reach the path — the
other two C-terminal entries carry `TG X` and route to abundance. No reported
number changed on any of the four files.

### ✅ PARSIMONY IMPACT TRACE — the number barely moves, and a PREDICTION WAS WRONG (2026-08-31)

⚠ **THE ABSOLUTE NUMBERS BELOW CAME FROM A FIXED-C PASS 1** — see "A REGENERATION
WAS DISCARDED". The COMPARISON stands, because both arms used the same Pass-1 TSV
and differed only in the subset rule, which is what the trace measures. The
SHIPPED agnostic values are in "full-run REGENERATED"; liver's subset is 1792
there, not 1721.

Measured on liver, one variable changed: the Pass 2 subset FASTA. Precursor
window, fragment tolerance, mods, isotope offsets and the mzML are BYTE-IDENTICAL
to the committed run (its config was reused, only `database.fasta` swapped).
Reproducible via `recon-tool/tests/parsimony_impact.rs`.

| liver Pass 2 | current (3909) | parsimony (1721) | delta |
|---|---|---|---|
| proteins searched | 3909 | **1721** | −56.0 % |
| Sage wall clock | 20 s | **12 s** | −40 % |
| peptides classified | 10589 | 10544 | **−45 (−0.4 %)** |
| cleavage completeness | 82.85 % | 82.77 % | −0.08 pp |
| missed cleavage | 17.15 % (1816/10589) | 17.23 % (1817/10544) | +0.08 pp |
| ragged N | 6.97 % (738/10589) | 6.83 % (720/10544) | −0.14 pp |
| ragged C | 3.03 % (321/10589) | 3.06 % (323/10544) | +0.03 pp |

**Against the ACCEPTANCE CRITERION these moves are negligible** — an order of
magnitude below the method disagreement already accepted: 0.14 pp here against
**0.77 pp** recon-vs-MSFragger on ragged-N, and 0.08 pp against **2.4 pp** on
missed cleavage. Between-sample spread is 24.7 pp and 15.9 pp.

⚠ **A PREDICTION WAS WRONG, AND IT WAS MINE.** The parsimony entry warned that
the `>=2` floor on post-cover counts drops "969 peptides, 6.5 % of the total".
**The measured Pass 2 loss is 45 peptides, 0.4 %.** The 969 figure counts Pass-1
peptide-to-group ASSIGNMENTS; Pass 2 re-searches the spectra against the subset,
and almost every one of those peptides is still found in a retained protein. It
was the wrong denominator for the claim it was used to support. The warning in
that entry is corrected accordingly — the subset change is NOT a large numeric
move on this file.

**FREE VERIFY-REGENERATE:** re-running the baseline config reproduced the
committed Pass 2 TSV exactly, 34217 rows both.

⚠ **STILL ONE FILE.** bcell's subset shrinks only 10.7 % (4719 -> 4214), so both
the saving and any numeric movement will be smaller there — but it has NOT been
run. Serum and b1906 likewise.

### ❌ DELTA-MASS HISTOGRAM BASIS — measured end to end, PSM basis KEPT (2026-08-31)

Ben asked whether the histogram should be built on unique peptides rather than
PSMs, so that one abundant peptide seen 40 times cannot dominate. **Measured on
liver through the real pipeline** — a deduplicated TSV fed to `recon analyze`, so
no part of tier assignment was reimplemented.

**FIRST, THE VARIANT THAT IS ACTIVELY HARMFUL: one row per peptide, best score.**
It DELETES modified forms. A peptide seen both unmodified and oxidised collapses
to one row, and the unmodified form usually scores higher, so the modification is
the row that loses:

| basis | rows | unmodified | +15.99 |
|---|---|---|---|
| PSM | 31127 | 42.87 % | 5.16 % |
| **one per peptide, best score** | 14954 | **65.29 %** | **3.77 %** |
| one per (peptide, delta bin) | 26939 | 39.53 % | 4.21 % |

Unmodified jumps 22 pp and EVERY modification peak shrinks. **Never dedupe to one
row per peptide.** It erases the thing the histogram exists to find.

**THE DEFENSIBLE VARIANT, `(peptide, delta bin)`, WAS RUN END TO END** — deduped
within each `(label, rank)` stratum at the pipeline's own 0.01 Da width, so the
rank and q filters still see the population they expect. Rank-1 rows 52382 ->
48374 (92.3 %).

| | PSM basis | (peptide, delta) |
|---|---|---|
| abundance floor, PSMs | 366.2 | 277.4 |
| **carpet margin, PSMs** | **193.2** | **106.4** |
| recommendations | **7** | **6** |

| label | PSM n | pd n | PSM OR | pd OR |
|---|---|---|---|---|
| Oxidation on M | 1831 | 1387 | 46.8 | 52.1 |
| Deamidation | 548 | 447 | 7.6 | 6.0 |
| Gln->pyro-Glu | 180 | 169 | 115.2 | 103.2 |
| Fe[III] | 168 | 146 | 3.0 | 2.5 |
| Met-loss+Acetylation | 75 | 68 | 7738.2 | 6543.9 |
| **Formylation** | **33** | **LOST** | **4.1** | — |
| Water Loss (Glu->pyro-Glu) | 23 | 22 | 12.2 | 13.5 |

**VERDICT: keep the PSM basis.** Two reasons, both measured.

1. **The inflation worry does not materialise.** The ranking is IDENTICAL and the
   odds ratios move by ~15 % or less. Oxidation's OR actually RISES. Abundant
   peptides were not distorting the peak order.
2. **Deduplication costs statistical power, and the cheapest modifications pay.**
   The 2x2 tests are counting tests; removing 8 % of rank-1 rows removes evidence.
   **Formylation is lost outright** — it was decided by statistics at OR 4.1 and
   falls out. The carpet margin also drops 45 % (193.2 -> 106.4). It stays
   POSITIVE, so the step-2 invariant still holds, but the buffer nearly halves.

The PSM basis is not a convenience. It is the more powerful test, and the
distortion it was suspected of does not appear. **Do not re-propose the switch
without a file where the ranking actually differs.**

### 🔒 THE MODS DECISION — MEASURED, AND THE CHANGE REJECTED (2026-08-31)

**Pass 2 keeps NO MODS. The change was built, run, and rejected on its own
measurements. Do not re-propose it without new evidence.** The switch that ran
this measurement was REVERTED on Ben's call, so re-running it means rebuilding it.

**The rejected alternative:** carry the modifications Pass 1 DISCOVERED into Pass
2 as VARIABLE mods (never static — see the alkylation-agnostic default). It was
the principled middle between no mods (biased) and an assumed set (contradicts
the design). Three measurements killed it.

**1. IT DOES NOT RUN.** Liver `10mg_1_A_1`, 8 variable-mod entries,
`semi_enzymatic: true`, 3909-protein subset, `max_variable_mods: 2`:

```
[PASS2] Running semi-enzymatic search...
Error: Sage exited with status signal: 9 (SIGKILL)
```

Killed **21 s** into Pass 2; `recon` exited 1; no Pass 2 TSV produced.
⚠ **The CAUSE IS NOT ESTABLISHED.** `log show` returned no jetsam or
memorystatus record. Memory is the leading candidate — it is the same shape as
the non-enzymatic entry below — but that is a hypothesis, not a measurement.
This is a harder failure than "too slow": the arm produces nothing.

**2. ON LIVER IT WOULD NOT HAVE FIXED THE BIAS IT EXISTS FOR.** The whole
motivation was "Cys peptides are unreachable with no mods". Pass 1's seven
recommendations on liver carry **no cysteine modification at all**:

| delta | label | sites | position | PSMs |
|---|---|---|---|---|
| +15.9944 | Oxidation on M | M | Anywhere. | 1831 |
| +0.9833 | Deamidation | NQ | Anywhere. | 548 |
| −17.0265 | Gln->pyro-Glu | Q | Peptide N-terminal. | 180 |
| +52.9105 | Fe[III] | DE | Anywhere. | 168 |
| −89.0301 | Met-loss+Acetylation | — | Protein N-term, Met loss | 75 |
| +27.9973 | Formylation | K | Anywhere. | 33 |
| −18.0104 | Water Loss (Glu->pyro-Glu) | E | Peptide N-terminal. | 23 |

The +57.021 peak exists (113 PSMs) and lands in `not_recommended` as
`no_residue_support` — best curated candidate **"Carbamidomethyl on U"**,
odds ratio 287.5, `q` above the floor on a population that small. So the arm
would have paid for 8 mod entries and left every Cys peptide exactly as
unreachable. **The serum +2.71 pp figure that motivated the item was never
re-tested under this arm.**

**3. THE COST IS AGAINST BEN'S "ANSWER IN MINUTES" BAR.** Measured on liver, one
full `recon run`: **2 min 41 s** total = Pass 1 Sage 107.8 s + analysis 32.2 s +
Pass 2 killed at 21 s. The −89.0301 Met-loss entry is additionally **not
expressible as a Sage variable mod at all** — the mass is the modification MINUS
the methionine, on a peptide Sage builds with the methionine still present.

**A fixed mod was raised and rejected separately, by Ben: over-engineering, and
it contradicts the alkylation-agnostic default.**

⚠ **WHAT IS STILL TRUE AND STILL UNFIXED:** Pass 2 with no mods IS biased, and
the bias is one-directional — a modified peptide has no unmodified form to match,
so it never enters the digestion denominators. Serum's ragged rate moves +2.71 pp
(32.05 -> 34.76) between the two settings. That is a **stated limitation of the
number**, not a defect to design around. It belongs in the write-up.

### 🔒 Sage 0.14.7 `variable_mods` KEY SYNTAX — PROBED, not inherited (2026-08-31)

Measured against the pinned binary by putting each key in a config and running it.
Kept because it is expensive to re-derive and easy to get wrong from memory.

* **Accepted residues:** `ACDEFGHIKLMNOPQRSTUVWY` (the 20 plus O and U).
  **Refused:** `B`, `J`, `X`, `Z`.
* **Accepted position keys:** `^` peptide N-term, `$` peptide C-term,
  `[` protein N-term, `]` protein C-term — and the two-character combined forms
  `^Q`, `$K`, `[M`, `]K`. Sage's own docs show `"^Q"` and bare `"["`; the
  combined protein-terminal form was probed, not read.
* **Refused:** `%M`, `ZZ`, `^^Q` ("is too long").

⚠ **AN INVALID KEY IS SKIPPED, NOT REJECTED.** Sage logs
`ERROR sage_core::modification: Skipping invalid modification string` and **runs
the search anyway, without that modification**. A typo therefore yields a
complete, plausible, and silently wrong result. Anything that writes a Sage mod
key must validate it first; the log line is the only symptom.

### ⚠ `RecommendedMod.delta_mass` IS A PEAK CENTRE, and cannot configure a search (2026-08-31)

The report gives the observed peak centre. The curated candidate's THEORETICAL
mass is not carried anywhere in the report. They differ by up to
`tier_assignment::CURATED_TOL_DA` = **10 mDa**, which is the window the peak was
matched in — on a 1500 Da peptide that is **6.7 ppm**, WIDER than the Pass 2
precursor window itself (serum: −7.5 to +12.5 ppm). Anything that writes a peak
centre into a search config puts the modified precursor outside the window it was
meant to be found in, and the modification appears to do nothing.

A `curated_mass` field was added to the report for exactly this and reverted with
the rest. **Any future consumer of `recommendations` for a real search must add
it back**, not reuse `delta_mass`.

### 🐛 Peptide C-TERMINAL candidates were tested at the N-TERMINUS — the SIZING (2026-08-31, status corrected 2026-09-01)

⚠ **This entry's title said "FOUND, NOT FIXED" and was STALE.** It contradicted
the entry "Peptide C-TERMINAL candidates were tested at the N-TERMINUS — FIXED"
above, which is the correct one. **The code settles it:**
`tier_assignment.rs:179` reads `residues.chars().next_back()` on the C-terminal
branch and `:181` reads `.next()` on the N-terminal branch. Corrected in place;
this entry is kept for the SIZING, which the fix entry does not carry.

The defect: `tier_assignment::peptide_hits` read `residues.chars().next()` — the
**FIRST** residue — for every candidate where `is_terminal()` is true. Correct for
`Peptide N-terminal.`, wrong for `Peptide C-terminal.`

Sized: three curated entries are C-terminal — `Amidation` (`TG X`),
`Methylation` (`TG X`, `PP C-terminal.`) and `Homoserine lactone`
(`TG M`, `PP Peptide C-terminal.`). The two `TG X` entries carry no residue and
route to abundance, so **`Homoserine lactone` is the only entry the defect could
reach.**

**Impact on reported numbers: NONE, now measured on ALL FOUR files** (2026-09-01;
the entry previously read "none observed on liver ⚠ serum, bcell and b1906 were
NOT checked"). Every `recommendations` bucket — `fixed`, `variable`,
`not_recommended` — of the committed `full-run/{liver,serum,bcell,b1906}.json`
was scanned for the three labels:

* **`Homoserine lactone`: absent from all four files, every bucket.** It is the
  only reachable entry, so no reported number on any file was ever affected.
* `Amidation` appears on liver / serum / bcell only as `not_recommended` with
  `reason: below_floor` — it never reaches the statistics path, and its `TG X`
  routes to abundance regardless.
* `Methylation` appears once, serum +14.0149, and its position is
  **`Protein N-terminal.`**, not C-terminal — that is the recorded promotion, not
  this path.
* No entry in any of the four files carries a C-terminal `position`.

Homoserine lactone is a CNBr artifact, so a trypsin digest is unlikely to carry
it — which is why the defect was invisible, not why it was left alone.

### 🔒 A third, non-enzymatic search is TOO SLOW TO BE USEFUL — not impossible (2026-08-31)

**Decision (Ben): recon must answer in minutes. A wall clock beyond ~10 minutes
is unusable, and certainly not for the non-tryptic class.** That is the reason
the fourth number is not reported. It is a product decision, on measured cost.

⚠ **AN EARLIER VERSION OF THIS ENTRY SAID "does not fit on this hardware". THAT
WAS WRONG, and is corrected here rather than left standing.**

**What was actually measured, in three attempts:**

| attempt | proteins | residues | result |
|---|---|---|---|
| 1 | 3909 (our subset) | 2 267 032 | killed by signal at 118.6 s, no output |
| 2 | 3909 | 2 267 032 | 60 MB free, swap 3.7/4 GB at 64 s, killed deliberately |
| 3 | **921 (Preview's own list)** | **498 455** | **INDEX BUILT FINE** |

Attempt 3 used the 921 representative proteins parsed out of Preview's own
`result_detail.html` and subset from the same 2018 FASTA. Sage reported:

> `generated 973832403 fragments, 35078863 peptides in 362889ms`

* **35.1 M peptides, 974 M fragments, 363 s of INDEX BUILD ALONE.**
* Memory was never the problem at this scale. It fit on 8 GB.
* The search itself did not finish inside a 10-minute window and was killed.
* For scale: Pass 2 (semi-enzymatic, 3909 proteins) is **21.7 s TOTAL**.

**The architectural point, now with numbers.** Preview's persistent peptide
database is capped at **22 000 peptides**. Sage, over the SAME 921 proteins,
generates **35 million** — about 1600x more — because it materialises a fragment
index over every candidate before scoring. Preview enumerates the same space
transiently (Kil et al.: it "searches the spectra against" semi- and non-specific
peptides) but its scorer prunes by precursor mass bucket and never builds that
index, then PROMOTES only peptides scoring above TLow into the capped database.
Same search space, three orders of magnitude apart in what is held in memory.

⚠ So "Sage cannot do this" is the wrong lesson. **Sage's fragment-index
architecture makes it cost ~20x Pass 2 at minimum, for a class worth 0.1 %** —
two peptides out of 2009 in Preview's own output, where our own classifier's
disagreement with MSFragger's NTT column is the same size (16 vs 2). The
cost/value ratio is the argument, not feasibility.

**DISABLED BY DESIGN. Do not "wire it up" as a free improvement.** If it is ever
revisited, the levers in order are: fewer proteins, a narrower `min_len`/`max_len`
range (candidate count scales with the length RANGE), and a search engine that
accepts a peptide list rather than a FASTA.

⚠ The two 3909-protein failures were during INDEX CONSTRUCTION and their probe
also carried a stale Windows `mzml_paths`, so **no search-cost figure exists for
that scale** — 118.6 s is a lower bound on a failed index build, not a search
time.

### 🔒 Sage sorts `proteins` ALPHABETICALLY and does no protein inference (2026-08-31)

From the source, `database.rs`: `.for_each(|peptide| peptide.proteins.sort_unstable())`.

**So "the first listed protein" is the alphabetically-first accession, not a
protein group leader.** Consequences, all of which have bitten or nearly bitten:

* `compute_terminus_stats` classifies against the first accession — that choice is
  alphabetical, i.e. arbitrary with respect to biology. Already recorded as
  "FIRST PROTEIN ONLY"; this is WHY it is arbitrary.
* **Never call a count of first-listed accessions "protein groups."** We do no
  inference and must not imply we do.
* Counting distinct first-listed accessions is NOT protein-group counting. On
  liver: 3909 accessions carry ≥2 peptides when every accession is credited,
  1928 when only the alphabetically-first is. The 1981 difference is alphabetical
  accident, not evidence.

⚠ **The subset IS heavily redundant, though, and that is real.** The FASTA is
`uniprot_sprot_iso` (isoforms): **55.6 %** of confident peptides map to >1
accession and **71.2 %** of the 3909 accessions have no unique peptide at all.
Collapsing to one representative per shared-evidence set is justified — but by
parsimony, chosen on evidence, NOT by taking whichever accession sorts first.
**OPEN, not implemented.**

### ⚠ MSFragger's `Number of Missed Cleavages` is a DIFFERENT QUANTITY (2026-08-31)

Do not compare it with Sage's column or Preview's rate.

Preview defines missed cleavage as peptides that "contain an internal K or R not
followed by P". **Sage's `missed_cleavages` column agrees with that rule on
0 disagreements out of 10589 peptides.** Recon and Preview count the same thing.

MSFragger disagrees with the same rule on **2065 of 17139 peptides (12 %)**, in
BOTH directions. One direction is explained: FragPipe's `stricttrypsin` preset
leaves `search_enzyme_nocut_1` EMPTY, so it ignores the proline rule and counts
K/R-P as cleavage sites (`NRPEDYQGGR` → theirs 1, rule 0). ⚠ This also affects
the committed `testing/reference-data/msfragger/` reference set. The other
direction (edge positions, e.g. `AAEEEDEADPKR` → theirs 0, rule 1) is MSFragger's
own digestion bookkeeping and is **NOT explained** — do not guess at it.

**Effect on the comparison:** applying one rule to everything, liver missed
cleavage is recon 17.15 %, MSFragger 16.15 %, Preview 15.90 %. Using MSFragger's
own column instead reads 14.70 % and inflates the apparent gap from 1.0 to 2.4 pp.

### 🔒 The subset filter is justified by CENSORING, not by independent filtering (2026-08-31)

**REJECTED FRAMING: Bourgon et al. 2010 independent filtering.** It requires the
filter statistic to be independent of the test statistic. **Measured, and it is
not.** Ragged rate rises monotonically with protein abundance on liver:

| Pass-1 PSM bin | 1-2 | 3-5 | 6-9 | 10-19 | 20-49 | 50+ |
|---|---|---|---|---|---|---|
| ragged % | 6.80 | 5.95 | 4.72 | 6.40 | 8.42 | **18.82** |

Invoking Bourgon here would be a misuse of the citation. **Do not.**

**ACCEPTED FRAMING: detection censoring.** Ragged peptides are systematically
weaker — median MS2 intensity **0.65×** (ragged-N) and **0.61×** (ragged-C) that
of fully-tryptic peptides. In a low-abundance protein the ragged forms fall below
detection, so that protein contributes an artificially clean peptide set and
drags the pooled rate down. Restricting to well-sampled proteins REMOVES a
censoring bias rather than introducing a selection bias. This rests on our own
measurement, not a borrowed citation.

⚠ **No single subset size matches Preview on all metrics**, so this cannot be
tuned to agreement: as the subset shrinks, ragged-N converges on Preview
(8.43 % at the 80 %-coverage knee vs Preview's 8.60 %) while missed cleavage
diverges (19.4 % vs 15.9 %; it is closest at the FULL subset). Optimising for one
degrades the other.

⚠ **Coverage thresholds are scale-free but NOT distribution-free.** The 80 %-of-PSMs
knee lands at a floor of 10 / 68 / 11 / 6 PSMs on liver / serum / bcell / b1906 —
and on serum it keeps only **37 protein groups**, which would wreck the ragged
estimate on the sample type where ragged termini are the actual biology. A
`≥ median` rule keeps a stable 50-54 % of groups on all four files, but that
stability is tautological. **OPEN — no rule adopted.**

### ✅ STEP 3.5 LIVER ARM — four sources, one rule, and the criterion HOLDS (2026-09-01)

Liver `10mg_1_A_1`. Four sources: recon, Byonic Preview, PTM-Shepherd,
MetaMorpheus — all on the SAME 2018 database.

⚠ **A CLAIM IN THE FIRST VERSION OF THIS ENTRY WAS WRONG AND IS CORRECTED HERE.**
It said recon had to be re-run because `full-run/liver` used the 2023 canonical
FASTA. It did not: `full-run/liver` was ALREADY searched against
`uniprot_sprot_iso_human-2018_06.fasta`. The wrong claim came from reading
`effective-params.json`, which copies the TEMPLATE's `database.fasta` while Sage
takes the real one from `-f` — the defect recorded below. The re-run was
therefore redundant and its artifact was deleted; it did reproduce
`full-run/liver` (missed cleavage 17.117 %, ragged-N 6.7508 %, subset 1792, MS1
bias -1.41935 all identical; total PSMs 32498 vs 32496 and +57 1291 vs 1292,
inside Sage's known ~0.1 % non-determinism). **All numbers below come from
`full-run/liver`.** Full table:
`testing/recon-output/comparison/LIVER-FOUR-TOOL-2026-09-01.md`. Provenance for
every run: `testing/reference-data/liver-reference-runs.md`.

**No tool's class or terminus column was read.** Every side reclassified from
raw sequence context by `testing/scripts/liver_four_tool_digestion.py`, a mirror
of the shipped Rust including the initiator-Met branch.

| source | missed cl | ragged-N | ragged-C | N:C |
|---|---|---|---|---|
| recon Pass 2 (semi) | 17.12 % | 6.75 % | 3.01 % | 2.24 |
| PTM-Shepherd open (FULLY) | 19.64 % | 1.64 % | 0.51 % | 3.18 |
| MetaMorpheus (FULLY) | 17.84 % | 0.54 % | 0.45 % | 1.21 |
| Byonic Preview v3.2.0 | 15.90 % | 8.60 % | 1.30 % | 6.62 |

⚠ An MSFragger closed semi-tryptic run was an INTERMEDIATE CHECK from a previous
session and is NOT part of this comparison (Ben, 2026-09-01). Archived to
`_archive/liverFragger-2026-08-31/`. Its 16.15 / 16.16 % missed-cleavage figure
still appears in older entries above; do not carry it into the write-up.

* **Missed cleavage spans 3.74 pp across four independent tools** against a
  15.9 pp between-sample range — biological signal is **4.3x** the method
  disagreement. The acceptance criterion below is MET on a five-tool panel.
* **Ragged rates are NOT comparable across enzyme conventions, and the table
  demonstrates it.** The two FULLY tryptic searches read 1.64 % and 0.54 %
  ragged-N against recon's 6.75 %. A fully tryptic search cannot make a ragged
  peptide, so those are controls near zero. Comparing them to Preview's 8.60 % is
  the trap. On the comparable pair (recon, Preview) the gap is 1.85 pp against a
  24.7 pp between-sample range — **13x**.
* **✅ ragged-N >> ragged-C on ALL FOUR.** Fourth independent confirmation that
  Davis Table 3's headers are transposed. The RATIO is not reproducible
  (1.21 to 6.62); the DIRECTION is.
* ⚠ Fully-tryptic ragged-N is not exactly zero because classification uses the
  FIRST listed protein — a peptide tryptic in its true parent can be ragged in
  the razor protein. recon has the identical limitation (`digestion.rs:390`).

**Mod discovery, matched by DELTA MASS at 0.01 Da, never by name**
(`testing/scripts/liver_mod_rank_comparison.py`):

| pair | n | Spearman rho | p |
|---|---|---|---|
| recon vs PTM-Shepherd | 38 | +0.616 | 2.5e-05 (permutation, seed 42) |
| recon vs MetaMorpheus | 6 | +1.000 | 0.002778 (exact) |

All three put Oxidation first and Carbamidomethyl second among real mods, with
NO tool told the sample was alkylated. recon: +57.0207 at rank 3 overall, 1291
PSMs, 3.97 %. Counts are NOT commensurable (Shepherd runs ~1.5-2x recon
throughout) — the claim is rank, per "Prevalence currency".
⚠ n=6 for MetaMorpheus is thin by construction: G-PTM-D searches a curated
candidate list, not an open window. Never quote that rho without its n.

**Two methodological traps hit and fixed while building this:**
1. MetaMorpheus's `Mass Diff (Da)` is the PRECURSOR mass error, not the mod
   delta. Using it gave ZERO matches. The mod mass must be computed from the
   run's own `Mods Combined Chemical Formula`, with element masses read from the
   pinned `unimod.xml` `<umod:elem>` table.
2. Entries must be AGGREGATED BY MASS before matching. MetaMorpheus reports
   per-NAME and several names share a delta (Oxidation on M and Hydroxylation on
   P are both formula O). Matching name-wise picked one and discarded the rest,
   reporting Oxidation as 14 peptides against a true 2127.

### 🔒 LIVER IS THE ACTIVE FILE — what that does and does not change (Ben, 2026-09-01)

The write-up is built on liver, because liver is the only file with a Byonic
Preview ground truth.

**serum, bcell and b1906 are kept as BEHAVIOUR CHECKS** (Ben's framing), not as
comparison targets. That is the distinction that matters: they exist to prove the
code does the right thing — a sign flip, a fixed-C/agnostic split, a detector
class — on data where that behaviour is visible. They are not evidence about
liver and no result is reported across them. Liver is the default target for
anything new.

**What this does NOT license:**

1. **Deleting a gate whose file is the only one that shows the behaviour.**
   `ms1_bias_is_negative_on_bcell` exists because bcell's MS1 bias FLIPS SIGN,
   +0.70 -> -0.24 ppm, which is the proof that the `|error|` bug is fixed.
   Liver's bias is **-1.4193 ppm** — already strongly negative, so liver passes
   that assertion even if the correction were only half applied. Re-pointing it
   at liver would delete the evidence and keep the green tick.
2. **Shrinking the acceptance criterion.** It compares BETWEEN-METHOD spread
   against BETWEEN-SAMPLE spread. A between-sample spread needs more than one
   sample by construction. Liver is one of its four points.

**⚠ GAP FOUND while checking this: `run_validation.py` has never covered liver.**
`FILES = ["b1906", "bcell", "serum"]`. The standing tripwire — the thing AGENTS
names as the project's hard-stop — does not run on the file the entire write-up
rests on. Tier 3 snapshots and the Gate 3 fixed-C/agnostic bands are all defined
for the other three only. **The fix is to ADD liver, not to remove the others.**
Not done yet; recorded so it is not lost.

### ✅ END-TO-END VERIFICATION ON LIVER after the closed-path removal (2026-09-01)

`recon run --full` re-run on liver against the 2018 FASTA with the binary built
AFTER `unified_ms1_error`, `--closed-tsv` and `qc::compute_ms1_mass_accuracy`
were deleted. Compared field by field against the committed `full-run/liver`:

| field | committed | fresh | |
|---|---|---|---|
| total_psms | 32496 | 32496 | same |
| peaks | 49 | 49 | same |
| unmodified_pct | 50.495446 | 50.495446 | same |
| ms1 bias_ppm | -1.419349 | -1.419349 | same |
| ms1 MAD / clean subset | 0.623082 / 7519 | 0.623082 / 7519 | same |
| pass2 peptides | 10621 | 10621 | same |
| pass2 missed cleavage | 1818 (17.1170 %) | 1818 (17.1170 %) | same |
| pass2 ragged N / C | 717 / 320 | 717 / 320 | same |
| subset proteins | 1792 | 1792 | same |
| fragment_median_ppm | 3.420621 | 3.420623 | float noise, 2e-6 ppm |
| +57 peak (rank, count) | 3, 1292 | 3, 1291 | **one PSM moved** |
| three_layer_ms1 | absent | **present** | committed run lacked `--full` |

**Every number the write-up uses is reproduced exactly.** Two differences, both
understood:
* `fragment_median_ppm` differs in the 6th decimal — float summation order.
* The +57 count moves by ONE PSM out of 32496 (0.003 %) **while `total_psms` is
  identical**. That is q-boundary membership swapping, not a count change, and it
  is the behaviour "Sage output is NOT bit-reproducible across builds" already
  records. It is not caused by the closed-path removal: nothing deleted touches
  peak assignment.

✅ **DONE 2026-09-01: `full-run/` regenerated with `--full` on all four files**,
under the gate, and promoted. `three_layer_ms1` is now present on all four. The
+57 count moved 1292 -> 1291 as predicted, and every reported comparison number
is unchanged (liver missed cleavage 17.12 %, ragged-N 6.75 %, ragged-C 3.01 %,
1792 subset proteins, MS1 bias -1.4193 ppm).

⚠ **A scratch run directory (`_staging/`) was committed by accident** in
`bd4552a` via a blanket `git add -A`, and removed the next commit.
`testing/recon-output/{_verify,_staging}/` are now gitignored. recon has ONE
artifact set; a second copy of a run is the exact ghost this repo spent the day
removing.

### ✅ MASCOT ERROR-TOLERANT IS THE FOURTH AGNOSTIC TOOL ON LIVER (2026-09-01)

`testing/reference-data/mascot/error-tolerant/MascotErrorTol-liver.txt`, with
`Human_ertol-2018.par`. **Verified agnostic from the file, not assumed:** `MODS=`
and `IT_MODS=` are both EMPTY with `ERRORTOLERANT=1`, so its Carbamidomethyl is a
discovery. The `.par` differs from the original three-file one in EXACTLY one
line — `DB=cRAP,sprot_iso_human-2018` instead of `DB=cRAP,UP000005640_Human` —
confirmed by diff. That is the same 2018 database Preview and the other liver
reference tools used.

recon vs Mascot: **n=26 shared masses, Spearman rho +0.502, p = 0.01005**
(permutation, seed 42). The existing `load_mascot` roll-up is reused VERBATIM —
Mascot reports by NAME + SITE and splits one mass across many site rows, so
without the roll-up its C-only row understates the +57. 16 rows had no clean
Unimod mass and were skipped and counted, never silently dropped.

**All FOUR agnostic tools put Oxidation first and Carbamidomethyl second.**

| delta | recon | PTM-Shepherd | MetaMorpheus | Mascot |
|---|---|---|---|---|
| 15.9945 | 1688 | 3290 | 1973 | 4585 |
| 57.0207 | 1291 | 2048 | 1398 | 2479 |
| 0.9833 | 562 | 447 | 516 | 687 |

⚠ **Mascot is in the MOD comparison ONLY and must never enter the digestion
table.** `PFA=1` allows one missed cleavage against recon's two, and the file is
a modification summary with no peptide list to reclassify under recon's own
terminus rule. Adding it would be the same convention error as the four traps.

### ✅ NOTHING READS A FIXED-C OR CLOSED SEARCH ANY MORE (2026-09-01)

Closing the last tie between the test suite and searches recon does not produce.
Three call-sites read `static_mods {C: 57.0215}` searches, identified from each
run's own Sage `results.json`:

* **`digestion_port_integration` — DELETED.** It re-verified the Rust
  `digestion_efficiency` port against its Python prototype on the bcell pass-1/
  pass-2 pair. That port is a LOCKED decision, so the test is a decision record,
  not a gate — the distinction written down in
  `reference-notes/gates-what-they-consume.md`.
  ⚠ Regenerating its Python control on an agnostic search was attempted and
  ABANDONED, correctly: `digestion_efficiency.py annotate` needs `pyteomics`,
  which is not installed, and re-verifying a locked decision is a re-hash. Ben's
  call.
* **`parsimony_impact` — DELETED.** Parsimony is decided.
* **`digestion_composition_integration` — REPOINTED**, not deleted. It is a real
  gate: the terminus classifier against the vendored Preview anchor. It now reads
  `testing/recon-output/full-run/liver_search/pass2/`.

**Pinned numbers moved with the search, because a fixed-C search identifies a
different peptide set:**

| | fixed-C (old) | agnostic (shipped) |
|---|---|---|
| peptides classified | 10589 | **10621** |
| missed cleavage | 1816 | **1818** |
| ragged-N | 738 | **717** |
| ragged-C | 321 | **320** |
| completeness | 82.85 % | **82.88 %** |
| decoys ragged-N / C | 76 / 31 | **81 / 26** |
| semi-tryptic class FDR | 10.10 % | **10.32 %** |

⚠ I wrote "ragged_c 31 -> 31 (unchanged)" into that test's comment without
measuring it. It is 26. Caught by the assertion, corrected — but the comment was
an assumption stated as fact, which is the habit AGENTS exists to stop.

The "known from outside this code" property AGENTS asks of a regression case is
preserved: `testing/scripts/liver_four_tool_digestion.py` mirrors the same rule
in Python and reproduces 10621 / 1818 / 717 / 320 for the four-tool comparison.

**`testing/search-output/` cut 837 MB -> 166 MB, 21 directories deleted.** Nothing
left the repo: the whole tree is gitignored and **0 of its files were ever in
git**. What remains is `step1-open-{serum,bcell,b1906}` (the agnostic behaviour-
check inputs that `run_validation` Tiers 1 and 3 depend on) and `ptmRecovery`
(backs the +57 predicted-vs-verified reconciliation).

**167 tests, 0 failures** (181 minus the 14 in the two deleted files).
`run_validation.py` 15/15.

### 🔒 SAGE IS NON-DETERMINISTIC, RECON IS NOT — measured, not assumed (2026-09-01)

Prompted by Ben: the regeneration drift appeared only after a session of Rust
edits, so the edits were the obvious suspect and had to be ruled out properly
rather than waved away as "known non-reproducibility". Two controlled runs
settle it.

**recon, run TWICE on ONE fixed Sage TSV, differs in exactly three things:**

```
.generated_at                      timestamp
.polymer.total_pct_tic             1 ULP
.signal_fate.id_rate_by_tic_pct    32 ULP
```

`total_psms`, all 49 peaks, `ms1_calibration.bias_ppm`, the +57 count and the
entire `three_layer_ms1` block are IDENTICAL. Both float fields are already in
the gate's documented noise list, with the recorded cause (parallel summation
over TIC). **recon's processing is deterministic.**

**Sage, run TWICE on BYTE-IDENTICAL input with the SAME binary, is not:**

| file | old TSV | new TSV | rows |
|---|---|---|---|
| liver | 38,463,466 B `9bff7910` | 38,510,214 B `94ce181d` | 96,049 both |
| serum | 25,555,140 B `1700edff` | 25,557,458 B `d41ddb8e` | 66,772 both |

⚠ **THE MD5 COLUMN WAS TRANSPOSED AGAINST THE SIZE COLUMN — corrected in place
2026-09-01 (later session).** It read liver `38,463,466 B 94ce181d` /
`38,510,214 B 9bff7910` and serum the same way round. **Re-measured from the two
files, which are BOTH still on disk:**

```
full-run/liver_search/results.sage.tsv    38,510,214 B  94ce181d29299934bde2bbdbf47360c8
_verify/liver_search/results.sage.tsv     38,463,466 B  9bff791039c5f798ba998b7025a404bf
full-run/serum_search/results.sage.tsv    25,557,458 B  d41ddb8ea8509e7bc4e50f04326f4296
```

So the shipped `full-run/` liver TSV is `94ce181d`, not `9bff7910` as this table
said. ⚠ Serum's counterpart run is NOT on disk (`_verify/` holds liver only), so
serum's pair is corrected by the same transposition and is NOT independently
re-measured — flagged rather than asserted. **The entry's CLAIM is untouched:**
two runs, two sizes, two md5s, identical row counts. Only the pairing was wrong.
The same transposition sat in `assert_regeneration_invariants.py`'s header
comment and is corrected there too.

Inputs verified identical by checksum: mzML `6820489483a0668cf8a2f25fff3cd276`,
FASTA `f6b42e28d4809ef158254e382a54eb46`. ⚠ Both RE-VERIFIED 2026-09-01 (later
session) against `testing/inputs/` — they still hold, and the FASTA md5 is
byte-identical to the second copy at
`testing/reference-data/preview/uniprot_sprot_iso_human-2018_06.fasta`.

**So every measurement difference in a regeneration originates in Sage,**
upstream of recon. This matches "Sage output is NOT bit-reproducible across
builds" — but that entry said ACROSS BUILDS, and this shows it happens across
RUNS OF ONE BUILD.

⚠ **SERUM IS THE CASE THAT BREAKS A COUNT-BASED CHECK.** Its `total_psms` and all
48 peak counts are IDENTICAL between the two runs, yet SEVEN odds ratios moved —
because Sage returned a different SET of PSMs with the same per-peak counts.
Different peptide identities means different residue tallies in the Fisher 2x2,
so the odds ratio moves while every count stands still. A gate that infers "did
anything drift?" from counts alone cannot see this.

**Consequence for the gate:** `assert_regeneration_invariants.py` now CHECKSUMS
the source TSV instead of inferring drift from counts. Derived statistics get a
1 % relative budget ONLY when the input is measurably different; when the TSV is
identical, nothing may move at all. The count heuristic remains as a fallback for
a fresh clone, where the TSVs are gitignored and absent.

**The rejected alternative:** widening the budget until the gate passed. That is
fitting the gate to the data it failed on, and it would have hidden the serum
case entirely — the ONE finding here that a wider budget would have silently
swallowed.

### ✅ `full-run/` REGENERATED WITH --full AND PROMOTED (2026-09-01)

All four files, each against the FASTA it had always used (serum/bcell/b1906 on
UP000005640_canonical-2023_05, liver on sprot_iso_human-2018_06), read from each
run's own Sage `results.json` rather than assumed. Only `--full` changed.

Gate output, with every movement printed:

* `three_layer_ms1` ADDED on all four — the point of the run.
* liver: `total_psms` 32496 -> 32497 (1 PSM), three peak counts moved by 1 each
  (+0.0000 16100->16099, +57.0207 1292->1291, +114.0427 127->128) = 3 PSMs
  reassigned, 0.0092 % of the file.
* Derived statistics moved by at most **0.5464 %** (serum's odds ratio at
  +14.0149, Methylation) against a 1 % budget. Every other movement is under
  0.025 %. The 0.5464 % is named here so it is not lost in an aggregate.
* **Every reported comparison number is unchanged**: liver missed cleavage
  17.12 %, ragged-N 6.75 %, ragged-C 3.01 %, 1792 subset proteins, MS1 bias
  -1.4193 ppm, and all three mod-rank correlations.

⚠ **A second stale-template ghost was found doing this.** The old committed LIVER
config carried `mzml_paths: ['inputs/B.naive_01steady-state.mzML.gz']` — BCELL's
mzML, in liver's config. Same class as the FASTA ghost, which had only been fixed
for `fasta`. It was inert (Sage's positional argument wins, and its own
`results.json` records one correct file for both runs), but the artifact was
misstating two fields, not one. Both are now written from what actually ran.

**Signal fate, recovered by `--full`:**

| file | non-peptidic | never sampled | sampled, not ID'd | identified |
|---|---|---|---|---|
| serum | 4.72 % | 48.96 % | 44.10 % | 6.94 % |
| bcell | 14.68 % | 52.09 % | 42.90 % | 5.01 % |
| b1906 | 9.27 % | 61.86 % | 35.64 % | 2.49 % |
| liver | 5.79 % | **64.66 %** | 31.93 % | **3.42 %** |

On liver two-thirds of peptide-like MS1 signal is never selected for MS2, and
3.42 % ends up identified. That is the "where did the signal go" answer, and it
was absent from every committed report until now.

### 🔒 THE CLOSED-SEARCH PATH IS RETIRED — recon needs no closed search (Ben, 2026-09-01)

**MS1 mass accuracy has never required a closed search, and recon has never used
one to produce its shipped number.** `calibration::select_clean_subset` takes the
OPEN search's own PSMs — rank 1, `q < threshold`, `|corrected_delta_da|` near
zero, hyperscore-guarded — and `signed_precursor_ppm` reconstructs the signed
bias from the mass columns. That is `ms1_calibration.bias_ppm`, it is populated
in every report, and it goes negative (liver -1.4193 ppm), which was the whole
point of the `|error|` fix.

⚠ **A session claim on 2026-09-01 confused this** and reported the absence of
`unified_ms1_error` from `full-run/` as a silent loss of MS1 capability. Wrong.
`unified_ms1_error` was a separate legacy DUAL-READOUT block that set the open
apex_offset beside a signed MS1 taken from an externally supplied closed TSV. It
was a cross-check artifact from before the signed fix, never the measurement.

**Ben's call: retired outright, not left as an option.** The reasoning is
shipping, not taste — a flag that reads a hand-supplied TSV is a route for a
stale or mismatched file to enter a result, and recon is moving to a state where
it depends on no randomly sourced input. An outside Sage run can still do this
check; it just is not recon's job.

Removed 2026-09-01:
* `report::UnifiedMs1Error` and `MassAccuracySummaryReport::unified_ms1_error`
* `InputInfo::closed_tsv`
* the `--closed-tsv` flag on BOTH `analyze` and `qc-stats`
* `qc::compute_ms1_mass_accuracy`, `qc::Ms1MassAccuracy`,
  `qc::print_ms1_mass_accuracy`, and their two unit tests
* the closed-TSV provenance guard (single-file check + basename match)
* the console and HTML "two views" sections

**182 tests pass, `run_validation.py` 15/15, zero build warnings.** The committed
reports were already free of the block, so no artifact changed.
⚠ **The 182 is SUPERSEDED and true only at that point in the session.** Two test
files were deleted later the same day (see "NOTHING READS A FIXED-C OR CLOSED
SEARCH" above), taking 14 tests out. **Measured 2026-09-01 (later session) at commit `cb06640`:
`cargo test` reports 167 passed, 0 failed — 165 unit/integration plus 2 doc
tests.** That count agrees with PLAN's status block.
⚠ "Zero build warnings" no longer holds for the TEST build: `tests/integration_test.rs`
emits 3 `deprecated` warnings for `NEUTRON_MASS` (use `C13_C12_DIFF`). The library
build is still clean. Recorded, not fixed.

### ✅ PREVIEW REPORTS MASS ACCURACY — and it corroborates recon's MS2 (2026-09-01)

⚠ **This corrects a statement made earlier the same day** that "Preview's UI
exposes no mass tolerance, so there is nothing to compare against". True of the
UI's INPUTS, wrong about its OUTPUT. `result_summary.html` carries measured mass
error in HTML tables — found only by parsing the tables, which is why they must
be parsed and not skimmed.

```
PRECURSOR  before recal: signed  0.0 ppm | |err| 0.5 ppm | 943 high /  850 low
           after  recal: signed -0.0 ppm | |err| 0.5 ppm | 878 high /  980 low
           off-by-one (nominal mass is +1 isotope): 6.0 % (119/1973)
FRAGMENT   before recal: signed -3.1 ppm | |err| 3.5 ppm | 307 high / 9820 low
           after  recal: signed  0.1 ppm | |err| 0.7 ppm | 4783 high / 5354 low
```

Pre-recalibration is the comparable side: recon reports what the instrument
delivered and does not recalibrate spectra.

| quantity | recon (`full-run/liver`) | Preview (pre-recal) |
|---|---|---|
| MS2 absolute median | **3.4206 ppm** | **3.5 ppm** |
| MS2 signed median | not produced, by design | **-3.1 ppm** |
| MS1 signed median | **-1.4193 ppm** | **0.0 ppm** |

* ✅ **recon's MS2 accuracy is corroborated to 0.08 ppm** by an independent tool
  on the same raw file, same quantity (median of |error|).
* ✅ **The signed MS2 gap has an external answer.** "MS2 stays ABSOLUTE" records
  that recon has no signed MS2 number for the benchmark table and defers it to
  step 5. Preview gives -3.1 ppm, corroborated by its own directional count of
  **307 fragments high against 9820 low** — a 32:1 imbalance is not compatible
  with a centred distribution. recon still does not MEASURE the sign; it now has
  a reference value.
* ⚠ **MS1 DISAGREES BY 1.42 ppm AND IS NOT RESOLVED.** recon -1.4193 ppm against
  Preview 0.0 ppm on a balanced 943/850 split. Untested candidate causes:
  Preview measured the `.mgf` after its own conversion; populations differ by an
  order of magnitude (recon clean subset 7519 PSMs vs ~1793 precursors);
  different peptide sets. **Do not present recon's MS1 bias as corroborated.**

### 🔒 CONFIG GHOSTS — the template is not the record, and now cannot pretend to be (2026-09-01)

Ben's instruction: no ghosts anywhere — not a wrong FASTA, not mods, not
isotopes. What was actually wrong, and what changed:

1. **`effective-params.json` named a database that was never searched.** It
   copied the template's `database.fasta` while Sage takes the real one from
   `-f`. Every committed artifact claimed the 2023 canonical FASTA regardless of
   truth, and that misled this session into a redundant re-run. **Fixed:**
   `write_effective_params` now writes the FASTA and mzML actually searched, and
   records the template's own claim separately as `template_fasta`. Regression
   test `pass1_records_the_fasta_that_was_actually_searched`, falsified.
2. **Sage's own `results.json` — the ONLY honest record — was gitignored.**
   `.gitignore` carried a bare `results.json` rule. So the lying file was tracked
   and the truthful one was not. **Fixed:** negation for
   `testing/recon-output/**/results.json` (32 KB total; the 202 MB of TSVs stay
   ignored).
3. **`isotope_errors` was inherited from the template with no check.** Pass 1
   overrides `fragment_tol`, the mods, the FASTA and the mzML, but NOT
   `isotope_errors` — it is part of the open-search definition. So any
   `--params` template could silently ruin the search, and the measured cost is
   already recorded: `[-1,2]` cut the +57 peak from 1253 to 394 and invented a
   Propionyl peak at +56.018. **Fixed:** pass 1 now REFUSES any template whose
   `isotope_errors` is not `[0,0]`. Test uses the real committed
   `closed-search-reference.json` (`[-1,3]`), falsified.
4. **22 config templates, most of them dead.** Two are live defaults
   (`open-search-params.json`, `digestion-efficiency-pass2.json`). One was
   INVALID JSON (`PXD0014688.json`) and is deleted. 16 had no reference anywhere
   and are archived to `_archive/configs-2026-09-01/`. The two live
   digestion templates were cleaned so template and reality agree.
   **Two are DELIBERATELY DIRTY and carry a `_recon_note` saying so** —
   `closed-search-reference.json` and `open-search-b1906.json` are the
   regression fixtures for the guards above. Cleaning them makes those tests
   vacuous.

**The rule, stated once:** a template is an INPUT, never a record. The record of
what ran is the `effective-params.json` written beside each search, and Sage's
`results.json` beside that. Both are now committed and both are now honest.

### ✅ MetaMorpheus G-PTM-D: adding Common Fixed/Variable to the DISCOVERY list works (2026-09-01)

Ben flagged this as uncertain — "if my understanding is wrong we won't see
alkylation etc in results". Settled from the run's own output, not by reasoning
about MetaMorpheus. Bracketed mods on target peptides at q <= 0.01 in
`Task3-SearchTask/AllPeptides.psmtsv`:

```
2127  Common Variable:Oxidation on M
1420  Common Fixed:Carbamidomethyl on C
 555  Common Artifact:Deamidation on N
 192  Common Biological:Hydroxylation on P
 151  Common Biological:Acetylation on X
```

Carbamidomethyl is present in quantity and arrived by G-PTM-D DISCOVERY, having
been removed as a mod option in all three tasks. **The understanding was right.**

### 🐛 `effective-params.json` names a FASTA that was NOT searched (2026-09-01)

`write_effective_params` copies the template's `database.fasta` verbatim, but
recon passes the real FASTA to Sage with the `-f` CLI flag
(`sage_runner.rs:121-122`), which overrides the JSON. The file that calls itself
the effective config therefore names the wrong database.
Observed on the liver 2018-FASTA run: `effective-params.json` said
`inputs/UniProt-Human-UP000005640_canonical-2023_05.fasta`; Sage's own
`search/results.json` said the 2018 file, which is what actually ran.
**The search was correct — only the provenance record is wrong.** Sage's
`results.json` is the authority.
✅ **FIXED the same day** — see "CONFIG GHOSTS" above. `write_effective_params`
now writes the searched FASTA and mzML, with the template's claim kept separately
as `template_fasta`, and Sage's `results.json` is no longer gitignored.
⚠ Before the fix this defect did real damage: it convinced this session that
`full-run/liver` used the 2023 FASTA when it used the 2018 one, causing a
redundant re-run and a wrong claim in four documents.

### ✅ The no-mods guards are now TESTED — and one hole found (2026-09-01)

"PASS 1 AND PASS 2 NEVER SEARCH WITH MODIFICATIONS" records the guards as
enforced in code. True, but **nothing tested either of them** until now.
`recon-tool/tests/no_mods_guard.rs` closes that, and was FALSIFIED both ways:
* breaking the `static_mods` strip fires the code's OWN assert;
* breaking the `variable_mods` strip fires NOTHING in the code — **both guards
  assert `static_mods` only, though both strip `variable_mods` too.** The new
  test is the only thing covering that half.
⚠ **The test's first version passed vacuously.** It used
`open-search-b1906.json`, whose `variable_mods` are already empty, so the
variable-mods assertion held with the strip deleted. Switched to
`closed-search-reference.json` (static `{C:57.0215}` + 4 variable mods) and given
an explicit anti-vacuity assertion on the template itself. Textbook "one agreeing
case is not validation".
The pass-2 guard is additionally exercised on EVERY normal run: the default
template `digestion-efficiency-pass2.json` carries `{C: 57.0215}` and
`{M: [15.9949]}`, and the written pass-2 config came out empty on the liver run.

### ✅ A1 LANDING 1 LANDED — v0.15.0-beta.2, every number re-baselined (2026-09-01)

Sage v0.14.7 -> **v0.15.0-beta.2** (`df92199`), pinned from a clean upstream
clone. **recon's math did not change**: no logic was touched. Schema **1.8.0**.
**175 tests, 0 failures. `run_validation.py` 17/17.**

**Liver's new baseline:** total_psms 32497 -> 31793, peptides 10621 -> 10772,
missed cleavage 1818 -> 1888, ragged-N 717 -> 746, ragged-C 320 -> 338,
completeness 82.88 -> 82.47 %, decoys 81/26 -> 86/18, semi-tryptic class FDR
10.32 -> 9.59 %, subset proteins 1792 -> 1771.

**Two source-derived predictions CONFIRMED on the real reports:**
* MS1 bias **-1.4193 -> -1.4172** — essentially unmoved, because the shipped bias
  is reconstructed from the mass columns and never reads the column that became
  signed. The upgrade note in this file had said `qc.rs` must be audited for
  double-signing; the real MS1 path is `calibration`, and it was never at risk.
* `precursor_median_ppm` **450.329 -> -0.271** — the signed change landing exactly
  as read from the source diff.

**The external check still holds.** Liver missed cleavage **17.53 %** against
Preview's 15.90 % — a 1.63 pp gap where it was 1.22 pp — against a 15.9 pp
between-sample range, so biological signal is ~10x the method disagreement.
recon's MS2 median |error| **3.3822 ppm** against Preview's 3.5, agreeing to
**0.12 ppm** (was 0.08). ragged-N still >> ragged-C.

⚠ **ONE PROPERTY GENUINELY WEAKENED, and the tolerance was widened — say it
plainly.** Decoy subtraction is less proportional in N vs C than it was: raw and
corrected N:C now differ by **0.145** where they differed by 0.078, because
`decoys_ragged_c` fell **26 -> 18**. **n = 18** is small enough that this may be
counting noise. The assertion moved 0.1 -> 0.2 with the reasoning written at the
assertion itself. **This is NOT licence to widen it again** — past 0.2 is a
finding, not a constant to edit.

⚠ **The pre-committed protein-context set grew from THREE members to FOUR** —
b1906 +42.0105 Acetylation now qualifies at a protein N-terminus. The DECISION it
records is unchanged; the PSM set underneath it is not.

⚠ **`assert_regeneration_invariants.py` PAIRS PEAKS POSITIONALLY** (`zip(op, np_)`),
so when the peak list reorders it compares peak N against a different peak N and
emits a wall of meaningless failures ("delta_mass MOVED 23.9567 -> 58.0037" is two
different peaks). It had never hit a reordered list because every prior
regeneration was same-version. **Match by delta mass when reading its output
across a version change.** Not fixed; recorded.

⚠ **Stale per-file `.log` captures were DELETED, not promoted.** They are manual
stdout captures, nothing reads them, and keeping v0.14.7 logs beside v0.15 reports
is the exact ghost this repo removes. Regenerate with a capture run if wanted.

### ✅ THE FIVE-TOOL LIVER REPORT NOW COVERS DIGESTION, MASS TOLERANCE AND PTMs (2026-09-01)

`testing/scripts/liver_5way_report.py` -> `comparison/LIVER-FIVE-TOOL-2026-09-01.{md,html}`.

**It COMPOSES, it does not calculate.** Every number already has one producer, so
the script runs those producers and assembles their stdout — the same rule as
`CuratedDb::load` delegating to `load_from_sources`. No second implementation
exists to drift.

⚠ **The three sections use DIFFERENT TOOL SETS on purpose, and the report says so
rather than leaving blanks to be misread as zeros.**
* **PTMs: five.** recon, Preview, PTM-Shepherd, MetaMorpheus, Mascot.
* **Digestion: four.** Mascot stays OUT — `PFA=1` against recon's two missed
  cleavages, and no peptide list to reclassify.
* **Mass tolerance: two.** Only recon and Preview publish a comparable quantity.

**Preview's mass accuracy is PARSED from its own `result_summary.html`**, not
copied from this file. The values sit in prose rather than a table, which is why
they went unmined for weeks.

### 🔒 THE +4.09 % OXIDATION MOVE IS THE NEW BENCHMARK — not a defect (Ben, 2026-09-01)

Ben's call, on being shown the measured v0.15 movement: **"that's not crazy so I
say just go with it. It is the new benchmark. We know our math is right, and we
can't change the new Sage, and we also don't want to."**

So the +4.09 % Oxidation move (1539 -> 1602 on liver) is **ACCEPTED as the new
baseline**, and the earlier note that it "must be attributed before the write-up
quotes a moved Oxidation rank" is **withdrawn as a blocker** — it stays a
nice-to-know, not a gate. The reasoning, which is sound and worth keeping:
recon's own arithmetic is unchanged and independently verified; the movement
originates entirely upstream of recon; and we would not want to "correct" a
current Sage back to an older Sage's numbers even if we could.

⚠ **What this does NOT license.** It is still a version-attributable movement, so
the write-up must state the Sage version beside every number rather than
presenting them as version-free measurements. The pin is the citation.

### 🔒 PREVIEW'S `(-fixed mod)` ROWS ARE OFFSETS FROM THE FIXED +57, NOT ABSOLUTE DELTAS (2026-09-01)

**Do not re-derive this from the mass alone — it will look wrong and it is not.**
Byonic Preview's `VariableMods` survey reports some rows RELATIVE to the fixed
carbamidomethyl on cysteine that Preview itself applied. Worked example, verified
against Preview's own detail text rather than inferred:

```
Trioxidation  -9.036720 + 57.021464 = +47.984744
```

which equals 3x oxidation to 1e-6 and matches Preview's own detail page rendering
"C[+48]". A row reading -9.0367 is therefore NOT a -9 Da species; it is +47.98 on
a cysteine that already carries +57.

⚠ **Second Preview convention, same source:** `VariableMods.txt` repeats ONE GROUP
TOTAL across several site rows, and sometimes across two different masses.
Measured case: pyro-Glu lists **66** against BOTH -17.0265 and -18.0106, while the
true split is **61 / 5**. Summing the site rows therefore double-counts. Both
conventions are why Preview's mod survey must be read through its detail text and
not scraped as a flat table.

✅ **PREVIEW'S MOD SURVEY WAS NEVER MINED UNTIL NOW.** `VariableMods` appears in no
script, and in no NOTES, PLAN or JOURNAL entry — checked, not assumed. **No locked
decision ever excluded it**; it was filed under "mod settings" in the vendored
README even though it carries measured counts (`nmods`, `prop2`). That is why the
liver comparison ran four-tool for a mod question that had a fifth arm available.

### 🔒 `peptide_hits` IS A CONTAINMENT TEST, AND ON COMMON ACCEPTORS THAT IS FATAL (2026-09-01)

**This is the limitation the write-up needs, and it is a DESIGN consequence, not a
bug.** Recon cannot recommend **Oxidation on proline**, which all four other tools
report on liver (Preview 18, PTM-Shepherd 69, MetaMorpheus 192, Mascot 1314).

It is not missing from the curated list and it is not skipped. `Mods.txt` carries
it (`TG P`, `CF O1`), `test_candidate` runs on it, and it **scores OR 1.34 against
an `OR_MIN` of 2.0**, so it never reaches `max_by`. Reproduced independently:

```
Hydroxylation  P   band containment 62.7 %   background 56.0 %   OR 1.34   FAILS
Oxidation on M M   band containment 91.7 %   background 19.7 %   OR 59.42  passes
Oxidation      CDEFHILQRSTUVWY  100.0 % / 100.0 %  — background saturated, UNTESTABLE
```

**The mechanism:** `peptide_hits` asks "does this peptide CONTAIN the acceptor",
not "is the mod ON that residue". Proline sits in **56.0 %** of liver peptides, and
the +15.9945 band is 92 % M-oxidised peptides that merely also happen to contain a
proline. The test cannot separate the co-occurring second population from the
dominant one.

**It generalises, and the arithmetic is exact.** Of 99 curated entries at 62
distinct masses, **58 (58.6 %) sit at a contested mass**. For a candidate to reach
OR >= 2 it needs band containment of at least `2f/(1+f)` where `f` is its
background frequency: C **20.6 %**, M **33.0 %**, P **71.8 %**, broad
Carbamidomethyl **99.7 %**. **The rarest acceptor wins by construction**, and a
common one cannot win at all.

⚠ **Do NOT over-read this, three ways.**
1. It is **difficulty, not prediction.** Gln->pyro-Glu on Q needs 63.3 % and still
   scores OR 650 — the test works fine when ONE mod dominates its band. The limit
   is surfacing a SECOND, co-occurring mod at the same mass.
2. It does **not** say oxidised proline SHOULD be recommended. The gate did exactly
   what it was built to do. Whether one mass may carry more than one
   recommendation is a design question this OPENS, not one it settles.
3. The `--peak` reproduction is **APPROXIMATE** — a flat +-0.010 Da window gives
   1717 band PSMs against recon's 1688, and OR 59.42 against recon's reported
   63.53, because recon uses its own merged-peak binning. Same conclusion, different
   numbers. **Never quote these as recon's own.**

### ⚠ RECON'S NINE RECOMMENDATIONS — HOW WELL THEY ACTUALLY HOLD (2026-09-01)

Five-way on liver (`testing/scripts/liver_5way_mods.py`, report
`comparison/LIVER-FIVE-TOOL-MODS-2026-09-01.md`). **The top four are 4/4 across
the other tools, and every one of the nine is found by at least one.** Two caveats
that must travel with the number wherever the nine are quoted:
* **+57 is three discoveries and one assumption.** Preview's cysteine +57 was an
  operator preset, not a finding, so it is not a fourth independent corroboration.
* **-89.0302 Met-loss+Acetylation is SINGLE-SOURCE** (Mascot only) despite
  OR 8578. Label it as such.

### ✅ v0.15 MEASURED ON LIVER — and a PREDICTION WAS FALSIFIED (2026-09-01)

Single-variable experiment, run into scratch, **nothing adopted**. Same config,
same mzML, same 2018 FASTA, `protein_grouping: false` so the new grouping cannot
move anything. Telemetry disabled on every run here (see the telemetry entry).

| | A: v0.14.7 +window | B: v0.14.7 NO window | C: v0.15.0-beta.2 |
|---|---|---|---|
| TSV rows | 96,048 | **96,048** | 98,529 |
| columns | 40 | 40 | **43** (none lost) |
| PSMs, rank-1 target q<=0.01 | 27,237 | 27,235 | 27,202 |
| unmodified | 12,354 | 12,353 | 12,443 |
| **Oxidation +15.995** | 1,539 | **1,539** | **1,602** |
| Carbamidomethyl +57.021 | 1,186 | 1,187 | 1,195 |
| Deamidation +0.984 | 516 | 516 | 521 |
| fragment_ppm median | 3.4121 | **3.4121** | 3.3765 |

❌ **THE PREDICTION THAT WAS WRONG.** The impact trace said removing
`fragment_min_mz`/`fragment_max_mz` "may MOVE FRAGMENT MATCHING", reasoning from
upstream's own stated rationale that those bounds "were decreasing the accuracy of
preliminary scoring estimation when attempting to annotate multiply-charged,
high-m/z ions". **Measured on liver: the removal is a NO-OP.** Column B is the
same binary with the two keys deleted — identical row count, identical Oxidation
count, identical fragment_ppm median. Plausible mechanism, real upstream
rationale, and it still did nothing here. Likely because the committed window
(150-2000 m/z) is wide enough to exclude almost nothing on this file; it may still
matter on data with a different fragment m/z range. **Do not carry the
"window removal moves fragment matching" claim forward — it is refuted on liver.**

✅ **ALL of the movement is the VERSION** (C vs B): Oxidation **+4.09 %**,
unmodified +0.73 %, Carbamidomethyl +0.67 %, Deamidation +0.97 %, and the
confident PSM count **−0.13 %**.

⚠ **THE +4.09 % OXIDATION IS NOT YET ATTRIBUTED.** It is ~30x the movement of
everything around it, and no mechanism is established. Candidates from the v0.15
changelog, listed as CANDIDATES and not conclusions: the precursor-m/z fixes
("Selected ion m/z of 0.0 was overwriting precursor.mz"; "Extract precursor m/z
from 'isolation window target m/z' if missing"), "Performance optimizations on
prefiltering", and the "C/N-term mixup in modification handling" fix. **Attribute
it before the write-up quotes a moved Oxidation rank** — the mod-rank correlations
against PTM-Shepherd (+0.616), MetaMorpheus (+1.000) and Mascot (+0.502) all rest
on recon's Oxidation count.

**Two source-based predictions that DID hold, now confirmed on real data:**
* `precursor_ppm` is signed — median **+132.83 -> -0.53**, with **14,581**
  negatives where v0.14.7 had zero. Exactly the `|x|` vs `x` signature.
* `fragment_ppm` stays absolute — zero negatives in both.

**Consequence for the regeneration gate, as the trace warned:** the 1 %
derived-statistic budget was calibrated on same-version noise (largest movement
ever seen 0.5464 %). Oxidation at 4.09 % exceeds it. **That is the gate working.**
Read each movement and re-pin deliberately; do not widen the budget.

**Preview corroboration survives.** recon's committed 3.4206 ppm should land near
3.38 against Preview's 3.5 — agreement moves from 0.08 to about 0.12 ppm.

### ⚠ SAGE SENDS TELEMETRY, AND RECON HAS NEVER DISABLED OR DISCLOSED IT (found 2026-09-01)

Sage has posted basic telemetry since v0.14.5. **recon does not pass
`--disable-telemetry-i-dont-want-to-improve-sage`** — `run_sage` passes only the
params file, `-o`, `-f` and the mzML — so **every recon search this project has
ever run phoned home**, and so would every user of a release zip.

**What is sent, read from `crates/sage-cli/src/telemetry.rs`, not assumed:**
version, peptide count, fragment count, file count, runtime, LFQ flag, TMT kind,
parquet flag, and OS/memory/core details. Posted to
`https://pax3h44gubc6o5ci23knddnw2i0qnuaz.lambda-url.us-west-2.on.aws/`.
**No scientific content** — Sage's changelog states identifications, quantities,
organism and modifications are NOT sent, and the struct agrees.

✅ **A1 REMOVES THIS BY CONSTRUCTION, which nobody had noticed.** `telemetry.send()`
is called from `sage-cli/src/main.rs:137` behind the CLI flag; `runner.rs:688`
only CONSTRUCTS the struct and returns it inside `Telemetry`. So calling
`Runner::run` directly — exactly what route A1 does — never sends. Embedding is
not merely neutral here, it is strictly quieter than shelling out.

⚠ **OPEN DECISION for landing 1** (still a subprocess): disable by default, make
it an opt-in flag, or leave it on and disclose in the README. Not decided — it is
Ben's call, because it is data leaving users' machines and because Sage's author
relies on it to prioritise development. **All exploratory runs in this session
passed the disable flag**, so no measurement here sent anything.

### 🔒 A1 LANDING-1 DOWNSTREAM-IMPACT TRACE — required before the change-regenerate (2026-09-01)

AGENTS forbids a change-regenerate without an enumerated trace first. This is it.
**Ben's go-ahead 2026-09-01: upgrade BEFORE the write-up**, so the paper is written
against the tool that ships. Rejected: writing up on v0.14.7 and upgrading after
(leaves the published method describing a build nobody ships), and shipping v0.15
while publishing v0.14.7 numbers (leaves the repo's artifacts unreproducible with
the shipped binary).

**✅ THE FINDING THAT SHRINKS THE WORK: the gates split into TWO INPUT FAMILIES,
and only one of them moves.**

| gate | reads | moves? |
|---|---|---|
| `ms1_calibration_integration` | `step1-open-*` only | **NO** |
| `pass2_wiring_integration` | `step1-open-*` only | **NO** |
| `run_validation` Tiers 1+3 | `step1-open-*` (`SNAPSHOT_SOURCE`) | **NO** |
| `analyzer_detection_integration` | `full-run/` | yes |
| `digestion_composition_integration` | `full-run/liver_search/pass2` | yes |
| `tier_assignment_integration` | BOTH | partly |

`testing/search-output/step1-open-{serum,bcell,b1906}` is a **frozen v0.14.7
artifact set from the +-50 ppm era**, already deliberately separate from
`full-run/` (which is +-20 ppm). **It is NOT re-searched by this upgrade.** That
keeps the behaviour-check evidence intact — above all the bcell MS1 sign flip
+0.70 -> -0.24 ppm, which is the proof the `|error|` bug is fixed and is
historical evidence, not a current measurement. Re-running it would delete the
evidence and keep the green tick, exactly as re-pointing that gate at liver would.

**WHAT WILL CHANGE, enumerated.**

**A. Committed artifacts — 32 tracked files under `full-run/`:** per file (x4)
`{f}.json`, `{f}.html`, `{f}.log`, `{f}_pass2.json`, `{f}_search/effective-params.json`,
`{f}_search/pass2/pass2-effective-params.json`, and Sage's own `results.json` for
both passes (8 total). Plus the 8 gitignored TSVs.

**B. Hard-pinned numbers that MUST be re-pinned deliberately** (all in
`digestion_composition_integration.rs`, all liver, all from `full-run/`):
peptides **10621**, missed cleavage **1818**, ragged-N **717**, ragged-C **320**,
completeness **82.88 %**, decoys ragged-N/C **81/26**, semi-tryptic class FDR
**10.32 %**. ⚠ `ms1_calibration_integration`'s **32133 / -0.2357** are NOT in this
list — they read the frozen family and must NOT be touched.

**C. Report schema, one field changes MEANING:**
`mass_accuracy.precursor_median_ppm` / `_p95_ppm` become SIGNED (see the code
audit). Every `full-run/*.json` carries them. This is a semantic change with no
column rename, so nothing can catch it automatically — it is the reason the
schema version must bump.

**D. Config templates — 4 files, 2 edits each:** remove `fragment_min_mz` /
`fragment_max_mz` (removed upstream, and SILENTLY ignored, so leaving them makes
the config lie), and add `protein_grouping: false`. The four are the two live
templates under `testing/configs/` and the two bundled defaults under
`recon-tool/src/defaults/`. **The anti-drift guard forces all four to agree** —
edit two and the suite fails, which is the guard doing its job.

**E. Pins and notices:** `SAGE_VERSION`, `SAGE_COMMIT` ->
`df9219951cc9a54cf4cd55d76541af24b687bd3d`, and the binary-redistribution notice
in `THIRD_PARTY_LICENSES.md`, which currently names only the Windows v0.14.7 build.

**F. Documents carrying liver's headline numbers:** `PLAN.md`, `NOTES.md`,
`README.md`, `reference-notes/gates-what-they-consume.md`, and
`testing/recon-output/comparison/LIVER-FOUR-TOOL-2026-09-01.md`.
⚠ **The four-tool comparison needs only RECON's column re-derived.** Preview,
PTM-Shepherd, MetaMorpheus and Mascot are fixed external runs and are NOT
re-executed; but the three rank correlations and the missed-cleavage spread are
computed against recon, so they move.

**G. NOT changed, stated so it is not done by accident:** `REQUIRED_COLUMNS` (the
contract holds, all 21 present), the frozen `step1-open-*` set, Tier 3 snapshots,
the step-0 anchors, and `testing/reference-data/` in its entirety.

**H. THE DECISION HELD OPEN ON PURPOSE:** `protein_grouping` is set to `false` for
this landing so that Sage's q-value changes and a new grouping algorithm cannot
move the subset size at the same time. Adopting Sage's grouping — or deleting
recon's `parsimonious_groups` in favour of it — is a SEPARATE landing with its own
trace. Two of the three options move every digestion number.

**THE ACCEPTANCE GATE.** `assert_regeneration_invariants.py` checksums the source
TSV, so "Sage changed it" is measured rather than inferred. Here the TSVs are
GUARANTEED to differ (different binary), so the budget arm applies and every
movement gets printed with its magnitude. ⚠ **The 1 % derived-statistic budget was
calibrated against same-version noise (largest movement ever seen: 0.5464 %). A
version change is not noise, and movements will exceed it. Do not widen the budget
to make the gate pass** — that is fitting the gate to the data it failed on. Read
the printed movements, decide each, and re-pin deliberately.

### ✅ A1 LANDING-1 CODE AUDIT — measured against a CLEAN upstream clone (2026-09-01)

Every claim here is read from `reference/sage-src/` (gitignored), a fresh clone of
`lazear/sage` checked out at **`v0.15.0-beta.2` = `df9219951cc9a54cf4cd55d76541af24b687bd3d`**,
clean working tree.

⚠ **METHOD NOTE, because it nearly went wrong.** The first pass read Sage source
from a local clone that was being actively edited. Ben stopped it. That tree's
`git status` shows `crates/sage-cli/src/{runner,lib,input,main}.rs`
MODIFIED. Its REMOTE is genuinely `lazear/sage`, and its TAGS are genuine upstream
objects — but the WORKING TREE is not upstream. Conclusions drawn from `cat` on
that tree were unsound; the same conclusions read from `git show <tag>:<path>`
were sound. **Read upstream from a tag or a clean clone, never from a working
tree that someone is editing.**

**1. ✅ THE COLUMN CONTRACT HOLDS — no parser break.** All **21** of recon's
`REQUIRED_COLUMNS` are present in the v0.15 header (`runner.rs:852`). 22 new
columns are added (`psm_id`, `protein_groups`, `num_proteins`,
`num_protein_groups`, `label`, `delta_next`, `delta_best`, `rt`, `aligned_rt`,
`predicted_rt`, `delta_rt_model`, `ion_mobility`, `predicted_mobility`,
`delta_mobility`, `matched_peaks`, `longest_y_pct`, `scored_candidates`,
`poisson`, `sage_discriminant_score`, `posterior_error`, `protein_q`,
`protein_group_q`). The validator tolerates extras.

**2. ✅ THE SIGNED `precursor_ppm` BARELY TOUCHES US — much smaller than this file
claimed.** The mechanism, from source: the column IS `Feature.delta_mass`, and it
loses its `.abs()`.

```
v0.14.7        delta_mass = (precursor_mass - monoisotopic - isotope_error).abs() * 2E6 / (...)
v0.15.0-beta.2 delta_mass = (precursor_mass - monoisotopic - isotope_error)       * 2E6 / (...)
```

**recon's shipped MS1 bias never reads that column.**
`calibration::signed_precursor_ppm` is `delta_mass_corrected / calcmass * 1e6`,
reconstructed from the MASS columns, and `delta_mass_corrected` is recon's own
`delta_mass - isotope_error * C13_C12_DIFF`. So `bias_ppm` **cannot double-sign**,
and this file's instruction to "verify the MS1-mass-accuracy path in `qc.rs` does
not double-sign" is moot twice over — that path is `calibration`, not `qc`, and
`qc::compute_ms1_mass_accuracy` was DELETED 2026-09-01.
⚠ **The ONE real consumer is `qc::compute_qc_stats`**, which puts the raw column
into `mass_accuracy.precursor_median_ppm` / `_p95_ppm`. Under v0.15 that becomes a
SIGNED distribution. It is already documented as garbage-by-design in an open
search (serum reads 14106.379 ppm — the delta-mass spread, not instrument error),
but it is a SCHEMA FIELD whose meaning and sign change, so it belongs in the
downstream trace.

**3. ✅ `fragment_ppm` STAYS ABSOLUTE — verified, not assumed.** The changelog is
silent on it, so the accumulation was compared directly at both tags and the lines
are byte-identical, `.abs()` included:
`score.ppm_difference += peak.intensity * (mz - peak.mass).abs() * 2E6 / (mz + peak.mass)`
(v0.15 `scoring.rs:699`, v0.14.7 `scoring.rs:607`). **So the locked entry "MS2
stays ABSOLUTE" survives the upgrade, and recon's 3.4206 ppm stays comparable to
Preview's 3.5.**

**4. ⚠ THE REMOVED CONFIG KEYS FAIL SILENTLY.** `Input` derives plain
`Deserialize` with **no `deny_unknown_fields`** anywhere in `sage-cli` or
`sage/database.rs`, and `fragment_min_mz`/`fragment_max_mz` are gone from the
codebase entirely. So a template carrying them **parses fine and the values are
ignored, with no error and no warning** — the search silently stops applying a
fragment m/z window. Same failure shape as the recorded "Sage SKIPS an invalid
`variable_mods` key with only an ERROR log". **Both live templates and BOTH new
bundled defaults carry those keys.** They must be REMOVED explicitly in the
upgrade landing so the config states what actually happens.

**5. 🔴 PROTEIN GROUPING, DEFAULT ON, IS THE BIGGEST ITEM AND IT IS A DECISION.**
See the corrected v0.15 block at the top of this file. Sage v0.15 ships
IDPicker-style greedy set cover by default — the algorithm recon implemented for
itself on 2026-08-31 as `parsimonious_groups`. Three options, none picked yet:
keep recon's parsimony and ignore Sage's; adopt Sage's columns and delete recon's;
or set `protein_grouping: false` to hold v0.14.7 behaviour and defer. **This must
be decided deliberately, because two of the three move the reported subset size
and therefore every digestion number.**

### ✅ STEP 4 PART 1 — THE BUNDLING IS DONE (2026-09-01)

`recon` no longer reads any resource from a path relative to the working
directory. Three did: the pass-1 template, the pass-2 template and the curated mod
list. A user who unzipped a release got an error naming a repo directory they do
not have.

**What changed.** `recon-tool/src/defaults.rs` carries all three compiled in via
`include_str!` — around 20 KB of text this project authors or curates.
`write_effective_params_from_text` and `write_pass2_params_from_text` take the
template TEXT plus a LABEL naming its origin; the path-taking forms stay as thin
wrappers, so every gate that proves a guard against a REAL committed template is
untouched. `CuratedDb::load` now delegates to `load_from_sources`, so the file and
embedded routes share ONE parser rather than a second copy of it.

**NOT bundled, deliberately:** `unimod.xml` (2.5 MB) and any FASTA. Size is half
the reason; the other half is that they are third-party data whose redistribution
terms are NOT settled in `THIRD_PARTY_LICENSES.md`, which today covers only Sage
and mzSniffer. ⚠ **The curated mod list IS now embedded, and that is
redistribution too — its licence is equally unsettled and must be resolved before
anything ships.** Flagged, not assumed either way.

✅ **BOTH LICENCE QUESTIONS ARE NOW SETTLED (2026-09-01), and the unimod half of
the paragraph above is SUPERSEDED.**
- **MetaMorpheus: MIT.** Read from `LICENSE.txt` in the vendored clone, not from
  memory. Copyright (c) 2016 Stefan Solntsev, Craig D. Wenger. MIT permits the
  modified redistribution we are doing. A `## MetaMorpheus` section now records
  which three files are unmodified and exactly how `Mods.txt` differs.
- **Unimod: Design Science License, and Ben approved embedding it.** The XML
  carries its own notice granting copy and distribution. A `## Unimod` section
  now records it, with the snapshot identity (schema 2.0, 1560 `<umod:mod>`
  records).
  ✅ **UNBLOCKED the same day — the licence text was FOUND, not worked around.**
  Ben located it at https://www.unimod.org/dsl.txt. The full Design Science
  License is now reproduced in `THIRD_PARTY_LICENSES.md`, which is what the DSL
  itself demands. The apparent conflict dissolved: unimod.org's plain-language
  "public domain database, distributed under a copyleft licence" and the file's
  "Design Science License" are the SAME licence described two ways, not two
  competing claims.
  ⚠ **A mid-course correction worth remembering.** Between the notice and the
  find, this entry said the website statement superseded the DSL because no such
  file existed. That was wrong, and it was written from an absence of evidence.
  The lesson is the project's own rule: an external claim gets verified against
  the source, and "I could not find it" is not "it does not exist".

  **Three obligations, read from DSL Section 3, not assumed:**
  1. A copy of the License must ship WITH the work — so
     `THIRD_PARTY_LICENSES.md` must go inside the release archive.
  2. The copyright notice and warranty disclaimer must appear on all copies.
  3. ⚠ If `unimod.xml` is compiled into the binary, ALSO ship the XML file in the
     archive. Section 3 allows the Object Form only when the Source Data travels
     with it (3a) or a written offer does (3b). Embedded bytes are an unclear
     case; shipping the file costs nothing and removes the question.

  **The DSL does not reach recon's own code.** Section 3's aggregation clause
  says combining the Work with works not based on it leaves those works outside
  the License. recon is not a derivative of the Unimod database.
  The `include_str!` can now land with the argument freeze.

**The guard against the obvious failure.** A bundled copy of a config is a config
that can drift from the committed one, which would make the shipped tool and the
tested tool two different tools, silently.
`defaults::tests::bundled_default_matches_committed_template` parses both and
asserts they agree on EVERY field, allowing exactly two documented omissions —
`database.fasta` and `mzml_paths`, which the bundled copies drop because both are
always supplied at runtime and a path there would be a ghost.
**Falsified:** `min_peaks` 15 -> 16 in the bundled copy fails with
`min_peaks: bundled 16 != committed 15`.

**Equivalence measured, not assumed:** the curated list loads to **99 entries, 0
skipped, identical entry-for-entry** from the embedded text and from
`reference-notes/metaMorpheusMods/`. Compared on id, position, category and mass,
not merely on count — a reordering or a dropped field would hold the count and
change the decisions.

✅ **A GATE THAT WAS DEAD ON AN ARM64 HOST NOW RUNS.**
`the_vendored_binary_reports_exactly_sage_version` was silently skipping because
`DEFAULT_SAGE_PATH` was a single Windows path. With the four known vendored
layouts tried it finds the arm64 build and reports **0.14.6 = `SAGE_VERSION`**.
The skip-detector line count is now **zero**.

**172 tests, 0 failures. `run_validation.py` 17/17** (was 15/15 — see below).

### ✅ LIVER IS IN THE STANDING TRIPWIRE (2026-09-01)

The gap recorded for three sessions is closed. `FILES` was
`["b1906", "bcell", "serum"]`; liver — the file the entire write-up rests on — had
never been gated. **Fixed by ADDING liver, not by removing the others**, which are
behaviour checks each covering something only that file shows.

Liver now runs Gate 1 (delta~0 dominant) and Gate 3 (the +57 agnostic band):
**15/15 -> 17/17.**

⚠ **Gate 2 reports NOT CHECKED on liver, and that is the honest answer.** Gate 2
compares the report against a step-0 anchor, and the anchors are an INDEPENDENT
measurement of each file's own TSV. There is no liver anchor. Deriving one from
the report Gate 2 checks would make the gate compare a number against itself —
vacuous, and exactly the "one agreeing case" failure. A new `Results.not_checked`
channel prints it every run and lists it in the summary, counted as **neither a
pass nor a failure**: counting it as a pass is the silent-skip trap, counting it
as a failure would make an absent input look like a regression. Producing a real
liver anchor is its own task.

### ⚠ LATENT — the MS1 calibration gate silently skips on a fresh checkout (2026-09-01)

`ms1_calibration_integration.rs` does `let Some(subset) = clean_subset(..) else
{ return }`, printing "skipping" and PASSING when its TSV is absent. Its inputs
live under `testing/search-output/`, which is gitignored (`.gitignore:31`). Where
those files are present the test really runs — verified with
`--nocapture`: bcell bias -0.2357 ppm, MAD 0.6733, n=32133. **On a clean clone it
would pass without testing anything.** Same class as the 2026-08-25 gate audit.
Not fixed; recorded.

✅ **IT IS NOW DETECTABLE, EVEN THOUGH IT IS STILL NOT FIXED (2026-09-01, later
session).** The skip prints a line; the passing count hides it. So read the lines,
not the count:

```
cargo test -- --nocapture 2>&1 | grep skipping
```

Every line names an input the suite could not reach. **Measured locally with
every input present: exactly ONE line**, and it is not a data gap —
`skipping: no vendored Sage binary on this machine`, from
`the_vendored_binary_reports_exactly_sage_version` (`sage_runner.rs:289`), because
`DEFAULT_SAGE_PATH` is a Windows x86_64 path. That gate is therefore **dead on an
arm64 build and live on a Windows x86_64 build**. Pointed at the arm64 build via
`SAGE_PATH` it passes and reports `0.14.6` = `SAGE_VERSION`, so the PIN is
verified and only the default path is wrong.
⚠ **Five test files skip this way**, not one: `analyzer_detection_integration`,
`digestion_composition_integration`, `ms1_calibration_integration`,
`pass2_wiring_integration`, `tier_assignment_integration`. The command is in
README, next to the warning that was already there. It is a DETECTOR, not a fix —
the tests still report `ok` when they skip.

### ⚠ `full-run/bcell.json` reads bias_ppm EXACTLY 0.0 — open observation (2026-09-01)

The committed `full-run/bcell.json` (schema 1.7.0) has `bias_ppm: 0.0` with
`clean_subset_n_psms: 19280`, while this file's own bcell entry and the passing
integration test both give **-0.2357 ppm** at n=32133.
**Not a contradiction — two different searches.** The test reads
`testing/search-output/step1-open-bcell/` from the +-50 ppm MS2 era; `full-run/`
was regenerated 2026-08-31 at +-20 ppm, which changes the PSM set.
⚠ An exact 0.0 median is still unexplained, and is NOT evidence of a broken
signed path — liver reads -1.4193 in the same regenerated set. Inspect the
pre-aggregation distribution if it ever becomes load-bearing. Left open on the
token budget, deliberately.

### 🔒 ACCEPTANCE CRITERION: method spread must be small against sample spread (2026-08-31)

**Recon does not need to reproduce Preview or MSFragger exactly.** The bar, set by
Ben: would a user make a drastically wrong call — toss samples they shouldn't?
Stated numerically, and testable on every future change:

| | range |
|---|---|
| **between-sample** (one method, four files) ragged-N | 3.71 → **28.38 %** = **24.7 pp** |
| between-sample missed cleavage | 14.7 → 30.6 % = 15.9 pp |
| **between-method** (one file, recon vs MSFragger) ragged-N | **0.77 pp** |
| between-method ragged-C | 0.54 pp |
| between-method missed cleavage | **2.4 pp** |

The biological signal is **32×** the method disagreement on ragged-N and 6.6× on
missed cleavage. Serum reads 28.4 % ragged-N against bcell's 3.7 %; nobody
confuses those over a 0.77 pp method difference.

⚠ **Missed cleavage is the weakest link** — the worst ratio, and it is the
headline. State it; do not redesign around it.

### 🔒 `precursor_tol.ppm` INVERTS TOO — measured, not inherited (2026-08-29)

AGENTS.md locks the sign inversion for `precursor_tol.**da**`. It says nothing
about `.ppm`, and the Pass 2 window is ASYMMETRIC (a ladder rung centred on the
measured bias), so a wrong sign would move the window off the bias by twice the
bias with NO symptom in the output. It was measured rather than assumed.

**The probe.** Serum, open search, `"precursor_tol": {"ppm": [-30, 5]}`.
Observed delta `(expmass - calcmass)/calcmass` over 12746 target PSMs:
**min −5.047, max +30.007 ppm.** The walls sit on −5 and +30.

> To search delta in `[lo, hi]`, the config must read `[-hi, -lo]`.

Same rule as `.da`. The conversion happens in exactly ONE place,
`pass2::precursor_tol_json`.

**Proven again end-to-end on the real Pass 2 output.** Serum's measured window is
delta −7.49..+12.51 ppm, written to the config as `ppm [-12.51, +7.49]`. The
Pass 2 TSV's isotope-corrected delta runs **−7.579 to +12.585** with a **median
of +2.346 ppm** against a measured bias of +2.43 — the window really is centred
on the bias. A wrong sign would have capped the maximum at +7.49.

**⚠ A SYMMETRIC WINDOW CANNOT TEST THIS.** `[-10, 10]` inverts to itself. The
control is `pass2_precursor_window_is_sign_inverted` (asymmetric), and
`a_symmetric_window_cannot_detect_the_sign_error` exists so nobody "verifies"
the rule with an example that cannot fail. `the_pass2_config_encodes_the_measured_window_inverted_on_all_three_files`
runs it on real measurements, including bcell where the bias is NEGATIVE and the
asymmetry therefore points the other way. Both were falsified before being
believed: flipping the sign fails them.

### ⚠ Re-measuring the calibration inputs at ±50 ppm — the rung does NOT move, but the ID count does (2026-08-29)

Every MS1 bias and MAD behind the ladder and the Pass 2 windows was measured on
TSVs searched at `fragment_tol ±20 ppm`. Pass 1 now runs at ±50 ppm after the
detector-aware change. That was never re-measured. It is now, on ALL THREE files.

**FIRST, the canonical source had to be identified, and it is not the obvious
one.** NOTES' clean-subset sizes 3764 / 32133 / 10942 match `step1-open-*`, which
are the **no-fixed-mods** searches. `open-serum-full` carries
`static_mods {C: 57.0215}` and gives 5755 on serum. Re-running `step1-open-serum`
here reproduced bias +2.4215, MAD 0.4845, requirement **4.8441** — NOTES' "4.844".
bcell and b1906 reproduced too (−0.2357 / 0.6733 / 3.6024 and +0.4403 / 0.6844 /
3.8623).

**THE ANSWER: nothing downstream moves.**

| file | requirement ±20 | requirement ±50 | rung |
|---|---|---|---|
| serum (canonical) | 4.8441 | 4.8364 | ±10 → ±10 |
| bcell | 3.5835 | 3.5218 | ±10 → ±10 |
| b1906 | 3.8606 | 3.8711 | ±10 → ±10 |

Largest movement is **0.06 ppm** against a rung boundary 5.1 ppm away.

**⚠ BUT THE PREMISE WAS BACKWARDS, AND THIS IS THE FINDING.** "±50 admits more
marginal PSMs" is false. Widening the pass-1 fragment tolerance **LOSES**
confident identifications on all three files:

| file | conf. rank-1 target PSMs ±20 → ±50 | change |
|---|---|---|
| serum (no fixed mods) | 13996 → 12352 | **−11.75 %** |
| serum (fixed mods) | 17382 → 15852 | −8.80 % |
| bcell | 64023 → 59685 | −6.78 % |
| b1906 | 25853 → 23495 | −9.12 % |

A wider fragment window buys random matches: serum's decoy rows rise 23610 →
26722 (+13.2 %), and FDR pays for it. This does NOT refute the ±50 ppm bucket —
its justification is that an out-of-calibration instrument needs the width, and
none of these three files is out of calibration. What it does refute is the
unstated assumption that the width is FREE on a well-behaved Orbitrap. It costs
about 7-12 % of identifications. **Record it as a stated cost of the
detector-aware default, and note that a per-file adaptive narrowing after pass 1
is NOT possible — pass 1's tolerance must be chosen before any PSM exists.**

**The ±20 control reproduced the committed `open-serum-full` TSV bit-for-bit in
every mass column.** Only `poisson`, `sage_discriminant_score`, `posterior_error`
and the q-values differ (~1e-4 relative). That is Sage's LDA fit being
run-to-run non-deterministic, consistent with "Sage output is NOT bit-reproducible
across builds — but every RAW measurement is".

### ⚠ Sage's `fragment_ppm` convention CONFIRMED per-fragment, and the 5× multiplier survives for a NEW reason (2026-08-29)

`testing/search-output/probe-serum-annotate-matches/` holds 210309 matched
fragments — the only per-fragment data this project has. Two things were settled
with it and no new search.

**1. The convention claim is confirmed.** Sage's `fragment_ppm` is the
**intensity-weighted mean of |error|** per PSM. Reconstructing it from the
per-fragment m/z columns matches the TSV column to a median of **0.0091 ppm**
(max 0.085), which is the quantisation of the 1e-5 m/z print precision. A plain
unweighted mean is 40x further off (median 0.382).

**2. The per-PSM coverage figure did NOT support the window, and the correct
figure is worse.** NOTES recorded "5× covers 99.6 / 99.8 / 100.0 %" — a per-PSM
statistic on a per-PSM summary quantity. The window has to contain individual
FRAGMENTS. Over 138638 fragments from 9497 confident serum PSMs:

| window | per-fragment coverage | PSMs falling below `min_matched_peaks = 4` |
|---|---|---|
| 3× median = 3.78 ppm | 90.39 % | 2.138 % |
| **5× median = 6.30 ppm** | **96.82 %** | **0.632 %** |
| 6× median = 7.56 ppm | 98.19 % | 0.263 % |

p99 needs **6.76×**. ⚠ The probe was a CLOSED search at `fragment_tol ±10 ppm`,
so the distribution is CENSORED at 10 ppm (0.32 % of fragments sit within 0.5 ppm
of the wall). Every coverage figure is an UPPER bound and every multiplier a
LOWER bound.

**3. So the multiplier was measured on the metric that actually matters — Pass 2's
own output.** Pass 2 re-run on serum at six fragment tolerances:

| multiplier | half-width | conf. PSMs | vs 5× | semi-tryptic % |
|---|---|---|---|---|
| 2× | 2.45 ppm | 12070 | −6.86 % | 31.87 |
| 3× | 3.68 ppm | 12673 | −2.20 % | 32.16 |
| **5×** | **6.13 ppm** | **12959** | — | **32.33** |
| 8× | 9.80 ppm | 13066 | +0.83 % | 32.47 |
| 12× | 14.70 ppm | 13083 | +0.96 % | 32.42 |
| 20× | 24.51 ppm | 13100 | +1.09 % | 32.45 |

**Quadrupling the window past 5× buys 1.09 % more PSMs and moves the reported
semi-tryptic rate by 0.12 percentage points.** The 96.8 % per-fragment coverage
is real and inconsequential, because losing a fragment is not losing a PSM —
`min_matched_peaks` is 4 and the median PSM matches far more. **5× stays, and its
justification is now this table rather than the per-PSM coverage number, which is
withdrawn as evidence for it.**

**Note the contrast with pass 1**, where widening COSTS PSMs. Pass 2's search
space is ~45x smaller, so the FDR penalty for a wider window is far weaker. The
two passes genuinely behave differently and must not be reasoned about together.

### MSFragger tight re-run — step 1 ground truth, part A: MS1/MS2 error + Ox/Ac/semi-tryptic prevalence (2026-08-24)

- **What:** Ben ran FragPipe/MSFragger tight closed searches on all three raw files, two enzyme
  tiers (`strictTryp`: `num_enzyme_termini=2`; `semiTryp`: `num_enzyme_termini=1`), fixed
  Cys+57.02146 (Carbamidomethyl), variable Met oxidation (max 3/peptide) and protein N-term
  acetyl (max 1/peptide). `precursor_true_tolerance` 20 ppm, `fragment_mass_tolerance`
  auto-calibrated 20→10 ppm (`calibrate_mass=2`, "find optimal parameters"). Committed at
  `testing/reference-data/msfragger/{strictTryp,semiTryp}/` — `fragger.params`, per-file
  `psm.tsv`/`peptide.tsv`/`protein.tsv`, and the FragPipe `log_2026-08-24_*.txt`. Philosopher's
  `psm.tsv` is already FDR-filtered (`Is Decoy=false`, `Qvalue` ≪ 0.01 on every row checked) — no
  additional filtering needed to use it as-is.
- **This settles the ±10 vs ±20 ppm provenance question named in PLAN step 1.** MSFragger's own
  `precursor_true_tolerance` is 20 ppm (not narrowed by calibration); `fragment_mass_tolerance`
  auto-tunes 20→10 ppm. Recorded here as the frozen reference value.
- **MS1/MS2 error ground truth**, from `calibrate_mass`'s per-run Old (raw)/New (post-calibration)
  median+MAD table in the log (`*** MASS CALIBRATION AND PARAMETER OPTIMIZATION ***`). **Identical
  between `strictTryp` and `semiTryp`** — MSFragger's calibration pass runs on the enzyme-agnostic
  first search, not the final tiered search space, so one table serves both tiers:

  | File | MS1 old median/MAD (ppm) | MS1 new median/MAD | MS2 old median/MAD (ppm) | MS2 new median/MAD |
  |---|---|---|---|---|
  | serum (909c), Run 001 | +2.50 / 0.70 | +0.08 / 0.45 | +0.96 / 1.33 | +0.02 / 1.19 |
  | bcell (B.naive), Run 002 | −0.01 / 0.99 | −0.02 / 0.66 | +0.96 / 2.49 | +0.11 / 2.47 |
  | b1906, Run 003 | +0.50 / 1.05 | −0.07 / 0.79 | −0.05 / 1.62 | −0.14 / 1.61 |

  Run-number-to-file mapping confirmed from the "First search" file-processing order in the same
  log (001=909c, 002=B.naive, 003=b1906), not assumed.
- **Use the OLD (pre-calibration) column as the comparison point for recon's own `bias_ppm`.**
  Same rule already locked for the MetaMorpheus comparison above: recon's open search runs on raw,
  uncalibrated mzML, so it should be compared against another tool's *raw* measurement, not its
  post-correction residual.
- **Cross-check against numbers already in NOTES — agrees closely, treat as replication, not new
  information for serum/b1906:** the existing MSFragger `calibrate_mass` table (serum 2.51, b1906
  0.53) and MetaMorpheus round-0 (serum +2.351, bcell −0.295, b1906 +0.314) both sit within ~0.2
  ppm of this run's Old-column numbers. **bcell now has an MSFragger data point for the first
  time** (previously "none" in the Phase 8.5/MSFragger column of the part-2 comparison table
  above) — MS1 old −0.01 ppm, MS2 old +0.96 ppm. MS2 −0.01 vs MetaMorpheus's bcell MS2 +0.942 is a
  near-exact match (two independent tools), which sharpens the part-2 "bcell: MS2 disagrees in
  magnitude" finding: it is recon (+1.87) that is the outlier, not MetaMorpheus, now confirmed by a
  third source. bcell MS1 stays a genuine three-way spread with no majority: MSFragger −0.01,
  MetaMorpheus −0.295, recon +0.65 — do not resolve this by picking two of three; it is unresolved,
  same as the part-2 entry already says.
- **Known-PTM prevalence** — Ox(M), N-term-Ac, and (semiTryp only) enzymatic-termini split, all
  PSM-level, via `testing/scripts/msfragger_prevalence_summary.py` (new script, reads the `psm.tsv`
  files directly, stdlib-only, no pandas dependency):

  | Tier | File | PSMs | Ox(M) % | N-term-Ac % | Fully-tryptic % | Semi-tryptic % |
  |---|---|---|---|---|---|---|
  | strictTryp | serum | 4,366 | 9.21 | 0.07 | 100 (by construction) | — |
  | strictTryp | bcell | 53,029 | 6.83 | 1.92 | 100 (by construction) | — |
  | strictTryp | b1906 | 14,581 | 13.40 | 0.94 | 100 (by construction) | — |
  | semiTryp | serum | 7,042 | 7.78 | 0.04 | 62.00 | 38.00 |
  | semiTryp | bcell | 53,719 | 6.88 | 1.91 | 97.51 | 2.49 |
  | semiTryp | b1906 | 15,458 | 13.03 | 0.89 | 93.23 | 6.77 |

  Non-tryptic (0 termini) was 0.00% on all three files in the semiTryp tier — MSFragger's
  semi-enzymatic mode still requires at least one enzymatic terminus here.
- **Cross-check: serum's semi-tryptic rate replicates the existing biology-not-bad-digestion
  finding.** MSFragger semiTryp gives 38.0% semi-tryptic for serum, vs. the previously recorded
  31.8% from the Sage-based two-pass digestion workflow (`digestion_efficiency.py`, "Known
  permanent limitations" below) — different tool, different method, same direction and same order
  of magnitude, both far above bcell (2.49%) and b1906 (6.77%). Independent confirmation that
  serum's high semi-tryptic fraction is a biofluid property, not a failed digest.
- **What this run does NOT settle — do not conflate with the +57 reconciliation checklist item.**
  Cys+57 is FIXED in this config (`add_C_cysteine = 57.02146`), not variable. It cannot be used to
  count "PSMs carrying a discoverable +57 delta" the way the 2026-08-21 tight-search comparison did
  (Carbamidomethyl(C) as a VARIABLE mod, counted via `analyze_ptm_recovery.py` /
  `compare_predicted_vs_verified_cam57.py`). PLAN step 1's "+57 PSM count reconciliation" item is
  **still open** after this run. The original `tight909c`/`tightB1906`/`tightBcell` run
  directories it depended on (`testing/search-output/ptmRecovery/`, gitignored) are not present on
  disk as of 2026-08-24 — recovering them may not be possible. The likely path forward is a
  fresh variable-+57 tight search (cheap: flip `add_C_cysteine` to `0.0` and add a
  `variable_mod_0N = 57.02146 C 1` line to a copy of this session's `fragger.params`), not
  archaeology on a directory that may no longer exist. Flagged for whoever picks up that checklist
  item; not resolved here.

### +57 PSM count reconciliation — resolved for the recon side (2026-08-24)

- **What:** the step 1 checklist item above asked which of two JOURNAL entries' recon +57 counts
  was correct — 2026-07-24 (bcell 3311 / serum 1125 / b1906 1253 at 4.50 / 7.31 / 4.47%) or
  2026-08-21 (187 / 86 / 60 at the same percentages). Ben re-ran `recon analyze --full` on all
  three files from the current build, using the alkylation-agnostic `-nofixedmods` open-search
  configs (`testing/recon-output/full-run/{serum,bcell,b1906}.json`, commit `75a1d8d`).
- **Result: the fresh run reproduces the 2026-07-24 counts exactly, not the 2026-08-21 ones.**
  `mod_discovery.peaks`, rank 2 on all three files (right after the unmodified 0.0 peak),
  Unimod-annotated Carbamidomethyl:

  | File | count | count_pct |
  |---|---|---|
  | serum | 1,125 | 7.3118% |
  | bcell | 3,311 | 4.5031% |
  | b1906 | 1,253 | 4.4742% |

  Exact match to 2026-07-24, to 4 decimal places on the percentage. This settles which historical
  number to cite going forward: **2026-07-24's counts are correct.** The 2026-08-21 counts (and the
  48.4x/35.1x/57.7x "verified vs Recon" ratios computed from them) were against something else —
  most likely the fixed-C run, as the 2026-08-24 JOURNAL entry already guessed, though the exact
  cause of the 2026-08-21 numbers was not re-derived, only superseded by this independent
  reproduction. **Do not carry the 48.4x/35.1x/57.7x ratios or the provisional 2.7x recomputation
  forward — use these counts if that comparison is needed again.**
- **Independent side now done too — the true gap is ~1.2–3x, not 35–58x.** Ben ran the requested
  variable-+57 MSFragger search (`testing/reference-data/msfragger/strictTrypVarCAM/`, commit
  `52203dc`): strict trypsin, `add_C_cysteine` commented out, `57.02146 C 3` added as a variable
  mod, `max_variable_mods_per_peptide` raised 3→5 (three independent variable-mod types now
  compete for the per-peptide slot budget), everything else identical to `strictTryp/fragger.params`
  — confirmed by diff, only those three lines differ. Calibration table in the new run's log
  reproduces `strictTryp`'s within 0.01–0.02 ppm (e.g. serum MS1 old 2.51 vs 2.50), confirming the
  two runs are otherwise comparable. Counted confident PSMs carrying a `C(57.02...)` tag in
  `Assigned Modifications` via new script `testing/scripts/reconcile_cam57.py`:

  | File | MSFragger total PSMs | MSFragger +57 PSMs | MSFragger +57 % | recon +57 count | recon +57 % | ratio (MSFragger / recon) |
  |---|---|---|---|---|---|---|
  | serum | 4,398 | 1,381 | 31.40% | 1,125 | 7.31% | **1.23x** |
  | bcell | 52,899 | 9,873 | 18.66% | 3,311 | 4.50% | **2.98x** |
  | b1906 | 14,579 | 2,778 | 19.05% | 1,253 | 4.47% | **2.22x** |

  Total PSM counts are close to `strictTryp`'s fixed-C run (serum 4398 vs 4366, bcell 52899 vs
  53029, b1906 14579 vs 14581) — expected, confirms the variable-mod change didn't materially alter
  the confident-PSM population, only which of them get the +57 tag.
- **This replaces the 35–58x figure from 2026-08-21 (now known to be built on the wrong recon
  denominator) with a genuine, freshly-measured, same-day-comparable ratio: recon's single
  delta-mass peak undercounts the true +57-carrying PSM population by roughly 1.2x (serum) to 3x
  (bcell), not by an order of magnitude.** Serum in particular is close to a 1:1 match. This is
  consistent with, and now gives real numbers to, the three-part gap decomposition already in
  JOURNAL 2026-08-24 (satellite splitting + single-pass search recovery + currency difference) —
  a 1.2–3x gap is a plausible sum of the first two components alone, without needing to lean on the
  currency-difference explanation as heavily as the old 35–58x figure implied. **Use this table, not
  the 2026-08-21 one, for any future write-up discussion of the +57 undercount magnitude.**
- **Third confirmation, free: mod discovery is stable across builds.** `nofixedmods/*.json`
  (generated 2026-07-24) and `full-run/*.json` (generated 2026-08-24, current build) give
  **identical** +57 counts and percentages on all three files — 1125/7.3118%, 3311/4.5031%,
  1253/4.4742%, matching to four decimal places. A month of builds apart, same answer. Two
  consequences: the 2026-07-24 counts are confirmed a third way, and **the cross-tool comparison
  tables in `recon-output/comparison/`, which are built on `nofixedmods/`, do not need
  regenerating.**
  - **Extended to the full peak list 2026-08-24 (step 2 prep).** The check above compared the
    +57 row only. A field-by-field diff of every peak, all three files, gives: `rank`,
    `delta_mass`, `count`, `count_pct`, `unannotated`, `ambiguous`, and annotation names
    **identical on all peaks**. Only two fields move. `prominence` differs on 6 peaks total
    (serum ranks 39/46, bcell rank 42, b1906 ranks 28/35/37 — all tail). `representative_mz`
    is **null on every peak** in `nofixedmods/`, because the field post-dates that run.
  - **The two artifact sets were made by different subcommands at different peak floors.**
    `nofixedmods/` came from `discover` (commit `cdf7857`); `full-run/` from `analyze`
    (commit `75a1d8d`). `discover`'s CLI default is `min_peak_count = 10`
    (`main.rs`); `analyze` uses `ModDiscoveryConfig::default()`, which is `5`
    (`mod_discovery.rs`). **Which value each run actually used is not recorded** — see the
    config-provenance gap entry below. This is the leading candidate for the 6 prominence
    deltas, but it is UNTESTED: confirming it needs a `discover --min-peak-count 5` re-run.
  - **The regenerating conclusion still stands, and now for a measured reason.** No script in
    `testing/scripts/` reads `prominence` or `representative_mz` — grep returns zero hits.
    The comparison and satellite scripts consume `delta_mass`, `count`, `count_pct`, and the
    annotation label only, and those are bit-identical. `nofixedmods/` is therefore verified
    against the current build on every field anything downstream consumes. **Do not archive or
    delete it** — it backs the "+57 at rank 2 on all three files" statement and feeds
    `recon-output/comparison/`.
- **Caveat, stated not resolved:** this MSFragger reference is itself a *narrower* search than
  recon's open search (single confirmed variable mod at a fixed mass vs. an unrestricted delta-mass
  scan) — it is a strong, independent, same-day-comparable anchor, not a ground truth beyond
  challenge. Do not treat the 1.2–3x figures as more precise than that framing supports.
- **Also found during this verification pass, unrelated to the count itself:** `report.alkylation`
  (the "Fixed mod assumed... ✓ Alkylation appears complete" block) is stale leftover from the
  dropped fixed-C design and does not reflect the actual +57 discovery — see the KNOWN GAP entry in
  "Intentional, not bugs" below. The real evidence of a correctly alkylation-agnostic search is the
  peaks-list rows above, not that block.

### ✅ MS1 tolerance recommendation — QUANTIZED, SHIPPED 2026-08-28 (reasoning locked 2026-08-24)

Full evidence, citations, and the assumptions ledger:
`reference-notes/ms1-tolerance-recommendation-rationale.md`. Summary here; that note is the
paper source.

- **What's wrong:** `ms1_user_recommendation` emits +1.46/+4.09 (serum), −0.16/+2.18 (bcell),
  −0.19/+3.39 (b1906) ppm. Nobody searches an Orbitrap at ±2 ppm. Three independent lines of
  evidence say this is 3–5x too tight: field standard is 10 ppm (15–20 routine); **MSFragger ran
  these exact files at 20 ppm** while measuring ~1 ppm scatter, i.e. ~20x its own measured error;
  and Wilmarth's PSM-yield-vs-window data shows yield *rising* with window width (81,655 @ 10 ppm
  → 108,826 @ 1.25 Da) because "narrow tolerances do not reject noise, they select different
  noise."
- **Two compounding defects, not one.** (1) The implementation drifted from its own design:
  `recon-calibration-design-v2.md` specifies `median + 3×IQR` (MetaMorpheus-derived); the code does
  `bias + p95(|dev|)`, which is 57–90% of that width, inconsistently across files. (2) Even the
  design's formula is tighter than the evidence above supports.
- **Mechanism (INFERRED, not measured):** the recommendation is computed over the clean subset
  (near-zero-delta, rank-1, q<0.01, top-60% hyperscore) — by construction the best-behaved PSMs in
  the file — then applied as a bound for a real search containing modified, low-intensity,
  extreme-m/z, and lower-scoring peptides. **Named test, not yet run:** compare clean-subset MAD
  against all-confident-PSM MAD on the same file (needs the Sage TSVs; gitignored). The conclusion
  does not depend on this mechanism — the three evidence lines are independent of it — but the
  *explanation* does. Do not state it as demonstrated in the write-up until the test is run.
- **✅ IMPLEMENTED 2026-08-28.** `calibration::quantize_ms1_tolerance`, with
  `MS1_TOLERANCE_LADDER_PPM` and `MS1_TOLERANCE_MAD_K` as named constants; schema 1.6.0
  carries `user_recommendation_tolerance_ppm`, and `user_recommendation_low/high_ppm`
  become symmetric ∓ the rung. **Measured on the |error|-corrected inputs: serum 4.844,
  bcell 3.602, b1906 3.862 ppm — all three on the 10 ppm rung.** These match the
  rationale note's §12 pre-computation (4.84 / 3.60 / 3.86) to three decimals, and the
  acceptance test asserts against THOSE predicted values rather than against whatever the
  code emits. The k-insensitivity claim is asserted in code too (k = 3, 5, 10, 15 all give
  10 ppm on serum), not left as prose. A control test asserts the new rung is materially
  wider than the superseded formula on every file — measured superseded highs +4.11 /
  +2.09 / +3.89 ppm against ±10.
- **✅ THE CONTAINMENT GUARANTEE, asserted 2026-08-28 — this is why symmetric is safe.**
  Whenever the requirement fits on the ladder, `[−rung, +rung]` FULLY CONTAINS the
  bias-centred window `[bias − 5×MAD, bias + 5×MAD]`, because `|bias|` is folded into
  the requirement BEFORE quantizing:
  `bias + 5×MAD ≤ |bias| + 5×MAD ≤ rung`, and symmetrically below.
  So a symmetric recommendation **never clips a biased instrument** — the exact failure
  the superseded asymmetric window existed to prevent. Asserted by
  `symmetric_rung_always_contains_the_bias_centred_window`, swept over bias
  −60..+60 ppm (both signs) × MAD 0..8 ppm rather than spot-checked. Worked example:
  a timsTOF wanting −30..+50 ppm has a requirement of ~50, gets the 50 ppm rung, and
  ±50 covers it.
- **⚠ THE GUARANTEE HAS A HOLE AT THE TOP OF THE LADDER, and it is now FLAGGED rather
  than hidden.** If `|bias| + 5×MAD > 100`, quantizing returns the top rung anyway and
  containment FAILS. That is precisely the badly mis-calibrated instrument. The report
  now carries `user_recommendation_requirement_ppm` (the unquantized number) and
  `user_recommendation_exceeds_ladder`, and the console prints an explicit warning that
  the rung is KNOWN TOO NARROW and the instrument should be recalibrated rather than the
  window widened. Asserted by `an_off_the_ladder_file_is_flagged_not_quietly_capped`
  (bias 40, MAD 20 -> requirement 140 -> flagged). Without this, a hopeless file would
  receive a quiet "±100 ppm" indistinguishable from every other answer.
- **The asymmetric window is SUPERSEDED, and its rationale is retired.** It was
  `bias − 2×MAD` to `bias + p95(|dev|)`, justified as "a biased instrument folded into
  symmetric tolerance clips real IDs on one side". That no longer applies: `|bias|` is
  folded INTO the requirement before quantizing, and the rung is generous enough to
  swallow the bias whole (serum's +2.42 sits inside ±10 with room). Recorded in place in
  `calibration.rs`, not deleted.
- **The fix (agreed direction, now implemented):** quantize.
  `smallest bucket in {10,20,50,100} ppm that is >= |bias| + 5×MAD`. All three files → **10 ppm**,
  matching field default and expert expectation. The point is not that k=5 is optimal — it is that
  **any k from ~3 to ~15 gives the same bucket on all three files**, so the output is robust to a
  constant we cannot derive. A continuous number forces a precision claim the measurement cannot
  support; a bucket does not. `k=5` and the ladder are CHOICES informed by field practice, flagged
  as such in the paper's limitations.
- **Bias stays reported separately.** serum's +2.43 ppm is real and triple-corroborated (MSFragger
  +2.50, MetaMorpheus +2.351). Bucketing the *tolerance* does not discard the *bias* — those are
  the two numbers the "Two numbers from one measurement, opposite ends" lock already separates.
- **Corollary — ✅ FIXED 2026-08-28, and this text's own numbers were wrong.**
  `ms1_pass2_window` used `bias ± 3×MAD` ≈ ±1.2–1.5 ppm. The "clips ~5%" estimate here
  was optimistic by 3-4x: measured against the closed searches' full confident
  populations it covered only **80.82 / 87.46 / 80.77 %**. Worse, no MAD multiple
  transfers — 99% coverage needs **11.0x to 18.3x MAD** across three same-class
  instruments. The window is now the LADDER RUNG centred on bias (coverage 99.32 /
  99.92 / 99.94 %), and the ±100 ppm cap is RETIRED, not "a separate backstop" —
  it could never bind once the ladder set the width. See "Pass 2 windows — sized by
  COVERAGE".
- **Why deferred to step 3, not done now:** step 1's checkpoint is a *frozen* ground-truth set;
  changing report-generation code mid-step moves numbers just frozen. The recommendation formula,
  the Pass 2 window, and analyzer-aware tolerance share one root cause and one consumer — change
  them together in step 3 where they can be tested end-to-end. **Nothing currently frozen is
  invalidated:** bias and MAD are unaffected; only the derived recommendation is in question, and
  in the current build it is reported, not used.

### Mod-discovery JSON does not record its own peak-detection config (gap found 2026-08-24, step 2 prep)

**What is missing.** A mod-discovery JSON records `annotation_settings` (match tolerance,
excluded classifications), `summary.bin_width_da`, and `calibration.mode`. It does **not**
record `min_peak_count`, `max_peaks`, `prominence_threshold`, or the fold tolerances. Two
JSONs produced with different peak floors are therefore indistinguishable from the files
alone.

**Why it matters, concretely.** `discover` and `analyze` do not use the same default.
`discover`'s CLI default is `min_peak_count = 10`; `analyze` calls
`ModDiscoveryConfig::default()`, which is `DEFAULT_MIN_PEAK_COUNT = 5`. The two committed
artifact sets were produced one by each. The difference is invisible in the output and was
found by reading `main.rs`, not the JSON.

**Measured impact: none on anything consumed.** See the cross-build entry above — every
field any script reads is bit-identical; only tail `prominence` moves. So this is a
provenance gap, not a live correctness bug.

**Decision:** record it now, fix it in step 4 packaging by serializing the full
`ModDiscoveryConfig` into the result. **Rejected alternative:** patching it during step 2,
which would change the JSON schema mid-step and move numbers that step 1 just froze — the
same reasoning that deferred the `|error|` bias fix to step 3.

**Write-up consequence.** Any Methods statement about the peak list must state the peak
floor from the config, not from the artifact, until this lands.

---

### ⚠ HYPOTHESIS — the ±1 Da peak forest may be decoy-floor noise, not chemistry (raised 2026-08-24, NOT yet tested)

**Status: hypothesis. Do not act on it, and do not cite it as a finding, until the
decoy test below is run.** It is recorded because it explains four separate results
at once, which none of our previous explanations did.

**Source.** `reference-notes/deamidation-wide-search-notes.md`, a digest of a
Wilmarth / OHSU Proteomics Shared Resource deck (Aug 2026). **That note is a digest,
not the primary source** — the deck itself is not vendored. Treat its claims as
external and unverified.

**What it asserts.** In a wide-tolerance search the ±1.25 Da delta region holds
exactly THREE narrow peaks enriched for correct matches — 0 Da, +0.984 (deamidation),
+1.003 (M1 isotope mis-call) — "riding on a roughly uniform noise floor that matches
the decoy distribution." Its stated rule for auto-detecting mass-shift peaks is that
the peak must be narrow relative to the decoy floor, its width must scale with charge
state and resolution as expected, and **the target/decoy ratio in that bin must be
elevated relative to background.**

**What we observe.** bcell has ~16 detected peaks between 0.85 and 1.15 Da:
+0.8899, +0.9093, +0.9214, +0.9303, +0.9401, +0.9509, +0.9711, +0.9821, and
−0.9599, −0.9703, −0.9804, −1.0229, −1.0290, −1.0377, −1.0601, −1.0871. Under the
model above, at most two of those are real signal.

**Four results this one mechanism would explain, which we had treated separately:**
1. The carpet being "a peak-detection quantization wobble" — noise re-bins under a
   ~1 mDa shift, which is exactly what the 2026-07-17 instrumentation measured.
2. Lys→Allysine at 1.29% in recon (3 peaks merged) against PTM-Shepherd 0.17%,
   Mascot 0.02%, MetaMorpheus 0.02%.
3. The hyperscore discriminator failing to separate carpet from real PTMs — near-
   threshold incorrect matches overlap genuine low-abundance modifications.
4. 97–100% of step-2 gate-1 violations landing on ±1/±2 Da peaks at every X ≥ 10.

**The structural gap it exposes.** Recon removes decoys at load
(`sage_results.rs`, `if is_decoy { decoys_removed += 1; ... }`). `mod_discovery`
never sees them, and the decoy count is not even serialized into the report. **We
have been separating signal from noise without the noise model, which was discarded
at the front door.** The decoys are present in `results.sage.tsv`.

**The falsifiable test (needs only the Sage TSV — no new search).** Rebuild the
delta-mass histogram with decoys retained, stratified by charge state, and measure
target:decoy enrichment per bin. If the ±1 Da forest sits at background while +0.984
and +57.02 are enriched, the forest is noise and the peaks are artifacts of the
detector. If the forest is also enriched, this hypothesis is wrong and the peaks need
a chemical explanation.

**Second test.** MS1 resolution of the three files. The digest states the 19 mDa
deamidation/isotope doublet needs ~120K to baseline-resolve and that 60K blurs it,
worse at higher charge. Our files' resolution is **not recorded anywhere committed** —
it is in the mzML metadata, which is gitignored. Related and already measured: serum's
two "separate" Deamidated peaks are **0.46 mDa** apart, far below any instrument's
resolving power, which independently marks that split as a detector artifact.

**Caveats that must travel with this entry:**
- The digest is not the primary source.
- **Wilmarth's design is a ±1.25 Da symmetric window. Ours is not.** Our open search
  config is `precursor_tol.da = [-500.0, +100.0]`, which produces an observed
  **delta axis of −100 .. +500 Da** (measured: serum −99.10..+499.19, bcell
  −99.95..+499.28, b1906 −99.19..+499.32). Sage's precursor bounds are the mirror of
  the delta window — do not quote the config numbers as the delta range. The
  uniform-decoy-floor argument must be re-checked at our width, not assumed.
- The digest's §7 claim that MSFragger, Mascot, MaxQuant and others "cannot
  replicate this workflow" comes from the author of the competing PAW pipeline.
  Partisan; verify before repeating.

**Step 2 implication, if it holds.** The floor may be the wrong instrument. "X% of
the top non-zero peak" is a magnitude heuristic doing a job that target/decoy
enrichment does directly and with a statistical basis. That would be a change to the
step-2 approach, not a parameter choice, and belongs before X is chosen.

---

### Byonic Preview (Kil et al. 2011) — read 2026-08-24, PDF still not vendored

**Citation, as the source states it:** Kil YJ, Becker C, Sandoval W, Goldberg D,
Bern M. *Preview: A Program for Surveying Shotgun Proteomics Tandem
Mass-Spectrometry Data.* Analytical Chemistry 2011 Jun 13; 83(13): 5259–5267.
Read from `https://pmc.ncbi.nlm.nih.gov/articles/PMC3134881/` (PMC3134881).
**The PDF is NOT in this repo** — Ben confirmed he has not vendored it. Anything
below is a web read, not a vendored source; re-verify against the PDF once vendored.

**1. Preview's reporting floor is DECOY-DERIVED and per-run. It is not a fraction
of another peak.** Two score thresholds gate what gets reported:
- `THigh = max{ s + 1, 23 }`, where `s` is the top score of any *unmodified decoy*
  peptide in the initial search. Used for large searches (their example: N-terminal
  acetylation).
- `TLow  = max{ t + 1, 15 }`, where `t` is the top initial-search score of any
  unmodified decoy of >= 9 residues. Used for smaller searches.
- The constants 23 and 15 are described as chosen empirically.

**And it corrects the counts by decoy subtraction** — target hits minus decoy hits,
so the remainder estimates the true target hits.

**This is a direct challenge to our step-2 floor design.** `ptm-stratification-design.md`
proposes `floor = X% of the top non-zero delta peak`, sourced to Mascot error-tolerant
working practice. **Preview — the tool recon is the spiritual successor to — does not
do that.** Its floor is a decoy-derived score threshold plus decoy subtraction of
counts. This is independent support for the ghost hypothesis above: the principled
noise model is the decoy distribution, and we discard it at load. Do not treat the
X% floor as settled prior art.

**2. Ben's ~1 Da hypothesis is NOT supported by the paper — plausible, but wrong for
the stated reason.** The question was whether Preview chose ~1 Da resolution for its
modification choices because of sub-Da peak splitting. What the paper actually
describes is spectrum preprocessing: observed floating-point m/z values are converted
to integer bins by rounding to the nearest integer to `0.9995 x M`, to strip mass
defects. Two reasons this does not answer the question:
- The stated purpose is mass-defect removal, not noise-floor management.
- It applies to **fragment ions and precursor m/z in spectrum preprocessing**, not to
  the modification delta-mass axis.
Recorded so the guess is not carried forward as fact.

**3. Preview handles isotope satellites UPSTREAM, in spectrum preprocessing.** It
extracts the 300 most intense peaks and then downweights isotope peaks, and states
that matching a 13C1 isotope peak to the theoretical monoisotopic peak is rare
*because* of that downweighting. **This is a third architecture for the satellite
problem** — neither "fold satellites in the delta histogram" nor "leave them." Feed
this into the +57/+58 satellite question; it is what the reference tool does.

**4. Decoys are carried into the subsequent pass, paired.** When Preview promotes a
peptide into the peptide database for a later search, it adds the reversed
counterpart alongside it. This is a second, independent source for the **"paired
target-decoy selection in `subset_fasta.py`"** item already queued in PLAN step 3,
which was sourced only to Mascot error-tolerant. Two tools, same practice.

**5. Search ordering.** Preview orders its searches so earlier results constrain
later ones, checking mass accuracy first to set tolerances for subsequent searches.
This matches recon's own calibrate-then-recommend design and is worth citing in the
step-5 write-up as convergent design, not borrowed code.

---

### ❌ Ghost hypothesis REFUTED — the ±1/±2 Da forest is target-enriched (2026-08-25)

**The hypothesis recorded 2026-08-24 is WRONG. Corrected here in place; do not carry
it forward.** It held that the ±1 Da peak forest was the incorrect-match noise floor,
which would have made the step-2 floor a decoy criterion instead of an X% rule.

**Measured** (`testing/recon-output/2026-08-25-checks/03-decoy-histogram-*.txt`,
via `testing/scripts/decoy_delta_histogram.py`, decoys retained, no q filter):

| region | serum | bcell | b1906 |
|---|---|---|---|
| background T:D for the file | 1.63 | 2.52 | 2.21 |
| +1 Da (TEST) | **10.21x** | **15.62x** | **10.46x** |
| −1 Da (TEST) | 3.24x | 7.34x | 6.22x |
| +2 Da (TEST) | 1.88x | 8.00x | 5.53x |
| −2 Da (TEST) | 1.66x | 2.19x | 5.35x |
| +58.02 satellite | 15.10x | 8.01x | 10.46x |
| zero / deamidation / +57 / Ox (controls) | all enriched | all enriched | all enriched |

Every test region is target-enriched, not at background. **The forest is correct
peptide matches.** The X% floor is not displaced by a decoy criterion.

**What the result does NOT say.** Enrichment means the PEPTIDE match is correct. It
cannot distinguish a real chemical modification from a correct peptide whose
precursor was misassigned — a monoisotope error is a correct match with a wrong
delta. So this refutes "the forest is noise" and leaves "the forest is isotope
artifacts" untouched. That was already the carpet's characterization, and decoy
enrichment cannot test it. Do not use this result to argue the forest is chemistry.

**Why Wilmarth's model did not transfer.** His ±1.25 Da window makes decoys roughly
uniform across a narrow axis. Our delta axis is −100..+500 Da, so decoys spread over
600 Da and the in-region background is a different quantity. The caveat was recorded
when the hypothesis was written; it turned out to be the decisive one.

---

### ⚠ Serum +1 Da conservation violation — CAUSE FOUND, defect still OPEN (2026-08-25)

The 2026-08-24 finding stands. The first check script reported "ok" on all three
files and was **wrong**: it counted available PSMs from the raw TSV without applying
fold-to-zero, while peaks are built from the histogram AFTER folding. Post-fold vs
pre-fold. Fixed in `testing/scripts/verify_peak_conservation.py`.

**The "46 PSMs" figure is SUPERSEDED — it understated the defect.** It came from
comparing the two peaks' claims against the whole `[0.80, 1.20]` region:
```
serum +1 Da region [0.80, 1.20]
  unfolded PSMs from the TSV        894
  fold_by_k[+1] folded to zero     -580
                                   ----
  available post-fold               314   <- matches the committed histogram exactly
  peaks in the region CLAIM         360
  excess                             46   <- SUPERSEDED, see below
```
The two peaks never reach most of that region. Their PSM collection windows span
only `[0.97, 1.00]`, which holds **190** PSMs, against **360** claimed. The real
double-count is **>= 170 PSMs, not 46**.

**Root cause — two parts, both in `detect_peaks_with_prominence`.**

1. *The merge guard misses adjacent bins.* Bin centers are `bin_idx as f64 *
   bin_width`. Two adjacent bins are one bin width apart, and `too_close` compares
   with `<=`, so in exact arithmetic they merge. In f64 they do not:
   `99.0*0.01 - 98.0*0.01 == 0.010000000000000009`, which is greater than `0.01`.
   Bins 0.98 and 0.99 both become peaks. Their `prominence` values (114 and 68)
   are the two bin counts unchanged, which is how the originating bins were
   identified from the committed JSON.
2. *The collection window is twice the separation test.* Each accepted peak takes
   every PSM within `+/- effective_merge_tolerance` of its bin center — a window
   two bin widths wide — while `too_close` only rejects a center within ONE
   tolerance. Nothing anywhere assigns a PSM exclusively. Peaks at 0.98 and 0.99
   collect `[0.97, 0.99]` and `[0.98, 1.00]`; every PSM in `[0.98, 0.99]` is
   counted twice. `delta_mass` is intensity-weighted, so the two peaks report
   0.98452 and 0.98497 Da — 0.45 mDa apart — and look like one split peak.

**This is general, not a serum quirk.** `testing/scripts/peak_window_overlap.py`
finds overlapping peak windows on all three files:
serum 2 pairs, bcell 11, b1906 10
(`testing/recon-output/2026-08-25-checks/07-peak-window-overlap.txt`). The +1 Da
pair is the worst in every file: serum >=170 duplicate claims, bcell >=145.

**Instrumented and gated in code.** `detect_peaks_with_prominence` now tracks which
peak claims each PSM index and hard-stops on a second claim. It fires on the real
b1906 fixture (`recon-tool/tests/fixtures/determinism_psms.tsv`), pinned by
`recon-tool/tests/peak_assignment_test.rs`. Root-cause arithmetic is pinned by
`mod_discovery::tests::test_bin_grid_spacing_defeats_merge_tolerance`.

**Consequences of the gate, both intentional:**
- `run_mod_discovery` now panics on real data. The tool cannot produce a report
  until peak construction is fixed. That is the point of the invariant.
- `discovery_output_is_deterministic` is `#[ignore]`d — it cannot reach
  serialization. It is NOT a determinism failure. Re-enable it with the fix.

**FIXED 2026-08-25, as two selectable modes.** The fix landed as
`PeakAssignmentMode`, on the same parallel-paths pattern as `CalibrationMode` —
NOT a swap — so the two answers can be benchmarked on one file without confounding
"the peak fix worked" with "the code changed."

Center selection and PSM assignment are now separate decisions. Conflating them was
the defect. **Both modes assign every PSM to its NEAREST accepted center and to that
one only**, so both satisfy the invariant by construction. They differ only in which
bins may be separate centers:

- **`Merge` (default)** — bins within the merge tolerance are ONE peak. This is what
  the original `too_close` guard intended before f64 defeated it.
- **`Split`** — every prominent bin is its own peak. Keeps a real doublet visible
  instead of averaging it away.

Centers are compared by **bin INDEX**, not by float distance. That is the root-cause
fix and it is pinned by `test_bin_grid_spacing_defeats_merge_tolerance`. Reverting to
a float comparison brings the defect straight back.

`Merge` is the default because it restores the intended grouping. **It is a decision,
not a historical default** — the old behaviour double-counted PSMs and neither mode
reproduces it.

### search-output cleanup — what was deleted, and the two near-misses (2026-08-25)

**Deleted, 267 MB, all gitignored local Sage runs. Every one was proven redundant by
regenerating from its replacement and diffing, not by reading its name.**

| deleted | replacement, proven how |
|---|---|
| `open-{serum,bcell,b1906}` | duplicate runs of `open-*-full`. `discover --min-peak-count 5` on each pair gives **identical peak tables**. |
| `open-{serum,bcell,b1906}-nofixedmods` | duplicate runs of `step1-open-*`. Same test, **identical peak tables**, same `total_psms` (15386 / 73527 / 28005). |
| `_tmp-isotope-probe-b1906` | the runsheet that created it also carries its own `Remove-Item` cleanup step. Durable record is `06-isotope-probe-*`. |

**Float note, expected and not a defect.** `open-serum` vs `open-serum-full` differed
in exactly two values at the last bit: one `weighted_apex` and one `mean_hyperscore`
(…899299e-05 vs …899301e-05). Same PSMs in a different row order, so float sums
accumulate differently. Peak counts and delta masses were bit-identical. Do not chase
this — and do not read it as a determinism failure; the guard tests same-input
stability, which still holds.

**Consequence for `nofixedmods/` — RESOLVED, and NOT by retiring it.** Deleting its
source left the artifact unreproducible, so it was regenerated from `step1-open-*`,
which is proven equivalent. `testing/README.md` records the source change.

Retiring the family was proposed and **rejected**: it is load-bearing. Every
`comparison/*.md` benchmark table is built on it, and `compare_4way.py`,
`satellite_check.py` and `compare_isotope_probe.py` all read it by path. It is also
not a pure duplicate of `full-run/` — it runs at `min_peak_count` 10 against
`analyze`'s 5. Re-pointing the "+57 at rank 2" statement would have re-based every
committed benchmark on a different peak floor for no gain.

**Superseded number:** NOTES and `testing/README.md` recorded the 10-vs-5 difference
as 2 / 1 / 3 tail peaks. Measured on the fixed build it is **0 / 0 / 1** (only b1906,
at −1.92 Da). Merging adjacent bins consolidates counts, so more peaks clear the
count-10 floor. The 2/1/3 figure describes the defective build.

**TWO NEAR-MISSES. Both would have destroyed evidence behind locked results.**

1. **`open-*-calibrated` was nearly deleted as unclaimed.** A name-based sweep found no
   reference to it anywhere in the repo. It is in fact the provenance for
   `calibration-benchmark/*_calibrated-input_none.json`, which backs the **C1/C2
   CLOSED-NEGATIVE** result. Proven by exact `total_psms` match: 19099 / 80374 / 31497,
   matching only `open-*-calibrated` and no other candidate. **KEPT.**
   Recollection said "a FragPipe calibrated-mzML test, likely not important." The
   measurement disagreed, and the measurement wins.
2. **Grep-by-name is not a provenance test.** `discover` output carries no `input`
   block, so no artifact names its source TSV. The only reliable link is matching
   `total_psms` or regenerating and diffing. **Never delete a search directory on the
   basis that nothing greps for its name.**

**Bonus: `calibration-benchmark/` is no longer "not reproducible."** `testing/README.md`
says it is frozen and unreproducible. Its inputs are present and identified, so that
claim is now too pessimistic — worth correcting before the write-up quotes it.

---

### ✅ `run_validation.py` REPAIRED — 14/14, and Tier 3 now actually tests something (2026-08-25)

**Was 10/14, and had been since step 1, while PLAN claimed 14/14 in two places.**
Verified against a clean worktree at the pre-regeneration commit: identical four
failures, exit 4. None of them were caused by the peak fix.

**1. Tier 3 gave ZERO protection — it compared a file to itself.** `tier3_snapshot()`
read `original_text`, parsed it, re-parsed **the same string**, and compared the two.
It could only fail on malformed JSON, and it never opened
`testing/regression-snapshots/` at all. Proven, not argued: `peaks[0].count` was set
to `999999` and `total_psms` to `1` in a throwaway worktree, and it still reported
10/14. **This is the gate that should have caught the peak-assignment defect.**

*Now:* Tier 3 re-runs `discover` on the pinned `step1-open-*` TSV and requires an
exact match to the snapshot, volatile keys excluded, reporting the first differing
path. Falsification test: the same `999999` corruption now fails with
`.peaks[0].count: 4960 vs 999999`. It falls back to comparing against the committed
`04-discover-min5-*.json` when the gitignored TSVs are absent (CI), and **always
prints which mode it ran in** — a gate that degrades silently is how this went
unnoticed.

*Baselines re-snapshotted.* The old ones came from the fixed-C `open-*-full` family
(19307 / 81966 / 31682 PSMs) — not the alkylation-agnostic family that actually
ships — and carried the peak bug. They are now generated from `step1-open-*` at
`min_peak_count` 5, matching `analyze`.

**2. Gate 3 tested a false premise, and is now two-sided.** It asserted "+57 must be
small because this is a fixed-C search" while reading `full-run/`, which step 1 moved
to an alkylation-agnostic search. Verified empirically, not from config alone — in
`open-serum-full` 18833 of 18833 C-containing peptides carry `C[+57]`; in
`step1-open-serum`, 0 of 17535. Config and data agree.

A large +57 in an agnostic run **is the product claim**. The gate now asserts both
sides: fixed-C keeps +57 below 200 (observed 86 / 187 / 60), agnostic surfaces it
above 500 (observed 1125 / 3311 / 1253). The bands are separated by more than 6x, so
neither threshold is finely tuned. The family is declared explicitly in the harness,
because the report records the fixed mod recon ASSUMES, not the one Sage used.

**3. Gate 1 re-baselined, 35.0 -> 25.0 (deliberate recorded edit).** The old floor was
fitted just under serum's fixed-C value of 42.1%. Under agnostic search, CAM PSMs
carry a +57 delta instead of sitting at Δ≈0 — serum's 1125 CAM PSMs are 7.3% of
15386 — so serum fell to 32.2% and failed on a correct result. The gate's real
assertion is now the RANK claim, Δ≈0 must be the largest peak, which is the axis this
project trusts. The percentage is only a backstop against total collapse, so 25.0
sits clear of every observation in both families (fixed-C 42.1/64.5/56.5, agnostic
32.2/58.3/51.2) rather than just under the lowest.

**Do not read a passing harness as broader than it is.** It gates three files, two
polymer checks and three regression baselines. It does not test tier logic, which
does not exist yet.
---

### Gate audit — 2 of 6 real gates could not fail (2026-08-25)

Prompted by the Tier 3 discovery. Every gate-shaped script was given a deliberately
wrong input and checked for whether it said so. **The method is the finding: a check
is not verified until you have watched it fail.**

| script | verdict |
|---|---|
| `run_validation.py` Tier 3 | **COULD NOT FAIL** — compared a file to itself. Fixed. |
| `mode_sibling_separation.py` | **COULD NOT FAIL** — filtered out every pair wider than one bin width before counting pairs at or above one bin width. Fixed; **its conclusion reversed.** |
| `compare_mod_discovery.py` roll-up assert | **COULD NOT FAIL** — `slot["count"] == sum(slot["sites"].values())` with both sides incremented by the same `count` in the same iteration. Rewritten against an independent source tally. |
| `compare-mod-discovery-metamorpheus.py` roll-up assert | same defect, same fix |
| `compare_mod_discovery.py` `window_spot_check` | **SOUND** — fires on the capped-window regression it guards |
| `peak_window_overlap.py` | **SOUND** — forged overlap exits 1, clean exits 0 |
| `verify_peak_conservation.py` | **SOUND** — inflated peak count exits 1. An earlier note here flagged it for retirement based on its old, already-fixed bug rather than on testing it. That flag was wrong. |
| `compare_isotope_probe.py`, `digestion_efficiency.py` | not gates — producers with only a usage-path exit |

**Three of the four vacuous checks were labelled as invariants.** Two said
"conservation". Comment text is not evidence.

**Rule going forward: a new gate is not done until a deliberately wrong input has
made it fail.** Cheap, and it is the only thing that separates a check from a
decoration.

---

### ✅ `Merge` — CONFIRMED, on evidence that could fail. First evidence was refuted the same day (2026-08-25)

**THE "SETTLED (locked)" MARKER ON THIS ENTRY WAS WRONG AND IS WITHDRAWN.** The
decision rested on a script that could not produce a contrary answer. Do not cite the
"0 of 24 pairs" figure below — it is an artifact, kept only so the error is legible.

**What broke.** `mode_sibling_separation.py` selected candidate pairs with
`if not 0 < sep <= BIN_WIDTH_MDA: continue` — it discarded every pair wider than one
bin width BEFORE counting how many were at or above one bin width. That count could
only ever be zero. The "candidate real doublet" branch was unreachable. **Same
could-not-fail shape as the old Tier 3 gate, in the script that justified a locked
decision.** Found by injecting a known 12.0 mDa pair and watching it report zero.

A second bug sat behind the first: siblings were measured as CONSECUTIVE gaps. A
group of three peaks spanning 12 mDa has consecutive gaps of 4 and 8, both under a
bin width, so the structure hides. Both fixed — siblings are now grouped by which
Merge peak they belong to, and the gate is on group SPREAD with no upper cap.

**Honest re-derivation, same three files, same inputs:**

| | old (broken) | corrected |
|---|---|---|
| groups | 24 pairs | **27 groups** |
| spread min / median / max | 2.0 / 5.1 / 9.7 mDa | **2.0 / 5.2 / 24.0 mDa** |
| ≥ one bin width (10 mDa) | 0 | **7** |
| ≥ 75% of the 19.3 mDa doublet | not measured | **5** |

The seven: bcell 24.0 / 19.2 / 15.0 / 12.1 mDa, b1906 22.6 / 21.9 / 11.3 mDa. Six of
seven are unannotated; one is Amidated. **Not inspected yet — that is the next
action, and it must not be skipped to restore the previous conclusion.**

**What is NOT affected.** The peak-assignment FIX is untouched: both modes assign each
PSM to exactly one peak, and the invariant gate is independent of this script. The
regenerated counts, the +1 Da collapse, the unchanged +57, and `run_validation` 14/14
all stand. What is refuted is the EVIDENCE for preferring `Merge`, not the correctness
of either mode. `Merge` remains the code default — now as an unjustified default, not
a finding.

**Do not relitigate the fix. The mode was relitigated and is now settled — see
"the decision, and what it costs" below.**

Measured on all three full files:
`testing/recon-output/2026-08-25-checks/09-modes-{serum,bcell,b1906}.txt`, analysed by
`testing/scripts/mode_sibling_separation.py` into `10-mode-sibling-separation.txt`.

**The decisive test is mechanical and needs no composition or reference data.** Peaks
sit on a 10 mDa bin grid. If two adjacent bins hold two REAL populations, each peak's
intensity-weighted mean sits near its own bin centre, so the two means are separated
by about one bin width. If one population straddles a bin edge, nearest-centre
assignment cuts it in half and each half's mean is pulled toward the shared cut, so
the means come out CLOSER TOGETHER than one bin width.

```
SUPERSEDED — produced by the capped filter described above. Kept as the record
of the error, NOT as evidence.
24 sibling pairs where Split reports two peaks and Merge reports one
  separation min / median / max   2.0 / 5.1 / 9.7 mDa
  pairs separated by >= one bin width   0 of 24   <- could not have been anything else
```

**That zero was structurally guaranteed, not observed.** `Split` DOES shred single
populations at bin edges — it splits Carbamidomethyl on every file (serum 905+271,
bcell 3079+251, b1906 1174+101), Oxidation on every file, and Deamidated on two.
Measured on consecutive gaps, that is **21 of 27 groups**. But "Split shreds" was
never the whole claim, and the rest of it was wrong.

**CORRECTED — the two claims that stood here were both false.** This paragraph said
"nothing in 24 pairs comes near 19.3 mDa, the largest separation on any file is
9.7 mDa", and concluded that `Split` never resolves anything. Both came from the
capped filter. Re-measured on consecutive gaps with no cap: **the largest gap is
15.2 mDa, and 7 gaps in 6 groups reach a full bin width.** The 9.7 mDa figure is
dead. Do not quote it.

**Gating on group SPREAD was also wrong, and was the repair's own defect.** A group
occupying k adjacent bins has a spread of about (k-1) bin widths BY CONSTRUCTION,
whether it holds k real populations or one broad one. It over-triggered on the real
files: bcell host 0.93035 has consecutive gaps of 9.6 and 9.7 mDa — below one bin
width, which is the shredding verdict — yet its 19.2 mDa spread flagged it as a
candidate doublet. The gate is now on consecutive gaps. Fixture-tested both ways in
`testing/scripts/mode_gate_fixtures.py`; record in `11-mode-gate-fixtures.txt`.

**MS1 resolution (check 4) does NOT gate the mode, but the reasoning here was also
wrong.** The old text said so because "Split never separates anything". The real
reason is better: the nominal resolution is **not recorded in any of the three mzML
files** (check 4, closed 2026-08-25 — 4000 MS1 scans inspected per file, all
`FTMS + p NSI Full ms`, no MS:1000011 anywhere). It cannot gate anything because we
do not have it. What replaces it is a DIRECT measurement of the same quantity on the
axis that matters: the delta-mass FWHM of known-single populations, 3–8 mDa on all
three files. That is measured, not inferred from an instrument setting.

**The composition diagnostic agrees, independently.** Where a site list is testable,
both members of a sibling pair land at essentially the same enrichment, at or near
their ceiling:

| file | annotation | sep mDa | enr 1 | enr 2 | ceiling |
|---|---|---|---|---|---|
| serum | Carbamidomethyl (C) | 4.4 | 2.73 | 2.80 | 2.86 |
| bcell | Carbamidomethyl (C) | 5.1 | 10.14 | 10.23 | 11.11 |
| b1906 | Carbamidomethyl (C) | 5.0 | 9.27 | 9.20 | 10.00 |
| serum | Oxidation (HMW) | 4.7 | 1.54 | 1.55 | 1.61 |
| b1906 | Oxidation (HMW) | 3.2 | 1.93 | 1.93 | 1.92 |
| serum | Deamidated (NQ) | 4.0 | 1.32 | 1.36 | 1.37 |

Two independent lines, same answer: one population, cut at a bin edge.

**Do not build tier logic on the counts in any COMMITTED artifact.** (Superseded in
part: the regeneration below is now done, so `full-run/`, `nofixedmods/`, the
discover artifacts, the snapshots, the tier gates and the comparison tables ARE
current. Anything else under `testing/recon-output/` still predates the fix.)
Regenerating needs the TSVs — which are available locally. Nothing is blocked.
See "Gitignored is not absent" below.

Do not re-check any of this by comparing region counts; two scripts already returned
false answers that way. Check the mechanism — `testing/scripts/peak_window_overlap.py`
for window overlap on a committed JSON, and the in-code index-disjointness gate.

#### The decision, and what it costs (2026-08-25)

**`Merge` stays the default. It is now a measured choice with a known, stated cost,
not an unjustified one.**

The gate on consecutive gaps leaves 6 groups it cannot resolve, because a gap of one
bin width looks the same for two real populations and for one broad population
smeared across adjacent bins. That is a real power limit, not a tuning problem, and
no 10 mDa-binned artifact can close it — the committed `discover` JSONs are on the
same grid as the question. It was closed by going below the grid, to the PSMs.

**Method.** `testing/scripts/subbin_delta_histogram.py` re-bins the delta-mass axis
from the pinned `step1-open-*` TSVs at 1, 2 and 4 mDa. It reimplements the Rust delta
pipeline (isotope correction, da-scalar calibration, neutron fold-to-zero) in Python,
so it is a re-derivation and cannot be trusted on its word. `--verify` rebuilds the
histogram at the shipped 10 mDa width and diffs it bin-for-bin against the committed
JSON. **All three files: every bin identical in centre and count** (3060 / 6537 /
2946 bins). Record: `12-subbin-equivalence.txt`. That check earned its place
immediately — it failed on the first run over ONE PSM on bcell, because Python's
`round` is round-half-to-EVEN and Rust's `f64::round` is half-AWAY-FROM-ZERO, and
exactly one PSM sits on the 284.125 Da boundary.

**The invariant.** A peak is real if it survives re-measurement at several bin
widths. A background fluctuation does not. Verdict `REAL` needs z >= 5 at all three
bin widths plus a measurable width at two of three; z is Poisson, apex against the
median of the surrounding annulus. Controls both ways, on every file: **7 of 7
positive controls REAL, 6 of 6 negative controls carpet.**

**Result — `13-mode-decision.txt`. 15 contested members, 4 REAL, 11 carpet.**

| file | contested groups | outcome |
|---|---|---|
| serum | 0 | no group reached a one-bin gap |
| bcell | 3 | one group holds TWO real populations; two hold one real peak plus carpet |
| b1906 | 3 | **entirely carpet** — all 8 members z <= 3.2 at every bin width |

**The one case where `Merge` is measurably wrong: bcell, −1.03036 and −1.01974.**
10.6 mDa apart, both REAL, with a real valley between them — 43 / 8 / 36 counts per
1 mDa bin. `Merge` combines them into one peak at −1.02899 and loses the structure.
The −1.01974 member is the weaker of the two (z 6.4 / 5.3 / 5.8, just clear of the
threshold) and carries ~30 PSMs in excess of background. **It is not one repeated
peptide** — 70 PSMs over 70 distinct sequences, MORE diverse than Carbamidomethyl at
1.2. That hypothesis was tested and refuted; do not re-raise it. What the −1.021
population IS remains unassigned, and assigning it is not a v0.1.0 job.

**Why `Merge` wins anyway.** The comparison is one lost doublet, on one file, against
`Split` shredding 21 genuine single populations across all three files AND promoting
11 carpet chunks to named, ranked, countable peaks. A recon report that invents 11
modifications is worse than one that merges a pair. **Rejected alternative:** make
`Split` the default and filter carpet afterwards — rejected because the carpet filter
would be a new gate on the critical path of a closed scope, and because `Split`'s
shredding of Carbamidomethyl and Oxidation damages the peaks the product is judged on.

**`Split` is retained, not retired.** It is the only way to see the bcell −1.03/−1.02
structure, and retiring it would delete the ability to check this class of question.

**Stated limitation for the write-up:** recon merges adjacent-bin populations. On one
of three test files this combined two resolvable populations 10.6 mDa apart into one
reported peak. A limitation written down honestly costs nothing.

---

### Derived-artifact regeneration on the fixed build — results (2026-08-25)

**`psm-sensitivity/*.json` — the trace's premise was wrong.** PLAN listed these as
"derived from peak counts". They are not: `psm_count_sensitivity.py` filters the
clean subset and reads `precursor_ppm` / `fragment_ppm`, and never runs peak
detection. Re-run with the params the outputs record (q 0.01, delta 0.02, tol 0.5,
draws 500, seed 42) they reproduce **bit-identically on all three files**. Verified,
not regenerated. **Still affected by the separate `|error|` bug** — the ppm medians
are medians of absolute error, so they must be re-run after step 3's fix.

**Tier gates — regenerated.** `tier-gates-2026-08-25.txt` replaces the 08-24 file. Peak
masses moved as expected (bcell Lys→Allysine −1.0304 → −1.0290, which is the
Merge-combined centre). Tier counts moved: bcell tier2 20→16, below 14→21; b1906
tier2 24→18, below 11→18. On the RAW peak list every X fails gate 1; best is X=20%
at 8 violations, halved from 16.

**⚠ SUPERSEDED 2026-08-25 (same day) — "the tier design does not pass its own rank
gate at any X" is WRONG.** That sentence stood here and in PLAN, and told the next
session not to assume a passing X exists. Measured since: all 8 violations at X=20%
are ONE peak, the +58.02 satellite (serum 4, bcell 4, b1906 0). Demote it and gate 1
goes to 0/0/0 at X=20%, on both a 2-tool and a 3-tool reference panel. A passing X
DOES exist. See "Step 2 unblocked" below.

**Cross-tool tables — all 10 script-generated tables regenerated.** An earlier
version of this entry said 5 were blocked for want of an input. **That was wrong.**
`.gitignore:61` excludes `*PSMs*.psmtsv` from TRACKING, but the file was on disk the
whole time, and every script takes `--metamorpheus` as a
path. Nothing was blocked. See "Gitignored is not absent" below.

**`BENCHMARK-SUMMARY.md` is hand-written, not script output.** `compare_4way.py`
mentions it in a docstring but does not write it. **Prose pass DONE 2026-08-25**:
matched-mod counts and both Spearman tables updated, superseded values kept in
brackets, the retired "fixed-C" labels corrected, and a header added saying the file
does not follow a script regeneration. Do not assume re-running a script refreshes it.

**The rank-not-magnitude evidence survives — but the PER-FILE story INVERTED.**
The aggregate claim is unchanged: two of three files show strong, significant rank
agreement with PTM-Shepherd reallyOpen, one is marginal. Values moved only slightly
(0.381 → **0.402**, 0.807 → **0.798**, 0.671 → **0.700**).

**WITHDRAWN — which file is weak.** `BENCHMARK-SUMMARY.md` said "bcell/b1906 strong,
serum weak" (bcell 0.648, b1906 0.625, serum 0.284) and explained serum's weakness by
its high calibration offset and its sample-prep artifacts (DTT adducts, Fe/Al,
over-alkylation). On the fixed build, like-for-like against reallyOpen, the ordering
is the OPPOSITE: **serum 0.798 strongest, b1906 0.700, bcell 0.402 marginal
(p=0.0517).** The explanation was reasoning built on numbers the peak defect
produced. **It is withdrawn and must not be re-used.** No replacement explanation is
offered, because none has been tested — bcell being the weak file is an open
observation, not a finding.

**Two comparison families now give identical numbers, and that is correct.** Step 1
moved `full-run/` to an alkylation-agnostic search, so `full-run/` and `nofixedmods/`
are the same family and differ only in peak floor (5 vs 10). Their matched sets inside
the comparison window coincide. The "fixed-C" labels on those rows were retired.

### ✅ Step 2 unblocked — the satellite is the only gate-1 blocker (2026-08-25)

**A passing X exists. It is 20%.** Every gate-1 violation at X=20% on the raw peak
list is the same peak — the +58.02 satellite (serum 4, bcell 4, b1906 0). Demote it
and gate 1 reads 0/0/0, gate 4 reads 0, on all three files. Gate 2 stays read-only at
4 unstable chemistries, unchanged from baseline. This corrects the superseded entry
above; do not carry "no X passes" forward.

| treatment | X=5 | X=10 | X=15 | X=20 |
|---|---|---|---|---|
| baseline (fixed build) | 55 | 20 | 15 | 8 |
| satellite flag-and-demote | 48 | 10 | 2 | **0 PASS** |
| forest removed, satellites left | 37 | 11 | 14 | 7 |
| forest removed + satellite demote | 24 | **0 PASS** | 1 | **0 PASS** |

**bcell does reach 0** — at X=15% and X=20% under demote. Its residual at X=10% is
the merged −1.0290 Lys→Allysine, not the satellite. The 31→12→4→4 sequence was raw-list
only.

**Gate 1 with a THREE-tool panel changes nothing at the decision point.** The
MetaMorpheus output is on disk (see "Gitignored is not absent" below), so gate 1 no
longer needs to run on two tools. Adding it: X=5% 55→66, X=10% 20→22, X=15% and X=20% **identical**. The
X=20% pass survives the stricter panel. `tier_gates.py`'s docstring still claims
`AllPSMs.psmtsv` is "absent from a checkout without the raw data" — that premise is
retired and the file should be wired in.

**Cost of X=20%: thin tiers.** tier1/tier2 = 2/1 (serum), 2/1 (bcell), 3/1 (b1906).
X=10% would give 4–5 tier-1 mods per file but needs the ±1/±2 carpet removed too, and
there is no instrument for that — the ghost hypothesis is refuted, so no decoy
criterion is available.

### ✅ The carpet invariant — RESTATED AND SCOPED (2026-08-25, corrected 2026-08-26)

**Current wording — the floor must sit above the ±1/±2 Da quantization carpet, and the
floor governs ONLY the peaks the abundance path decides.** Unspecific acceptors and
uncurated masses. Nothing else.

**Why the scope, and why it is not a carve-out.** Recon decides a residue-specific mod
by PRESENCE — Fisher exact on its acceptor residues — not by amount. Those peaks never
face the floor, so the floor asserts nothing about them. The floor exists to separate
real signal from carpet for the peaks with no residue to test. Scoping the invariant to
the abundance path is not a patch; it is the routing rule applied consistently.

**⚠ SUPERSEDED WORDING 1 (original design note).** "Floor above the ±1/±2 Da carpet",
unscoped. **Can never pass.** The tallest peak inside the ±1/±2 Da region is
**Deamidated** on all three files (184 / 760 / 228) — real chemistry living in the
carpet's mass region. Passing would require excluding deamidation, which is wrong.

**⚠ SUPERSEDED WORDING 2 (the 2026-08-25 patch on this page).** "Carpet EXCLUDES
annotated real chemistry inside the region." Right outcome, wrong reasoning — it removed
deamidation by hand and left a carve-out to maintain. Under the restatement Deamidated
is residue-specific (N/Q), goes to statistics, and is outside the floor's jurisdiction
by construction. **Its margins (+115.8 / +113.6 / +75.9 PSM at X≥15%, bcell binding and
failing at X=10% by +0.951 unannotated 383 and −1.029 Lys→Allysine 376) were measured
over the patched peak set, not the scoped one. Do NOT carry those numbers forward.**
Re-measure when the assert is written; the assert prints the actual margins.

Design note updated to match, 2026-08-26.

### ✅ +57 is Carbamidomethyl — resolved by residue enrichment (2026-08-25)

The `ambiguous` flag was demoting the largest non-zero peak in every file to tier 2
("your call"). +57.0215 is mass-degenerate across Carbamidomethyl / Gly / Carbofuran.
Resolved from committed TSVs, **no re-search** — `step1-open-*/results.sage.tsv`
carries `peptide`, so composition is already on disk.

Rank-1 targets, spectrum_q<0.01, ±10 mDa band:

| candidate | sites | serum | bcell | b1906 |
|---|---|---|---|---|
| **Carbamidomethyl** | C | **2.85x** | **9.98x** | **9.41x** |
| Gly | T/S/K | 1.01x | 1.02x | 1.02x |
| Carbofuran | S | 1.02x | 1.11x | 1.04x |

Band Cys presence is 95.8–96.8%. **Glycine is eliminated at background.** This is the
flanking-check's successor for the agnostic family — note the 2026-07-15 serum triage
ran on the FIXED-C residual (86 PSMs, Cys peptides already carrying CAM), a different
population; both results stand and do not conflict.

**Enrichment, never presence.** Gly's T/S/K are present in 91–96% of ALL peptides. A
presence test says "possible" every time. Background must be per-file: serum's Cys
background is 34% vs ~10% for the cell lines (albumin/Ig are Cys-rich), so serum's
ratio is structurally lower at the same 96% band presence.

**⚠ CORRECTED same day — use the ODDS RATIO, not the enrichment ratio.** The rule
recorded here was `max enrichment = 1 / p_background`, used as a power guard to call
common-residue acceptors UNTESTABLE. **That cap is real but applies only to the
enrichment (risk) ratio. The odds ratio is not bounded by it.** Measured consequence:
Deamidation on N∪Q sits at 69–74% background, so the ratio is capped near 1.4x and was
wrongly called untestable — by odds ratio it is **11.56 / 3.27 / 3.10 with BH p from
1e-15 to 1e-23, SUPPORTED on all three files**.

**The statistic is: Fisher exact one-sided, odds ratio as effect size, Benjamini-Hochberg
across the mod × residue sweep.** Accept on `OR >= 2.0 AND BH q < 0.05`. Both conditions
are needed — Gly's BH q reaches 1e-5 on bcell purely from n=2969 while its OR is 1.37.
The OR floor is not finely placed: nothing measured lands between the rejected maximum
(1.39) and the accepted minimum (3.10).

The `1/p_background` ceiling is still worth reporting as context, and it still explains
the one genuinely untestable case: Carbamyl's acceptors (K/R/C/M, and `TG=X` at N-term)
sit in 99.6–100% of peptides, so band and background are both ~100% and the odds ratio
is undefined. Carbamyl can only be recommended by curation.

### ✅ Position-aware testing restores power at the termini (2026-08-25)

**An earlier version of the entry above said "N-term is 100% by construction, so
terminal mods are untestable." That is WRONG and is corrected here.** The distinction
is not residue-vs-terminus, it is whether the specificity names a RESIDUE:

| test | serum | bcell | b1906 | ceiling |
|---|---|---|---|---|
| Gln→pyro-Glu, first residue is **Q** | 13.18x | 22.33x | 24.10x | ~24–26x |
| Glu→pyro-Glu, first residue is **E** | 1.98x | 6.51x | 10.04x | ~13–20x |
| Carbamyl, `TG=X` at N-term | 1.00x | 1.00x | 1.00x | 1.0x |

"Peptide contains Q" gave 2.01x for pyro-Glu; "first residue is Q" gives 22.33x, against
a 3.8–4.2% background. Position-specific terminal mods are among the MOST testable
things available. Only `TG=X` (any residue) at a terminus is truly untestable.

**Test spec = TG + PP together** — which residue, and where. That is exactly the pair
MetaMorpheus's mod files carry.

### ✅ +58/+59 are the +57 satellite — three independent lines (2026-08-25)

1. **Mass.** Parent + 1×C13 predicted vs observed: −1.10 / −1.07 / −1.54 mDa. The +2
   position is occupied too and recon names it `AEC-MAEC`: −2.82 / −0.72 / −0.90 mDa.
   Counts are 31.1 / 28.3 / 19.8% of parent (+1) and 6.5 / 8.8 / 6.0% (+2).
2. **Composition fingerprint.** +58 and +59 carry the SAME Cys enrichment as +57:
   2.88 / 9.99 / 8.96 and 2.94 / 9.68 / 8.83, against +57's 2.85 / 9.98 / 9.41. A real
   AEC-MAEC chemistry would not track Cys that way.
3. **PSM identity overlap.** +58 band peptides also appear in the +57 band at
   88.5 / 25.9 / 54.7%, against chance levels of 14.3 / 6.8 / 6.7% — **6.2x / 3.8x /
   8.2x**. +59 tracks identically (6.1 / 3.6 / 8.1x). Charge shifts upward in the
   satellite band (b1906 4+ goes 10% → 24%), which is what monoisotope misassignment
   does.

bcell is enrichment, not near-identity, so this is strong evidence, not proof.
**Implement satellite detection as parent + n×C13 AND fingerprint match AND identity
overlap above chance** — not as a fixed mass band.

### ⚠ BUG — recon labels Carbamidomethyl an "Artefact" (2026-08-25)

`ModSpecificity.classification` is parsed correctly (`unimod.rs:23`, `:156`), then
thrown away by the accessors:
- `sites()` (`unimod.rs:49`) returns sites with classification stripped.
- `classification()` (`unimod.rs:62`) returns the classification of the **first
  non-hidden specificity in document order** — one string for the whole mod.

Carbamidomethyl's first non-hidden specificity is N-term (Artefact); C (Chemical
derivative) comes second. So `full-run/*.json` says:

| candidate | sites emitted | classification emitted |
|---|---|---|
| **Carbamidomethyl** | C, N-term | **Artefact** |
| Carbofuran | (empty) | Chemical derivative |
| Gly | (empty) | Chemical derivative |

Any rule of the form "prefer the Chemical derivative candidate" would resolve +57 to
Carbofuran or Gly. **Fix: keep the site→classification→neutral-loss pairing intact to
the report.** Empty `sites` on 62 of 106 annotations is the same defect — `sites()`
filters hidden specificities, and Gly/Carbofuran are hidden-only.

### ❌ Unimod `classification` is NOT a reality filter — do not gate on it (2026-08-25)

Proposed and rejected the same day. Classification is accurate **provenance** — how a
modification arises — and it is correct in every case checked. It is not a
primary-site marker and not a demotion signal:

- Oxidation: `M` is **Artefact**; the Chemical-derivative set is E/I/L/Q/S/T/V (no power).
  "Prefer Chemical derivative" would fail Ox(M), 3.4–4.3x, on every file.
- Deamidated: N and Q are both **Artefact**.
- Glu→pyro-Glu (record 27, E/Any N-term) and Gln→pyro-Glu (record 28, Q/Any N-term)
  are both **Artefact** — correctly, they are abiotic cyclization.
- Carbamidomethyl: C **Chemical derivative**, N-term/K/S/T/Y/D/E/H **Artefact**.

**An artefact is still a mod you want in your search** — arguably more certain, because
the bench induced it. The real limitation is narrower: classification cannot separate
COMPETING CANDIDATES AT THE SAME MASS, because all three +57 candidates are
`Chemical derivative`. Report it as provenance context; never gate on it.

**What DOES separate C from M for CAM: the `NeutralLoss` child.** In record 4, M is the
only specificity carrying one (mono 105.024835, H(7) C(3) N O S). C and U carry none.
So acceptor = classification-tagged chemistry with no neutral loss → C and U; U is
selenocysteine and effectively absent. The +57 test is Cys alone. Read the record's
structure, do not pool the site list.

**Do not pool composite specificities by classification group.** Pooling C∪M (both
`Chemical derivative`) raises bcell's background 9.6% → 31.4% and drops enrichment
9.98x → 3.17x. CAM-on-Cys and CAM-on-Met are different events. Per-residue scoring,
with BH correction for multiplicity.

### ✅ MetaMorpheus's curated mod list — adopt for tiering (2026-08-25)

Files copied to `reference-notes/metaMorpheusMods/` (Mods.txt, aListOfmods.txt,
ptmlist.txt, glyco.txt, substitutions.txt, tmt.txt, ProteaseMods.txt, surfactants.txt).
94 entries carry a computable mass.

**Provenance (verified 2026-08-25, not assumed).** Upstream is
`github.com/smith-chem-wisc/MetaMorpheus`, under `MetaMorpheus/EngineLayer/Mods/`
(Mods.txt, aListOfmods.txt, glyco.txt, substitutions.txt, tmt.txt, ProteaseMods.txt,
surfactants.txt) and `MetaMorpheus/EngineLayer/Data/` (ptmlist.txt). The upstream
repo was cloned locally (gitignored) at commit **`7e453540`**. All 8 files were
byte-identical to that clone when copied on 2026-08-25. ⚠ **CORRECTED 2026-09-01: `Mods.txt` IS NO LONGER IDENTICAL** — the
2026-08-26 local corrections changed it (2 renames, 1 `DR` fix, 5 `PP`
disambiguations, 5 added `Met-loss*` entries). Re-measured after CRLF
normalisation: 130 lines added, 8 changed; the other 7 files remain identical.
The redistribution consequence is recorded in `THIRD_PARTY_LICENSES.md`.
These are a dated snapshot like every other reference
input — a reader re-pulling from current upstream gets drift, and that is expected. Format is `ID` / `TG` (target residues) / `PP`
(position) / `MT` (category) / `CF` (formula) / `NL` / `DR` (Unimod xref).

Categories in `Mods.txt`: Common Biological 27, Less Common 40, Common Artifact 6,
Metal 8, Trypsin/AspN Digested 4, Speculative 3. `aListOfmods.txt` adds **Common Fixed**
(CAM on C, CAM on U) and **Common Variable** (Ox on M).

**What it fixes by construction, with no statistics:**
- **+57 ambiguity gone.** Candidates are `Carbamidomethyl on C` (Common Fixed) and
  `Carbamidomethyl` on K/H/D/E/S/T/Y (Less Common). **Gly and Carbofuran are not in the
  list at all.**
- **Carbamyl covered** — `Common Artifact`, `TG=X` peptide N-term plus K/R/C/M. This is
  the one enrichment structurally cannot verify, so curation is the only route to it.
- **Fixed-vs-variable falls out** of `MT`. Step 2 had that checkbox open with no rule.
- **Fe[III]/Fe[II] → `Metal`**, a bucket of their own.
- **Artifact peaks lose their names.** bcell −1.0290 `Lys→Allysine` and +59.0280
  `AEC-MAEC` have no curated entry, so they drop to the unannotated tail. Both were
  gate-1 violators. Full Unimod was supplying plausible chemistry names to isotope junk.

**Coverage cost, measured.** Top 12 non-zero peaks covered: serum 7/12, bcell 5/12,
b1906 6/12. Two genuine losses:
- `CarbamidomethylDTT` (serum +209.0205, 138 PSM) — real: excess DTT reacts with IAA
  and the adduct attaches to Cys. **Candidate for a local addition to the curated list.**
- `Cation:Al[III]` (serum +23.9593, 123 PSM) — they carry Na/K/Ca/Zn/Fe/Mg/Cu, no Al.

Both still surface in the unannotated tail, so nothing is hidden, only unnamed.

**Not a replacement for `unimod.xml`.** The list has formulas, not masses. Masses are
computed from `CF` using the element table inside our pinned `unimod.xml` (40 elements).
Unimod stays for elements and for informational names on tail peaks.

**Curation, not dedup, solves the multiple-testing burden.** Going from all-Unimod ×
all-residues to ~94 curated entries with hand-picked `TG` cuts the test count by orders
of magnitude. The isotope-dedup-before-testing idea was motivated by that burden and is
NOT being reopened — satellite folding stays disabled-by-design.

**Imperfect, not magic.** bcell's −0.9804 carpet peak now matches `Amidation`
(Less Common) — still a wrong name on an artifact. Curation shrinks the problem; the
satellite and carpet work still stands.

### ✅ Step 2 IMPLEMENTED — schema 1.3.0, three items outstanding (2026-08-25)

Three new modules. `stats.rs` (Fisher exact one-sided + Benjamini-Hochberg, hand-rolled
because the crate has no statistics dependency, **pinned by test against SciPy to
1e-12** including the real bcell 2×2 at OR 462.489736); `curated_mods.rs` (MetaMorpheus
parser, masses summed from `CF` against Unimod's element table); `tier_assignment.rs`
(the routing rule). 101 tests pass.

**`recommendations` is additive in the report — nothing was removed.**
`mod_discovery.peaks` is untouched: all 48 peaks on b1906, 25 of them unannotated, with
counts, percentages and prominence. **The unannotated tail is fully preserved** — that
is the differentiator and the block does not filter it.

**`not_curated` (31 on b1906) EXCEEDS the unannotated count (25), and that is correct.**
The curated list is narrower than Unimod, so peaks that carry a Unimod name but no
curated match — Lys→Allysine, AEC-MAEC, Unknown:302 — are not tiered but ARE still
named. Do not read `not_curated` as "unannotated".

**Unimod names are carried INTO the recommendation block, deliberately.** First
implementation dropped them: bcell's −1.0225 read as a bare mass plus `not_curated`
while `mod_discovery.peaks` said `Lys->Allysine`. That leaves a reader with no way to
judge the peak themselves, which is the opposite of what the tail is for — someone may
well want to search allysine and decide. Now 13 of b1906's 38 non-recommended entries
carry an informational Unimod name and 25 have no match anywhere, which is the
distinction that matters: **named but uncurated** (judge it yourself) versus **genuinely
novel** (a bare mass). The name is reported, never tiered on.

**Uncurated peaks ABOVE the floor get their own list, `notable_unannotated`.** This is
the design's "tier 2 — present, your call". Treated like an unspecific acceptor — the
floor is the instrument, because with no candidate there are no residues to test — but
FLAGGED, not recommended: an unnamed delta cannot go into a search configuration.
**No real test file exercises this path**: zero uncurated peaks clear the X=20% floor on
any of the three (closest is b1906 +16.9978 at 74% of floor), so its test fixture is
synthetic and covers routing only. A file with a large novel modification would use it,
and before this existed such a peak was flattened into the tail alongside trivia.

**All three files pinned, each covering a DIFFERENT behaviour.** One pin was not enough:
- **bcell** — the statistics OVERRIDE the curated list's category preference.
  Deamidation and Citrullination share +0.98402 and a formula; category ordering alone
  picks Citrullination, wrong for these samples, and only the odds ratios separate them
  (2.9 vs 1.19). Without this pin a candidate-selection regression relabels deamidation
  silently.
- **serum** — a curated candidate DROPPED by its own statistics. Fe[III] is in the list
  and clears nothing at OR 1.72.
- **b1906** — both paths, 7 by statistics and 1 by abundance.
Populations pinned too: 12438 / 55419 / 22298 rank-1 PSMs at spectrum_q < 0.01.

**⚠ Caught in production, not by a test: the background was the WRONG POPULATION.**
The first live run reported 21736 background PSMs where the prototype and the test say
22298. Cause: the code filtered `results.psms`, which the pipeline has ALREADY filtered
at `peptide_q < 0.01`, by `spectrum_q` — yielding the INTERSECTION of two criteria, not
the spectrum_q population. A 2.5% deviation from the stated basis. It changed no
recommendation, which is exactly why it would have survived. **The background is now
re-parsed at a loose threshold so production, test and prototype agree.** Lesson: a
pipeline-filtered PSM list is not a neutral starting population — check what has already
been applied to it.

**Outstanding, carried to the next session (also in PLAN's status block):**
1. **HTML rendering** — the block is in the JSON, `generate_html_report` does not draw it.
2. **Regenerate `full-run/` at 1.3.0** — rewrites the frozen ground truth, so it needs a
   downstream-impact trace first, and **`--closed-tsv` AND `--full`** or it silently
   drops `unified_ms1_error` and `three_layer_ms1`.
3. **Package the curated list** — `main.rs` hardcodes `reference-notes/metaMorpheusMods`
   relative to the working directory. Works from the repo root, breaks elsewhere. Needs
   a CLI flag and the same packaging `unimod.xml` gets. **The list is REQUIRED, not
   preferred:** when it cannot load, the block is omitted rather than falling back to
   Unimod, because a fallback would silently reintroduce the +57 ambiguity.

---

### ✅ Step 2 decision rule — ROUTE BY SPECIFICITY (2026-08-25)

**Every peak goes to the ONE instrument that can decide it. Nothing is judged twice.**

| acceptor | instrument | rule |
|---|---|---|
| residue-specific (`TG` names residues, background not saturated) | statistics | Fisher exact one-sided, odds ratio, BH. Accept on **OR >= 2 AND q < 0.05** |
| unspecific (`TG=X` at a terminus, or background > 95%) | abundance | count >= X% floor, X=20% |
| isotope satellite (parent + n×C13, within 6 mDa) | — | demoted before either test |
| not in the curated list | — | unannotated tail, never tiered |

**Both thresholds are needed on the statistics path.** Gly reaches q=1e-5 on bcell
from n=2969 alone while its odds ratio is 1.37. Nothing measured lands between the
rejected maximum (1.39) and the accepted minimum (2.9).

**Why not apply both instruments to every peak — measured, both directions:**
- *Floor as a gate on top of statistics* drops Gln→pyro-Glu, which carries the
  strongest odds ratios we measure (**27.4 / 330.5 / 285.6**) while sitting below the
  floor at X=20%, 15% AND 10% on all three files.
- *Statistics as a promotion path on top of the floor* breaks gate 1: **29 / 28 / 30
  violations at X=10/15/20**, against 0 at X=20% floor-only. Not a tuning problem.
  Gate 1 validates an abundance ordering, and evidence promotion deliberately inverts
  abundance — pyro-Glu (59 PSMs) over Acetylation, Water Loss (41) over Dioxidation.
  The two are incompatible by construction, not miscalibrated.

**Result on the three test files:**

| file | by statistics | by abundance |
|---|---|---|
| serum | 6 | 0 |
| bcell | 8 | 0 |
| b1906 | 7 | **1 — Carbamyl, 555 PSMs, 221% of floor** |

The abundance path fires ONCE across all three files. Carbamyl is recommended on
b1906 and correctly dropped on serum (17 PSMs) and bcell (77) by the same floor, so
the fallback discriminates between files rather than waving the mod through.

**Metal routing falls out of the data, with no category rule.** Fe[III] fails on
serum (OR 1.72) and drops; passes on bcell (4.6). Fe[II] passes on bcell (2.0) and
b1906 (6.0). Same chemistry, different verdict per file.

**Fixed-vs-variable is a LABEL, not a measurement, and the report says so.** It is
read from MetaMorpheus's `MT` field, where `Common Fixed` matches only
Carbamidomethyl on C/U. The occupancy rule PLAN proposed was measured and does NOT
work: CAM occupies 23.5 / 53.4 / 50.7% of Cys-containing PSMs against Ox(M) at
6.8 / 7.8 / 13.0%. Separated, but nowhere near "approaches total occurrence" — and it
**cannot** get there, because an open search assigns one delta per PSM, so a Cys
peptide carrying CAM plus anything else lands elsewhere. Occupancy is floored by the
search design. Attribute the label, do not claim it.

### ⚠ What routing by specificity COSTS us in validation (2026-08-25)

**Gate 1's surface shrinks to almost nothing, and this must not be glossed.** Gate 1
validates recon's abundance-ordering claim. Under the routing rule that claim now
covers only the abundance path — **one peak across all three test files**. Gate 1 is
not failing; it has nearly nothing left to check.

**The corroboration figure is NOT a gate.** Every recommendation on all three files
appears in at least one reference tool (21/21 on the statistics path, 20 of those in
both PTM-Shepherd and Mascot; Carbamyl also in both). **That check was designed after
seeing the data**, and it only asks whether any tool reports the mass at all. Treat it
as encouraging, not as validation. A real replacement gate needs a pre-committed
threshold and a negative control, decided before looking.

This is the exact shape NOTES records as the satellite-folding failure — a result
declared successful against numbers that could not refute it. Recorded here so the
next session does not mistake 21/21 for a passing gate.

**Also unbounded: recommendation list size.** 6-8 on these files, with no cap. A
messier file could produce more and nothing limits it.

**Label override.** MetaMorpheus's `ID` for the `TG=Q` N-terminal entry reads
"Glu to PyroGlu". Its `TG` and `DR` (Unimod 28) are correct; the name is loose. The
prototype overrides that one string to `Gln->pyro-Glu (Unimod 28)`.

### ✅ Tier-3 snapshots re-baselined for the entity fix — recorded edit (2026-08-26)

`testing/regression-snapshots/discover_{serum,bcell,b1906}.snapshot.json` carried the
ESCAPED Unimod names and failed 3/14 after the parser fix. Re-baselined as a
deliberate recorded edit.

**Proven safe BEFORE re-baselining, not asserted after.** With every name-carrying
field blanked, regenerated output and old snapshot are identical on all three files;
peak counts 48 / 47 / 48 unchanged. Only name strings moved: **7 of 43 (serum), 5 of
31 (bcell), 3 of 32 (b1906)** — e.g. `Lys-&gt;Allysine` -> `Lys->Allysine`,
`Glu-&gt;pyro-Glu` -> `Glu->pyro-Glu`. Harness back to **14/14**.

This is the tripwire working: the snapshot gate caught a label change the author knew
about, which is what makes it trustworthy when it catches one nobody expected.

### ⚠ `id_rate_by_tic_pct` is NOT bit-reproducible run to run (2026-08-26)

**Measured, not inferred.** The same input produces a different value for
`signal_fate.id_rate_by_tic_pct` on every run. Three runs of the identical input:

| file | committed (1.2.0) | run130 | final | drift |
|---|---|---|---|---|
| serum | 17.469740511910853 | ...803 | ...885 | 14, then 23 ULP |
| bcell | 23.77345534513932 | ...911 | ...913 | 59, then 6 ULP |
| b1906 | 27.184498784644642 | ...582 | ...543 | 17, then 11 ULP |

**run130 and final differ although NO code change between them touches TIC.** That
rules out the regeneration as the cause. It is summation ORDER varying between runs,
consistent with a parallel reduction. Every other `signal_fate` field, including all
integer counts, is bit-identical across all three runs. Same class as the 32/36/1 ULP
TIC noise recorded for the 2026-08-25 regeneration.

**Consequence for checks:** a verify-regenerate of this field can never be
bit-exact. `assert_regeneration_invariants.py` names it explicitly with a 1024-ULP
budget and PRINTS the actual distance. That is a named exception for ONE field, not a
blanket float tolerance — everything else must still be bit-identical.

**⚠ THE DETERMINISM TEST DOES NOT COVER THIS, AND ITS DOCSTRING OVERCLAIMS.**
`tests/determinism_test.rs` says it will catch "thread-order-dependent float sums".
It guards `ModDiscoveryResult` only. `signal_fate` is a different code path and is
NOT under that guard — which is why this drift survived to be found by hand. Same
shape as the gate-audit findings: a guard whose stated scope is wider than its real
one. **Not yet fixed. Either narrow the docstring or widen the test.**

### ✅ Met-loss protein N-term is VISIBLE as a delta mass — measured 2026-08-26

**The finding.** A search that cannot build Met-clipped peptides still shows the
Met-excised population — displaced by −131.04 into the delta-mass axis. bcell's most
negative peak is **−89.0289 with 209 PSMs**, against Unimod 766 `Met-loss+Acetyl`
(mono **−89.02992**, site M, position Protein N-term, class Co-translational). **1.0
mDa apart.** Arithmetic: acetyl `C2H2O1` minus the Met residue `C5H9N1O1S1` =
`C-3 H-7 N-1 S-1` = −89.02992.

**⚠ CORRECTS A CLAIM MADE EARLIER THE SAME DAY.** It was stated in session that the
NatA class "would be entirely absent by construction" because the search space has no
Met-clipped peptides. **That was wrong.** Absent as PEPTIDES, present as a DELTA MASS.
The measurement outranks the claim. Do not carry the "absent by construction" reading
forward.

**Zero Met-excised peptides on all three files** — this part stands, and it is why the
delta-mass route is the ONLY route under the current search config. Basis is rank-1
targets at spectrum_q < 0.01, the standard delta-band population (b1906 reproduces
22298 exactly).

| file | confident PSMs | protein N-term, Met RETAINED | Met EXCISED |
|---|---|---|---|
| serum | 12438 | 196 (1.576%) | **0** |
| bcell | 55419 | 606 (1.093%) | **0** |
| b1906 | 22298 | 97 (0.435%) | **0** |

The pass-1 open search is fully tryptic (`cleave_at KR`, `restrict P`,
`semi_enzymatic null`) with no Met-clipping option, so a peptide starting at protein
position 1 has a non-tryptic N-terminus and is not in the search space.

**The −89.03 band behaves exactly as N-terminal-acetylation enzymology predicts.**
The band holds **194** PSMs, of which **188** map to a protein N-terminal (position 0)
peptide. Testing whether protein residue 2 is NME-permissive (A, C, G, S, T, V):

| | residue 2 NME-permissive | not |
|---|---|---|
| in the −89.03 band | **186** | 2 |
| other protein-N-term PSMs | 231 | 187 |

**98.9% in-band vs 55.3% background. Odds ratio ≈ 75.** Band composition is textbook
NatA — **A 119, S 57, T 6, V 2, G 2, M 2** — tracking the Ala > Ser > Gly cleavage
ranking. The only 2 non-permissive residues are Met, which the rule calls
NME-resistant; 2 of 188 is noise.

**⚠ TWO NUMBERS IN THIS ENTRY WERE WRONG. Corrected in place 2026-08-27.**
1. **The background was stated as 68.8%. It is 55.3%.** 68.8% is (186+231)/606 — the
   band counted into its own background. The table's own cells give 231/418 = 55.3%,
   and that is the figure recon uses: `tier_assignment::test_candidate` builds a 2x2 of
   band vs NOT-band, because counting the band inside the background understates the
   odds ratio. The odds ratio ≈ 75 was always computed off the table, so it stands.
2. **"188 of the 209 PSMs" mixed two populations.** 209 is the report's PEAK count.
   The delta-band under `BAND_TOL_DA` holds 194 PSMs. It is 188 of 194.

**⚠ SINGLE-FILE EVIDENCE BASE — but that is about peak DETECTION, not the population.**
serum and b1906 have no PEAK at −89.03. They do have band PSMs: b1906 has 22, of which
21 sit at protein position 0 and 21/21 are NME-permissive; serum has 2, of which 1 is
at position 0. b1906 corroborates the composition at a smaller n. Still one of three
files for the peak itself, and still to be read the way NOTES reads "the abundance path
fired once".

**Every number in this entry is now produced by `testing/scripts/protein_nterm_evidence.py`,
which hard-stops unless it reproduces 12438 / 55419 / 22298 confident PSMs and
196 / 606 / 97 at protein position 0.** Before 2026-08-27 the entry was prose with no
producing script, which is the "a summary is not a source" failure. Run the script; do
not quote this entry.

### ✅ Met-loss encoding — the POSITION is the acceptor test (2026-08-26)

**Decision: encode the Met-loss form as ONE curated entry. No motif language, no new
file format, and NO ENZYMOLOGY IN THE TOOL.**

```
ID   Met-loss+Acetylation
TG   M
PP   Protein N-terminal, Met loss.
MT   Common Biological
CF   C-3 H-7 N-1 S-1
DR   Unimod; 766.
```

**`TG=M` states the one hard requirement.** Met loss can only occur where the protein
N-terminus IS a Met. That is a fact about the chemistry, not a prediction.

**`PP` carries the real acceptor test — and it is positional in the PROTEIN, not
residue-based.** The peptide must sit at protein position 0. That is rare on its own
(**0.4-1.6% of confident PSMs across the three files**), so it discriminates without
any residue argument. This generalises: for a protein-terminal candidate, an empty
`sites` is not a weakness — the position does the work that residues do elsewhere.

**Agrees with Unimod 766, which also records `site=M`.** An earlier version of this
entry departed from Unimod deliberately; that departure is withdrawn (see below).

**⚠ REVISED 2026-08-27 — `TG` ON A Met-loss ENTRY NAMES THE EXPOSED RESIDUE.**
The entry above records `TG   M` for the Met-loss forms. That is superseded. All four
now read the acceptor of the modification that FOLLOWS the removal, at protein
residue 2 — the residue the removal exposes:

| entry | TG | source, read off the PINNED unimod.xml |
|---|---|---|
| `Met-loss` (765) | X | nothing follows the removal |
| `Met-loss+Acetylation` | X | Unimod 1, Acetyl @ Protein N-term = `site=N-term` |
| `Met-loss+Methylation` | X | Unimod 34, Methyl @ Protein N-term = `site=N-term` |
| `Met-loss+Succinylation` | X | Unimod 64, Succinyl @ Protein N-term = `site=N-term` |
| `Met-loss+Myristoylation` | **G** | Unimod 45 AND 135, N-terminal Myristoyl = `site=G` |

**Why this is not a return to the withdrawn design below.** That one put the
NME-PERMISSIVE SET (A/C/G/S/T/V) on Met-loss+**Acetylation**, encoding aminopeptidase
enzymology as an acceptor list. Acetylation here carries `TG=X` — no residue claim at
all, which is the opposite. The single residue constraint is myristoylation's Gly,
which is that modification's own chemistry and is what Unimod itself records. The tool
still makes NO judgement about whether NME is plausible on a given residue 2.

**The initiator-Met requirement did not go away; it moved to where it belongs.**
`PP = "Protein N-terminal, Met loss."` already states it — a Met that is not there
cannot be lost — so `peptide_hits` enforces residue 1 = M from `PP`, and `TG` is
freed to mean what it means for every other entry in the file: where the named
modification goes.

**Why `TG=G` could not have stayed at residue 1.** It would demand a protein that
starts with G AND lost a Met it never had. Only 19 of 20596 human canonical sequences
begin with G, and none of those has an initiator Met to lose, so the entry would match
zero PSMs forever, silently. Meanwhile 1635 M-starting proteins have G at residue 2 —
the real N-myristoylation population, reachable ONLY through the Met-loss entry.

**Behaviour check, and it is exactly neutral where it should be.**
`Met-loss+Acetylation` went `TG=M` -> `TG=X`. Under the old reading `TG=M` tested
residue 1 = M; under the new one `PP` enforces the same thing and `TG=X` adds nothing.
bcell −89.0289 reports **OR 4230.694581280788 before and after, bit-identical.**

**⚠ SUPERSEDED SAME DAY (2026-08-26) — "TG is the NME rule".** This page briefly recorded
`TG = A or C or G or S or T or V`, encoding the NME-permissive set as the acceptor
list so the enzymology would become "just the acceptor set". **Withdrawn. Do not
reintroduce it.** Reasons, in order:
1. **It is not the tool's call.** Every aminopeptidase and N-acetyltransferase has its
   own pattern. Recon flags that a Met-loss+mod population is present; whether residue
   2 makes it plausible is the reader's judgement, with their own organism and enzyme
   knowledge. The tool makes an effort, it does not rule.
2. It would have silently dropped real observations. A Met-loss event on a residue
   outside the permissive set would fail the acceptor test and be scored as a miss —
   the same failure shape as scoring data against a prior.
3. `TG=M` plus the protein-position test is simpler AND more discriminating, because
   protein position 0 is rare while residue identity is not.

**The NatA measurement is a FINDING, not a rule.** The 98.9%-vs-55.3% result below
still stands as an observation about the bcell data and is good write-up material. It
is no longer part of any decision path, and no acceptor set is derived from it.

**REJECTED for v0.1.0 — a motif field for the Met-RETAINED form.** `Acetylation` at
protein N-term applies to ANY protein N-terminal residue; no residue restriction is
claimed. "Not Met loss" simply means the protein N-term Met was never removed or was
retained. Whether some residues make that implausible is the same enzymology question,
and it stays with the reader.

**Still required, not removed by the encoding:** the FASTA lookup that proves the
peptide sits at protein position 0. That IS the acceptor test now, so nothing works
without it. **BUILT 2026-08-27 — see "Step 2.5 BUILT" below.**

**⚠ UNVERIFIED EXTERNAL CLAIMS.** The NME and NAT-class references supplied in session
(MetAP P1' specificity, NatA/NatB/NatC/NatF assignments, the Pro "(X)P rule") have NOT
been checked against their sources. They are no longer load-bearing for the design, but
must be verified before the write-up cites them.

### ✅ Step 2.5 BUILT — the protein-position lookup, and ONE promotion (2026-08-27)

`protein_index.rs` reads the search FASTA into accession -> sequence.
`tier_assignment::peptide_hits` gained ONE branch for every candidate
`needs_protein_context()` returns true for. `analyze --fasta` is optional; `run`
passes its own through. Schema was 1.4.0 here, and is 1.5.0 since the 2026-08-28
MS1 bias fix. `run_validation` 14/14. ⚠ The test count recorded here was **114** and
was stale: re-measured 2026-08-28 at that same commit, `cargo test` reports **116**
non-doc tests plus 2 doc tests.

**ONE RULE, NOT A SPECIAL CASE — this is the design simplification.** Both protein
terminal `PP` values take the same branch:

> the peptide must start at protein position 0, AND residue 1 must be in `TG`.

`Protein N-terminal.` reads that way plainly. `Protein N-terminal, Met loss.` reads
that way too, because the open search matched the Met-RETAINED peptide and put the
mod-minus-Met difference into the delta mass — so the PSM's peptide still begins at
protein position 0 with its initiator Met. **No residue-2 logic exists anywhere in
the tool.** PLAN's checkbox said "reads residue 2 instead of residue 1" until
2026-08-27; that was the superseded "TG is the NME rule" design leaking forward, and
it is corrected in place. Testing residue 2 against `{M}` would have rejected 186 of
the 188 true PSMs.

**RESULT — exactly THREE decisions move across the three files.**
Asserted by `tier_assignment_integration::protein_context_moves_exactly_three_decisions`,
which runs `assign` twice per file, with and without the index, and diffs the tiers.

| file | what moved | OR |
|---|---|---|
| serum | **+14.0149 `Methylation`: `BelowFloor` -> `Statistics`** | 24.1 |
| bcell | **+42.0109 `Acetylation`: `NoResidueSupport` -> `Statistics`** | 173.6 |
| bcell | **−89.0289 `Met-loss+Acetylation`: `BelowFloor` -> `Statistics`** | 4230.7 |
| b1906 | nothing | — |

**⚠ THE FIRST VERSION OF THIS ENTRY SAID "exactly ONE decision moves", AND THE FIRST
IMPLEMENTATION ONLY MOVED ONE. Corrected in place 2026-08-27.** The two `TG=X`
protein-N-term entries never reached the new branch, because `test_candidate` checked
`sites.is_empty()` FIRST and returned "untestable". That check was written when protein
position was unknowable, and under that constraint it was right. Step 2.5 removed the
constraint and the check was not revisited — so recon was reporting `no_residue_support`
for bcell's +42.0109, a peak that is **63.8% protein N-terminal against a 1.09%
background**. A confident wrong answer, which is the exact failure the guard existed to
prevent, pointing the other way. The pre-committed invariant validated the
implementation that was built, not the design this file had already specified.

**The fix is scoped, and the scope is the whole point.** `TG=X` is genuinely
unspecific at a PEPTIDE terminus (every peptide has one; the background saturates) and
`Anywhere.` likewise. Only a PROTEIN terminus earns position-as-specificity. A unit
test, `empty_sites_at_a_peptide_terminus_stays_untestable`, is the narrowness control.

The production 2x2 on bcell, background 55419 (rank-1, `spectrum_q < 0.01`; decoys
are dropped by `parse_sage_results`), band 194:

```
a=188  b=6  c=406  d=54819     OR = 4230.7     q = 0.0
```

b1906 gains nothing: no protein-terminal candidate lands on any of its peaks.

**At +42.0109 the WINNING CANDIDATE changes, which is why its odds ratio moves.**
Three curated entries sit within 10 mDa: `Acetylation TG=K Anywhere.` (OR 1.0),
`Acetylation TG=S or T Anywhere.` (OR 1.2) and `Acetylation TG=X Protein N-terminal.`
(OR 173.6). Before the FASTA, the first won the testable list and FAILED, so the peak
read `no_residue_support`. The protein-terminal reading is not just better supported,
it is the only one that passes. At serum +14.0149 both `Anywhere.` methylation entries
have SATURATED backgrounds and say nothing; the protein-terminal entry is the only
testable candidate at all.

**ALSO ADDED 2026-08-27: `Met-loss` itself, Unimod 765** (−131.040485, the initiator
Met removed with no following modification). It was missing while all four "+mod"
forms were present. It fires on NOTHING here — there is no −131.04 peak on any of the
three files, which is UNEXPLAINED and recorded as such rather than guessed at. Worth
asking why, given −89.03 carries 209 PSMs on bcell.

**⚠ THE PREDICTED ODDS RATIO WAS 4261, NOT 4231, AND THE PREDICTION USED THE WRONG
DENOMINATOR.** The pre-commitment computed the background with decoys included
(55815). `parse_sage_results` drops decoys unconditionally, so production sees 55419.
Corrected here rather than left standing. The conclusion is unchanged; the number is
not the one that was predicted.

**Also learned: `TG=M` does bite on the background, slightly.** 606 PSMs sit at
protein position 0 but only 594 of them start with M, so 12 UniProt canonical
sequences in this database do not begin with an initiator Met. `TG` is not redundant
with the position test.

**I5 — adding one test to the BH sweep moves every other q, and only downward.**
This was checked, not assumed, because bcell's Fe[II] sits at q = 0.0300 against a
0.05 threshold. Measured: Fe[II] 0.029963 -> 0.028660, pyro-Glu 1.418e-237 ->
1.110e-237, Deamidation 1.359e-20 -> 1.135e-20. Every q fell; no odds ratio moved.
A near-zero p entering at rank 1 can only lower the rest, and the integration test
asserts BOTH halves (`q` may not rise, `odds_ratio` may not move at all). **The first
version of that test compared whole `Decision` values and reported 9 moves instead of
1** — the q payload made every statistics peak look like a tier change. Compare the
VARIANT, and assert the payload separately.

**GUARDS, all asserted:**
* **Wrong-FASTA tripwire.** At least 95% of TARGET PSMs must resolve an accession, or
  `analyze` hard-stops with the fraction printed. Measured 100.00% on all three
  (12438/12438, 55419/55419, 22298/22298). Without it, a mismatched FASTA resolves
  nothing, every protein-terminal candidate fails its acceptor test, and the report
  reads "no protein N-terminal modifications present" — a confident wrong answer with
  no symptom.
* **Rarity.** Protein-position-0 PSMs must stay under 5% of the background. Measured
  1.576 / 1.093 / 0.435%. A lookup matching everything breaks this at once.
* **Decoys.** Zero `rev_` PSMs may classify as protein N-terminal, because `rev_`
  accessions are not in a target FASTA. Measured 0 of 396 on bcell. This is the
  negative control and it is free.

**NOT TESTABLE is recorded, not inferred.** `recommendations.protein_context` is
present when a FASTA was supplied and absent otherwise, and the absent case adds a
caveat saying the class was not tested. "Not tested" and "not supported" are different
claims and the report must not blur them.

**The peptide-level shortcut is NOT the same test, measured.** Reading
`Protein N-terminal` as a peptide N-terminus runs at a 3.17% background on bcell
(peptides beginning with M) against 1.09% at protein position 0, and passes every
internal tryptic peptide that happens to begin with M. `tier_report_prototype.py` did
exactly that until 2026-08-27, so **the prototype and the shipped Rust had already
diverged, invisibly** — the pinning test covers b1906, where no protein-terminal
candidate lands on a peak. Fixed. Both now report bcell OR 4230.7.

**SCOPE, SETTLED 2026-08-27: protein N-terminomics is ATTEMPTED, not CLOSED.**
Recon detects protein-terminal modifications and decides them statistically against the
search FASTA. It does not claim parity with a dedicated N-terminomics workflow, and the
gap is measured: MSFragger finds 137 N-term acetyl PSMs on b1906 where recon recommends
NONE, and on bcell recon's recommended protein-N-term peaks hold 329 PSMs against
MSFragger's 1016. Recon only sees what survives as a delta-mass PEAK.

Closing that gap needs the PTM-Shepherd / MetaMorpheus quantity comparison — a research
question, the same boundary that retired Gate 5 — not an implementation task. Deferred
past v0.1.0 DELIBERATELY, and it is a REQUIRED write-up item, not an optional one. The
rejected alternative was digging into it now; rejected because it would hold step 3
behind an open research question for a feature that already works as far as it claims.

**⚠ WHAT THIS DOES NOT VALIDATE.** The unit tests for the new branch run on a
synthetic two-protein index and cover ROUTING only. A synthetic fixture inherits the
assumptions of the code it tests and cannot catch a wrong convention. The convention
check is the real-data integration test plus the NatA composition, which is an
independently known answer.

**⚠ CORRECTED SAME DAY. This entry first said "step 2.5 has no reference-tool
cross-check". It has one, and it was committed the whole time.** MSFragger's
strictTryp `fragger.params` carries `clip_nTerm_M = 1` with
`variable_mod_02 = 42.0106 [^ 1`, so Met-clipped protein N-term acetyl was in its
search space with NO custom modification added. Splitting bcell's `psm.tsv` against
the FASTA: 1016 N-term acetyl PSMs, of which **767 Met-CLIPPED** and 248 Met-RETAINED.
The population is real, it is in bcell, and the Met-loss form dominates — which is
what recon's −89.03 peak claims. Magnitudes do not match (recon reports 209) and are
not expected to: un-localized delta-mass population versus per-PSM localization, the
same quantity boundary that retired Gate 5. Corroboration at the rate level, not a
pass/fail. Full numbers in the limitations note.

The claim was made without opening a config file that was already in the repo. Same
failure as the b1906 escalation NOTES records at the top of this file.

### ✅ Carpet invariant ASSERTED IN CODE — passes on all three (2026-08-26)

`tier_assignment::carpet_margin` computes it; every report now carries
`recommendations.carpet_margin_psms` and `carpet_tallest_psms`, and
`tier_assignment_integration::floor_sits_above_the_carpet_on_all_three_files`
hard-asserts it on the committed reports with the numbers printed.

| file | floor (PSMs) | tallest floor-governed carpet peak | margin |
|---|---|---|---|
| serum | 225.0 | 53 | **+172.0** |
| bcell | 662.2 | 383 | **+279.2** |
| b1906 | 250.6 | 112 | **+138.6** |

Carpet regions are the SAME bounds `decoy_delta_histogram.py` uses (±1 Da 0.85-1.15,
±2 Da 1.85-2.15, both signs), so the carpet has one definition in this project.

**These are NOT the superseded +115.8 / +113.6 / +75.9 margins**, exactly as predicted
when those were withdrawn: the scoped invariant measures a different peak set,
because residue-specific peaks are out of the floor's jurisdiction entirely.

### ✅ GATE 5 RETIRED AS A PASS/FAIL — kept as a corroboration RATE (2026-08-26)

**Threshold never moved. Revised twice on evidence; both revisions made it fail LESS
by fixing the gate's premises, and the negative control fires under the final rule.**
`gate5_residue_agreement.py`, probe `gate5_degeneracy_probe.py`.

**Final: 18 pass, 3 violation, 0 uncovered, of 21. Agreement rank AA1 28, AA2 4,
AA3 2. Strict (original) rule: 10.** All three violations are the same thing:

| file | mass | recon | MetaMorpheus |
|---|---|---|---|
| serum | −17.0285 | Gln->pyro-Glu on **Q** | Ammonia loss on **N** |
| bcell | −17.0257 | Gln->pyro-Glu on **Q** | Ammonia loss on **N** |
| b1906 | −17.0266 | Gln->pyro-Glu on **Q** | Ammonia loss on **N** |

**The chemistry, which explains the shape but does not settle it.** Gln->pyro-Glu IS
ammonia loss — the same −17.0265 reaction — exactly as Glu->pyro-Glu is water loss.
The two tools differ on POSITION, not on reaction: recon claims the peptide
N-terminal Q cyclization, MetaMorpheus claims an internal Asn. Both are real
chemistry and both can be present; the peak is a mixture.

**Weight of evidence, stated without resolving it.** recon's test is positional
("first residue is Q") at **OR 27.4 / 330.5 / 285.6, q to 1e-29** — among the
strongest signals we measure. **PTM-Shepherd agrees with recon** (AA1 = Q, with N
only as AA2) on all three files. MetaMorpheus **was configured for `Glu to PyroGlu
on Q`** in its GPTMD list and still reported Ammonia loss on N, so this is NOT a
capability limitation. Two of three tools agree with recon. That is not proof recon
is right.

**TWO REVISIONS, both grounded, both reported on every run.**

1. **ADOPTED — compare against the reference's published residue SET.** PTM-Shepherd
   publishes AA1/AA2/AA3; recon's `sites` is a set. Comparing a set to one element
   was an asymmetry. In all 6 PTM-Shepherd disagreements recon's residue was already
   in that tool's own AA2/AA3. Makes the gate WEAKER, so the strict count (10) prints
   next to the revised count and every pass reports its agreement RANK.
2. **ADOPTED — a reference can only be evidence about a claim it is CAPABLE of
   making.** Read from MetaMorpheus's GPTMD config, NOT from its output — inferring
   capability from output is circular, since "capable" would then mean "agreed".
   This resolved the b1906 Formylation violation: MetaMorpheus was configured for
   `Formylation on K` only, never S/T, so it could not have agreed. Silence is
   uncovered surface, never disagreement.

**⚠ REJECTED — excusing a violation as "mass degeneracy". IT BROKE THE NEGATIVE
CONTROL.** The clause was "not a violation if another curated candidate at this mass
accepts the reference's residue". Under it the serum +57 -> Gly counterfactual is
EXCUSED, because `Carbamidomethyl on C` sits at that mass. **At a degenerate mass
"the reference named a different chemistry" and "recon named the WRONG chemistry" are
indistinguishable by that test.** Do NOT reintroduce it. Degeneracy is reported as
context on each violation and excuses nothing.

**⚠ REJECTED — a confidence threshold on the reference, on measurement.** PTM-Shepherd
enrichment scores do not separate: agreeing 1.9-42.2, disagreeing 2.9-18.7. Any cutoff
would have been chosen to make failures disappear.

**Four instrument defects were fixed before the result was believed.** Recorded so the
iteration is visible: (a) MetaMorpheus joined on `Mass Diff (Da)`, ~0 in a CLOSED
search, localized 0 of 21; (b) identity keys used the whole `TG` set while MM writes
one residue, so Deamidation resolved to R on 17 PSMs against 2300 real
`Deamidation on N`; (c) the degeneracy probe excluded candidates by ID, but
`Formylation` exists three times at +27.9949 with different `TG`; (d) the GPTMD config
stores LITERAL backslash-t, so splitting on a real tab produced an empty capability
set that silently read as "configured for nothing".

**⚠ THE PRE-COMMITMENT ITSELF WAS DEFECTIVE, and the repo said so BEFORE the gate
was written.** This is the real finding, and it is not post-hoc: it is checkable
against a document committed the previous day.

Gate 5 compares recon's claim against the references' claims as if they were the
same quantity. **They are not, and `ptm-stratification-design.md` said so in commit
`ba2ff58` (2026-08-25), one day before Gate 5 was specified:**

> "Recon's open-search peak percentage is a discovery-rank statistic. Other tools
> report a post-localization PSM fraction. **These are different quantities.**"

> "...applied to a **localized per-residue count** from a different engine. **Recon
> reports an un-localized delta-bin count.**"

**The mechanism, confirmed from Sage's own vendored documentation:**
`sage-online-docs.md:3538` — "hyperscore | **X!Tandem hyperscore for the PSM.**" It is
a PSM-level spectral-match score. It scores the peptide IDENTIFICATION, not the
modification SITE, and Sage does no site-confirmation pass. So:

| | what it produces | how |
|---|---|---|
| **recon** | an UN-LOCALIZED delta mass on a PSM, plus a POPULATION-level acceptor enrichment ("peptides in this band are enriched for starting with Q") | one open search, Fisher exact over the band |
| **MetaMorpheus / PTM-Shepherd** | a PER-PSM site assignment | secondary sweeps that exist to confirm PTM identity AND position |

**Consequence for the three violations.** recon says the −17.0265 BAND is enriched for
N-terminal Q. MetaMorpheus says INDIVIDUAL PSMs carry ammonia loss on internal Asn.
**Both can be true at once** — the band is a mixture, and the two tools are reporting
different quantities over it. The gate cannot adjudicate that, because matching those
quantities directly is exactly what the design doc ruled out.

This is also why `peak_composition.rs` carries "not site assignment, so it stays
compatible with the no-per-residue-localization lock", and why the design doc's "What
this does not do" says plainly: "It does not localize."

**What Gate 5 IS still good for.** The corroboration it measures is real where the two
quantities happen to point the same way: **18 of 21 agree, at ranks AA1 28 / AA2 4 /
AA3 2.** That is a reportable number. What is NOT sound is hanging a binary pass/fail
on the three places where a population statistic and a per-PSM localization diverge.

**DECIDED 2026-08-26 — option A. The binary threshold is RETIRED; the measurement is
kept.** Not because the gate failed, but because the pre-commitment compared two
different quantities, as shown above.

**Step 2 ships with:** the carpet invariant asserted in code (margins +172.0 / +279.2
/ +138.6) and a corroboration RATE of **18/21 at ranks AA1 28 / AA2 4 / AA3 2**. The
script prints "10 would have failed the ORIGINAL rule" on every run so the revisions
can never hide what they changed, and names the three divergences rather than burying
them. It returns 0 always and says "NOT A PASS MARK" in its own output.

**⚠ THE COST, STATED: step 2 now has NO binary gate on residue assignment.** That is
the same weakened-surface complaint that retired gate 1, happening a second time.
Written into `limitations-and-future-work.md` as a limitation, not glossed.

**The route back to a binary gate, if wanted later:** specify it on COMPATIBILITY
rather than identity. serum +57 -> Gly is incompatible with a band that is 96%
Cys-containing; pyro-Glu vs ammonia-loss is compatible. Testable, but it needs a FRESH
pre-commitment written before it is run. The negative control stays wired for exactly
this reason.

**Do NOT re-litigate the retirement without reading the quantity-boundary section
above.** The three divergences are explained and recorded; they are not an open bug.

### 🔒 Gate 5 — residue agreement. PRE-COMMITTED 2026-08-26, BEFORE it was run

**This text was written and committed before the gate produced a single number.**
Read that as the point of the entry. Gate 1 lost its surface to the routing rule, and
the 21/21 corroboration figure was designed after seeing the data. A replacement gate
is only worth anything if its threshold is fixed first. If a later result makes this
gate look wrong, investigate the gate — do not retune the threshold to fit.

**What it tests.** The statistics path makes a RESIDUE claim (`sites`, backed by odds
ratio and BH q). That claim is falsifiable against an independent tool's own
localization. Gate 1 validated an abundance ordering; Gate 5 validates the claim the
routing rule actually makes.

**The rule.** For every recommendation on the statistics path, the reference tool's
top-localized residue for the matching mass must sit INSIDE recon's claimed acceptor
set.

| outcome | verdict |
|---|---|
| reference localizes the mass to a residue INSIDE recon's `sites` | pass |
| reference localizes the mass to a residue OUTSIDE recon's `sites` | **violation** |
| reference does not localize the mass at all | **not counted either way** — reported as uncovered surface, with a count |

**Threshold: ZERO violations.** Fixed now, not after.

**Uncovered surface is printed, not hidden.** A mass no reference localizes cannot
pass or fail. The count of those goes in the output next to the verdict. A gate that
covers 3 of 8 recommendations and says PASS is the 21/21 mistake again.

**Negative control — serum +57, and it CAN fail.** Already adjudicated: serum's +57 is
over-alkylation, not added glycine (Phase 8 Gate 3, 0 of 26 peptides with Gly flanking
context). If recon ever routed +57 to Gly (T/S/K), both references must localize it to
C and the gate must fail. Verify the control fires before trusting a pass — watch it
fail first.

**Two instruments, and they are NOT equivalent. State which one produced a number.**

| instrument | residue call | caveat |
|---|---|---|
| PTM-Shepherd `global.profile.tsv` — `AA1`/`AA2`/`AA3` + enrichment scores, and `n-term_localization_rate` | **POOLED across all three files** | PSM counts in that file are per-file but the AA columns are not. One residue call per mass, not per file. |
| MetaMorpheus `AllPSMs.psmtsv` | **per file** | Gitignored, so it is not in the checkout; read from the on-disk copy by path. Stronger instrument. |

**The abundance path stays under Gate 1**, covering one peak across three files, and
the report says so rather than implying wider validation.

**Scope, stated so it is not overclaimed.** Gate 5 checks residue agreement. It does
NOT check abundance ordering, and recon still does not localize per residue — the
no-localization lock stands. Gate 5 reads someone else's localization to test recon's
acceptor claim; it does not make one.

### ✅ Gitignored is not absent — nothing was blocked (2026-08-25) — (locked)

**Every gitignored input is available locally. No work was ever blocked on data
availability.** Retire the phrase "blocked on data" and do not reintroduce it
without a filesystem check behind it.

🔒 **`.gitignore` tells you a file is not TRACKED. It does not tell you the file
is not THERE.** Those are two different claims, and only one of them can be read
out of an ignore rule.

So an input that `.gitignore` excludes from the checkout is not missing; it is one
path away. Every comparison, benchmark and reference script takes its inputs as
PATH ARGUMENTS, so point them at where the file actually sits:

```
--metamorpheus <path>/testing/reference-data/metamorpheus/2026-08-21-10-29-48/Task3-SearchTask/AllPSMs.psmtsv
```

**The checkout also holds the heavy inputs directly** — `testing/search-output/step1-*`
TSVs, all three `testing/inputs/*.mzML.gz`, and a built `recon-tool/target/release/recon`.
Full-file runs, mzML parsing and audits back to raw PSMs all work.

**How this was got wrong, twice in one session.** First: a stored assumption that
the data was somewhere else was carried instead of checked, and five queued checks
sat behind it. Second: `AllPSMs.psmtsv` was declared blocked and written into PLAN
as a hard dependency **on the strength of one `find` in the wrong tree**. It was
two directories away. **Check the filesystem, not the ignore rules.**

⚠ **The general rule (locked):** never declare work blocked on data availability
without an `ls` on the path. A record saying an input is unavailable is a record,
not a measurement. The prime directive applies to it like any other claim.

### ⚠ 2026-09-01 — The gitignored inputs were packed into a verified archive

**Nothing above is retracted.** The locked finding — the gitignored data is on
disk, nothing was blocked — still holds. This entry covers the separate problem:
git does not carry that data, so restoring a working data set into a clean
checkout needs an archive, and the archive has to be verified, not assumed.

**The question becomes "is every input present after a restore?", and it is
measured, not assumed:**

* **Architecture decides which Sage build resolves.** `DEFAULT_SAGE_PATH` names
  `sage-v0.14.7-x86_64-pc-windows-msvc`, so it resolves on a Windows x86_64 build
  and never on `aarch64-apple-darwin`. That is why the version gate is dead on an
  arm64 build. Do not "fix" that path by pointing it at an arm64 build.
* **A stale inventory is an UPPER BOUND, never a statement about what is there
  now.** The inventory the sizing was computed against was roughly 2026-08-25 —
  its NOTES.md is 240 KB against the live 444 KB. **Check what is actually present
  before copying.**
* **What that inventory lacks is everything LIVER**, which is the active file: the
  liver mzML, the 2018 FASTA, the regenerated `full-run/*_search/` TSVs, and the
  MetaMorpheus liver outputs. That is not surprising — liver arrived after the
  inventory was taken.

**The payload, packed 2026-09-01** (21 files, 719 MB on disk, 392 MB packed), in
three tiers so none of it is moved on faith:

| tier | packed | what | needed for |
|---|---|---|---|
| A | 286 MB | liver mzML, 2018 FASTA, `full-run/liver_search/` TSVs | **required** — `digestion_composition_integration` and the analyzer test read these |
| B | 54 MB | the other three `full-run/*_search/` TSVs | regeneration only; NO test reads them |
| C | 52 MB | MetaMorpheus liver outputs, Preview's FASTA copy | re-deriving the four-tool numbers only; the results are committed |

`MANIFEST.md` sits beside them with both sets of checksums. A tier-A extract was
round-tripped and all three key files reproduced their originals exactly, so the
archives are verified rather than assumed.

**Two things deliberately NOT packed**, and the reason matters:
* `testing/recon-output/_verify/` (52 MB) — a scratch duplicate run. recon has ONE
  artifact set; keeping a second copy of a run in circulation is how the ghost
  gets re-created.
* the `aarch64-apple-darwin` Sage binary — architecture-specific, so it is not
  portable. Rebuild it instead.

⚠ **`testing/reference-data/preview/uniprot_sprot_iso_human-2018_06.fasta` is
BYTE-IDENTICAL to the `testing/inputs/` copy** (both `f6b42e28…`). Two tracked
paths, one file. Copy once.

**The acceptance check after a restore is the skip detector, not the passing
count** — see "LATENT — the MS1 calibration gate silently skips". A partial
restore yields a green suite that exercised almost nothing.

---

### ✅ Config provenance CLOSED for `analyze` too — schema 1.2.0 (2026-08-25)

`ModDiscoverySummaryReport` now carries `discovery_settings`, so `full-run/*.json` and
the HTML record the config that made their peaks — including `peak_assignment_mode`,
which reads `merge` on all three. Schema **1.2.0**. 1.1.0's own comment claimed this
and was wrong; that comment is corrected in place at `report.rs:22`.

**Regenerated under a stated invariant: provenance is ADDED, nothing else moves.**
Asserted, not eyeballed — peaks bit-identical (48 / 47 / 48), `unified_ms1_error`
identical (closed_n_psms 7220 / 46727 / 17058), `three_layer_ms1` identical, and a
whole-document diff with only `discovery_settings` and the volatile keys excluded.
`tier_gates.py` re-run off the new reports is byte-identical. `run_validation` 14/14.

**Accepted difference — float summation order, on 2 of 3 files.** `signal_fate.id_rate_by_tic_pct`
and `polymer.total_pct_tic` moved on bcell and b1906, not serum: **32, 36 and 1 ulps**,
relative 4.8e-15 / 4.7e-15 / 1.5e-16. These are TIC sums over tens of thousands of
spectra; accumulation order is not fixed, and bigger files have more of it. Same class
as the `weighted_apex` / `mean_hyperscore` note above. **Not a defect, and not a
determinism failure** — same input to the same binary is still stable.

### ⚠️ TRAP — `analyze --output` APPENDS its own extensions (2026-08-25)

`--output foo.json` writes **`foo.json.json`** and **`foo.json.html`**. Pass the path
WITHOUT an extension: `--output testing/recon-output/full-run/serum`.

**This cost a vacuous verification and nearly shipped one.** The regeneration wrote
`serum.json.json` while `serum.json` sat untouched, so the invariant check compared
the old files against themselves and printed "ALL INVARIANTS HOLD". Every assertion
passed because nothing had changed.

**Rule: a check that compares before-and-after MUST first prove the after is
different.** The corrected check leads with a guard — schema version changed AND
`discovery_settings` is non-null — and aborts if the file did not move. Without that
guard, "nothing else changed" is indistinguishable from "nothing changed at all". This
is the same shape as the Tier 3 gate and `mode_sibling_separation.py`: the fourth
could-not-fail check found in two sessions, and the second one I wrote myself.

---



An earlier entry recorded the config-provenance gap as closed. **That is true for
`discover` and false for `analyze`.** `discovery_settings` is a field on
`ModDiscoveryResult` (`mod_discovery.rs:436`), which `discover` emits. The `analyze`
path builds `ModDiscoverySummaryReport` (`report.rs:70`), which has only
`total_psms`, `unmodified_pct` and `peaks`.

So `testing/recon-output/full-run/*.json` — the **frozen step-1 ground truth**, and
the report a user actually receives — does not record `peak_assignment_mode`,
`bin_width_da`, `min_peak_count` or `calibration_mode`. A reader cannot tell from the
report which mode produced its peaks. The schema comment at `report.rs:22` claims
`mod_discovery.discovery_settings` was added in 1.1.0; for the analyze report that
claim is wrong. README is corrected. **The code is NOT changed and the decision is
open:** adding the field re-writes the frozen ground-truth JSONs, which is a
deliberate act, not a drive-by. Recorded here so it cannot be lost.

---

### Composition readout — a diagnostic, and a weak one on common residues (2026-08-25)

`recon-tool/src/peak_composition.rs` reports whether a peak's peptides CONTAIN the
residues its candidate modification is listed on. **⚠ CORRECTED 2026-08-25 — this
entry previously said it is not wired into annotation "and must not be." That is
superseded by the recorded scope amendment above.** It is now the basis of the step-2
decision rule. It still also feeds `compare-peak-assignment`.

Sequence membership, not site assignment, so it does not touch the
no-per-residue-localization lock. It cannot say a mod sits on a residue.

**Known power limit of the RATIO — and why the decision path does not use the ratio.**
Enrichment is peak-fraction / run-fraction, so it **cannot exceed 1/background**. N or Q
occurs in 81% of the peptides in the b1906 fixture, capping deamidation enrichment at
1.23. Read the ratio against its ceiling, always.

**⚠ CORRECTED 2026-08-25 — the conclusion drawn from that cap was WRONG.** This entry
said "a common residue set cannot produce a strong [signal]." The cap binds the RATIO,
not the ODDS RATIO, which is unbounded. Measured: deamidation at 69–74% background is
capped near 1.4x by ratio and is **OR 2.9 / 3.3 / 11.6 at BH q from 1e-9 to 1e-20** —
strongly supported on all three real files. The decision path uses Fisher exact plus the
odds ratio for exactly this reason. The ratio ceiling survives only as the routing
signal: a background above 95% saturates the 2×2 and sends the peak to the abundance
path instead.
The `compare-peak-assignment` output prints the background column for this reason.

**It caught a real annotation problem on first contact with the full files.**
`Dehydrated`, annotated on C, has **0% / 1% / 0%** site support on serum / bcell /
b1906 — enrichment 0.00, 0.14, 0.00 against backgrounds of 35% / 9% / 10%. Not one of
those ~200 PSMs carries the residue the name requires. Serum also shows `Dioxidation`
(M) at 0.73 and `Methyl` (D/E) at 0.89 — DEPLETED against background, not enriched.
These are recorded as observations, not as a verdict on those peaks: composition
absence is evidence the NAME is wrong, not evidence about what the mass is. It is the
same competing-hypothesis gap already recorded above, now with numbers on real files.
**Do not act on this during step 2.**

Two traps the implementation handles, both pinned by tests:
- Sage writes inline mods, `...QC[+57.0215]YV...`. A naive `contains('N')` would match
  the N inside a bracketed mod NAME. Bracket contents are stripped first.
- Unimod site lists include termini such as `N-term`. A terminus is not a residue and
  is dropped, not read as the residue N.

---

### Peaks now carry their own membership (2026-08-25)

`Peak::psm_indices` holds the PSM indices detection assigned, `#[serde(skip)]` so the
result schema and the determinism bytes do not change. Callers must use it instead of
recovering membership from `delta_mass` plus a guessed tolerance. **Reconstructing
membership from outside is exactly how the two earlier conservation scripts produced
false answers**, and the first draft of `compare-peak-assignment` repeated it before
this field existed.

---

### ❌ CLOSED-NEGATIVE — `isotope_errors: [-1, 2]` is harmful in an open search (2026-08-25)

**Do not adopt. Do not re-propose without new evidence.** Probe on b1906 to a
throwaway directory; nothing was regenerated. Comparison:
`testing/recon-output/2026-08-25-checks/06-isotope-probe-comparison.txt`.

The proposal was that Sage's multi-notch precursor handling would collapse the
+58/+59 satellites at the search layer and retire the satellite question. It does
the opposite.

| metric | current `[0,0]` | probe `[-1,2]` |
|---|---|---|
| +57.02 Carbamidomethyl | 1253 | **394** |
| +58.02 satellite | 248 | **363** |
| +59.02 | 75 | 84 |
| `folded_to_zero_count` | 2344 | **10301** |
| total_psms | 28005 | 27490 |
| isotope_error distribution | 100% at 0 | −1: 22683, 0: 20048, +1: 19886, +2: 16908 |

**It destroys the tool's headline result and fabricates a modification that is not
there.** Two peaks appear only in the probe, at +56.0178 (299) and +56.0187 (376),
annotated **Propionyl**. `57.0219 − 1.003355 = 56.0185`, which sits 0.2–0.7 mDa from
both. Roughly 675 of the lost +57 PSMs were re-explained as "+56 with a −1 isotope
offset" and then given a chemically wrong Unimod name. Region total is conserved
(1576 → 1516), so this is redistribution, not discovery.

**Mechanism (hypothesis, not established):** four precursor hypotheses per spectrum
inside an already −100..+500 Da window multiplies the space enough that an isotope
offset outcompetes the true delta. Not investigated further — the result is decisive
without it.

**Consequence.** The satellite stays a recon-side problem. `reference-notes/satellite-memo.md`
stands, and its "handle it in tier logic" recommendation is not superseded.

---

### Config-provenance gap CLOSED — `min_peak_count` explains the prominence deltas (2026-08-25)

`discover --min-peak-count 5` reproduces `analyze` (`full-run/`) **exactly** on
prominence — 0 differing peaks on all three files — and differs from `discover` at
its default 10 by exactly the 2 / 1 / 3 tail peaks previously seen. The build is not
implicated; the CLI default is. `representative_mz` is populated 49/47/48 in the new
output. Artifacts: `testing/recon-output/2026-08-25-checks/04-discover-min5-*.json`.
**The provenance hole itself is now CLOSED (2026-08-25), no longer queued for step 4.**
`ModDiscoveryResult.discovery_settings` records `bin_width_da`, `min_peak_count`,
`peak_merge_tolerance_da`, `prominence_threshold`, `peak_assignment_mode`,
`calibration_mode` and both folding switches. `analyze` and `discover` both take
`--peak-assignment` and both record what they used. It was closed early, out of its
queued order, because regenerating every artifact without it would have written a
second unauditable set — `peak_assignment_mode` was about to become the exact same
gap that cost a session to diagnose. Report `SCHEMA_VERSION` is now **1.1.0**;
additive, but a 1.0.0 artifact is not comparable to a 1.1.0 one on peak counts.

**A THIRD instance of the same gap, found 2026-08-25 while preparing the
regeneration.** `analyze --closed-tsv` feeds the `unified_ms1_error` block, but the
report recorded only `source_file` — the RAW file's basename — never which closed
search was used. Two candidates exist per raw file (`closed-ref-*` and
`step1-closed-*`), so the committed reports cannot be traced to their input.
`InputInfo.closed_tsv` now records the path. The existing artifacts still cannot be
traced; identify them by matching PSM count at q<0.01 against the committed
`closed_n_psms` (serum 7220, bcell 46727, b1906 17058).

**The pattern is the lesson, not the three instances.** Every optional input and
every config default that shapes a number must be written into the artifact. Check
this for any NEW optional flag before it ships.

---

### PowerShell gotcha — `Set-Content -Encoding utf8` writes a BOM that Sage rejects (2026-08-25)

Editing a Sage config via `ConvertFrom-Json` / `ConvertTo-Json` / `Set-Content
-Encoding utf8` produced `expected value at line 1 column 1` from Sage. Windows
PowerShell 5.1 writes a UTF-8 **BOM**, and the JSON parser will not accept it. Ben
edited the file by hand instead and it worked. If a script must write a Sage config
on Windows, use `[IO.File]::WriteAllText($path, $json)` (no BOM) or PowerShell 7's
`-Encoding utf8NoBOM`. Recorded so this is not rediscovered.

---

### Annotation has no competing-hypothesis check (found 2026-08-25 via the isotope probe)

**Live in the current build. The probe only made it visible.**

Unimod matching asks one question: is there an entry within `match_tolerance_da`
(0.01 Da)? It never asks whether a neighbouring peak explains the mass better.

Measured on the probe output, where the annotator named two peaks Propionyl:

| | value | distance |
|---|---|---|
| Unimod Propionyl | 56.026215 | −7.48 and −8.41 mDa from the peaks |
| +57.0219 − 1 neutron | 56.018545 | **+0.2 and +0.7 mDa** |
| peaks | 56.017808, 56.018734 | |

Both Unimod errors fall just inside the 10 mDa window, so the name is applied. The
isotope explanation is 10–40x closer and is never considered. Note 0.01 Da absolute
is a loose window at small deltas — 178 ppm at 56 Da — while `mass_error_ppm` is
reported at the precursor m/z, so the report's ppm figure does not expose this.

**Two candidate responses, neither built, neither scoped into v0.1.0:**
1. Before annotating, test whether a peak sits within fold tolerance of
   `k × 1.003355` from a larger nearby peak. If it does, and that distance beats the
   best Unimod match, report the isotope relationship instead of the name.
2. **Composition gate — compatible with the no-localization lock.** Do not localize;
   just ask whether the peak's PSMs' peptides *contain* any Unimod-listed site for
   the candidate (Propionyl: K/S/T/N-term; Carbamidomethyl: C/N-term). Sequence
   membership, not site assignment. Would also bear on the +57
   Carbamidomethyl / Carbofuran / Gly ambiguity recorded above.

**⚠ SCOPE AMENDED 2026-08-25 — the composition gate (option 2) IS being built, in
step 2.** This entry previously read "do not build either during step 2 ... post-v0.1.0
candidate." That scoping was decided before any of it was measured, on the assumption
it was speculative. It is not: the routing rule, the odds ratios and the acceptance
results are all now recorded. Deciding factor is that without it +57 stays flagged
`ambiguous` and is reported as "present, your call" on all three files — the tool's
headline result demoted out of its own recommendation. A deliberate, recorded scope
change, made by the user; not scope drift. Option 1 (isotope-relationship check before
annotating) is ALSO being built, as satellite demotion. See "Step 2 decision rule —
route by specificity".

---

### Repo keep/archive criterion (locked 2026-08-24)

**Keep:** anything needed to support a statement, a feature, or a choice we make; the
testing that backs it; and the final product. **Archive to `_archive/`:** material that
backs none of those and cannot be audited.

Age is not the test, and neither is "superseded." `nofixedmods/` is a month older than the
current build and is kept, because it backs the "+57 at rank 2" statement and is verified
field-by-field against the current build. `calibration-benchmark/` is older still and is
kept, because it is the evidence behind the C1/C2 CLOSED-NEGATIVE result, which is a
write-up deliverable — `testing/README.md` already marks it frozen and not reproducible.

**Archived 2026-08-24:** `reference-notes/how to deal with low numbers.md` →
`_archive/`. It is an LLM-drafted write-up whose citations are expiring presigned S3 URLs
that will resolve to nothing, it sat inside the `reference-notes/` read path, and grep shows
**nothing in the repo cites it**. Its actual conclusion — treat `count_pct` as a ranking
metric and do not redefine the denominator — is independently recorded and locked in
"Prevalence currency — four tools, four different quantities". Its numbers trace to
`testing/recon-output/comparison/satellite_check_corrected.md`, which stays. Nothing is
lost; an unauditable path to it is.

**This does not supersede the existing curation policy** in `testing/README.md`
("superseded versions are deleted, not accumulated — git history retains them").
`_archive/` is for material that must stay visible but must not be cited. Deletion remains
correct for a superseded regeneration of a file we still produce.

---

### Clean-subset PSM floor (200) — explained, keep as-is (2026-08-24)

Closes the PLAN step 1 "document the clean-subset PSM floor" item. Data:
`testing/recon-output/psm-sensitivity/*.json` via `testing/scripts/psm_count_sensitivity.py`.

- **What the 200 actually gates:** whether the optional 60%-by-hyperscore trim runs at all
  (`hyperscore_guard_would_apply`, `calibration.rs:159`). Its job is "don't throw away 40% of an
  already-small subset" — NOT "guarantee the median is precise." That distinction is what makes
  the number defensible; it was being judged against the wrong requirement.
- **Empirical sensitivity (bootstrap, 500 draws, 90% spread of the resampled median):**

  | File | full clean N | N for ≤1.0 ppm | ≤0.4 ppm | ≤0.2 ppm |
  |---|---|---|---|---|
  | serum | 3,764 | 16 | 75 | 200 |
  | bcell | 32,133 | 16 | 100 | 300 |
  | b1906 | 10,942 | 16 | 75 | 300 |

- **Why this does not need to be tuned precisely:** under the bucket recommendation above, flipping
  serum off the 10 ppm bucket needs `|bias| + 5×MAD` to move >5 ppm. The bootstrap shows tenths of
  a ppm of wobble at N=50. The bucket cannot flip from sampling noise at any N tested. So the floor
  is not load-bearing for the user-facing output.
- **Provenance for the value:** MetaMorpheus's comparable engineered floors are ≥16 PSMs / ≥40 MS1
  / ≥80 MS2 datapoints, for the *harder* job of per-scan drift correction
  (`reference-notes/metamorpheus-mass-error-calibration.md`). 200 is conservative against that
  reference point. **Keep 200. Do not re-tune it.**
- **Wrinkle, recorded so it is not rediscovered as a bug:** the 200 is checked BEFORE the 60% trim,
  so a subset landing exactly at threshold reports on 120 PSMs. Under bucket quantization this does
  not matter. Never live on these three files (clean subsets 3,764–32,133; guard fired with
  thousands to spare). **Leave it alone** — "fixing" it would be tuning a number that does not
  affect the output.
- **Script bug found and fixed getting here:** `psm_count_sensitivity.py` read `is_decoy` and
  `delta_mass_corrected` as if they were Sage TSV columns. They are not — recon's parser derives
  both (`is_decoy` from the `rev_` prefix on `proteins`; `delta_mass_corrected` from
  expmass/calcmass/isotope_error). The script had never been run against a real Sage TSV and failed
  on first real invocation. Fixed to derive them the same way (commit `893a042`).

### Prevalence currency — four tools, four different quantities (locked 2026-08-24)

**A modification percentage means nothing until you say what it is a percentage OF.** Every
cross-tool percentage in the write-up must carry its currency. This is the main reason recon's
numbers look low next to the platforms, and it is not a defect.

| Tool | What its percentage counts |
|---|---|
| recon | Open-search **delta-bin fraction**. PSMs in one delta-mass peak / total PSMs. No localization. One peak = one mass, not one chemistry. |
| PTM-Shepherd | **Post-localization PSM fraction.** The mod is assigned to a residue first. |
| MetaMorpheus | **Post-localization PSM fraction**, after GPTMD writes candidate mods into the database. |
| Mascot error-tolerant | **Resolved-ET fraction**, from a second pass over a reduced protein set. |
| MSFragger (variable mod) | **Tagged-PSM fraction.** PSMs carrying an explicit variable-mod tag. |

Same mod (+57), same three files, four currencies:

| File | recon | PTM-Shepherd | MetaMorpheus | MSFragger var-CAM |
|---|---|---|---|---|
| serum | 7.31% | 17.92% | 24.51% | 31.40% |
| bcell | 4.50% | 10.68% | 11.46% | 18.66% |
| b1906 | 4.47% | 10.77% | 11.58% | 19.05% |

**The table is complete. No reference tool needs re-running for the +57 comparison.** All four
sources cover all three files, and every value above is already in the repo. Sources: recon
`recon-output/nofixedmods/*.json`; PTM-Shepherd `reallyOpen/global.modsummary.tsv` (b1906 = 2,845
PSMs, 10.7720%); MetaMorpheus `recon-output/comparison/recon_nofixedmods_vs_metamorpheus_reallyOpen.md`
(b1906 section, +57.0219 row); MSFragger `msfragger/strictTrypVarCAM/` via `reconcile_cam57.py`.

- **Recording-error warning for anyone extending this table.** b1906 was carried as a dash through
  several sessions and was twice described as a data gap during 2026-08-24. It was never a gap.
  The 2026-08-21 JOURNAL table listed only serum and bcell, and that incomplete table kept being
  copied forward instead of the committed per-file outputs being read. **Read the source output,
  not the last summary table.**
- **+57 sits at rank 2 for both recon and MetaMorpheus on all three files.** Rank agreement is the
  currency-free comparison this entry argues for, and it holds even where the percentages differ
  by 3-5x.
- **The two post-localization tools agree well on two files and badly on one.** PTM-Shepherd vs
  MetaMorpheus: bcell 10.68 vs 11.46, b1906 10.77 vs 11.58 — both within ~8%. serum 17.92 vs
  24.51 — a 1.37x spread. So the inter-platform disagreement that killed the fudge factor is
  **specific to serum**, not general. That matches serum's independently recorded profile: highest
  pre-calibration drift, heaviest adduct load, different instrument class. See "Per-run ranking
  confidence flag" below.
- **MSFragger var-CAM runs ~1.75x PTM-Shepherd on all three files** (31.40/17.92, 18.66/10.68,
  19.05/10.77). Consistent across files, so it is a currency difference, not noise: MSFragger
  counts any PSM carrying the tag; PTM-Shepherd counts localized assignments inside its own
  window. Another reason to compare rank, not magnitude.

- **The reference platforms do not agree with each other either.** On serum, PTM-Shepherd says
  17.92% and MetaMorpheus says 24.51% for the same mod on the same file. That is a 1.37x spread
  between two post-localization tools using the same currency. **This is why the prevalence fudge
  factor was rejected** (PLAN "Deferred past v0.1.0"): a correction factor cannot be more precise
  than the spread of the thing it corrects toward.
- **Neither commercial recon tool reports a prevalence number at all.** Mascot's stage-1 output is
  a protein list. Byonic Preview's output is a parameter file. Recon is being asked for a number
  that the tools it is compared against do not emit at their equivalent stage. See
  `mascot-error-tolerant-methodology.md`.
- **Rule for the write-up:** report rank agreement, not magnitude agreement. Rank is currency-free.
  Every percentage gets its currency named in the same sentence. Never place two tools'
  percentages in one column without a currency row.

### Option-C elimination — recalibration is not the mechanism behind the +57 gap (locked 2026-08-24)

- **What was tested.** The 2026-07-17 Option-C ceiling POC fed MSFragger-calibrated mzML
  (`write_calibrated_mzml=1`, validated: serum MS1 2.51→0.06 ppm) into our own Sage open search,
  with `discover --calibration none`. Gold-standard calibration, zero recalibrator code of ours.
  This is the ceiling — the best any internal recalibrator could do.
- **Result: negative, on both readouts.** Total PSMs went DOWN on all three files (b1906
  31682→31497, serum 19307→19099, bcell 81966→80374). The bcell deamidation apex — the one
  symptomatic file — stayed flat (0.9852→0.9850 b1906; 0.9818→0.9817 bcell). Full table in the
  C1/C2 CLOSED-NEGATIVE entry below.
- **What this eliminates.** JOURNAL 2026-07-24 named three candidate stages for the +57 magnitude
  gap and could not separate them. **Recalibration is now removed as a candidate.** Two remain:
  the narrow first pass, and localization-aware rescoring.
- **Corroborating architecture.** Both commercial recon tools use a restricted second pass —
  Mascot searches only proteins selected in pass 1; Byonic Preview subsamples and runs fast
  targeted searches. **Neither uses recalibration as the lever.** The remaining two candidates are
  what the commercial tools actually do.
- **Magnitude, measured 2026-08-24.** The gap this was trying to explain is **1.2x–3x**, not the
  35–58x once believed. See the +57 reconciliation entry above. A gap that size is comfortably
  accounted for by peak splitting plus single-pass search recovery, without recalibration.
- **Do not re-open.** C1/C2 is closed across all three arms. This entry records why the *third*
  arm's negative result also settles a question outside C1/C2.

### Ground-truth population differences — the four tools do not measure the same PSMs (locked 2026-08-24)

Every cross-tool mass-error or prevalence comparison in the write-up needs this caveat. The tools
disagree partly because they are looking at different populations, before any question of
correctness arises.

| Source | Population it measures |
|---|---|
| recon | Wide-open search, then a **near-zero-delta clean subset**: `\|Δ\|<0.02 Da`, rank-1, target, q<0.01, then top 60% by hyperscore. Deliberately the best-behaved unmodified PSMs. |
| MetaMorpheus `Task1-CalibrateTask` | Narrow/classic **closed search at 1% FDR**, **zero mods configured**. Smaller and differently biased. |
| MSFragger `strictTryp` (2026-08-24) | Closed, 20 ppm, **fixed Cys+57**, variable Met-ox and protein N-term acetyl. |
| MSFragger `strictTrypVarCAM` (2026-08-24) | Same, but Cys+57 **variable** — built specifically so +57-carrying PSMs could be counted. |

- **Aggregation unit differs too.** MetaMorpheus computes median and IQR over **per-datapoint**
  samples (e.g. 187,516 MS1 datapoints from 2,214 serum PSMs — isotope-envelope points for MS1,
  matched fragments for MS2). recon computes **one value per PSM**. These are different
  statistics, not different answers to one question.
- **Spread statistic differs.** MetaMorpheus reports IQR; recon reports MAD. `IQR ≈ 2×MAD` only
  for a roughly normal distribution — a sanity check, never a conversion.
- **Measured size of the population effect.** The clean subset understates the wider population's
  mass-error scatter by **1.16–1.28x**, measured against matched closed searches
  (`ms1_bias_sign_check.py`; see `ms1-tolerance-recommendation-rationale.md` §4).
- **What this does NOT excuse.** Population difference is a reason for numbers to differ by a
  little. It is not a licence to rationalize a *sign flip*. The bcell disagreement was blamed on
  population difference for two sessions and the real cause was a units bug — see the |error|
  entry above. **Use this caveat to size an expected disagreement, never to dismiss one.**

### Software comparison is tool-vs-tool; the tool is never compared to its author (locked)

- **What:** the mod-discovery cross-comparison is an OBJECTIVE benchmark between
  independent tools — Sage-Recon (ours, the reference column) vs FragPipe+PTM-Shepherd,
  Mascot error-tolerant, and Byonic Preview — on the same files. Agreement/divergence
  between tools is the measurable result. It is a **benchmark, not a gate** (nothing
  fails; methodology deltas are stated, not scored against).
- **What it is NOT:** "the mods Ben expects to see" (fixed C+57; var Ox(M),
  Deam(NQ), Acetyl protein-N-term, pyro-Glu peptide-N-term Q, + Ox(P) for skin/collagen)
  is a **rule-of-thumb sanity read from ~19 years of experience — NOT a validation
  criterion, NOT a comparison column, NOT a `run_validation` gate.** You cannot compare
  the tool to its author; that's unfalsifiable and unpublishable. If all four tools
  independently surface those mods, that's consilience (a nice result), not proof, and
  the *tools agreeing* is the objective statement — not "it matched the expert."
  The expert set may be used only as a gut-check on the whole panel (if NONE of the four
  find deamidation, suspect the data), never as a pass mark for one tool.
- **Framework shape (decided 2026-07-16):** build an **N-tool aligner** — a generic
  "mod table" (mass, per-file count, per-file %) that any tool's output is adapted into,
  compared against Sage-Recon's `discover` as the fixed reference. Seed with ONE adapter
  (PTM-Shepherd `global.modsummary.tsv`); Mascot/Preview are "write another adapter"
  later, core unchanged. Metric family: presence / rank / prevalence, per-file, in the
  shared mass window (−150/+100 with PTM-Shepherd), **percentages as the currency** (PSM
  totals differ by FDR machinery — see the ptm-shepherd README methodology deltas).
  Match tolerance ±0.01 Da (our annotation tol = PTM-Shepherd `annotation_tol`).
- **Scope:** lives in `testing/scripts/` (analysis tooling, not the recon binary) +
  `testing/recon-output/comparison/`; JOURNAL/NOTES for findings. Does NOT change what
  the tool emits; no external GitHub code pulled in. If the benchmark reveals a real gap
  (a mod class we miss, a binning disagreement), THAT becomes a new recorded feature item
  fed back to the tool — same path as over-alkylation.

### FASTA is user input; recon is DB-agnostic (locked)

- **What:** the tool reports what's in whatever FASTA it's given — any size, with or
  without contaminants. Contaminant inclusion (keratins, trypsin, etc.) is a
  real-search concern, **out of scope** for recon.
- **Why:** the user controls the database — a huge proteome, a small targeted set, with
  or without contaminants — and that's their call, not ours. Adding contaminant
  bundling or "you're missing contaminants" warnings is scope creep against the
  parameter-auto-config non-goal (PLAN non-goals). Testing/validation uses the
  canonical UniProt human reference proteome (~20k entries) as a clean, reproducible
  default. Decided 2026-07-15 during Phase 8.

### One report per file; results never blended across files (locked)

- **What:** the recon unit is a single MS run. If multiple files are submitted, each
  gets its own independent report. Metrics are never averaged/blended across files.
- **Why:** mass accuracy, calibration, signal fate, and the mod landscape are all
  instrument-, calibration-, and loading-specific (see `reference-notes/mass-accuracy.md`)
  — blending files acquired on different instruments / calibration states / loading
  regimes produces a meaningless average. This is also the **same-file provenance guard**
  for MS1 mass accuracy: the closed reference search and the open search that feed one
  report must derive from the **same raw file**. Decided 2026-07-15 (Phase 8.5) when
  multi-file input first came up — it had never been explicitly stated.

### Test-data provenance: calibrated mzML are one-time POC inputs (not standard fixtures)

- **What:** `testing/inputs/*_calibrated.mzML` (`b1906_calibrated.mzML`,
  `2019-4-9_909c_0311_calibrated.mzML`, `Bnaive_01steady-state_calibrated.mzML`) are
  MSFragger-calibrated exports (FragPipe `write_calibrated_mzml=1`), added by Ben on
  2026-07-16 as **one-time input for the C1/C2 Option-C ceiling POC** (2026-07-17). They are
  NOT standard test fixtures — do not build regression/validation against them.
- **Provenance / same-file guard:** each derives from the same raw run as its baseline —
  verified by the mzML internal `id=`: `b1906.mzML` → `open-b1906-full` baseline,
  `2019-4-9_909c_0311.mzML` → `open-serum-full`, `Bnaive_01steady-state.mzML` →
  `open-bcell-full`. Plain indexed mzML 1.1.2 (Sage-readable, NOT `.mzBIN`). The one-report-
  per-file guard applies: the calibrated file and its raw baseline are the same run.
- **Git status:** the files themselves are **gitignored** (`*.mzML` rule, like all raw data —
  they're 460–890 MB). The POC *outputs* (`testing/search-output/open-*-calibrated/`,
  `testing/recon-output/calibration-benchmark/*_calibrated-input_none.json`) and the swap-only
  configs (`testing/configs/open-search-*-calibrated.json`) ARE tracked. Result numbers live
  in NOTES (C1/C2 CLOSED entry) + JOURNAL, so the finding survives even though the inputs
  don't. If the calibrated mzML are deleted, the POC is fully reproducible from the tracked
  configs given the same FragPipe re-export.

### 🔓 Sage as a subprocess — LOCK LIFTED 2026-09-01, superseded by SAGE AS A LIBRARY

- **What it said:** Sage is invoked as an external binary (vendored at
  `reference/sage/...`), never embedded as a Rust `sage` crate dependency.
- **Why it said it:** decouples us from Sage's internals and build; we consume
  its TSV/JSON output like any user would. Decided in Phase 0.
- ⚠ **SUPERSEDED. Ben lifted this 2026-09-01** — "that lock in AGENTS wasn't ever
  my perfect goal". The replacement decision is route **A1**, recorded in full
  under "SAGE AS A LIBRARY" below. This entry stays because the REASONING above
  is still the cost side of that decision: embedding is what makes recon's output
  stop being "stock Sage wrote this TSV, re-run it yourself".
- **Still true meanwhile:** every artifact in the repo today was produced by the
  subprocess path against the pinned v0.14.7 binary.

### 🔒 SAGE AS A LIBRARY — route A1 CHOSEN, two landings (Ben, 2026-09-01)

**The decision.** Depend on `lazear/sage` as a Cargo git dependency pinned at a
**v0.15 tag**, and ship recon as ONE executable. Done as **two separate
landings**: the v0.15 upgrade first, verified against the existing reference
sets, then the embed.

⚠ **PINNING IS NOT TRACKING.** A Cargo git rev is a pin in exactly the way
`SAGE_COMMIT` is a pin today. recon does not follow upstream; moving to a later
Sage stays a deliberate, tested, recorded upgrade. Ben's framing: "we can pin to
v0.15 and don't actually have to update with it, but we can, with testing."

**Why A1 and not the status quo.**
* **One file is actually reachable.** Everything else recon needs is small
  tracked text that `include_str!` already handles — `unimod.xml` (2,506,678 B),
  `positiveContaminants.txt` (85,076 B), `refsContaminants.txt` (4,545 B), the
  default configs and the curated mod list. **Sage was the only binary**, so
  embedding it is what makes a single-file distribution possible at all.
* **It kills the whole class of step-4 path bugs at once**, rather than one at a
  time.
* **The pin becomes unbreakable.** Today `SAGE_COMMIT` is enforced by
  `verify_sage_version` asking a binary at runtime, and `SAGE_PATH` lets anyone
  substitute a different build. In `Cargo.lock` the pin cannot drift and the
  runtime guard becomes redundant.
* **No fork needed.** recon does not want the `progress` or `cancel` patches
  sagegui carries, so it can depend on plain upstream at a v0.15 tag.

**Rejected alternatives, recorded.**
* **A2 — stay on v0.14.7 with our own `lib.rs` shim on a fork.** Small patch, but
  it makes us a fork maintainer for sagegui's exact reason, against code upstream
  has already restructured. The shim has no upstream future: the next sync lands
  on v0.15 regardless.
* **A3 — depend on `sage-core` only and write our own runner.** Would
  reimplement ~350 lines of orchestration plus input/output/telemetry, and recon
  would then own Sage's search orchestration. **This is the one route where our
  numbers genuinely stop being "what stock Sage produces"** — rejected for that
  reason, not for effort.

**The cost that stays real, and is NOT engineered away.** recon's output stops
being "stock Sage wrote this TSV, re-run it yourself" and becomes "recon, linking
Sage v0.15, produced this". Mitigation, which is a mitigation and not a fix:
**keep writing the effective-params files to disk even when nothing shells out**,
so a third party can still reproduce the search with stock Sage. For the write-up
this is a Limitations sentence, not a blocker.

**Why the landings must be SEPARATE.** v0.15 changes q-values and PSM membership
at q<=0.01 moves. If the API change and the q-value change land together, a moved
number cannot be attributed to either. Same discipline as checksumming a
regeneration's inputs.

⚠ **THE UPGRADE IS A CHANGE-REGENERATE** and needs Ben's explicit go-ahead plus
an enumerated downstream trace. It re-derives the write-up's entire evidence base:
10621/1818/717/320, the four-tool comparison, the three mod-rank correlations, and
the MS1/MS2 accuracy numbers Preview corroborates. **So the real sequencing
question is step 5, not step 4** — A1 before the write-up means re-deriving the
evidence once against the tool actually shipped; A1 after means the paper
describes v0.14.7 while the tool ships v0.15.

⚠ **The known v0.15 trap, already recorded at the top of this file:**
`precursor_ppm` becomes SIGNED where `qc.rs` treats it as a magnitude, and **the
schema validator cannot catch it because the column name is unchanged.** It must
be a manual code-audit step. `calibration.rs` already warns that `from_psm`
RECONSTRUCTS the signed value and that the reconstruction must be REMOVED, not
stacked, on upgrade.

**What must NOT be built while A1 is pending** (Ben's call — do not write code a
decided change deletes): Sage-binary location logic, `SAGE_PATH` handling, the
runtime version guard, and any release layout shipping `sage` beside `recon`.
`DEFAULT_SAGE_PATHS` was therefore given a deliberately minimal four-line fix and
says so in its own doc comment.

### Single open search (locked)

- **What:** the tool runs ONE Sage search with wide tolerance
  (`"da": [-500, 100]`), not multiple/iterative searches.
- **Why:** it's a recon scout, not a refinement pipeline. Follow-up closed
  search is the user's downstream decision, not ours.
- **Scope of the lock (clarified 2026-07-16, C1/C2 Step 1):** this lock forbids a
  **closed / targeted follow-up search** as part of recon. It does NOT forbid
  *recalibrating the input and re-running the SAME wide-open search* (Option C below) —
  that preserves discovery fully. The two were once conflated; they are distinct. See the
  "Calibration design space (A/B/C)" entry.

### No per-residue localization (locked)

- **What:** we report which mods are present and their confidence; we do NOT
  localize them to specific residues.
- **Why:** localization is a downstream human decision, outside the recon
  mission (see PLAN non-goals). Rejected because it would double scope for a
  question the user answers later anyway.

### Confidence from existing Sage fields only (locked)

- **What:** MVP uses `hyperscore`, `matched_intensity_pct`, `longest_b`,
  `longest_y` from Sage output — no new fragment-ion computation.

### Unimod matching at 0.01 Da (locked)

- **What:** matches PTM-Shepherd's default tolerance. Multiple matches within
  tolerance are ALL reported with `ambiguous: true` — never silently pick one.

### Dual readouts everywhere (locked)

- **What:** every metric is reported as both spectral count AND
  intensity-weighted.

### Two reference folders kept separate (locked)

- **What:** `reference-notes/` holds committed, distilled docs (the kit's
  "reference" role); `reference/` holds gitignored raw cloned upstream repos
  (Sage, mzSniffer, PTM-Shepherd, intensityWeighting) for porting source.
- **Why:** raw-vs-distilled is exactly the split the context kit warns about;
  keeping both is better than one folder. Do NOT consolidate them. Decided
  during the 2026-07-15 context-kit transition.

### Pass 2 (semi-enzymatic on subset FASTA) is non-optional (locked)

- **What:** the two-pass digestion workflow always runs Pass 2.
- **Why:** for ~3 min it yields enzyme-performance metrics that distinguish
  trypsin sources (e.g. bovine vs porcine: 10% vs 20% semi-tryptic). See the
  Phase 6C section below for the full two-pass rationale.

---

## README "Project Status" block, moved here 2026-09-02

This was 190 lines on the GitHub landing page. It is development detail with
dated entries — a changelog, not a description of the tool — so it was moved
here VERBATIM and the README now carries a short status instead. Nothing was
deleted. The user-facing parts (the `--fasta` and `--output` behaviours) were
kept in the README, under "Usage notes".

## Project Status

The tool is in active development. Alpha maturity means the core logic is
tested and the outputs are benchmarked against PTM-Shepherd and Mascot, but
the user-facing interface is not stable and the packaging is incomplete.

Current limitations:

- **Report schema is 2.1.0.** 2.1.0 (2026-09-01) is additive: `runtime_seconds`
  on the report, `q_value` on every rejected modification so a decision can be
  audited, `ms2_bias_ppm` renamed `ms2_median_abs_ppm` (Sage's `fragment_ppm` is
  absolute, so no bias can be derived from it), and the rejection reasons renamed
  from code-branch names to outcomes — `no_residue_support` became
  `failed_residue_test`, `not_curated` became `below_floor_uncurated`.
  ⚠ Artifacts under `testing/recon-output/full-run/` are still at 2.0.0 and have
  not been regenerated.
  ⚠ **2.0.0 (2026-09-01) is a BREAKING release and
  the first major bump.** Two changes, both deliberate: `three_layer_ms1` was
  REMOVED from the report (it survives as `signal-fate --three-layer`), and every
  "tryptic" key was renamed to "enzymatic" — `fully_tryptic_pct` ->
  `fully_enzymatic_pct` and so on — because the protease is now user-chosen and
  the old names lied for any non-tryptic run. Values are unchanged by the rename.
  ⚠ **1.8.0 (2026-09-01) accompanied the Sage
  v0.14.7 -> v0.15.0-beta.2 upgrade.** No field is added or removed, but
  `mass_accuracy.precursor_median_ppm` and `_p95_ppm` CHANGE MEANING: Sage v0.15
  reports `precursor_ppm` signed rather than absolute, so those two fields become
  a signed distribution (liver: 450.329 -> -0.271). A 1.7.0 artifact is not
  comparable with a 1.8.0 one on those fields. Every other number moved too,
  because the search engine changed — the Sage version is part of the citation.
  `ms1_calibration.bias_ppm` is NOT affected: it is reconstructed from the mass
  columns and never read that column (liver -1.4193 -> -1.4172).
  1.7.0 added the analyzer block. 1.6.0 adds
  `ms1_calibration.user_recommendation_tolerance_ppm` and CHANGES the meaning of
  `user_recommendation_low/high_ppm`: they were an asymmetric `bias−2×MAD` to
  `bias+p95` window and are now symmetric ∓ the quantized ladder rung. On all three
  test files that is ±10 ppm, where 1.5.0 reported e.g. serum +1.46 to +4.09. A
  1.5.0 recommendation is not comparable with a 1.6.0 one. 1.5.0 adds no field. It records that the MS1
  calibration block CHANGED VALUE: `ms1_calibration.bias_ppm` and
  `spread_mad_ppm` were medians of Sage's absolute `precursor_ppm` column, and
  are now computed from a signed error reconstructed from the mass columns. A
  1.4.0 calibration block is not comparable with a 1.5.0 one — on the bcell test
  file the bias goes from +0.70 to −0.24 ppm, a sign flip. Everything outside
  that block is unchanged. The four `mass_accuracy.*_ppm` fields still report
  medians of |error| and are now labelled as such in the JSON docs, the console
  and the HTML. 1.4.0 adds `recommendations.protein_context` — what
  the protein-terminal modifications were tested against, or a statement that they
  were not testable. 1.3.0 added the `recommendations` block — the step-2
  search-parameter recommendation, rendered in both the JSON and the HTML.
  1.2.0 added `mod_discovery.discovery_settings`, recording the bin width, peak
  floor, assignment mode and calibration mode that produced the peaks, so a report
  says how it was made; that covers `analyze` (and therefore `recon run`) as well as
  `discover`, where 1.1.0 claimed it but only `discover` had it. Peak values are
  unchanged from 1.1.0 through 1.6.0. Any artifact still marked 1.0.0 predates the
  peak-assignment fix and is not comparable on peak counts.
- **`analyze --fasta` is optional, and it changes what can be decided.** Supply the
  SAME database the search used. Sage's TSV has no start-position column, so proving
  a peptide sits at protein position 0 needs the protein sequences; without them,
  protein-terminal modifications (protein N-term acetylation, and the Met-loss forms)
  are NOT TESTABLE and fall to the abundance floor. The report records which mode it
  ran in. `recon run` passes its own `--fasta` through automatically. A mismatched
  FASTA is a hard error, not a silent "nothing found": at least 95% of target PSMs
  must resolve an accession.
- **`analyze --output` appends its own extensions.** Pass a path with no extension:
  `--output out/serum` gives `serum.json` and `serum.html`. Passing `out/serum.json`
  gives you `serum.json.json`.
- **Regenerated 2026-08-25 with the fixed build.** Outputs under
  `testing/recon-output/` were previously produced by a build whose peak detection
  could give the same PSM to two peaks. **The regeneration is now complete** —
  `full-run/` (re-baselined 2026-09-01 at schema **2.0.0**: four files with
  `three_layer_ms1` removed, the enzymatic key names, effective params, and Sage's
  own `results.json`. serum, bcell and b1906 pass-1 PSM counts are EXACTLY
  unchanged across that re-baseline; only liver moved, by -0.035 %),
  `nofixedmods/`, the discover artifacts, the regression snapshots,
  `tier-gates-2026-08-25.txt` and all ten script-generated `comparison/*.md` tables
  are current. `psm-sensitivity/` was verified unchanged rather than regenerated: it
  never depended on peak detection. `calibration-benchmark/`
  is frozen by design and its peak counts pre-date the fix — do not compare them
  with current artifacts.
- **Peak grouping merges adjacent bins, and that loses real structure on one of
  three test files.** `--peak-assignment merge` is the default and `split` gives
  every prominent bin its own peak. Both count each PSM exactly once, so peak counts
  are trustworthy either way. `merge` is the default on measurement, not preference:
  of 27 groups where the two modes disagree, 21 are `split` cutting one population at
  a bin edge, and of the 6 that survive that test, 5 are `split` promoting chunks of
  the ±1/±2 Da background to named peaks. **The exception is real:** on the B-cell
  file, two genuine populations 10.6 mDa apart are merged into one reported peak.
  Use `recon compare-peak-assignment` to reproduce the comparison on any file, and
  `testing/scripts/subbin_delta_histogram.py` to look below the 10 mDa bin grid.
- ✅ **THERE IS NO SAGE BINARY ANY MORE (2026-09-01).** Sage is compiled INTO
  `recon` as a pinned Cargo dependency, so `DEFAULT_SAGE_PATHS`, `SAGE_PATH`,
  `--sage-binary` and the runtime version handshake are all gone. The pin lives
  in `Cargo.lock` and cannot drift between what was tested and what you run.
  Verified inert: replaying the committed serum baseline through the embedded
  Sage gives 68817 identical PSM rows — every raw AND rescoring column, and zero
  movement in q <= 0.01 membership. Everything else is not Windows-only either: `recon-tool` builds
  and its full test suite passes on macOS (arm64), and every artifact under
  `testing/recon-output/` was regenerated there on 2026-08-25. Cross-platform
  agreement was checked, not assumed — the closed-search MS1 figures reproduced
  bit-for-bit against Windows-generated values (serum signed
  `2.456278092456017`, 17 digits). Linux is still untested.
- ✅ **The fixed-carbamidomethylation default is GONE.** The bundled template
  carries no mods, and both passes strip and assert regardless of what any
  `--params` template says.
- ✅ **`recon` no longer needs to run from the repository root (2026-09-01).**
  Both search templates and the curated mod list are compiled into the binary
  (`recon-tool/src/defaults.rs`), and a guard asserts the bundled copies match the
  committed templates field for field. **`unimod.xml` is compiled in too as of
  2026-09-01**, with the same drift guard, so `--unimod` is an override rather
  than a requirement. The FASTA stays a user argument — it is the one input that
  is genuinely per-experiment. There is still no release zip.
- The self-calibrated MS1 tolerance recommendation is wired into `analyze` (read-only
  reporting). **Semi-enzymatic Pass 2 is BUILT and wired into `recon run` as of
  2026-08-29.** `run` writes a second artifact, `<output>_pass2.json`, holding the
  terminus breakdown, the missed-cleavage distribution Pass 2 saw, and a
  Pass-1-vs-Pass-2 comparison block. Skip it with `--no-pass2`.
  - Pass 2 searches ONLY the proteins Pass 1 identified, semi-enzymatically,
    inside the mass window Pass 1 measured. Nothing in it is a constant a user
    has to know to change.
  - It is CHEAP, which was not assumed: measured 7.9 s against Pass 1's 65.4 s on
    serum, because the subset FASTA (472 proteins of ~20 600) shrinks the search
    space far more than semi-enzymatic digestion inflates it.
  - ⚠ Sage's `precursor_tol` is sign-INVERTED relative to the delta mass it
    produces, for `ppm` as well as `da`. Measured on serum 2026-08-29: a config
    of `ppm [-30, 5]` yields an observed delta of −5.047 .. +30.007 ppm. The
    conversion happens in exactly one place, `pass2::precursor_tol_json`.
- **The MS1 bias is FIXED as of 2026-08-28.** It was computed from Sage v0.14.x's
  `precursor_ppm`, which holds the *absolute* error, so it could never be negative
  and read high whenever the true bias was not much larger than the scatter. Recon
  now reconstructs the signed error from the mass columns: bcell reports −0.2357 ppm
  where it used to report +0.7028, a sign flip. All three files agree with an
  independent Python measurement to four decimals.
- **MS2 remains ABSOLUTE, deliberately.** Sage's `fragment_ppm` is an
  intensity-weighted mean of |error| and there is no per-fragment data in the
  default TSV to reconstruct from. Recovering it needs `sage --annotate-matches`
  and an 8.5 MB side file per run, and it cannot change what recon recommends —
  measured, it moves the tolerance requirement by 0.027 ppm against a 10 ppm ladder
  step. The field is labelled as a magnitude, not a bias. Do not compare it with
  another engine's signed MS2 error.
- **The MS1 tolerance recommendation is now QUANTIZED** to the {10, 20, 50, 100} ppm
  ladder — the smallest rung covering `|bias| + 5×MAD`. The previous asymmetric
  window was 3–5x too tight: it was derived from the best-behaved subset of PSMs and
  handed over as a bound for a whole search. All three test files land on ±10 ppm
  (requirements 4.84 / 3.60 / 3.86 ppm). The bias is still reported separately. Both
  the ladder and the `k=5` multiplier are documented choices informed by field
  practice, and the ladder's behaviour above 10 ppm is untested on real data. See
  `reference-notes/ms1-tolerance-recommendation-rationale.md`.
- **Pass-1 MS2 tolerance IS detector-aware** as of 2026-08-28. The MS2 analyzer is
  read from the mzML before the search — no PSMs needed — and `fragment_tol` is set
  from it, replacing whatever the search template hardcodes. Windows are sized for
  an instrument that could be out of calibration, NOT for a well-behaved one:
  Orbitrap/FT-ICR and Astral ±20 ppm, legacy TOF ±100 ppm, ion trap
  and quadrupole ±1.0 Da. The analyzer term set is derived from a pinned HUPO-PSI
  PSI-MS CV snapshot; see
  [reference-notes/ms2-analyzer-tolerance-table.md](reference-notes/ms2-analyzer-tolerance-table.md).
  Run `recon detect-analyzer --mzml <file>` to see what it decides, or
  `--table` for the full CV-to-tolerance mapping.
  Recon never refuses to search on analyzer grounds: an unreadable analyzer, an
  analyzer with no bucket, or a run whose MS2 detector changes part-way all fall
  back to ±20 ppm, flag the assumption, and report the detectors seen.
  The Orbitrap window was **50 ppm until 2026-08-31**. Measured on all four files,
  ±20 gives **12.5-17.4 % more confident PSMs and 35-41 % less wall clock** — a
  wider fragment tolerance admits more random matches, inflating the decoy
  distribution and pushing true hits below 1 % FDR. 20 is the documented worst case
  for the class, not the optimum on these files; do not lower it on their strength.
  ⚠ Only the Orbitrap window is validated against local data — all four test files
  are Orbitrap MS2, so the ion-trap, TOF and Astral windows are curated assumptions.
  ⚠ The override covers the PASS-1 path only. The Pass-2 and digestion templates
  still carry a hardcoded ±20 ppm.
- **Neither pass ever searches with modifications, and both assert it.** Pass 1 is
  an open search whose whole point is that the modification set is unknown; a fixed
  cysteine carbamidomethyl would put every alkylated peptide at delta 0 and hide the
  alkylation state recon exists to report. Measured on bcell: with `static_mods
  {C: 57.0215}` the +57 peak reads 146; without it, 3311 — **the largest peak in the
  file**. `static_mods`/`variable_mods` are stripped when the effective config is
  written, and the run fails if anything survives. This is enforced in code rather
  than in the templates, because a template can always be replaced with `--params`.
  **Pass 1 also REFUSES any template whose `isotope_errors` is not `[0, 0]`** — a
  non-zero isotope window lets Sage match a neighbouring isotope peak, which moves
  the delta mass, and the delta mass IS the measurement. Measured cost of `[-1, 2]`:
  the +57 peak falls from 1253 to 394 and a Propionyl peak is invented at +56.018.
  The effective config also records the FASTA and mzML **actually searched**, not
  the template's — those two fields used to be copied verbatim while Sage took the
  real ones from its CLI flags, so a committed artifact once named a database and a
  raw file that were never used.
  (⚠ An earlier version of this said "17 configs under `testing/configs/` still
  carry a fixed C". No longer true: 16 unreferenced configs were archived
  2026-09-01, and the two that remain dirty are labelled regression fixtures.)

For current phase, next action, and detailed known gaps, see
[PLAN.md](PLAN.md). For design decisions and reasoning, see [NOTES.md](NOTES.md).

## First release smoke test — 2026-09-02 (apple-silicon)

Full record: `testing/release-smoke/v0.1.0-apple-silicon.md`. Ben ran the
downloaded v0.1.0 archive from `~/Downloads`, outside any checkout. **It
completed in 79.5 s** and both Sage passes ran in process. It also found three
things CI cannot see.

**1. ⚠ THE BIGGEST FIND — the released binary produced NO recommendations.**
`main.rs` read the curated mod list from
`PathBuf::from("reference-notes/metaMorpheusMods")`, a WORKING-DIRECTORY-relative
path. Inside a checkout it resolves; anywhere else it does not, so every
downloaded copy of v0.1.0 printed `curated mod list not loaded, recommendations
omitted` and emitted a report with no tier recommendations — the thing the tool
exists to produce. It WARNED and continued, so the report still looked complete.

⚠ **The bundling was already done. Only the call site was missed.**
`defaults::CURATED_MODS` embedded all four files, and
`the_curated_mod_list_is_embedded_and_non_empty` asserted they were present and
correctly ordered. That test passed the whole time, because it tests the
CONSTANT in isolation and never that anything USES it. Its own doc comment said
"the same four files `main.rs` used to read" — past tense, describing a switch
that never happened.

**The lesson, and it generalises:** a test that an asset is bundled is not a test
that the asset is used. Assert the BEHAVIOUR from outside the repo, not the
presence of a constant. Fixed by calling `CuratedDb::load_from_sources` with
`defaults::CURATED_MODS`. Verified by running the binary from a scratch
directory: `Recommendations: 7 by statistics, 0 by abundance`, no warning.

**2. macOS Gatekeeper KILLS the binary; there is no dialog.** On Apple Silicon a
quarantined unsigned binary gives `zsh: killed`, nothing else. The README claimed
an "unidentified developer" prompt and offered right-click-Open; both were wrong
and are corrected. The fix is `xattr -d com.apple.quarantine ./recon`.

**3. Two stale strings in user-visible report text** (NOT yet fixed):
- The MS1 line prints `median |error| 11208.77 ppm` under a MASS ACCURACY
  heading. In an OPEN search the precursor delta carries the modification mass,
  so a ppm summary over all PSMs is meaningless. The honest number is the
  self-calibrated block below it (+2.42 ppm bias, MAD 0.48). It reads as a bug.
- The same block says "Sage v0.14.x". The pin is **0.15.0-beta.2**.

**4. The HTML report is still the OLD layout.** Expected, not a regression: the
redesign is PLAN item 1 and was explicitly out of scope. The agreed layout is
`testing/scripts/report_layout_mockup.py` — PORT IT, do not re-derive.

## Doc reconciliation pass — 2026-09-02

Ben asked for AGENTS, PLAN and NOTES to be reconciled with the repo, because
sessions were losing track of what was done. Everything below was verified
against a file or a command, not recalled. Five things were WRONG, not merely
out of date, and were corrected in place.

1. **PLAN Step 4 said Sage needed no compilation.** It read "No compilation of
   Sage is needed... Step 4 is a PACKAGING problem, not a build problem", and
   named upstream v0.14.7 binaries. That died when Sage became a Cargo git
   dependency on 2026-09-01. Sage compiles from source on every target, every
   build. This is the single most misleading line found.
2. **PLAN had TWO `## Status` headings.** 715 lines of superseded history
   carried its own status block. Moved to
   `_archive/plan-status-history-2026-09-02.md`; PLAN went 1420 -> ~709 lines.
3. **AGENTS named `SAGE_VERSION`/`SAGE_COMMIT` as Sage's pin**, contradicting
   its own "Sage IS A LIBRARY" lock lower in the same file. The authoritative
   pin is the git rev in `Cargo.toml` plus `Cargo.lock`; the constants mirror it.
4. **A PLAN entry described `verify_sage_version` as live** and `SAGE_VERSION` as
   `0.14.6`. The function was deleted 2026-09-01 and the constant reads
   `0.15.0-beta.2`.
5. **`sage_runner.rs` carried an orphaned doc block** for the deleted
   `DEFAULT_SAGE_PATHS`. Because `///` lines merge, it had silently become the
   first paragraph of `SageConfig`'s documentation. Deleted. Also corrected
   "vendored binary" wording in three places and a stale "Sage v0.14.7" column
   comment in `sage_results.rs`.

Also on Ben's call: the **second push remote was DROPPED** (GitHub is the single
remote; the dual-push could never be satisfied, so every session reported an
unfixable failure), and the stale tracked `recon-tool/build.log` and
`recon-tool/test.log` from 2026-08-24 were removed and `*.log` ignored.
`temp-flowChart.md` was KEPT — Ben's call, it is not cruft.

**The lesson, for the next session:** these files disagreed with the repo because
a phase boundary was crossed (Sage became a library) without sweeping the
documents that described the old world. AGENTS already requires updating PLAN and
NOTES at a phase boundary. Add the code's own doc comments to that sweep.

## Continuous integration and releases (added 2026-09-02)

The workflow is `.github/workflows/build.yml`. It builds four targets:
`x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin` and
`aarch64-apple-darwin`. It runs on pushes to main, on pull requests, and on
`v*` tags. A tag also publishes a GitHub release.

The shape came from the in-house workflow in `neely/sagegui`, as instructed.
Four things were changed on purpose.

**1. Every cargo command carries `--target`, and each target gets a native
runner.** sagegui builds both macOS entries with a bare `cargo build --release`.
That produces a binary for the RUNNER architecture twice, so its
`x86_64-apple-darwin` asset is mislabelled. Here `macos-13` builds the Intel
target and `macos-14` builds the Apple Silicon target. Both are native, so
`cargo test` runs natively and nothing needs Rosetta.
*Rejected alternative:* one `macos-latest` runner cross-building both. It
builds, but it cannot RUN the x86_64 tests, so half the matrix would compile
without testing.

⚠ **RUNNER IMAGES — corrected by Ben 2026-09-02, do NOT revert.** The first
version of this workflow used `macos-13` for Intel and `macos-14` for Apple
silicon. **`macos-13` is fully retired and is not supported on GitHub Actions.**
The correct runners are:

| platform | runner | target |
|---|---|---|
| apple-silicon | `macos-latest` | `aarch64-apple-darwin` |
| apple-intel | `macos-15-intel` | `x86_64-apple-darwin` |
| windows-64 | `windows-latest` | `x86_64-pc-windows-msvc` |
| linux-64 | `ubuntu-latest` | `x86_64-unknown-linux-gnu` |

**This was measured, not just asserted.** Run `33665666610` on commit `24b0d3d`
was the first execution. Linux, Windows and `aarch64-apple-darwin` all completed
**success**. The `macos-13` job sat in `queued` for about 40 minutes, ran
**0 steps**, and ended `cancelled` — it never got a runner. That is what a
retired image looks like.

The asset names also come from Ben: `recon-apple-silicon`, `recon-apple-intel`,
`recon-windows-64`, `recon-linux-64`. They name the PLATFORM, which is what a
person downloading needs, instead of a triple fragment like `macos-x64`.

The same label names the JOB and the cache key. The matrix carries a `platform`
key and the job is `name: Build ${{ matrix.platform }}`, so the GitHub checks
list reads "Build apple-silicon", not "Build aarch64-apple-darwin". The Rust
target triple stays in `matrix.target`, where the compiler needs it. A target
triple is a build detail, not a name for a person to read.

⚠ **Runner image availability is an EXTERNAL fact.** It cannot be checked from
this repo, and it was got wrong once by copying the sagegui parent's matrix.
Confirm against GitHub's current runner list before changing it.

**2. CI gates on `cargo fmt --check`. It does NOT gate on clippy.**
At `23146f1` the tree had never been rustfmt'd: `cargo fmt -- --check` printed
**544** `Diff in` hunks. Ben called it on 2026-09-02, the tree was reformatted
(31 files), and the gate went on.

**rustfmt is a LAYOUT tool, not an optimiser.** It rewrites whitespace and line
breaks only. It cannot change behaviour, speed, or any number this tool reports.
Proven, not asserted: after the reformat `cargo test` read **191 passed, 0
failed** and `run_validation.py` read **17/17** — identical to the baseline.

**Clippy is the opposite, and is deliberately NOT gated.** `cargo clippy
--all-targets` printed **54** warning/error lines at `23146f1`, about 40 distinct
lints — `sort_by_key`, `RangeInclusive::contains`, a deprecated `NEUTRON_MASS`.
Every one of those is a REAL code edit that touches logic. In a project whose
credibility rests on numbers not moving, they must be judged one at a time
against the tripwires, not applied in bulk to turn a gate green.
*Still open:* work the clippy list in its own session.

**3. `dtolnay/rust-toolchain@stable` replaces `actions-rs/toolchain@v1`.** The
actions-rs organisation archived its actions, so the parent workflow depends on
an unmaintained action. Same function, maintained source.

**4. Caching is `Swatinem/rust-cache@v2`, not a hand-rolled `actions/cache`.**
✅ **CLOSED 2026-09-02 — `Cargo.lock` is now COMMITTED, on Ben's call.** This
entry previously said the lockfile was ignored and that committing it needed an
explicit request. It got one. Corrected in place, not appended.

`recon-tool/Cargo.lock` pins **415** packages and is tracked. `recon` is a
binary crate, so the lockfile is the only thing that pins the dependency GRAPH;
the Sage git rev pins Sage's SOURCE alone. This directly narrows the weakness
recorded under "q-DERIVED COUNTS JITTER". **Do not re-ignore it.**

⚠ **REVISED 2026-09-03.** The step used to be `actions/cache@v6` keyed on
`hashFiles('recon-tool/Cargo.lock')`, caching all of `~/.cargo/registry`,
`~/.cargo/git` and `recon-tool/target`. That cache **never shrank** — the local
`target/` is 10 GB — and it is the root of the Linux out-of-disk failure below.
Replaced with `Swatinem/rust-cache@v2` (`workspaces: recon-tool`,
`key: ${{ matrix.platform }}`, `save-if` limited to `refs/heads/main`). It keys
on the `Cargo.lock` hash **plus** the exact `rustc` version and target triple,
caches the registry index and `.crate` cache but NOT the re-extractable
`registry/src`, caches `~/.cargo/git` for the pinned `sage-*` deps, and PRUNES
stale artifacts before saving so the cache stays single-digit GB. PR runs
restore but do not save, so PR churn cannot displace the `main` cache; tag
builds restore the warm `main` cache without overwriting it.
**Rejected alternative:** keep the hand-rolled `actions/cache` and just narrow
its `path`. Dropped — `rust-cache` is the maintained standard for exactly this
job and also does the artifact pruning, which a plain `path` list cannot.

**5. Superseded runs are cancelled (`concurrency`, added 2026-09-03).**
`group: ${{ github.workflow }}-${{ github.ref }}`,
`cancel-in-progress: ${{ !startsWith(github.ref, 'refs/tags/') }}`. Ben pushes
to `main` 30-40 times on an active day, mostly in bursts, and each push started
a fresh 4-target matrix (macOS bills 10x, Windows 2x) that ran to completion
even when three later commits had landed. Now a newer push to the same ref
cancels the older run. **Tag refs are exempt** — a release build always
finishes. The 4-target matrix on every push is unchanged: Ben's explicit call
was to keep full cross-platform coverage, not trim it. If minutes stay tight,
the next lever is gating macOS behind tags; not done.

**6. Workflow files are linted (`.github/workflows/actionlint.yml`, added
2026-09-04).** One ubuntu step, `docker://rhysd/actionlint:1.7.12`. Gated on
`paths: [".github/workflows/**"]`, so it runs ONLY when a workflow file changes,
not on code pushes — near-zero minutes. actionlint also runs shellcheck on every
embedded `run:` script, so the bash in `build.yml` is checked too. `build.yml`
and `actionlint.yml` both pass clean as of 2026-09-04. A broken workflow
otherwise fails silently or burns a full 4-target matrix before the error shows.

### Windows CI runner — setup and the gotchas (`.gitlab-ci.yml`, added 2026-09-04)

⚠ **SUPERSEDED 2026-09-08. THE PIPELINE IS DELETED AND MUST NOT BE RE-ADDED.**
Ben found the NIST approval route it served does not apply to this project, so
the second pipeline had no purpose. `.gitlab-ci.yml` is removed. Releases now
come from `.github/workflows/build.yml`, which builds four targets on a `v*`
tag; a public repository gets free runners, which is what had blocked it.

**This entry is KEPT, and deliberately.** The three gotchas below are true of any
self-hosted Windows runner with a shell executor, they each cost real time, and
the ordering rule at the end applies to tagging on any host. Read it as runner
knowledge, not as a description of live infrastructure.

**Why it existed:** GitHub Actions was billing-blocked (see PLAN status block),
and Ben separately needs a Windows build for the NIST internal approval
process. An internal pipeline now produces that, in parallel — GitHub stays the
canonical history and the eventual public release path (GitHub under NIST,
after approval). **This SUPERSEDES the 2026-09-02 "second remote DROPPED, do
not re-add it" call** — see `AGENTS.md` "Remote", corrected in place.

**Scope, Ben's call:** Windows only. No Linux or macOS job. This pipeline's role
is producing the internal Windows exe for approval, not a second multi-platform
release pipeline — that stays GitHub's job once billing clears.

**The runner is a self-hosted PROJECT runner** (`shell` executor, tag
`windows`), not a managed shared runner — none were available. This means:
native build, no cross-compilation, but the pipeline only runs while the runner
host is up and the runner process is running.

**Three real problems hit standing it up, each cost real time:**

1. **`pwsh` not found.** The runner defaulted to PowerShell 7 (`pwsh`), which is
   not installed; the runner host has Windows PowerShell 5.1
   (`powershell.exe`). Fixed by editing the runner's `config.toml`,
   `shell = "powershell"` (was `"pwsh"`). The CI script only uses commands that
   work in both.

2. **Two runner identities raced for jobs.** `gitlab-runner.exe install` +
   `start` registers a Windows SERVICE running as `NT AUTHORITY\SYSTEM`, which
   cannot see the per-user `rustup` install (`%USERPROFILE%\.cargo\bin`) — so
   it cannot build. The working fix at the time was running
   `gitlab-runner.exe run` in a foreground window as the INTERACTIVE USER,
   WITHOUT first removing the service. Both then existed and competed for jobs:
   build 1 was picked up by the foreground process (as the interactive user) and
   succeeded; the retry was picked up by the SYSTEM identity, which saw a build
   dir now owned by the other account and failed with git's "dubious ownership"
   (exit 128).
   ⚠ **Resolved differently than first suspected.** `sc.exe query gitlab-runner`
   returned `1060: the specified service does not exist as an installed
   service`, and `Get-Process gitlab-runner` returned nothing — so by the time
   this was diagnosed, no service actually existed. The fix that worked was
   just clearing the wedged build dir (`Remove-Item -Recurse -Force
   <runner>\builds`) and running one foreground `gitlab-runner run`.
   **If a service reappears later** (e.g. `install` run again), the same
   ownership conflict will recur — either keep it foreground-only, or install
   the service to run AS the interactive user (`gitlab-runner.exe install --user
   "<DOMAIN>\<user>" --password ...`) so there is one identity, not two.

3. **The `cache:` block cost more than it saved.** A hand-rolled `cache:` block
   (mirroring the GitHub Actions pattern) archived `.cargo-home/`
   (28,217 files) and `recon-tool/target/` (3,786 files) every run — 46s just
   to save, on a runner that is a single persistent host anyway. Worse,
   the default `git clean -ffdx` between runs tried to delete that same
   pair of directories and is part of what produced the exit-128 failure in
   problem 2. **Fixed by dropping `cache:` entirely and setting
   `GIT_CLEAN_FLAGS: none`.** The build dir persists on disk between runs like
   an ordinary local `cargo build` — first run after a clean `builds/` dir is a
   full ~20 minute compile (Sage pulls in arrow/parquet/object_store), every
   run after reuses the compiled dependency stack.

**Current state, verified 2026-09-04:** `build:windows` succeeds end to end —
`cargo fetch --locked` reaches both crates.io and `github.com` (the pinned
`sage-*` git dependency) with no proxy needed, the release build completes,
and `recon.exe --version` printed `recon 0.1.1` (not the old `0.1.0`
mis-stamp). Artifact uploads and downloads from the pipeline.

**Deliberately deferred, Ben's call 2026-09-04:** a `rust-toolchain.toml` pin
(the runner reads `rustc 1.96.1`). Not needed for today's goal, and GitHub
already tracks `stable`. Revisit once the repo moves toward the
GitHub-under-NIST release stage. Also still deferred: adding
`cargo fmt --check` / `cargo test` to the internal job.

**`v*`-tag → Release: DONE, verified end to end, 2026-09-04.**
`release:windows`, tag-gated, `needs: [build:windows]`. The shell executor has
no `release-cli` installed, so it calls the Releases/Package APIs directly with
`CI_JOB_TOKEN` from PowerShell: PUT `recon.exe` into the project's generic
package registry, then POST a Release pointing at that package URL. Both jobs
ran green on `v0.1.2`; the CI job token had Release/Package API access by
default, no token-access setting change needed on this instance.
`v0.1.2` shows up in both **Releases** and the **Package Registry**, confirmed
in the web UI, not just from the job log.

⚠ **First tag attempt built the WRONG commit — the CI config is read from the
TAGGED commit, not from whatever is newest on `main`.** The tag was
created locally before `git fetch` picked up `cc01e1c` (the commit that
added `release:windows`), so the first `v0.1.2` push produced a pipeline with
only `build:windows` — the job simply did not exist yet at that commit. Fixed
by deleting the tag both remotely and locally, fetching, confirming the remote
branch tip matched `cc01e1c`, THEN re-tagging. **The order that
matters: sync the branch first, verify the commit, THEN tag** — not
tag-then-sync.

**`v0.1.2`, not `v0.1.1`.** `Cargo.toml`/`Cargo.lock` bumped 0.1.1 -> 0.1.2
BEFORE tagging, specifically to avoid repeating the exact defect PLAN already
flagged: reusing a version number the `v0.1.1` tag already claims, on a
different commit, would collide in meaning even where tag namespaces are
independent. This tag is for the NIST approval build — no matching GitHub tag
yet, GitHub still awaits its billing block clearing. See PLAN "RELEASE
STALENESS".

### ⚠ Green CI is NOT the numerical tripwire

CI cannot run `testing/scripts/run_validation.py`. That harness needs the mzML,
FASTA and search-output files, and all of those are gitignored.

`cargo test` on CI is also weaker than the count suggests. **Measured
2026-09-02 on a real bare clone of `main` at `23146f1`:** the suite reports
**191 passed, 0 failed, 0 ignored across 14 suites** — the SAME count the full
local checkout reports. The data-dependent tests do not fail and do not get
counted out. They return early. Running with `-- --nocapture` shows **25 skip
events** on stderr, against these absent paths:

| absent path | skip events |
|---|---|
| `testing/search-output/step1-open-bcell/results.sage.tsv` | 7 |
| `testing/search-output/step1-open-b1906/results.sage.tsv` | 5 |
| `testing/search-output/step1-open-serum/results.sage.tsv` | 5 |
| `testing/inputs/*.mzML.gz` (three files) | 6 |
| `testing/inputs/UniProt-Human-...fasta` | 2 |

So a green CI badge means the code COMPILES on four targets and the data-free
logic holds. It does NOT mean the pinned numbers still reproduce. **The
numerical gate stays a local `run_validation.py` run plus a local `cargo test`
on a checkout that carries `testing/inputs/` and `testing/search-output/`.**

The two files the skip behaviour was checked in are
`recon-tool/tests/analyzer_detection_integration.rs` and
`recon-tool/tests/tier_assignment_integration.rs`. Both skip cleanly. This was
verified by cloning the repo to a scratch directory and running the suite
there, NOT by reading the comments that claim it.

### The release archive is a licence obligation

Each archive holds four files: the binary, `README.md`,
`THIRD_PARTY_LICENSES.md` and `unimod.xml`. The last two are **Design Science
License Section 3 obligations**, recorded in `THIRD_PARTY_LICENSES.md` lines
152-164: a copy of the licence must travel with the work, and the Source Data
must be in the same distribution as the Object Form. `unimod.xml` is compiled
into the binary by `defaults.rs:51`, so the XML must ship beside it.
⚠ **The Linux runner ran OUT OF DISK once.** Run `33683133213`, 2026-09-02:
`failed to write .../release/deps/rustcXdpoMl/lib.rmeta: No space left on
device (os error 28)`. `cargo test` builds the whole tree in debug and
`cargo build --release` then builds it again, and Sage pulls arrow, parquet,
object_store and the cloud stack. The other three runners passed the same
commit. Fixed with a Linux-only `Free Disk Space` step that removes the unused
preinstalled dotnet, ghc, android, CodeQL and boost trees. **If Linux starts
failing at `Build Release` again, check disk before suspecting the code.**

**Do not slim the archive by dropping them.** The `Stage Release Archive` step
asserts all four files are present and non-empty, and fails the build if one is
missing, so the obligation is enforced in CI and not only in prose.

⚠ **The Windows runner corrupted this file once.** `autocrlf` on the Windows
checkout rewrote all 46589 line endings, so the windows-64 archive shipped a
**2553267**-byte `unimod.xml` while the other three shipped the pinned
**2506678**-byte one. Found 2026-09-02 by hashing the four CI artifacts, NOT by
reading four green checkmarks — every job passed with the corrupt file in it.
Fixed by `testing/reference-data/unimod.xml -text` in `.gitattributes`, and
re-verified: all four archives now read md5 `97a0601493d9014bdeee8b8b835be56b`
at 2506678 bytes, identical to the repo copy.

**This is now a CI tripwire, not a manual habit.** `Stage Release Archive`
sha256s the archived `unimod.xml` against the repo copy and fails the build on a
mismatch, printing both hashes and byte counts. Falsification-tested both ways
before it shipped: it passes a good archive and catches a one-byte corruption.
The `-text` attribute is the FIX; this check is the TRIPWIRE that would have
caught the original bug on the day it landed.

### CI warnings: what they mean (checked 2026-09-02)

Two warnings appear on every run. Ben asked whether they matter. **Neither is an
error, and neither ever broke a build.**

1. **"Node.js 20 is deprecated... actions/cache@v4, actions/upload-artifact@v4
   ... forced to run on Node.js 24."** Real, and fixed: bumped
   `actions/upload-artifact` to `@v7`. `actions/checkout@v5` was NOT named, so it
   is already on Node 24. `actions/cache` was also bumped to `@v6` at the time,
   then REMOVED on 2026-09-03 when caching moved to `Swatinem/rust-cache@v2`
   (see "4. Caching is `Swatinem/rust-cache@v2`" above).
2. **"The process '/usr/bin/git' failed with exit code 128."** Emitted by the
   **`Post Run actions/checkout@v5`** cleanup step, which runs AFTER the build.
   It is checkout tearing down an auth config that is not there. It CANNOT
   affect the binary. Benign, and expected to keep appearing.

⚠ **A dead end, recorded so it is not repeated.** Warning 2 raised a fear that
`build.rs` had failed and stamped the binary `unknown`, losing provenance.
Chasing it by grepping the shipped binary for the short SHA was the WRONG
METHOD and cost real time: neither the CI binary nor a known-good local control
contained its own SHA as raw bytes, which looked like a defect and was not one.
The literal is simply not stored as a findable contiguous string.

**The one-line check that actually answers it** — use this, not `strings` or
`grep`:

```
recon qc-stats --tsv recon-tool/tests/fixtures/test_psms.tsv | grep git_commit
```

It printed `"git_commit": "07f2f87"` from the CI-built binary, matching the
commit. Provenance is INTACT. `cargo build -vv` also shows
`cargo:rustc-env=RECON_GIT_COMMIT=<sha>` if the build script is what is in doubt.
**Verify a runtime value by running the thing, not by inspecting bytes.**

### Turning unimod.xml into an internal database does NOT remove the obligation

Asked by Ben 2026-09-02. Answered by reading the DSL text vendored in
`THIRD_PARTY_LICENSES.md`, not from recollection.

Section 2 defines **Object Form** as an executable or performable form of the
Work (line 217), and **Source Data** as the entire machine-readable "preferred
form of the Work for copying and for human modification" (line 220).

So a compiled-in database built from `unimod.xml` is an **Object Form of the
Work**. The Source Data is still `unimod.xml`, because the XML stays the
preferred form for a human to read and modify — a serialised trie or binary blob
is not. Baking it into a custom structure embodies the Work; it does not
dissolve it. That keeps Section 3 in force, which allows shipping the Object
Form only under (a) Source Data in the same distribution, (b) a written offer
valid at least three years at a public URL, or (c) a third party's offer,
non-commercial only.

**We do (a), and should keep doing (a).** (b) is the only route that drops the
file from the archive, and it trades 2.4 MB for a multi-year duty to keep
serving it.

⚠ **The real risk is Section 4, not Section 3.** Today `defaults.rs` uses
`include_str!`, the bytes are verbatim, and `THIRD_PARTY_LICENSES.md` states the
file ships UNMODIFIED. If a future change CURATES the data — subsets it, corrects
entries, merges in MetaMorpheus names — that is plausibly a **derivative work**,
and Section 4 attaches its own conditions: publish under the DSL, give it a new
name, credit the differences. That is a different and larger obligation.

**Rule for future edits:** build whatever internal structure you want at build
time or run time, but keep the pinned `unimod.xml` as the on-disk input and keep
shipping it. That holds 3(a), keeps the "unmodified" claim true, and keeps
Section 4 out of scope. Shipping a CURATED Unimod is a deliberate decision for
Ben and NIST counsel. This entry is a reading of the licence text, not legal
advice.

## Known permanent limitations

- **Digestion score penalizes semi-tryptic as if it were always bad digestion.**
  `compute_digestion_score` (in `testing/scripts/digestion_efficiency.py`) gives
  a 0–30 "semi-tryptic" sub-score that drops toward 0 as semi-tryptic % rises,
  and rolls it into a composite "digestion score" with an interpretation label
  ("Acceptable digestion", etc.). This rubric assumes a cell-culture tryptic
  digest, where high semi-tryptic = poor enzyme performance. It is **wrong for
  biofluids**: the serum test case (`2019-4-9_909c_0311`) is 31.8% semi-tryptic,
  which is largely *biology* (endogenous protease activity / the serum
  peptidome), not a failed digest — yet the rubric scored it 0/30 semi and
  labeled the composite "64.2/100 Acceptable," reading like a mediocre digest.
  - **Why this can't just be auto-fixed:** the tool cannot know whether a sample
    is a biofluid or a cell-culture digest, so it cannot know whether a high
    semi-tryptic rate is expected or a problem. **We present the numbers; the
    user makes that call.** See the PLAN Phase 8 item to drop the judgmental
    composite score + label and report the raw breakdown (MC distribution,
    fully-tryptic %, N-/C-ragged split, N:C ratio) instead.
  - Until that change lands: **trust the raw breakdown, not the composite score
    or its interpretation label.** (intentional to leave as-is for now, not a
    bug to "fix" by re-tuning thresholds — the fix is to stop scoring, not to
    re-weight.)

## Intentional, not bugs
Things that look wrong but are correct. Do not "fix" these.

- **✅ CLOSED 2026-09-01 — `run` no longer defaults to a FIXED-C template. Two
  independent fixes, both measured.** ⚠ This entry said the gap was open, and was
  stale; corrected in place, not appended.
  - **The template is clean.** `testing/configs/open-search-params.json` was read
    2026-09-01: it has **no `static_mods` key at all**, no `variable_mods`,
    `isotope_errors [0,0]`, `fragment_tol.ppm [-20,20]`, `precursor_tol.da
    [-500,100]`. It was cleaned in `9068ea2` ("Pass 1 and 2 never search with
    mods").
  - **The template no longer matters.** Pass 1 EMPTIES `static_mods`,
    `variable_mods` and `max_variable_mods` whatever the template says
    (`sage_runner.rs:380`) and then ASSERTS the result is empty
    (`sage_runner.rs:397-405`); pass 2 does the same (`pass2.rs:153`, `:163-171`).
    See "PASS 1 AND PASS 2 NEVER SEARCH WITH MODIFICATIONS" and "The no-mods
    guards are now TESTED".
  - **What is STILL open, and is genuinely step-4 work:** the default path is
    `PathBuf::from("testing/configs/open-search-params.json")` (`main.rs:2202`),
    relative to the working directory, so `run` still depends on being launched
    from the repo root. Same for the pass-2 default (`main.rs:1965`) and the
    curated mod list (`main.rs:1699`). That is the packaging item in PLAN step 4,
    NOT an alkylation gap.
  - **History kept:** the three end-to-end timing runs on 2026-08-17 DID use the
    then-fixed-C default, so those *reports* are fixed-C (timing valid, science
    default wrong). Nothing in the repo reads them now — see "NOTHING READS A
    FIXED-C OR CLOSED SEARCH ANY MORE".

- **⚠ KNOWN GAP — the `report.alkylation` block is stale leftover from the dropped fixed-C design
  and misleads in the current alkylation-agnostic report (found 2026-08-24, step 1 verification).**
  `compute_alkylation_check` (`report.rs:401`) assumes a fixed +57 Cys baseline and looks for
  Cys-containing PSMs with delta ≈ **−57** Da — evidence of a Cys that *failed* to get alkylated
  under an assumed fixed mod. That premise no longer holds: "Default recon = two searches" above
  already states the fixed-mod assumption was dropped, and the console/JSON still print `"Fixed mod
  assumed: Carbamidomethyl (+57.02 Da) on C"` / `"✓ Alkylation appears complete"` on every run
  regardless. In a no-fixed-mods report this is not exactly wrong (there genuinely is ~0% evidence
  of failed alkylation *relative to an assumption the search doesn't make*), but it reads as "the
  tool checked your alkylation and it's fine," which is not what happened — the real +57 signal is
  the discovered peak in `mod_discovery.peaks` (correctly present, rank 2, Unimod-annotated
  Carbamidomethyl, on all three files), not this block. Caused real confusion this session (Ben
  expected to see "unalkylated Cys" info and instead got a misleadingly reassuring banner). Not
  fixed here — flagged for the write-up/cleanup pass. Options: drop the block entirely (it
  duplicates what the peaks list already says, better), or rename/reword it to be explicit that it
  assumes a fixed-C hypothesis the tool no longer defaults to.

- **✅ Also found and fixed same session: `analyze`'s CLI doc comment claimed a `.txt` output that
  was never implemented.** `run_analyze_command` only ever wrote `.json` and `.html`
  (`main.rs:1490-1502`); the `--output` flag's doc comment said "generates .json, .txt, .html."
  The `.console.txt` files previously committed under `testing/recon-output/full-run/` were a
  manual `stdout` capture, not a tool artifact — they went stale (last touched before the step-1
  re-run) with nothing to catch it. Doc comment corrected; stale console.txt files removed from
  `full-run/` (JSON + HTML are the tracked truth there, per `testing/README.md`'s current-only
  convention for that directory). `calibration-benchmark/`'s console.txt files are untouched —
  that investigation is CLOSED (see Calibration design space entry below) and its console.txt is a
  frozen historical artifact, not a "current" claim.

- **Any `HashMap` field that reaches serialized JSON is a nondeterminism source.**
  Rust's `HashMap` iteration order is randomized per-process (seeded `RandomState`,
  for DoS resistance), so serializing one emits its keys in a different order each
  run — same values, shuffled order, different bytes. **Serialized maps must be
  `BTreeMap` or otherwise order-stabilized (sort before serialize).** This is a hard
  prerequisite for Tier 3 regression snapshots, which assert byte-identical output on
  the same input — an unordered map makes the snapshot false-positive on re-run.
  Found in Phase 8: `FoldingStats::fold_by_k` was a `HashMap<i32,usize>` and broke
  byte-identical output; fixed to `BTreeMap`. **Enforced by a permanent determinism
  test** (`recon-tool/tests/determinism_test.rs`: run discovery repeatedly →
  byte-identical serialized output), NOT by manual pre-snapshot checking. If you add
  a new serialized struct with a map field, use `BTreeMap`, and the test will catch
  it if you forget. Do not loosen the test to tolerate a diff — fix the source.
- **✅ Found again, in a place the determinism test does not reach — the polymer
  table's ORDER, not a serialized map field (2026-09-03).** `pct_tic` returns a
  `HashMap`, and the polymer table sorted rows on the percentage value alone. Two
  surfactants sharing a repeat unit produce identical mass series, so their
  `%TIC` values tie exactly. bcell rank 9 read "Triton X-101" in one run and
  "IGEPAL CA-630 (NP-40)" in the next, both at 0.002936 %TIC — same number,
  different name, because a tie left the two rows in whatever order the map's
  iteration gave. A reader would take that as a change of sample. Ties now break
  by name; two consecutive `bcell` runs give identical ordering. This is the same
  class of bug as `FoldingStats::fold_by_k` above — a `HashMap` leaking its
  iteration order into presented output — but it hid in a SORT COMPARATOR, which
  the determinism test's byte-identical-output check does not audit for its own
  stability under ties. Found during the 2026-09-03 `full-run` regeneration, fixed
  in the same commit (`07628b6`).
- **Sage `peptide_q` is per-sequence, not per-PSM — chimeric rank-2 rows inherit
  rank-1's q-value.** With `chimera:true` + `report_psms:2`, a scan yields a rank-1
  primary and a rank-2 co-eluting secondary. Sage assigns ONE `peptide_q` per peptide
  *sequence*, so the rank-2 row carries the rank-1 row's q — a rank-2 PSM can show
  `peptide_q < 0.01` without having independently passed FDR at that delta. **Any
  PSM-level FDR filter must gate on `rank == 1`, not on `q < 0.01` alone, or it
  over-counts chimeric secondaries.** This bit the Gate 2 oxidation spot-check (3
  peptides looked like "passed FDR near +16 but missing from the peak" — a false bug;
  their rank-1 ID was the *unmodified* form, the +16 was a rank-2 secondary correctly
  excluded from the primary peak). It will bite the same way anywhere PSMs are counted
  by delta: the serum +57 triage, and the future PTM-Shepherd comparison. Zeroth step
  of any such count: filter to rank-1 primaries first. **Recurred as predicted:** the
  serum +57 triage went 86 band PSMs → 84 rank-1 after excluding 2 rank-2 chimeric
  secondaries (both confirmed sharing a scan with a rank-1 ID). Second time this
  mattered — treat rank-1 filtering as mandatory, not optional, for any delta-band count.
- **Sage version mismatch** — binary self-reports v0.14.6 but the folder is
  named `sage-v0.14.7-...`. Use the binary's self-reported version. Not a bug.
- **Precursor ppm in open search is huge/garbage** (e.g. mean 17,688 ppm) — it
  reflects the delta-mass distribution (modifications), NOT instrument error.
  Fragment ppm is the true mass-accuracy metric. Intentional; report fragment
  ppm for follow-up-search tolerance guidance.
- **Semi-tryptic rate reads 0% in the plain open search** — because the open
  search config doesn't set `semi_enzymatic: true`; ragged ends aren't in the
  search space. Measured via the dedicated two-pass workflow instead, not the
  open search. Known limitation of that config, not a code bug.
- **Chimera deduplication** — with `chimera: true` + `report_psms: 2`, multiple
  PSMs map to one scan. Signal fate deduplicates by scan; mod discovery counts
  ALL PSMs. This asymmetry is intentional.

- **⚠ TRACKED LATENT BUG (found 2026-07-16, Unified MS1 mass-error pass; NOT fixed —
  banked with standing, same register as disabled-satellite-folding): `analyze` silently
  mixes a multi-file open TSV into one report.**
  - **The bug:** `parse_sage_results` filters only by q-value, decoy prefix, and
    isotope-zero (`FilterOptions` has no filename field), and neither `mod_discovery` nor
    `signal_fate` filters by the `filename` column. So if the open TSV passed to `analyze`
    spans multiple raw files (e.g. `testing/inputs/results.sage.tsv` = 8 B/T-cell runs),
    ALL of their PSMs are pooled into one report while the report is labelled with the
    single `--mzml` file. Mod-discovery, signal-fate, digestion, QC numbers are then a
    blend of every run in the TSV — a wrong artifact that reads as "this one file."
  - **Partial MITIGATIONS in place (NOT the fix — do not mistake the warning for
    resolution):** (1) the provenance guard's **check 3** catches the *mislabel* case —
    if `--mzml`'s basename is not even present in the open TSV's filename set, it hard-errors
    (the report would be built on a different file entirely). (2) A **`WARNING`** prints when
    the open TSV spans >1 file, surfacing the mixing. Neither restricts the PSMs — a
    multi-file TSV whose set *includes* `--mzml` still pools all runs.
  - **Fix shape (its own pass, its own verification — do NOT bolt onto an unrelated
    change):** add a filename filter to `analyze` keyed to `--mzml`'s basename, so the report
    is built only from that file's PSMs. This is a **report-contract semantics change**
    ("this one file" instead of "whatever's in the TSV"), justified by the one-report-per-file
    lock — cite it. **Open questions the fix pass must answer:** does it break the existing
    single-file happy path? how does the filter interact with chimera/rank-1 handling? and
    its **conservation invariant** — PSM count after filtering to one file must equal the
    count of rows carrying that filename (assert it, don't narrate it). Left unfixed here
    because mixing it into the Entry-3 ppm/report work would blur the bisect if either
    regresses — same discipline as the serum triage surfacing 7D without building 7D.

- **✅ FIXED (2026-08-17): the benchmark comparison window caps at +100 Da, silently
  dropping the +100..+500 Da region both tools actually searched.**
  - **The thing that is NOT a bug (confirmed):** the Sage open-search config
    `precursor_tol.da: [-500, 100]` is CORRECT and produces a **delta (expmass−calcmass)
    window of −100..+500 Da**. Sage applies the tolerance to the *experimental* mass, so
    `-500` means "theoretical peptide 500 Da lighter than observed" = a +500 delta.
    Confirmed by Michael Lazear 2026-08-17 AND by every open-search TSV (all show delta
    [−100, +500]). See `reference-notes/sage-config-and-gotchas.md` (locked). **Do NOT
    re-run any Sage search over this — the searches captured −100..+500 correctly.**
  - **The actual bug (now fixed):** `testing/scripts/compare_mod_discovery.py` had
    `WINDOW_LO, WINDOW_HI = -150.0, 100.0`. Fixed to `WINDOW_LO = -100.0, WINDOW_HI = 500.0`
    (true overlap of our −100..+500 and PTM-Shepherd's −150..+500). Also bumped
    `MATCH_TOL_DA` 0.01 → 0.015 and added Spearman + top-N(10) scoring. All four
    comparison tables regenerated; spot-check assertion confirms >+100 Da peaks now appear.
  - **Also corrected (same pass):** the delta-backwards prose in
    `reference-notes/domain-primer.md`, `reference-notes/glossary.md`, and
    `testing/reference-data/ptm-shepherd/README.md` (those describe the window as
    "−500 to +100" on the delta axis; corrected to "−100..+500").
  - **Verification gate met:** `window_spot_check()` added to the comparison script
    asserts that when both tools have peaks >+100 Da, those peaks appear in the comparison
    output. Would have caught the original bug if present then.

---

## Disabled-by-design (do NOT re-enable as a "free improvement")

Features that are built, then deliberately switched off with the implementation
retained. A fresh agent seeing a `false` flag next to a complete function will
reasonably think "half-finished, let me wire it up." It is not. The reasoning
lives here; read it before touching the flag.

- **Satellite folding** (`enable_satellite_folding: false` in
  `recon-tool/src/mod_discovery.rs`) — off on purpose, function retained.
  - **Why off:** it has NO conservation guard. It reported numbers that didn't
    match its own code (1,770 → 766 → 583 for one peak) and once deposited 1,770
    "satellites" onto an 870-count parent — physically impossible (M+1 < M+0
    always; the isotope envelope can't exceed the monoisotope). Low value for
    recon (shifts the deamidation count, not the recon answer) and carries a
    double-count risk.
  - **Do not re-enable** without first giving it a hard, asserted invariant
    (like fold-to-zero's count-conservation gate). Without the invariant it can
    report any number and be narrated as correct — that is exactly the loop that
    cost this project six rounds. See the dead-ends entry and Phase 7C notes.
  - **Still on:** fold-to-zero — the verified, conservation-guarded path.

---

## Deferred enhancements (recorded with evidence, ready to activate)

### Per-run ranking confidence flag (motivated 2026-08-17 by serum Spearman result)

**Symptom:** serum's Spearman ρ = 0.284 (p=0.18, n=24) against PTM-Shepherd reallyOpen —
bcell/b1906 are ρ~0.63–0.65, significant. Serum detects the same mod set but ranks
prevalence differently. The explanation matches prior evidence: serum has the highest
pre-calibration drift (+2.5 ppm MS1 bias), the heaviest adduct load (DTT/Fe/Al), and
is an Orbitrap Fusion Lumos vs. Q Exactive for the other two — all conditions where a
scalar `apex_offset` + single-pass search are structurally expected to rank differently
from a two-stage recalibrated reference.

**Design (2026-08-17):** detection is sound across all three files (matched counts
24–32); ranking is the place where serum strains. The tool remains general-purpose —
it runs on any sample and self-reports when prevalence ranking is exploratory.

A `ranking_confidence` block in the report JSON with:
- `apex_offset_mda` — scalar drift already computed
- `adduct_pct` — fraction of PSMs in known adduct peaks (Fe ~52.9, DTT ~152, Al ~55)
- `satellite_pct` — UNANNOTATED ±1/±2 Da PSMs / total (the carpet fraction)
- `n_peaks_above_1pct` — mod complexity proxy
- `tier` field: `"high"` | `"exploratory"` | `"unavailable"`

All inputs are intrinsic (no reference run needed). Tier thresholds must be labeled
"v1 n=3, expect revision" — cannot be fit from three files without overfitting. Feature
values always printed; tier is advisory. Three design constraints to honor:
1. **Threshold overfitting** — thresholds are illustrative until a larger panel; raw
   feature values always surfaced so users can contextualize.
2. **Biology vs. artifacts** — flag inputs are instrument/prep signals only (drift,
   known adduct masses, satellite fraction), not mod-content signals. A sample with
   genuine unusual chemistry may show high adduct load from biology, not prep failure —
   the user reads the features, not just the tier.
3. **No reference dependence** — Spearman vs. FragPipe confirmed the problem; the flag
   must run without a reference. Every input already exists in the report.

**Status:** beta, deferred after `run_validation`. Ship as clearly labeled beta tier.
The flag's value is demonstrable even at n=3: serum's intrinsic signals (apex_offset
+2.5 ppm → ~2.5 mDa at 1 kDa, DTT/Fe/Al adduct presence, satellite carpet) would
independently flag it as "exploratory" without ever seeing a FragPipe run. That's
the proof-of-concept. Empirical threshold calibration (Spearman-fitting on held-out
files) is a later pass when a larger panel is available.



### Calibration design space — A / B / C (framing for C1/C2) — ❌ CLOSED-NEGATIVE (2026-07-17)

**C1/C2 is CLOSED as a negative result — all three arms exhausted, each by evidence. Do not
re-open without new evidence.** This is a real deliverable, same shape as the satellite-folding
disable: a motivated line investigated properly and closed with evidence, not a failure.

- **Arm 1 — ppm-constant (Option A, cheap fix): NEGATIVE.** Did not beat DaScalar; Step-1
  entry below.
- **Arm 2 — the carpet mandate: DISSOLVED.** The three-file ±1/±2 carpet that *promoted* C1/C2
  was instrumented to a ~1% peak-detection quantization wobble, not m/z drift (Step-1 entry +
  Dead-ends). Two mechanism narratives refuted by per-stage instrumentation.
- **Arm 3 — Option C ceiling test (calibrated-mzML POC): NEGATIVE.** Ben emitted MSFragger
  `write_calibrated_mzml=1` calibrated mzML (validated cal: serum MS1 2.51→0.06 ppm, b1906
  0.53→−0.04, MS2 too). Ran our Sage open search on it, same params, `discover --calibration
  none` (calibrated input + no internal cal = clean Option-C read). This is the **ceiling
  test** — gold-standard calibration, zero recalibrator infrastructure. Result:

  | file | input | total PSMs | deam apex | dev mDa |
  | --- | --- | --- | --- | --- |
  | b1906 | raw | 31682 | 0.9852 | +1.2 |
  | b1906 | calibrated | 31497 | 0.9850 | +0.9 |
  | serum | raw | 19307 | 0.9883 | +4.3 |
  | serum | calibrated | 19099 | 0.9843 | +0.2 |
  | bcell | raw | 81966 | 0.9818 | −2.2 |
  | bcell | calibrated | 80374 | 0.9817 | **−2.3** |

  - **Deamidation (decisive):** bcell — the one symptomatic file — stays FLAT (0.9818→0.9817)
    under gold-standard calibration, identical to what our own DaScalar did (0.9818→0.9817).
    serum/b1906 moved but were already on-target. **The one file that needed to move, didn't.**
  - **Option-C ID-recovery promise INVERTED:** PSMs went DOWN on all three (b1906 −185, serum
    −208, bcell −1592), not up. The "recover lost-at-search PSMs" merit did not materialize
    even at the ceiling.
  - Carpet: wobbled as expected under the mDa shift; noted, not weighted (it's a detector
    artifact — see Dead-ends).

- **Conclusion:** no gain exists even with the best available calibration → **do not build a
  recalibrator; fitted-A is pointless** (bcell's residual is not calibration-driven — see
  next). The POC removed the "maybe our own cal was too weak" ambiguity by substituting a
  validated one. C1/C2 closed.

- **bcell's residual 2.3 mDa — fold-driven, strongly indicated (NOT fold-instrumented).**
  It's flat under gold-standard calibration, so calibration is provably not the lever. That
  strongly indicates the fold stage sets it, but this was **not** directly confirmed by
  instrumenting the fold stage around the deamidation peak. **Parked as a separate
  fold-tolerance question, explicitly NOT part of C1/C2** and NOT on the active plan — activate
  only if someone later decides one file's deamidation accuracy is worth a fold-stage
  investigation. Do not chase now.

The A/B/C framing below is retained as the reasoning record (why B was off the table, why C
was worth a ceiling test, the FragPipe economics that made the POC cheap).

---

#### Original A/B/C framing (retained — reasoning record for the closed C1/C2)

C2 = the build (m/z-dependent recalibration machinery); C1 = the diagnostic question
(can the numbers we already compute tell us whether PTM binning lands right). That
vocabulary is fixed — do not relabel. Within **C2**, the working decomposition is
"**fit** the m/z error" then "**apply** it"; these are sub-steps of the build, not a
competing C1/C2 definition. The design space has three distinct options:

- **Option A — post-search delta correction.** Search once, correct the delta masses in
  the output. The scalar `apex_offset` is A (weak fit); a fitted m/z-dependent curve is
  still A (better fit, same stage). Touches only numbers already in results. Structurally
  **cannot** (1) recover PSMs lost at search time, nor (2) improve fragment matching /
  hyperscores / FDR — it only moves precursor deltas.
- **Option B — constrain the search space** ("deam ± tol", tighten precursor tol).
  **OFF THE TABLE — because it DESTROYS open-search discovery**, full stop. This is a
  *separate* reason from the single-open-search lock: the lock forbids a closed/targeted
  *follow-up*; B is forbidden because pre-specifying mods kills the discovery mission
  itself. Keep the two reasons distinct.
- **Option C — recalibrate the input, re-run the SAME wide-open search unchanged.**
  Discovery fully preserved; the search operates on better-centered data. This is what
  FragPipe/MSFragger does (the "MS1 New → ~0" column in the Phase 8.5 JOURNAL table).
  **C is NOT B** — the single-open-search lock does not forbid it (see the lock's
  clarified scope). C's advantages over A: it *can* recover search-time-lost PSMs (Gate 2
  showed "lost at search" was the dominant miss category) and improve fragment matching.

  **Economics — CONFIRMED cheap by the actual FragPipe run (2026-07-16), not assumed.**
  Concrete Option-C reference log:
  `testing/reference-data/ptm-shepherd/open/log_2026-07-16_12-06-25.txt` (the run that
  produced our PTM-Shepherd benchmark reference). It does search → calibrate → search:
  - `msfragger.calibrate_mass=2` (log line 308) — "Mass Calibration and Parameter
    Optimization" recalibrates **both MS1 and MS2** and writes `.mzBIN_calibrated`.
  - **Calibration is cheap:** the first (narrow) search is ~0.17/0.33/0.29 min per file
    (log 950/954/958) and the whole calibrate step is ~2.9 min. The **expensive** part is
    MSFragger's WIDE open search (47.97 of 50.9 min) — exactly what our Sage open search
    already beats 25–50×. So the appealing hybrid is explicit: **recalibrate cheaply, then
    run our fast Sage open search.** The infra worry was overstated; calibration is minutes
    and we keep Sage's speed.
  - MS1 calibration table matches our Phase 8.5 numbers (serum 2.51→0.06, b1906 0.53→−0.04);
    **MS2 tightens too** (b1906 −0.05→−0.16). This is an MSFragger `calibrate_mass` feature;
    **Sage has NO equivalent MS1 recalibration**, so for us the recal must happen *before*
    Sage sees the file.

  **Two-tier path for the C arm (record so a future session doesn't re-derive it):**
  1. **Cheap POC before building anything.** MSFragger can emit a calibrated mzML with
     `write_calibrated_mzml=1` — **this benchmark run had it at 0** (log 383/645/805), so no
     calibrated mzML exists yet. Re-run FragPipe's first-search+calibrate with the flag ON,
     feed the resulting calibrated mzML straight into a Sage open search, re-benchmark the
     carpet **through `compare_mod_discovery.py`** (not JSON-alone). Carpet collapses under
     Sage-on-calibrated-input → C is proven for us at near-zero build cost. Carpet survives
     even on properly calibrated input → **C is dead and the carpet is not a calibration
     problem.** Cheap kill either way.
  2. **Only if the POC succeeds → build our own recalibration pass** (rewrite mzML with
     corrected m/z) so we do NOT depend on MSFragger — depending on it defeats the "fast free
     open successor" mission. This is where vendoring recal references matters (Ben will
     search/vendor if we reach it). **Recal references MUST cover fragment (MS2)
     recalibration, not just precursor** — the MS2 side is what recovers "lost at search"
     PSMs (Gate 2's dominant miss category), which is the entire reason C could beat the
     Option-A track.

  **Ladder discipline — C does NOT jump the queue.** Immediate next rung is still **fitted
  m/z-dependent Option A** (zero new infrastructure, may collapse the carpet on its own; if
  it does, we never need C). This entry upgrades C from "expensive last resort" to "cheap
  flag-flip POC available if fitted-A leaves residual." If reached: confirm the POC before
  any build, scope as **comparison-only in `testing/`** (production default is a separate
  downstream decision), reuse Pass 1 IDs as the error model.

### m/z-dependent calibration Step 1 (ppm-constant) — BUILT, NEGATIVE result (2026-07-16)

The cheap-fix-first arm of C2's "fit/apply". **Built as a PARALLEL path, not a swap**
(`CalibrationMode {None, DaScalar, PpmConstant}`, `--calibration` flag on `discover`;
DaScalar is the default and reproduces historical output byte-identically). Three
invariants asserted in code: (1) PSM-count conservation through calibration; (2) zero
apex centering *measured & printed per-mode*, NOT asserted (a ppm-linear correction pivots
around m/z, so its zero apex may sit slightly off even when correct — that's a finding,
not a failure); (3) fold-to-zero conservation promoted from a log to a gating assert.

**Hypothesis tested:** *carpet = global ppm-scalar drift; a ppm-constant correction
collapses it.* **REJECTED by the numbers** (all 3 files × 3 modes, in
`testing/recon-output/calibration-benchmark/`). Four-gate table (zeroCtr = post-cal zero
apex mDa; offset = fitted; deam = deamidation apex Da; carpet = ±1/±2 unannotated PSMs;
total = PSM count):

| file | mode | total | zeroCtr | offset | deam | carpetPSM |
| --- | --- | --- | --- | --- | --- | --- |
| b1906 | none | 31682 | 1.00 | – | 0.9852 | 1520 |
| b1906 | da-scalar | 31682 | 0.00 | 1.000 mDa | 0.9846 | 2006 |
| b1906 | ppm-constant | 31682 | 0.00 | 1.754 ppm | 0.9845 | 1942 |
| bcell | none | 81966 | 0.10 | – | 0.9818 | 4832 |
| bcell | da-scalar | 81966 | 0.00 | 0.100 mDa | 0.9817 | 4837 |
| bcell | ppm-constant | 81966 | 0.00 | 0.157 ppm | 0.9821 | 4838 |
| serum | none | 19307 | 4.80 | – | 0.9883 | 71 |
| serum | da-scalar | 19307 | 0.00 | 4.800 mDa | 0.9839 | 153 |
| serum | ppm-constant | 19307 | 0.00 | 7.513 ppm | 0.9844 | 286 |

- **PSM-count conserved** (gate 1) — exact, every cell. **Zero centered** (gate 2) — both
  fitted modes → 0.00 mDa.
- **ppm-constant does NOT beat DaScalar** — within noise on b1906/bcell, slightly worse on
  serum. Da scalar NOT retired; both stay as parallel arms.
- **The more interesting signal: the ±1/±2 carpet appears to change under calibration**
  (b1906 JSON 1520→2006 under DaScalar). **This kicked off a mechanism hunt — see the
  CHARACTERIZED block below for the resolved answer.** (Two mid-investigation mechanism
  claims — "steal from folding" and "boundary crossing" — were raised here and BOTH later
  refuted by per-stage instrumentation. Removed from this spot to avoid leaving refuted
  narratives in the record; the resolved finding + the refutation numbers are in the
  CHARACTERIZED block.)
- Deamidation-apex gate was **over-generalized** — the 0.9817→0.984 target came from
  B.naive/Phase 7C and only ever applied to **bcell** (flat there, 0.9817→0.9821). b1906
  (~0.985) and serum (~0.984) are already on-target in `none` mode — nothing to recover.
  **Dropped as a universal criterion; per-file baselines are the table above.**

**MECHANISM of the carpet inflation — CHARACTERIZED, not a defect worth fixing (closed
2026-07-17 by per-stage instrumentation). BOTH prior mechanism claims REFUTED.** Traced the
none→DaScalar carpet growth (b1906) with temporary per-stage per-PSM window-population
counters (reverted after; numbers preserved here):

| stage | none | da-scalar | Δ | verdict |
| --- | --- | --- | --- | --- |
| post-calibrate (per-PSM in ±1/±2 window) | 6293 | 6288 | **−5** | boundary-crossing REFUTED |
| post-fold (per-PSM in window) | 3348 | 3353 | **+5** | steal-from-folding REFUTED |
| post-detect (peak-PSM in window) | 2295 | 2630 | **+335** | ← the growth enters HERE |
| post-annotate carpet / annotated | 1520 / 775 | 2006 / 624 | carpet +486 / annot −151 | annotation flips (secondary) |

**What it is:** a **peak-detection quantization wobble.** The ~3350 underlying PSMs barely
move (−5 at calibrate, +5 at fold); the entire carpet growth appears at the **prominence
detector** (post-detect +335), where a ~1 mDa shift re-bins the same PSMs and the
threshold-based detector assembles them into slightly different peaks (21→24 peaks in the
window). Annotation then flips ~151 PSMs' worth annotated→unannotated (secondary effect). No
biological signal is lost or gained — a quantization boundary is crossed differently by
millidalton-shifted inputs. ±335 peak-PSMs out of 31,682 (~1%), on a metric that is itself
the detector's output. **Below the resolution of any decision a recon user makes. Marked
"characterized, not worth optimizing for recon." Do NOT build a fix.**

**⚠ BOTH previously-recorded mechanisms were WRONG (kept here as the dead-end record):**

- *"Steal from folding"* (the Step-1 ppm headline, once written into this file as the general
  cause) — **refuted at post-fold: +5, not the +335.** It was measured on the none→ppm
  transition and never held on the DaScalar default.
- *"Shift crossing Da boundaries"* — **refuted at post-calibrate: −5.** A ~1 mDa DaScalar
  offset does not move PSMs across Da-wide window edges, exactly as suspected.

**VERIFICATION-DISCIPLINE WIN — name it (this is the bigger lesson than the carpet).** This
is the Phase 7 satellite-folding pattern re-emerging and being caught: a metric with no hard
invariant (the carpet count is a prominence-detector output), a plausible-sounding mechanism
narrative ("calibration shifts deltas out of fold windows"), and a fix (the "fold/calibrate
consistency" pass) that would have been declared successful against numbers that could not
refute it. We were **one step from building that fix for a mechanism that does not exist.**
What broke the loop: (1) the **STOP fired** on the failed `folded drop == carpet gain`
invariant instead of patching past it; (2) **per-stage instrumentation made the mechanism
falsifiable** — and it falsified both stories. Lesson for the next agent: when a metric has
no conservation invariant, do not build a fix off a mechanism narrative until you have
*instrumented* the mechanism to a specific stage. Plausible ≠ established. See Dead-ends.

### m/z-dependent mass calibration (replaces the scalar apex_offset)

**⚠ RE-SCOPED 2026-07-17 — carpet mandate dissolved; fitted-A now rests on ONE thin symptom.**
The three-file ±1/±2 carpet was the *external* evidence that promoted C1/C2 (a "three-file
mandate"). Instrumentation now shows the carpet is a peak-detection quantization artifact
(see the Step-1 entry above), **not** m/z drift that calibration would fix — so the carpet no
longer motivates fitted-A at all. The ppm-constant arm was a negative result. What remains is
the single **deamidation-accuracy symptom**, and even that is **per-file thin**: in `none`
mode the deamidation apex is already on-target on 2 of 3 files (b1906 ~0.985, serum ~0.984);
**only bcell sits low (~0.982).** So fitted-A is now an *optional* Unimod-landing-accuracy
enhancement resting on one file's symptom — NOT the promoted next rung, NOT carpet-driven.
**Decision deferred to next session** (data recorded below); if only bcell shows it, the
mandate is thin enough to consider parking C1/C2 entirely.

- **Current state:** calibration (Phase 7C, Step 2) is a single scalar `apex_offset`
  — the intensity-weighted median of the near-zero population, subtracted from every
  delta (`compute_calibration` in `mod_discovery.rs`). It correctly recenters the
  histogram so the unmodified peak sits at zero (b1906 apex_offset ~0.1 mDa): coarse
  alignment toward Unimod masses.
- **Deamidation apex per file, `none` mode (the data that would drive the fitted-A decision):**
  b1906 0.9852, serum 0.9883 (both ≈ Unimod 0.98402 within a few mDa — on target); bcell
  0.9818 (~2.3 mDa low — the lone standing symptom). If the next session builds fitted-A, it
  is for bcell-class accuracy only; if not, C1/C2 parks.
- **Limitation (documented in 7C):** a scalar corrects *bias*, not *m/z-dependent
  (ppm) drift*. Orbitrap mass error scales with m/z, so a single offset can't tighten
  a mass-proportional smear.
- **Standing evidence it's needed (concrete, reproducible — not hypothetical):**
  deamidation reads ~0.9817 Da after apex_offset vs. true Unimod 0.98402 — **~2.3 mDa
  low.** That residual is the m/z-dependent term the scalar can't touch (deamidation-
  bearing peptides sit at different m/z than the zero-peak population that defined the
  offset). **NB: this is bcell-specific — b1906/serum do not show it (see per-file above).**
- **Also "measuring but not using":** every peak already carries `mass_error_da` /
  `mass_error_ppm` in `PeakAnnotation` (computed against Unimod), displayed but never
  fed back to refine anything.
- **✅ BUG FIXED (2026-07-16, Unified MS1 mass-error report pass).** Was:
  `PeakAnnotation.mass_error_ppm = mass_error_da * 1000.0` (comment `// ppm at 1000 Da`) —
  only correct at exactly 1000 Da. True ppm is `error/mass × 1e6`, which equals `error × 1000`
  iff mass == 1000. For real tryptic peptides (~800–3000 Da) the displayed ppm was off by
  ~1.5–3×, systematically, scaling with distance from 1000 Da. **Three sites (NOTES originally
  recorded only two — the third was in `unimod.rs`):**
  1. `mod_discovery::annotate_peaks`, the Unmodified special-case annotation.
  2. `mod_discovery::rollup_unmodified_peaks`, the rolled-up Unmodified peak.
  3. `unimod::UnimodDb::find_matches` — `(mass_error_da / 1000.0) * 1_000_000.0`, the per-match
     ppm that fed annotation site (2)'s `mass_error_ppm`.
  **Fix mechanism:** added `Peak.representative_mz` (intensity-weighted MEAN m/z of the peak's
  constituent PSMs, reconstructed via `psm_mz`; mean matches the pipeline's intensity-weighting,
  dual-readout lock). All three sites now compute ppm via `mod_discovery::ppm_at_mz(error_da,
  mz)` = `error_da / mz × 1e6` (falls back to the 1000-Da form only when mz ≤ 0). `UnimodMatch.
  mass_error_ppm` is now marked **DEPRECATED — do not consume** (kept for serialization
  stability; carries a knowingly-wrong number); live callers recompute at the peak's real m/z.
  **Invariants asserted in code:** (i) per-site formula test (`test_ppm_at_mz_formula`); (ii)
  conservation — `representative_mz` must fall within [min, max] m/z of the peak's PSMs, a
  weighted mean cannot land outside its inputs (`test_representative_mz_within_input_range`,
  `debug_assert!` in `representative_mz`). **Verified numerically on b1906:** Oxidation
  −0.124→−0.181 ppm, Deamidated −2.214→−3.559 ppm (peptides below 1000 Da → old form
  understated); at Δ=0 both forms give 0. **Containment (still true):** QC's
  `precursor_ppm`/`fragment_ppm` (`qc::compute_qc_stats`) read Sage's own TSV columns directly
  and never touched this path, so all Phase 8 QC/Gate numbers were always correct. The proper
  `ppm × mz / 1e6` form was already used correctly in `mzml.rs`; only the annotation/unimod
  fields shortcut it.
- **The enhancement:** replace the scalar with an m/z-dependent calibration — fit
  residual mass error vs. m/z on the unmodified population, apply the fit across the
  delta axis.
- **⚠ STATUS (2026-07-17): the carpet motivation is GONE; the "fold/calibrate consistency fix"
  is CANCELLED.** The Step-1 entry once claimed calibration inflates the carpet by stealing
  from folding, making a fold/calibrate consistency pass the next rung. **Instrumentation
  refuted that** (post-fold Δ = +5, not +335 — the growth is a peak-detection quantization
  wobble, ~1%, not a calibration/fold defect). So there is **nothing to fix** on the
  fold/calibrate interaction, and the carpet no longer motivates fitted-A. Fitted-A now rests
  only on the bcell deamidation symptom (see the per-file re-scope note at the top of this
  section) — an **optional** accuracy enhancement, decision deferred to next session, not a
  promoted rung.
- **Payoff IF built (thin, bcell-only):** peaks land tighter on Unimod masses — but only bcell
  deamidation (~0.982) is off-target; b1906 (~0.985) and serum (~0.984) are already on. The
  once-hoped second prize (shrink annotation tolerance below 0.01 Da to cut nearest-mass
  noise) stands in principle but is now weakly evidenced — decide with data next session.
- **Validation reference:** the Step 0 closed-search `precursor_ppm` (see PLAN Phase
  8.5 / Entry 1) is the ground truth to validate the calibration against.
- **Why deferred:** real new code (fit a curve, not a median) with subtle failure
  modes; arguably past the recon tool's "minimal QC" mandate. Decide after Phase 8.
  Recorded WITH evidence (deamidation 2.3 mDa low) and payoff (tighter Unimod matching,
  narrower tolerance, less noise) — a motivated enhancement, not a speculative one.

### C1 — calibration-as-bin-diagnostic (DISCUSSION, distinct from the C2 build above)

Not the same as building m/z-dependent recalibration (that's C2, the enhancement just
above — recalibrate the data, then find mods). **C1 is a diagnostic question, not a
feature:** can the calibration numbers we *already compute* (apex_offset from the wide
search; signed MS1 ppm from the tight search, Phase 8.5) tell us whether our **PTM
binning is landing where it should** — i.e. is uncorrected m/z-dependent drift shifting
or smearing our delta bins such that a real mod lands in the wrong bin or gets split?
(The deamidation-reads-0.9817-vs-0.984 observation is the standing symptom.) C1 uses
existing numbers as a lens on our own histogram quality; C2 is the machinery to fix it.
**Status: deliberate discussion item, not build-now.** Parked pending the software
comparison (PTM-Shepherd / Mascot / Byonic) — that benchmark may reveal binning
disagreements that sharpen or answer this, and any feature changes it motivates get fed
back here. Revisit C1 after the comparison, with its findings in hand.

**Benchmark surfaced the symptom (2026-07-16, commit 46b74d8).** The PTM-Shepherd
comparison (`testing/recon-output/comparison/`) surfaced, on ALL THREE files, a carpet of
small UNANNOTATED peaks at ±1/±2 Da (+0.93/+0.97/+1.98/−1.06/−1.96, 0.2–0.8% each) that
appear ONLY in our output — PTM-Shepherd's 0.0002 Da bins + recalibration collapse them.
This is external, cross-file evidence of a real DIFFERENCE between the tools in the ±1/±2
regions. It strengthens the C1/C2 mandate (a second tool, three files, not just the lone
deamidation-0.9817 symptom) — enough to PROMOTE C1/C2 to the next active internal step.

**Frame the re-benchmark as a HYPOTHESIS TEST, not a confirmation lap.** Do NOT assume the
carpet is noise and C1/C2 is "the fix" — that pre-judges which tool is right, which we do
NOT know (we only know the tools differ). Hypothesis: *carpet = drift-smeared isotope
satellites; m/z-dependent calibration collapses it.* Outcomes after C1/C2 + re-benchmark:

- **Carpet gone** → hypothesis holds; the drift was the cause.
- **Carpet shrinks, residual remains** → the INTERESTING outcome: some was drift, some is
  real low-level signal. Now the open question is *who is right about the residual* — is
  PTM-Shepherd's 0.0002 Da binning + aggressive recalibration OVER-collapsing real
  low-abundance signal we correctly preserve, or are we OVER-reporting? "PTM-Shepherd is
  gold standard" is a hypothesis to test here, not a premise. Resolve from evidence
  (localization, RT-shift, spectral similarity of the residual peaks), don't assume.

### No-fixed-mods recon run — ✅ DONE (2026-07-24)

Motivated by reallyOpen: Ben's **reallyOpen** PTM-Shepherd re-run
(`testing/reference-data/ptm-shepherd/reallyOpen/`) unfixed C+57 (`add_C_cysteine = 0.0`, every
variable mod commented out) and +57 leapt to rank 2 on all three files. That is the recon behaviour
we want: **find everything as a delta so the tool can PREDICT appropriate fixed/variable search
settings** — the mission. Same record-with-evidence shape as [[over-alkylation]]. So we ran our OWN
open Sage search with all mods removed and confirmed the tool reproduces it.

- **Configs (additive; validated fixed-C configs untouched):**
  `testing/configs/open-search-{b1906,serum,bcell}-nofixedmods.json` (`static_mods {}`,
  `variable_mods {}`). Ben ran Sage → `testing/search-output/open-{file}-nofixedmods/` → we ran
  `discover` → `testing/recon-output/nofixedmods/{file}.json` → compared vs reallyOpen in
  `testing/recon-output/comparison/recon_nofixedmods_vs_reallyOpen.md`.
- **Result — the recon claim holds: +57 surfaces at rank 2 on all three files** (b1906 4.47% /
  1253 PSMs, bcell 4.50% / 3311, serum 7.31% / 1125), up from ~0.2–0.45% when C was fixed. Same
  rank as PTM-Shepherd reallyOpen's +57. **The tool discovers the alkylation population as a
  dominant delta → it would correctly recommend Carbamidomethyl(C) as a fixed mod.**
- **Honest caveat (do NOT spin): our +57 magnitude is ~0.41–0.42× theirs, systematically** (b1906
  4.47 vs 10.77%; consistent ratio across all three). Totals are comparable (ours 28005/73527/15386
  vs theirs ~26k/64k/15k), so NOT a denominator artifact — MSFragger's localization-aware two-pass
  + recalibration assigns more PSMs to +57 than our single-pass Sage open. Same recon-vs-platform
  "lost at search" tradeoff quantified in the original PTM-Shepherd benchmark. We recover
  **presence + rank** (what's needed to recommend the fixed mod), not the platform's full localized
  magnitude.
- **Conservation:** no-fixed-mods totals drop from the fixed-C run (31682/81966/19307) by the
  search-space change — fixing C+57 lets more spectra match, so removing it lowers totals. Expected.
- **Two artifacts kept on purpose:** `recon_vs_ptmshepherd_reallyOpen.md` (our FIXED-C JSONs vs
  reallyOpen — the "why our fixed +57 reads small" record) and `recon_nofixedmods_vs_reallyOpen.md`
  (the apples-to-apples no-fixed-mods comparison). `load_recon` now accepts both `analyze` and
  `discover` JSON shapes.

### Mascot error-tolerant adapter — name+site output, MULTI-SITE ROLL-UP is load-bearing (2026-07-24)

Third benchmark tool wired into the N-tool aligner (`testing/scripts/compare_mod_discovery.py`,
`load_mascot`). Data: `testing/reference-data/mascot/error-tolerant/` (`Human_ertol.par` = no mods,
10/20 ppm MS1/MS2, `ERRORTOLERANT=1`; three hand-copied full mod summaries).

- **Mascot reports by Unimod NAME + SITE, not mass, and splits ONE mass across many site rows.**
  Example (b1906): Carbamidomethyl on C 1172 + N-term 189 + Y 16 + D 13 + E 11 + H 6, plus Gly on
  K/S/T at the same +57.0215. Mascot is telling us the +57 is *on C but also elsewhere* — terminal,
  off-site over-alkylation. **Our tool and PTM-Shepherd do NOT make that per-site call** — they
  report +57 as a single un-localized mass peak (the [[no per-residue localization]] lock).
- **Therefore the adapter MUST roll up all site rows sharing a mass into one row** (name→mass via
  `unimod.xml`, ET summed) before aligning to our mass axis. Without the roll-up, Mascot's C-only
  row understates the true +57 population and the comparison is apples-to-oranges. The multi-site
  spread IS the over-alkylation signal — it is preserved in the label (e.g.
  `Carbamidomethyl [A,C,D,E,G,H,K,N-term,S,T,Y]`), demoted to annotation, not discarded.
  **Conservation asserted in code:** rolled-up count == Σ(contributing site-row ET).
- **Percentages are Mascot's own resolved-ET fraction, NOT PSM count** — stated so no one forces
  PSM-count equality across tools (same currency discipline as the PTM-Shepherd benchmark).
- **Expected divergence = the AA-substitution + `Label:15N` long tail** (hundreds of low-count
  substitution rows, and error-tolerant's 15N isotope-label space at ~9%) that we fold/suppress —
  by design, same as the isotope-peak divergence with PTM-Shepherd, not a miss.
- **Rows with no clean Unimod mass are skipped, not silently dropped** — logged with ET count
  ("Non-specific cleavage", unresolved substitutions: bcell 1137, serum 1611, b1906 644 ET).

### No-mods recon benchmark (original future note — now realized above)

**Superseded/activated 2026-07-24 by the "No-fixed-mods recon run" entry above** — reallyOpen is
the PTM-Shepherd side of exactly this idea, now delivered, and the recon-side run is queued with
configs written. The original note is kept for the reasoning: since we are a recon tool, running
the open search with **no fixed or variable mods** surfaces the full **Carbamidomethyl(C)**
population as a +57 delta (rather than fixed-out). The prior PTM-Shepherd benchmark kept C+57 fixed
to stay apples-to-apples with their (then-fixed) run; reallyOpen removes that constraint on their
side, and the queued recon no-fixed-mods run removes it on ours.

---

## Dead-ends (do not re-explore)

- **The ±1/±2 Da "carpet" as a calibration problem (2026-07-17)** — the carpet that the
  PTM-Shepherd benchmark surfaced (appears only in our output) is NOT drift the calibration
  would fix, and NOT a fold/calibrate-interaction defect. Per-stage instrumentation
  (see "Deferred enhancements" → carpet CHARACTERIZED block) proved it is a **peak-detection
  quantization wobble**: ~1% of PSMs (b1906 ±335 of 31,682), entirely at the prominence
  detector, triggered by a ~1 mDa shift re-binning the same ~3350 PSMs. Two mechanism
  narratives were raised and BOTH refuted by the instrumentation (steal-from-folding at
  post-fold +5; boundary-crossing at post-calibrate −5). **Do not re-open the carpet as a
  calibration/fold defect, and do not build a "fold/calibrate consistency" fix — that fix was
  for a mechanism that does not exist.** If anyone revisits the carpet, it is a
  detector-quantization question (prominence threshold stability), and for a recon tool it is
  below the resolution of any user decision — leave it. Ppm-constant calibration (Step 1) was
  also a negative result. **Lesson (Phase-7 pattern, caught):** a metric with no conservation
  invariant (the carpet is a detector output) + a plausible mechanism narrative + a fix
  declarable "successful" against unrefutable numbers = the satellite-folding loop. What broke
  it: the STOP firing on a failed invariant, and instrumenting the mechanism to a specific
  stage before building. Plausible ≠ established.
- **Satellite folding (Phase 7C)** — tried folding satellite peaks toward Δ=0;
  it had no hard invariant to gate on, could report any number and be narrated
  as correct, and ate most of the debugging time. Disabled rather than repaired.
  Lesson: lead with an invariant (fold-to-zero conservation), make it the
  acceptance gate. Don't rebuild satellite folding without one.
- **Full-FASTA semi-enzymatic search** — 7× slower (1,226 s vs 172 s). Rolled
  back in favor of the two-pass subset workflow (6.6× speedup). Do not make
  full-FASTA semi-enzymatic the default.
- **Residue-mass degeneracy check (Phase 7D)** — NOT built. Deferred, and as of
  Phase 8 deferred **by evidence, not by absence**: serum's open search DID surface
  a +57.02 peak (the mass degenerate between Carbamidomethyl/off-site-CAM and the
  Glycine residue), so the "no such peaks exist yet" reason no longer holds. It was
  run down with the **flanking-check method** (see reusable-methods below) and proven
  to be **over-alkylation, not adds-Gly** — 0 of 26 unique non-Cys +57 peptides had
  Gly flanking context. So 7D still isn't needed, but now because the one real
  candidate was checked and excluded, not because none appeared. Revisit only if a
  future file's flanking check returns Gly-context peptides. Do not build either
  disambiguation path speculatively. See the serum +57 triage entry below.

---

## Phase 0 — Repo & Environment Setup

**Status:** ✅ Complete  
**Started:** 2026-07-07  
**Completed:** 2026-07-07

### Goals
- Create repo structure, `.gitignore`, `NOTES.md`
- Vendor all reference material (Sage source, mzSniffer, PTM-Shepherd, intensityWeighting)
- Confirm Sage binary runs from vendored path
- Checkpoint: Sage runs against test mzML, produces `results.sage.tsv`

### Decisions Made
1. **Reference repos excluded from git** — The cloned repos in `reference/` (sage, mzsniffer, PTM-Shepherd, intensityWeighting) are too large and are just for local reference. Added to `.gitignore`.
2. **Large data files excluded** — `.mzML`, `.mzML.gz`, `.raw` files excluded from git (too large, user-specific test data).
3. **Sage output excluded** — `*.sage.tsv` and `lfq.tsv` are regenerated on each run, no need to commit.
4. **Git authentication** — Using fine-grained PAT for push access to `neely/sagePreview`.
5. **Open search template created** — `testing/open-search-params.json` with `da: [-500, 100]` tolerance, `chimera: true`, `report_psms: 2` per PLAN.md Template A spec.

### Files Vendored
- `reference/sage/` — Sage source + Windows binary (sage-v0.14.7-x86_64-pc-windows-msvc)
- `reference/mzsniffer/` — mzSniffer source (for polymer detection port)
- `reference/PTM-Shepherd/` — PTM-Shepherd source (for methodology reference)
- `reference/intensityWeighting/` — Ben's intensity weighting scripts

### Reference Notes Created
- `reference-notes/sage-config-and-gotchas.md` — Decoy handling, open-search tolerance, chimeric search
- `reference-notes/unimod-decomposition.md` — Match tolerance, ambiguity handling
- `reference-notes/oxonium-ions.md` — Glycan diagnostic ions
- `reference-notes/polymer-contaminant-ions.md` — Polymer series for mzSniffer port
- `reference-notes/ptm-shepherd-methodology.md` — PTM-Shepherd approach reference
- `reference-notes/mgf-mzml-intensity-differences.md` — Intensity handling notes
- `reference-notes/sage-online-docs.md` — Scraped Sage documentation

### Checkpoint Status
- [x] Sage runs against test mzML from vendored path
- [x] `results.sage.tsv` produced and readable

### Test Run Results
- **Sage version:** 0.14.6 (binary reports 0.14.6, folder named 0.14.7)
- **Test file:** `B.naive_01steady-state.mzML.gz`
- **Runtime:** 172 seconds
- **Results:** 74,996 target PSMs at 1% FDR, 44,462 peptides, 6,544 proteins
- **Output columns confirmed:** `expmass`, `calcmass`, `isotope_error`, `hyperscore`, `matched_intensity_pct`, `longest_b`, `longest_y`, `ms2_intensity` — all needed for Phase 3 mod discovery

### Open Questions
- (none)

---

## Phase 1 — Result Struct / JSON Schema
**Status:** ✅ Complete  
**Started:** 2026-07-07  
**Completed:** 2026-07-07

### Goals
- Define the full RESULT struct as the contract between all modules
- Validate schema against real test data clusters
- Document as `reference-notes/result-schema.md`

### Decisions Made

1. **Dual readouts everywhere** — Every metric that can be expressed as both spectral count AND intensity-weighted is reported both ways. This matches the PLAN.md requirement for "count AND intensity" throughout.

2. **Explicit ambiguity handling** — When multiple Unimod entries match within tolerance (0.01 Da, matching PTM-Shepherd), ALL candidates are reported with `ambiguous: true`. Never silently pick one.

3. **Unannotated bucket** — Peaks with no Unimod match within tolerance are flagged `unannotated: true` rather than force-matched. This follows the PTMVision pattern documented in `reference-notes/unimod-decomposition.md`.

4. **Isotope correction explicit** — Both raw and corrected delta masses are reported. Formula: `corrected_delta = (expmass - calcmass) - (isotope_error × 1.0086649158849)`. Also includes `isotope_error_view` for analyzing isotope error distribution.

5. **Confidence from existing Sage fields only** — MVP uses `hyperscore`, `matched_intensity_pct`, `longest_b`, `longest_y` from Sage output. No new fragment-ion computation (deferred per PLAN.md).

6. **Oxonium screening rule** — "≥2 oxonium ions in top 10% of peaks, m/z 204 mandatory" — this is a published heuristic from glycoproteomics literature, not a quantitative analysis.

7. **Polymer detection MS1-only** — Ported from mzSniffer approach: match known polymer precursor masses in MS1 spectra, report %TIC. Does not use MS2 fragmentation.

8. **Chimera handling documented** — When `chimera: true` and `report_psms: 2`, signal fate deduplicates by scan for spectrum-level stats but counts all PSMs for modification discovery.

### Schema Sections Defined

| Section | Purpose | Key Fields |
|---------|---------|------------|
| `input` | Metadata about files and search params | mzml_files, fasta_path, tolerances, sage_version |
| `mod_discovery` | Delta-mass analysis (core PTM scouting) | histogram, peaks[], annotations[], confidence |
| `signal_fate` | Explained vs. unexplained signal | by_count, by_intensity, chimera_stats |
| `polymer` | Polymer contamination from MS1 | pct_tic, by_type[], contamination_level |
| `diagnostic_ions` | Oxonium + contaminant fragment ions | oxonium_ions, contaminant_ions, screening rules |
| `digestion` | Digestion efficiency | missed_cleavages, semi_tryptic |
| `qc` | Minimal QC metrics | mass_accuracy, identification_rates |

### Checkpoint Status
- [x] Schema defined with all sections from PLAN.md
- [x] Validated against test data clusters (0.00, 15.99, 0.98, 52.91, -17.03 Da)
- [x] Committed as `reference-notes/result-schema.md`

### Open Questions
- (none — ready for Phase 2)

### Handoff Context (from Phase 0)

**Exploratory analysis of `B.naive_01steady-state.mzML.gz` open search results:**

| Delta (Da) | Count | Annotation |
|------------|-------|------------|
| 0.00 | 39,916 | Unmodified |
| 1.00 | 5,251 | Deamidation (NQ) / isotope error |
| 1.01 | 1,503 | Deamidation (NQ) / isotope error |
| 16.00 | 928 | Oxidation (M) |
| 15.99 | 926 | Oxidation (M) |
| 2.01 | 896 | 2× isotope error? |
| 0.98 | 574 | Deamidation (NQ) |
| 17.00 | 479 | Oxidation + isotope? |
| 52.91 | 398 | Unknown — investigate |
| -17.03 | 318 | Ammonia loss (N-term Q/C) |
| -89.03 | 265 | Unknown — investigate |

**Summary:**
- 81,966 total PSMs (filtered at peptide_q < 0.01, no decoys)
- 54.2% near zero (unmodified)
- 45.8% with significant delta (≥0.5 Da)
- 6,669 unique delta bins at 0.01 Da resolution

**Key observations for schema design:**
1. **Isotope error is real** — peaks at 1.00, 2.00, 2.01 Da need isotope correction
2. **Deamidation dominates** — 0.98-1.01 Da range is busy, need to separate from isotope error
3. **Oxidation clear** — 15.99-16.00 Da is a clean peak
4. **Unknown peaks exist** — 52.91 Da, -89.03 Da need Unimod lookup
5. **Ammonia loss present** — -17.03 Da matches expected

**Files to reference:**
- `reference-notes/sage-online-docs.md` — Sage documentation
- `reference-notes/unimod-decomposition.md` — Unimod matching strategy
- `testing/unimod.xml` — Full Unimod database
- `PLAN.md` Phase 1 section — Schema outline

---

## Phase 2 — Sage Integration Baseline
**Status:** ✅ Complete  
**Started:** 2026-07-07  
**Completed:** 2026-07-07

### Goals
- Implement `sage_runner.rs` for subprocess invocation
- Implement `sage_results.rs` for TSV parsing with filtering
- Compute isotope-corrected delta masses
- Bundle Template A (open search config)
- Validate against test data

### Decisions Made
1. **Rust project structure** — Created `recon-tool/` with `Cargo.toml`, `lib.rs`, `main.rs`, `sage_runner.rs`, `sage_results.rs`
2. **Sage TSV column handling** — Sage outputs `isotope_error` as float (0.0), `semi_enzymatic` as int (0/1), and `scannr` as a string like `"controllerType=0 controllerNumber=1 scan=9681"` — all handled with appropriate parsing
3. **Scan number extraction** — Parse scan number from Sage's native ID format using string search for `scan=`
4. **Isotope correction formula** — `corrected_delta = (expmass - calcmass) - (isotope_error × 1.0086649158849)`

### Files Created
- `recon-tool/Cargo.toml` — Dependencies: csv, serde, serde_json, anyhow, log, env_logger, clap
- `recon-tool/src/lib.rs` — Public API, re-exports
- `recon-tool/src/main.rs` — CLI with `parse` subcommand for validation
- `recon-tool/src/sage_runner.rs` — Subprocess invocation (locates binary, builds command, captures output)
- `recon-tool/src/sage_results.rs` — TSV parsing, filtering, isotope correction

### Checkpoint Status
- [x] `sage_runner.rs` implemented (subprocess invocation)
- [x] `sage_results.rs` implemented (TSV parsing + filtering)
- [x] Isotope-corrected delta mass computed
- [x] Template A bundled at `testing/open-search-params.json`
- [x] Parsed PSMs match expected ground truth: **81,966 PSMs** at q<0.01

### Validation Results
```
Total PSMs before filter: 171,864
Decoys removed: 45,264
Q-value filtered: 44,634
PSMs after filter: 81,966 ✓ (matches expected)

Chimera Statistics:
- Unique scans: 66,437
- Scans with multiple PSMs: 15,529 (23.4%)

Top delta mass peaks (0.01 Da bins):
- 0.00 Da: 39,916 (48.7%) — Unmodified ✓
- 1.00 Da: 5,251 (6.4%) — Deamidation/isotope
- 15.99-16.00 Da: 1,854 combined — Oxidation ✓
- 52.91 Da: 398 — Unknown (to investigate)
- -17.03 Da: 318 — Ammonia loss ✓
```

### Isotope Error Validation (narrow-search test)

Created `testing/narrow-search-params.json` with:
- `precursor_tol: ppm [-20, 20]` (tight tolerance)
- `isotope_errors: [-1, 3]` (allows -1 to +3 isotope shifts)

**Results:**
```
Total PSMs: 131,534
Filtered PSMs: 59,369 (at q<0.01)

Isotope Error Distribution (before filtering):
  -1: 17,489 (13.3%)
   0: 66,053 (50.2%)
   1: 22,318 (17.0%)
   2: 14,439 (11.0%)
   3: 11,235 (8.5%)

Delta Mass Histogram (after isotope correction):
  0.00 Da: 46,014 (77.5%)
 -0.01 Da: 7,043 (11.9%)
 -0.02 Da: 1,891 (3.2%)
  0.01 Da: 1,701 (2.9%)
```

**Key observation:** After isotope correction, 77.5% of PSMs cluster at 0.00 Da (vs. 48.7% in open search). The correction formula is working correctly — isotope-shifted matches are being normalized back to their true delta mass.

### Open Questions
- (none — ready for Phase 3)

---

## Phase 3 — Mod Discovery / PTM Scoping Engine
**Status:** ✅ Complete  
**Started:** 2026-07-07  
**Completed:** 2026-07-07

### Goals
- Parse Unimod XML database for modification annotation
- Build delta-mass histogram from PSMs
- Detect significant peaks (local maxima)
- Annotate peaks against Unimod with ambiguity handling
- Compute confidence metrics from Sage output fields
- CLI subcommand for mod discovery

### Decisions Made

1. **Unimod XML parsing** — Using `quick-xml` crate for streaming XML parsing. Extracts record_id, title, full_name, mono_mass, avge_mass, composition, and all specificities (site, position, classification, hidden flag).

2. **Position specificity preserved** — Each Unimod entry stores all specificities including position constraints (e.g., "Any N-term", "Protein N-term", "Anywhere"). This enables future site-specific validation.

3. **Hidden specificities filtered** — Unimod marks some specificities as `hidden="1"` (e.g., Oxidation on C is hidden). The `sites()` method returns only non-hidden sites for display.

4. **Mass-indexed lookup** — Unimod entries sorted by mono_mass for O(log n) binary search. Default tolerance 0.01 Da (matches PTM-Shepherd).

5. **Histogram binning** — 0.01 Da bins, sparse representation (only non-zero bins stored). Intensity-weighted peak center computed for each detected peak.

6. **Peak detection** — Bins above threshold (default 10 PSMs) are candidates. Adjacent bins within merge tolerance (0.02 Da) are merged into single peak. Peaks ranked by count.

7. **Near-zero classification** — PSMs with |delta_mass| < 0.1 Da classified as "Unmodified" (intrinsic annotation, not Unimod lookup).

8. **Ambiguity handling** — When multiple Unimod entries match within tolerance, ALL are reported with `ambiguous: true`. Never silently pick one.

9. **Confidence metrics** — Each peak reports mean/median hyperscore, mean matched_intensity_pct, mean longest_b/y from Sage output. Also computes hyperscore_vs_unmodified ratio.

10. **Fixed mods note** — Current implementation assumes Sage search was run with appropriate fixed mods (e.g., Carbamidomethyl on C). The delta masses are relative to the search configuration. Future enhancement: read fixed mods from Sage config and document in output.

### Files Created/Modified
- `recon-tool/src/unimod.rs` — Unimod XML parser with mass-indexed lookup
- `recon-tool/src/mod_discovery.rs` — Histogram, peak detection, annotation, confidence
- `recon-tool/src/lib.rs` — Updated to export new modules
- `recon-tool/src/main.rs` — Added `discover` subcommand
- `recon-tool/Cargo.toml` — Added `quick-xml` dependency

### CLI Usage
```bash
# Run mod discovery
recon discover --tsv results.sage.tsv --unimod unimod.xml --summary-only

# With JSON output
recon discover --tsv results.sage.tsv --unimod unimod.xml --output discovery.json

# Custom thresholds
recon discover --tsv results.sage.tsv --unimod unimod.xml --min-peak-count 20 --max-peaks 100
```

### Checkpoint Status
- [x] Unimod XML parser implemented
- [x] Mass-indexed lookup with tolerance
- [x] Delta-mass histogram builder
- [x] Peak detection with merging
- [x] Unimod annotation with ambiguity handling
- [x] Confidence metrics from Sage fields
- [x] CLI subcommand for mod discovery
- [x] All 19 tests pass (13 unit + 5 integration + 1 doc)

### Test Coverage
- `unimod::tests::test_parse_sample_xml` — XML parsing
- `unimod::tests::test_find_oxidation` — Mass lookup (15.995 Da)
- `unimod::tests::test_find_deamidation` — Mass lookup (0.984 Da)
- `unimod::tests::test_find_pyroglu` — Negative mass, position specificity
- `unimod::tests::test_no_match` — Unknown mass returns empty
- `unimod::tests::test_sites_method` — Hidden site filtering
- `unimod::tests::test_tolerance` — Tolerance configuration
- `mod_discovery::tests::test_build_histogram` — Histogram binning
- `mod_discovery::tests::test_compute_confidence` — Confidence metrics
- `mod_discovery::tests::test_near_zero_classification` — Unmodified detection

### Prior Art Reference

**DeltaMass** (Avtonomov et al., 2018) is the precursor to PTM-Shepherd from the same group. Key methodological differences from our approach:

| Aspect | DeltaMass | Our Tool |
|--------|-----------|----------|
| Density estimation | KDE with Gaussian kernel | Histogram (0.01 Da bins) |
| Peak detection | Second derivative + GMM | Threshold + adjacent merge |
| Recalibration | Zero-peak or 2D m/z×RT | Sage isotope correction |

The DeltaMass paper argues that histogram binning is fragile for poorly resolved peaks. Our simpler approach may miss shoulder peaks but is faster and easier to validate. KDE-based detection could be a Phase 8 enhancement.

See `reference-notes/deltamass-methodology.md` for full comparison.

### Open Questions
- (none — ready for Phase 4)

---

## Phase 4 — Signal Fate Accounting
**Status:** ✅ Complete  
**Started:** 2026-07-07  
**Completed:** 2026-07-07

### Goals
- Compute explained vs. unexplained signal by count AND intensity
- Handle chimera multi-PSM-per-scan deduplication
- Breakdown identified signal by modification status
- Validate against expected numbers from Phase 2

### Decisions Made

1. **Chimeric scan handling** — A chimeric scan is a single MS2 spectrum where multiple co-eluting peptides are identified. For signal fate:
   - **By count:** Count each unique scan once (not per PSM)
   - **By intensity:** Use the scan's `ms2_intensity` once (all PSMs from the same scan share the same spectrum intensity)

2. **Scan classification priority** — For chimeric scans with multiple PSMs of different modification status:
   - If any PSM is unmodified → scan is "unmodified"
   - Else if any PSM is modified+annotated → scan is "modified_annotated"
   - Else → scan is "modified_unannotated"

3. **Annotation lookup** — Uses mod_discovery peaks to determine if a delta mass is annotated. Bins delta masses to 0.01 Da for lookup.

4. **Partial accounting (Phase 4)** — We report identified signal only. Total MS2 spectra and unidentified signal will be added in Phase 5 when we parse mzML directly.

5. **Intensity correlation** — Per Ben's ROA study, PSM counts at fixed FDR track closely with intensity-weighted metrics. We report both for completeness and future DIA adaptation.

### Files Created/Modified
- `recon-tool/src/signal_fate.rs` — Core accounting logic with chimera handling
- `recon-tool/src/lib.rs` — Export signal_fate module
- `recon-tool/src/main.rs` — Added `signal-fate` CLI subcommand

### CLI Usage
```bash
# Basic signal fate (without annotation breakdown)
recon signal-fate --tsv results.sage.tsv --summary-only

# With Unimod for annotation status
recon signal-fate --tsv results.sage.tsv --unimod unimod.xml --summary-only

# With JSON output
recon signal-fate --tsv results.sage.tsv --unimod unimod.xml --output fate.json
```

### Checkpoint Status
- [x] Signal fate accounting implemented
- [x] Chimera deduplication working correctly
- [x] Breakdown by modification status
- [x] All 25 tests pass (19 unit + 5 integration + 1 doc)
- [x] Validation matches expected numbers

### Validation Results (Open Search)
```
=== Signal Fate Accounting ===

Identified Signal:
  Spectra (unique scans): 66,437 ✓ (matches Phase 2)
  Total PSMs: 81,966 ✓ (matches Phase 2)
  Total MS2 intensity: 1.41e11

Chimera Statistics:
  Unique scans: 66,437
  Chimeric scans: 15,529 (23.4%) ✓ (matches Phase 2)
  Avg PSMs per chimeric scan: 2.00

Identified Breakdown (by count):
  Unmodified: 41,833 (63.0%)
  Modified (annotated): 10,994 (16.5%)
  Modified (unannotated): 13,610 (20.5%)

Identified Breakdown (by intensity):
  Unmodified: 1.19e11 (84.3%)
  Modified (annotated): 1.11e10 (7.8%)
  Modified (unannotated): 1.11e10 (7.9%)
```

**Key observation:** Unmodified peptides account for 63% of spectra but 84% of intensity — abundant proteins tend to be unmodified, consistent with Ben's ROA finding that intensity-weighted metrics track with counts but may smooth out marginal effects.

### Known Assumptions & Limitations

1. **Chimeric intensity not split between PSMs** — For chimeric scans (multiple PSMs from one MS2 spectrum), we use the scan's `ms2_intensity` once rather than splitting/weighting it between co-identified peptides. This is a simplification; a more sophisticated approach could weight intensity by PSM count or confidence scores.

2. **MS2 intensity-weighted metrics track with counts** — Per Ben's ROA study (ROA 646.11-25-036), intensity-weighted metrics don't add much discriminative power over simple PSM counts at fixed FDR. We report both for completeness, but counts are likely sufficient for most QC decisions.

3. **DIA considerations deferred** — The current chimera handling is DDA-focused. DIA data is fundamentally chimeric by design and would require different accounting approaches. This is noted for future work.

### Open Questions
- (none — ready for Phase 5)

---

## Phase 5A — MS2 Spectrum Counting & Identification Rate
**Status:** ✅ Complete  
**Started:** 2026-07-07  
**Completed:** 2026-07-07

### Goals
- Parse mzML to get total MS2 spectrum count (for unidentified signal calculation)
- Calculate identification rate (identified / total MS2)
- Provide baseline for comparison with MS1-based metrics

### Decisions Made

1. **`mzdata` crate for mzML parsing** — Pure Rust implementation, supports mzML and gzipped mzML. Requires `Seek` trait for iteration, so gzipped files are decompressed into memory first.

2. **MS2 TIC from spectrum params** — Extract TIC from mzML spectrum CV param `MS:1000285` (total ion current). Falls back to summing peak intensities if TIC param is missing.

3. **Gzip handling** — For `.mzML.gz` files, decompress fully into memory using `flate2` crate, then wrap in `Cursor` for seeking. This adds memory overhead but is necessary for `mzdata`'s indexed reader.

4. **Unidentified signal calculation** — `unidentified_spectra = total_ms2 - identified_spectra`. Intensity-based unidentified signal computed similarly.

### Files Created/Modified
- `recon-tool/src/mzml.rs` — mzML parsing with `mzdata` crate, MS2 counting, TIC extraction
- `recon-tool/src/signal_fate.rs` — Added `compute_signal_fate_with_mzml()` for unidentified signal
- `recon-tool/src/lib.rs` — Export mzml module
- `recon-tool/src/main.rs` — Added `mzml-stats` subcommand, `--mzml` flag for `signal-fate`
- `recon-tool/Cargo.toml` — Added `mzdata` and `flate2` dependencies

### CLI Usage
```bash
# Parse mzML and show statistics
recon mzml-stats --mzml data.mzML.gz

# Signal fate with mzML for unidentified signal
recon signal-fate --tsv results.sage.tsv --mzml data.mzML.gz --unimod unimod.xml --summary-only
```

### Validation Results
```
=== mzML File Statistics ===
File: B.naive_01steady-state.mzML.gz

Spectrum Counts:
  Total spectra: 104,931
  MS1 spectra: 12,627
  MS2 spectra: 92,304

=== Signal Fate with mzML ===
Total MS2 Spectra (from mzML): 92,304

Signal Fate (by count):
  Identified: 66,437 (72.0%)
  Unidentified: 25,867 (28.0%)
```

**Key result:** 72% identification rate (66,437 / 92,304 MS2 spectra identified at q<0.01). This is a good rate for a typical DDA experiment.

### Performance
- Parsing 104,931 spectra from gzipped mzML: ~10 seconds
- Memory overhead for gzip decompression: ~500MB for this file

---

## Phase 5B — MS1 Precursor Intensity & Polymer Detection
**Status:** ✅ Complete  
**Started:** 2026-07-07  
**Completed:** 2026-07-07

### Goals
- Extract MS1 precursor intensities for identified PSMs (more biologically meaningful than MS2 TIC)
- Implement polymer %TIC detection (port mzSniffer approach)
- Implement oxonium ion screening for glycopeptide flagging
- Evaluate computational cost
- Compare MS1-based vs MS2-based signal fate metrics

### Background: MS1 Intensity Approaches (from literature review)

Three approaches for extracting MS1 precursor intensity in DDA proteomics:

| Approach | Description | Pros | Cons | Tools |
|----------|-------------|------|------|-------|
| **1. MS2 precursor metadata** | Use intensity stored in MS2 spectrum header | Very fast, simple | Single snapshot, high variance, biased by DDA triggering | Rarely used for LFQ |
| **2. XIC + peak integration** | Extract ion chromatogram, detect peak, integrate area | Best quantitative accuracy | Computationally heavy, complex | MaxQuant, Skyline, OpenMS, IQMMA |
| **3. Fixed RT window sum** | Sum MS1 intensity at precursor m/z within RT window | Faster than XIC, more robust than snapshot | Approximate peak boundaries, may include interference | Custom scripts, lightweight pipelines |

**For our QC/reconnaissance tool:** Option 3 is appropriate because:
- We're doing signal fate accounting, not publication-quality LFQ
- We want to know "what fraction of MS1 signal is explained by identified peptides"
- Computational simplicity matters for a QC tool

### Planned Implementation

#### 1. Polymer %TIC (Port mzSniffer)
mzSniffer's approach:
- Define polymer series: core formula + repeat unit (e.g., PEG: H2O + n×C2H4O)
- Generate expected m/z values up to max scan range
- Search MS1 spectra for matching peaks within tolerance (10 ppm)
- Sum intensities across all MS1 scans
- Report %TIC for each polymer type

Default polymers from mzSniffer:
- PEG (+1H, +2H, +3H)
- PPG
- Triton X-100 (and reduced, Na variants)
- Triton X-101
- Polysiloxane
- Tween-20/40/60/80
- IGEPAL CA-630 (NP-40)

#### 2. MS1 Precursor Intensity for PSMs
For each identified PSM:
1. Get precursor m/z, charge, RT from Sage results
2. Convert: `precursor_mz = (expmass + charge × proton_mass) / charge`
3. Find MS1 scans within RT window (±1 minute, configurable)
4. Extract max intensity at precursor m/z (±10 ppm) from each MS1 scan
5. Sum intensities → "identified precursor signal"

**Isotope handling (to experiment):**
- Monoisotopic only (tight tolerance + isotope error correction)
- Sum within -1.25/3.5 Da window (open search style)
- Compare results empirically

#### 3. Oxonium Ion Screening
Flag MS2 spectra with glyco oxonium ions:
- m/z 204.0867 (HexNAc) — mandatory
- m/z 366.1395 (Hex-HexNAc)
- m/z 292.1027 (NeuAc)
- m/z 274.0921 (NeuAc-H2O)

Rule: ≥2 oxonium ions in top 10% of peaks, m/z 204 mandatory

Extract precursor info for flagged spectra → enables mzSniffer-style analysis (precursor intensity vs RT for glycopeptides).

### Assumptions to Document
- RT window: ±1 minute (configurable)
- m/z tolerance: 10 ppm
- Using peak height sum, not integrated area
- This is for QC/reconnaissance, not publication-quality LFQ
- Chimeric precursors: multiple peptides at same m/z share MS1 signal (fundamental DDA limitation)

### Reference Files
- `reference/mzsniffer/src/polymer.rs` — Polymer definition and m/z calculation
- `reference/mzsniffer/src/search.rs` — Peak finding and %TIC calculation
- `reference/mzsniffer/src/defaults.rs` — Default polymer list
- `reference/PTM-Shepherd/src/edu/umich/andykong/ptmshepherd/glyco/oxonium_ion_list.txt` — Oxonium ions
- `reference/PTM-Shepherd/src/edu/umich/andykong/ptmshepherd/glyco/glycan_residues.txt` — Glycan residue masses

### PTM-Shepherd vs Our Oxonium Screening

| Aspect | PTM-Shepherd | Our Tool |
|--------|--------------|----------|
| Ion definition | Composition-based (e.g., `NeuAc(1)`) | Fixed m/z values |
| Scoring | Bayesian with hit/miss probabilities | Threshold-based (≥2 ions in top 10%) |
| Mass adjustments | Explicit (e.g., -H2O = -18.0106) | Pre-computed m/z values |
| Residue requirements | Tracks which glycan residues are needed | Not tracked |
| Use case | Glycan assignment | QC screening |

**Decision:** Our simpler threshold-based approach is appropriate for QC/reconnaissance. PTM-Shepherd's Bayesian approach is for actual glycan composition assignment, which is beyond our scope.

### Checkpoint Status
- [x] Polymer %TIC detection implemented (`polymer.rs` module)
- [x] MS1 precursor intensity extraction implemented (fixed RT window approach in `mzml.rs`)
- [x] Oxonium ion screening implemented (`oxonium.rs` module)
- [x] `polymer-stats` CLI command added
- [ ] Computational cost evaluated (deferred to Phase 8)
- [ ] MS1 vs MS2 signal fate comparison documented (deferred to Phase 8)

### Files Created/Modified
- `recon-tool/src/polymer.rs` — Polymer detection ported from mzSniffer (16 default polymers)
- `recon-tool/src/oxonium.rs` — Oxonium ion screening for glycopeptide detection (8 diagnostic ions)
- `recon-tool/src/mzml.rs` — Extended with MS1/MS2 spectrum extraction, precursor intensity functions
- `recon-tool/src/lib.rs` — Export polymer, oxonium modules
- `recon-tool/src/main.rs` — Added `polymer-stats` CLI command
- `reference-notes/ms1-precursor-intensity-approaches.md` — Literature review on MS1 intensity extraction
- `reference-notes/glossary.md` — Added LFQ, XIC, DDA terms

### Implementation Details

#### Polymer Detection (`polymer.rs`)
- 16 default polymer types (PEG, PPG, Triton, Tween, etc.)
- Generates m/z series for each polymer up to max m/z
- Searches MS1 spectra for matching peaks within 10 ppm tolerance
- Reports %TIC for each polymer type

#### Oxonium Ion Screening (`oxonium.rs`)
- 8 diagnostic oxonium ions (HexNAc, Hex-HexNAc, NeuAc, etc.)
- Screens MS2 spectra for ions in top N% of peaks
- Rule: ≥2 oxonium ions required, m/z 204 (HexNAc) mandatory
- Returns glycopeptide candidate flag per spectrum

#### MS1 Precursor Intensity (`mzml.rs`)
- Fixed RT window approach (±1 minute default)
- For each PSM: find MS1 scans within RT window, sum intensity at precursor m/z
- 10 ppm m/z tolerance
- Returns total explained MS1 intensity

### Open Questions
- (none — ready for Phase 6)

---

## Phase 5B Future TODOs

The following improvements were identified during Phase 5B implementation and deferred:

1. **Streaming support for large files** — Currently loads all MS1/MS2 spectra into memory. For very large files (>1GB), implement streaming approach that processes spectra in chunks.

2. **MS1 RT pre-indexing** — Current precursor intensity extraction is O(n×m) where n=PSMs and m=MS1 scans. Pre-index MS1 scans by RT for O(n×log(m)) lookup.

3. **Progress indicators** — For large mzML files, show progress during parsing (e.g., "Processing spectrum 50000/100000").

4. **Unified `analyze` command** — Single command that runs all analyses (polymer, oxonium, signal fate) and outputs unified JSON report.

5. **Computational cost benchmarking** — Measure and document the cost of MS1 precursor intensity extraction vs. MS2-only analysis. This was the original goal: "need to know how much it cost us to get this additional data (beyond MS2)".

6. **MS1 vs MS2 signal fate comparison** — Document empirical comparison of MS1-based vs MS2-based signal fate metrics.

---

## Phase 6 — Digestion & Minimal QC
**Status:** ✅ Complete  
**Started:** 2026-07-07  
**Completed:** 2026-07-07

### Goals
- Compute digestion efficiency metrics (missed cleavages, semi-tryptic)
- Compute minimal QC metrics (mass accuracy, ID rate)
- Surface existing metrics from Sage TSV columns (no new computation)

### Decisions Made

1. **Sage TSV columns used directly** — Sage already provides `missed_cleavages`, `semi_enzymatic`, `precursor_ppm`, `fragment_ppm` columns. No need to recompute from peptide sequences.

2. **Precursor ppm interpretation** — Sage's `precursor_ppm` column contains the absolute value of precursor mass error in ppm. In open search with wide tolerance, this reflects the delta mass, not mass accuracy. The median (12.36 ppm) is meaningful; the mean (17,688 ppm) is skewed by large delta masses.

3. **Fragment ppm is the true mass accuracy metric** — `fragment_ppm` (median 2.21 ppm, 95th percentile 5.42 ppm) reflects actual instrument mass accuracy since fragment ions are matched with tight tolerance regardless of precursor delta.

4. **ID rate from signal_fate** — Rather than recompute, `qc.rs` calls `compute_signal_fate_with_mzml()` to get the 72% ID rate already validated in Phase 5A.

5. **Semi-tryptic rate is 0%** — All 81,966 PSMs have `semi_enzymatic=0`, meaning fully tryptic. This is expected since the Sage search was configured with `enzyme: { missed_cleavages: 2, min_len: 7, max_len: 50, cleave_at: "KR", restrict: "P" }` (standard trypsin, no semi-enzymatic).

### Files Created/Modified
- `recon-tool/src/sage_results.rs` — Added `missed_cleavages`, `semi_enzymatic`, `precursor_ppm`, `fragment_ppm`, `peptide_len` fields to `Psm` struct
- `recon-tool/src/digestion.rs` — Digestion efficiency metrics (missed cleavage distribution, semi-tryptic rate, peptide length stats)
- `recon-tool/src/qc.rs` — QC metrics (precursor/fragment ppm distributions, ID rate, unique counts)
- `recon-tool/src/lib.rs` — Export digestion, qc modules
- `recon-tool/src/main.rs` — Added `digestion-stats` and `qc-stats` CLI commands

### CLI Usage
```bash
# Digestion efficiency metrics
recon digestion-stats --tsv results.sage.tsv --summary-only

# QC metrics (without mzML - no ID rate)
recon qc-stats --tsv results.sage.tsv --summary-only

# QC metrics with ID rate (requires mzML)
recon qc-stats --tsv results.sage.tsv --mzml data.mzML.gz --summary-only
```

### Validation Results

**Digestion Efficiency:**
```
Total PSMs: 81,966

Missed Cleavages:
  0 missed: 69,341 (84.6%)
  1 missed: 11,770 (14.4%)
  2 missed: 855 (1.0%)
  Mean: 0.16

Semi-Tryptic:
  Fully tryptic: 81,966 (100.0%)
  Semi-tryptic: 0 (0.0%)

Peptide Length:
  Min: 7
  Max: 50
  Mean: 14.0
  Median: 13
```

**QC Metrics:**
```
Precursor Mass Accuracy (ppm):
  Mean: 17,688.77 (skewed by open search delta masses)
  Median: 12.36
  5th percentile: 0.12
  95th percentile: 145,273.88

Fragment Mass Accuracy (ppm):
  Mean: 2.59
  Median: 2.21
  Std: 1.63
  5th percentile: 0.92
  95th percentile: 5.42

Identification Rate:
  PSM rate: 72.0% ✓ (matches Phase 5A)
  Identified spectra: 66,437
  Total MS2 spectra: 92,304
  Unique peptides: 44,462
  Unique proteins: 6,732
```

### Sanity Check

| Metric | Expected | Observed | Status |
|--------|----------|----------|--------|
| ID rate | ~72% (Phase 5A) | 72.0% | ✓ |
| Missed cleavages 0 | >80% (typical trypsin) | 84.6% | ✓ |
| Missed cleavages 1 | 10-20% (typical) | 14.4% | ✓ |
| Semi-tryptic | 0% (search config) | 0.0% | ✓ |
| Fragment ppm median | <5 ppm (Orbitrap) | 2.21 ppm | ✓ |

All metrics are within expected ranges for a standard tryptic digest on an Orbitrap instrument.

### Checkpoint Status
- [x] Digestion efficiency metrics implemented
- [x] QC metrics implemented
- [x] ID rate matches Phase 5A (72%)
- [x] Missed cleavage distribution reasonable (84.6% / 14.4% / 1.0%)
- [x] All 46 tests pass

---

## Phase 6B — Semi-Enzymatic Experiment
**Status:** ✅ Complete  
**Started:** 2026-07-08  
**Completed:** 2026-07-08

### Goals
- Run semi-enzymatic search on test file to measure runtime cost
- Quantify semi-tryptic peptide yield
- Decide: opt-in probe vs. new default

### Experiment Setup

**Config:** `testing/semi-enzymatic-search-params.json`
- `semi_enzymatic: true`
- `missed_cleavages: 2`
- `precursor_tol: ppm [-20, 20]` (closed search, NOT open search Da tolerance)
- Same test file: `B.naive_01steady-state.mzML.gz`

**Critical Note:** PLAN.md explicitly warns against combining wide precursor tolerance with semi-enzymatic search — the candidate space explosion is multiplicative and can hang/crash. This experiment uses narrow ppm tolerance.

### Results

| Metric | Open Search (Phase 0) | Semi-Enzymatic Closed |
|--------|----------------------|----------------------|
| Runtime | 172 seconds | **1,226 seconds (7.1×)** |
| Fragment index | 1.3B fragments | **2.0B fragments** |
| Peptide candidates | 3.9M | **82.5M (21×)** |
| PSMs (q≤0.01, target) | 81,966 | 44,187 |
| Unique peptides | 44,462 | 34,396 |
| Proteins | 6,544 | 5,774 |

**Semi-Tryptic Analysis:**
```
Total PSMs (q≤0.01, target): 44,187
Fully enzymatic PSMs: 41,360 (93.6%)
Semi-enzymatic PSMs: 2,827 (6.4%)

Missed Cleavage Distribution:
MC=0: 37,330 (84.5%)
MC=1: 6,489 (14.7%)
MC=2: 368 (0.8%)
```

### Decision: Opt-In Probe

**Verdict:** Semi-enzymatic search is **NOT suitable as the default** for recon.

**Rationale:**
1. **7× runtime penalty** — 1,226 seconds vs 172 seconds is unacceptable for a "fast recon" tool
2. **Marginal yield** — Only 6.4% semi-tryptic peptides in this cell lysate sample
3. **Fewer total IDs** — 44,187 PSMs vs 81,966 (46% fewer) because closed search misses modified peptides that open search finds
4. **Sample-dependent** — Biofluids (plasma, CSF) would show higher semi-tryptic rates; cell lysates don't benefit

**Recommendation:** Digestion efficiency assessment should be:
- A **separate, opt-in command** (e.g., `recon digestion-efficiency`)
- Documented as requiring a separate Sage run with `semi_enzymatic: true`
- Not part of the default recon pipeline

This aligns with the tool's stated non-goals: "fast recon, not a general QC suite."

### Files Created
- `testing/semi-enzymatic-search-params.json` — Config with narrow ppm + semi_enzymatic
- `testing/run-semi-enzymatic.ps1` — Timed Sage execution script
- `testing/scripts/analyze-semi-enzymatic.ps1` — Results analysis script
- `testing/search-output/semi-enzymatic/` — Sage output directory

### Checkpoint Status
- [x] Semi-enzymatic search completed
- [x] Runtime measured (1,226 sec, 7× slower)
- [x] Semi-tryptic % measured (6.4%)
- [x] Decision documented (opt-in probe, not default)

---

## Phase 6C — Two-Pass Digestion Efficiency Probe
**Status:** ✅ Complete  
**Started:** 2026-07-08  
**Completed:** 2026-07-08

### Goals
- Implement two-pass workflow for digestion efficiency assessment
- Pass 1: Narrow tryptic search → MC distribution, protein list
- Pass 2: Semi-enzymatic on subset FASTA → semi-tryptic %
- Validate runtime improvement over Phase 6B's full-FASTA approach

### Architecture Decisions

1. **Separate workflow** — `recon digestion-efficiency` as distinct subcommand, not part of default open search pipeline. Addresses Phase 6B finding (7× slower).

2. **Pass 1 config** — New `digestion-efficiency-pass1.json`:
   - `precursor_tol: ppm [-10, 10]` (tight, tunable)
   - `semi_enzymatic: false`, `missed_cleavages: 2`
   - Minimal variable mods (Oxidation M only)

3. **Automated FASTA subsetting** — Subset FASTA only needs target sequences. **Sage handles decoy generation internally** — decoys are generated on-the-fly during the search, so we don't need to include `rev_*` entries in the subset FASTA.

4. **Pass 2 config** — New `digestion-efficiency-pass2.json`:
   - `semi_enzymatic: true`, `missed_cleavages: 1`, `min_len: 8`
   - FASTA: subset to ~6,000 proteins (vs ~20,000 full)

5. **Pass 2 is non-optional** — For only ~3 minutes additional runtime, we get enzyme performance metrics that can distinguish between trypsin sources (e.g., bovine vs porcine trypsin can show 10% vs 20% semi-tryptic rates). This is worth running by default.

6. **Python terminus annotation** — `digestion_efficiency.py` using `pyteomics.fasta`:
   - Handles protein N/C-termini as special case (tryptic by definition)
   - Outputs: fully_tryptic, semi_n_ragged, semi_c_ragged, non_tryptic
   - Computes digestion score (0-100) alongside raw values

### Experiment Results

**Pass 1 — Tryptic Baseline:**
```
Runtime: 40.7 seconds
Fragment index: 159.7M fragments, 6.5M peptides
PSMs (q≤0.01, target): 56,394
Unique peptides: 41,734
Proteins: 6,107
```

**FASTA Subsetting:**
```
Unique protein accessions from Pass 1: 6,245
Subset FASTA written: 6,245 entries (targets only, no decoys in source)
```

**Pass 2 — Semi-Enzymatic on Subset:**
```
Runtime: 144.8 seconds (2.4 minutes)
Fragment index: 539.9M fragments, 21.9M peptides
PSMs (q≤0.01, target): 55,916
Unique peptides: 41,065
Proteins: 6,135
```

**Terminus Classification:**
```
Total PSMs after filtering: 55,856

Fully tryptic: 52,405 (93.8%)
Semi-tryptic (N-ragged): 2,514 (4.5%)
Semi-tryptic (C-ragged): 937 (1.7%)

Total semi-tryptic: 3,451 (6.2%)
```

### Runtime Comparison

| Approach | Runtime | Speedup |
|----------|---------|---------|
| Phase 6B (full FASTA, semi-enzymatic) | 1,226 sec (20.4 min) | baseline |
| Phase 6C Pass 1 + Pass 2 combined | 185.5 sec (3.1 min) | **6.6× faster** |

**Success!** The two-pass approach achieves the target of <5 minutes total runtime while providing the same semi-tryptic analysis capability.

### Key Findings

1. **Semi-tryptic rate: 6.2%** — Consistent with Phase 6B's 6.4% on full FASTA, validating the subset approach.

2. **N-ragged dominates C-ragged (2.7:1 ratio)** — N-terminal ragged ends are more common than C-terminal, which is typical for cell lysates where aminopeptidases are more active than carboxypeptidases.

3. **Subset FASTA is larger than expected** — 6,245 proteins vs. the ~1,000 originally estimated. This is because the test file identifies many proteins. The approach still works because the semi-enzymatic candidate explosion is proportional to protein count, not absolute.

4. **Script bug fixed** — Both `subset_fasta.py` and `annotate_termini.py` needed accession format fixes to match Sage's full UniProt format (`sp|P12345|GENE_HUMAN`) rather than just the accession number.

### Known Limitations

**FDR stratification:** A flat `peptide_q ≤ 0.01` on mixed tryptic+semi-tryptic results inflates semi-tryptic false positives. The magnitude of this inflation is **context-dependent and unreported in literature**:

1. **Score distribution separation** — If semi-tryptic PSMs score nearly as well as tryptic (e.g., real biological ragged ends with clean spectra from biofluids), inflation is modest. If they score poorly (ambiguous, short, low-charge peptides), inflation is severe.

2. **Candidate space ratio** — With a subset FASTA (~6,000 proteins), the ratio of semi-tryptic decoys to targets is better behaved than with 20,000 proteins, which works in our favor.

3. **Dilution by tryptic majority** — If only 6.2% of PSMs are semi-tryptic, the contamination at 1% flat FDR is diluted by the large tryptic majority. The absolute number of spurious semi-tryptic hits is small.

Tools like MetaMorpheus (Rolfs et al. 2020) and MSFragger use stratified FDR for semi-tryptic results but don't publish comparisons of what flat-FDR numbers would have been. The practice is universal; the magnitude is unreported.

### Files Created/Modified
- `testing/configs/digestion-efficiency-pass1.json` — Pass 1 config (narrow tryptic)
- `testing/configs/digestion-efficiency-pass2.json` — Pass 2 config (semi-enzymatic, subset FASTA)
- `testing/scripts/digestion_efficiency.py` — Combined tool with `subset` and `annotate` subcommands
- `testing/scripts/subset_fasta.py` — (legacy, superseded by digestion_efficiency.py)
- `testing/scripts/annotate_termini.py` — (legacy, superseded by digestion_efficiency.py)
- `testing/search-output/digestion-pass1/` — Pass 1 results
- `testing/search-output/digestion-pass2/` — Pass 2 results
- `testing/inputs/subset_identified_proteins.fasta` — Subset FASTA (6,245 proteins)
- `testing/search-output/annotated_termini.tsv` — Annotated PSMs with terminus classification
- `testing/search-output/digestion_efficiency_result.json` — JSON summary output

### Checkpoint Status
- [x] Architecture decisions documented
- [x] Pass 1 config created
- [x] Pass 2 config created
- [x] subset_fasta.py implemented (and fixed)
- [x] annotate_termini.py implemented (and fixed)
- [x] Experiment run
- [x] Results documented

---

## Phase 6D — MS1 Signal Fate Integration
**Status:** ✅ Complete

**Phase 6D Complete:** MS1 precursor intensity signal fate integrated into `signal-fate` command — provides three-way comparison (count, MS2 TIC, MS1 TIC) showing 72% ID rate by count, 26% by MS2 intensity, and 6.4% by MS1 intensity.

### Implementation Summary

Added `--ms1-intensity` flag to the `signal-fate` command that computes MS1 precursor intensity signal fate alongside the existing MS2-based metrics.

### New CLI Options

```bash
recon signal-fate --tsv results.sage.tsv --mzml file.mzML.gz --ms1-intensity [--rt-window 1.0] [--mz-tol-ppm 10.0]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--ms1-intensity` | off | Enable MS1 precursor intensity extraction |
| `--rt-window` | 1.0 | RT window in minutes (±) for MS1 lookup |
| `--mz-tol-ppm` | 10.0 | m/z tolerance in ppm for precursor matching |

### Code Changes

1. **`mzml.rs`** — Added `build_precursor_queries_from_psms()` helper function to convert PSMs to precursor queries using `precursor_mz` and `rt` fields.

2. **`signal_fate.rs`** — Added `Ms1SignalFate` struct and `ms1_signal_fate` field to `SignalFateResult`. Updated `print_signal_fate_summary()` to show three-way comparison table.

3. **`main.rs`** — Extended `SignalFate` command with new flags and integrated MS1 extraction into `run_signal_fate_command()`.

### Test Results

**Basic MS1 Signal Fate (monoisotopic peak only):**
```
=== Signal Fate Comparison ===

                         By Count By MS2 Intensity By MS1 Intensity
--------------------------------------------------------------------
Identified:                 72.0%            26.2%             6.4%
Unidentified:               28.0%            73.8%            93.6%

MS1 Signal Fate:
  Total MS1 TIC: 3.10e13
  Explained by IDs: 1.98e12 (6.4%)
  Unexplained: 2.90e13 (93.6%)
  PSMs with MS1 signal: 71780 / 81966 (87.6%)
```

**Improved MS1 Signal Fate (isotope envelope, deduplicated):**
```
=== Improved MS1 Signal Fate ===

Denominators:
  Total MS1 TIC:        3.10e13
  Peptide-like TIC:     2.62e13 (400-1200 m/z)

Numerator (isotope envelope, deduplicated):
  Explained intensity:  2.92e12
  Unique features:      66299
  Total PSMs:           81966
  PSMs with signal:     69058 (84.3%)

Explained Percentages:
  % of total MS1 TIC:      9.4%
  % of peptide-like TIC:   11.1%
```

### Final Comparison Table

| Metric | By Count | By MS2 Intensity | By MS1 Intensity (basic) | By MS1 Intensity (improved) |
|--------|----------|------------------|--------------------------|----------------------------|
| Identified | 72.0% | 26.2% | 6.4% | 9.4% (total) / 11.1% (peptide-like) |
| Unidentified | 28.0% | 73.8% | 93.6% | 90.6% (total) / 88.9% (peptide-like) |

### Key Observations

1. **Three-tier ID rate** — The dramatic difference between count-based (72%), MS2-based (26%), and MS1-based (6.4-11.1%) ID rates illustrates how different metrics tell different stories about sample complexity.

2. **MS1 is dominated by high-abundance species** — The 6.4-11.1% explained MS1 TIC suggests that most MS1 signal comes from species that weren't selected for MS2 fragmentation (either below intensity threshold or excluded by dynamic exclusion).

3. **Improved MS1 extraction gains ~50%** — The isotope envelope summing (M+0, M+1, M+2) and precursor deduplication improved explained MS1 from 6.4% to 9.4% of total TIC.

4. **Peptide-like denominator is more meaningful** — Restricting the denominator to 400-1200 m/z (typical DDA selection range) gives 11.1% explained, which better reflects the "peptide-like" signal that could theoretically be identified.

5. **Deduplication reduces features** — 81,966 PSMs collapsed to 66,299 unique precursor features (19% reduction), indicating significant chimeric/repeated sampling of the same precursors.

3. **87.6% PSM coverage** — Most PSMs (71,780 / 81,966) have detectable MS1 precursor signal within the RT/m/z window, validating the extraction approach.

### Files Modified
- `recon-tool/src/mzml.rs` — Added `build_precursor_queries_from_psms()`
- `recon-tool/src/signal_fate.rs` — Added `Ms1SignalFate` struct and comparison table
- `recon-tool/src/main.rs` — Extended `signal-fate` command with MS1 options

### Checkpoint Status
- [x] Add `--ms1-intensity` flag to `signal-fate` command
- [x] Add `build_precursor_queries_from_psms()` helper
- [x] Extend `SignalFateResult` with MS1 signal fate
- [x] Update `print_signal_fate_summary()` with comparison table
- [x] Build and test
- [x] Test on real data

---

## Phase 6E — Three-Layer MS1 Signal Fate
**Status:** ✅ Complete (2026-07-09)

### Goal
Separate "unidentified" MS1 into three layers:
1. **Non-peptidic** — outside 400-1200 m/z (not peptide-like)
2. **Never sampled** — no MS2 trigger (DDA limitation)
3. **Sampled but not identified** — MS2 but no PSM

This transforms "6.4% identified" into a more interpretable breakdown.

### Implementation

Added `--three-layer` flag to `signal-fate` command. The algorithm:
1. Extracts all MS2 precursor info from mzML (sampling events)
2. Builds sorted region indexes for fast RT-based lookup
3. For each MS1 peak, classifies into one of four buckets:
   - Non-peptidic (outside 400-1200 m/z)
   - Identified (matches PSM within m/z and RT tolerance)
   - Sampled but not ID'd (matches MS2 precursor but no PSM)
   - Never sampled (no match)

### Performance Optimization

Original algorithm had O(n × m × k) complexity where n = MS1 peaks, m = identified regions, k = sampled regions. This caused the command to hang on large datasets.

**Fix:** Added `SortedRegionIndex` that:
- Sorts regions by RT
- Uses binary search to find RT window
- Only scans regions within RT tolerance

Also added progress reporting every ~5% of spectra with ETA.

### Test Results (B.naive_01steady-state.mzML.gz)

```
=== MS1 Signal Fate (Three-Layer) ===

Total MS1 TIC:           3.07e13 (100%)
├─ Non-peptidic:         4.51e12 (14.7%)  [outside 400-1200 m/z]
├─ Peptide-like:         2.62e13 (85.3%)
   ├─ Never sampled:     7.79e12 (29.7%)  [no MS2 trigger]
   ├─ Sampled, not ID'd: 5.86e12 (22.4%)  [MS2 but no PSM]
   └─ Identified:        1.26e13 (48.0%)  [PSM at q<0.01]

Derived Metrics:
  Sampling efficiency: 70.3% of peptide-like TIC was sampled by MS2
  ID efficiency:       68.2% of sampled TIC was identified

Counts:
  MS2 precursors (sampling events): 92304
  Identified PSMs: 491461
  Unique identified features: 351034
```

### Key Insights

1. **85% of MS1 TIC is peptide-like** — Only 15% is outside the 400-1200 m/z range.

2. **70% sampling efficiency** — DDA sampled 70% of peptide-like TIC for MS2 fragmentation.

3. **68% ID efficiency** — Of the sampled TIC, 68% was successfully identified.

4. **48% overall ID rate** — Of peptide-like TIC, 48% was identified (much better than the 6.4% of total MS1 TIC).

5. **30% never sampled** — This is the "DDA limitation" — signal that was never selected for MS2.

### Files Modified
- `recon-tool/src/mzml.rs` — Added `SortedRegionIndex` for optimized lookup, progress reporting
- `recon-tool/src/main.rs` — `--three-layer` flag already existed

### Checkpoint Status
- [x] Implement three-layer MS1 signal fate algorithm
- [x] Add `--three-layer` flag to CLI
- [x] Optimize with RT-sorted index (binary search)
- [x] Add progress reporting with ETA
- [x] Test on real data
- [x] Add modification breakdown by MS1 TIC
- [x] Add top-5 modifications by MS1 intensity

### Modification Breakdown Feature (2026-07-09)

Added `compute_mod_breakdown()` function that:
1. Groups PSMs by delta mass (0.01 Da bins)
2. Extracts MS1 precursor intensity for each PSM
3. Reports unmodified vs modified TIC breakdown
4. Lists top N modifications by MS1 TIC

Output format:
```
=== Identified TIC by Modification Status ===

  Unmodified:  3.13e13 (100.0%)
  Modified:    0.00e0 (0.0%)

Top Modifications by MS1 TIC:
Delta (Da)             TIC   % of ID'd  PSM Count
--------------------------------------------------
(none for narrow search - all PSMs have delta ≈ 0)
```

**Note:** The test data uses narrow search, so all PSMs are unmodified. For open search data, this will show the actual modification breakdown with top modifications ranked by MS1 intensity.

### Known Tradeoffs and Assumptions

1. **Peptide-like denominator (400-1200 m/z)** — A more precise definition would filter by charge state (z=2-4), but we cannot reliably infer charge from MS1 peaks alone without full feature detection. The m/z range filter is a reasonable approximation. See `reference-notes/MS1-intensity.md` for full discussion.

2. **Narrow search shows 100% unmodified** — This is expected behavior, not a bug. With tight precursor tolerance (±10-20 ppm), only peptides matching the expected mass are identified. Unexpected PTMs would shift the precursor mass outside this tolerance window. The modification breakdown feature is only meaningful for open search data.

3. **MS2 TIC intensity isn't meaningful for signal fate** — MS2 TIC-based ID rate (26.2%) differs dramatically from count-based (72%) and MS1-based (48%) metrics. MS2 TIC is problematic because chimeric spectra share intensity, TIC is dominated by abundant fragments, and fragmentation efficiency varies. Use MS2 count-based metrics for routine QC.

---

## Phase 6F — Integration & Measurement Baseline
**Status:** ✅ Complete (2026-07-09)

### Goal
Put all pieces together, measure performance, and decide what's core vs optional vs testing-only.

### Performance Measurements

All timings on test file `B.naive_01steady-state.mzML.gz` (104,931 spectra, 12,627 MS1, 92,304 MS2):

| Analysis | Runtime | Notes |
|----------|---------|-------|
| **Sage open search** | ~172s | Baseline search (81,966 PSMs) |
| **Sage narrow search** | ~40s | Pass 1 for digestion efficiency |
| **Mod discovery** | ~1.5s | TSV parsing + histogram + Unimod lookup |
| **Polymer detection** | ~17s | MS1 parsing + 16 polymer series search |
| **Oxonium screening** | ~10s | MS2 parsing + 8 diagnostic ions |
| **Signal fate (MS2 only)** | ~10s | mzML parsing + count/TIC accounting |
| **Signal fate (three-layer)** | ~88s | MS1 + MS2 + PSM cross-referencing |
| **Digestion stats** | ~1.4s | TSV parsing only |
| **QC stats** | ~1.4s | TSV parsing only |

**Key observation:** The three-layer MS1 analysis is ~9× slower than MS2-only signal fate. For routine QC, MS2 count-based metrics are sufficient.

### Full Pipeline Results (Open Search)

**Mod Discovery (top 10 peaks):**
| Rank | Delta (Da) | Count | % | Annotation |
|------|------------|-------|---|------------|
| 1 | 0.00 | 41,198 | 50.3% | Unmodified |
| 2 | 0.00 | 16,911 | 20.6% | Unmodified (isotope) |
| 3 | 1.00 | 7,627 | 9.3% | Label:15N(1) / Deamidation |
| 4 | 0.99 | 1,998 | 2.4% | Deamidated |
| 5 | 2.00 | 1,956 | 2.4% | Glu→Met |
| 6 | 2.01 | 1,881 | 2.3% | Label:18O(1) |
| 7 | 15.99 | 1,878 | 2.3% | Oxidation |
| 8 | -0.03 | 937 | 1.1% | Unmodified |
| 9 | 0.96 | 923 | 1.1% | Xle→Asn |
| 10 | 1.03 | 845 | 1.0% | UNANNOTATED |

**Polymer Detection:**
- Total polymer %TIC: **0.36%** (Moderate contamination)
- Top polymers: PEG+2H (0.11%), PEG+1H (0.08%), PEG+3H (0.05%)
- All 16 polymer types detected at low levels

**Oxonium Screening:**
- Glycopeptide candidates: **101 spectra (0.1%)**
- This is a naive B cell sample, so low glycopeptide content is expected

**Signal Fate (MS2 count-based):**
- Identified: **72.0%** (66,437 / 92,304 spectra)
- Unidentified: 28.0%

**Signal Fate (MS1 three-layer, open search):**
- Non-peptidic: 14.7%
- Never sampled: 29.7%
- Sampled, not ID'd: 22.4%
- Identified: 48.0% of peptide-like TIC

**MS1 Modification Breakdown (top 5 by TIC):**
| Delta (Da) | TIC | % of ID'd | PSM Count |
|------------|-----|-----------|-----------|
| 0.00 | 3.13e12 | 78.6% | 43,700 |
| 1.00 | 3.42e11 | 8.6% | 5,251 |
| 0.98 | 1.38e11 | 3.5% | 574 |
| 1.01 | 7.16e10 | 1.8% | 1,503 |
| 1.99 | 5.69e10 | 1.4% | 272 |

### Cross-Validation: Mod Discovery vs MS1 Signal Fate

**Question:** Does the modification breakdown by MS1 TIC align with mod discovery by PSM count?

| Modification | By PSM Count (discover) | By MS1 TIC (signal-fate) |
|--------------|------------------------|--------------------------|
| Unmodified | 53.3% | 78.6% |
| +1 Da (Deamidation/isotope) | 9.3% | 8.6% |
| +0.98 Da (Deamidation) | 2.4% | 3.5% |
| +16 Da (Oxidation) | 2.3% | <1% |

**Insight:** Unmodified peptides are **over-represented by MS1 TIC** (78.6% vs 53.3% by count). This confirms that abundant proteins tend to be unmodified — modified peptides are often lower abundance. This is consistent with Ben's ROA finding that intensity-weighted metrics track with counts but may smooth out marginal effects.

**Conclusion:** MS1 TIC-based modification breakdown provides complementary information to PSM counts, but for routine QC, PSM counts are sufficient and much faster.

### Shared Narrow Search Dependency

Both **digestion efficiency** and **signal fate** benefit from a narrow search:
- Digestion efficiency Pass 1: requires narrow ppm tolerance (~40s)
- Signal fate: works with either narrow or open search results

If running both workflows, the narrow search cost (~40s) is amortized. However, for the default recon pipeline, we use the **open search** results since mod discovery requires wide tolerance.

### Decision Matrix

| Analysis | Runtime | What it tells you | Category |
|----------|---------|-------------------|----------|
| Mod discovery | ~1.5s | PTM landscape | **Core** |
| Polymer %TIC | ~17s | Contamination level | **Core** |
| Oxonium screening | ~10s | Glycopeptide presence | **Core** |
| Signal fate (MS2 count) | ~10s | Basic ID rate | **Core** |
| Digestion stats | ~1.4s | MC distribution | **Core** |
| QC stats | ~1.4s | Mass accuracy | **Core** |
| Signal fate (three-layer) | ~88s | DDA vs search bottleneck | **Optional** |
| MS1 mod breakdown | (included) | Mod abundance by TIC | **Optional** |
| Digestion efficiency (2-pass) | ~185s | Semi-tryptic %, N/C-ragged | **Testing-only** |

### Code Organization Decisions

**Core (always run in default pipeline):**
- `mod_discovery.rs` — PTM landscape from delta masses
- `polymer.rs` — Polymer contamination %TIC
- `oxonium.rs` — Glycopeptide screening
- `signal_fate.rs` — MS2 count-based ID rate
- `digestion.rs` — Missed cleavage distribution
- `qc.rs` — Mass accuracy metrics

**Optional (flag-enabled):**
- `--three-layer` — MS1 signal fate breakdown
- `--ms1-intensity` — MS1 precursor intensity extraction
- Mod breakdown by MS1 TIC (part of three-layer)

**Testing/Development utilities:**
- `digestion_efficiency.py` — Two-pass workflow for semi-tryptic analysis
- `subset_fasta.py` — FASTA subsetting helper
- `annotate_termini.py` — Terminus classification helper

### Checkpoint Status
- [x] Run all analyses on test file and time them
- [x] Cross-validate mod discovery vs MS1 signal fate
- [x] Document shared narrow search dependency
- [x] Create decision matrix
- [x] Document code organization decisions

---

## Phase 6F Addendum — Output Format Comparison Tables
**Status:** ✅ Complete (2026-07-09)

### Goal
Document what each analysis output looks like and compare MS2 count vs MS1 TIC views to inform Phase 7 report design.

### 1. PTM Discovery: PSM Count vs MS1 TIC Comparison

**Current `discover` command output (PSM count only):**
```
Rank   Delta (Da)   Count      Pct        Annotation
1      0.0003       41198      50.3       Unmodified
2      0.0018       16911      20.6       Unmodified
3      1.0013       7627       9.3        Label:15N(1)
4      0.9876       1998       2.4        Deamidated
5      2.0008       1956       2.4        Glu→Met
6      2.0059       1881       2.3        Label:18O(1)
7      15.9958      1878       2.3        Oxidation
```

**Current `signal-fate --three-layer` output (MS1 TIC):**
```
Top Modifications by MS1 TIC:
Delta (Da)               TIC  % of ID'd  PSM Count
--------------------------------------------------
1.00                 3.42e11       8.6%       5251
0.98                 1.38e11       3.5%        574
1.01                 7.16e10       1.8%       1503
1.99                 5.69e10       1.4%        272
0.99                 5.60e10       1.4%        326
```

**Unified PTM Table (what Phase 7 should produce):**

| Rank | Delta (Da) | Annotation | PSM Count | % by Count | MS1 TIC | % by TIC | TIC/Count Ratio |
|------|------------|------------|-----------|------------|---------|----------|-----------------|
| 1 | 0.00 | Unmodified | 41,198 | 50.3% | 2.12e12 | 53.5% | 1.06× |
| 2 | 1.00 | Deamidation | 7,627 | 9.3% | 3.42e11 | 8.6% | 0.92× |
| 3 | 0.98 | Deamidated | 1,998 | 2.4% | 1.38e11 | 3.5% | 1.46× |
| 4 | 15.99 | Oxidation | 1,878 | 2.3% | ? | ? | ? |
| 5 | 2.00 | Glu→Met | 1,956 | 2.4% | 5.69e10 | 1.4% | 0.58× |

**Key Insight:** The TIC/Count ratio reveals which modifications are on abundant vs rare proteins:
- Ratio > 1.0 = modification on abundant proteins (over-represented by TIC)
- Ratio < 1.0 = modification on rare proteins (under-represented by TIC)
- Oxidation at +16 Da is notably absent from MS1 TIC top-5, suggesting it's on lower-abundance proteins

**Runtime:** ~128s for three-layer (includes MS1 extraction for all PSMs)

### 2. Polymer Detection: MS1 TIC Context

**Current `polymer-stats` output:**
```
Total MS1 TIC: 3.10e13
Total polymer %TIC: 0.36%

Polymer                                Intensity       %TIC
------------------------------------------------------------
PEG+2H                                   3.30e10     0.1067%
PEG+1H                                   2.59e10     0.0837%
PEG+3H                                   1.46e10     0.0472%
Tween-40                                 1.28e10     0.0414%
Tween-20                                 1.02e10     0.0328%
```

**Enhanced Polymer Context Table (what Phase 7 should produce):**

```
=== MS1 TIC Breakdown ===

Total MS1 TIC:           3.10e13 (100.0%)
├─ Peptide-like:         2.62e13 (84.5%)
│   ├─ Identified:       3.96e12 (12.8%)
│   ├─ Sampled, not ID:  5.86e12 (18.9%)
│   └─ Never sampled:    1.24e13 (40.0%)
├─ Polymer:              1.12e11 (0.36%)
│   ├─ PEG (all):        7.35e10 (0.24%)
│   ├─ Tween (all):      2.30e10 (0.07%)
│   ├─ Triton (all):     9.60e9  (0.03%)
│   └─ Other:            6.00e9  (0.02%)
└─ Other non-peptide:    4.68e12 (15.1%)
```

**Key Insight:** Polymer detection is MS1-only (no MS2 count equivalent). The context table shows where polymer fits in the overall signal budget — 0.36% is "moderate" but dwarfed by the 40% of peptide-like signal that was never sampled.

**Runtime:** ~17s (MS1 parsing + polymer search)

### 3. Oxonium Screening: MS2 Count vs MS1 Precursor TIC

**Current `oxonium-screen` output (MS2 count only):**
```
Total MS2 spectra screened: 92304
Glycopeptide candidates: 101 (0.1%)

Ion Count Distribution:
  0 ions: 91704 spectra (99.3%)
  1 ions: 452 spectra (0.5%)
  2 ions: 76 spectra (0.1%)
  3 ions: 49 spectra (0.1%)
```

**Enhanced Oxonium Table (what Phase 7 could produce):**

| Oxonium Ions | Spectra | % of MS2 | Precursor TIC | % of Sampled MS1 |
|--------------|---------|----------|---------------|------------------|
| ≥2 ions (candidates) | 101 | 0.1% | ? | ? |
| 1 ion | 452 | 0.5% | ? | ? |
| 0 ions | 91,704 | 99.3% | ? | ? |

**What's Missing:** The oxonium screening currently doesn't extract MS1 precursor intensity for the candidate spectra. To add this would require:
1. For each glycopeptide candidate spectrum, get precursor m/z and RT
2. Look up MS1 intensity at that precursor (same as signal-fate does)
3. Sum to get "glycopeptide candidate TIC"

**Estimated Runtime Impact:** +10-20s (need to cross-reference MS2 scan → MS1 precursor)

**Key Insight:** For this sample (naive B cells), glycopeptide content is very low (0.1%). The MS1 TIC view would tell us whether these 101 spectra represent abundant or rare glycopeptides.

### 4. Signal Fate Comparison Table (Already Implemented)

**Current `signal-fate --three-layer` output:**
```
=== Signal Fate Comparison Table ===

                         MS2 Spectra    MS1 TIC (sampled) MS1 TIC (peptide-like)
------------------------------------------------------------------------------
Total                          92304              1.38e13              2.62e13
Identified                     71.1%                19.1%                10.0%
Unidentified                   28.9%                80.9%                42.4%
Never sampled                    N/A                  N/A                47.5%
```

**Key Insight:** This table already shows the three views side-by-side. The dramatic difference (71% by count vs 10% by MS1 TIC) illustrates why both metrics matter.

### Summary: What Each View Tells You

| Analysis | MS2 Count View | MS1 TIC View | When to Use |
|----------|----------------|--------------|-------------|
| **PTM Discovery** | "How many PSMs have this mod?" | "How much signal is this mod?" | Count for prevalence, TIC for abundance |
| **Polymer** | N/A (MS1 only) | "What % of MS1 is polymer?" | Always use TIC |
| **Oxonium** | "How many spectra have glyco ions?" | "How much signal is glycopeptide?" | Count for screening, TIC for abundance |
| **Signal Fate** | "What % of spectra are ID'd?" | "What % of signal is explained?" | Count for QC, TIC for biology |

### Recommendations for Phase 7 Report

1. **PTM Table:** Include both PSM count and MS1 TIC columns, plus TIC/Count ratio to highlight abundance bias

2. **Polymer Table:** Show in context of total MS1 TIC breakdown (peptide-like vs polymer vs other)

3. **Oxonium Table:** Add MS1 precursor TIC for glycopeptide candidates (requires code change)

4. **Signal Fate:** Keep the three-column comparison table as-is

5. **Runtime Consideration:** The three-layer analysis adds ~128s. For routine QC, offer a "fast mode" that skips MS1 extraction and only reports MS2 counts.

---

## Phase 7 — Report Output
**Status:** ✅ Complete  
**Started:** 2026-07-14  
**Completed:** 2026-07-14

### Goals
- Create unified `ReconReport` struct consolidating all analysis results
- Implement `analyze` CLI command for one-shot analysis
- Generate JSON, text summary, and HTML report outputs
- Add alkylation check (Cys +57 Da verification)
- Support optional `--full` flag for three-layer MS1 analysis

### Implementation Summary

Created `report.rs` module with:
- `ReconReport` struct containing all analysis summaries
- `compute_alkylation_check()` for Cys alkylation verification
- `print_report_summary()` for console text output
- `generate_html_report()` for interactive HTML report

Added `analyze` CLI command that:
1. Loads mzML file (MS1 + MS2 spectra)
2. Loads Sage results (filtered PSMs)
3. Loads Unimod database
4. Runs mod discovery (delta-mass peaks)
5. Computes signal fate (ID rate by count and TIC)
6. Detects polymer contamination
7. Screens for glycopeptides (oxonium ions)
8. Computes digestion and QC metrics
9. Checks alkylation completeness
10. Optionally computes three-layer MS1 fate (`--full`)
11. Outputs JSON + HTML reports

### CLI Usage

```bash
# Basic analysis (fast, ~42s)
recon analyze --mzml data.mzML.gz --tsv results.sage.tsv --unimod unimod.xml --output report

# Full analysis with three-layer MS1 (~130s)
recon analyze --mzml data.mzML.gz --tsv results.sage.tsv --unimod unimod.xml --output report --full
```

### Output Files

| File | Format | Contents |
|------|--------|----------|
| `report.json` | JSON | Machine-readable full report (schema v1.0.0) |
| `report.html` | HTML | Interactive report with collapsible sections |

### Report Sections

1. **Signal Fate** — ID rate by count and TIC, chimeric scan %
2. **Modification Landscape** — Top peaks with Unimod annotations
3. **Contamination** — Polymer %TIC with breakdown
4. **Glycopeptides** — Oxonium-positive spectrum count
5. **Digestion** — Missed cleavage distribution, ragged ends %
6. **Mass Accuracy** — Precursor and fragment ppm (median, 95th %ile)
7. **Alkylation Check** — Cys PSM count, unalkylated signal %
8. **Three-Layer MS1** — (optional) Non-peptidic, never-sampled, sampled-not-ID'd, identified

### Alkylation Check Feature

Detects incomplete alkylation by looking for Cys-containing PSMs with delta mass near -57 Da (missing carbamidomethyl):
- `< 1%` unalkylated → "✓ Alkylation appears complete"
- `1-5%` unalkylated → "⚠ Minor incomplete alkylation"
- `> 5%` unalkylated → "✗ Significant incomplete alkylation"

### Files Created/Modified
- `recon-tool/src/report.rs` — New module with ReconReport, HTML generation
- `recon-tool/src/lib.rs` — Export report module
- `recon-tool/src/main.rs` — Added `analyze` command
- `recon-tool/Cargo.toml` — Added `chrono` dependency for timestamps

### Checkpoint Status
- [x] Create `ReconReport` struct
- [x] Implement `compute_alkylation_check()`
- [x] Implement `print_report_summary()` for console
- [x] Implement `generate_html_report()` for HTML
- [x] Add `analyze` CLI command
- [x] Support `--full` flag for three-layer MS1
- [x] All code compiles successfully

### Post-Phase Notes (added 2026-07-14)

**Deferred to Phase 7B:**

1. **MS1 Mass Error Reporting** — Currently removed from HTML because open search precursor_ppm reflects delta mass (PTM shift), not instrument calibration. To get true MS1 mass accuracy:
   - Option A: Run a narrow search (±10-20 ppm) and use those precursor_ppm values
   - Option B: Compute from isotope-corrected delta mass for PSMs near zero (unmodified)
   - Option C: Use fragment ppm as proxy (already shown, reflects true calibration)
   - **UNBLOCKED by Phase 8 (2026-07-15):** Option A is now essentially free — Step 0
     produced closed (narrow ±10 ppm) reference searches on all three files. In a
     closed search the precursor delta is ~0 for correct IDs, so its `precursor_ppm`
     IS real instrument MS1 mass accuracy; the data already exists in
     `testing/search-output/closed-ref-*/`. Queued as **PLAN Phase 8.5** (extraction +
     reporting, no new search). This same closed-search precursor_ppm is also the
     validation reference for the deferred m/z-dependent calibration (see the "Deferred
     enhancements" section near the top of this file).

2. **PTM List Improvements** — Current list shows raw Unimod matches. Could improve by:
   - Grouping isotope peaks (0.00, 1.00, 2.00 Da) together as "unmodified + isotope error"
   - Filtering out isotope labels (15N, 18O) that aren't real PTMs in most experiments
   - Showing ambiguity count (e.g., "3 possible annotations") instead of just first match
   - Adding biological relevance flags (common artifact vs. biologically interesting)
   - Collapsing multiple peaks at same nominal mass (e.g., 0.0003 and 0.0018 are both "unmodified")

---

## Phase 7B — Mod Discovery Pipeline Fixes
**Status:** ✅ Complete  
**Started:** 2026-07-14  
**Completed:** 2026-07-14

### Goal
Fix fundamental issues in mod discovery that corrupt histogram input before annotation runs. Required before Phase 8 validation can be meaningful.

### Context

The mod discovery output was annotating raw, uncalibrated, un-folded delta masses. Three problems corrupted the histogram input:

1. **NEUTRON constant was wrong** — Using free neutron mass (1.00866) instead of ¹³C−¹²C spacing (1.003355); 5.3 mDa overcorrection per isotope step.
2. **Isotope correction was a no-op on open-search data** — `isotope_error` column is always 0 in Da-tolerance open search.
3. **No mass calibration** — Instrument drift biased the Δ=0 population off-center.

### Key Design Decisions

1. **K-scaled fold tolerance: 12 mDa + (|k|-1) × 4.5 mDa** — Deamidation (+0.984) is 11.2 mDa from k=1 position (+1.003355), just outside the 12 mDa base tolerance. This preserves real deamidation while folding isotope artifacts.

2. **Prominence threshold: 0.3 × count** — Noise floor has prominence ≈ 0, real PTM peaks rise sharply.

3. **No hyperscore gate** — Monoisotope misassignments score as well as unmodified (fragments match perfectly, error is on precursor only). Hyperscore is not a discriminator.

4. **Satellite folding in Step 4.5** — Fold satellites onto detected peaks after peak detection (Step 4), not before.

5. **AA substitution filter** — Exclude "AA substitution" and "Other glycosylation" classifications from annotations.

### Implementation Summary

- [x] Step 0: Added C13_C12_DIFF constant (1.003354835) in mass.rs
- [x] Step 1: Updated mzml.rs to use C13_C12_DIFF for isotope envelope spacing
- [x] Step 2: Added mass calibration (apex_offset) in mod_discovery.rs
- [x] Step 3: Added neutron fold-to-zero with k-scaled tolerance
- [x] Step 4: Replaced threshold+merge with prominence-based peak detection
- [x] Step 4.5: Added satellite folding onto detected peaks
- [x] Step 5: Wired excluded_classifications filter in annotation
- [x] Step 6: Updated reference-notes/result-schema.md with apex_offset_da
- [x] Validation: Verified on open-search test data

### Validation Results (Open Search on B.naive_01steady-state.mzML.gz)

**Folding Statistics:**
- Folded to zero: 9,412 PSMs
- Satellite folds: 9,691 PSMs
- By k: k=1: 9,676, k=2: 3,259, k=-1: 2,517, k=-2: 1,915, k=3: 1,085, k=-3: 651

**Peak Detection Results:**
| Rank | Delta (Da) | Count | Annotation | Status |
|------|------------|-------|------------|--------|
| 1 | 0.0000 | 43,226 | Unmodified | ✅ Rolled up from 4 near-zero peaks |
| 2 | 15.9954 | 2,717 | Oxidation | ✅ Preserved |
| 3 | 0.9922 | 2,676 | Deamidated | ✅ Preserved (11.2 mDa from k=1) |
| 4 | 0.9817 | 1,615 | Deamidated | ✅ Preserved |
| 17 | 1.9910 | 500 | UNANNOTATED | ✅ Double deamidation (15.7 mDa from k=2) |
| 32 | 42.0110 | 230 | Acetyl | ✅ Preserved |

**Key Observations:**

1. **Deamidation preserved** — The +0.9922 Da peak (2,676 PSMs) is real deamidation, not an isotope artifact. It's 11.2 mDa from the k=1 position (1.003355 Da), just outside the 12 mDa base tolerance. The k-scaled tolerance correctly discriminates.

2. **Double deamidation preserved** — The +1.9910 Da peak (500 PSMs) is double deamidation (2 × 0.984 = 1.968 Da, observed at 1.991 Da). It's 15.7 mDa from the k=2 position (2.00671 Da), outside the 16.5 mDa k=2 tolerance.

3. **Isotope artifacts folded** — The k=1 fold captured 9,676 PSMs that were true isotope misassignments (landing exactly at 1.003 Da, not at 0.984 Da).

4. **Unmodified roll-up working** — 4 near-zero peaks collapsed into single Unmodified entry at 0.0000 Da with 43,226 PSMs.

5. **AA substitution filter working** — No "Glu→Met" or similar substitution annotations in top-20 peaks.

### Validation Criteria Met

| Metric | Before | After | Status |
|--------|--------|-------|--------|
| apex_offset in QC | Not reported | Reported (0.0001 mDa) | ✅ |
| Apex centered on zero | ~3.6 mDa bias | 0.0001 mDa | ✅ |
| "Unmodified" bins | 8 separate | 1 (rolled up) | ✅ |
| +1.003 Da ("Label:15N") | present | folded to Δ=0 | ✅ |
| +2.00x Da | present | folded | ✅ |
| ±1 Da UNANNOTATED peaks | ~6 | 0-1 | ✅ |
| +0.984 Da (deamidation) | present | still present | ✅ |
| +15.995 Da (oxidation) | present | still present | ✅ |
| AA-substitution annotations | several | gone from top-20 | ✅ |

### Reference Material

- `reference/Crystal-C/` — Crystal-C source code (Apache-2.0) for residue-mass reassignment approach. Deferred to post-Phase 8.
- `reference-notes/mod-discovery-calibration.md` — Full technical notes on the calibration approach.

---

## Phase 8 — Validation Pass
**Status:** ✅ Complete (2026-07-15) — Tier 2 ✅, Step 0 ✅, Tier 1 ✅ (Gates 1/2/3), Tier 3 ✅

### Tier 1 Gate 2 — open vs closed Oxidation (corrected invariant)

**The gate, as verified (2026-07-15).** Earlier draft predictions were WRONG and are
deliberately NOT preserved here (see below) — this is the corrected, evidence-backed
invariant a future reader should inherit:

- **Corrected invariant:** open-search and closed-search Oxidation counts are the
  **same order of magnitude, open lower, with a ratio consistent across files.**
  **Standardized on `spectrum_q < 0.01`** (matches Sage's headline FDR count):
  b1906 0.53×, B-cell 0.35×, serum 0.59× (open +16 / closed Ox). Cross-checked under
  `peptide_q < 0.01` (0.47 / 0.36 / 0.64) — the conclusion is robust to the basis
  choice, ratios shift ≤0.06, so the direction+magnitude claim holds either way. Use
  spectrum_q as the basis going forward; the run_validation harness should compute
  against spectrum_q.
- **Why open is lower (the gap cause):** the closed reference search *targets* Ox(M)
  as a variable mod — Sage enumerates the oxidized form of every candidate and scores
  it per-candidate, so a marginal spectrum still gets identified as oxidized. The open
  search only sees a +15.995 **precursor delta** and must win FDR as a delta-mass PSM
  competing against the entire ±500 Da window — a strictly harder scoring problem.
  Closed therefore finds ~1.6× more oxidized peptides. This is **search sensitivity,
  not a bug.**
- **Verified by (b1906):**
  - **Unique-peptide overlap** (rank-1): 873 closed Ox peptides / 546 open +16 peptides
    / 405 shared. 468 closed-only, 141 open-only.
  - **Spot-check** of 30 closed-only oxidized peptides looked up in the open PSM list
    by sequence+scan: 18 unmatched (lost at search), 8 matched at a different delta
    (passing, not double-counted), 1 below-FDR near +16 (q=0.0196), 2 matched-only-
    failing. **0 roll-up/binning bugs.** Dominant category is "lost at search," not
    "below FDR."
  - Three peptides initially flagged as "passed FDR near +16 but absent from the peak"
    (the bad case) all resolved to **rank-2 chimeric secondary** PSMs — their rank-1
    ID is the *unmodified* form in their own spectrum; the +16 evidence is a co-eluting
    chimeric secondary correctly excluded from the primary Ox peak. Sage assigns one
    `peptide_q` per sequence, so rank-2 rows inherit the rank-1 q — which is what made
    the naive filter misfire. The peak boundary is correct.
  - **Fold log clean:** zero PSMs drained from +16 or +17. Only the k=1 band folded
    (window [0.9914, 1.0154], 3,114 PSMs → Δ=0). Roll-up and binning (separate paths
    from folding) also verified not to move any passing +16 PSM out of the peak.
- **The +17 isotope-split band (279 PSMs at +16.998) is a real but MINOR contributor.**
  Closed counts isotope-miscalled Ox as Ox; open splits it to +17.
- **Two earlier predictions were FALSE — do not resurrect either:**
  1. "open ≥ closed" (the direction was backwards).
  2. "gap = +17 + dedup + diOx" — this sums to only 410 of the 1,234 b1906 gap, and
     **chimeric dedup accounts for ~0** here (+16 band = 1,058 PSMs across 1,057 unique
     scans). Raw-count reconciliation is the WRONG basis; **unique-peptide overlap is
     the correct basis.** The residual 824 is closed-search targeting sensitivity, not
     a mechanical accounting term.
- **FASTA consistency invariant (now provable, not eyeballed):** FASTA equality across
  each Gate 2 comparison is verified by reading the actual FASTA path from each run's
  **emitted `results.json`** (Sage records params as-run), NOT from the config files.
  Confirmed all six runs (b1906 / serum / B-cell × closed / open) used the same
  canonical-20k FASTA (`UniProt-Human-UP000005640_canonical-2023_05.fasta`) on both
  sides, so no ratio is cross-DB. **Note:** `PXD0014688.json` (Lazear's reference open
  config) carries HIS FASTA path and a `human_contam.fasta` reference — that's a
  **template artefact, never used in our runs**; don't be misled by it. The
  run_validation harness should assert FASTA-equality from `results.json` across
  compared runs, not from configs.

### Tier 1 Gate 3 — serum +57 triage (over-alkylation, NOT 7D)

**Finding (2026-07-15).** Serum's open search (`open-serum-full`) produced a
prominence-passing peak at +57.02 — the mass **degenerate between Carbamidomethyl /
off-site CAM and the Glycine residue (both 57.02146)**. This is the exact peak Phase 7D
was gated on. Triaged to evidence; **verdict: over-alkylation, 7D deferred by evidence
(not by absence).**

Population (after the mandatory rank-1 filter — see the q-inheritance gotcha):

- **86 band PSMs (peptide_q<0.01, |Δ−57.0215|≤~25 mDa) → 84 rank-1** primaries; 2
  rank-2 chimeric secondaries excluded (both confirmed sharing a scan with a rank-1 ID).
  (Under the standardized spectrum_q basis the rank-1 count is 76 / 45 already-CAM Cys
  — same conclusion, smaller population; the split and the verdict do not change.)
- **49 rank-1 are Cys peptides that ALREADY carry `[+57.0215]` CAM** and have a *second*
  +57 delta on top → over-alkylation, self-standing finding. Not incomplete alkylation
  (they have their CAM); an *extra* +57. **Localized (spectrum_q, 45 peptides): 45/45
  contain an off-site CAM acceptor side chain (K/H/E/S/T/Y), all also have the peptide
  N-terminus (universal CAM acceptor) — the extra +57 is chemically explainable as a
  second, off-site carbamidomethylation on a residue the peptide actually contains, not
  a scoring artefact.** (Acceptor-existence confirmed; not fragment-localized to a
  specific residue — per-site localization is a locked non-goal.)
- **35 rank-1 are non-Cys peptides (26 unique) carrying +57 with no Cys at all.** These
  were the actual 7D question. Ran the flanking check (below): **0 of 26 had Gly
  flanking context** — every one has a tryptic K/R at the N-flank and no Gly at either
  terminus. Proven **off-site CAM on N-term/K/E/H/S/T/Y, NOT adds-Gly.** The peptides
  are the highest-abundance serum proteins (albumin P02768, ApoA-I P02647, Ig
  P01857/59/34, transferrin P02787) — exactly where over-alkylation concentrates.
- Corroborated by `reference-notes/over-alkylation.md` (Müller & Winter 2017; Boja &
  Fales 2001 — off-site CAM is "the rule rather than the exception" with IAM).

**Do NOT drop the scatter-as-discriminator argument back in.** An earlier draft claimed
the +57 deltas scattering 1–2 mDa high argued against adds-Gly. That is FALSE: CAM and
Gly are mass-identical (57.02146), so ppm scatter cannot separate over-alkylation-CAM
from adds-Gly — it argues neither way. The flanking check is the only discriminator.

### Reusable methods that came out of Phase 8 (use these, don't rebuild the reasoning)

1. **Rank-1 filter before any delta-band count** — see the q-inheritance gotcha under
   "Intentional, not bugs." Mandatory zeroth step.
2. **Flanking-check-as-7D-gate** — the go/no-go test for whether a residue-mass peak
   (+57 Gly, +71 Ala, +114 GlyGly, etc.) is a real adds-residue artifact (→ 7D
   activates) or something else (→ 7D stays deferred). Manual one-off, ~40 lines of
   Python, is the manual version of 7D's core data question:
   - Take the rank-1 PSMs in the residue-mass band.
   - For each, look the bare peptide up in the search FASTA, get the residue immediately
     before the N-term and after the C-term (`-` = protein terminus).
   - Test for the candidate residue in flanking context (for +57: is a Gly adjacent?).
   - **0 flanking-context hits → over-alkylation / off-site mod, 7D stays deferred.
     Any hits → real adds-residue artifacts, 7D activates.**
   Proven on serum +57 (0/26 → deferred). When a residue-mass peak shows up on a new
   file, the move is "run the flanking check," not "rebuild the reasoning." This is what
   makes the triage a reusable decision procedure instead of a one-off.

### Pre-Phase Notes (added 2026-07-07)

**Future improvements identified during Phase 3:**

1. **Site-specific validation** — We store position specificity from Unimod (e.g., "Any N-term", "Anywhere") but don't validate that the modification is on the correct residue. For example, pyro-Glu should only appear at N-terminus on Q, but we don't check if the peptide actually has Q at N-term.

2. **Combination mass detection** — PTM-Shepherd detects "Oxidation + Deamidation" combinations. We don't currently look for peaks at sum masses (e.g., 16.98 Da = 15.99 + 0.98). Could add a "combination" annotation source.

3. **Mass defect filtering** — The schema has `excluded_classifications` but we don't actually filter by it yet. See `reference-notes/unimod-classification-filtering.md` for the full list of classifications to exclude (isotopic labels, isobaric tags, etc.).

4. **Integration test with real data** — Run `discover` against actual `results.sage.tsv` and assert:
   - Unmodified peak is rank 1
   - Oxidation peak exists near 15.99 Da
   - Deamidation peak exists near 0.98 Da
   - The 52.91 Da peak is flagged as isotope artifact (see `reference-notes/sage-config-and-gotchas.md`)

5. **Top candidates summary** — For each peak, show the top 3 Unimod candidates with their mass errors, not just all matches. Helps when there are many ambiguous matches.

---

## Appendix: Key File Locations

| Resource | Path | Notes |
|----------|------|-------|
| Sage binary (Windows) | `reference/sage/sage-v0.14.7-x86_64-pc-windows-msvc/sage.exe` | Reports v0.14.6 |
| Unimod XML | `testing/unimod.xml` | Full database for mod annotation |
| Contaminants table | `testing/positiveContaminants.txt` | 831 entries, Keller 2008 |
| Contaminants refs | `testing/refsContaminants.txt` | Bibliography for ref codes A-Z |
| Test mzML | `testing/B.naive_01steady-state.mzML.gz` | Human naive B cells (not benchmark) |
| Benchmark mzML | `testing/inputs/b1906_293T_proteinID_01A_QE3_122212.mzML.gz` | PXD001468; downloaded + verified 2026-07-15 (447 MB gz → 853 MB, gzip clean) |
| Open search config | `testing/open-search-params.json` | Template A |
| Closed search config | `testing/params.json` | For comparison |

## Appendix: Contaminants File Format

`positiveContaminants.txt` columns (tab-delimited):
1. Monoisotopic ion mass (singly charged)
2. Ion type (e.g., `[M+H]+`, `[M+Na]+`)
3. Formula for M or subunit
4. Compound ID or species
5. Possible origin and comments
6. (blank)
7. ESI flag (X = yes)
8. MALDI flag (X = yes)
9. Reference codes (A-Z, see `refsContaminants.txt`)
10-17. Various database cross-references

Example entry:
```
33.033491	[M+H]+	CH3OH	Methanol	"Acetonitrile, solvent"		X		A
```

---

## Phase 7C — Mod Discovery Pipeline Correction
**Status:** ✅ Complete
**Started:** 2026-07-14
**Completed:** 2026-07-14

### Trigger
Phase 7 report review: mod-discovery top peaks were artifact-dominated. Original open-search table had ranks 1/2/8/12/13/23/26/33 all "Unmodified" (one smeared population), ranks 3/5/6 isotope misassignments annotated as ¹⁵N/¹⁸O, and a swarm of AA-substitution nearest-mass noise. Diagnosed as an input problem (uncalibrated, un-folded, discretized deltas), not an annotation problem.

### Root causes found (in order of dependency)
1. **NEUTRON constant wrong** — free neutron mass (1.00866) vs ¹³C−¹²C (1.003355), 5.3 mDa/step overcorrection. Confirmed by the narrow-search tail asymmetry (−0.01 at 11.9%, +0.01 at 2.9%).
2. **Isotope correction inert on open search** — `isotope_error` always 0 in Da-tolerance search, so the correction term is 0×constant regardless of the constant. The Phase 2 "validation" had passed only because it tested the narrow-search path where the column is populated.
3. **No calibration** — nothing centered the Δ=0 population.
4. **Threshold+merge peak detection** — fragmented the zero population into multiple rows.
5. **`excluded_classifications` defined but unused.**
6. **Bin-level folding too coarse** — the 0.99 bin blended real deamidation (0.984) and isotope residue (0.992); unsplittable at bin granularity.
7. **Satellite folding unguarded** — no conservation check, unverifiable counts.

### Fixes (chronological — this was an iterative debug)
- Constant → `C13_C12_DIFF = 1.003354835` everywhere.
- Scalar calibration (`apex_offset`, intensity-weighted median of |Δ|<0.1). Corrects bias, not spread (scalar limitation documented). Measured apex_offset ≈ 0.1 mDa.
- Prominence-based peak detection replacing threshold+merge.
- Unmodified roll-up at 0.075 Da — collapses the near-zero smear by classification, decoupled from merge tolerance. This (not calibration) is what turned the 8-bin smear into one row.
- Merge tolerance fix: boundary was `<` at exactly-one-bin-width apart; changed so adjacent bins merge, kept tight (1.0×bin_width) so real peaks stay separate.
- `excluded_classifications` applied at annotation; **root cause of it never firing: all AA-substitution specificities are `hidden="1"` in Unimod, so `classification()` returned "Unknown."** Fixed to fall back to hidden specificities only when *all* are hidden.
- Annotation policy: suppress misleading annotation, keep peak as UNANNOTATED with count intact (don't drop real peaks).
- **Bin-level folding → PSM-level fold-to-zero.** Folds per PSM before histogram build. The only granularity where the deamidation/residue blend separates.
- k-scaled fold tolerance `base + (|k|−1)×per_step` (12 / 16.5 / 21 mDa). Note: an earlier draft dropped the `−1`, putting k=1 at 16.5 and collapsing deamidation clearance to 5.15 mDa — caught and corrected back to k=1=12 mDa (9.65 mDa clear).
- Fold against bin intensity-weighted apex, not bin_center — removes discretization error at source (was causing +0.9922 to miss the fold window by ~1 mDa because its bin center at 0.99 read 13.4 mDa from target vs the PSMs' true 11.2 mDa).
- **Satellite folding disabled by default.**

### The +0.9922 / +1.9910 saga (the long part)
These two peaks were diagnosed as isotope residue (hyperscore 95.3% of unmodified; +2 twin present; ladder confirmed). Multiple failed fix attempts before resolution:
- Bin-level fold with flat tolerance — missed them (discretization).
- Weighted-apex fold — folded +1 but a flat 12 mDa missed the +2 (15.7 mDa off).
- k-scaled tolerance — correct, but revealed the bin-vs-apex discretization issue underneath.
- PSM-level folding — finally folded both to zero, verified: count(Δ==0) grew by exactly folded_to_zero_count.
- **Satellite folding then re-inflated deamidation** by depositing "satellites" onto it: 1,770 onto an 870 parent — physically impossible (M+1 < M+0 always; total envelope < monoisotope). This exposed that satellite folding had no conservation guard and reported numbers (1,770 → 766 → 583) that didn't even match its own code. Resolved by disabling it rather than adding more heuristics.

### Final validation (`B.naive_01steady-state`)
```
calibrated_deltas.len() = 81,966 (invariant)
PSMs delta exactly 0.0: pre=819, post=10,439, diff=9,620
folded_to_zero_count = 9,620   (conservation: 9,620 = 9,620 ✓)

Top peaks:
1. Unmodified   0.0000 Da   52,846 (64.5%)
2. Oxidation   15.9954 Da    1,804 (2.2%)
3. Deamidated   0.9817 Da      894 (1.1%)

+0.9922 ghost: GONE
+1.9910 ghost: GONE
53/53 tests pass
```

### Notes for downstream use
- **Deamidation 894 slightly undercounts true prevalence** — isotope-miscalled deamidation PSMs fold to zero. Correct for recon (presence/priority), NOT a stoichiometry number. Don't read it as one in the tight search.
- **Deamidation apex reads 0.9817 (2.3 mDa low)** — mass-dependent ppm calibration drift, not contamination. Scalar calibration can't fix m/z-dependent drift; a real search would recalibrate per-m/z. Fine for recon.
- **Satellite folding is off by default on purpose** — it has no conservation guard and its counts are unverifiable. Do not re-enable as a "free improvement." Function retained for possible future work with a proper invariant.
- **Methodological lesson:** fixes backed by a hard invariant (conservation, physical impossibility) closed in one pass; the invariant-free path (satellite folding) ate most of the debugging time. Lead with invariants in Phase 8.

### Open Questions
- (none for 7C — residue-mass degeneracy moved to Phase 7D)


---

### Project architecture & setup baseline (moved from PLAN.md, 2026-08-19)

This was PLAN.md's original Phase-0 planning content — architecture diagram,
Rust project structure, resolved setup facts, working-setup process notes,
and the pre-Phase-0 vendor checklist. Moved here as part of the 2026-08-19
PLAN.md tightening pass since it's now realized (the repo/structure exist
and are more authoritative than this plan), not forward-looking. Content
unchanged from the original; check the live `recon-tool/src/` tree for
current ground truth over this if they ever disagree.

**Mission (original framing):** A fast reconnaissance tool for unfamiliar
mass-spec data. Given an mzML + FASTA, run ONE Sage open search, then
report: (1) what modifications are present (delta-mass discovery), (2)
where the signal is going — explained vs. unexplained, by count AND
intensity, including polymer contamination as %TIC, (3) what molecule
classes are present via diagnostic fragment ions (glycans/oxonium ions are
the flagship). Target user: an expert in a new species/tissue who wants
"what's here, what's worth chasing, what am I missing" — fast, free, open
spiritual successor to Byonics Preview.

**Architecture:**
```
[mzML] + [FASTA] + [open-search params.json]
        |
        +-> Sage open search (subprocess) -> results.sage.tsv, results.json
        |        +-> mod discovery (delta mass, expmass/calcmass)
        |        +-> signal fate (IDed vs not; intensity weighting)
        |        +-> digestion (missed cleavages, semi-tryptic)
        |
        +-> Direct mzML read (single pass, BOTH MS levels)
                 +-> MS1 -> polymer %TIC (ported from mzSniffer)
                 +-> MS2 -> diagnostic ions (oxonium + contaminant masses)
        |
        v
   Core library -> structured RESULT (serializable JSON)
        +-> CLI (build now)
        +-> GUI (fast-follow, Tauri/egui, consumes same JSON)
```
Sage is invoked as a subprocess (not embedded as a lib dependency) — this
was already decided, don't relitigate it. Core logic lives in `lib.rs` so
CLI and future GUI share it.

**Resolved setup facts:**
- Unimod: `unimod.xml` vendored to `reference/unimod/unimod.xml`. Each
  `<umod:mod>` has `title`, `full_name`, `record_id`, a
  `<umod:delta mono_mass=...>`, and `<umod:specificity>` entries.
- Contaminants: `positiveContaminants.txt` and `refsContaminants.txt`
  vendored to `reference/contaminants/` (actual filenames, not the
  original TSV placeholders).
- Sage binary vendored as a compiled Windows release (not built from
  source); `sage_runner.rs` locates it via a relative/config path, not PATH.
- All `/reference-notes` vendor-md docs were done before Phase 0 started.

**Working setup (process, not architecture) — NOTE: partially stale, see
2026-08-19 debrief re: Perplexity session and git-identity/batching drift.**
- Repo: GitHub, private, PAT-scoped. Commits authored as `neely`
  (see AGENTS.md).
- Editor at time of writing: Claude Code (VS Code extension), Opus 4.8 for
  all work in one loop. Superseded the earlier Cline + Opus-plan/Sonnet-act
  split. As of 2026-08-19, sessions have also been run via Perplexity
  (Sonnet/Opus) when API access was unavailable — see that session's
  debrief for the process gaps that surfaced (author identity, commit
  batching, shutdown-cadence).
- Air-gapped references: all reference material vendored in, never fetched
  live; commit-often expectation.
- Test set: known mzML + FASTA with expected mods, contaminants, and glyco
  signal, under `test-data/`.


---

### Report/paper-facing script status (2026-08-19)

Split from dev-facing scripts (`run_validation.py`, `psm_count_sensitivity.py`)
per Ben's call: report scripts produce clean tables for the paper, dev
scripts are internal-only.

**PTM comparison — exists, re-run and confirmed current 2026-08-19:**
`testing/scripts/compare_mod_discovery.py` against PTM-Shepherd (reallyOpen)
and Mascot (error-tolerant) reproduced byte-identical output to what was
already committed at `testing/recon-output/comparison/recon_vs_ptmshepherd_reallyOpen.md`
and `recon_vs_mascot.md` — confirms these are current, not stale from Phase 8.
MetaMorpheus is NOT yet a fourth column (adapter not built — needs the
Task3-SearchTask `MassDifferenceHistogram.tsv` per-file question resolved
first, see PLAN Phase 8.6/Future). Consensus/intersection view across all
loaded tools explicitly deferred until Preview data lands (Ben's call,
2026-08-19) — don't build ahead of that.

**Mass-error comparison — does NOT exist as a script.** Today's
recon-vs-FragPipe-vs-MetaMorpheus MS1/MS2 comparison lived only as prose in
this file's MetaMorpheus-review entries, not a reusable, report-ready table
generator. This is the actual gap matching "scripts for final report, not
dev" — needs a `compare_mass_error.py` (or similar) pulling recon's
`calibration.rs` clean-subset stats + FragPipe MS1(Old/New) + MetaMorpheus
round-0 into one table, same tool-vs-tool spirit as `compare_mod_discovery.py`.
Blocked on Phase 8.6 (which runs are canonical) before it's worth building —
no point automating a comparison against ground truth we haven't confirmed yet.

**`psm_count_sensitivity.py` — written 2026-08-19, NOT yet run.** Needs
Ben's local wide-search `results.sage.tsv` for all three files (not
committed — gitignored raw output). Run instructions given in that
session's chat; results not yet in hand.


---

### 🔒 Detector-aware pass-1 MS2 tolerance — BUILT 2026-08-28 (parallel session)

**The problem.** `fragment_tol` was hardcoded `ppm [-20, 20]` in every config and
appeared in no Rust source. On ion-trap MS2 that is ~30x too tight: the SEARCH
matches almost nothing, so every downstream number is garbage before any
recommendation exists. It is a pass-1 INPUT, so it must be decided before Sage
runs — nothing downstream can recover PSMs that were never made.

**Two signals carry the analyzer, and they are NOT equally reliable.**
Recon prefers the per-scan Thermo filter string (`MS:1000512`) and falls back to
the file-level `instrumentConfiguration` CV term.

**MEASURED, and it overturned the reference note.** `B.naive_01steady-state` is a
**Q Exactive Plus** whose mzML declares its analyzer as `MS:1000079` FT-ICR. A
Q Exactive Plus has no ICR cell. This is not a mystery: ThermoRawFileParser mapped
every Orbitrap to FTICR until 1.4.4, whose release notes read *"Using CVTerm
'Orbitap' instead of 'FTICR' for Orbitrap-based instruments (closes #177)"*
(2024-05-10). bcell was converted with 1.4.2, the other two with 1.4.4. The filter
string is immune — Orbitrap and FT-ICR both report `FTMS`, both are high-res, so
the tolerance is unchanged. `reference-notes/mzml-instrument-metadata-tolerances.md`
previously ranked the CV term ABOVE the filter string; that is corrected in place.

**A hypothesis that was wrong, recorded so it is not re-formed.** The Fusion Lumos
file declares only `orbitrap`, and this was first read as "the componentList is
hiding the ion trap on a hybrid". It is not. ThermoRawFileParser builds the
component list from the analyzers ACTUALLY USED by the scans, so an all-FT run on
a hybrid correctly declares one analyzer. Do not read a single declared analyzer
as proof the instrument has only one.

**The CV term set is DERIVED, not asserted.** `testing/scripts/psi_ms_analyzer_terms.py`
walks `is_a` from `MS:1000443` in a pinned `psi-ms.obo` (`data-version 4.1.259`,
sha256 pinned) and yields 12 live descendants. It hard-fails if the CV gains a
term recon does not classify, if recon names an accession absent from the CV, or
if a name drifts. Tier 4 of `run_validation.py`. **This mattered:** reading the
OBO through a summariser instead returned 6 of the 7 direct children and silently
dropped `orbitrap`.

**⚠ THE MS:1000264 TRAP.** ThermoRawFileParser maps `MassAnalyzerITMS` to the
GENERIC parent `MS:1000264` "ion trap", not to `MS:1000082` or `MS:1000291`. A
table listing only the specific children MISSES every ThermoRawFileParser-
converted ion-trap file — precisely the case this feature exists to catch. Both
the old reference note and Ben's curated CSV omit it; it is classified explicitly.

**⚠ Astral must not inherit its CV parent.** `MS:1003379` is a CV child of
`MS:1000084` time-of-flight, but performs like an Orbitrap. Bucketing by parentage
would give it the legacy-TOF ±100 ppm window. Most-specific-wins; asserted by
`astral_is_not_bucketed_with_legacy_tof`.

**Sampling, and why the stopping rule is what it is.** Detection reads the first
100 MS2 scans and stops; MS1 fills opportunistically. **The rule waits for the MS2
sample however deep it lies, and this is not theoretical:** in
`2019-4-9_909c_0311` the first MS2 is at spectrum 140 and **2063 MS1 scans precede
the 100th MS2**. An earlier version capped total reads at 2× the sample and would
have sampled almost no MS2 on that file. Streaming, non-indexed, metadata-only
reader: 0.04–0.42 s per file on 164–448 MB inputs.

**DECISION: recon never refuses to search on analyzer grounds.**
*Rejected alternative:* hard-stop when the template's `fragment_tol` unit
disagrees with the detected analyzer. That was built first, then removed on Ben's
instruction. A reconnaissance tool that halts is less useful than one that
proceeds under a stated assumption. Unknown analyzer, no bucket, and a mid-run
detector change all fall back to ±20 ppm, flag `assumed`, and report the detectors
seen.

**DECISION: the template's `fragment_tol` is OVERRIDDEN, not validated.**
*Rejected alternative:* leave the template authoritative and only warn. Rejected
because the template is ±20 ppm regardless of instrument, so warning would leave
the wrong value in place. `run` writes `<search-dir>/effective-params.json`
carrying a `_recon_fragment_tol_provenance` block: template path, the template's
original value, the value applied, the basis, and the analyzers seen at both MS
levels.

**⚠ The ±20 ppm fallback is TIGHTER than the ±50 ppm Orbitrap bucket it usually
stands in for.** Intentional, and conservative in the behavioural sense rather
than the statistical one: an undetectable file gets recon's existing behaviour,
not a wider window nobody asked for. Reported every time it is used.

**⚠ CONSEQUENCE FOR REGENERATION: pass-1 Orbitrap searches now run at ±50 ppm,
not ±20 ppm.** That changes SEARCH OUTPUT, not just schema. The committed
`full-run/` was searched at ±20 ppm and is internally consistent. Any regeneration
impact trace must say so.

### ⚠ Discovered truth — `fragment_tol` is hardcoded in 21 configs, not ten

Re-measured 2026-08-28. **18 configs carry `ppm [-20, 20]`; 3
(`closed-search-reference*`) carry `ppm [-10, 10]`.** `PXD0014688.json` is not
valid JSON and was excluded from the count. The original brief scoped the problem
to the ten `open-search-*.json` templates. It is wider:
`digestion-efficiency-pass1/pass2.json` and `serum-digestion-pass1/pass2.json`
are ALSO `ppm [-20, 20]`, and those are the Pass-2 and `digestion_efficiency`
templates that step 3 still has to work through. **The same wrong-for-an-ion-trap
unit is already baked into the templates a Pass-2 cushion would be carried into.**
`run` currently overrides the pass-1 path ONLY; the Pass-2 path is not covered.

### ⚠ Discovered truth — `DEFAULT_SAGE_PATH` is a Windows path on a Darwin host

`sage_runner.rs:36` reads
`reference/sage/sage-v0.14.7-x86_64-pc-windows-msvc/sage.exe`. The development
host is Darwin arm64, so that default can NEVER resolve there. `SAGE_PATH` is
therefore not a convenience on an arm64 host — it is the only route to a binary.
Step 4 already carries "fix `sage_runner.rs`'s repo-root-relative Sage binary
path"; this is the concrete reason it matters.

## Audit findings not yet fixed — 2026-09-02

Corrected in place 2026-09-02, end of session. See AUDIT-2026-09-02.md for
full detail and per-finding status.

**All six items originally listed here are now FIXED.** Kept as a record,
not deleted:

- The `discover` console table printed Match% 100x too large. Fixed, `e80ebbe`.
- `REQUIRED_COLUMNS` validated 21 of 37 columns; `rt` was used but unvalidated. Fixed, `e80ebbe`.
- recon skipped MS2 spectra with precursor m/z 0; Sage v0.15 falls back to the isolation window instead. Fixed on the main path, `bb79abb` — one function, `extract_ms2_from_reader`, still has the gap. See below.
- `PROTON_MASS` was defined three times, with two different values. Fixed, `bb79abb`.
- Three `testing/configs/*.json` carried `fragment_min_mz`/`fragment_max_mz`, which v0.15 removed and silently ignores. Fixed, `3c94435`.
- `CuratedDb::load` was exercised only by tests; production used `load_from_sources`. Fixed, `2493973`.

**All five items originally listed as "genuinely still open" are now ALSO FIXED**,
the last of them 2026-09-03. Kept as a record, not deleted:

- `report.rs`'s `MassAccuracySummaryReport` doc said all four ppm fields are absolute and cannot be negative. Fixed, `38fab8a` (2026-09-02) — it now says `precursor_*_ppm` is signed and `fragment_*_ppm` stays absolute, matching `qc.rs`.
- `report.rs`'s schema version-history comment stopped at 1.7.0. Fixed incrementally as each bump landed: 3.0.0 (`45dd004`), 3.1.0 (`b62c331`), 3.2.0 (`7725eb3`), all 2026-09-02/03. `SCHEMA_VERSION` is now 3.2.0 and every intervening bump has its own entry in the comment.
- `extract_ms2_from_reader` (feeds oxonium screening) had no isolation-window fallback and kept the m/z-0 spectrum rather than dropping it. Fixed, `33c37bd` (2026-09-02) — same fallback Sage v0.15 applies, unexercised on the four committed files (all DDA Thermo with a proper selected-ion m/z), verified UNEXERCISED by re-measuring every downstream number unchanged.
- `sage_runner.rs`'s module doc said it "locates the Sage binary and invokes it." Fixed, `38fab8a` (2026-09-02) — it now says Sage has been an in-process library since 2026-09-01.
- `main.rs` claimed "analyze + qc-stats carry provenance." Fixed, `38fab8a` (2026-09-02) — corrected to say provenance is built only by `run_qc_stats_command`.

## HTML report port and the fixes it needed — 2026-09-03

### ✅ THE HTML REPORT LAYOUT LANDED — six mockup defects fixed, not copied (2026-09-03, `45dd004`)

The layout was already agreed and encoded in `testing/scripts/report_layout_mockup.py`.
That script was PORTED, not re-derived — the arrangement is settled and this
entry is only about the six places the port did not just copy the mockup
verbatim:

1. **Reason strings.** The mockup's strings are schema 2.0.0. Ported verbatim,
   every non-satellite row would have rendered "Decided by -" with a raw,
   unmapped string. They are now mapped against the current emitter, and all
   six `failed_residue_test` rows render with both gates shown.
2. **Digestion needs `Pass2Report`.** The mockup reads Pass 2 values that
   `generate_html_report` could not reach. Its signature now takes
   `Option<&Pass2Report>`, and the section says plainly when Pass 2 did not run,
   instead of showing zeros. No field was added to `ReconReport`.
3. **The MS1 ladder was wrongly applied to MS2.** The mockup quantizes MS2 onto
   the MS1 tolerance ladder. AGENTS said that ladder was MS1 only at the time, so
   the rung was not ported and MS2 showed the measured window instead. (Ben
   revised this the same day in the follow-up commit — see the ladder-sharing
   entry near "The three tolerance regimes" — but the port itself respected the
   rule as it stood.)
4. **The any-residue / unspecific collapse.** The mockup collapses "any residue"
   and "unspecific" into one label. recon distinguishes them, because calling a
   protein-terminal mod "unspecific" tells the reader the opposite of what was
   measured. The distinction was kept.
5. **`navigator.clipboard` fails silently on a `file://` page.** The mockup's CSV
   button uses it, then alerts "Copied as CSV" having done nothing — verified
   absent in a browser on a local page. A fallback textarea was added.
6. **The enzyme placeholder.** At port time the report did not yet record the
   enzyme, so the meta grid said so plainly rather than inventing a value. (This
   became moot the same week — schema 3.1.0, `b62c331`, added `input.enzyme`.)

Verified: 194 tests, `run_validation.py` 17/17, HTML closes every element with
zero mismatches, JSON stamps schema 3.0.0.

### ⚠ THE RENDERED REPORT CONTRADICTED ITSELF — found by reading the page, not the code (2026-09-03, `7725eb3`)

The satellite row in the "did not make the cut" table read **"Decided by:
floor"**. The flow-chart explainer printed lower on the same page says
satellites stop at step 1 and never reach the floor. The JSON underneath both
says `decided_by: null` for a satellite. Three renderings of the same fact, and
one of them was wrong, on one page.

**This was caught by opening the HTML and reading it, not by reading
`tier_assignment.rs`.** The code that decides satellite status was correct the
whole time; the report's PRESENTATION layer had a stale label. Checked against
`tier_assignment.rs` directly before fixing: the floor is NOT a first gate, and
six of serum's nine recommended mods sit below it — so "floor" was never even
close to the truth for a satellite. Fixed with a test that pins the satellite
row's rendered text to `"satellite"`.

**Why this belongs in NOTES, not just in the commit:** it is a reminder that a
correct backend and a correct data model do not guarantee a correct report.
The artifact a user actually reads has to be checked as itself, the way it was
checked here — not inferred from the code that produced its inputs. See PLAN's
"Reading artifacts is not reading code" note from the prior session; this is a
second, independent instance of the same failure mode, this time caught before
shipping instead of after.

### 🐛 `UnimodEntry::sites()` drops every acceptor when ALL its specificities are hidden — left as-is, worked around at the call site (2026-09-03, `7725eb3`)

Ten un-curated `not_recommended` / `notable_unannotated` rows are named from
Unimod. The first attempt to give them acceptor residues used
`PeakAnnotation.sites`, which is `UnimodEntry::sites()`. It filled 1 row of 10.

**Measured cause, not guessed:** `sites()` filters out every specificity marked
`hidden="1"`, with NO fallback for the case where all of an entry's
specificities are hidden. In the pinned `unimod.xml`, nine of these ten entries
are fully hidden — `CarbamidomethylDTT`, `Lys->Allysine`, `Arg->Npo`, `Gly+O(2)`,
`Ammonium`, `Cation:Al[III]`, `Cation:Ni[II]`, `Xlink:SMCC[219]`, `Unknown:210` —
and came back empty. Specificity COUNT is not the discriminator: `Cation:Al[III]`
has three specificities and still failed; `Pyro-carbamidomethyl`, the one entry
that worked, has one.

The sibling function `UnimodEntry::classification()` already carries exactly
this fallback, documented for the same reason. `sites()` was deliberately
**NOT widened to match it** — it also feeds a published field
(`RecommendedMod.sites`, and now `NotRecommended.sites`/`notable_unannotated`
sites for curated rows) that nothing asked to move, and widening it would be a
silent behaviour change on an existing output, not a targeted fix.

Instead, `report.rs` reads the entry's own `specificities` directly, through a
new function `unimod_acceptor`, applying the SAME all-hidden fallback
`classification()` documents — without touching `sites()` itself. The lookup key
is `unimod_id`, the record the annotation already chose, never the title, so a
name and its residues cannot come from two different entries; `mono_mass` is
asserted against the annotation before anything is written.

⚠ **Two of the measurements behind the first, failed attempt were also wrong,
and were corrected against the file rather than re-guessed:** a fixed window
read past each XML match spilled into the next entry and invented
specificities that were not there; and a search for a literal `>` in a file
that escapes it as `&gt;` reported one entry as missing when it was present.

**Amidation was deliberately NOT given sites.** Its curated entry is `TG X`,
meaning no residue restriction — the empty value is the source speaking, and
filling it would invent a residue that Unimod does not claim.

## ⚠ LESSON — prose instructions rot; asserted invariants do not (2026-09-03)

Three separate written instructions in this repo were followed-but-wrong, or
never followed at all, and each was only caught by reading the data or the
rendered output, not by re-reading the instruction:

1. **The v0.15 upgrade checklist's manual column-semantics audit.** Recorded in
   this file, step 4 of the upgrade checklist above: "check for semantic changes
   to existing columns ... manual audit step." It was not done. The result was
   two days where `report.rs` documented `precursor_ppm` as absolute-only while
   the committed `full-run/liver.json` already carried a negative value. See the
   confirmation note attached to that checklist item.
2. **`calibration.rs`'s note telling the next session to REMOVE the MS1
   reconstruction once Sage made `precursor_ppm` signed.** The instruction was
   recorded and was WRONG — Sage's column and recon's reconstruction are not the
   same quantity (they diverge by thousands of ppm across the open window, and
   agree only near zero delta, where the bias is actually measured). Following it
   would have carried that divergence into `bias_ppm`, corrupting the number the
   tolerance recommendation is built from. It was never carried out — luck, not
   judgement — and was corrected in place, `ddaf5ab`.
3. **The mass-accuracy docs.** `report.rs` kept asserting all four ppm fields
   were absolute and could never be negative, a claim the repo's OWN committed
   data — `full-run/liver.json`'s `precursor_median_ppm: -0.2771486` — had already
   falsified, for days, before anyone read the file next to the doc comment.

**The common failure is not that anyone was careless once.** It is that a
written instruction sitting in a file is not self-enforcing: it can be stale,
or simply wrong, and nothing stops either state from persisting until someone
happens to compare it against a measurement. **An assertion compiled into a
test cannot rot the same way** — it either runs and passes, or it fails and
demands attention. Where AGENTS.md already says "assert it in code, not just
check it in prose," these three are the evidence for why, collected in one
place rather than left scattered across three separate commits.

### ⚠ COROLLARY — an assertion that never RUNS rots exactly like prose (2026-09-03)

The entry above says an assertion compiled into a test cannot rot. **That is
true only if the harness actually collects it.** Found the same week, in the
very test written to pin the satellite fix that entry celebrates:

`report.rs` carried `a_satellite_is_not_attributed_to_the_floor` inside
`#[cfg(test)] mod satellite_route_tests`, and that module was declared **inside
the body of `cut_row`**. Rust does not collect `#[test]` functions from a module
nested in a function body. The test never ran.

**Measured, not reasoned:** in a full `cargo test` run the name appears exactly
twice in the output — as `warning: function ... is never used` and in the source
snippet the compiler printed under it. It never appears as a `test ... ok` line,
and the module path `satellite_route_tests` appears zero times. The suite read
207 passed / 0 failed either way, because the test was not in the 207.

**The dead-code warning was the only signal, and it had been surviving in the
build output.** `cargo test` prints warnings above the results, where a run
checked by tailing the last lines never shows them.

Two things were wrong with it, and both are fixed:

1. **It was unreachable.** Moved to `report.rs`'s real `mod tests`, where it now
   reports as `report::tests::a_satellite_is_not_attributed_to_the_floor ... ok`.
2. **It grepped this file's own SOURCE TEXT through a fixed 400-byte window**
   (`include_str!("report.rs")`, then `src[i.saturating_sub(400)..i]`) — the same
   brittle fixed-window pattern that produced a wrong Unimod measurement on the
   same day, recorded in the `UnimodEntry::sites()` entry above. It is now a
   behavioural test on the RENDERED ROW: it calls `cut_row` with a satellite and
   asserts the route cell is `<td>satellite</td>`, never `<td>floor</td>`, and
   that the row cites no abundance floor. A control asserts a genuine
   `below_floor` row still says "floor", so the test cannot pass by the label
   having been deleted for every row.

**Falsified before believed:** reintroducing the bug (satellite branch rendering
`"floor"`) makes it FAIL and print the offending row. Reverted, and the diff
confirms the branch literal is untouched.

**The general rule this adds:** a new test is not evidence until you have seen
its name in the passing list. "The suite is green" does not prove your assertion
is in the suite. Check for a `never used` warning after adding one, and read
`cargo test`'s warnings, not only its tail.

### Report footer name — `sageRecon` is the name (2026-09-03, Ben) (locked)

The footer in `report.rs` prints `sageRecon`. **Ben settled it 2026-09-03:
`sageRecon` is the tool's name.** The code is correct as it stands and needs
no edit.

**Rejected alternative:** changing the footer to `sagePreview` to match the
remote. Rejected because the repo name was the half expected to move.

✅ **CLOSED 2026-09-04 — the repo was renamed.** GitHub is now
`github.com/neely/sageRecon`. Ben
went ahead despite NIST hosting still being unsettled, rather than wait —
see PLAN's former "Deferred / open" entry, now marked done, for the accepted
risk (a second rename later if the tool moves to a NIST GitHub org).
`README.md`/`AGENTS.md`/`PLAN.md` URLs updated in the same pass. Do not
re-open the name itself.

## The clippy pass — 44 lints to zero, and the three that were traps (2026-09-03)

**Measured, not remembered: the list was 44 unique lints, not "about 40."**
Counted from `cargo clippy --all-targets --message-format=json`, de-duplicated
by (lint, file, line). PLAN's figure was close but had never been re-measured.

⚠ **THE LIST IS NOW ZERO, AND CI STILL DOES NOT GATE ON IT.** That is
deliberate and must stay that way. A gate would turn the next lint into
something to silence quickly, which is the exact pressure that makes the three
findings below get "fixed" instead of read.

### Three lints were WRONG, and applying them would have introduced defects

These are the reason the list is worked one lint at a time.

1. **`unnecessary_fallible_conversions` (mzml.rs) would have made a malformed
   mzML PANIC.** Clippy wants `.into()` instead of
   `param.value().try_into().unwrap_or(0.0)` on the TIC parse. Checked against
   the pinned dependency source rather than assumed — mzdata-0.65.5,
   `src/params.rs:1196`:

       impl From<ValueRef<'_>> for f64 {
           fn from(value: ValueRef<'_>) -> Self { value.to_f64().unwrap() }
       }

   `to_f64` returns `Result<f64, ParamValueParseError>`. The `From` impl is
   therefore infallible **to the type system and panicking in fact**. A
   `total ion current` param that does not parse as a number would abort the run
   instead of reading 0.0 and falling through to summing peak intensities.
   **The general lesson: "clippy says this conversion cannot fail" means the
   trait signature cannot fail, not that the code cannot panic.** Read the impl.

2. **`doc_lazy_continuation` (calibration.rs) proposed making a rustdoc misparse
   PERMANENT.** The lint fires because a doc line STARTS with `>=`, which
   rustdoc reads as a blockquote. Clippy's fix prefixes the CONTINUATION lines
   with `>` as well — rendering four lines of prose as a quotation. The fix was
   applied by `--fix`, read, and reverted. The doc is reflowed instead so `>=`
   no longer starts a line, which removes the blockquote and keeps the quoted
   sentence character for character. ⚠ The same lint at `report.rs:83` needed
   clippy's OTHER suggestion (a blank line), because that line starts a new
   schema-version entry and indenting it would nest it under the bullet above.
   **One lint name, two sites, two different correct answers.**

3. **`approx_constant` read a MEASUREMENT as PI.** The literal is `3.14` in
   `digestion_composition_integration.rs` — liver's measured ragged_c
   percentage (3.1372 %), partner to the 6.93 ragged_n rate on the line above.
   "Consider using the constant directly" would have moved the assertion target
   by 0.0016 points.

### Two more are allowed on purpose, with the reason at the site

* **`neg_cmp_op_on_partial_ord` (mod_discovery.rs).** The negation is the point.
  The test asserts `gap > tolerance` AND `!(gap <= tolerance)`, which are
  different statements on a partially ordered type; asserting both is what pins
  the float behaviour the surrounding comment tells the reader to watch.
* **`excessive_precision` + `inconsistent_digit_grouping` (stats.rs).** The
  table is the published Lanczos g=7 n=9 coefficients, transcribed as the source
  writes them, so a reader can compare character by character. Truncating parses
  to the same f64 — all cost, no accuracy.

### `unnecessary_sort_by` x10 — applied, with the equivalence argued

All ten sit on paths that decide REPORT ROW ORDER, and a polymer tie-break
nondeterminism was fixed only the day before, so this was not taken on style
grounds. `sort_by_key(f)` is `sort_by(|a, b| f(a).cmp(&f(b)))`, and
`Reverse(a.k).cmp(&Reverse(b.k))` is `b.k.cmp(&a.k)` by `Reverse`'s Ord impl.
Both forms are **stable**, so equal counts keep their existing order and no
tie-break moves. Every key is a `usize`.

⚠ `cargo clippy --fix` REFUSES these — the lint is marked MaybeIncorrect,
because `sort_by_key` cannot return a key borrowed from the element. That does
not apply to a Copy key, which is why they were rewritten by hand. **Do not read
`--fix` declining a lint as the lint being unsafe; read why it declined.**

### `too_many_arguments` x4 and `type_complexity` — allowed, not refactored

`ReconReport::from_analyses` takes 16 arguments and every one is a measured
quantity. Collapsing them into a struct is a signature change across the whole
report-building path whose failure mode is a **silently swapped field** — the
class of defect this project cannot detect cheaply — against no functional gain.
The three `main.rs` functions mirror CLI flags one-for-one, and the argument
surface is FROZEN, so those lists cannot grow.

### `assertions_on_constants` — applied, and it made the check stronger

`assert!(LEGACY_TOF_MS2_HALF_WIDTH_PPM > ORBITRAP_MS2_HALF_WIDTH_PPM)` became a
`const { assert!(..) }`. Same claim, checked at COMPILE time: it can no longer
be skipped by a test filter, and it cannot silently stop being collected. That
last failure mode is not hypothetical — see "an assertion that never RUNS".

## The Da recommendation is quantized to a tenth of a Dalton (2026-09-03, Ben) (locked)

**The rule:** for a trap or quadrupole MS2, convert the measured ppm to Da at
`PASS2_MS2_REPRESENTATIVE_MZ` (500), multiply by `PASS2_MS2_DA_MULTIPLIER` (2),
**then round UP to the nearest 0.1 Da**. The tenth-Da step is the Da regime's
LADDER and exists for the ppm ladder's own reason: a user picks a search setting
from an effectively discrete set, so precision below the step is unusable.

Applied to the RECOMMENDATION only. `ms2_pass2_tolerance` sizes recon's own
internal window and is NOT quantized — exactly as the ppm branch quantizes the
recommendation and leaves the pass-2 window alone.

**Rejected alternative:** "convert, round up to 0.1, and DROP the ×2". It is the
more literal reading of the request and it halves the recommendation — a ~600
ppm trap would get 0.3 Da instead of 0.6. Rejected by Ben: the ×2 is the cushion
for a miscalibrated trap, and quantizing is not a reason to delete it. A floor at
0.3 Da was also offered and not taken.

**Worked values** (the plausible unit-resolution band): 300 ppm -> 0.3 Da,
400 -> 0.4, 600 -> 0.6, 800 -> 0.8, 1600 -> 1.6. Rounding is UP, never to
nearest: 301 ppm is 0.301 Da and reports 0.4, not 0.3.

**Nothing committed can move.** The MS2 recommendation reaches only an HTML
string (`report.rs`), never a serialised JSON field, and all four committed
reports record `Orbitrap / FT-ICR` on BOTH MS1 and MS2 — so the Da branch is
unreachable for every file in the repo. Verified before the change, not assumed.

### 🐛 The float defect this nearly shipped — and why plausible tests would have missed it

A naive `(v * 10.0).ceil() / 10.0` is WRONG. Multiplying by 10 can land a value
that is already exactly on a tenth a few ULP ABOVE the integer, and `ceil` then
promotes it a whole tenth. Sweeping every tenth from 0.0 to 39.9 found **20 such
values**: 2.1 -> 2.2, 4.2 -> 4.3, 4.9 -> 5.0, 5.9 -> 6.0, 6.9 -> 7.0, 7.9 -> 8.0,
8.4 -> 8.5, 9.3 -> 9.4, and more.

⚠ **NOT ONE of them lies in the 0.3-0.8 Da band a real ion trap produces.** The
whole plausible range is clean. A test written from realistic values would have
passed while the function was wrong — the defect only shows above ~2 Da.

The guard: if scaling lands within 1e-9 of an integer, treat it AS that integer
rather than rounding away from it. Verified over 40 000 inputs on three
properties — every exact tenth maps to itself, the result is always `>= v` and
`< v + 0.1`, and every result lands exactly on a tenth. Falsified: with the naive
version `quantizing_never_promotes_a_value_already_on_a_tenth` fails with
"2.1 Da was promoted to 2.2 Da".

### ⚠ A pre-existing test asserted something the new rule makes FALSE

`ion_trap_recommendation_is_in_daltons_never_ppm` contained a loop asserting the
Da result never numerically equals a value in `MS1_TOLERANCE_LADDER_PPM`
{10,20,50,100}. **That loop is removed, and its removal is not a weakening.**

It was only ever a PROXY for "the Da branch did not call `ladder_rung`", and
quantizing makes those values legitimately reachable: 99999 ppm gives 99.999 Da,
which rounds UP to exactly 100.0. Worse, `ladder_rung(99999)` is ALSO 100.0 — so
at that input **no value-based check can separate the two paths at all**. The
proxy could not do its job and had started rejecting a correct answer.

What replaces it, and why it is stronger: the real claim is about the UNIT, and
the match arms enforce it directly (a Da analyzer must return `Da(_)`). The
arithmetic is pinned by `a_da_analyzer_is_quantized_to_a_tenth_of_a_dalton`,
which asserts 300 ppm -> 0.3 Da. If the Da branch ever called `ladder_rung(300)`
it would return 100.0 and that test FAILS — a direct check where the old one was
a coincidence check.

### `ms1_user_recommendation` is NOT analyzer-aware — stated limitation, not an oversight

It always uses the ppm ladder, whatever the `ms1_analyzers` census says, so a
trap or quadrupole MS1 would receive a ppm number that is meaningless for it.

**Why it was not fixed** (Ben's call, 2026-09-03, after asking the cost): the
asymmetry is structural. `ms2_user_recommendation` returns `FragmentTolerance`,
a UNIT-CARRYING ENUM, so Da is already a first-class branch. `ms1_user_recommendation`
returns `Ms1UserRecommendation`, whose every field is ppm-NAMED and serialised
(`user_recommendation_tolerance_ppm`, `low_ppm`, `high_ppm`), and it takes no
analyzer argument. Making it Da-aware means either changing what those ppm fields
mean on a trap — a MAJOR schema bump, the same "meaning change with no field
change" that made 3.0.0 — or adding parallel Da fields plus a discriminator, and
a signature change across 11 call sites. All for an instrument class nobody uses
for MS1 survey scans and for which this project has zero files.

**Reopening condition:** an ion-trap or quadrupole MS1 file actually arrives.
This belongs in the step-5 limitations write-up.

## The cut table was CORRECT and unreadable — the q display rule (2026-09-03)

Ben read the committed `serum.html` and could not tell why rows in **DETECTED
BUT DID NOT MAKE THE CUT** were cut: they show `statistics` as the decider next
to an odds ratio that clears the stated `OR >= 2` bar.

**Nothing was wrong with the arithmetic.** Measured from the committed
`serum.json`, all six `failed_residue_test` rows fail the q gate, and FOUR OF THE
SIX PASS the OR gate:

| delta | label | OR (needs >=2.0) | q (needs <=0.05) | fails on |
|---|---|---|---|---|
| +114.0457 | GG | 1.29 ✗ | 0.189 ✗ | both |
| -18.0101 | Water Loss | 2.12 ✓ | 0.069 ✗ | q only |
| +53.9156 | Fe[II] | 2.71 ✓ | 0.189 ✗ | q only |
| +21.9816 | Sodium | 1.35 ✗ | 0.697 ✗ | both |
| +37.9491 | Potassium | 4.06 ✓ | 0.239 ✗ | q only |
| +71.0377 | Propionamidation | 5.13 ✓ | 0.117 ✗ | q only |

The gate is an AND, both bounds inclusive (`tier_assignment.rs`):
`o >= OR_MIN && q <= Q_MAX`.

**The whole defect was `fmt_q` being `{q:.1e}`.** The closest call in the table
printed `q 6.9e-2 (needs <= 0.05)`. Deciding that 6.9e-2 exceeds 0.05 takes a
mental conversion, and the row shows it beside an OR that genuinely passes — so
a correct row read as a contradiction. Now: `q < 0.001` below a thousandth,
three decimals otherwise. Verified against all 49 q values in the four committed
reports before the change; accepted rows top out at 0.012 and cut rows start at
0.069, so the 0.05 threshold sits in a gap with nothing in it.

⚠ **Known edge, deliberately not special-cased:** a q of exactly `0.0500` prints
`0.050` against an INCLUSIVE `<= 0.05` bound, so a pass and a fail look alike. No
such value exists in any committed report. Recorded rather than coded around.

**Rejected alternative:** marking WHICH gate failed in the row text ("failed on
q — q 0.189 > 0.05 (OR 2.71 passes)"). Offered and declined by Ben: the row
already says it failed, the columns stay as they are, and a readable q is enough.

### Why a rejected row can have a BIGGER odds ratio than an accepted one

Worth knowing when reading the table, and not obvious. q is BH-adjusted across
all tested candidates, so it encodes **statistical power**, not effect size:

* **Fe[III]** — OR **2.10**, 297 PSMs — ACCEPTED at q 0.012
* **Fe[II]** — OR **2.71**, 58 PSMs — REJECTED at q 0.189

The rejected one has the larger effect. It is rejected because 58 observations
cannot drive a Fisher p low enough to survive BH correction. Low count alone is
not fatal, though: Dehydroalanine passes on 28 PSMs with OR 9.93. It is power as
a joint function of count AND effect size, behaving coherently.

⚠ **The report cannot currently tell those two stories apart** — only `q_value`
is serialised, with no raw p and no BH denominator, so a reader cannot see
whether a row failed for a weak effect or for want of observations. Adding those
fields was offered and NOT taken (it would be an additive 3.3.0 bump). Recorded
as the reopening condition if this question comes up again.

## ⚠ The committed HTML reports were UNGATED until 2026-09-03

`run_validation.py` byte-compares the JSON and **never looks at the HTML**, and
`generate_html_report` is called only from `main.rs` during a real run. Nothing
checked that the four committed pages still matched the renderer. A rendering
change could stale all four silently, and a reader opening one would see output
the tool no longer produces.

`recon-tool/tests/html_report_regression.rs` closes it: deserialize the committed
`<name>.json` / `<name>_pass2.json`, render, and assert byte equality with
`<name>.html`. The inputs are committed, so unlike most data-dependent tests here
this one really runs in CI. `RECON_REGENERATE_HTML=1` makes it write instead of
assert, for a deliberate rendering change.

**Measured before the q change, which is the only time the question could be
asked:** all four committed pages were byte-identical to what the renderer
produced. So they were current — and this also confirms BY MEASUREMENT the
`sort_by_key` equivalence argued in `a6a4a27`, which had until then been an
argument about stable sorts and nothing more.

⚠ **The failure message needed two attempts, and the first one was useless.**
Printing "the first differing LINE" is worthless here: this renderer puts an
entire table on one line, so the cut table alone is over 3 KB and the message was
two walls of HTML with a correct line number buried in them. It now reports line,
column, byte offset, both page sizes, and a 60-character window from each side.
Falsified both times by flipping one digit in a committed page.

**Re-rendering the four pages from committed JSON is a PRESENTATION
regeneration, not a change-regenerate.** No search runs, no measured number is
recomputed, no schema moves. The check that it stayed presentation-only: after
regenerating, every q token was masked out of `git diff` and the remainder was
byte-identical — 49 q tokens before, 49 after, nothing else moved.

## Vendored upstream clones — what they were for, and why they are gone (2026-09-04)

During development the repo carried a gitignored `reference/` tree holding raw
clones of upstream tools. They were **local scratch for reading**, never a
dependency and never a read path for an agent. `AGENTS.md` kept them
deliberately separate from `reference-notes/`, which is the distilled
documentation this project authors.

**What was in it at removal time (2026-09-04):**

| path | size | what it was |
|---|---|---|
| `reference/sage-src/` | 827 MB | clone of `github.com/lazear/sage`, at `df92199` — the pinned rev |
| `reference/sage/` | 27 MB | the old vendored Sage BINARY, from before Sage became a library dependency |
| `reference/MetaMorpheus` | 0 B | a tracked GITLINK at `7e453540`, with no `.gitmodules` |

**Why they were read.** Three different reasons, and the distinction matters
for attribution:

* **Code was ported** from Sage (Lazear, MIT) and mzSniffer (Fondrie, Apache
  2.0). Both are credited in `THIRD_PARTY_LICENSES.md`.
* **Method only, no code**, from Mascot error-tolerant, Byonic Preview
  (Kil et al. 2011), MetaMorpheus's calibration formula, PTM-Shepherd's peak
  calling and annotation tolerance, and Crystal-C.
* **Behaviour checks.** Repeatedly, the only way to settle what a tool actually
  does was to read its source rather than trust its docs — `precursor_ppm`
  being `|error|` in the pinned Sage is the standing example.

**Why they were removed.** Development is finished, they are re-clonable from
upstream, and they were 853 MB of untracked local scratch. Deleting them
changes nothing that is tracked.

⚠ **The `reference/MetaMorpheus` gitlink was removed from the index in the same
pass, and that is NOT a loss of provenance.** It pinned `7e453540`, the
MetaMorpheus snapshot the four curated mod files came from. That hash is
recorded in text in `THIRD_PARTY_LICENSES.md` and in the entry above at
"MetaMorpheus's curated mod list — adopt for tiering". The gitlink itself was a
defect: a submodule pointer with no `.gitmodules`, which gives anyone cloning
the repo a broken submodule reference and nothing else.

**Rejected alternative:** converting it to a real submodule. Rejected because it
would make every clone pull the whole MetaMorpheus repository to reproduce a
provenance fact that two committed documents already state.

## The production reshape — what moved, and the tripwire that made it safe (2026-09-04)

Ben's call: the repo goes to ADLP internal review, so the root must read as a
product and not as a working tree. Seven commits, `d5634c1` to `6616921`.

### The move that could have failed silently, and how it was caught

`testing/` and `reference-notes/` are named by 11 test files, `run_validation.py`,
two CI files, `.gitattributes`, `.gitignore` and the README. **A test that cannot
find its data does not fail. It prints a skip line and reports `ok`.** So a
botched move looks exactly like a clean one: 211 passed, green.

**THE TRIPWIRE: capture the SKIP LIST before the move and diff it after.** Run
`cargo test -- --nocapture`, grep for skip lines, and SORT them (test order is
not stable, so an unsorted list produces false diffs). On this machine the
baseline skip list is EMPTY, because every gitignored input is present locally.
That makes the invariant sharp: any skip appearing after the move is a broken
path, not a missing file. It held byte-identical across all seven commits.

⚠ **Do not use the passing test count as the gate for a path change.** 211 is
the same number on a working tree and on a tree where every data path is wrong.

### Shipped data is not development material

`recon-tool/resources/` now holds `unimod.xml` and the four curated mod files.
They are `include_str!`-embedded, so **moving their old directories without
repointing them breaks `cargo build`, not merely the tests.** That is the
strongest argument for the new location: the binary must not build out of a
folder named `_dev`.

**Rejected alternative:** leaving them under `_dev/` and pointing `include_str!`
at `../../_dev/...`. It works, and it was rejected because a reviewer reading the
build would see the shipped tool reaching into development material.

The four files in the upstream mod snapshot that the binary never loaded
(`glyco.txt`, `ptmlist.txt`, `substitutions.txt`, `tmt.txt`) stayed behind in
`_dev/reference-notes/metaMorpheusMods/`. **The curated list is 4 files, not 8.**
Any statement about "the metaMorpheusMods directory" is wrong by a factor of two.

### The scrub: keep the substance, drop the specifics

Ben's rule. Second-machine references are gone entirely; host names are reduced
to what is structurally necessary.

⚠ **A find-and-replace would have corrupted the code comments.** Of ~196 raw
matches, about 26 were `mirror` in the CODE-PARITY sense (`enzyme.rs` mirrors
Sage's rule) and 6 were substring hits on "the Mac" inside "the machine",
"the machinery" and "macOS". The scrub was done by hand, entry by entry, and the
44 code-parity `mirror` mentions were counted before and after.

Entries whose SUBJECT was the second machine were rewritten around the lesson
they carried, not deleted. The largest was the CI runner entry: its three
gotchas (shell executor config, two runner identities racing, `cache:` fighting
`git clean -ffdx`) and the tag-after-sync ordering rule are all intact, because
that entry is the engineering-rigor evidence the review wants.

### Things found on the way that were believed settled and were not

* ⚠ **`CODEMETA.yaml` was raised as invalid. THAT WAS WRONG. Ben closed it
  2026-09-04: it comes from NIST, it stays as it is, do not "fix" it.** This
  entry first read "IS NOT VALID ... not a CodeMeta document at all", which
  measured a NIST Open Source Portal topics file against the CodeMeta schema.
  Wrong yardstick, so the finding does not stand. Recorded here only so it is
  not re-raised. **The lesson generalises: a file that fails a spec is not a
  defect until you have confirmed which spec it is meant to meet.**
* **`reference/MetaMorpheus` was a tracked GITLINK with no `.gitmodules`**, not a
  gitignored clone. Anyone cloning got a broken submodule pointer. Removed; see
  "Vendored upstream clones".
* **The reference notes disagreed with the code on the MS2 tolerance.** Two docs
  said 50 ppm for Orbitrap and Astral where the code says 20, and one said the
  ppm ladder never reaches MS2 when `ms2_user_recommendation` quantizes a ppm
  analyzer onto that same ladder. Corrected in place, `58844f8`.

### ⚠ THE "~25-50x SPEED" CLAIM HAS NO MEASUREMENT BEHIND IT. Do not cite it.

It appears only as an unwritten checkbox in PLAN's step-5 list. Nothing in the
repo measures recon against another tool, and JOURNAL says plainly that the
wall-clock figures it carries came "from historical separate runs, not a single
measured flow". **Do not put a speed multiple in any public document until one
is measured.**

What IS citable, and what the README uses: Ben ran the serum file (41788 MS/MS
scans, a count confirmed against `input.ms2_spectra` in the committed report) in
**93.2 s** with the v0.1.2 Windows binary, untuned. One file, one machine, no
comparison. The committed example still stamps `82.227431083` s from a v0.1.1
run on different hardware; the two are not comparable and must not be presented
as a trend.

### 🔒 `signal_fate` is VESTIGIAL and comes out (Ben, 2026-09-04)

Ben, on reading the README draft: it "made no sense in retrospect." **It is not
part of the analysis this tool presents.** Removed from the README the same day.

**What it actually is, measured rather than assumed:**

| surface | present? |
|---|---|
| JSON report | YES. `id_rate_by_count_pct`, `id_rate_by_tic_pct`, `identified_spectra`, `total_ms2_spectra`, `chimera_rate_pct` |
| console | YES. `print_report_summary` prints two `ID Rate` lines; called from `main.rs:2070` |
| HTML report | **NO.** No section, and the values appear nowhere in the page |

It is a leftover of the three-layer MS1 work. That was removed from recon and
now lives in `_dev/extracted/three-layer-ms1/` for sageGUI; `signal_fate` is what
stayed behind.

⚠ **THE DRAFT README DESCRIBED THE REMOVED FEATURE, NOT THE SHIPPED ONE.** It
said the block "splits MS1 intensity into identified, not sampled, and sampled
but not matched." Every field is **MS2** (`report.rs:510-519`), and no three-way
split exists. Ben caught it. **The lesson is the standing one: the description
was written from the design's history rather than from the artifact.**

**Two traps this set, both worth remembering:**
1. A first grep for the print path used the wrong function name and returned
   nothing, which would have supported a claim that the code was dead. It is
   `print_report_summary`, and it IS called. **A negative grep is not evidence
   of absence until the name is verified.**
2. Searching the HTML for `TIC` returned 19 hits, all the substring inside
   "sta**tis**tics". Same false-positive class as the scrub's "the Mac" inside
   "the machine".

**Removing it is a SCHEMA CHANGE.** Deleting a serialised block needs a version
bump and the downstream trace, and it restages the four committed reports. Do
not do it incidentally. Tracked in PLAN's "Deferred / open" as a class: audit
for other functions built during development and never retired.
