# MSFragger / FragPipe: Calibration and Tolerance Optimization

**Source:** FragPipe workflow documentation, MSFragger papers, run logs
**License:** See bottom of this file

---

## Workflow structure

MSFragger's open search workflow embeds a calibration and parameter optimization stage between an initial narrow search and the main open search. The sequence per file:

### Step 1 — First (narrow) search

Uses a standard precursor tolerance (e.g., ±20 ppm on corrected masses) plus default fragment tolerance (e.g., 20 ppm). Purpose: collect high-confidence PSMs to characterize mass error. Logged as `FIRST SEARCH`.

### Step 2 — Mass calibration

High-confidence PSMs (filtered by expectation value) are split into two halves:

- **First half:** MSFragger builds a **two-dimensional m/z × retention-time grid** of mass error for MS1 and MS2:
  - For MS1/precursor: traces extracted-ion chromatograms across the 0/+1/+2 ¹³C isotope peaks for each PSM, computes intensity-weighted deviations, fits a grid of systematic errors.
  - For MS2/fragments: computes analogous fragment error statistics.
- **Second half:** validates the calibration; computes updated median/MAD error statistics per run, printed in the log as `MS1 Old/New, MS2 Old/New`.

This is a **2D (m/z × RT) grid model**, not a single scalar offset. It is stricter than a global apex_offset-style correction.

### Step 3 — Parameter optimization

With calibrated masses, MSFragger sweeps candidate fragment tolerances and peak-count settings:
- Fragment tolerance candidates: 5, 7, 10, 15, 20, 25, 30, 50 ppm
- `usetopNpeaks` candidates: 3000, 2000, 1750, 1501, 1251, 1001

Chooses the combination that maximizes PSMs at target FDR (e.g., 1% spectrum-level) using target–decoy scoring. Chosen settings appear in the log as, e.g., `New fragmentmasstolerance 10.000000 PPM`, `New usetopNpeaks 100`.

### Step 4 — Main open search

Uses calibrated masses and optimized tolerances with the full open window. Typical values from a real run:
- `precursormasslower -150.0`, `precursormassupper 500.0` (open ∼−150/+500 Da)
- `precursortruetolerance 4.0 PPM`
- `fragmentmasstolerance 10.0 PPM`

Followed by the usual downstream pipeline: Crystal-C → PeptideProphet → Philosopher → PTM-Shepherd.

---

## Key properties

- **Per-run calibration:** calibration is run-level; each mzML/RAW file gets its own calibration profile and optimized tolerances.
- **Grid-based model:** drift is modeled as a 2D function of m/z and RT, not a single scalar offset.
- **Tolerance tightening, not widening:** calibration typically lowers true precursor tolerance from ~20 ppm to ~4 ppm, improving specificity while preserving sensitivity.

---

## Contrast with MetaMorpheus calibration

| | MSFragger | MetaMorpheus |
|---|---|---|
| Error model | 2D grid: m/z × RT | 1D scan-number sliding window |
| Correction applied to | Masses directly (before main search) | Spectrum peak m/z values in mzML |
| Bootstrap rounds | 2 (narrow search + optimization sweep) | 3 (iterative search-calibrate loops) |
| Tolerance estimation | FDR-optimal sweep over candidate values | `median + k×IQR` formula |
| Output | Optimized `fragmentmasstolerance` in params | Per-file `.toml` with `PrecursorMassTolerance` / `ProductMassTolerance` |

---

## License

- **MSFragger:** Free for academic research, non-commercial, and educational use under an academic license. Commercial use requires a paid license via Fragmatics, LLC. Distributed as a single JAR; source not publicly available.
- **FragPipe:** GPL-3.0 for the GUI/pipeline wrapper. Each bundled tool (MSFragger, Philosopher, PTM-Shepherd, etc.) retains its own license. Users must comply with both the FragPipe license and the licenses of each included tool.

For use in this project: MSFragger cannot be vendored (source unavailable; commercial license required for non-academic use). FragPipe wrapper code is GPL-3.0 and could be referenced but not incorporated without license review.
