# Sage Config & Gotchas — Open/Wide-Window Search

Source: Sage's own `DOCS.md` (github.com/lazear/sage/blob/master/DOCS.md)
and Lazear's project writeup (lazear.github.io/sage), verified July 2026.

## Open search tolerance syntax (CONFIRMED by Michael Lazear, 2026-08-17)
Sage applies the precursor tolerance to the **experimentally observed** mass,
and searches for theoretical peptides within `[expmass + lo, expmass + hi]`.
So the config bound sign is the OPPOSITE of the delta-mass sign it produces:

> "It is correct but yeah counter intuitive. Sage applies tolerances to the
> experimentally observed m/z values. So -500 searches for theoretical peptides
> that have a +500 delta mass (e.g. 500 Da below the experimental mass)."
> — Michael Lazear (Sage author), 2026-08-17

To run a **−100 to +500 Da delta-mass** open search (losses to 100 Da, adducts
to 500 Da), the config field is:
```json
"da": [-500, 100]
```
i.e. `da: [-500, 100]` ⇒ theoretical peptide is 500 Da lighter to 100 Da heavier
than observed ⇒ **delta (expmass − calcmass) ranges −100 to +500**. This is
correct and intended — verified against every open-search TSV (all show delta
[−100, +500]) AND confirmed by the author. **Do not "fix" the sign** (locked —
don't relitigate). It is genuinely counterintuitive; that's why this note exists.

## Decoy handling — a real gotcha
Sage can either use decoys already present in your supplied FASTA, or
generate them internally. If `database.generate_decoys` is left `true`
(the default, or if unspecified) and your FASTA also already contains
decoy sequences matching `database.decoy_tag`, Sage will **ignore your
supplied decoys and generate its own**. If `decoy_tag` doesn't match what's
actually in your FASTA, Sage will treat your real supplied decoys as if
they were target hits — silently corrupting FDR calculation with no error
raised. Confirm this setting explicitly for any FASTA you didn't generate
yourself, rather than trusting the default.

Sage reverses tryptic peptides (not whole proteins) for its internally
generated decoys, specifically so the picked-peptide approach to FDR can
be used correctly.

## Running it
```
git clone https://github.com/lazear/sage.git
cd sage
cargo run --release tests/config.json
```
CLI usage: `sage [OPTIONS] <parameters> [mzml_paths]...` — mzML paths
passed on the command line override any mzML files listed in the config
file itself, which is handy for scripting runs across many samples without
editing the config each time.

## Known limitation vs. MSFragger at very wide windows
Per Lazear's own writeup: Sage's identification rate at the widest
open-search windows (hundreds of Da) is somewhat behind MSFragger's
**unless chimeric/co-fragmenting spectra search is explicitly turned on**.
Worth enabling that setting rather than assuming Sage's defaults alone
match MSFragger-equivalent open-search sensitivity out of the box.

## Built-in scoring/FDR (relevant to Phase 2/3)
Sage includes PSM rescoring via a built-in linear discriminant analysis
(LDA) model, PEP calculation using a non-parametric (KDE) model, and FDR
via target-decoy competition with picked-peptide/picked-protein approaches
— all usable directly from Sage's own output without needing a separate
rescoring tool (e.g. Percolator/Mokapot) bolted on.

## Sage TSV Output Column Reference

The `results.sage.tsv` file contains these columns (in order):

| # | Column | Type | Notes |
|---|--------|------|-------|
| 0 | `psm_id` | u64 | Unique PSM identifier |
| 1 | `peptide` | String | Peptide sequence with modifications |
| 2 | `proteins` | String | Protein accessions (semicolon-separated) |
| 3 | `num_proteins` | u32 | Number of proteins matched |
| 4 | `filename` | String | Source mzML filename |
| 5 | `scannr` | String | Native spectrum ID, e.g., `"controllerType=0 controllerNumber=1 scan=9681"` |
| 6 | `rank` | u32 | PSM rank (1 = best) |
| 7 | `label` | i32 | Target (1) or decoy (-1) |
| 8 | `expmass` | f64 | Experimental precursor mass (Da) |
| 9 | `calcmass` | f64 | Calculated peptide mass (Da) |
| 10 | `charge` | u32 | Precursor charge state |
| 11 | `peptide_len` | u32 | Peptide length |
| 12 | `missed_cleavages` | u32 | Number of missed cleavages |
| 13 | `semi_enzymatic` | u32 | 0 = fully tryptic, 1 = semi-tryptic |
| 14 | `isotope_error` | f64 | Isotope error (output as float, typically 0.0, 1.0, 2.0, etc.) |
| 15 | `precursor_ppm` | f64 | Precursor mass error (ppm) |
| 16 | `fragment_ppm` | f64 | Fragment mass error (ppm) |
| 17 | `hyperscore` | f64 | Sage hyperscore |
| 18 | `delta_next` | f64 | Score difference to next-best PSM |
| 19 | `delta_best` | f64 | Score difference to best PSM |
| 20 | `rt` | f64 | Retention time (minutes) |
| 21 | `aligned_rt` | f64 | Aligned retention time |
| 22 | `predicted_rt` | f64 | Predicted retention time |
| 23 | `delta_rt_model` | f64 | RT prediction error |
| 24 | `ion_mobility` | f64 | Ion mobility value |
| 25 | `predicted_mobility` | f64 | Predicted ion mobility |
| 26 | `delta_mobility` | f64 | Mobility prediction error |
| 27 | `matched_peaks` | u32 | Number of matched fragment peaks |
| 28 | `longest_b` | u32 | Longest consecutive b-ion series |
| 29 | `longest_y` | u32 | Longest consecutive y-ion series |
| 30 | `longest_y_pct` | f64 | Longest y-ion series as fraction |
| 31 | `matched_intensity_pct` | f64 | Fraction of intensity explained |
| 32 | `scored_candidates` | u64 | Number of candidates scored |
| 33 | `poisson` | f64 | Poisson score |
| 34 | `sage_discriminant_score` | f64 | LDA discriminant score |
| 35 | `posterior_error` | f64 | Posterior error probability |
| 36 | `spectrum_q` | f64 | Spectrum-level q-value |
| 37 | `peptide_q` | f64 | Peptide-level q-value |
| 38 | `protein_q` | f64 | Protein-level q-value |
| 39 | `ms2_intensity` | f64 | Total MS2 intensity |

**Gotchas discovered during parsing:**
- `scannr` is a native ID string, not an integer — extract scan number with regex
- `isotope_error` is output as float (0.0) even though it's conceptually an integer
- `semi_enzymatic` is 0/1 integer, not boolean

## Isotope Error and Delta Mass Artifacts

### The 52.91 Da Peak — Isotope Selection Artifact

A delta mass of exactly **+52.905 Da** (often seen as ~52.91 Da in histograms) typically represents a **triply ¹³C isotope shift** — an instrument peak-picking artifact, not a biological modification.

**What happens:**
1. High-resolution mass spectrometers (Orbitrap, Q-TOF) sometimes miss the monoisotopic peak (M+0)
2. Instead, the instrument selects the third heavy isotope peak (M+3) for fragmentation
3. Each ¹³C atom adds ~1.0033 Da, so M+3 selection shifts the measured precursor mass by ~3 × 1.0033 Da ≈ 3.01 Da
4. But the **charge state** matters: for a +3 charge, the m/z shift is 3.01/3 ≈ 1.00 Da, which gets multiplied back to mass as 3.01 Da
5. For higher charge states or different isotope selections, you can get larger apparent delta masses

**The 52.91 Da case specifically:**
- This appears when the instrument selects an isotope peak that's ~53 Da heavier than the monoisotopic mass
- This can happen with very high charge states or when the monoisotopic peak is below detection threshold
- **No actual chemical or biological modification is present** — it's purely an instrument artifact

**Reference:** PMID 40160755 — "Comparison of the Human Plasma Peptides from the Fit of Fragmentation Spectra versus Accurate Monoisotopic Precursor Mass"

**Key insight from the paper:**
> "In nature, ionized peptides with heavy isotopes and hydrogen rearrangements show a broad mass distribution with signals at discrete delta mass values from −3 to +5 Da by mass spectrometry (MS). For many peptides, the intensity of the +1 or +2 Da isotope exceeds the signal from the monoisotopic mass."

### Isotope Error Correction Formula

Sage reports `isotope_error` as the number of isotope shifts detected. To get the true delta mass:

```
corrected_delta = (expmass - calcmass) - (isotope_error × 1.0086649158849)
```

Where 1.0086649158849 Da is the neutron mass (mass difference between ¹³C and ¹²C).

### Expected Isotope Error Distribution

From narrow-search validation (Phase 2):
- isotope_error -1: ~13%
- isotope_error 0: ~50%
- isotope_error +1: ~17%
- isotope_error +2: ~11%
- isotope_error +3: ~8%

After correction, ~77% of PSMs cluster at delta mass 0.00 Da (unmodified).

## Prior art worth checking before building from scratch
**SagePTMScanner** (OpenMS WebApps project, repo
`JohannesvKL/SageAdapterApp`, described in the OpenMS WebApps paper,
arXiv:2411.13189) already wraps Sage specifically for open searches with
PTM annotation, producing both a spectra view and a PTM table/graph. It's
a Streamlit/Python web app, not built for an air-gapped environment or for
cluster-level confidence scoring (frequency/RT/spectral-similarity/
fragment-rescoring) as far as the paper describes — but worth cloning and
reading directly before assuming a from-scratch build is necessary. Either
it covers more than expected (fork/extend it), or it confirms exactly what
gap this project fills.
