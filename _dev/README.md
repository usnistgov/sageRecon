# Development record

This directory is the development history of `recon`. It is not the product, and
none of it is needed to build or run the tool. The product is `recon-tool/`, and
`README.md` at the repository root is the document for users.

We publish this because the reasoning behind a scientific tool is part of the
tool. A number in a report is only as good as the decision that produced it, and
those decisions are written down here rather than lost.

## What is in here

| Path | What it is |
|---|---|
| `NOTES.md` | Settled decisions, discovered truths, dead ends, and entries marked "intentional, not a bug". The main record |
| `JOURNAL.md` | Dated session debriefs. What was least certain, what was assumed, what to do differently |
| `PLAN.md` | The roadmap and its status block |
| `AGENTS.md` | The working protocol, including the rule that every stated fact must come from a file on disk |
| `AUDIT-2026-09-02.md` | A code audit. All 13 findings are closed |
| `reference-notes/` | Distilled external material: search-engine documentation, controlled vocabularies, methodology notes, and vendored papers |
| `testing/` | Test data, search configurations, the validation harness, and committed reference reports |
| `extracted/` | Code moved out of `recon-tool` for a separate tool |
| `temp-flowChart.md` | A working sketch |

## Reading it honestly

These are working documents. They record mistakes as well as results, including
entries that begin "THAT WAS WRONG" and correct an earlier conclusion. That is
deliberate. A record that only holds the conclusions we kept would hide the
evidence for how much the conclusions were tested.

Some entries are marked `(locked)`. Those are settled and should not be
reopened without new evidence.

## Two directories are not published

`testing/reference-data/` and `_archive/` are in the private development
repository but not here. They hold 212.7 MB of output from other tools
(MSFragger, PTM-Shepherd, MetaMorpheus) that we ran to benchmark against, plus
archived intermediate work. They were withheld for two reasons: their size is 85
percent of the repository for material nobody needs to build or run `recon`, and
their data columns record the absolute filesystem paths of the machines that
produced them.

**Documents in here still refer to those paths.** Those references are not
broken links to fix. They point at material that exists in the development
record and is not part of this repository.

The numerical gate does not depend on either directory. `testing/scripts/run_validation.py`
reads only `testing/recon-output/`, `testing/regression-snapshots/`,
`reference-notes/psi-ms-CV/`, and the built binary.

## Running the gate

From the repository root, after a release build:

```bash
python3 _dev/testing/scripts/run_validation.py
```

It reports 17 of 17 gates passed. One gate is reported as NOT CHECKED by design:
liver has no independent step-0 anchor, so checking it would compare a report
against itself.
