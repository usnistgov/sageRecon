# MetaMorpheus: Mass Error Estimation, Calibration, and G-PTM-D

**Source:** `reference/MetaMorpheus/` (vendored, MIT license, Smith/Coon lab)
**Key papers:** Solntsev et al. 2018 (JASMS) for G-PTM-D; see README for calibration paper
**Language:** .NET / C#

---

## Calibration

### Relevant files

- `TaskLayer/CalibrationTask/CalibrationTask.cs` — top-level orchestrator
- `TaskLayer/CalibrationTask/CalibrationParameters.cs` — user-facing config
- `EngineLayer/Calibration/CalibrationEngine.cs` — applies corrections to scans
- `EngineLayer/Calibration/DataPointAcquisitionEngine.cs` — extracts labeled (observed, theoretical) m/z pairs from confident PSMs
- `EngineLayer/Calibration/DataPointAquisitionResults.cs` — computes median ppm error and IQR
- `EngineLayer/Calibration/LabeledDataPoint.cs` — single training point
- `TaskLayer/FileSpecificParameters.cs` — per-file tolerance storage as TOML

### Initial wide tolerances

Hard-coded constants in `CalibrationTask.cs`:
```
InitialPrecursorTolerance = 10 ppm
InitialProductTolerance   = 30 ppm
```
If the first search round fails sufficiency thresholds (≥16 PSMs, ≥40 MS1 datapoints, ≥80 MS2 datapoints), both are doubled (`×2.0`) and the round retried once. If still insufficient, calibration is abandoned and the uncalibrated file is passed downstream.

### Chicken-and-egg solution: three-round iterative bootstrap

MetaMorpheus bootstraps with wide tolerances and iterates — it does not require pre-calibrated data and does not do a two-stage "calibrate then search":

| Round | What happens |
|-------|-------------|
| 1 | Search at 10/30 ppm → collect PSMs → estimate new tolerances from error distribution → apply first mass correction to file |
| 2 | Search calibrated file with updated tolerances → collect PSMs → estimate tolerances again → apply second mass correction |
| 3 | Search doubly-calibrated file → test improvement criterion → keep round 2 or round 3 output |

The test `CalibrationRetainsProperTolerances` confirms precursor tolerances shrink through the rounds, e.g., 10 → 8.6 → 7.3 ppm.

### Tolerance estimation formula

Computed in `DataPointAquisitionResults`:
```
new_precursor_ppm = round( 3 × IQR_precursor + |median_precursor_ppm| , 1 )
new_product_ppm   = round( 6 × IQR_product   + |median_product_ppm|   , 1 )
```
Where `median_ppm_error = median( (observed − theoretical) / theoretical × 1e6 )` across good PSMs, and IQR is the interquartile range of the same distribution. Product uses a larger multiplier (6×) to account for lower MS2 mass accuracy.

**Guardrail:** the new tolerance is never allowed to exceed the previous tolerance. This prevents runaway expansion from isotopologue misassignment inflating the IQR. `DataPointAquisitionResults.cs` notes that "most-abundant" precursor mode can pathologically widen the precursor IQR; `GetObservedMonoisotopicMass()` strips the isotopologue offset before computing statistics.

### Mathematical model: scan-local sliding window, not regression

`CalibrationEngine.cs` fits **no polynomial or linear regression**. The approach is empirical and time-local:

**1. Per-scan intensity-weighted error** (`PopulateErrors`):
```
relError_i = (experimentalMz − theoreticalMz) / theoreticalMz
error[scan] = Σ(relError_i × intensity_i) / Σ(intensity_i)
```
Intensities are unlogged so high-intensity peaks dominate. Scans with no datapoints default to 0; gaps between populated scans are linearly interpolated.

**2. ±100-scan sliding window average** (`SmoothErrors`):
```
smoothed[i] = mean( error[i−100 … i+100] )
```

**3. Apply correction** (`CalibrateScan`):
```
corrected_mz = original_mz × (1 − smoothed_relative_error)
```

The model is **scan-number dependent (time-dependent), not m/z-dependent**. It assumes mass drift is slow and smooth across the chromatographic run. No regression terms for m/z, RT, charge state, or intensity are included in the correction itself.

This is meaningfully different from a common approach of fitting a linear or polynomial model of `ppm_error ~ m/z` — MetaMorpheus instead models drift as a function of time in the run.

### MS1 vs MS2: both calibrated independently

- `ms1RelativeErrors` and `ms2RelativeErrors` are computed and smoothed in separate arrays.
- MS1 scans → corrected with `ms1SmoothedErrors`.
- MS2 scans → corrected with `ms2SmoothedErrors`.
- Each MS2 scan's precursor fields (`SelectedIonMZ`, `IsolationMz`, `SelectedIonMonoisotopicGuessMz`) are corrected using the most-recent **MS1** smoothed error, not the MS2 error.
- **Exception:** LowCID (low-resolution ion trap) data skips MS2 calibration entirely — `ms2SmoothedErrors` is left empty.

### Improvement criterion (`CalibrationHasValue`)

Round 3 output is kept over round 2 if:
- **Fast pass:** PSM count AND unique peptide count both strictly increased, OR
- **All four hold:** PSM count didn't decrease AND unique peptide count didn't decrease AND (`|median precursor ppm| ≤ 1` OR decreased) AND (`|median product ppm| ≤ 1` OR decreased).

Otherwise round 2's output is used.

### Per-file tolerance storage

After calibration, `FileSpecificParameters.PrecursorMassTolerance` and `ProductMassTolerance` are updated and written as `<original>-calib.toml` alongside `<original>-calib.mzML`. Downstream tasks (G-PTM-D, Search) load these TOMLs and merge them with global `CommonParameters` via `SetAllFileSpecificCommonParams()`. After calibration, tolerances are always stored as `PpmTolerance` objects.

---

## G-PTM-D (Global PTM Discovery)

### Relevant files

- `TaskLayer/GPTMDTask/GPTMDTask.cs` — task orchestrator
- `EngineLayer/Gptmd/GptmdEngine.cs` — core logic
- `EngineLayer/Gptmd/GptmdResults.cs` — output container (`Dictionary<string, HashSet<Tuple<int, Modification>>>`)
- `EngineLayer/Gptmd/IGptmdFilter.cs` — pluggable localization filter interface
- `EngineLayer/PrecursorSearchModes/DotMassDiffAcceptor.cs` — multi-notch acceptor (binary search on sorted notch list)

### What it is

G-PTM-D is a **precomputed-notch, closed-database search followed by database expansion**. It is not an open search. Its output is an augmented protein XML database (`*GPTMD.xml`) that feeds a subsequent normal closed-search task. The two-step design (G-PTM-D search → standard search with expanded DB) is central to how it controls false positives.

### Step-by-step algorithm

**Step 1 — Build the notch set** (`GptmdTask.GetAcceptableMassShifts()`):

From the default GPTMD mod list ("Common Artifact", "Common Biological", "Metal" — "Less Common" is commented out):
- Direct notch: `+mod.MonoisotopicMass` for each mod in the list
- Replacement notch: `gptmd_mod_mass − existing_mod_mass` for sites already bearing a modification (accounts for swapping one PTM for another)
- Combo notch: loaded from `combos.txt` — pre-tabulated pairs whose sum is added as a single notch, enabling detection of two co-occurring PTMs as one precursor shift; handled recursively in `GetPossibleMods()`
- Zero notch: always included so unmodified peptides are found

Result: O(hundreds) of notches, deduplicated (rounded to 5 decimal places) and sorted by absolute value.

**Step 2 — Multi-notch ClassicSearch:**

`DotMassDiffAcceptor` wraps the notch list with binary-search lookup. Precursor windows remain **tight (e.g., 5 ppm)** but are centered at each known shift, not only at zero. Each PSM is tagged with which notch it matched.

**Step 3 — Cross-file FDR:**

FDR is computed across all files pooled, not per-file. The code comment states: "GPTMD doesn't work as well if you do FDR on a file-by-file basis. Presumably this is because it takes multiple files to get enough PSMs for all the different notches." PSMs with `QValueNotch ≤ 0.05` pass to the engine.

**Step 4 — GptmdEngine: localization and annotation** (parallel per PSM):
1. Find which GPTMD mods are consistent with the PSM's observed precursor mass gap (`GetPossibleMods()`)
2. For each candidate mod, iterate over every position in the peptide
3. Check motif + location restriction (`ModFits()`)
4. Create a virtual modified peptide, fragment it, score against MS2 spectrum
5. Optionally apply `IGptmdFilter` plugins (`ImprovedScoreFilter`, `DualDirectionalIonCoverageFilter`, `UniDirectionalIonCoverageFilter`, `FlankingIonCoverageFilter`)
6. Annotate all positions within `ScoreTolerance = 0.1` score units of the best — ties are kept intentionally (localization ambiguity is preserved rather than arbitrarily resolved)

**Step 5 — Write augmented database:**

`GptmdResults.Mods` maps `proteinAccession → Set<(siteIndex, Modification)>`. Written as a new protein XML; used as input for the downstream Search task.

### G-PTM-D vs open search (Sage / MSFragger)

| | G-PTM-D | Open Search (Sage / MSFragger) |
|---|---|---|
| Precursor window | Tight per-notch (e.g., 5 ppm) | Wide (e.g., ±500 Da) |
| Modification space | Predefined list (~200–300 known PTMs) | Any mass shift whatsoever |
| Novel PTM discovery | No — only finds mods on the list | Yes |
| FDR control | Per-notch FDR, cross-file | Global or mass-bin FDR |
| Localization | Fragment scoring during the G-PTM-D run itself | Usually post-hoc |
| Output | Augmented database → 2nd search required | PSMs with delta masses, one step |
| False positive risk | Low (tight windows per known mod) | Higher (open window) |

The core design difference: G-PTM-D leverages prior biochemical knowledge to avoid open-search FDR inflation. It cannot find a PTM with a mass not on its predefined list; a true open search can. The trade-off is specificity vs. discovery scope. G-PTM-D is a database-expansion tool, not a standalone result producer.

---

## Documented caveats (from source comments and code)

- **Localization ambiguity:** `GptmdEngine.cs` — "TODO: not necessarily here. I think we're creating ambiguity. If we're going to add a gptmd mod to a peptide that already has that mod, then we need info to suggest that it is at a position other than that in the database."
- **LowCID MS2 skip:** `CalibrationEngine.cs` — MS2 calibration is silently skipped for ion trap low-res data.
- **Isotopologue IQR inflation:** `DataPointAquisitionResults.cs` — "most-abundant" precursor mode can pathologically widen the precursor IQR; `GetObservedMonoisotopicMass()` strips the isotopologue offset before statistics.
- **Tolerance guardrail:** Calibration must never write a tolerance wider than the one it searched with — "that would explode the candidate space and memory in downstream GPTMD/Search."
- **Cross-file FDR requirement:** G-PTM-D "doesn't work as well" with per-file FDR; needs multi-file pooling for reliable per-notch FDR.
- **"Less Common" mods disabled by default:** `GptmdParameters.cs` — commented out of the default GPTMD mod list.
