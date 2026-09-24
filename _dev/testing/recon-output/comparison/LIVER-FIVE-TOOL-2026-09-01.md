# Liver — five-tool comparison: digestion, mass tolerance, modifications

Generated 2026-09-24 by `testing/scripts/liver_5way_report.py`, which RUNS the per-section producers and composes their output. It recomputes nothing.

**recon numbers are Sage `0.15.0-beta.2`, report schema `4.0.0`.** ⚠ Every recon number below is search-engine specific. The Sage version is part of the citation, not a footnote — the v0.14.7 -> v0.15.0-beta.2 upgrade moved most of them.

⚠ **The three sections deliberately use different tool sets.** PTMs have five tools; digestion has four, because Mascot ships no peptide list to reclassify; mass tolerance has two, because only recon and Preview publish a comparable quantity. A blank is 'this tool does not report it', never 'zero'.


---

## 1. Digestion — four tools, one classifier

Every tool's peptide list is reclassified with **recon's own terminus rule**. No tool's own class columns are read: they do not share a convention. Preview's row is quoted from `result_summary.html` because its peptide list is not shipped.

```
source                      peptides     den   missed cl   ragged-N   ragged-C   nontryp    N:C
-----------------------------------------------------------------------------------------------
recon Pass 2 (semi)            10697   10697      17.58%      6.95%      3.17%     0.00%   2.19
PTM-Shepherd open (FULLY)      15998   15954      19.64%      1.64%      0.51%     0.02%   3.18
MetaMorpheus (FULLY)           15256   15253      17.84%      0.54%      0.45%     0.01%   1.21
Byonic Preview v3.2.0           2009    2008      15.90%      8.60%      1.30%     0.10%   6.62

⚠ PTM-Shepherd open (FULLY): 41 peptides unresolved (accession absent from this FASTA, or peptide not in that protein) — EXCLUDED
⚠ MetaMorpheus (FULLY): 1 peptides unresolved (accession absent from this FASTA, or peptide not in that protein) — EXCLUDED

Preview row is quoted from result_summary.html (see the vendored README),
not recomputed: its peptide list is not shipped.
```

---

## 2. Mass tolerance — recon vs Byonic Preview

Preview's figures are **parsed from its own `result_summary.html`**, not copied from NOTES. Pre-recalibration is the comparable side: recon reports what the instrument delivered and does not recalibrate spectra.

| quantity | recon | Preview (pre-recal) | agreement |
|---|---|---|---|
| MS2 median \|error\| (recon: all kept PSMs) | **3.3820 ppm** | **3.5 ppm** | 0.12 ppm |
| MS2 signed median | not produced, by design | **-3.1 ppm** | recon has no signed MS2 number; Preview supplies the reference |
| MS1 signed median | **-1.4076 ppm** | **+0.0 ppm** | ⚠ **1.41 ppm apart — UNRESOLVED** |
| MS1 median \|error\| | not reported separately | 0.5 ppm | |

Preview's own directional counts: fragments **307 high / 9820 low** (a 32:1 imbalance, which is not compatible with a centred distribution and corroborates its signed MS2 value); precursors **943 high / 850 low** (balanced).

✅ **recon's MS2 accuracy is corroborated by an independent tool on the same raw file, same quantity.**

⚠ **The MS1 disagreement is NOT resolved and must not be presented as corroborated.** Untested candidate causes: Preview measured the `.mgf` after its own conversion; the populations differ by an order of magnitude (recon's clean subset is 7177 PSMs against Preview's ~1793 precursors); and the peptide sets are not the same.

### What recon recommends from this

* MS1 search window: **±10 ppm** (quantized ladder rung; the raw requirement is 4.541 ppm).
* Pass-2 MS1 window actually used: -5.949 .. +3.134 ppm.
* MS2: measured |error| 3.2684 ppm, spread 0.3281 ppm MAD.
  ⚠ `ms2_tolerance_*` = -1.1396 .. +1.1396 ppm is a SPREAD ABOUT THE MEDIAN, **not a search window**. A closed search wants roughly ±5 ppm here, not ±1.1.


---

## 3. Modifications — five tools

## Do recon's recommendations line up?

| # | delta | recon calls it | Preview | PTM-Shepherd | Mascot | MetaMorpheus | corroborated by |
|---|---|---|---|---|---|---|---|
| 1 | +15.9945 | variable — Oxidation on M (Common Variable) | 20.85 | 13.67 | 39.24 | 11.67 | **4/4** |
| 2 | +57.0207 | **FIXED** — Carbamidomethyl on C (Common Fixed) | 2.34 | 8.58 | 21.22 | 8.27 | **4/4** |
| 3 | +0.9833 | variable — Deamidation (Common Artifact) | 8.89 | 2.35 | 5.88 | 3.05 | **4/4** |
| 4 | -17.0260 | variable — Gln->pyro-Glu (Common Biological) | 2.29 | 0.86 | 1.82 | 0.44 | **4/4** |
| 5 | +52.9105 | variable — Fe[III] (Metal) | — | 0.54 | — | 0.69 | **2/4** |
| 6 | +47.9844 | variable — Trioxidation (Less Common) | 0.52 | 0.68 | 0.87 | — | **3/4** |
| 7 | -89.0303 | variable — Met-loss+Acetylation (Common Biological) | — | — | 0.17 | — | **1/4** |
| 8 | -33.9890 | variable — Dehydroalanine (Less Common) | — | 0.19 | 0.45 | — | **2/4** |
| 9 | +27.9952 | variable — Formylation (Common Biological) | 0.19 | 0.15 | 0.39 | 0.12 | **4/4** |
| 10 | -18.0104 | variable — Water Loss (Glu->pyro-Glu) (Common Artifact) | 0.41 | 0.10 | 0.34 | 0.05 | **4/4** |
| 11 | +42.0079 | variable — Acetylation (Common Biological) | 0.67 | 0.57 | 1.12 | 0.93 | **4/4** |
| 12 | +3.9989 | variable — Oxidation to Kynurenine (Less Common) | — | 0.07 | 0.22 | — | **2/4** |
| 13 | -32.0074 | variable — Oxidation and then loss of oxidized M side chain (Less Common) | — | — | — | — | **0/4** |

### recon's tail — the biggest peaks it declined to recommend

| delta | recon PSMs | reason | Preview | PTM-Shepherd | Mascot | MetaMorpheus |
|---|---|---|---|---|---|---|
| +162.0522 | 310 | below_floor_uncurated | — | 1.17 | 1.54 | — |
| +16.9974 | 280 | satellite (+1 C13 of +15.9945) | — | — | 0.50 | — |
| +73.0157 | 259 | below_floor_uncurated | — | — | — | 0.09 |
| +31.9894 | 235 | below_floor — Dioxidation | 0.97 | 1.72 | 0.57 | 0.89 |
| +58.0224 | 200 | below_floor_uncurated | — | 0.40 | 0.03 | — |
| -1.0294 | 161 | below_floor_uncurated — Lys->Allysine | — | 0.26 | 0.11 | 0.04 |
| +114.0428 | 146 | failed_residue_test — GG (Ubiquitination Site) | 2.34 | 1.22 | 0.18 | 0.07 |
| -0.9596 | 123 | below_floor_uncurated | — | 0.48 | 0.12 | — |
| +23.9566 | 86 | below_floor_uncurated — Cation:Al[III] | — | 0.37 | — | — |
| +74.0188 | 78 | below_floor_uncurated | — | — | 0.13 | — |

---

## Caveats that travel with these numbers

* **+57 is three discoveries and one assumption.** Preview's cysteine +57 was an operator preset, not a finding, so it is not a fourth independent corroboration of the alkylation call.
* **-89.0302 Met-loss+Acetylation is SINGLE-SOURCE** (Mascot only) despite a large odds ratio. Label it as such wherever the recommendation set is quoted.
* **Oxidation on proline is not recommended, and all four other tools report it.** That is a design consequence, not a gap: `peptide_hits` is a CONTAINMENT test, proline sits in 56.0 % of liver peptides, and the candidate scores OR 1.34 against a threshold of 2.0. See NOTES "`peptide_hits` IS A CONTAINMENT TEST".
* **Preview's `(-fixed mod)` rows are offsets from its fixed +57 on cysteine**, not absolute deltas, and its `VariableMods.txt` repeats one group total across site rows. Both conventions are verified against Preview's own detail text.

