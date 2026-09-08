# Vendoring Checklist — Phase 0

Two kinds of material: source repos (clone as-is) and distilled notes
(research with Perplexity/Sonar, write up, vendor the writeup — not raw
search results).

## 1. Source repos to clone

```
git clone https://github.com/lazear/sage reference/sage
git clone https://github.com/wfondrie/mzsniffer reference/mzsniffer
git clone https://github.com/Nesvilab/PTM-Shepherd reference/ptm-shepherd
```

Plus your own:
```
# copy (not clone) intensityWeighting folder into the new repo
reference/intensity-weighting/  ← proteomicScripts/intensityWeighting
```

Also grab, from inside the Sage repo or its releases:
- an example `config.json`
- a sample output file (`results.sage.tsv` or `.parquet`) from a test run,
  so there's real ground truth for output format

## 2. Distilled notes to produce via Perplexity, then vendor as markdown

For each, the ask isn't "give me a summary" — it's "verify against multiple
sources and flag anything uncertain or version-specific," since these get
treated as ground truth once vendored in air-gapped.

**`reference-notes/sage-config-and-gotchas.md`**
> Prompt: "What are the key configuration options in Sage (lazear/sage)
> proteomics search engine's config.json, specifically for open/wide-window
> search mode? What are common gotchas or non-obvious settings people have
> run into? Cite the actual DOCS.md content and any GitHub issues discussing
> config problems."

**`reference-notes/unimod-decomposition.md`**
> Prompt: "How does Unimod modification matching/decomposition typically
> work when a mass shift could correspond to more than one modification, or
> to a combination of two? What tolerance windows are standard in the field
> (ppm/Da) for matching an observed delta mass to a Unimod entry? Are there
> known ambiguous mass collisions worth flagging (e.g. trimethylation vs.
> acetylation)?"

**`reference-notes/ptm-shepherd-methodology.md`**
> Prompt: "Explain PTM-Shepherd's (Nesvilab) mass-shift clustering
> methodology in detail: how peaks are called from a delta-mass histogram,
> how per-position rescoring works for localization, and how retention-time
> shift and spectral similarity are used to distinguish real PTMs from
> artifacts/adducts. Include the formaldehyde-adduct example if findable."

**`reference-notes/oxonium-ions.md`**
> Prompt: "What are the standard diagnostic oxonium ions used to detect
> glycopeptides in HCD MS2 spectra? Give exact m/z values for the common
> ones (HexNAc, Hex-HexNAc, etc.), typical intensity thresholds used in
> glycoproteomics screening tools, and which ions are most diagnostic vs.
> ambiguous."

**`reference-notes/polymer-contaminant-ions.md`**
> Prompt: "What are common polymer contaminant signatures in LC-MS/MS
> proteomics data (PEG, PPG, Triton, etc.) — characteristic mass
> differences/series, and how does mzsniffer (wfondrie/mzsniffer) detect
> them at the MS1 level? Include any known m/z reference lists."

**`reference-notes/mgf-mzml-intensity-differences.md`**
> Prompt: "How does precursor and fragment ion intensity reported in an mgf
> file (as produced by msconvert) differ from intensity values available
> directly in mzML? Are there known discrepancies or approximations in
> msconvert's mgf export worth accounting for when porting an
> intensity-weighting analysis from mgf-based to direct mzML-based
> extraction?"

## 3. Optional, if time allows
- Sage's Rust crate docs (docs.rs) if planning to use `sage-core` as a
  library dependency rather than shelling out to the binary — worth a
  distilled note on its public API surface specifically
