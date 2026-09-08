# DeltaMass Methodology Reference

**Paper:** Avtonomov DM, Kong A, Nesvizhskii AI. DeltaMass: Automated detection and visualization of mass shifts in proteomic open-search results. J Proteome Res. 2018 Dec 17;18(2):715–720.  
**DOI:** 10.1021/acs.jproteome.8b00728  
**PMID:** PMC8864583  
**GitHub:** https://github.com/chhh/deltamass

---

## Context

DeltaMass is the precursor to PTM-Shepherd, from the same group (Nesvizhskii lab at University of Michigan). It's a downstream analysis tool for open-search proteomics results, designed to detect, rank, annotate, and visualize precursor mass shifts after PSMs have already been generated.

The paper positions it as infrastructure for interpreting open-search output rather than as a primary search engine — it sits after database search and focuses on the empirical distribution of mass deltas, modification hypotheses, and artifact discovery.

---

## Key Methodological Insights

### 1. KDE vs. Histogram Binning

**Central claim:** Fixed histogram binning is fragile for poorly resolved peaks.

DeltaMass uses **kernel density estimation (KDE)** with a Gaussian kernel instead of ordinary histogram binning. The authors argue that binning can shift peaks or fail to resolve nearby peaks depending on bin width and placement.

The Gaussian kernel is chosen partly because derivatives of the KDE can be computed analytically, enabling derivative-based peak detection.

**Relevance to our tool:** Our Phase 3 implementation uses 0.01 Da histogram binning with adjacent bin merging. This is simpler but may miss shoulder peaks. KDE-based detection could be a Phase 8 enhancement.

### 2. Peak Detection via Second Derivative

Within each 1 Da region, DeltaMass uses the number of **local minima in the second derivative** of the KDE as the number of components for fitting a Gaussian Mixture Model (GMM).

The GMM is then fit using Expectation Maximization (EM). This approach performed better than using Bayesian Information Criterion (BIC) for selecting component count, because BIC tended to underestimate the number of components when peaks were poorly resolved.

**Relevance to our tool:** Our peak detection uses a simpler threshold + merge approach. The derivative-based method is more sophisticated but also more complex to implement.

### 3. Recalibration is Critical

The paper treats mass recalibration as a major prerequisite for good peak resolution. Without alignment of mass scales, nearby peaks broaden and become harder to separate.

Two recalibration modes:
1. **Zero-peak correction:** Detect the large peak at zero mass shift, fit it precisely, use deviation to shift the full scale
2. **Raw-data-based recalibration:** Build 2D calibration curves (m/z × RT) from confidently identified unmodified peptides

**Relevance to our tool:** We rely on Sage's isotope correction, which is related but not the same. The 2D m/z-by-RT calibration is more sophisticated than what we currently do.

### 4. Peak Quality and Ranking

Detected peaks receive:
- **Support:** Number of PSMs supporting the peak
- **Quality:** Based on second derivative of KDE (more pronounced peak = stronger curvature)
- **Score:** Product of quality × intensity

This separates "a peak exists" from "how prominent it is."

**Relevance to our tool:** We use hyperscore/matched_intensity from Sage output for confidence, which serves a similar purpose but is PSM-level rather than peak-shape-level.

### 5. Annotation is Hypothesis, Not Proof

DeltaMass maps detected mass shifts to UniMod and PSI-MOD, then augments with auto-generated possibilities (hydrogen replacement, amino-acid substitutions, additions/deletions).

Candidate annotations are reported if they lie within 2σ of the fitted GMM mean, ordered by distance from that mean.

**Key quote:** "DeltaMass is proposing hypotheses for a delta-mass peak, not claiming definitive site localization or causal assignment."

**Relevance to our tool:** Our `ambiguous: true` flag and multiple-candidate reporting aligns with this philosophy.

### 6. Interactive Exploration is Part of the Method

The paper treats visualization as part of the method, not optional convenience. The viewer supports:
- Zooming, panning, area selection
- Inspecting supporting PSM sequences for selected regions
- Overlaying known modification annotations

**Relevance to our tool:** Phase 7 (Report Output) should consider interactive exploration, not just static reports.

---

## Technical Blueprint from the Paper

1. Read peptide IDs from pepXML or mzIdentML
2. Optionally read raw mzML/mzXML for improved recalibration
3. Recalibrate masses (zero-peak or 2D m/z-by-RT)
4. Compute peptide mass shifts
5. Partition delta-mass axis into 1 Da regions
6. Estimate local density with Gaussian-kernel KDE
7. Use minima in second derivative to choose GMM component count
8. Fit Gaussian mixture using EM
9. Quantify support, width, quality, score for each component
10. Filter peaks by user criteria (PSM support, FWHM)
11. Annotate using UniMod, PSI-MOD, auto-generated hypotheses
12. Present via interactive viewer or command-line export

---

## Demonstration Dataset

The paper used the **Chick et al. dataset** from ProteomeXchange **PXD001468** — the same benchmark dataset referenced in our PLAN.md. This is the "b1906_293T_..." files we plan to use for validation.

Search parameters:
- MSFragger against UniProt
- Precursor tolerance: ±500 Da
- Fragment tolerance: 20 ppm
- Fixed Cys alkylation: +57.02146
- 1% peptide and protein FDR

Results: 788 peaks with ≥20 PSM support, 438 peaks with ≥50 PSM support.

---

## Comparison to Our Approach

| Aspect | DeltaMass | Our Tool (Phase 3) |
|--------|-----------|-------------------|
| Density estimation | KDE with Gaussian kernel | Histogram (0.01 Da bins) |
| Peak detection | Second derivative + GMM | Threshold + adjacent merge |
| Recalibration | Zero-peak or 2D m/z×RT | Sage isotope correction |
| Peak quality | KDE curvature-based | Sage hyperscore/matched_intensity |
| Annotation | UniMod + PSI-MOD + auto-generated | UniMod only |
| Ambiguity handling | 2σ from GMM mean | All matches within tolerance |
| Visualization | Interactive GUI | JSON output (Phase 7 TBD) |

---

## Future Enhancements (Phase 8)

Based on DeltaMass methodology, consider:

1. **KDE-based peak detection** — More robust for poorly resolved peaks
2. **Zero-peak recalibration** — Shift entire delta-mass scale based on unmodified peak center
3. **Peak shape quality metric** — Complement PSM-level confidence with peak-level prominence
4. **PSI-MOD integration** — Additional annotation source beyond UniMod
5. **Interactive viewer** — HTML/JS visualization for exploration

---

## Citation

If referencing this paper in our project:

> Avtonomov DM, Kong A, Nesvizhskii AI. DeltaMass: Automated detection and visualization of mass shifts in proteomic open-search results. J Proteome Res. 2018;18(2):715-720. doi:10.1021/acs.jproteome.8b00728
