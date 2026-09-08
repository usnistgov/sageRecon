# What the gates consume, and why

Written 2026-09-01, when "do we still need all these search results in
`testing/`?" was asked and could not be answered without reading the code.
This is the answer, per gate, with the input it reads and the reason that input
and not another one.

**The distinction that governs everything here:**

| kind | keep? | why |
|---|---|---|
| **Gate** — guards SHIPPED behaviour, fails when the code regresses | **yes** | it is a release check, not scaffolding |
| **Decision record** — was run once to settle a question now locked | no | the decision is in NOTES; the script is history |
| **Dev test** — checks something recon does not ship | no | delete it and the input it drags along |

`ms1_calibration_integration.rs`'s Pass-2 coverage test was a **dev test** by that
rule and was deleted 2026-09-01. It read a CLOSED search as an external
reference. Recon does not produce, consume, or need a closed search — the
`--closed-tsv` path was retired the same day — so the test held the
`step1-closed-*` directories alive for a question already settled.

---

## The files, and what each is FOR

**liver is the ACTIVE file.** It is the only one with a Byonic Preview ground
truth, and the write-up is built on it.

**serum, bcell and b1906 are BEHAVIOUR CHECKS**, not comparison targets. They
exist to prove the CODE does the right thing on data where that behaviour is
visible. Nothing is reported across them. A gate on one of them is kept when
that file is the only place the behaviour shows:

* `ms1_bias_is_negative_on_bcell` — bcell's MS1 bias **flips sign**, +0.70 ->
  -0.24 ppm, which is the proof the `|error|` bug is fixed. Liver reads
  **-1.4193 ppm**, already strongly negative, so liver passes that assertion even
  if the correction were only half applied. Moving it to liver deletes the
  evidence and keeps the green tick.
* `run_validation` Gate 3 is two-sided, fixed-C vs agnostic. Extending it to
  liver would need a deliberate fixed-C liver search, which the no-mods rule
  forbids. Liver can take Tier 3 snapshot and unmodified-% coverage only.

---

## `testing/scripts/run_validation.py` — the standing tripwire

AGENTS names this as the project's hard-stop. **15/15.**

| tier | consumes | why that input |
|---|---|---|
| 1 (per-file gates) | `testing/search-output/step1-open-{b1906,bcell,serum}/results.sage.tsv` | the ALKYLATION-AGNOSTIC open searches — the only three with `static_mods {}`. Gate 3 is two-sided and needs a fixed-C counterpart to compare against |
| 2 (polymer) | `testing/recon-output/tier2_peg_recon.json`, `tier2_peg_mzsniffer.json` | pinned PEG-spike fixtures |
| 3 (snapshot) | `testing/recon-output/full-run/*.json` + `testing/regression-snapshots/` | byte-equality against committed output |
| 4 (analyzer CV) | pinned `psi-ms.obo` snapshot | the 12 mass-analyzer terms are DERIVED, not hand-listed |

⚠ **`FILES = ["b1906", "bcell", "serum"]` — liver is NOT covered.** The standing
tripwire has never run on the file the write-up rests on. Recorded 2026-09-01,
not yet fixed. The fix is to ADD liver, not to remove the others.

---

## Rust integration tests

| test | consumes | why |
|---|---|---|
| `no_mods_guard` | `testing/configs/{open-search-b1906,closed-search-reference}.json`, `tests/fixtures/dirty-mods-open-template.json` | REAL committed dirty templates, so the guard is proven against files that actually exist. The two configs are deliberately dirty and say so in a `_recon_note` |
| `ms1_calibration_integration` | `step1-open-{serum,bcell,b1906}` | the sign-flip regression case. bcell only |
| `tier_assignment_integration` | `step1-open-*`, `full-run/*.json`, unimod, UP FASTA | tier routing on real committed reports |
| `analyzer_detection_integration` | the four mzML files, `testing/configs/open-search-*.json` | detector class read from real headers; MSFragger's log pins `MS2 FTMS = true` |
| `pass2_wiring_integration` | `step1-open-*` | Pass-2 plan from a real pass-1 measurement |
| `digestion_composition_integration` | `full-run/liver_search/pass2`, vendored Preview | REPOINTED 2026-09-01 off the fixed-C search onto the shipped agnostic one |
| `curated_mods_integration`, `peak_assignment_test`, `determinism_test` | unimod, committed fixtures | self-contained |

### ✅ RESOLVED 2026-09-01 — nothing reads a fixed-C or closed search any more

Three call-sites read searches carrying `static_mods {C: 57.0215}`, read from
each run's own Sage `results.json` rather than assumed. All three are gone:

* `digestion_port_integration` — **DELETED.** It re-verified the Rust
  `digestion_efficiency` port against its Python prototype. That port is a
  LOCKED decision, so the test is a decision record, not a gate.
* `parsimony_impact` — **DELETED.** Same reason: parsimony is decided.
* `digestion_composition_integration` — **REPOINTED** onto
  `full-run/liver_search/pass2`, the shipped agnostic artifact. It is a real gate
  (the classifier against the Preview anchor), so it stays. Pinned numbers moved
  with the search, `10589/1816/738/321` -> `10621/1818/717/320`, plus the
  decoy-corrected set `76/31` -> `81/26` and semi-tryptic class FDR
  10.10 -> 10.32 %. The independent-reproduction property AGENTS asks for is kept
  by `testing/scripts/liver_four_tool_digestion.py`, which mirrors the same rule
  in Python for the four-tool comparison.

**No new search was needed for any of it.**

---

## The regeneration gate: `assert_regeneration_invariants.py`

Run BEFORE copying a regenerated report over a committed one. It hard-stops on
any change outside a pre-committed allow-list, and prints every number it allows.

**It CHECKSUMS the source TSV rather than inferring drift from counts.** That is
not a convenience — serum proved the inference wrong. Its `total_psms` and all 48
peak counts were IDENTICAL across two runs while seven odds ratios moved, because
Sage returned a different SET of PSMs with the same per-peak counts. Different
peptide identities, different residue tallies in the Fisher 2x2, same counts.

* TSV identical -> **nothing may move.**
* TSV differs -> derived statistics may move within **1 % relative**, each one
  printed with its magnitude; counts within 2 PSMs or 0.1 % of total.
* TSVs absent (fresh clone — they are gitignored) -> falls back to the count
  heuristic and SAYS SO.

Grounded in a controlled experiment, not in assumption: recon run twice on one
fixed TSV differs only in `generated_at` and two documented ULP fields, while
Sage run twice on byte-identical input emits different TSVs. See NOTES
"SAGE IS NON-DETERMINISTIC, RECON IS NOT".

## What is safe to delete, and what is not

**DONE 2026-09-01: 837 MB -> 166 MB.** 21 directories deleted. Nothing was lost
from the repo — `testing/search-output/` is gitignored and **0 of its files were
ever in git**, so this was local disk only.

What remains, and why:

* `step1-open-{serum,bcell,b1906}` — the three ALKYLATION-AGNOSTIC open searches
  (`static_mods {}`). `run_validation` Tiers 1 and 3 and two integration tests
  read them. These are the behaviour-check inputs.
* `ptmRecovery` — read by `compare_predicted_vs_verified_cam57.py`, which backs
  the +57 predicted-vs-verified reconciliation recorded in NOTES.

Deleted: every fixed-C open search (`open-*-full`, `open-*-calibrated`,
`open-search`, `liver-10mg_1_A_1`), every closed search (`step1-closed-*`,
`closed-ref-*`, `narrow-search`), and the old digestion probes
(`digestion-pass1/2`, `serum-digestion-pass1/2`, `semi-enzymatic`,
`probe-serum-annotate-matches`, `annotated_termini.tsv`,
`digestion_efficiency_result.json`).

⚠ **Deleting a gate's input does not fail the gate — it SKIPS it.** Several tests
do `if !path.exists() { eprintln!("skipping"); return; }` and still report `ok`.
`testing/search-output/` is gitignored, so on a fresh clone those tests pass
without testing anything. That is a known weakness, recorded in NOTES
2026-09-01, and it is the reason this document exists: the inputs are invisible
to git, so the only record of what depends on them is this file.
