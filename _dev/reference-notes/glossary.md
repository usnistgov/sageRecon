# Glossary

Term definitions for the Sage-Based Proteomics Reconnaissance Tool.

---

## Mass Spectrometry Basics

### MS1 / MS2
- **MS1** (survey scan): Full scan of all ions in the sample at a given time. Shows precursor masses.
- **MS2** (fragmentation scan): Selected precursor ions are fragmented; the resulting fragment masses are recorded. Used for peptide identification.

### m/z
Mass-to-charge ratio. The fundamental measurement in mass spectrometry. A peptide with mass 1000 Da and charge +2 appears at m/z 500.

### TIC (Total Ion Current)
Sum of all ion intensities in a spectrum or across a run. Used as a normalization factor and for assessing contamination levels.

### Precursor
The intact ion selected for fragmentation in MS2. Its mass (precursor mass) is compared to calculated peptide masses during database searching.

---

## Database Searching

### PSM (Peptide-Spectrum Match)
A match between an MS2 spectrum and a peptide sequence from the database. The fundamental unit of identification.

### FDR (False Discovery Rate)
The estimated proportion of incorrect identifications. Typically controlled at 1% (q-value < 0.01).

### q-value
The minimum FDR at which a PSM would be accepted. A PSM with q-value 0.005 means if you accept all PSMs with that score or better, ~0.5% would be false.

### Decoy
A fake protein sequence (usually reversed) used to estimate FDR. Sage prefixes decoys with `rev_`.

### Hyperscore
Sage's primary scoring metric for PSMs. Higher is better. Based on the number and intensity of matched fragment ions.

---

## Open Search Concepts

### Open Search
A database search with very wide precursor mass tolerance (e.g., Sage config `da: [-500, 100]`
which produces a delta window of −100..+500 Da), allowing identification of peptides with
unexpected modifications. Note: "−500 to +100" describes the config values, not the delta axis —
see `reference-notes/sage-config-and-gotchas.md` for the sign convention.

### Closed Search
A database search with narrow precursor tolerance (e.g., ±10 ppm), only matching peptides with pre-specified modifications.

### Delta Mass
```
delta_mass = experimental_precursor_mass - calculated_peptide_mass
```
In an open search, the delta mass reveals the mass of any modification present. A delta of ~16 Da suggests oxidation; ~80 Da suggests phosphorylation.

### Isotope Error
When the mass spectrometer selects the wrong isotope peak (e.g., M+1 instead of M+0), the measured mass is offset by ~1.003 Da per isotope. Sage reports this as `isotope_error` (integer: -1, 0, 1, 2, 3).

### Isotope-Corrected Delta Mass
```
corrected_delta = delta_mass - (isotope_error × 1.0086649158849)
```
The delta mass after accounting for isotope selection error.

---

## Modifications (PTMs)

### PTM (Post-Translational Modification)
Chemical modification of a protein after translation. Examples: phosphorylation, oxidation, acetylation.

### Unimod
The standard database of protein modifications. Each entry has a name, monoisotopic mass, and list of possible sites. Used to annotate observed delta masses.

### Static Modification
A modification assumed to be present on ALL instances of an amino acid (e.g., carbamidomethyl on all cysteines from iodoacetamide treatment).

### Variable Modification
A modification that may or may not be present on a given residue. The search engine considers both modified and unmodified forms.

### Common Modifications

| Name | Delta Mass (Da) | Sites | Notes |
|------|-----------------|-------|-------|
| Oxidation | +15.9949 | M, W | Very common artifact |
| Deamidation | +0.9840 | N, Q | Artifact, can confuse with isotope error |
| Phosphorylation | +79.9663 | S, T, Y | Biologically important |
| Acetylation | +42.0106 | K, N-term | Biologically important |
| Carbamidomethyl | +57.0215 | C | From iodoacetamide (usually static) |
| Ammonia loss | -17.0265 | N-term Q, C | Common neutral loss |

---

## Glycoproteomics

### Glycopeptide
A peptide with one or more sugar (glycan) modifications attached.

### Oxonium Ion
A diagnostic fragment ion produced when glycopeptides fragment in MS2. The presence of oxonium ions indicates glycosylation.

### Key Oxonium Ions

| Name | m/z | Diagnostic Value |
|------|-----|------------------|
| HexNAc | 204.0867 | Universal glycopeptide marker |
| Hex-HexNAc | 366.1395 | Common disaccharide |
| NeuAc | 292.1027 | Sialylation marker |
| NeuAc-H₂O | 274.0921 | Sialylation marker |

### Screening Rule
A heuristic for identifying glycopeptide spectra: "≥2 oxonium ions in top 10% of peaks, m/z 204 mandatory."

---

## Contamination

### Polymer Contamination
PEG (polyethylene glycol), PPG (polypropylene glycol), and polysiloxanes are common lab contaminants that produce characteristic ion series.

### Polymer Repeat Units

| Polymer | Repeat Unit Mass (Da) |
|---------|----------------------|
| PEG | 44.0262 |
| PPG | 58.0419 |
| Polysiloxane | 74.0188 |

### %TIC
Percentage of total ion current attributed to a contaminant. Used to assess contamination severity.

---

## Sage-Specific Terms

### Chimeric Search
When `chimera: true`, Sage looks for multiple peptides co-fragmenting in the same MS2 spectrum. With `report_psms: 2`, up to 2 PSMs can be reported per spectrum.

### matched_intensity_pct
Fraction of MS2 intensity explained by matched theoretical fragment ions. Higher values indicate better matches.

### longest_b / longest_y
Length of the longest consecutive series of b-ions or y-ions matched. Longer series indicate more confident identifications.

### ms2_intensity
Sum of intensities of matched fragment ions. Used for intensity-weighted statistics.

---

## Report Schema Terms

### Histogram
Array of all delta mass bins at configured resolution (default 0.01 Da). Contains raw counts and intensities.

### Peaks
Significant local maxima in the delta mass histogram, after peak-picking. Each peak has annotations, confidence metrics, and ambiguity flags.

### Ambiguous
A peak where 2+ Unimod entries match within tolerance. All candidates are reported; user must interpret.

### Unannotated
A peak with no Unimod match within tolerance. Reported as-is rather than force-matched to the nearest wrong entry.

### Signal Fate
Accounting of where MS2 signal goes: identified vs. unidentified, by count and by intensity.

---

## Identification Metrics

### Total MS2 Spectra
The total number of MS2 (fragmentation) scans acquired by the mass spectrometer, as counted from the mzML file. This is the **denominator** for identification rate calculations.

### Identified Spectra
MS2 spectra that were successfully matched to a peptide sequence by the search engine (Sage) at a given FDR threshold (typically q < 0.01). This is the **numerator** for identification rate.

### Unidentified Spectra
MS2 spectra that could not be confidently matched to any peptide. Reasons include:
- No peptide in the database matches the precursor mass
- Fragmentation quality too poor for confident matching
- Peptide present but below FDR threshold
- Contaminant or non-peptide species

### Identification Rate
```
identification_rate = identified_spectra / total_ms2_spectra
```
Typical values for DDA experiments: 50-75%. Higher rates indicate good sample quality and appropriate search parameters.

### MS1 Precursor Intensity vs MS2 TIC
- **MS1 precursor intensity**: The intensity of the intact peptide ion in the MS1 survey scan. This reflects actual peptide abundance and is the biologically meaningful metric.
- **MS2 TIC**: Sum of fragment ion intensities in the MS2 spectrum. This is affected by fragmentation efficiency and is less directly related to peptide abundance.

For quantitative comparisons (e.g., "what fraction of signal is identified?"), MS1 precursor intensity is preferred. MS2 TIC comparisons require careful interpretation.

---

## Label-Free Quantification (LFQ)

### LFQ
Label-Free Quantification. Estimating protein/peptide abundance without isotopic labels, typically using MS1 precursor intensity or spectral counting.

### XIC (Extracted Ion Chromatogram)
The intensity trace of a specific m/z value across retention time. Used to visualize and integrate peptide elution profiles.

### Feature Detection
The process of identifying 3D peaks (m/z × RT × intensity) in MS1 data that correspond to peptide ions. Tools: MaxQuant, Dinosaur, Biosaur2, OpenMS FeatureFinder.

### Peak Integration
Calculating the area under a chromatographic peak (XIC). More robust than single-point intensity measurements.

### MS1 Intensity Extraction Approaches

| Approach | Description | Quality | Speed |
|----------|-------------|---------|-------|
| MS2 precursor metadata | Single intensity from mzML header | Worst | Fastest |
| Fixed RT window sum | Sum MS1 intensity in ±N min window | Intermediate | Moderate |
| XIC + peak integration | Full chromatogram extraction + integration | Best | Slowest |

For QC/reconnaissance tools, fixed RT window summation is often sufficient. For publication-quality LFQ, XIC + peak integration is required.

---

## DDA-Specific Terms

### DDA (Data-Dependent Acquisition)
Acquisition mode where the instrument selects precursors for MS2 based on MS1 intensity. The most common mode for discovery proteomics.

### Dynamic Exclusion
After fragmenting a precursor, the instrument excludes it from re-selection for a set time (e.g., 30 seconds). Prevents repeatedly fragmenting the same abundant peptide.

### Chimeric Spectrum
An MS2 spectrum containing fragments from multiple co-eluting peptides that were co-isolated. Common when precursor isolation window is wide or peptides have similar m/z.

### Co-elution
When multiple peptides elute from the LC column at the same retention time. Can lead to chimeric spectra if they have similar m/z.

---

## Abbreviations

| Abbrev | Meaning |
|--------|---------|
| Da | Dalton (atomic mass unit) |
| ppm | Parts per million (mass accuracy) |
| RT | Retention time |
| LC | Liquid chromatography |
| ESI | Electrospray ionization |
| HCD | Higher-energy collisional dissociation |
| FASTA | Text format for protein sequences |
| mzML | Standard format for mass spec data |
| TSV | Tab-separated values |

---

*Last updated: 2026-07-07*
