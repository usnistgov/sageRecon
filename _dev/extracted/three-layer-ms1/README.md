# Three-layer MS1 signal fate (extracted from recon)

## What this is

An algorithm that splits the total MS1 TIC of a run into four buckets:

1. **Non-peptidic** — signal outside 400-1200 m/z.
2. **Peptide-like, never sampled** — in range, but no MS2 was triggered on it.
3. **Peptide-like, sampled but not identified** — an MS2 exists, but no PSM.
4. **Peptide-like, identified** — an MS2 exists and a PSM passed the q-value cut.

It also computes two derived rates (sampling efficiency, ID efficiency), a
comparison table, and a breakdown of the identified TIC by delta mass
(unmodified vs modified, plus the top N modifications).

The measurement is a **DDA-METHOD diagnostic**, not sample reconnaissance. That
is why it left recon.

## Status

**This code is NOT compiled.** It is not part of any crate. Nothing in
`recon-tool/` references it, and `cargo build` never reads it.

It is kept here as source so the other tool (sageGUI) still has it.

- Removed from recon on **2026-09-02**, on Ben's instruction.
- Extracted from the working tree at commit **`84b8f0c`**. HEAD moved to
  **`ddaf5ab`** during the same session, from concurrent work by another agent.
  That commit touched only doc-comments in `recon-tool/src/calibration.rs`, so
  the extracted source is identical under either commit.
- Earlier history: the metric was removed from the unified `run`/`analyze`
  report on 2026-09-01 but kept behind a hidden `signal-fate --three-layer`
  flag. That flag is now gone too, so the release binary no longer carries the
  code.

⚠ `AGENTS.md` still says three_layer_ms1 "survives only as
`signal-fate --three-layer`. Do not add it back." That entry is **superseded**
by this removal and needs updating (a different agent owns the .md files).

## Files

| file | what it is |
|---|---|
| `three_layer_ms1.rs` | The algorithm, moved verbatim out of `recon-tool/src/mzml.rs`. Only a header doc-comment and a `use` block were added. |
| `cli_integration.rs` | A verbatim record of the plumbing fragments from `recon-tool/src/main.rs` and `recon-tool/src/signal_fate.rs`. **Not valid standalone Rust** — a reference, so the wiring does not have to be re-derived. |

## What moved

All of it came out of `recon-tool/src/mzml.rs` (712 lines, one contiguous
block). Every item below had no caller outside the three-layer feature.

Types:
- `Ms2PrecursorInfo`
- `ThreeLayerMs1Fate`
- `ModificationBreakdown`
- `ModificationTic`
- `MzRtRegion` (private)
- `SortedRegionIndex` (private) and its `impl`

Functions:
- `extract_ms2_precursor_info`
- `extract_ms2_precursor_info_from_reader` (private)
- `compute_three_layer_ms1_fate`
- `compute_mod_breakdown`
- `deduplicate_regions` (private)
- `print_three_layer_ms1_fate`
- `print_mod_breakdown`

## Interface: what it needs from a host crate

These items **stayed in recon** because live, non-three-layer code still calls
them. A host crate must supply equivalents. This is the full dependency
surface.

| item | recon location | why it stayed |
|---|---|---|
| `Ms1Spectrum` | `mzml.rs` | Used by polymer detection, precursor intensity, improved MS1 fate. Also re-exported from `lib.rs`. |
| `PrecursorQuery` | `mzml.rs` | Same. Re-exported from `lib.rs`. |
| `build_precursor_queries_from_psms` | `mzml.rs` | Also called by the `--ms1-intensity` path in `main.rs`. |
| `extract_precursor_intensities` | `mzml.rs` | Same. Re-exported from `lib.rs`. |
| `extract_scan_from_id` | `mzml.rs` | Also called by `extract_ms2_from_reader`. **Private** to that module — a port must copy it or make it public. |
| `Psm` | `sage_results.rs` | The whole result parser. `compute_mod_breakdown` reads its `delta_mass_corrected` field. |

External crates used: `anyhow`, `flate2`, `mzdata`, `serde`.

Shapes the code assumes:

- `Ms1Spectrum` — `{ scan: u32, rt: f64, mz: Vec<f64>, intensity: Vec<f64>, tic: f64 }`
- `PrecursorQuery` — `{ mz: f64, rt: f64, charge: u32, .. }`
- `Psm` — field `delta_mass_corrected: f64`

## Notes for a port

- `extract_ms2_precursor_info` mirrors Sage's own precursor-m/z rule: it falls
  back to the isolation window target when the selected ion m/z is absent or
  zero. Source is `sage-cloudpath/src/mzml.rs` at the pinned rev `df92199`.
  Keep that fallback — recon must agree with Sage on which MS2 spectra exist.
- The 400-1200 m/z peptide-like window is hardcoded as `MZ_MIN` / `MZ_MAX`
  inside `compute_three_layer_ms1_fate`.
- `compute_three_layer_ms1_fate` returns `mod_breakdown: None`. The caller runs
  `compute_mod_breakdown` and assigns it, as `cli_integration.rs` shows.
- The default tolerances used by the old CLI were 20.0 ppm and a 1.0 minute RT
  window.
- The mod breakdown bins delta mass to 0.01 Da and calls anything under 0.1 Da
  unmodified.
