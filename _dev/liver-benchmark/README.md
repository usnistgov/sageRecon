# Liver benchmark: five tools on one file

This folder holds the inputs of the liver comparison in the recon technical
note. With it, a reader can rerun the comparison from a fresh clone of this
repository. The folder holds only the files the comparison scripts read, plus
each tool's run settings. It holds no raw data.

## The sample

- NIST RM 8461 human liver, file `10mg_1_A_1`.
- Davis et al. 2019, *Sci Data* 6:324.
- Raw data: PRIDE PXD013608. Instrument: Orbitrap Fusion Lumos.
- Database for every tool: `uniprot_sprot_iso_human-2018_06.fasta`, SwissProt
  plus isoforms. The same file ships as
  `examples/uniprot_sprot_iso_human-2018_06.fasta` (SHA-1
  `5fd174af57ab0368a88dfaff8d9809acdf315acf`). Mascot also searched cRAP
  (`DB=cRAP,sprot_iso_human-2018` in its `.par`).

This is the only file that went through all five tools.

## The tools and their settings

Each setting below was read from the file named in the last column.

| tool | version | key settings | source file |
|---|---|---|---|
| recon | 0.1.1, Sage 0.15.0-beta.2 | Both passes search with no fixed and no variable mods, so alkylation is discovered. Enzyme trypsin. | `_dev/testing/recon-output/full-run/liver.json`, `liver_search/results.json` |
| PTM-Shepherd | FragPipe 23.1, MSFragger 4.4.1, PTM-Shepherd 3.0.2 | Stock FragPipe Open workflow with all fixed and variable mods removed (`add_C_cysteine` is commented out; all 29 `add_*` lines are 0.0; no `variable_mod` line is active). Precursor -150 to +500 Da. Fragment 20 ppm. `calibrate_mass = 2`. `num_enzyme_termini = 2` (fully tryptic). `allowed_missed_cleavage_1 = 2`. `isotope_error = 0`. | `ptm-shepherd/liverShepherd/fragger.params`, `fragpipe.workflow`, log |
| MetaMorpheus | 1.1.7 | Three tasks: Calibrate, G-PTM-D, Search. `ListOfModsFixed` and `ListOfModsVariable` are empty in all three. The G-PTM-D list holds Carbamidomethyl on C (Common Fixed) and Oxidation on M (Common Variable), so +57 is discovered. Calibrate: MS1 10 ppm, MS2 30 ppm. Search: MS1 5 ppm, MS2 20 ppm. `SpecificProtease = "trypsin"`, 2 missed cleavages. | `metamorpheus/liverMetaMorpheus/Task Settings/*.toml`, `allResults.txt` |
| Mascot | 2.6.0 (see note) | Error-tolerant search (`ERRORTOLERANT=1`). `MODS=` and `IT_MODS=` are empty, so its Carbamidomethyl is a discovery. MS1 10 ppm, MS2 20 ppm. Trypsin, `PFA=1`. | `mascot/error-tolerant/Human_ertol-2018.par` |
| Byonic Preview | v3.2.0 | The operator set `CysMod=+57.021464`, so its +57 is an input, not a discovery. `DigestLetters=KR`. No mass tolerance is an input. Run 2019-03-14. | `preview/10mg_1_A_1/objs/params.prv`, `result_summary.html` |

Notes on the table:

- The Mascot version is not in the vendored Mascot files. The operator
  recorded it. `MascotErrorTol-liver.txt` is the modification summary copied
  by hand from the Mascot report.
- Both Preview HTML pages say v3.2.0. The title bar of `preview-ui.jpg` says
  v3.6.0. We report v3.2.0, from the pages that hold the results. The
  screenshot may come from a later install. We did not resolve this.
- Preview's peptide list is not shipped. Its digestion row is quoted from
  `result_summary.html`.

## Which file feeds which result

Scripts are in `_dev/testing/scripts/`. Each one stops with an error if an
input is missing. It does not report a partial comparison.

| script | reads from this folder | also reads | result |
|---|---|---|---|
| `liver_mod_rank_comparison.py` | `ptm-shepherd/liverShepherd/global.profile.tsv`, `metamorpheus/liverMetaMorpheus/Task3-SearchTask/AllPeptides.psmtsv`, `mascot/error-tolerant/MascotErrorTol-liver.txt` | `_dev/testing/recon-output/full-run/liver.json`, `recon-tool/resources/unimod.xml` | Spearman rank agreement of mod counts |
| `liver_four_tool_digestion.py` | `recon/pass2/results.sage.tsv`, `ptm-shepherd/liverShepherd/peptide.tsv`, `metamorpheus/liverMetaMorpheus/Task3-SearchTask/AllPeptides.psmtsv` | `examples/uniprot_sprot_iso_human-2018_06.fasta` | missed cleavage, ragged-N, ragged-C |
| `liver_5way_mods.py` | `ptm-shepherd/liverShepherd/global.modsummary.tsv`, `mascot/error-tolerant/MascotErrorTol-liver.txt`, `metamorpheus/liverMetaMorpheus/Task3-SearchTask/AllPeptides.psmtsv`, `preview/10mg_1_A_1/objs/VariableMods.txt` | `liver.json`, `unimod.xml` | five-tool mod table |
| `liver_5way_report.py` | `preview/10mg_1_A_1/result_summary.html` | the two scripts above, `liver.json` | the composed five-tool report |
| none (read by hand) | `ptm-shepherd/liverShepherd/log_2026-09-01_09-18-03.txt`, `metamorpheus/liverMetaMorpheus/Task1-CalibrateTask/results.txt` | | MS1 calibration medians |

`recon/pass2/results.sage.tsv` is recon's own pass-2 Sage output for this
file. It belongs to the committed `liver.json` run: it holds 15025 target PSMs
and 10774 distinct peptides at `peptide_q < 0.01`, the same two counts that
`liver_pass2.json` records.

The other files are settings and provenance. They are not read by a script.

## Values this folder reproduces

Run on 2026-09-24 from this repository. recon side: Sage v0.15.0-beta.2,
`full-run/liver.json`.

Mod rank agreement, recon against each tool:

| tool | shared masses | Spearman rho |
|---|---|---|
| PTM-Shepherd | 39 | +0.609 |
| MetaMorpheus | 6 | +0.943 |
| Mascot | 28 | +0.472 |

Digestion, distinct peptides, one classifier for every tool:

| tool | missed cleavage | ragged-N | ragged-C |
|---|---|---|---|
| recon pass 2 | 17.53 % | 6.94 % | 3.14 % |
| PTM-Shepherd | 19.64 % | 1.64 % | 0.51 % |
| MetaMorpheus | 17.84 % | 0.54 % | 0.45 % |
| Byonic Preview | 15.90 % | 8.60 % | 1.30 % |

PTM-Shepherd searched fully tryptic (`num_enzyme_termini = 2`). The digestion
script treats the MetaMorpheus trypsin search as fully specific too. Their
ragged rates are a control near zero, not a measurement.

MS1 calibration, read from each tool's own output:

- PTM-Shepherd log, table "MASS CALIBRATION AND PARAMETER OPTIMIZATION",
  row `001`: MS1 (Old) median -1.43 ppm, MAD 0.97.
- MetaMorpheus Task 1 `results.txt`, first `DataPointAcquisitionEngine`
  block: `MS1 ppm error median: -1.57`.

## How to rerun

From the repository root. Python 3 standard library only.

```bash
python3 _dev/testing/scripts/liver_mod_rank_comparison.py
python3 _dev/testing/scripts/liver_four_tool_digestion.py
python3 _dev/testing/scripts/liver_5way_mods.py
grep -A5 "MS1   (Old)" _dev/liver-benchmark/ptm-shepherd/liverShepherd/log_2026-09-01_09-18-03.txt
grep -m1 "MS1 ppm error median" _dev/liver-benchmark/metamorpheus/liverMetaMorpheus/Task1-CalibrateTask/results.txt
```

`liver_5way_report.py` writes by default to
`_dev/testing/recon-output/comparison/LIVER-FIVE-TOOL-2026-09-01.md` and
overwrites the committed copy. Its `-o` path must be inside `_dev/`, or the
script fails after it writes the file. The committed copy is from an earlier
recon run, so its recon numbers differ slightly from a rerun. Its
third-party rows do not change.

## Window-sign and fragment check (`sage-window-check/`)

This check uses stock Sage, not recon. It confirms two facts on the liver
file.

- Sage: v0.15.0-beta.2, rev `df9219951cc9a54cf4cd55d76541af24b687bd3d`,
  built from source. Telemetry off.
- Settings: `precursor_tol.da [-3.5, 1.25]`, `fragment_tol.ppm [-10, 10]`,
  static C +57.0215, decoys generated, `--annotate-matches`. Everything
  else is a Sage default. `sage-window-check/results.json` is Sage's own
  record of the search.
- Script: `window_and_fragments.py`. Output: `sage-window-check/summary.txt`.

Results:

- 44218 PSM rows, all rank 1 (`report_psms` = 1).
- Observed delta, `expmass - calcmass`: -1.2496 to +3.4987 Da. Sage applies
  the tolerance to the observed mass, so `da [-3.5, 1.25]` gives deltas of
  -1.25 to +3.5 Da.
- 18865 rank-1 target PSMs at `spectrum_q <= 0.01`, with 250882 matched
  fragments. Sage's log says 19052 because that count includes the 187 decoys.
- `fragment_mz_experimental`: median 605.3758, intensity-weighted median
  726.4010. 17.27 % of matched fragments are at 400 to 600 m/z.

The two Sage TSVs (21 MB and 16 MB) are not committed. To make them again:

```bash
sage config.json --annotate-matches \
  -f examples/uniprot_sprot_iso_human-2018_06.fasta -o OUT 10mg_1_A_1.mzML.gz
python3 _dev/liver-benchmark/window_and_fragments.py OUT
```

`config.json` holds the settings above. The `database`, `precursor_tol` and
`fragment_tol` blocks of `results.json` show them.

## Redaction

Some files carried the filesystem paths of the machines that ran the tools.
The paths named people. Before publication we replaced the user part of each
path with `<redacted>/` and changed nothing else.

The rule, as a Python bytes regex:

```
(?i)(?:[A-Za-z]\\?:)?(?:\\+|/)Users(?:\\+|/)[^\\/\s"'<>]+(?:\\+|/)
```

It matches an optional drive letter, then `Users`, then one folder name, with
any number of `\` or `/` as separators. So `C:\Users\<name>\Documents` became
`<redacted>/Documents`. The local scratch folder of the window-check run was
replaced in the same way.

Files changed, with the number of replacements:

| file | replacements |
|---|---|
| `ptm-shepherd/liverShepherd/fragger.params` | 1 |
| `ptm-shepherd/liverShepherd/shepherd.config` | 3 |
| `ptm-shepherd/liverShepherd/fragpipe.workflow` | 2 |
| `ptm-shepherd/liverShepherd/log_2026-09-01_09-18-03.txt` | 124 |
| `metamorpheus/liverMetaMorpheus/allResults.txt` | 2 |
| `metamorpheus/liverMetaMorpheus/Task1-CalibrateTask/results.txt` | 1 |
| `preview/10mg_1_A_1/result_summary.html` | 2 |
| `preview/10mg_1_A_1/result_detail.html` | 2 |
| `preview/10mg_1_A_1/run.prv` | 3 |
| `preview/10mg_1_A_1/objs/params.prv` | 3 |
| `sage-window-check/results.json` | 4 |

A check ran on every file. The line count did not change. Each changed line
equals the original with only the matched text replaced.

`preview/10mg_1_A_1/preview-ui.jpg` is a screenshot. A black box covers the
drive, `Users` and the user name in its first file path (pixels x 283 to 348,
y 79 to 94). Saving it again as JPEG changed its bytes. The original md5 was
`087a5f7db4ba4d93b68c369970c1e107`.

Paths that name no person are kept. Examples: `C:\FragPipe\...` and
`C:\ProgramData\Thermo\...`.

## What is not here

- Raw, mzML, MGF and FASTA files. Get the raw data from PRIDE PXD013608. The
  FASTA ships in `examples/`.
- The MSFragger closed semi-tryptic run `liverFragger`. It was an intermediate
  check and is not part of the comparison.
- Files that the Preview HTML pages link to but that were never vendored:
  the Protein Metrics web assets (`objs/pmi_style.css`, `objs/pmi_banner.gif`,
  `objs/spacer.gif`) and four PNG images. The links in the pages are broken
  on purpose.
- Other outputs of each tool, such as full PSM tables. They are not read by
  the comparison.

## Licensing

- recon and the files we wrote (this README, `window_and_fragments.py`,
  `summary.txt`) are under the repository license, `LICENSE.md`.
- The other files are outputs of third-party tools: FragPipe, MSFragger,
  PTM-Shepherd, MetaMorpheus, Mascot, Byonic Preview and Sage. We publish them
  unchanged except for the redaction above, as the record of this benchmark.
  `LICENSE.md` covers NIST-developed software. The tools that made these
  files keep their own licenses.
- `preview-ui.jpg` shows the Protein Metrics name and logo. They are the
  marks of Protein Metrics Inc.
- Unimod, which the scripts read from `recon-tool/resources/unimod.xml`, is
  under the Design Science License. See `THIRD_PARTY_LICENSES.md`.
