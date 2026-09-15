# Agent Protocol

`recon` is a Sage-based proteomics reconnaissance tool. See
[README.md](README.md) for what it does and how to run it.

## Repository layout
- `recon-tool/` — the product. Rust source, Cargo manifest, compiled
  resources.
- `docs/` — generated and curated public documentation (e.g.
  `curated-modifications.md`, `AI_USAGE.md`).
- `examples/` — pre-computed example reports.
- `_dev/` — the development record: design decisions, test data, and the
  full internal operating protocol. Not needed to build or run the tool.

## Settled decisions (do not re-derive)
These are locked. Re-opening them wastes a session; see
[`_dev/dev_AGENTS.md`](_dev/dev_AGENTS.md) for the full reasoning and
evidence behind each one.
- **The CLI surface is frozen:**
  `recon run <MZML> <FASTA> --enzyme <ENZYME> [--output NAME]`. `mzml` and
  `fasta` are positional; `unimod.xml` is compiled in.
- **`--enzyme` is a required parameter, no default.** Fourteen presets,
  sourced from Mascot, vendored at `_dev/reference-notes/mascot-enzymes.md`.
- **Sage is a pinned library dependency, not a vendored binary.**
  `sage-core`, `sage-cli`, and `sage-cloudpath` are Cargo git dependencies
  pinned to a specific commit in `recon-tool/Cargo.toml`. Do not reintroduce
  a vendored Sage binary, `SAGE_PATH`, or a runtime version handshake.
- **`recon-tool/Cargo.lock` is committed** (415 packages) and must not be
  re-ignored.
- **There is deliberately no clippy gate.** CI gates on `cargo fmt --check`
  only; clippy findings need case-by-case human judgment, not a bulk-applied
  fix.

## Working here
- Commit to `main`, plainly. No branches, no squashing, no commit-message
  prefixes.
- Targeted edits only — change the precise lines, not the whole file.
- Keep `README.md` in sync with anything it describes.
- For substantial or ongoing development work in this repo, read
  [`_dev/dev_AGENTS.md`](_dev/dev_AGENTS.md) first. It holds the full
  operating protocol: the prime directive ("never assume, always check"),
  reproducibility rules, numerical tripwires, the shutdown routine, and every
  locked decision in detail.

## AI use
This project uses AI coding agents under human review. See
[`docs/AI_USAGE.md`](docs/AI_USAGE.md).
