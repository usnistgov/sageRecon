# Deamidation Detection & Wide-Tolerance Search Notes

**Source:** Dr. Phillip Wilmarth (OHSU Proteomics Shared Resource), *Detecting Deamidation Guide*, Aug 18 2026 — vendored/digested from `Deamidation-how-to-guide_20260802-2.pptx` and accompanying README for internal repo reference.

This is a condensed, action-oriented digest of the source deck, focused on the parts most relevant to **using wide precursor-tolerance searches to compute mass deltas and to predict/validate PTMs** (deamidation as the worked example, but the logic generalizes).

---

## 1. Core Idea: Wide-Tolerance Search + Delta Mass Histograms

The central technique is to run the initial search engine (Comet) pass with:

- **No variable modifications** specified.
- **Wide precursor mass tolerance**: ±1.25 Da (monoisotopic), not a PPM-based narrow window.
- Fully tryptic peptides, minimally redundant FASTA (one protein/gene canonical UniProt reference), sensible fragment tolerance for the analyzer used.

For every PSM, compute:

**delta mass = experimental precursor mass − theoretical mass of the top-scoring assigned sequence**

Because the search is unconstrained by modifications, delta mass becomes a diagnostic variable rather than a search filter. Plot delta mass histograms **separately per charge state (2+, 3+, 4+)**, target (blue) vs. decoy (red), each with a full-range panel (−1.25 to +1.25 Da) plus zoomed panels around 0 Da and 1 Da.

**Why narrow (PPM) tolerance searches defeat this approach:** in a 10-ppm search, every match — correct or incorrect — will trivially have an "accurate" mass, so mass accuracy can no longer discriminate correct from incorrect PSMs. Wide-tolerance search preserves delta mass as a real signal, and lets target/decoy logic be applied directly to the delta mass histogram itself (not just to score distributions).

---

## 2. Anatomy of the Delta Mass Histogram

Three narrow peaks show up in target matches, riding on a roughly uniform "noise" floor that matches the decoy distribution:

| Peak location | Interpretation |
|---|---|
| 0 Da | Correct sequence, unmodified, correct monoisotopic mass call |
| 1.003 Da | Correct sequence, unmodified, but the M1 isotope peak (extra ¹³C neutron) was mistaken for M0 |
| 0.984 Da | Correct sequence with a single deamidated N or Q (amide → acid mass shift) |

Key numeric facts to hard-code into any tooling:

- ¹³C isotope spacing: **+1.003 Da** per extra neutron (M0/M1/M2...).
- Deamidation mass shift: **+0.984 Da** (N→D or Q→E, isobaric across isomerized/racemized product forms).
- The two are separated by only **19 mDa** — resolving them requires high resolution (≥120K baseline-resolves the doublet; 60K blurs it, especially at 3+/4+).
- Decoys distribute roughly uniformly across the full ±1.25 Da window; incorrect target matches coincide with that decoy distribution. Only the narrow peaks (0, 0.984, 1.003) are enriched for correct matches.

**Practical read for tooling:** if you're auto-detecting PTM mass-shift peaks from delta-mass histograms, don't just look for a peak — check that (a) it's narrow relative to the decoy floor, (b) its width scales with charge state/resolution as expected, and (c) the blue/red (target/decoy) ratio in that bin is elevated relative to background.

---

## 3. Workflow for PTM Prediction via Delta-Mass Region Splitting

1. **First-pass search**, no mods, wide tolerance (±1.25 Da) → get delta mass histograms per charge state.
2. **Interactively define 3 windows** on the histogram (0-Da, 0.984-Da, 1.003-Da), per charge state. Assign every PSM to a region purely by delta mass — ignore search score/target-decoy at this stage.
3. **Split the raw data** into 3 subsets (treat like pseudo-fractions) and write filtered spectrum files per subset.
4. **Second-pass search per subset**, now with the info from step 2 baked in:
   - 0-Da subset → no mods search, FDR-filter normally.
   - 1.003-Da subset → no mods search (still unmodified peptides, just M1-called), FDR-filter normally.
   - 0.984-Da subset → allow variable deamidation (**max 1 per peptide** — see Section 4 for why the max matters), FDR-filter, then also get site localization.
5. **Recombine the 3 FDR-filtered subsets** for protein inference and PTM/PTM-site reporting.

This effectively turns "detect deamidation" into a **delta-mass-conditioned two-pass search design**, rather than trusting a single search with deamidation as a variable mod.

---

## 4. Critical Pitfall: "Just Allow More PTMs" Backfires

This is arguably the most important cautionary finding for any automated PTM/mass-delta prediction pipeline you build.

Naive assumption: search engines have long since solved variable-mod searching, so just let Comet allow up to 3 deamidations per peptide (its default) instead of capping at 1.

**Observed result when max deamidations was raised from 1 → 3 (on the 0.984-Da filtered subset):**

- A **new incorrect peak appears at −0.984 Da**, strongly **biased toward target matches** (not decoys) — worse at higher charge states.
- Apparent noise in the 0.984-Da region *drops* (looks like an improvement) — but this is an artifact: matches are migrating to −0.984 Da, not disappearing.
- Correct 0-Da PSMs are lost/reassigned into the −0.984-Da region at higher charge states (4+: ~150 counts → ~100 counts at the 0-Da peak top).
- Repeating this experiment on the 1.003-Da (M1-mistaken) subset produced excess target-biased peaks at **+0.019 Da** (1.003 − 0.984) and at **−0.965 Da**, again worsening with charge state, with decoys effectively vanishing from the expected 1.003-Da region.

**Interpretation:** giving the search engine more degrees of freedom (more allowed PTM copies) opens scoring space for *implausible* peptide/PTM combinations that happen to score best — and these improbable matches are strongly target-biased, meaning **standard target/decoy FDR estimation breaks down** for them (decoys don't populate the same artifact peaks, so the FDR estimate for that region is wrong). This isn't unique to deamidation — any workflow with multiple PTM copies or multiple PTM types per peptide expands scoring space in ways that undermine simple target/decoy error control.

**Actionable rule for your repo/tooling:** when configuring or evaluating search engines for PTM/mass-delta workflows, treat "max modifications per peptide" as a parameter that must be empirically stress-tested against delta-mass histograms (does it introduce new target-biased peaks?), not just set to a permissive default. Prefer the minimal number of variable PTM copies actually justified by the delta-mass-region split (e.g., 1, informed by which delta-mass bin the PSM came from) over blanket "allow several" defaults.

---

## 5. Confounders Worth Encoding as Checks in Any Pipeline

- **MS survey scans vs. MS/MS scans are decoupled measurements.** Precursor isotope/monoisotopic-mass calling happens on survey scans (different resolution/AGC/fill-time/centroid settings) while PTM site ID comes from MS/MS fragment ions. A pipeline shouldn't assume consistency between the two without checking.
- **Isolation window width** means multiple isotopologues are often co-isolated for 3+/4+ ions, so a wrong M1-as-M0 call rarely hurts the underlying MS/MS match quality — it just miscalls the delta mass, and wide-tolerance search self-corrects this via the 1.003-Da region rather than requiring a modification setting.
- **Resolution and charge state jointly determine separability** of the 0.984/1.003 doublet: 120K resolution baseline-resolves it; 60K blurs it increasingly with charge state (2+ cleanest, 4+ worst).
- **Score ties / multiple top hits per scan**: when several sequences tie in score, pipelines (e.g., PAW) pick one at random — this alone introduces some cross-window "leakage" between passes and should be expected as background noise, not a bug.
- **Retention time as an orthogonal check**: deamidated (acidic) peptides elute slightly later than amidated forms on low-pH C18 RP; multiple deamidation reaction products (isomers/racemates) elute at slightly different times, producing complex/split chromatographic peaks — useful as a secondary confirmation signal beyond mass alone.
- **DIA and TOF are flagged as poor choices for this kind of PTM work** — DIA especially, because it isn't a clean one-precursor-per-spectrum measurement, and TOF resolution/accuracy generally isn't sufficient to resolve the 19 mDa doublet.

---

## 6. Instrument/Experiment Design Recommendations (for generating usable delta-mass data)

- DDA (not DIA) acquisition, bottom-up, trypsin digestion, fully tryptic search space.
- Precursor (MS1) resolution ideally **120K**; fragmentation mode/mass analyzer choice is not critical.
- 90–120 min single-shot gradients minimum if not fractionating; fractionate when possible since PTMs are substoichiometric.
- Maximize sample load; check/optimize gradients.
- Charge-state-stratified analysis (2+, 3+, 4+ separately) is mandatory — resolution, monoisotopic-calling error rates, and peptide length/N-Q content all vary systematically with charge state.

---

## 7. Why No Mainstream Pipeline Currently Does This

The deck explicitly claims MaxQuant/Perseus, MSFragger/FragPipe, TPP, Scaffold, Mascot, OpenMS, PeptideShaker, etc. cannot replicate this workflow because it requires: interactive/visual delta-mass histogram inspection, manual or validated definition of delta-mass windows per charge state, splitting raw data into delta-mass-defined subsets, running conditional second-pass searches per subset, independent FDR control per subset, and recombination for protein inference + PTM reporting. This is implemented in the open-source **PAW pipeline** (Python scripts around the Comet search engine): https://github.com/pwilmart/PAW_pipeline. Reference blog post on wide precursor tolerance rationale: https://pwilmart.github.io/blog/2021/04/22/Parent-ion-tolerance

**Takeaway for building your own tooling:** if you want to programmatically predict PTMs from mass deltas, the minimum viable feature set is (1) wide-tolerance search support, (2) per-charge-state delta mass histogramming with target/decoy overlay, (3) programmatic/interactive window definition and data splitting by delta-mass region, (4) per-region second-pass search with region-appropriate, minimally permissive variable-mod settings, (5) explicit stress-testing of how increasing allowed-PTM-copy settings shifts the histogram (watch for new target-biased peaks), and (6) recombination logic for downstream inference — none of which exist off-the-shelf in mainstream GUI pipelines.

---

## Key Numeric Reference Table

| Quantity | Value | Note |
|---|---|---|
| ¹³C isotope spacing (M0→M1) | +1.003 Da | dominant isotope effect in peptides |
| Deamidation mass shift (N→D, Q→E) | +0.984 Da | isobaric across isomer/racemate products |
| Deamidation vs. M1 isotope separation | 19 mDa | requires ~120K resolution to baseline-resolve |
| Recommended wide precursor tolerance | ±1.25 Da | monoisotopic mass window for first-pass search |
| Recommended MS1 resolution | 120,000 | 60,000 blurs the 0.984/1.003 doublet, worse at higher z |
| Max deamidations/peptide (safe default) | 1 | raising to 3 (Comet default) introduces target-biased artifact peaks at −0.984 Da (and −0.965/+0.019 Da for the M1 subset) |

---

*Digested from the OHSU Proteomics Shared Resource deamidation how-to guide (README.md + Deamidation-how-to-guide_20260802-2.pptx) for internal repo reference. See original files for full slide-by-slide detail, figures, and citations.*
