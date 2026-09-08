# mzML Instrument Metadata: Analyzer Type and Tolerance Derivation

> **The authoritative term list and tolerance table now live in
> [`ms2-analyzer-tolerance-table.md`](ms2-analyzer-tolerance-table.md)**, derived
> from a pinned HUPO-PSI `psi-ms.obo` snapshot by
> `testing/scripts/psi_ms_analyzer_terms.py`. This file remains the notes on
> WHERE the metadata sits in an mzML and how converters behave. Where the two
> disagree, the derived table wins.

**Standard:** HUPO-PSI mzML 1.1, PSI-MS controlled vocabulary
**Converters:** ProteoWizard msconvert, ThermoRawFileParser

---

## Structure overview

mzML encodes instrument metadata via `<cvParam>` elements using PSI-MS CV accessions. The relevant sections:

- `<cvList>` — references to PSI-MS CV and other ontologies
- `<instrumentConfigurationList>` — analyzer type, detector, source
- `<spectrumList>` — per-spectrum metadata including ms level, scan filter strings

---

## Where analyzer type lives

```
instrumentConfigurationList
  └─ instrumentConfiguration
       └─ componentList
            └─ analyzer
                 └─ cvParam (accession = child of MS:1000443)
```

`MS:1000443` is *mass analyzer type*. Full child set, corrected 2026-08-28
against `psi-ms.obo` and `ThermoRawFileParser/Writer/OntologyMapping.cs`:

| Accession | Analyzer | Class |
|-----------|----------|-------|
| `MS:1000484` | orbitrap | high-res |
| `MS:1000079` | fourier transform ion cyclotron resonance | high-res |
| `MS:1000084` | time-of-flight | high-res |
| `MS:1003379` | asymmetric track lossless TOF (Astral) | high-res |
| `MS:1000264` | **ion trap** (generic parent) | LOW-res |
| `MS:1000082` | quadrupole ion trap | LOW-res |
| `MS:1000291` | linear ion trap | LOW-res |
| `MS:1000078` | axial ejection linear ion trap | LOW-res |
| `MS:1000083` | radial ejection linear ion trap | LOW-res |
| `MS:1000081` | quadrupole | LOW-res |
| `MS:1000080` | magnetic sector | not supported |
| `MS:1000254` | electrostatic energy analyzer | not supported |

**⚠ TWO CORRECTIONS TO THE EARLIER VERSION OF THIS TABLE (2026-08-28).**

1. It listed `MS:1000078` as "linear ion trap". That is wrong: `MS:1000078` is
   *axial ejection* linear ion trap. Plain "linear ion trap" is `MS:1000291`.
2. **It omitted `MS:1000264`, and that omission was load-bearing.**
   ThermoRawFileParser maps its `MassAnalyzerITMS` to the GENERIC parent
   `MS:1000264`, not to any of the specific children. A detector built from the
   old table would therefore have failed to recognise every
   ThermoRawFileParser-converted ion-trap file — exactly the case the whole
   feature exists to catch.

**These are NOT "written consistently" across converters.** Measured on this
repo's own files: `B.naive_01steady-state.mzML.gz` is a **Q Exactive Plus** and
declares its analyzer as `MS:1000079` FT-ICR. A Q Exactive Plus has no ICR cell.
ThermoRawFileParser emitted FTICR for every Orbitrap until 1.4.4, whose release
notes read *"Using CVTerm 'Orbitap' instead of 'FTICR' for Orbitrap-based
instruments (closes #177)"* (2024-05-10). B.naive was converted with 1.4.2; the
other two files with 1.4.4, and they declare `MS:1000484`.

---

## Per-spectrum metadata

Each `<spectrum>` carries:

- `MS:1000511` (*ms level*): value `1` for MS1, `2` for MS2.
- Centroid/profile flag (less standardized; occasional mislabeling bugs in msconvert have been reported and fixed).
- Thermo converters add a **filter string** (`MS:1000512`) whose first token is
  the analyzer that acquired that scan: `FTMS`, `ITMS`, `TOFMS`, `ASTMS`,
  `TQMS`, `SQMS`, `Sector`.

**⚠ CORRECTED 2026-08-28 — this note previously called the filter string "less
reliable than the CV-based analyzer terms". For the question recon actually asks,
the ranking is the other way round, and recon prefers the filter string:**

1. It is **per-scan**, so it can express "FT MS1, ion-trap MS2" within one run.
   The componentList is file-level and cannot.
2. It is **immune to the FTICR mislabel above** — Orbitrap and FT-ICR both report
   `FTMS`, and both are high-resolution, so the tolerance is unchanged either way.
3. It was present on **100% of 229,348 spectra** across all three reference files.

The CV term remains the correct fallback for non-Thermo data, which has no filter
string. mzdata's own mzML reader uses the same `ITMS` test internally.

---

## Using analyzer metadata for tolerance initialization

### Approach

1. **Parse** `instrumentConfiguration/componentList/analyzer/cvParam` — find accessions that are children of `MS:1000443`.
2. **Map** accession → analyzer category (high-res vs low-res).
3. **Derive** initial tolerances from category, before any calibration data is available.

### Recommended starting tolerances

| MS level | Analyzer | Starting tolerance |
|----------|----------|--------------------|
| MS1 | Orbitrap / FT-ICR | 5–10 ppm |
| MS2 | Orbitrap | 10–20 ppm (or 0.02–0.03 Da) |
| MS2 | Linear/quadrupole ion trap | 0.5–1.0 Da |

Then refine per-file from high-confidence PSM error distributions (median + k×IQR or equivalent), with the guardrail that the refined tolerance never exceeds the starting tolerance.

### Hybrid instruments (e.g., Orbitrap Fusion / Lumos)

**⚠ CORRECTED 2026-08-28.** This previously stated that both analyzers "appear
in the component list". That holds for msconvert, which emits one
`instrumentConfiguration` per analyzer and points each scan at one via
`instrumentConfigurationRef`. It does **not** hold the way you would guess for
ThermoRawFileParser, which builds the component list from the analyzers **actually
used by the scans**, not from the instrument's capability. `2019-4-9_909c_0311` is
an Orbitrap Fusion Lumos — a genuine hybrid — but that run used the Orbitrap for
every scan, so only `orbitrap` is declared. That is CORRECT, not incomplete: do
not read a single declared analyzer as proof the instrument has only one.

Given a real hybrid acquisition, assign by `ms level`:

- MS1 scans → Orbitrap → ppm tolerance, full MS1 calibration
- MS2 scans → ion-trap → Da tolerance, limited or no MS2 calibration (analogous to MetaMorpheus skipping MS2 calibration on LowCID data)

This detection can be done without any PSMs — purely from the mzML header before the first search.

---

## Converter notes

### msconvert (ProteoWizard)
- Open-source; standard converter for Thermo, Waters, Bruker, Agilent raw data.
- Writes mzML with PSI-MS cvParams for instrument and spectra.
- Analyzer CV terms generally follow PSI-MS accurately; occasional centroid/profile mislabeling bugs exist but are not relevant to analyzer type.

### ThermoRawFileParser
- Open-source; uses Thermo's RawFileReader.
- Writes mzML plus optional JSON metadata with explicit instrument settings and scan settings.
- Benchmarking showed equal or better identification rates vs msconvert-derived mzML in some pipelines.
- JSON export provides structured instrument metadata as an alternative to parsing XML.

---

## Key point for recon

**mzML `instrumentConfiguration` + PSI-MS CV give a standard, converter-independent way to identify analyzer type before any PSMs are available.** This allows choosing initial MS2 tolerances (ppm vs Da) and deciding whether to skip or limit MS2 calibration for ion-trap data — the same logic MetaMorpheus applies when it detects LowCID and zeroes out `ms2SmoothedErrors`.
