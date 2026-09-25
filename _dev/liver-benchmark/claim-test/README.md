# Liver claim test

## Criterion, fixed before any run (2026-09-25)

Written before the first search. Not changed after the results.

- Success: arm 4 (recon's mods and recon's tolerances) identifies more
  peptides at 1 % FDR than arm 1 (vanilla), at an acceptable runtime cost.
  Arms 2 and 3 attribute the gain.
- "Peptides" (the headline): distinct stripped sequences of target rows
  (`label == 1`) with `peptide_q <= 0.01` in `results.sage.tsv`. Distinct
  modified forms are a second column only, because extra mods can split
  one sequence into several forms.
- PSMs: target rows with `spectrum_q <= 0.01`. Protein groups: distinct
  `protein_groups` strings of target rows with `protein_group_q <= 0.01`
  (edited before the first run finished: the column list showed
  `protein_groups`, which is the grouped form). Sage's log
  lines are a cross-check only (they can include decoys).
- "More": the arm 4 gain must be larger than the run-to-run difference of
  two arm 1 runs.
- "Acceptable runtime cost" (my choice, for Ben to review): arm 4 wall time
  at most 3x arm 1, and the run finishes without the watchdog (peak RSS
  under 6.5 GB on this 8 GB machine, so no swap-bound run).
- Amendment (2026-09-25, after arm 1 started, before any identification
  count was seen): the 6.5 GB memory limit is dropped from the gate. Arm 1,
  the vanilla search, needs about 17 GB of footprint by itself, so the
  limit fails every arm and tests nothing. The runs move to a larger
  machine. The gate is now: arm 4 completes, and its wall time is at most
  3x arm 1 on the same machine. Peak memory is reported for each arm, not
  gated. `analyze.py` applies this gate.

## Status (2026-09-25): a run kit. The claim is not tested yet.

The laptop (8 GB) cannot run this test. Arm 1 alone ran swap-bound for
over an hour. Ben's decision: run all four arms on one larger machine,
with `run_claim_test.sh`, then import the outputs and run `analyze.py`.
One laptop arm 1 run is kept in `laptop-arm1/` as a cross-check of the
identifications (not of the runtime). There is no verdict yet.

## What and why

The question: does a Sage search set up exactly as recon recommends
identify more than Ben's "vanilla" search, on the liver file?

- Sample: NIST RM 8461 liver, `10mg_1_A_1.mzML.gz` (SHA-1
  `b50bb42c774e0971c6e093c6ddb339e2863f6b06`), not committed (PRIDE
  PXD013608).
- FASTA: `examples/uniprot_sprot_iso_human-2018_06.fasta` (SHA-1
  `5fd174af57ab0368a88dfaff8d9809acdf315acf`).
- recon's recommendations: `_dev/testing/recon-output/full-run/liver.json`,
  SHA-1 `5428838c90e291bc30aac249dfa0f3efe4551868`, `generated_at`
  2026-09-24T22:30:43Z, recon 0.1.3, `git_commit` `ba4d30e`. The
  configs were built from this file. A later regeneration (recon 0.2.0)
  may change the file; rerun `build_configs.py` and compare.
- Engine: stock Sage v0.15.0-beta.2, rev
  `df9219951cc9a54cf4cd55d76541af24b687bd3d` (the recon pin), built from
  source with `cargo build --release`. Telemetry off.

## Arms

| arm | mods | MS1 / MS2 tolerance |
|---|---|---|
| 1 vanilla | static C +57.021464; variable M +15.994915, `^Q` -17.026549, N/Q +0.984016, `[` +42.010565 | 20 / 20 ppm |
| 2 | as arm 1 | 10 / 10 ppm (recon) |
| 3 | recon's mods (table below) | 20 / 20 ppm |
| 4 | recon's mods | 10 / 10 ppm (recon) |

Shared by every arm: trypsin (`KR`, not before `P`), fully enzymatic,
`missed_cleavages` 2, `min_len` 7, `max_len` 50, precursor charge 2 to 4,
`max_variable_mods` 2, decoys generated, all other settings Sage defaults.
These shared settings are the claim test's, not recon's. recon's own
passes now use 1 missed cleavage and a minimum length of 8.

recon's tolerance: `liver.html` prints "Recommended MS1 / MS2 10 / 10 ppm",
and `liver.json` has `ms1_calibration.user_recommendation_tolerance_ppm` =
10.0. The MS2 half is the same ladder rung (`report.rs`,
`ms2_user_recommendation`). The fields `ms2_tolerance_low_ppm` /
`ms2_tolerance_high_ppm` (+/-1.14 ppm) are recon's internal pass-2 window,
not the recommendation. They are not used.

## recon's mods in Sage syntax

`configs/mod_mapping.tsv` is written by `build_configs.py`. Each search
mass is the Unimod monoisotopic mass, read from
`recon-tool/resources/unimod.xml` by title, not recon's measured delta.
The script stops if a measured delta differs from Unimod by more than
0.01 Da (largest difference: Acetylation, -0.0027 Da), or if the report's
sites or position do not match the mapping.

| recon label | Sage key | mass |
|---|---|---|
| Carbamidomethyl on C (fixed) | static `C` | +57.021464 |
| Oxidation on M | `M` | +15.994915 |
| Deamidation | `N`, `Q` | +0.984016 |
| Gln->pyro-Glu | `^Q` | -17.026549 |
| Fe[III] | `D`, `E` | +52.911464 |
| Trioxidation | `C` | +47.984744 |
| Met-loss+Acetylation | `[M` (workaround, below) | -89.029920 |
| Dehydroalanine | `C` | -33.987721 |
| Formylation | `K` | +27.994915 |
| Water Loss (Glu->pyro-Glu) | `^E` | -18.010565 |
| Acetylation | `[` | +42.010565 |
| Oxidation to Kynurenine | `W` | +3.994915 |
| Oxidation and then loss of oxidized M side chain | `M` | -32.008456 |

### Checked against Sage's source at the pinned rev

Source: the Cargo checkout of `df92199`, `crates/sage/src/peptide.rs` and
`crates/sage/src/enzyme.rs`.

1. **Variable mods on C replace the static C mod. They do not stack.**
   `Peptide::apply` applies the variable mods first, then the static mods.
   `static_mods` for a residue writes the mass only where
   `self.modifications[idx] == 0.0`. So a C that carries +47.984744 or
   -33.987721 does not also get +57.021464. recon measured both deltas
   against unmodified C (both of its passes search with no mods), so the
   configs give the absolute deltas. No change was needed.
2. **Sage cannot express Met-loss+Acetylation exactly.** At this rev,
   `Enzyme::cleavage_sites` makes sites only at enzyme matches, and
   `digest` labels a peptide `Position::Nterm` only when it starts at
   residue 0. Sage never removes the initiator Met, so the Met-clipped
   peptide is never generated as a protein N-terminal peptide. The
   workaround is `[M: -89.029920`: `ProteinN(Some('M'))` puts the mass on
   residue 0 of a protein-N-terminal peptide that starts with M. That M
   then weighs 131.040485 - 89.029920 = 42.010565, the mass of an
   N-terminal acetyl. So the precursor mass and every b and y ion equal
   those of the clipped, acetylated peptide. Side effects:
   - The reported sequence keeps the M, and `min_len` / `max_len` count
     it. A clipped peptide of 6 residues is not searched.
   - b-ion indices shift by one, so `min_ion_index` (2) drops one more
     small b ion from the preliminary search.
   - `[` (+42.010565) is `Site::Nterm` and `[M` is `Site::Sequence(0)`.
     They are different sites, so with `max_variable_mods` 2 Sage also
     makes a form with both (net -47.019355). That form is chemically
     meaningless. It adds candidates but does not double-count a PSM.
     Sage config cannot forbid the pair.
   - `M` oxidation and `[M` share site 0, so Sage never combines them
     (`apply` skips a combination with a repeated site).

## Memory and runtime on this laptop (8 GB)

The first plan put a 6.5 GB kill limit on each search. Arm 1 alone passed
it (9.4 GB footprint after 13 s), so the limit was raised to 30 GB of
footprint and a 4 h wall limit, and each run is recorded as is. `top` MEM
(footprint) counts compressed memory. RSS does not: the first arm 4 probe
showed 0.5 GB RSS and a 41 GB footprint (40 GB compressed) before it was
stopped by hand. Arm 1 ran swap-bound: Sage built 1,322,386,968 fragments
and 29,381,957 peptides (targets and decoys) in 731 s.

Estimate for the recon-mod arms (`estimate_forms.py`): a
Python count of target peptide forms under Sage's combination rules
(fully tryptic, 2 missed cleavages, length 7 to 50, at most 2 variable
mods, one mod per site):

| config | target peptide forms | x arm 1 |
|---|---|---|
| arm 1 / 2 (vanilla mods) | 16,373,393 | 1.0 |
| arm 4 stage 1 (+ rare-site mods) | 38,549,333 | 2.4 |
| arm 4 stage 2 (+ Cys mods) | 55,099,405 | 3.4 |
| arm 3 / 4 (all recon mods) | 110,683,514 | 6.8 |

The count ignores the 500 to 5000 Da mass filter and decoys, so treat it
as a ratio. If memory scales with it, arms 3 and 4 need about 7 times arm
1's footprint.

## How to run it (the run kit)

Use one machine for every arm, so the runtimes compare. Suggested: 64 GB
of RAM or more (arm 1 needs about 17 GB; arms 3 and 4 hold about 7 times
as many peptide forms, see above). Close other heavy work while it runs;
the script records the load average before each arm.

### 1. Get Sage at the pinned rev

Build it from source (Rust from https://rustup.rs):

```bash
git clone https://github.com/lazear/sage.git sage-df92199
cd sage-df92199
git checkout df9219951cc9a54cf4cd55d76541af24b687bd3d
cargo build --release
./target/release/sage --version     # must print: sage 0.15.0-beta.2
```

The binary is `sage-df92199/target/release/sage`. `_dev/dev_AGENTS.md` records
this rev as tag `v0.15.0-beta.2`, so a release download of that tag may
also do, but only a source build lets the script check the rev
(`--sage-src`).

### 2. Get the inputs

- `10mg_1_A_1.mzML.gz`, sha256
  `460cd316cb95f0db468dfdbcdaeabcb195ba51eff585a1af2ae861f37575512e`
  (the copy in Ben's `sageRecon/_dev/testing/inputs/`; raw data PRIDE
  PXD013608).
- `examples/uniprot_sprot_iso_human-2018_06.fasta` from this repository,
  sha256 `75cc5a96a489a04b385e07a3d4caa223cf361a3727b267c8961b86211c14c922`.

### 3. Run

From the repository root:

```bash
bash _dev/liver-benchmark/claim-test/run_claim_test.sh \
  --sage /path/to/sage-df92199/target/release/sage \
  --sage-src /path/to/sage-df92199 \
  --mzml /path/to/10mg_1_A_1.mzML.gz \
  --fasta examples/uniprot_sprot_iso_human-2018_06.fasta \
  --work /path/to/claim-work
```

- It checks the Sage version, the source rev, and both sha256 sums, and
  stops on any mismatch. `--check-only` does the checks and stops.
- It runs, one at a time: arm 1, arm 2, arm 3, arm 4, then arm 1 again
  (the noise band). `--stages` adds the two arm 4 stages. `--no-repeat`
  drops the repeat.
- It stops before an arm if another Sage search is running.
- A failed arm (for example out of memory) is recorded and the next arm
  runs. Rerun the same command to retry: finished arms are skipped.
- macOS and Linux (`/usr/bin/time -l` or `-v`; on Linux install the `time`
  package). On Windows, use WSL with a Linux build of Sage.
- Each arm writes `claim-work/out_<config>/`: `results.sage.tsv`,
  `results.json`, `sage.log` (Sage's log and the time report),
  `run_meta.json` (wall time, peak memory, load). The folder also gets
  `machine.tsv` and `runs.tsv`.

### 4. Import and analyze

If the runs were on another machine, copy the whole work folder back (or
just every `out_*/` folder plus `machine.tsv`). Then, from the repository
root:

```bash
python3 _dev/liver-benchmark/claim-test/analyze.py /path/to/claim-work
```

It writes `claim-test/results.md`: the per-arm table (PSMs, peptides,
modified forms, protein groups, gained and lost vs arm 1, wall time, peak
memory), PSMs per mass shift, the PTM-Shepherd / MetaMorpheus / Mascot
counts for each recon-only mod, arm 1 against `laptop-arm1/`, and the
verdict against the criterion above.

To commit the run: copy `results.md`, `machine.tsv`, `runs.tsv`, and each
arm's `results.json`, `run_meta.json` and `sage.log` into
`claim-test/results/<arm>/`. Redact personal paths as
`_dev/liver-benchmark/README.md` does. Do not commit the TSVs (each is
tens of MB).

## Files

- `build_configs.py`, `configs/`: the Sage configs and the mod mapping.
- `run_claim_test.sh`: the run kit (all arms, checks, timing).
- `run_arm.py`: one search with a memory watchdog (macOS). Used for the
  laptop run only.
- `analyze.py`: counts, overlap with arm 1, PSMs per mod, and the
  PTM-Shepherd / MetaMorpheus / Mascot liver counts for each recon-only
  mod (parsers imported from `_dev/testing/scripts/compare_4way.py`).
- `estimate_forms.py`: the peptide-form count behind the memory estimate.
- `laptop-arm1/`: the laptop arm 1 run: `counts.json`, the sorted
  peptide list, Sage's `results.json`, `sage.log` and `run_meta.json`.
