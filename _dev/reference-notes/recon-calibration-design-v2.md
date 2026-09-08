# Recon Calibration Design v2: Analyzer-Aware Two-Pass Tolerance Estimation

**Status:** Design note for future redesign — not yet implemented
**Purpose:** Reference for the calibration/tolerance pass when we redesign the recon pipeline

---

## Guiding decisions

1. If mzML lacks analyzer metadata, **do not touch MS2 defaults** — leave MS2 tolerance at the generous fallback and skip analyzer-specific tightening. MS1 estimation proceeds independently since the wide open-search window is expected to tolerate ion-trap-level MS1 error regardless.
2. Initial tolerances are **generous by design** — this is a rough estimation pass, not a final search. Erring wide avoids losing true PSMs to a too-tight starting guess, which is the failure mode that would corrupt everything downstream.
3. Two-pass structure: **Search 1 (wide, analyzer-informed) → estimate MS1/MS2 tolerances → Search 2 (semi-tryptic, tightened tolerances)**. No iterative bootstrap — one round is sufficient for a fast estimation pass; recon doesn't need MetaMorpheus's convergence guarantees.

---

## Step 1 — Read mzML analyzer metadata

Parse `instrumentConfigurationList/instrumentConfiguration/componentList/analyzer/cvParam` for accessions under `MS:1000443` (mass analyzer type). Works regardless of converter (msconvert or ThermoRawFileParser both populate PSI-MS CV terms here).

- If found → classify analyzer per spectrum type (MS1 / MS2 analyzer) using the `ms level` cvParam alongside instrument config.
- If **not found or ambiguous** → do not select an analyzer-specific MS2 default; fall back to the widest MS2 tolerance in the table below and document that Search 1 ran "unclassified."

See [mzml-instrument-metadata-tolerances.md](mzml-instrument-metadata-tolerances.md) for the full CV accession list and parsing approach.

---

## Step 2 — Initial (Search 1) tolerance table by analyzer

Deliberately generous starting points for a rough first pass, not final search values.

| Analyzer (MS2) | Initial fragment tolerance | Rationale |
|---|---|---|
| Orbitrap | 30 ppm | Matches MetaMorpheus's own initial product tolerance default for its calibration first-pass search |
| QTOF | 50 ppm | Several open-search/comparison studies use 50 ppm for QTOF fragment tolerance pre-calibration; Byonic's blind default is 30 ppm — 50 ppm gives more margin |
| Ion trap (linear/quadrupole, low-res CID) | 0.8 Da | Between Byonic's blind default (0.5 Da) and literature examples using 0.7–2.5 Da for ion-trap fragment windows; 0.8 Da gives margin without being excessive |
| Unclassified / metadata missing | 0.8 Da (treat as worst case, ion-trap-like) | Conservative fallback — assume least forgiving (Da-scale) case to avoid silently under-tolerating |

**Precursor (MS1) initial tolerance:** 20 ppm regardless of analyzer. Matches MSFragger's own first-search default of ±20 ppm, applied inside the already-wide open precursor window. The wide open-search window absorbs MS1 error even for ion-trap-adjacent instruments.

**Inspiration:** The per-analyzer initial tolerance table concept is borrowed from Byonic's published blind defaults and MetaMorpheus's fixed 10 ppm/30 ppm calibration-task starting constants. We are not copying MetaMorpheus's 3-round iterative bootstrap (see Step 4).

---

## Step 3 — Search 1 → estimate MS1/MS2 tolerances

Run Search 1 (wide precursor open window, analyzer-informed initial MS2 tolerance from the table above).

From high-confidence PSMs (rank-1, target, q<0.01, near-zero delta mass), compute:

- Median and IQR of `(observed − theoretical) / theoretical × 1e6` for precursor (ppm) and fragment (ppm or Da, depending on analyzer classification).
- Derived tolerance using the MetaMorpheus-inspired formula:
  ```
  new_precursor  = median_precursor_ppm + 3 × IQR_precursor
  new_fragment   = median_fragment      + 6 × IQR_fragment
  ```
  (k=3 for precursor, k=6 for fragment — fragment error is noisier. Directly from MetaMorpheus `DataPointAquisitionResults`; see [metamorpheus-mass-error-calibration.md](metamorpheus-mass-error-calibration.md).)
- **Guardrail:** never let the derived tolerance exceed the Search 1 initial tolerance — same "no runaway widening" rule MetaMorpheus uses to prevent isotopologue misassignment from pathologically inflating the IQR.
- If MS2 metadata was missing (Step 1 fallback): **skip MS2 tolerance re-estimation** — keep the 0.8 Da fallback for Search 2 unchanged. MS1 estimation still proceeds normally.

---

## Step 4 — Search 2 (semi-tryptic, tightened tolerances)

Run Search 2 semi-tryptic, using the Search 1-derived MS1 and MS2 tolerances.

**This is not MetaMorpheus's 3-round bootstrap.** Deliberate simplification:
- MetaMorpheus's extra rounds exist to squeeze marginal PSM/tolerance improvements for a production search engine — that precision is not needed for a 60-second recon pass.
- Recon only needs an estimate to report to the user and to seed Search 2's patterns.

Search 2's output (percent_PSMs per mass shift, mod ranking, etc.) becomes the actual recon report. The derived MS1/MS2 tolerances are also surfaced to the user as recommended search parameters.

---

## What's borrowed vs. original

| Element | Origin |
|---|---|
| Per-analyzer initial tolerance table | Inspired by Byonic's published blind defaults; ppm/Da split confirmed by general practice (ppm for Orbitrap/FT, Da for ion trap) |
| `median + k×IQR` tolerance formula and guardrail against widening | Directly from MetaMorpheus `DataPointAquisitionResults` approach |
| mzML analyzer detection via PSI-MS cvParams | HUPO-PSI mzML standard; converter-agnostic (msconvert / ThermoRawFileParser) |
| Single-round (not 3-round bootstrap) calibration | Original simplification — recon doesn't need convergence guarantees, only a usable estimate |
| "Don't touch MS2 if metadata missing" | Explicit design decision — intentional deviation from analyzer-aware tightening |
