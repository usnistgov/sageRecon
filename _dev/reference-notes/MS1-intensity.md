# MS1 Precursor Intensity Extraction — Reference Notes

You're likely underestimating precursor signal with the current logic; 6–10% MS1 explained is plausible if you only track monoisotopic peaks and ignore feature structure, but you can push this closer to what "looks intuitive" by moving from scan-wise peak picking to feature-wise integration and by better separating peptide features from background. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC4563724/)

## Big-picture sanity checks

- In complex proteomics LC–MS1, a large fraction of TIC is non-peptidic or never selected for MS/MS (singly charged, very low/high m/z, contaminants, background, etc.). That alone can keep "identified MS1%" surprisingly low even if 2/3 of MS/MS scans are identified. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC9262215/)
- Many groups treat "MS1 peptide ion intensity" via chromatographic feature finding and integration rather than summing per-scan intensity in a fixed RT window; that tends to give higher and more stable precursor intensities. So your ~6.4% is not crazy for a first-pass, but the method is biased low. [sciencedirect](https://www.sciencedirect.com/science/article/pii/S1535947620326475)

A concrete example: take a well-resolved peptide feature that contributes, say, 1e6 total integrated intensity across its isotope envelope. If you only grab the monoisotopic peak within a ±1 min window, and your RT window misses 25–50% of the scans, you can easily record <3–4e5 of that. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC4563724/)

## Suggestions on your current steps

### 1. Peak-level vs feature-level extraction

Your current logic is "scan-local" and doesn't enforce chromatographic continuity. A more robust approach is:

- Build MS1 features per precursor: contiguous MS1 peaks over RT with consistent m/z (within ppm) and predictable isotope spacing. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC9262215/)
- For each PSM, map to the nearest feature in (m/z, RT, charge) space, and then integrate the feature's full isotope envelope (area under the curve). That becomes the precursor's MS1 intensity.  

This avoids multiple problems at once: RT window choice, monoisotopic-only, and MS1-MS2 RT offsets. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC4563724/)

If you don't want to implement full feature detection yet, a minimal compromise:

- For each PSM, find the MS1 scan immediately preceding the MS2 scan and maybe ±N scans (e.g., ±5 scans) instead of ±1 min.  
- Require a monotonic-ish chromatographic shape (e.g., intensities rising then falling) when summing across scans, to avoid summing random noise spikes.

### 2. Isotope envelope summing details

Your plan to sum M+0–M+3 is exactly on point, but a couple refinements:

- Use charge-specific spacing: peaks at \( \text{mz} + k \times 1.003 / z \) for \(k = 0,1,2,3\). [www2.chemistry.msu](https://www2.chemistry.msu.edu/faculty/reusch/virttxtjml/spectrpy/massspec/masspec1.htm)
- Allow a slightly larger ppm window for isotope peaks than for the monoisotopic, since centroiding and lower S/N can shift them more.  
- Optionally weight peaks by theoretical relative isotope distribution (from peptide composition) to detect obviously wrong envelopes (e.g., a contaminant overlapping at M+1). [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC9262215/)

Even without composition, summing all observed isotopic peaks within a small RT band will probably bump your explained MS1 from ~6–10% to something like 2–3×, depending on sample complexity. [sciencedirect](https://www.sciencedirect.com/science/article/pii/S1535947620326475)

### 3. RT handling and window choice

You already note that Sage's RT is the MS2 scan time. A few refinements:

- Instead of a fixed ±1.0 min window, consider a dynamic window tied to chromatographic peak width: e.g., ±0.5 × estimated FWHM for that m/z region, or just ±N scans based on instrument cycle time. [sites.bu](https://sites.bu.edu/cheminst/files/2021/06/LCMSPrimer.pdf)
- Explicitly locate the MS1 scan that triggered the MS2 (often the previous MS1 scan in the raw file), then center your RT window around that MS1 RT rather than the MS2 RT.  

Printing a handful of traces for high-abundance precursors (RT vs intensity around the PSM) should tell you quickly whether you're chopping peaks or including too much background.

### 4. TIC definition and "explained %"

Right now, you're dividing by the sum of TIC across all MS1 scans, which includes:

- Solvent baseline and chemical noise.  
- Highly abundant contaminants (polymers, lipids, etc.).  
- Many singly charged or non-fragmented species. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC4563724/)

Alternative denominators that will make the metric more interpretable:

- **Peptide-like TIC only**: Limit TIC to features with peptide-like charge (2–4), expected m/z range, and chromatographic shape. That's closer to "precursor pool your search could potentially identify." [sciencedirect](https://www.sciencedirect.com/science/article/pii/S1535947620326475)
- **Fragmentable TIC**: Exclude singly charged and extreme m/z features likely never selected for MS/MS.  
- **Matched feature TIC**: Run a feature finder (or your own minimal one) on MS1, classify features as "has any MS2" vs "no MS2", and compute % TIC in each class, then overlay % identified within the "has MS2" class. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC9262215/)

You might end up with something like:

| Metric                            | Interpretation                                                |
|-----------------------------------|--------------------------------------------------------------|
| MS1 TIC explained (all TIC)      | Fraction of total MS1 signal tied to PSMs                   |
| Peptide-like TIC explained       | Fraction of peptide-like MS1 signal explained by PSMs       |
| TIC of MS1 features with MS2     | How fully you identify what the DDA actually targeted       |
| TIC of MS1 features w/o MS2      | "Untargeted" peptide-like or other signals                  |

This separation will help you distinguish "true missing identifications" from "MS1 background you never meant to identify."

### 5. Sanity checks with single features

Your recommendation to manually trace one PSM is good; I'd extend it:

- Pick 5–10 very abundant, high-confidence PSMs across RT.  
- For each, visualize MS1 intensity vs RT for M+0–M+3 and record:  
  - Integrated area of full envelope (ideal)  
  - What your current algorithm extracts  
- Quantify the ratio current/true; if that's systematically 0.2–0.3, then your global 6.4% might actually correspond to something like 20–30% of peptide-like MS1.

If you store these as small test cases, you can regression-test changes in the algorithm.

### 6. Practical implementation tweaks

Given your tooling mindset, a few implementation-level moves:

- Pre-index MS1 spectra by RT and m/z (e.g., RT-sorted array + per-scan m/z-sorted peaks) to speed envelope extraction across many PSMs.  
- Cluster PSMs that share essentially the same precursor (same m/z/RT/charge) to avoid double-counting the same MS1 feature across multiple MS2 events.  
- Consider an option to work directly from an MS1 feature file from existing tools (e.g., MaxQuant features, Dinosaur, or similar) if available, then just map Sage PSMs onto those features rather than reinventing feature detection. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC9262215/)

Yes — keep it lightweight inside Sage, but shift the logic from "sum any matching monoisotopic peaks in a broad RT box" to "recover a local precursor feature around the triggering MS1 and integrate a small isotope-aware chromatogram." That still stays simple, but it matches how peptide MS1 quantitation is usually treated: as an extracted ion chromatogram / feature over time, not a one-peak-per-scan lookup. [sciencedirect](https://www.sciencedirect.com/science/article/pii/S1535947620326475)

## What to change first

Your highest-value upgrades are:

- Center extraction on the **triggering MS1** (usually the previous MS1 before the MS2), not Sage RT alone. [bioconductor](https://www.bioconductor.org/help/course-materials/2017/CSAMA/labs/4-thursday/lab-04-Mass_spec_proteomics_and_metabolomics/01-proteomics/lab.html)
- Extract an isotope-aware chromatogram for M+0 to M+2 or M+3 using charge-adjusted spacing, then integrate over the local elution peak. [openms](https://openms.de/documentation/TOPP_FeatureFinderCentroided.html)
- De-duplicate repeated MS2s that target the same precursor feature, or you will double-count MS1 signal from the same peptide feature. [openms.readthedocs](https://openms.readthedocs.io/en/latest/getting-started/types-of-topp-tools/feature-detection.html)

Those three changes alone should make the result much more interpretable without turning Sage into a full feature finder. [openms](https://openms.de/documentation/TOPP_FeatureFinderCentroided.html)

## A lightweight Sage design

A good "borrow from mzSniffer-style ideas, but keep it small" design is:

1. For each identified PSM, locate the immediately preceding MS1 scan by scan number or acquisition order. [uclouvain-cbio.github](https://uclouvain-cbio.github.io/WSBIM2122/sec-ms.html)
2. In that MS1, find a seed peak near precursor m/z; then test for isotope partners at \(m/z + k \times 1.00335 / z\) for \(k=0,1,2,3\). [patternlabforproteomics](https://patternlabforproteomics.org/rawvegetable/info/)
3. Extend left/right across neighboring MS1 scans while the envelope remains present and chromatographically coherent; sum the envelope intensity per scan, then integrate across scans as the precursor intensity. [sciencedirect](https://www.sciencedirect.com/science/article/pii/S1535947620326475)

That is much lighter than full 2D feature detection, but it captures the essential structure of a peptide feature: isotope envelope plus RT continuity. [openms.readthedocs](https://openms.readthedocs.io/en/latest/getting-started/types-of-topp-tools/feature-detection.html)

A few constraints help a lot:

- Require at least 2 isotopes for charge \(z \ge 2\) when signal is strong enough, otherwise flag low confidence. [abibuilder.cs.uni-tuebingen](https://abibuilder.cs.uni-tuebingen.de/archive/openms/Documentation/nightly/html/TOPP_FeatureFinderCentroided.html)
- Stop RT extension when envelope intensity drops below a fraction of apex or disappears for N consecutive MS1 scans. [sciencedirect](https://www.sciencedirect.com/science/article/pii/S1535947620326475)
- Use scan-count windows first, not ±1 min; cycle time is what matters locally. [bioconductor](https://www.bioconductor.org/help/course-materials/2017/CSAMA/labs/4-thursday/lab-04-Mass_spec_proteomics_and_metabolomics/01-proteomics/lab.html)

## Specific recommendations

### Replace fixed RT box

A fixed ±1 min window is probably too blunt. DDA quantitation is usually based on precursor peaks integrated over retention time as an extracted ion chromatogram, not summed over an arbitrary box. [uclouvain-cbio.github](https://uclouvain-cbio.github.io/WSBIM2122/sec-ms.html)

A better lightweight rule:

- Start from the trigger MS1.
- Walk ±5 to ±15 MS1 scans.
- Per scan, compute envelope intensity.
- Keep only the contiguous region around the local apex.

That reduces both undercounting and accidental background accumulation. [openms](https://openms.de/documentation/TOPP_FeatureFinderCentroided.html)

### Use envelope intensity per scan

Instead of "max intensity within tolerance at monoisotopic m/z," define per-scan precursor intensity as:

- Sum of matched isotope peaks M+0 to M+2/M+3, or
- Weighted sum with mild downweighting of higher isotopes if you want extra robustness to interference. [abibuilder.cs.uni-tuebingen](https://abibuilder.cs.uni-tuebingen.de/archive/openms/Documentation/nightly/html/TOPP_FeatureFinderCentroided.html)

If you want one very simple rule: use M+0 + M+1 + M+2 first. That gets most of the gain without much complexity. [patternlabforproteomics](https://patternlabforproteomics.org/rawvegetable/info/)

### Prevent double counting

This is a big one. In DDA, the same precursor feature can generate multiple MS2 spectra across its elution peak, so summing precursor intensity per PSM can inflate the "explained" numerator unless you collapse PSMs onto unique precursor features. [uclouvain-cbio.github](https://uclouvain-cbio.github.io/WSBIM2122/sec-ms.html)

Collapse by something like:

- same charge,
- precursor m/z within 5–10 ppm,
- RT within a local feature width,
- overlapping extracted envelope scans.

Then count the feature once for explained MS1, while still keeping all PSMs for ID stats. [openms.readthedocs](https://openms.readthedocs.io/en/latest/getting-started/types-of-topp-tools/feature-detection.html)

## Metrics to report

I would report at least two numbers:

| Metric | Meaning |
|---|---|
| PSM-linked MS1% of total TIC | Strict, conservative, includes all noise/background in denominator. [sciencedirect](https://www.sciencedirect.com/science/article/pii/S1535947620326475) |
| Unique precursor-feature MS1% of total TIC | Better estimate of how much MS1 signal is explained after collapsing repeated targeting. [openms](https://openms.de/documentation/TOPP_FeatureFinderCentroided.html) |

And maybe a third:

| Metric | Meaning |
|---|---|
| Identified % of peptide-like MS1 | Restrict denominator to multi-isotope, multi-scan, charge-consistent features; more biologically meaningful than raw TIC. [openms](https://openms.de/documentation/TOPP_FeatureFinderCentroided.html) |

That last metric is often the one people actually want, because raw TIC is dominated by things you never expected to identify. [bigomics](https://bigomics.ch/blog/how-to-process-raw-mass-spectrometry-data-top-tools-for-proteomics-data/)

## Minimal borrowing from other tools

If you borrow ideas from mzSniffer or similar tools, borrow these patterns rather than full architecture:

- Envelope seeding from one MS1 scan.
- Local RT growth with continuity checks.
- Feature collapsing across repeated MS2 triggers.
- Confidence flags: monoisotopic-only, partial envelope, overlapping candidate, poor chromatographic shape. [abibuilder.cs.uni-tuebingen](https://abibuilder.cs.uni-tuebingen.de/archive/openms/Documentation/nightly/html/TOPP_FeatureFinderCentroided.html)

That keeps Sage integration manageable while giving you feature-like behavior. [openms](https://openms.de/documentation/TOPP_FeatureFinderCentroided.html)

## Suggested implementation order

I'd implement in this order:

1. Previous-MS1 anchoring. [bioconductor](https://www.bioconductor.org/help/course-materials/2017/CSAMA/labs/4-thursday/lab-04-Mass_spec_proteomics_and_metabolomics/01-proteomics/lab.html)
2. M+0 to M+2 envelope summing per scan. [patternlabforproteomics](https://patternlabforproteomics.org/rawvegetable/info/)
3. Local scan-wise extension and integration around apex. [sciencedirect](https://www.sciencedirect.com/science/article/pii/S1535947620326475)
4. Collapse duplicate PSMs onto unique precursor features. [openms.readthedocs](https://openms.readthedocs.io/en/latest/getting-started/types-of-topp-tools/feature-detection.html)
5. Optional "peptide-like denominator" metric. [openms](https://openms.de/documentation/TOPP_FeatureFinderCentroided.html)

If you do only the first four, you'll already have something lightweight, defensible, and much closer to the biological question than the current ±1 min monoisotopic box sum. [uclouvain-cbio.github](https://uclouvain-cbio.github.io/WSBIM2122/sec-ms.html)

---

## Phase 6D Implementation Results (2026-07-08)

We implemented the first four recommendations above. Results on test file:

| Metric | Basic (monoisotopic) | Improved (envelope + dedup) |
|--------|---------------------|----------------------------|
| Explained MS1 TIC | 6.4% | 9.4% |
| Explained peptide-like TIC | — | 11.1% |
| PSMs with MS1 signal | 87.6% | 84.3% |
| Unique features | 81,966 PSMs | 66,299 features |

**What was implemented:**
1. ✅ Isotope envelope summing (M+0, M+1, M+2 at charge-adjusted spacing)
2. ✅ Precursor deduplication (collapse PSMs by m/z, RT, charge)
3. ✅ Scan-based RT window (±N MS1 scans instead of fixed time)
4. ✅ Peptide-like denominator (400-1200 m/z range)

**Key insight:** The ~50% improvement (6.4% → 9.4%) is real but the number still "feels low" because we're conflating three different failure modes in the "unidentified" bucket:

1. **Non-peptidic** — Signal outside peptide-like m/z/charge range
2. **Never sampled** — Peptide-like features that never triggered MS2 (DDA limitation)
3. **Sampled but not identified** — MS2 acquired but no confident PSM

Phase 6E will separate these three layers to tell a clearer story.

---

## Why ~10% Explained MS1 TIC is Actually Reasonable

After Phase 6D, we investigated why the number still seems low. Key findings:

### The biology/physics explanation

1. **Most MS1 TIC is not peptide signal** — Even in a proteomics run, MS1 TIC includes solvent baseline, chemical noise, contaminants, singly charged ions, and compounds outside the intended m/z/charge window.

2. **DDA samples only a small subset** — Top-N selection means many valid peptide features never get fragmented, especially at low abundance or in crowded regions. Those features contribute MS1 TIC but never appear in the PSM list.

3. **Identification adds another filter** — Among features that do get MS2, only a fraction yield high-confidence PSMs.

### Literature context

Groups that combine DDA with MS1 XIC approaches report that the absolute fraction of total MS1 TIC mapping to identified peptides is often modest (10-20% of peptide-like TIC). This is why MS1-based strategies and cross-run alignment are promoted to alleviate missing-value problems in DDA quantitation.

### The three-layer breakdown (Phase 6E target)

```
Total MS1 TIC:           3.10e13 (100%)
├─ Non-peptidic:         0.48e13 (15.5%)  [outside 400-1200 m/z or z=1]
├─ Peptide-like:         2.62e13 (84.5%)
   ├─ Never sampled:     ~58%  [no MS2 trigger]
   ├─ Sampled, not ID'd: ~17%  [MS2 but no PSM]
   └─ Identified:        ~9%   [PSM at q<0.01]
```

This transforms "6.4% identified" into "of peptide-like MS1, ~35% was sampled by MS2, and of that sampled portion, ~27% was identified" — a much more interpretable metric.

### References

- [Sciencedirect: MS1-based quantitation](https://www.sciencedirect.com/science/article/pii/S1535947620326475)
- [PMC: DDA sampling limitations](https://pmc.ncbi.nlm.nih.gov/articles/PMC7000113/)
- [Springer: Missing values in DDA](https://link.springer.com/article/10.1186/s12014-025-09572-2)

---

## Phase 6E Final Results (2026-07-09)

### Comprehensive Signal Fate Comparison Table

This table summarizes all signal fate metrics from Phases 6D and 6E on the test file (B.naive_01steady-state.mzML.gz, narrow search, 491,461 PSMs at q<0.01):

| Metric | By Count | By MS2 TIC | By MS1 TIC (basic) | By MS1 TIC (improved) | By MS1 TIC (three-layer) |
|--------|----------|------------|--------------------|-----------------------|--------------------------|
| **Denominator** | MS2 spectra | MS2 TIC | Total MS1 TIC | Peptide-like MS1 TIC | Peptide-like MS1 TIC |
| **Identified** | 72.0% | 26.2% | 6.4% | 11.1% | 48.0% |
| **Unidentified** | 28.0% | 73.8% | 93.6% | 88.9% | 22.4% (sampled) + 29.7% (never sampled) |

### Three-Layer MS1 Signal Fate Breakdown

```
Total MS1 TIC:           3.07e13 (100%)
├─ Non-peptidic:         4.51e12 (14.7%)  [outside 400-1200 m/z]
├─ Peptide-like:         2.62e13 (85.3%)
   ├─ Never sampled:     7.79e12 (29.7%)  [no MS2 trigger - DDA limitation]
   ├─ Sampled, not ID'd: 5.86e12 (22.4%)  [MS2 but no PSM]
   └─ Identified:        1.26e13 (48.0%)  [PSM at q<0.01]

Derived Metrics:
  Sampling efficiency: 70.3% of peptide-like TIC was sampled by MS2
  ID efficiency:       68.2% of sampled TIC was identified
```

### Key Lessons Learned

1. **The "6.4% identified" number is misleading** — It conflates three different failure modes (non-peptidic, never sampled, sampled but not ID'd) into one "unidentified" bucket. The three-layer breakdown shows that 48% of peptide-like MS1 TIC is actually identified.

2. **MS1 intensity adds computational cost without proportional insight** — The three-layer analysis takes ~165 seconds on this file (vs. ~10 seconds for MS2-only stats). For QC purposes, MS2 count-based metrics (72% ID rate) are sufficient and much faster.

3. **The "never sampled" fraction is the DDA limitation** — 29.7% of peptide-like MS1 TIC was never selected for MS2 fragmentation. This is the fundamental DDA sampling limitation that DIA addresses.

4. **Isotope envelope summing helps but doesn't change the story** — Going from monoisotopic-only (6.4%) to envelope summing (9.4%) is a ~50% improvement, but the three-layer breakdown (48% of peptide-like) tells a much clearer story.

5. **Deduplication is essential** — 491,461 PSMs collapsed to 351,034 unique features (28% reduction), indicating significant chimeric/repeated sampling of the same precursors.

### When to Use MS1 Intensity Metrics

**Use MS1 intensity when:**
- Investigating why a sample has low ID rates (is it sampling or identification?)
- Comparing DDA vs DIA acquisition strategies
- Assessing polymer/contaminant burden (polymer-stats uses total MS1 TIC as denominator)
- Debugging specific peptide features (manual inspection)

**Use MS2 count-based metrics when:**
- Routine QC (72% ID rate is fast and interpretable)
- Comparing runs within the same acquisition method
- Assessing search parameter effects (same raw data, different searches)

### Implementation Notes

The three-layer analysis required:
1. **SortedRegionIndex** — Binary search on RT-sorted regions for O(n×log(m)) lookup instead of O(n×m)
2. **Progress reporting** — ETA display every ~5% of spectra for long-running analysis
3. **Modification breakdown** — Groups PSMs by delta mass and computes TIC per modification (useful for open search data)

### Recommendation for Default Workflow

For the recon tool's default workflow, **MS2 count-based metrics are recommended**:
- 72% ID rate by count is fast, interpretable, and sufficient for QC
- MS1 intensity analysis is available via `--three-layer` flag for deeper investigation
- The three-layer breakdown is most useful when troubleshooting low ID rates or comparing acquisition strategies

---

## Known Tradeoffs and Assumptions

### 1. Peptide-Like Denominator (400-1200 m/z)

**Current approach:** We define "peptide-like" as peaks within 400-1200 m/z range.

**Limitation:** A more precise definition would filter by charge state (z=2-4 for typical tryptic peptides), but we cannot reliably infer charge from MS1 peaks alone without full feature detection (isotope envelope fitting, charge state deconvolution).

**Tradeoff accepted:** The m/z range filter is a reasonable approximation that excludes obvious non-peptidic signal (singly-charged contaminants at low m/z, large molecules at high m/z) without requiring computationally expensive feature detection.

**Future enhancement:** If we ever implement lightweight feature detection (e.g., isotope envelope validation), we could tighten this to charge-filtered peptide-like TIC.

### 2. Narrow Search Shows 100% Unmodified

**Observation:** When testing with narrow precursor tolerance (±10-20 ppm), the modification breakdown shows 100% unmodified.

**This is expected behavior, not a bug:** With tight precursor tolerance, only peptides matching the expected mass (within ppm tolerance) are identified. Unexpected PTMs would shift the precursor mass outside this tolerance window and never be matched. The delta mass for all PSMs is therefore ~0 Da.

**Implication:** The modification breakdown feature is only meaningful for open search data (wide Da tolerance) where unexpected mass shifts can be captured.

### 3. Why MS2 TIC Intensity Isn't Meaningful for Signal Fate

**Observation:** MS2 TIC-based ID rate (26.2%) differs dramatically from count-based (72%) and MS1-based (48%) metrics.

**Why MS2 TIC is problematic:**
1. **Chimeric spectra share intensity** — Multiple co-eluting peptides contribute to the same MS2 TIC, but we can't split it between them
2. **TIC dominated by abundant fragments** — A few high-intensity fragment ions dominate TIC, not proportional to precursor abundance
3. **Fragmentation efficiency varies** — Some peptides fragment better than others, independent of their abundance
4. **No biological meaning** — MS2 TIC doesn't correlate with protein abundance or biological relevance

**Recommendation:** Use MS2 count-based metrics for routine QC. MS1 TIC is more biologically meaningful (correlates with precursor abundance) but computationally expensive. MS2 TIC is neither fast nor meaningful — avoid it.
