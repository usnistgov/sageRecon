<!-- agent-context-kit v4 - adapted 2026-08-24 - github.com/neely/agent-context-project-template -->

# Agent Protocol

## Authority (read if present)
- If a field-level grounding spec applies to this project (GROUNDING.md), it outranks this file.
  Defer to it and cite the relevant constraint when a conflict arises — field
  validity beats project preference. **No GROUNDING.md exists in this repo as of
  2026-08-24, so this section is currently a no-op.** (Example:
  github.com/OmicsGrounding/proteomics-grounding)

## The prime directive: never assume, always check
This project's entire credibility rests on this. It is not a style preference.
- **Every fact you state — about code, config, state, or a measurement,
  identifier, count, set membership, or derived value — comes from a FILE on
  disk or a COMMAND/SCRIPT that reads one. Never from memory or recollection.**
  "I think this function does X" or "I think that number was N" is forbidden;
  read the file, run the check, and emit what you actually saw.
- If a fact isn't in something you can read, either REQUEST it or WRITE a
  script/query/command to produce it. Do not fill the gap from training
  knowledge.
- **A summary is not a source.** A table in JOURNAL, a number in a debrief, or a
  count in a previous NOTES entry is a *record of* a measurement, not the
  measurement. When a value matters, open the output file that produced it.
  Copying a stale summary forward is the same failure as inventing a number.
  (2026-08-24: a "missing" b1906 value was escalated to "needs a tool re-run"
  three times; both sources were committed in the repo the whole time.)
- **Check the convention, not just the column.** A field's name does not tell you
  its units, sign, or semantics. Verify against the producing tool's own
  documentation and against the data itself. (2026-08-24: `precursor_ppm` is
  `|error|` in the pinned Sage, and was read for months as a signed bias.)

- **For any EXTERNAL or citation claim** (a paper's figures, a DOI, a current
  name in a controlled vocabulary, an API's behavior, a library version, a
  tool's limit): surface it for verification — quote exactly what the source
  currently asserts and flag it for check. Do not silently trust or silently
  "correct" an external claim from memory.
- **A contradicting result outranks your hypothesis.** If a test, run, or
  measurement disagrees with what you expected, report the disagreement and
  stop. Do not re-interpret it, adjust assumptions, or argue until it agrees.
  "Expected behavior" is not a valid conclusion when the result contradicts the
  goal. (This is the exact failure mode behind the satellite-folding loop — see
  NOTES dead-ends.)
- **One agreeing case is not validation.** If a method matches a reference on one
  file and disagrees on others, the disagreements are the signal. Do not treat
  the agreement as proof the method works. (2026-08-24: serum agreed to two
  decimal places while a units bug made two other files wrong.)

### The two conventions that keep getting re-litigated (memorize these)

Both have been settled, with the tool author or the raw data as the source. If you
find yourself reasoning about either one, STOP and re-read this block instead.

**1. `precursor_tol.da` signs are INVERTED relative to the delta mass they produce.**

```
"precursor_tol": { "da": [-500, 100] }   ⇒   delta mass −100 to +500 Da
```

Sage applies the tolerance to the **experimentally observed** mass, so `-500`
means "look for theoretical peptides 500 Da LIGHTER than observed", which shows up
as a **+500** delta. Delta here is `expmass − calcmass`.
- The units are **DALTONS, not ppm.** This is the open-search modification window
  — how far a mod may shift the precursor. It is NOT a mass-accuracy number and
  nothing measures it.
- So the search covers losses to 100 Da and additions to 500 Da.
- Confirmed by Michael Lazear (Sage author) 2026-08-17, by every open-search TSV
  (`min delta = -100.00, max delta = 500.00` on serum), and locked in
  `reference-notes/sage-config-and-gotchas.md`. **Do not "fix" the sign.**

**2. Four different tolerance numbers exist. Do not merge them.**

| number | where | what it is |
|---|---|---|
| `precursor_tol.da [-500,100]` | pass-1 template | open MOD window, Da (above) |
| `fragment_tol.ppm [-20,20]` | pass-1 template | pass-1 MS2 setting; hardcoded, NOT detector-aware yet |
| `PASS2_HALF_WIDTH_CAP_PPM = 100` | `calibration.rs` | RETIRED: `#[deprecated]`, zero code references. Kept here as history only. |
| `{10,20,50,100}` ppm ladder | `calibration.rs` | the user-facing MS1 tolerance RECOMMENDATION (an output) |

**REVISED by Ben 2026-09-03. The ladder now serves MS1 AND a ppm MS2.** It used
to read "MS1 only". The report gives one combined recommendation, `Recommended
MS1 / MS2`, and the MS2 half is the measured fragment spread quantised to the
SAME rung.

⚠ **The reason behind the old rule still holds, and is the constraint:** a ppm
ladder is meaningless for an ion trap or quadrupole, which needs ~0.5-1.0 Da.
So the ladder applies ONLY when the MS2 analyzer is ppm-based. For a Da class
the recommendation is converted at m/z 500, doubled, **and then rounded UP to
the nearest 0.1 Da**, using `PASS2_MS2_REPRESENTATIVE_MZ`,
`PASS2_MS2_DA_MULTIPLIER` and `round_up_to_tenth_da`. Any Da analyzer in
a mixed MS2 census wins, so a trap can never receive a rung.

**The tenth-Da step is the Da regime's LADDER** (Ben, 2026-09-03), and it exists
for the ppm ladder's own reason: a user picks a search setting from an
effectively discrete set. It is applied to the RECOMMENDATION only, never to
`ms2_pass2_tolerance`, which sizes recon's internal window — exactly as the ppm
branch quantizes the recommendation and not the pass-2 window.

⚠ **`ms1_user_recommendation` IS NOT ANALYZER-AWARE, and that is a known
limitation, not an oversight.** It always uses the ppm ladder, whatever
`ms1_analyzers` says, so a trap or quadrupole MS1 would receive a ppm number
that is meaningless for it. Making it Da-aware is a SCHEMA change: every field
of `Ms1UserRecommendation` is ppm-named and serialised, and the function takes
no analyzer argument. Ben's call 2026-09-03: not worth the blast radius for an
instrument nobody uses for MS1 survey scans. See NOTES.

**Do not "restore" MS1-only.** The revision is deliberate; the trap protection
is the part that must not be lost. All four committed files are Orbitrap on BOTH
levels, so the Da branch is covered by unit tests and by no real data. See NOTES
"The three tolerance regimes".

## Reproducibility is locked
- **External data dependencies are pinned to a specific snapshot or version.**
  ⚠ **Sage's authoritative pin is the git `rev` in `recon-tool/Cargo.toml`, plus
  `recon-tool/Cargo.lock`** (committed 2026-09-02, 415 packages). `SAGE_VERSION`
  and `SAGE_COMMIT` in `sage_runner.rs` still exist and still matter, but they
  MIRROR that pin for reporting — a test asserts `Cargo.lock` names the same rev
  as `SAGE_COMMIT`. Change the manifest and the constants together, or that test
  fails. Corrected 2026-09-02: this bullet used to name only the constants, which
  contradicted the "Sage IS A LIBRARY" lock below.
  The reference-tool outputs under `testing/reference-data/` are dated snapshots.
  State this in any Methods text. A reader re-running against current upstream
  gets drift — that's expected.
- **Verify-regenerate is free and encouraged:** re-run scripts to confirm they
  reproduce the pinned outputs. Do this after any migration or edit that could
  touch a derived number.
- **Change-regenerate (re-running against CURRENT upstream and adopting the new
  data) happens ONLY on explicit user request** — and MUST be preceded by an
  enumerated downstream-impact trace: which caches, counts, figures, and docs
  would change. Never change-regenerate incidentally while doing other work.
  Upgrading the pinned Sage DEPENDENCY is a change-regenerate; NOTES holds a
  step-by-step upgrade checklist for it.
- **Manual curation and pinned values change ONLY by deliberate, recorded
  edits.** "Re-derive from raw" faithfully reproduces AUTOMATED steps only;
  anything manual lives in data+code or it silently reverts.

## Tripwire every derived set
- Recompute against a known count before trusting or interpreting any derived
  set. **Hard-stop on mismatch.** Numbers are certified against the pipeline's
  own scripts, not eyeballed. `testing/scripts/run_validation.py` is this
  project's standing tripwire harness.
- **Lead with the invariant.** Before implementing anything that moves or
  transforms data (counts, intensities, PSMs, masses), state the hard invariant
  it must satisfy — conservation, a physical constraint, a sum that must balance
  — and **assert it in code**, not just check it in prose. A fix backed by a
  checkable invariant closes in one pass; a fix verified only by narrating the
  output can report any number and still be wrong.
- **Acceptance is numeric, not asserted.** When a checkpoint has a numeric gate,
  print the actual values and let them be read — do not substitute "verified ✓"
  or a "✅ PASS" with no numbers. If the numbers in your own report don't
  reconcile with each other, that is a failure to resolve before proceeding, not
  a rounding note to wave past.
- When an aggregation looks surprising, INSPECT THE RAW PRE-AGGREGATION
  DISTRIBUTION before trusting it.
- **Unit tests built on synthetic fixtures inherit the assumptions of the code
  they test.** They cannot catch a wrong convention. A regression case needs a
  real input whose correct answer is known independently.
- Project-specific tripwires (expected counts, known splits, sanity bounds) live
  in NOTES.md.

## Files (read in this order on a cold start)
1. This file — how to behave.
2. `PLAN.md` — status block (top) + active phase.
3. `NOTES.md` — what this project has concluded: locked decisions, discovered
   truths, limitations, dead-ends, and "intentional, not a bug" entries. Skim
   for relevance; it is long. (This file plays the role the kit calls
   FINDINGS.md.)
4. `JOURNAL.md` — recent debriefs, only if you need the backstory. Do not read
   it wholesale.
5. `reference-notes/` — distilled external material (Sage docs, Unimod, oxonium
   lists, polymer series, methodology, domain primer, result schema). Consult
   targeted, only when the task needs it.

`README.md` is for humans arriving cold — not part of your read path, but keep
it in sync (see below).

**`reference-notes/` is the only reference folder.** Note the name is inverted
from the kit's default naming — do not "fix" it:
- `reference-notes/` — committed, **distilled** docs you author/curate. This is
  the "reference" the kit means. (locked — see NOTES.)

A second folder, `reference/`, once held gitignored raw clones of upstream repos
(Sage, mzSniffer, PTM-Shepherd, Crystal-C, intensityWeighting, and others). It
was local scratch for porting source, never a read path. It is gone. Do not
recreate it.

## Remote
**`github.com/usnistgov/sageRecon` is the published repository. It is PUBLIC.**
Anything committed there is public immediately and permanently.

`github.com/neely/sageRecon` is a PRIVATE ARCHIVE. It holds the full
pre-publication history, 329 commits. Keep it. Do not delete it, and do not
publish from it.

⚠ **The two histories are not the same.** The public repository starts from a
squashed commit, so its history is short by design. A plain `git push` between
them will not fast-forward. Check which remote you are on before pushing.

⚠ **Two directories are withheld from the public repository:**
`_dev/testing/reference-data/` and `_dev/_archive/`. They are 212.7 MB of
vendored third-party output, and they carry personal filesystem paths in their
data columns. See `_dev/README.md`.

**Releases come from GitHub Actions.** `.github/workflows/build.yml` builds four
targets and attaches the archives to a Release when a `v*` tag is pushed. A
public repository gets free runners, which is what unblocked this.

⚠ **A second, internal pipeline used to exist and is GONE (2026-09-08).** It ran
a Windows build for a NIST approval route that turned out not to apply.
`.gitlab-ci.yml` is deleted. Its runner lessons are kept in NOTES because they
are real engineering history, marked superseded. Do not re-add the pipeline.

⚠ **"Committed" is not "pushed."** They are two operations. Report the commit
hash AND confirm the remote accepted the push.

## How to work
- **Targeted edits only.** Never rewrite a whole file to change a few lines.
  Edit the precise lines.
- **Commit to main, plainly.** Standard commit messages, straight to main.
  No branches, no squashing, no commit-message prefixes.
- **Batched atomic commits.** Group logically related file changes into one
  commit. A code change and its doc update belong together. One commit reads
  as one coherent decision. Hold related edits together before committing
  rather than committing each file as it is finished.
- **Write in ASD-STE100.** Simplified technical English. Short sentences, one
  idea each. Applies to commit messages and everything written in PLAN, NOTES,
  and JOURNAL.
- **Respect the markers.** Do not reopen anything tagged `(locked)` or
  "don't relitigate" unless explicitly told to. Do not "fix" anything tagged
  "intentional, not a bug." Do not re-explore anything recorded as a dead-end.
  Do not re-enable anything under "disabled-by-design" in NOTES.
- **Disabled-by-design is not half-finished.** If you find a feature switched
  off with its implementation retained (e.g. a `false` config flag next to a
  full function), do NOT "wire it up" as a free improvement. It may be off on
  purpose because it lacks a verification guard. Check NOTES for a
  "disabled-by-design" entry and respect the reasoning there before touching it.
- **Sage IS A LIBRARY. This is DONE — do not re-plan it (landed 2026-09-01,
  commit `a4f09b9`).** `sage-core`, `sage-cli` and `sage-cloudpath` are Cargo git
  dependencies pinned to `df9219951cc9a54cf4cd55d76541af24b687bd3d`
  (tag `v0.15.0-beta.2`, UPSTREAM `lazear/sage`, no fork). `run_sage` calls
  `sage_cli::runner::Runner` in process.
  **GONE, and must not be reintroduced:** the vendored binary, `SAGE_PATH`,
  `--sage-binary`, `locate_sage_binary`, `DEFAULT_SAGE_PATHS`, the runtime
  version handshake, and the telemetry opt-out flag (linking the library never
  sends telemetry — `Telemetry::send` is called only from Sage's own `main.rs`).
  ⚠ **Pinning is not tracking.** A Cargo git rev is a pin exactly as
  `SAGE_COMMIT` was. Moving to a later Sage stays a deliberate, tested, recorded
  upgrade.
  ⚠ **AND THE PIN IS WEAKER ON ONE AXIS THAN IT LOOKS.** A git rev pins Sage's
  SOURCE, not its dependency graph: 155 of 335 shared packages resolve
  differently from Sage's own `Cargo.lock`. Say both halves in any Methods text.
  See NOTES "q-DERIVED COUNTS JITTER".
- **Do not re-derive these; they are settled and recorded (all 2026-09-01).**
  A cold start that re-opens any of them is going backwards:
  * **The enzyme is a PARAMETER**, `--enzyme`, required, no default. Fourteen
    presets sourced from Mascot and vendored at
    `reference-notes/mascot-enzymes.md`. Identity only — never the tuning fields.
  * **The argument surface is FROZEN:**
    `recon run <MZML> <FASTA> --enzyme <ENZYME> [--output NAME]`. mzml and fasta
    positional; `unimod.xml` compiled in; 11 development subcommands hidden.
  * **`three_layer_ms1` was REMOVED** from the report and survived only as
    `signal-fate --three-layer`.
    ⚠ **Superseded 2026-09-02: Ben decided it does not belong in `recon` at
    all.** It is being MOVED out of `recon-tool/src` into
    `extracted/three-layer-ms1/`, so the release binary stops carrying it. The
    source stays in the repo, for a separate tool. This was in progress,
    uncommitted, as this entry was last edited — check PLAN/NOTES for whether
    the move has landed. **Do not add it back to `recon-tool` — that holds
    more strongly now than before the move, not less.**
  * **"tryptic" is "enzymatic"** in code and serialised keys. Statements about
    measurements actually made WITH trypsin keep saying trypsin.
  * **The report HTML redesign is DESIGNED, not built.** The agreed layout is
    `testing/scripts/report_layout_mockup.py` — PORT IT; do not re-derive the
    arrangement from prose.
- **Do not re-derive these either; settled 2026-09-02.**
  * **CI EXISTS and is GREEN on four targets.** `.github/workflows/build.yml`.
    Runners: `windows-latest`, `ubuntu-latest`, `macos-15-intel` (apple-intel),
    `macos-latest` (apple-silicon). ⚠ **`macos-13` is RETIRED — do not use it.**
    Jobs and release assets are named by PLATFORM (`apple-silicon`,
    `apple-intel`, `windows-64`, `linux-64`), not by target triple.
  * **`recon-tool/Cargo.lock` is COMMITTED.** 415 packages. Do not re-ignore it.
  * **The tree is rustfmt'd and CI gates on `cargo fmt --check`.** There is
    deliberately NO clippy gate, and adding one is not an improvement — see
    below. ⚠ "about 40 lints remain" is SUPERSEDED (2026-09-03): the list
    measured 44 unique lints and is now **ZERO**. The rule it carried still
    holds — each lint is a real code edit to judge against the tripwires, not
    bulk-apply. **Three of the 44 were WRONG and would have introduced
    defects**, including one that turned a malformed-mzML fallback into a panic;
    five are `#[allow]` with the reason at the site. A clippy gate would make
    the next such lint something to silence quickly, which is exactly how those
    three would have shipped. See NOTES "The clippy pass".
  * **The release archive ships four files** — binary, `README.md`,
    `THIRD_PARTY_LICENSES.md`, `unimod.xml`. The last two are DSL Section 3
    obligations. This holds even if `unimod.xml` becomes an internal database:
    a compiled database is an Object Form, and the XML stays the Source Data.
    See NOTES.
  * **`testing/reference-data/unimod.xml` is `-text` in `.gitattributes`.** A
    Windows CI checkout rewrote its line endings and shipped a corrupt copy.
    Do not remove that guard.
  * ⚠ **GREEN CI IS NOT THE NUMERICAL TRIPWIRE.** `cargo test` reads 191 passed
    on a bare checkout, the SAME as a full one, because 25 data-dependent
    assertions skip themselves — their data is gitignored. `run_validation.py`
    cannot run in CI at all. The numerical gate stays a LOCAL run.
  * **q-derived counts JITTER run to run** (bcell pass-1 PSMs over seven runs:
    72801, 72802 x5, 72803). **Never `assert_eq!` a q-derived count** — band it,
    and assert the claim the number exists to support.
- **Keep README in sync.** If a change alters anything README describes,
  update README in the same pass. It rots silently; treat that as a bug.
- **Update BOTH PLAN and NOTES at a phase boundary**, and keep them agreeing —
  if PLAN says a phase is complete, NOTES should too.

## Start of session
Read the files above. Before writing any code, sanity-check that PLAN's status
block, its checkboxes, and NOTES agree with each other and with the actual repo
— flag anything stale or contradictory. (This catches a botched shutdown from
last session for free.) Then state the next step to confirm you're oriented.

## End of session (shutdown routine)
Do these in order, and reply with each step and its result so nothing is
silently skipped — a prose "done!" hides gaps; an itemized report surfaces them.
1. Update the status block in `PLAN.md` (current state + next action).
2. Tick finished PLAN checkboxes (say "none" if nothing changed).
3. Update `NOTES.md`. New choices go in with the rejected alternative recorded.
   New discovered truths and dead-ends go in with the right markers. **Entries
   are edited in place: if a result contradicted an entry, correct that entry.
   Do not append a correction and leave the wrong text standing.** Superseded
   numbers must be marked where they sit, not only in a new section.
4. Update `README.md` if anything it describes changed (say "no change" if not).
5. Run the debrief and append it to the TOP of `JOURNAL.md`.
6. Commit AND push, with a plain `git push`. Report the commit hash and confirm
   the remote accepted the push — these are two separate operations and
   "committed" is not "pushed." One remote only; see "Remote" above.

## Debrief
Ask Q1 and Q5 every session; all five for big sessions. This is step 5 of
shutdown, but it doesn't depend on the agent remembering to run it — you can
trigger it directly at any point ("run the debrief"), which is the more reliable
habit. Either way the output gets appended to the top of `JOURNAL.md`.
1. What are you least confident about, and what would prove each one right or wrong?
2. What did you assume without stating it?
3. What's the biggest thing I'm missing here?
4. What could I have done differently to make this session more useful?
5. What would you suggest to improve?
