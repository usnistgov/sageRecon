# Byonic Preview output — NIST liver RM 8461, `10mg_1_A_1`

**This is the accuracy anchor for recon's digestion numbers.** It is the only
file in this project with a report from Preview, the tool recon is the open
successor to.

## Provenance (pinned)

| | |
|---|---|
| Preview version | **v3.2.0** |
| Run date | 2019-03-14 |
| Spectra | `10mg_1_A_1.mgf` (converted from `10mg_1_A_1.raw`) |
| Database | `uniprot_sprot_iso_human-2018_06.fasta` — SwissProt + varsplic, **no cRAP** |
| Alkylation | **PRE-SET** by the operator: `CysMod=+57.021464 (carbamidomethylation)` |
| Sample | NIST RM 8461 human liver, 10 mg preparation |
| Paper | Davis et al. 2019, Sci Data 6:324 (Ben is senior author) |
| Raw data | PRIDE **PXD013608** |

The raw file, the mzML, and the FASTA are NOT vendored (1.6 GB / 273 MB /
29 MB). They live in `~/Documents/proteomicsTesting/`. The 361 MB `.mgf` is not
vendored either.

## The numbers, quoted verbatim from `result_summary.html`

```
Cleavage sites (C-side): KR
Missed cleavage: 15.9% (319/2008) of tryptic and semitryptic peptides
Semitryptic peptides (% of tryptic and semitryptic): 8.6% (172/2008) ragged-N,
                                                     1.3% (26/2008) ragged-C
Nontryptic peptides (% of all peptides): 0.1% (2/2009)
```

`result_detail.html` defines missed cleavage as peptides that "contain an
internal K or R not followed by P".

**Denominators, which is why this file matters:**
* missed cleavage, ragged-N, ragged-C → **tryptic + semitryptic peptides**
  (non-tryptic EXCLUDED), distinct peptides, not PSMs
* non-tryptic → **all peptides**

⚠ Preview's own arithmetic is internally off by one: 2008 + 2 != 2009. Observed
quirk. Do not replicate it.

## Three warnings for anyone using this

1. **⚠ USE THIS FILE, NOT DAVIS TABLE 3.** Table 3 reports this material as
   ragged **C**-term 7.4 % / ragged **N**-term 1.1 % — the labels the other way
   round from the tool that produced them, with magnitudes consistent with a
   transposition. Preview itself, recon, and MSFragger all give ragged-N >>
   ragged-C. Table 3 is not the comparison basis.
2. **⚠ The cysteine result is not a detection.** "Cysteine: 100.0 % (226/226)
   C[+57]" confirms what the operator told it (`params.prv`). recon is
   alkylation-agnostic by design, so a difference there is expected.
3. **⚠ Preview reports the same numerator under two different denominators.**
   Oxidised methionine is "19.1 % (509 additional identifications over 2667
   baseline)" in the summary and "69.3 % (509/735) of peptides containing M[+0]
   or M[+16]" in the detail. Same 509 peptides, 50 points apart. Always read the
   fraction, never the percentage alone.

## Protein selection — how Preview got 921 proteins

`result_detail.html` carries the full representative-protein table (rank,
ProtScore, #spectra, #unique peptides). Parsed 2026-08-31:

* **Threshold is `ProtScore >= 20`** — scores descend smoothly and stop at
  **20.04**, with nothing below.
* **No peptide-count rule.** One protein has a single unique peptide; 73 have
  two. Proteins at the cutoff carry 2-5 spectra.
* ProtScore itself is NOT reconstructable from the shipped
  `Spectrum.identifications.csv` — only 57 proteins join between that file and
  the table, all near the top. Do not fit a model to those 57.

## Files

| file | what |
|---|---|
| `result_summary.html` | the report above |
| `result_detail.html` | full protein table + per-assay detail |
| `Spectrum.identifications.csv` | 2476 spectrum-level IDs (the BASELINE search, not the digestion assay — it does not reproduce the 2008-peptide digestion numbers) |
| `run.prv`, `objs/params.prv` | run parameters |
| `objs/FixedMods*.txt`, `objs/VariableMods*.txt` | mod settings |
| `preview-ui.jpg` | the Preview UI as configured for this run |

Compare against it with `testing/scripts/preview_compare.py`.
