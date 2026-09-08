# recon: a Sage-based proteomics reconnaissance tool

`recon` is a reconnaissance tool for unfamiliar bottom-up proteomics data. It
runs one wide open search plus one semi-enzymatic confirmation pass, then emits a
single report that answers three questions about a file you have not worked with
before: which modifications are present, where the MS1 signal went, and what mass
tolerances the data actually supports. It is not a general-purpose search engine
and it produces no protein list for publication. The intended user is a
proteomics practitioner facing a new species, tissue, matrix, or instrument, who
wants an unbiased first look before committing to a search strategy.

```
recon run <MZML> <FASTA> --enzyme <ENZYME>
```

Version 0.1.2. Everything the tool needs is compiled into one binary: the
[Sage](#third-party-software) search engine, the Unimod element table, and the
curated modification list. There is no search engine to install, no path to
configure, and no network call at search time.

## Why this exists

Byonic Preview established the idea we are building on: survey a shotgun run
before the real search, and emit search parameters rather than results (Kil et
al., *Anal. Chem.* 2011, 83(13), 5259–5267). We wanted that capability without
its constraints, and we wanted several things it does not provide. Five design
choices follow from that.

**It is free and open.** Preview is commercial software. `recon` is
NIST-developed, source-available, and built on an open search engine, so a core
facility or an individual lab can run it without a licence and can read exactly
how every number was produced.

**It is alkylation-agnostic.** The open search assumes no fixed modification at
all, including carbamidomethyl on cysteine. Most pipelines hard-code +57.0215 on
C and therefore cannot see an incomplete alkylation, an over-alkylation, or an
alternative alkylating agent, because the assumption removes the evidence before
the evidence is examined. `recon` reports the cysteine chemistry it observes and
recommends the fixed modification rather than presuming it.

**It adds a statistically gated recommendation step.** A delta-mass histogram
lists peaks; it does not tell a user which peaks belong in the next search.
`recon` routes every detected peak through an explicit decision path and records
which route it took: residue-specific candidates are decided by a Fisher exact
test with an odds ratio and Benjamini-Hochberg correction, unspecific ones (any
residue, or an acceptor present in nearly every peptide) by a per-file abundance
floor, and isotope satellites are demoted with their parent named. Peaks with no
curated match are reported in a tail and never recommended. The report prints the
odds ratio and q-value for every decision, including the ones that failed, so the
gate is auditable rather than asserted.

**It measures the digest instead of assuming it.** A second Sage pass re-searches
the proteins the open search identified, semi-enzymatically, inside the mass
window the first pass measured. Peptide termini are then classified against the
protein sequences as fully enzymatic, N-ragged, C-ragged, or non-enzymatic. Every
rate is printed with its numerator and denominator, and those denominators count
distinct peptides, which is the basis Preview states for its own percentages
("the number of peptides with the property divided by the number that could have
that property", Kil et al. 2011). We adopted that basis deliberately so the two
reports sit on the same axis, though we have not run Preview and recon on the
same file and then reconciled the figures line by line.

**It is fast enough to run routinely.** Our serum test file contains 41,788 MS/MS
scans and completed in 107.2 seconds with the v0.1.2 binary on a standard work
computer, with no performance tuning applied. Critically, that is one file on one
machine and not a controlled benchmark: we have measured no comparative timing
against any other tool, and we make no speed claim relative to one. What the
figure supports is narrower and still useful: a survey at this cost can be run on
every unfamiliar file rather than reserved for problem cases.

Several of the points above are design intent rather than demonstrated
superiority. We have not run a head-to-head study against Preview or against any
other survey tool, and the [Limitations](#limitations) section states what our
evidence base does and does not cover.

## Quick start

```
recon run sample.mzML.gz UniProt-Human.fasta --enzyme trypsin
```

This writes `sample_recon.json`, `sample_recon.html`, `sample_recon_pass2.json`,
and a `sample_recon_search/` directory beside them. Open the HTML file in a
browser.

## Installation

### Requirements

- Windows x86-64, macOS (Apple Silicon or Intel), or Linux x86-64.
- A stable Rust toolchain (1.75 or later recommended), installed with `rustup`.
  This is needed only to build from source; a release binary needs no toolchain.
- An mzML file (`.mzML` or `.mzML.gz`) and the protein FASTA the file should be
  searched against.

No Sage binary is required. Sage v0.15.0-beta.2 is a Cargo git dependency
compiled into `recon`, pinned to commit
`df9219951cc9a54cf4cd55d76541af24b687bd3d` on the upstream `lazear/sage`
repository, with no fork and no local patch. A beta is pinned deliberately,
because no v0.15.0 final exists upstream; the pin is the exact commit, so
reproducibility does not depend on tag stability. Linking Sage as a library sends
no telemetry: its `Telemetry::send` is called only from Sage's own `main.rs`,
which `recon` does not use.

### Getting the binary

**There is no prebuilt release yet. Build from source, described below.** The
build is one command and needs only a Rust toolchain.

We intend to publish archives for Windows, Linux, macOS Intel and macOS Apple
Silicon. The workflow that produces them is committed at
`.github/workflows/build.yml` and builds all four targets on a version tag.
GitHub Actions is disabled for this organization, so it does not run here. Until
that changes, the source build is the supported route.

If you redistribute the binary, keep `README.md`, `THIRD_PARTY_LICENSES.md` and
`unimod.xml` with it. The last two are Design Science License Section 3
obligations for Unimod, not optional extras. `unimod.xml` never needs to be
passed on the command line (a copy is compiled into the binary); the shipped file
satisfies the licence and lets a reader inspect the source data.

### Build from source

```
cd recon-tool
cargo build --release
```

The binary is written to `recon-tool/target/release/recon` (`recon.exe` on
Windows). The first build also compiles Sage from the pinned commit, so expect
several minutes.

## Usage

```
recon run <MZML> <FASTA> --enzyme <ENZYME> [--output NAME]
```

The mzML file and the FASTA are positional and both are required. `--enzyme` is
required and has no default: assuming trypsin would silently mis-report every
digestion number for any other protease, so `recon` refuses to guess. See
[Choosing the protease](#choosing-the-protease).

| flag | effect |
|---|---|
| `-e`, `--enzyme <ENZYME>` | Protease. Required, no default. A preset name or an explicit `CLEAVE_AT[/RESTRICT][/n]` rule. |
| `-o`, `--output <NAME>` | Output base name. Defaults to the mzML basename plus `_recon`, in the current directory. |

The remaining flags are grouped under `Advanced` in `--help` and are not needed
for a normal run.

| advanced flag | effect |
|---|---|
| `-u`, `--unimod <PATH>` | Use an external `unimod.xml` instead of the compiled-in copy. |
| `--params <PATH>` | Open-search parameter template. `mzml_paths`, `database.fasta` and the enzyme identity are overridden at run time; everything else comes from the template. |
| `--search-out <DIR>` | Directory for Sage output. Defaults to `<NAME>_search`. |
| `--cleave-at <RESIDUES>` | Override the resolved enzyme's cleavage residues, for example `KR`. |
| `--restrict <RESIDUES>` | Override the restriction residues. An empty string means none. |
| `--c-terminal <BOOL>` | Override whether cleavage is C-terminal to the matched residue. |
| `-q`, `--q-threshold <F>` | PSM q-value threshold, default `0.01`. Changing it makes results incomparable with our pinned baselines, which are all at 0.01. |
| `--pass2-params <PATH>` | Pass-2 parameter template. Its `precursor_tol`, `fragment_tol` and `database.fasta` are overridden with what Pass 1 measured. |
| `--no-pass2` | Skip the semi-enzymatic pass. It roughly doubles wall-clock time, so this exists for the case where only the open-search report is wanted. |

Eleven further subcommands exist as internal development tools. They are hidden
from `--help` and are not part of the product surface.

### Output files

With `--output NAME` (or the default base name):

| file | contents |
|---|---|
| `NAME.json` | The full report, schema version 3.2.0. |
| `NAME.html` | The same report as a self-contained page for a human reader. |
| `NAME_pass2.json` | The semi-enzymatic digestion measurement, schema version 1.1.0. Absent with `--no-pass2`. |
| `NAME_search/` | Sage's own outputs (`results.sage.tsv`, `results.json`) plus `effective-params.json`, the exact configuration the search ran with. |

Two usage notes are worth stating before they cost you a run.

**Pass `--output` a base name with no extension.** `--output out/serum` gives
`serum.json` and `serum.html`; `--output out/serum.json` gives `serum.json.json`.

**The FASTA must be the database the search should use.** A mismatch is a hard
error rather than a silent empty result: at least 95 % of target PSMs must
resolve an accession. Protein-terminal modifications (protein N-terminal
acetylation and the Met-loss forms) can only be decided with the protein
sequences in hand; without them they fall back to the abundance floor. The report
records which mode it ran in, because "not tested" and "not supported" are
different claims.

## Choosing the protease

`--enzyme` accepts a preset name or an explicit rule. It sets enzyme IDENTITY
only, never the search tuning (`missed_cleavages`, `min_len`, `max_len`,
`semi_enzymatic`), which Pass 1 and Pass 2 set differently on purpose.

Fourteen presets ship. The rules are transcribed from Mascot's published enzyme
list, with the deviations recorded in
[`_dev/reference-notes/mascot-enzymes.md`](_dev/reference-notes/mascot-enzymes.md).

| preset | cleaves at | but not before | cut side |
|---|---|---|---|
| `trypsin` | K, R | P | C-terminal |
| `trypsin/p` | K, R | none | C-terminal |
| `arg-c` | R | P | C-terminal |
| `asp-n` | D | none | **N-terminal** |
| `asp-n/ambic` | D, E | none | **N-terminal** |
| `chymotrypsin` | F, Y, W, L | P | C-terminal |
| `cnbr` | M | none | C-terminal |
| `glu-c` | E | P | C-terminal |
| `glu-c/de` | D, E | P | C-terminal |
| `lys-c` | K | P | C-terminal |
| `lys-c/p` | K | none | C-terminal |
| `lys-n` | K | none | **N-terminal** |
| `pepsin-a` | F, L | none | C-terminal |
| `trypchymo` | F, Y, W, L, K, R | P | C-terminal |

Two of these are buffer-dependent, which is why each ships as a pair. Collapsing
either to a single name would silently choose a reaction condition on the user's
behalf.

- **Glu-C / V8** cleaves after E in phosphate buffer (`glu-c`) but after both D
  and E in ammonium bicarbonate (`glu-c/de`).
- **Asp-N** cleaves before D (`asp-n`), and before both D and E in ammonium
  bicarbonate (`asp-n/ambic`).

The preset list is a convenience, not a ceiling. Any protease can be given as an
explicit rule of the form `CLEAVE_AT[/RESTRICT][/n]`, where a trailing `/n` means
cleavage N-terminal to the matched residue:

```
recon run sample.mzML.gz database.fasta --enzyme "KR/P"
recon run sample.mzML.gz database.fasta --enzyme "D//n"
recon run sample.mzML.gz database.fasta --enzyme lys-c --restrict ""
```

`--cleave-at`, `--restrict` and `--c-terminal` override single fields of whatever
`--enzyme` resolved to.

Sage accepts the 20 standard residues plus `U` and `O`. The ambiguity codes `B`,
`Z`, `J` and `X` are rejected with an error, which is why Mascot's `BD` for Asp-N
and `EZ` for Glu-C/V8-E ship here as `D` and `E`.

## Curated modifications

`recon` names a delta mass, and decides which residues can carry it, from a
curated list of **99 entries** compiled into the binary. The list is a dated
snapshot of the curated modification files from MetaMorpheus (MIT licence) taken
at commit `7e453540`, with local corrections recorded in
`THIRD_PARTY_LICENSES.md`. Masses are not stored in those source files; we
compute each one from the entry's chemical formula using the element table in the
bundled `unimod.xml`, which is the same rule the tool applies at run time.

We adopted this list rather than all of Unimod deliberately. Unimod is a
catalogue of everything ever reported, so a Unimod fallback would reintroduce the
+57.0215 mass degeneracy (carbamidomethyl versus the glycine residue mass, and
the equivalent degeneracies at Ala +71, Ser +87, Pro +97 and Val +99) without
saying so. The curated list is a working set with acceptor residues already
assigned.

The full list, with source file, category and acceptor residues for every entry,
is at [`docs/curated-modifications.md`](docs/curated-modifications.md). That
document is generated from the same four files the binary embeds and is not
edited by hand.

## Mass tolerance assumptions per analyzer

`recon` reads the mass analyzer from the mzML header before it searches, and sets
the pass-1 MS2 fragment tolerance from the detected class. The search runs once,
so a fragment tolerance in the wrong unit cannot be corrected afterwards.

| analyzer class | pass-1 MS2 fragment tolerance | basis |
|---|---|---|
| Orbitrap / FT-ICR | ±20 ppm | **Validated on real data** (all four test files) |
| Astral | ±20 ppm | Curated assumption, no local data |
| Legacy TOF / QTOF | ±100 ppm | Curated assumption, no local data |
| Ion trap / quadrupole | ±1.0 Da | Curated assumption, no local data |
| Undetermined or mixed | ±20 ppm (fallback) | Behaves as if Orbitrap; flagged in the report |

**Only the Orbitrap value is validated against real data, and this is the most
important caveat in this section.** All four of our test files are Orbitrap on
both MS levels. The Astral, legacy-TOF and ion-trap values are curated choices
made from documented instrument behaviour, exercised by unit tests, and backed by
no measurement of ours. They are stated here as assumptions so that a user on one
of those instruments treats the first run as a check on the assumption rather
than as a result.

The Orbitrap value of 20 ppm is itself a documented worst case for the class, not
an empirical optimum on our files. All four would keep improving below 20 ppm;
tuning to them would overfit a default that has to survive somebody else's
poorly-calibrated instrument.

The undetermined-analyzer fallback deserves an explicit warning. It equals the
Orbitrap bucket, so on a file whose analyzer cannot be read, `recon` searches as
if it were an Orbitrap. That is reasonable for the Thermo-dominated data this
tool has seen and wrong for an unreadable ion-trap file, which needs roughly 1
Da. The report sets `assumed: true` and says so every time the fallback is used.

## What the report contains

The HTML page presents the same content as the JSON, arranged for reading. The
sections are:

1. **Detectors.** Instrument model, MS1 and MS2 analyzer classes, MS2 scan count,
   and the pass-1 fragment tolerance that followed from them, with its basis
   (detected or assumed).
2. **Mass accuracy.** Signed median precursor error, absolute median fragment
   error, the measured MS2 tolerance interval, and a combined
   **Recommended MS1 / MS2** setting for your next search. The MS1 half is
   quantized onto a {10, 20, 50, 100} ppm ladder covering `|bias| + 5×MAD`,
   because a user picks a search setting from a discrete set and precision below
   a rung is unusable. The MS2 half is the measured fragment spread on the same
   ladder when the MS2 analyzer is ppm-based, and is converted to Daltons and
   rounded up to the nearest 0.1 Da when it is not.
3. **Contamination.** Common polymer series (PEG, PPG, Tween, polysiloxane and
   others) as a percentage of MS1 TIC, with a per-series breakdown and a
   qualitative level.
4. **Glycopeptides.** Oxonium-ion screening: the count of candidate MS2 spectra
   and their percentage of all MS2. This flags glycopeptide presence; it does not
   identify glycopeptides.
5. **Digestion.** Missed-cleavage rate, ragged N-terminus and ragged C-terminus
   rates, and their ratio, from the semi-enzymatic Pass 2 (described below).
6. **Recommended search modifications.** Three tables: fixed, variable, and
   "detected but did not make the cut". Every row carries the delta mass, the
   candidate name and acceptor residues, the PSM count and percentage, which
   decision route was taken, and the evidence (odds ratio and q-value, or the
   abundance floor, or the satellite parent). The page also prints the decision
   chart itself and offers the three tables as CSV.

The JSON report additionally carries blocks that have no separate HTML section:
the full **modification discovery** histogram with per-peak PSM counts and
intensities, the **alkylation** summary (observed cysteine
chemistry and the unalkylated fraction), and the **MS1 calibration** detail
behind the tolerance recommendation.

### Pass 2, the semi-enzymatic digestion measurement

The proteins Pass 1 identified are written to a subset FASTA and re-searched
semi-enzymatically, inside a window centred on the measured mass bias rather than
on zero. That bias-centred rung covered 99.3–99.9 % of real PSMs across our four
test files, against 81–87 % for a `bias ± 3×MAD` window, and no MAD multiple
transferred between instruments. Peptide termini are then classified against the
protein sequences.

The reported quantity is defined, not inferred. The headline is **cleavage
completeness = 100 − % missed cleavage**, counted over distinct peptides, with
missed-cleavage and ragged rates taken over fully plus semi-enzymatic peptides
and non-enzymatic peptides excluded from the denominator. Those are Byonic
Preview's own denominators, read from its report for NIST liver RM 8461. Counts
are decoy-subtracted per specificity class, as Preview does, which matters: on
the liver reference the class FDR is 0.10 % for fully-tryptic peptides against
10.10 % for semi-tryptic, so a global 1 % cut is carried by the fully-tryptic
majority.

Three boundaries on this number are worth stating in advance.

- **Pass 1's own semi-enzymatic rate is a control, not a prediction.** Pass 1 is a
  fully-enzymatic search and cannot generate a ragged peptide, so it reads about
  0 % by construction. The two rates have different denominators and different
  search spaces, and their difference is not a delta.
- **Non-enzymatic peptides are reported as a count, never as a rate.** Under
  `semi_enzymatic` Sage only generates candidates with one non-specific terminus,
  so a fully non-enzymatic peptide is never scored. Measuring it needs a third,
  non-enzymatic search, which does not fit in 8 GB of RAM and is not run.
- **Pass 2 searches with no modifications**, which makes modified peptides
  unreachable and shifts the serum ragged rate by +2.7 percentage points (32.05 to
  34.76 %). This is a stated limitation of the number rather than an open
  question. Carrying Pass 1's discovered modifications into Pass 2 was built, run,
  and rejected: on our liver file the search was killed by the operating system 21
  s in and produced nothing.

We deliberately do not call this "digestion efficiency", because Mouchahoir &
Schiel 2018 and Davis et al. 2019 both use that phrase as an umbrella over a set
of metrics. There is also **no composite score**: missed cleavage and ragged
termini are opposite phenomena, neither reference tool sums them, and a composite
we prototyped scored normal serum biology as a mediocre digest because its rubric
assumed a cell-culture digest. The report presents raw numbers and lets the user
decide.

## Example output

A complete report from our human serum test file (Orbitrap Fusion Lumos, tryptic,
41,788 MS/MS scans) ships in this repository:

- [`examples/serum.html`](examples/serum.html) (open in a browser)
- [`examples/serum.json`](examples/serum.json)
- [`examples/serum_pass2.json`](examples/serum_pass2.json)

## Testing and validation

Two independent surfaces are maintained, and they cover different things.

**Unit and integration tests: 211 pass, 0 fail** (`cargo test` in `recon-tool`).
These cover mass and formula arithmetic, the analyzer-class table against the
PSI-MS controlled vocabulary, tolerance quantization in both the ppm and the
Dalton regime, enzyme-rule parsing for all fourteen presets and the custom-rule
form, peak detection, the decision routing described above, and report
serialization for both schemas.

**Numerical validation harness: 17 of 17 gates passed**
(`_dev/testing/scripts/run_validation.py`). This is the tripwire that certifies
derived numbers against committed reference outputs, including cross-tool
comparisons with PTM-Shepherd, MSFragger, MetaMorpheus and Mascot on our test
files. It is a local run and cannot execute in CI.

Three honest qualifications belong with those counts.

**A green test run on a bare checkout exercises fewer assertions than the number
suggests.** Many data-dependent tests need mzML and FASTA inputs that are
gitignored and therefore absent from a fresh clone. Those tests skip themselves
rather than fail, so the passing count does not tell you whether they ran. This
does:

```
cargo test -- --nocapture 2>&1 | grep skipping
```

Every line it prints names an input the suite could not reach. A checkout with
all inputs present prints nothing.

**Counts derived from a q-value threshold jitter between runs.** Pass-1 PSMs on
our B-cell file measured 72801, 72802 (five times) and 72803 over seven identical
runs. No test asserts equality on such a count; each asserts a band, and asserts
the claim the count exists to support.

**Unit tests built on synthetic fixtures inherit the assumptions of the code they
test.** They can catch an arithmetic regression; they cannot catch a wrong
convention. That is why the numerical harness runs against real files with
independently known answers, and why the limitations below are stated in terms of
which real files exist.

The Sage dependency is pinned by git revision in `recon-tool/Cargo.toml`, with
`recon-tool/Cargo.lock` committed (415 packages). One caveat applies to that pin
and should appear in any Methods text: a git revision pins Sage's source, not its
dependency graph, and 155 of 335 shared packages resolve differently here than in
Sage's own lock file.

## Limitations

**The entire evidence base is four tryptic Orbitrap files.** Nothing in this
project has ever been run on a non-tryptic digest or on an ion-trap instrument.
Thirteen non-trypsin presets ship, transcribed from Mascot, and none has been
exercised end to end on real data. The Dalton tolerance regime is covered by unit
tests and by no measurement. This is the largest gap in the project and it
conditions every other number here: thresholds derived from four files can
overfit, which is why the modification tiers derive their floor from each file's
own distribution rather than from an absolute percentage.

**No per-residue localization.** `recon` reports an un-localized delta mass and a
population-level acceptor enrichment. It does not state which residue on which
peptide carries the modification. Consequently our percentages are not directly
comparable to tools that localize (PTM-Shepherd, MetaMorpheus, Mascot), and the
difference must be stated wherever a percentage is quoted.

**Percentages are discovery-rank statistics, not occupancy.** We report the
fraction of PSMs whose observed delta mass fell in a bin from a single open
search, in which each spectrum is assigned one delta mass, so occupancy is split
across every form a peptide takes. Other tools report the fraction of PSMs
carrying a modification after explicit scoring and localization. Use the ranking,
not the magnitude: a targeted search with these modifications enabled will report
higher numbers for the same chemistry. Measured against localizing platforms, our
+57 percentage runs about 0.41–0.42× theirs, consistently across three files, and
the mechanism is only partly identified.

**Protein N-terminal modifications are attempted, not closed.** `recon` detects
protein-terminal modifications and can decide them statistically against the
search FASTA. It does not claim parity with a dedicated N-terminomics workflow,
and the gap is measurable: on one of our files MSFragger finds 137 N-terminal
acetyl PSMs where `recon` recommends none, because that population never forms a
delta-mass peak that clears detection. So "protein N-terminal modifications are
minor in this file" is a statement about `recon`'s view, not about the sample.

**The MS1 tolerance recommendation ignores the MS1 analyzer.** It always
quantizes onto the ppm ladder, whatever MS1 analyzer was detected, so an
instrument doing MS1 survey scans in an ion trap or quadrupole would receive a ppm
number that is meaningless at unit resolution. The MS2 recommendation is
analyzer-aware and a trap can never receive a ppm rung; the MS1 side has no
equivalent branch. This is a recorded scope decision, not an oversight: closing it
is a schema change across every ppm-named field of the recommendation, and we have
no file of that class.

**Unimod annotation is ambiguous at some masses.** Nearest-match annotation
returns plausible but non-authoritative labels where several modifications share a
mass (+43 carbamyl versus trimethyl, +28 formyl versus dimethyl). The mass is
reported faithfully; the name is a best match.

**Residue-mass degeneracy is not handled programmatically.** The +57.0215
carbamidomethyl / glycine degeneracy, and its equivalents at Ala, Ser, Pro and
Val, are resolved by hand if at all. The manual method exists (we ran a flanking
check on our one real candidate and found 0 of 26 unique non-cysteine +57
peptides had glycine flanking context, indicating over-alkylation rather than
added glycine); the automated pass does not.

**FDR is flat across mixed peptide populations in Pass 2.** A single
`peptide_q ≤ 0.01` applied to a mixed tryptic and semi-tryptic set inflates
semi-tryptic false positives, because semi-tryptic peptides score worse on
average. Stratified FDR by terminus class would address this and is not
implemented.

**Mass accuracy is run-specific and must not be compared across files.** One
report describes one file. Two of our four files disagree with reference tools on
mass error and the disagreement is unresolved: serum agrees within 0.16 ppm three
ways, while the B-cell MS1 bias flips sign between `recon` (+0.65) and
MetaMorpheus (−0.295) with no third source to break the tie.

**Stated non-goals.** There is no quantitation, no general QC, no parameter
auto-configuration, and no re-run loop: the tool measures once, applies once, and
reports both. If Pass 2's observed window disagrees with what Pass 1 predicted,
that is reported as a note and never silently corrected. The tool also does not
judge sample quality. A recommendation is a search-parameter suggestion, never a
verdict on the sample.

## Future work

These are ordered by how much they would change what a user can trust.

1. **Other detector classes.** Ion trap, legacy TOF and Astral files, to replace
   the curated tolerance assumptions in the table above with measurements. An
   ion-trap file would also settle whether the MS1 recommendation needs a Dalton
   branch.
2. **Non-tryptic digests.** End-to-end runs on Lys-C, Glu-C, chymotrypsin and
   Asp-N data, to exercise the thirteen presets that currently ship untested on
   real data.
3. **Samples with heavy detergent load.** The polymer series are detected and
   quantified as a fraction of TIC, but no file in our panel carries a large
   surfactant burden, so the reporting level thresholds are untested at the high
   end.
4. **Specific modification classes.** Glycopeptides (oxonium screening currently
   flags presence without identification), and an automated residue-mass
   degeneracy pass, to be activated if a future file's flanking check returns
   glycine-context peptides.
5. **Prebuilt binaries for Windows, Linux, macOS Intel and macOS Apple Silicon.**
   All four build from source today, and none is released. The four-target
   workflow is written and committed, so this is blocked on GitHub Actions being
   available for this organization rather than on work we have not done.
6. **Stratified FDR by terminus class**, so Pass 2 semi-enzymatic rates can be
   reported with proper error control.
7. **Grow the file panel past four**, which would allow finer recommendation tiers
   and absolute rather than file-relative thresholds, if those prove stable.
8. **Remove vestigial code left over from development.** Some functions were
   built during development, kept as the design changed, and are no longer part
   of the analysis this tool presents. The signal-fate block is the known case:
   it still populates a JSON field and prints an identification rate to the
   console, but it has no section in the report and it is not something we ask a
   user to act on. These should be audited and removed rather than left to
   accumulate, because a field in the output implies a claim we are making.

## Citation

If you use `recon` in published work, please cite the software:

> Neely, B.A. (2026). *sageRecon: a Sage-based proteomics reconnaissance tool*
> (Version 0.1.2) [Computer software]. National Institute of Standards and
> Technology. https://github.com/usnistgov/sageRecon

Cite the version you ran, not the repository in general, because the
recommendations a report makes depend on the pinned search engine and on the
curated modification list, and both move between versions. `recon --version`
prints the version, and every report records it.

A `CITATION.cff` file is included, so GitHub's "Cite this repository" control
produces the same reference in BibTeX or APA.

Please also cite Sage, which performs the searches:

> Lazear, M.R. "Sage: An Open-Source Tool for Fast Proteomics Searching and
> Quantification at Scale." *Journal of Proteome Research* 2023, 22(11),
> 3652–3659. doi:10.1021/acs.jproteome.3c00486

## Third-Party Software

`recon` incorporates code from:

- **Sage** (Michael Lazear), MIT License. Compiled in as a Cargo git dependency
  at the pinned commit named above. The source is not modified.
- **mzSniffer** (William E. Fondrie), Apache License 2.0. The polymer-series
  detection logic is ported from it, with changed files marked as required by
  Apache License 2.0 Section 4(b).

It also redistributes third-party DATA:

- **MetaMorpheus** (Smith Chem Wisc), MIT License. Four curated modification
  files are compiled into the binary. Three are unmodified; `Mods.txt` carries
  local corrections and additions, itemized in `THIRD_PARTY_LICENSES.md`. No mass
  or acceptor residue in an upstream entry was changed.
- **Unimod**, Design Science License. `unimod.xml` is a dated, unmodified
  snapshot. The licence text is reproduced in full in
  `THIRD_PARTY_LICENSES.md`, which the Design Science License requires to ship
  with any redistribution.

Full licence texts, attributions, and per-file modification notices are in
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

## License

This software was developed by employees of the National Institute of Standards
and Technology. See [LICENSE.md](LICENSE.md) for the NIST Software Licensing
Statement. Third-party components retain their original licences; see
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

## Contact

Benjamin A. Neely
Data Science and AI Group, Material Data Division
Material Measurement Laboratory
National Institute of Standards and Technology
benjamin.neely@nist.gov
