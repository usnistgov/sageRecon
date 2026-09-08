# Domain Primer

Background knowledge for working on this tool. Distilled from the original
`CONTEXT.md` (now retired). This is teaching material — the "what is a delta
mass" layer —
kept out of NOTES so NOTES stays skimmable reasoning. See also
`reference-notes/glossary.md` for term definitions.

---

## What is this tool?

A **reconnaissance tool** for unfamiliar mass spectrometry proteomics data.
Given an mzML file (mass spec data) and a FASTA file (protein database), it runs
a single Sage open search and reports:

1. **What modifications are present** — delta-mass discovery (PTM scouting)
2. **Where the signal is going** — explained vs. unexplained, by count AND intensity
3. **What molecule classes are present** — glycans (via oxonium ions), polymer contamination

Target user: an expert analyzing a new species/tissue who wants "what's here,
what's worth chasing, what am I missing" — a fast, free, open-source spiritual
successor to Byonic's Preview.

## What is an open search?

In a **closed search**, you specify which modifications to look for (e.g.,
"oxidation on M, phospho on STY"). The engine only considers those.

In an **open search**, you use a very wide precursor mass tolerance (e.g., Sage config
`da: [-500, 100]`) so the engine can match peptides even with unexpected mass shifts.
The **delta mass** (experimental − calculated) reveals what modifications are
actually present. Note: Sage applies tolerance to the *experimental* mass, so
`da: [-500, 100]` means experimental can be up to 500 Da lighter OR 100 Da heavier
than theoretical → delta window is **−100..+500 Da** (not "−500 to +100").

## What is a delta mass?

```text
delta_mass = experimental_precursor_mass - calculated_peptide_mass
```

An oxidation (+15.9949 Da) produces a delta mass of ~16 Da. Histogramming delta
masses across all PSMs reveals which modifications are prevalent.

## What is isotope error?

Mass spectrometers sometimes pick the wrong isotope peak as the monoisotopic
mass. Picking M+1 instead of M+0 makes the measured mass ~1.003 Da too high.
Sage reports this as `isotope_error`; we correct for it:

```text
corrected_delta = delta_mass - (isotope_error × 1.0086649158849)
```

## What are oxonium ions?

Glycopeptides (peptides with sugar modifications) produce characteristic
fragment ions called **oxonium ions** in MS2 spectra. The most diagnostic is
HexNAc at m/z 204.0867. Detecting these indicates glycosylation is present.

## What is polymer contamination?

PEG, PPG, and polysiloxanes are common lab contaminants producing characteristic
ion series in MS1 spectra. Reporting their %TIC (total ion current) helps assess
sample quality.

## Why report mass error in a scout run?

**Critical for users who want to run a tight search after scouting.** The
fragment ppm distribution from an open search tells you the actual mass accuracy
of the data, which informs the precursor tolerance for a follow-up closed search.

- **Fragment ppm** (not precursor ppm) is the true mass-accuracy metric in open
  search — fragment ions are matched with tight tolerance regardless of
  precursor delta mass.
- Tools like MSFragger and MetaMorpheus report this for the same reason.
- Example from our test data: fragment ppm median 2.21, 95th percentile 5.42 →
  suggests a 10–15 ppm precursor tolerance for a follow-up closed search.

**Precursor ppm in open search is NOT mass accuracy** — it reflects the delta
mass distribution (modifications), not instrument error. The median may be
meaningful, but the mean is garbage (e.g., 17,688 ppm in our test data due to
large delta masses). See the "intentional, not bugs" note in NOTES.
