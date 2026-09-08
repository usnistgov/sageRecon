# MS1 Precursor Intensity Extraction Approaches in DDA Proteomics

This document summarizes the three main approaches for extracting MS1 precursor intensity for identified peptides in DDA (Data-Dependent Acquisition) proteomics, based on literature review and community discussion.

## Overview

| Approach | Description | Quantitative Quality | Computational Cost | Tools |
|----------|-------------|---------------------|-------------------|-------|
| **1. MS2 precursor metadata** | Use intensity stored in MS2 spectrum header | Worst | Minimal | Rarely used for LFQ |
| **2. XIC + peak integration** | Extract ion chromatogram, detect peak, integrate area | Best | Highest | MaxQuant, Skyline, OpenMS, IQMMA |
| **3. Fixed RT window sum** | Sum MS1 intensity at precursor m/z within RT window | Intermediate | Moderate | Custom scripts, lightweight pipelines |

---

## Approach 1: MS2 Precursor Intensity from mzML Metadata

### What It Is
Take the precursor intensity recorded in the MS2 spectrum header (e.g., `precursorList/precursor/intensity` in mzML) and use that single value for peptide quantitation.

### What This Intensity Actually Is
- The MS1 peak height at the time the precursor was selected for fragmentation
- Measured in the preceding full MS1 scan or survey scan
- A **single point** estimate, not an integrated chromatographic area

### Pros
- Very fast: just read a single number per MS2 event
- Simple to implement: parsers can grab this from MS2 headers with minimal logic

### Cons
- **High noise and poor robustness**: a single snapshot is sensitive to transient fluctuations, scan scheduling, and interference at that exact RT
- **Biased by DDA triggering**: instrument tends to pick high-intensity points; different runs may trigger at slightly different points on the peak, increasing variance and missingness
- Does not handle overlapping features or isotopic envelopes

### Tool Usage
- Mainstream LFQ tools (MaxQuant, Skyline, OpenMS, IQMMA) do **not** use this as the primary quant
- Some basic database-search outputs expose this value for reporting, but it's rarely the main quantitative metric

---

## Approach 2: Extracted Ion Chromatogram (XIC) + Peak Integration

### What It Is
The "canonical" MS1-based LFQ approach: reconstruct the precursor's chromatographic trace from MS1 scans and integrate peak area (or use peak height).

### What Happens
1. For each peptide ID (m/z, charge, RT), extract the MS1 signal in a narrow m/z window (plus isotopic envelope: M, M+1, M+2, etc.)
2. Obtain an XIC over retention time
3. Detect the corresponding chromatographic peak
4. Integrate area under the curve (AUC) or use peak height
5. Many tools also apply feature detection: building 3D peaks in m/z–RT–intensity space

### Pros
- **Best quantitative performance**: integrating across the elution profile averages out MS1 cycle timing effects and noise
- Handles sampling density: even if MS1 scans are sparse, AUC across multiple points is more robust
- Can explicitly model isotopic envelope and charge, improving specificity

### Cons
- Computationally heavier: requires scan-level MS1 parsing, peak detection, and integration
- Sensitive to feature detection parameters (RT tolerance, m/z tolerance, minimum peak length)
- More complex to implement correctly, especially for noisy data or overlapping elution profiles

### Tool Usage
This is the standard in essentially all serious DDA LFQ pipelines:
- **MaxQuant / MaxLFQ**: detects MS1 peptide features (3D peaks) and uses integrated MS1 intensities
- **Skyline MS1 Filtering**: extracts MS1 ion intensity chromatograms and integrates peaks
- **OpenMS FeatureFinder**: detects MS1 peptide features (m/z–RT–intensity volumes)
- **IQMMA**: integrates multiple feature detection tools (Dinosaur, Biosaur2, OpenMS FeatureFinder)

---

## Approach 3: Fixed RT Window Summation

### What It Is
A simplified version of XIC-based integration: instead of explicit peak detection, sum MS1 precursor intensity (within a given m/z window) over all MS1 scans in a fixed RT range around the MS2 acquisition.

### What Happens
1. For each peptide ID, pick a retention time window (e.g., ±30s to ±3min around the MS2 RT)
2. Collect the MS1 intensities at the precursor m/z from all MS1 scans in that window
3. Sum or average them to obtain a pseudo-AUC value
4. Often still define a narrow m/z tolerance and may include isotopes, but skip explicit peak boundary detection

### Pros
- Faster and simpler than full-blown feature detection
- More robust than a single MS2 snapshot (Approach 1) because it considers multiple MS1 points
- Useful for intermediate complexity between "just use precursor intensity" and "run a feature detector"

### Cons
- **Peak boundaries are approximate**: fixed RT windows may truncate peaks or include baseline/off-peak regions
- **Can over-include interference**: nearby co-eluting peptides at similar m/z may contribute
- **Sensitive to RT shifts**: if RT drifts between runs, fixed windows can misalign with peaks

### Tool Usage
- Many **custom scripts** and lightweight in-house pipelines use this approach
- Not the flagship method in mainstream LFQ software
- Appears as a simplification discussed in community forums

---

## Comparison Summary

### Quantitative Quality
1. **Approach 1 (MS2 precursor)**: Worst; high variance, strong dependence on DDA triggering
2. **Approach 3 (fixed window sum)**: Intermediate; better than single point, but approximate
3. **Approach 2 (XIC + integration)**: Best; explicitly models chromatographic peak

### Computational/Implementation Cost
1. **Approach 1**: Minimal; just read metadata per MS2 spectrum
2. **Approach 3**: Moderate; scan-level MS1 parsing plus windowing
3. **Approach 2**: Highest; full chromatogram reconstruction, feature detection, peak integration

---

## Recommendation for QC/Reconnaissance Tools

For a QC/reconnaissance tool (not publication-quality LFQ), **Approach 3 (fixed RT window sum)** is appropriate because:

1. **We're doing signal fate accounting**, not quantitative proteomics
2. We want to know "what fraction of MS1 signal is explained by identified peptides"
3. Computational simplicity matters for a QC tool
4. The precision loss vs. full XIC integration is acceptable for QC purposes

### Implementation Notes
- RT window: ±1 minute is a reasonable default (configurable)
- m/z tolerance: 10 ppm
- Use peak height sum, not integrated area
- Document clearly that this is not LFQ-quality quantitation

---

## mzSniffer's Approach (Reference Implementation)

mzSniffer uses a variant of Approach 3 for polymer detection:

```rust
fn find_peaks(query_vec: &[f64], tol_vec: &[f64], mz_vec: &[f64], intensity_vec: &[f64]) -> f64 {
    let mut total_intensity = 0.;
    for (query_mz, tol) in query_iter {
        let mut biggest = 0.;
        for (mz, intensity) in spec_iter {
            if (mz - query_mz).abs() <= *tol && intensity > &biggest {
                biggest = *intensity;  // Takes MAX intensity at that m/z
            }
        }
        total_intensity += biggest;  // Sums across all query m/z values
    }
    total_intensity
}
```

Key points:
- For each MS1 spectrum, finds the **max intensity** at each query m/z (within tolerance)
- Sums those max intensities across all query m/z values
- Then sums across all MS1 scans to get total
- This is essentially **summing peak heights across RT**, not integrating area

---

## References

- Reddit discussion: [Are there any tools which use MS1 precursor?](https://www.reddit.com/r/proteomics/comments/1dw1f3e/are_there_any_tools_which_use_ms1_precursor/)
- Reddit discussion: [How does XIC based LFQ work?](https://www.reddit.com/r/proteomics/comments/1d06njs/how_does_xic_based_lfq_work/)
- IQMMA paper: [bioRxiv 2023.02.03.526776](https://www.biorxiv.org/content/10.1101/2023.02.03.526776v1.full.pdf)
- RSC Book Chapter: [MS1 Label-free Quantification Using Ion Intensity](https://books.rsc.org/books/edited-volume/1080/chapter/835055/MS1-Label-free-Quantification-Using-Ion-Intensity)
