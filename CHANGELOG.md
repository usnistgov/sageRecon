# Changelog

All notable changes to `recon` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Releases are tagged `vX.Y.Z` on `usnistgov/sageRecon`. Each release on the
[Releases page](https://github.com/usnistgov/sageRecon/releases) has the full
release notes. The entries below summarize them.

## [Unreleased]

### Added

- **NIST disclaimers in the README.** A generative AI disclaimer, which links to
  `docs/AI_USAGE.md`, and the NIST commercial product disclaimer.
- **This changelog.**
- **Status and support in the README.** `recon` is under active development.
  Support is best effort, through GitHub Issues or email.
- **Memory note in the README requirements**, and a link from the output files
  table to the JSON schema description.

## [0.2.0] - 2026-09-25

A breaking release: both JSON report schemas change. The code is the version
the forthcoming technical note describes.

### Changed

- **Main report schema 4.0.0** (was 3.x). Removed: `alkylation`,
  `mass_accuracy`, the Pass 1 `digestion` block, and `signal_fate`. The all-PSM
  MS2 median moved to `ms1_calibration.ms2_all_psms_median_abs_ppm`. New:
  `discovery_settings.max_peaks`, and the tolerance each screen used.
- **Pass 2 schema 2.0.0** (was 1.x). `composition` is now the only digestion
  measurement. `terminus`, `digestion` and `comparison` are removed.
- **Pass 1 uses 1 missed cleavage and length 8**, the same as Pass 2. On the
  liver file, Pass 1 time fell from about 68 s to 43 s. The recommendations and
  the recommended tolerances did not change.
- **Peak cap raised from 50 to 500.** On liver, 171 peaks pass the prominence
  test. `recon` gains four variable recommendations (Formylation K, Acetylation,
  Kynurenine W, -32 Da on M) and loses none.
- **Peak prominence is topographic**, as in PTM-Shepherd.
- **Polymer and oxonium screens use the run's measured MS1 and MS2 error**, not
  fixed 10 and 20 ppm. The old values remain as fallbacks.
- **The ppm-to-Da conversion for ion-trap and quadrupole MS2 is at m/z 600, not
  500.** The median matched fragment on liver is 605 m/z.

Each behaviour change was measured on the NIST RM 8461 liver file (PXD013608)
before it was adopted.

### Removed

- **Ten hidden development subcommands.** `recon run` is unchanged.

## [0.1.3] - 2026-09-15

### Changed

- **The Quick Start uses real, published sample data.** It named
  `sample.mzML.gz` and `UniProt-Human.fasta`, and neither file existed. It now
  uses the NIST Candidate RM 8461 human liver reference material (PRIDE
  PXD013608). The matching FASTA,
  `examples/uniprot_sprot_iso_human-2018_06.fasta`, and a pre-computed report,
  `examples/liver.*`, ship in the repository.
- **`AGENTS.md` moved to the repository root.** The full development protocol is
  now at `_dev/dev_AGENTS.md`.

### Added

- **Vulnerability scanning.** `build.yml` runs `cargo audit` and a Trivy
  filesystem scan before the build matrix. The same two checks are required
  steps in the manual release checklist.
- **`docs/AI_USAGE.md`**, which states how AI coding agents were used.

### Security

- **`quick-xml` updated to 0.41.** This fixes two findings in `recon`'s own
  direct dependency. Nine more findings are inside Sage's pinned dependency
  tree. They are documented, unfixed exceptions, tied to the next deliberate
  Sage upgrade.

## [0.1.2] - 2026-09-08

The first public release.

### Added

- **`recon run`**: one wide open search and one semi-enzymatic confirmation
  pass, then one report. It says which modifications are present, how
  completely the sample was digested, and what mass tolerances the data
  supports.
- **Sage v0.15.0-beta.2 compiled in**, pinned to commit `df92199`. The Unimod
  database and the 99-entry curated modification list are embedded too, so
  nothing else needs to be installed.
- **Archives for Windows x86-64, macOS Apple Silicon and macOS Intel.** Linux
  builds from source.

[Unreleased]: https://github.com/usnistgov/sageRecon/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/usnistgov/sageRecon/releases/tag/v0.2.0
[0.1.3]: https://github.com/usnistgov/sageRecon/releases/tag/v0.1.3
[0.1.2]: https://github.com/usnistgov/sageRecon/releases/tag/v0.1.2
