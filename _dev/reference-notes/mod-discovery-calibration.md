# Reference Notes: Open Search Delta Mass — Calibration, Isotope Folding, and Peak Detection

**Purpose:** Background and rationale for the Phase 8 mod discovery fixes. Read this to understand *why* the pipeline was changed. For *what* to change, see `PLAN-PHASE8-MOD-DISCOVERY-FIX.md`.

---

## Why the isotope correction was a no-op on open-search data

Sage's `isotope_error` column in `results.sage.tsv` is populated only when the `isotope_errors` config parameter is set and the search is narrow enough that isotope disambiguation is meaningful. In a Da-tolerance open search (e.g., `precursor_tol: {"da": [-500, 100]}`), a precursor selected from the M+1 isotope peak simply scores as a match with Δm = +1.003 Da — the engine doesn't try to correct it, because with a 500 Da window every precursor finds its best match regardless of isotope state. The `isotope_error` column is 0.0 for essentially every row in open-search output.

The Phase 2 validation that appeared to confirm isotope correction was run on `testing/narrow-search-params.json` with `isotope_errors: [-1, 3]` explicitly set. That config populates the column. The open-search config does not. The correction formula `corrected_delta = (expmass - calcmass) - isotope_error * NEUTRON` is mathematically correct but operates on a column of zeros, making it a no-op on the mod-discovery data path.

**The fix:** Don't try to correct per-PSM in open search. Instead, after histogram construction, fold bins that fall at integer multiples of the ¹³C−¹²C spacing (1.003355 Da) back into the Δ=0 bin or the nearest real peak. This is what PTM-Shepherd does.

---

## The NEUTRON constant error

The codebase uses `NEUTRON = 1.0086649158849`, which is the rest mass of the free neutron. This is **wrong** for isotope envelope calculations.

The correct value for isotope peak spacing in peptide mass spectrometry is the **¹³C − ¹²C mass difference**:

```
¹³C mass:  13.003354835 Da
¹²C mass:  12.000000000 Da
Difference: 1.003354835 Da
```

The error is 5.31 mDa per isotope step. For `isotope_error = 2`, the overcorrection is 10.6 mDa; for `isotope_error = 3`, it is 15.9 mDa. This produces a negative skew in the corrected delta mass distribution — PSMs where Sage assigned `isotope_error = 2` or `3` end up at −0.011 or −0.016 Da instead of 0.000 Da. The Phase 2 narrow-search validation showed exactly this: −0.01 Da at 11.9% and −0.02 Da at 3.2%, but +0.01 Da at only 2.9%.

The same constant is used in `mzml.rs` for MS1 isotope envelope intensity summation (M0/M1/M2 spacing). Both must be updated.

**Correct constant name:** `C13_C12_DIFF = 1.003354835`. Keep `NEUTRON` if it's used elsewhere for different physics, but never use it for isotope peak spacing.

**Validation:** Re-run the narrow-search test after the fix. The tail should symmetrize: approximately equal counts at +0.01 Da and −0.01 Da.

---

## Why mass calibration is necessary

Even after neutron folding, the Δ=0 population in an open search is smeared across ±0.05–0.1 Da due to:
1. Residual instrument mass error (systematic offset from the calibration state at acquisition time)
2. The now-corrected 5.3 mDa overcorrection (partially; fixed by constant change but instrument drift remains)
3. Temperature-dependent mass drift over long LC-MS/MS gradients

Without calibration, the histogram merge tolerance must be set wide (0.02 Da) to capture the smeared Δ=0 population as one peak. But 0.02 Da is also wide enough to bridge deamidation (+0.984 Da) and its nearest neighbor, and to merge legitimate shoulder peaks. The calibration step removes the systematic offset, allowing the merge tolerance to be tightened to 0.01 Da.

**Implementation:** Collect all PSMs with |Δm| < 0.1 Da (the unmodified population). Compute the intensity-weighted median of their Δm values — this is the `apex_offset`. Subtract it from every PSM's Δm before binning. Store it in the QC output.

**Why intensity-weighted median, not mean:** Mean is sensitive to outliers (chimeric PSMs with high intensity but wrong delta mass). Median is robust. PTM-Shepherd calls this the "zero-peak correction"; DeltaMass (Avtonomov et al., 2018) calls it the same thing.

**What the apex_offset tells you:** On a well-calibrated Orbitrap, expect <2 mDa. Values of 5–10 mDa suggest the instrument was drifting. Values >20 mDa suggest the FASTA database or fixed modification settings don't match the acquisition (wrong enzyme, wrong alkylation state, etc.). This makes `apex_offset_da` a useful QC metric on its own.

---

## How neutron folding works (PTM-Shepherd methodology)

After calibration and histogram construction, bins at k × 1.003355 Da (for small integer k) are artifacts of monoisotope peak misassignment by the instrument's peak-picking algorithm. The instrument selected the M+k peak as the precursor, but reported its m/z as if it were the monoisotopic peak, resulting in a peptide match with Δm ≈ k × 1.003355.

These are **not real modifications**. They should be:
1. Detected by proximity to k × 1.003355 (within ~10 mDa)
2. Their PSM counts and intensity merged into the Δ=0 bin (or the nearest real peak apex if the satellite is near a real PTM mass)
3. Logged as folded artifacts (not silently deleted — the count of folded PSMs is useful for assessing instrument monoisotope selection quality)

**Important:** Deamidation at +0.984 Da is NOT a neutron-fold artifact. The nearest k×1.003355 value is k=1 at +1.003 Da, which is 19 mDa away. With a 10 mDa fold tolerance, deamidation is safe. Real PTMs at masses near k×1.003355 (e.g., a PTM at exactly +1.003 Da) can be distinguished by spectral quality: misassignment artifacts have fragment ions matching the unmodified peptide (hyperscore similar to unmodified), while real PTMs at that mass would have shifted fragment ions and a distinct hyperscore pattern. A confidence gate of `mean_hyperscore < unmodified_mean * 0.95` before folding protects real +1 Da modifications.

**What neutron folding eliminates from your original table:**
- Rank 3 (+1.0013, annotated "Label:15N") — k=1 misassignment
- Rank 5 (+2.0008, annotated "Glu→Met") — k=2 misassignment
- Rank 6 (+2.0059, annotated "Label:18O") — k=2 misassignment
- Ranks 10, 11, 14, 17–20, 24–25 (various UNANNOTATED near ±1–2 Da) — misassignment fog

If +1.003 Da survives folding with a high prominence score, it is genuine ¹⁵N metabolic labeling — which is scientifically meaningful and should be surfaced with a note, not suppressed.

---

## Why AA substitution annotations are noise in this context

Unimod contains a class called "AA substitution" that covers single-amino-acid replacements (e.g., Glu→Met at +2.001 Da, Xle→Asn at +0.962 Da, Thr→Cys at +1.963 Da). These appear in open-search output because the database sequence doesn't match the sample (species variant, SNP, sequence error in FASTA) — the peptide is identified via a mass match, not a known PTM.

For the recon tool's purpose (PTM scouting to inform a tight search), AA substitutions are not actionable: you would never add "Glu→Met" as a variable modification in a follow-up search. The `excluded_classifications` filter removes them from annotation output without removing the PSMs from the histogram — the peak still appears, it just gets no annotation, which is more honest than a misleading "Glu→Met" label.

Classifications to exclude by default:
- `"AA substitution"` — sequence variants, not PTMs
- Optionally `"Other glycosylation"` — too broad; specific glycan annotations (HexNAc, Hex, etc.) should be kept

---

## Prominence-based peak detection vs. threshold+merge

The current implementation uses: bins above count threshold → merge adjacent bins within 0.02 Da. This passes any cluster of bins that exceeds the count floor, regardless of whether it rises above the local noise baseline.

Prominence-based detection requires that a peak rise above its local baseline by a minimum fraction. A noise floor of uniformly distributed low-count bins has prominence ≈ 0 for every bin (no bin is notably higher than its neighbors). A real PTM peak rises sharply above its immediate neighborhood and has high prominence.

**Implementation note:** The prominence calculation is O(n log n) over the histogram — negligible for a sparse 0.01 Da histogram over a ±500 Da window (100,000 bins maximum, almost all zero). No performance concern.

**Reference:** PTM-Shepherd documentation states it uses a prominence threshold (0.3 by default) during peak-calling. DeltaMass uses GMM fitting for the same purpose (more sophisticated, harder to implement and validate). Prominence on a histogram is the right tradeoff for this tool: simple, fast, explainable, and directly testable.

---

## Relationship to Crystal-C (chimeric artifact removal)

Crystal-C (Apache-2.0, Nesvilab) corrects a different class of artifact: PSMs where the precursor is a chimeric spectrum (two co-fragmented peptides), resulting in a Δm that is neither the target peptide's true Δm nor a neutron-fold artifact. Crystal-C checks whether the observed `expmass` equals `calcmass + adjacent_residue_mass` (missed cleavage artifact) or a sub-sequence mass (semi-tryptic artifact), and reassigns those PSMs to Δm = 0.

Crystal-C's corrections and neutron folding address different populations and are complementary:
- Neutron folding: precursor monoisotope misassignment → peaks at k × 1.003355 Da
- Crystal-C: chimeric spectra, missed cleavage artifacts → diffuse fog across ±0–4 Da

For the recon tool's Phase 8 scope, neutron folding is the prerequisite (it's the dominant noise source and fully contained in `moddiscovery.rs`). Crystal-C-style correction can be added as a future enhancement if the diffuse ±0–4 Da fog persists after neutron folding. The Crystal-C repo is at `Nesvilab/Crystal-C` (Apache-2.0) and can be vendored under `reference/` if that work proceeds.

---

## Summary of the corrected pipeline order

```
PSMs from Sage TSV
  ↓
[sageresults.rs]  Apply per-PSM isotope correction (correct for narrow search;
                  no-op on open search — that's OK, it's handled downstream)
                  [USES C13_C12_DIFF = 1.003355, not NEUTRON = 1.00866]
  ↓
[moddiscovery.rs] 1. Calibrate: compute apex_offset from |Δm| < 0.1 Da population
                                subtract apex_offset from all Δm before binning
  ↓
                  2. Build histogram (0.01 Da bins, sparse)
  ↓
                  3. Neutron fold: merge k×1.003355 satellite bins into parent peaks
  ↓
                  4. Peak detection: prominence-based (not threshold+merge)
  ↓
                  5. Annotate apex only: Unimod lookup, exclude AA substitutions,
                                         report top-3 candidates with mass_error
```

Previous (broken) order:
```
PSMs → [no-op isotope correction] → histogram → threshold+merge peaks → annotate all bins
```
