# Limitations and Future Work — write-up draft

**Status:** Draft for the v0.1.0 write-up. Update as ship-track steps change
what is true.
**Purpose:** Gather every known limitation in one place, with its source, so the
write-up states them plainly instead of discovering them late.

Each item gives the limitation, why it exists, and where the evidence sits.

---

## Design limitations — chosen, not defects

**The MS1 tolerance recommendation ignores the MS1 analyzer** (2026-09-03). It
always quantizes onto the {10, 20, 50, 100} ppm ladder, whatever the detected MS1
analyzer is. The MS2 recommendation IS analyzer-aware — a trap or quadrupole MS2
gets Daltons and can never receive a ppm rung — but the MS1 side has no
equivalent branch. Consequence: on an instrument doing MS1 survey scans in an ion
trap or quadrupole, recon prints a ppm number that is meaningless at unit
resolution.

The asymmetry is structural, not an oversight. `ms2_user_recommendation` returns
a unit-carrying enum, so Daltons are already a first-class result;
`ms1_user_recommendation` returns a struct whose every field is ppm-named and
serialised into the schema, and it takes no analyzer argument. Closing the gap
means either changing what those ppm fields mean on a trap — a MAJOR schema bump
— or adding parallel Da fields and a discriminator, plus a signature change
across 11 call sites. Ben's call: not worth it for an instrument class nobody
uses for MS1 survey scans, and for which this project has no files at all.
Reopening condition is simply that such a file arrives. Note this shares the
root cause of the gap below: **the entire Da regime is curated and unmeasured.**

**No per-residue localization.** The tool reports an un-localized delta mass. It
does not say which residue carries it. This is locked in NOTES. Consequence: our
output is not directly comparable to tools that localize (PTM-Shepherd,
MetaMorpheus, Mascot), and the difference must be stated whenever a percentage
appears. Mascot in particular reports by Unimod name plus site and splits one
mass across many site rows, which is why its adapter must roll site rows back up
before comparison.

**Percentages are discovery-rank statistics, not occupancy.** Recon reports the
fraction of PSMs whose observed delta fell in a bin. Other tools report the
fraction of PSMs carrying a modification after explicit scoring and
localization. Four tools in the benchmark use four different currencies: recon =
open-search delta-bin fraction; PTM-Shepherd and MetaMorpheus =
post-localization PSM fraction; Mascot = resolved error-tolerant fraction. This
is the single most important caveat in the write-up.

**No quantitation, no general QC, no parameter auto-configuration.** Stated
non-goals. There is no re-run loop: the tool measures once, applies once, and
reports both. If Pass 2's observed window disagrees with Search 1's prediction,
that is reported as a note, never silently corrected.

**The tool does not judge sample quality.** The composite digestion score was
built and then removed, because it scored normal serum biology as a mediocre
digest — the rubric assumed cell culture, and the tool cannot know sample type.
The report presents raw numbers and lets the user decide. The same discipline
constrains the PTM tiers: a tier is a search-parameter suggestion, never a
verdict on the sample.

**The tool is never compared to its author.** Ben's rule-of-thumb modification
set is a sanity read across the whole benchmark panel, not a validation
criterion for any one tool. Comparing a tool to its author's expectations is
unfalsifiable. This constrains what the PTM tiers may be tuned against.

---

## Measurement limitations — real, quantified

**The ~0.4x magnitude gap versus localizing platforms.** Recon's +57 percentage
runs about 0.41–0.42x the platforms', consistently across all three files. It is
not a denominator artifact: PSM totals are comparable, and recon's peak count
matches the raw TSV count in the delta window (b1906: peak 1253, raw 1334
q<0.01 target PSMs in [56.98, 57.06]). The gap is search-time PSMs a single-pass
open search does not recover.

*Cause partly identified.* The Option-C ceiling POC fed MSFragger-calibrated
mzML into Sage. PSMs went DOWN on all three files and bcell's deamidation apex
stayed flat. Recalibration is therefore ruled out as the mechanism. Two
candidates remain: the narrow first pass, and localization-aware rescoring.
Settling it would need per-stage PSM lists from a FragPipe run with
intermediates kept.

**Part of the gap is our own peak splitting.** Augmenting +57 with its
satellite peak moves bcell 4.50 -> 5.78% and serum 7.31 -> 9.86% — roughly a
quarter to a third of the distance to the platforms. This follows from
fold-to-zero and the disabled satellite folding, both deliberate.

**The ±1/±2 Da quantization carpet.** Roughly 1% of PSMs appear as an
unannotated small-delta carpet that PTM-Shepherd does not show. Per-stage
instrumentation traced its entire growth to the prominence detector: a ~1 mDa
shift re-bins PSMs and the threshold-based detector assembles slightly different
peaks. Both originally proposed mechanisms (steal-from-folding,
boundary-crossing) were refuted by the numbers. It is a peak-detection
quantization artifact, below the resolution of any recon decision, and it sets a
reporting-resolution floor: sub-~1% shifts in the unannotated small-delta region
are pipeline noise, not signal.

**Satellite folding is disabled by design.** The function exists and is switched
off. It has no conservation guard, so it can report any number and be narrated
as correct; it once reported 1,770 satellites on an 870-PSM parent, which is
physically impossible. Fold-to-zero — the guarded path, with an asserted
conservation invariant — remains on. Do not re-enable satellite folding without
a hard invariant.

**Mass accuracy is run-specific and must not be compared across files.** One
report per file, never blended. The three test files use three different
instruments (Lumos, QE, QE Plus) and two are public with unknown detector
calibration and loading history.

**FDR is flat across mixed peptide populations in Pass 2.** A single
`peptide_q <= 0.01` applied to a mixed tryptic and semi-tryptic set inflates
semi-tryptic false positives, because semi-tryptic peptides score worse on
average. Mascot avoids this by keeping its two pass search spaces disjoint and
thresholding each separately. Recon does not currently do that.

---

## Step 2's validation surface is thin, and that is a limitation (2026-08-26)

**Step 2 ships with NO binary validation gate on residue assignment.** Its surface is
one asserted invariant plus one reported rate. This is stated rather than glossed
because it is the second time the surface has narrowed.

**What was lost, twice.** Gate 1 validated an abundance ORDERING. Routing by
specificity moved almost every peak to the statistics path, which makes no abundance
claim, so gate 1's surface fell to ONE peak across three files. Gate 5 was built to
replace it and was then retired as a pass/fail — not because it failed, but because
its premise compared two different quantities (below).

**What remains, and it is real:**
- **The carpet invariant, asserted in code.** The abundance floor sits above the
  ±1/±2 Da quantization carpet on all three files, margins **+172.0 / +279.2 /
  +138.6 PSM**. Hard-asserted against committed reports, numbers printed.
- **A residue corroboration RATE.** **18 of 21** statistics-path recommendations are
  corroborated by at least one reference tool, at ranks AA1 28 / AA2 4 / AA3 2. The
  AA2/AA3 agreements are weaker and are reported separately rather than blended in.

**Why the corroboration cannot be a pass/fail.** Recon reports an UN-LOCALIZED delta
mass on a PSM plus a POPULATION-level acceptor enrichment. MetaMorpheus and
PTM-Shepherd run secondary sweeps producing PER-PSM site assignments. Sage's
`hyperscore` is X!Tandem's, a PSM-level spectral match score that rates the peptide
IDENTIFICATION, not the modification SITE, and Sage runs no site-confirmation pass.
The design note said these were different quantities before the gate was written.
Matching them directly is what that note ruled out.

**The three divergences, reported not hidden.** On all three files recon calls the
−17.0265 population **Gln→pyro-Glu on Q** (peptide N-terminal cyclization; odds ratio
27–331, q to 1e-29) while MetaMorpheus calls it **Ammonia loss on internal Asn**. It
is the same reaction — ammonia loss — differing in position, just as Glu→pyro-Glu is
water loss. **Both can be true over a mixed peak.** PTM-Shepherd agrees with recon.
MetaMorpheus was configured for `Glu to PyroGlu on Q` and did not use it, so this is
not a capability limitation. **Recon is not claiming to have resolved this.**

**What would restore a binary gate.** Specify it on COMPATIBILITY rather than
identity: serum +57 → Gly is incompatible with a band that is 96% Cys-containing,
while pyro-Glu vs ammonia-loss is compatible. That distinction is testable, but it
needs a fresh pre-commitment written before it is run, not after.

---

## Protein-terminal modifications need the search FASTA (2026-08-27)

**One limitation, plus a cross-check that turned out to exist.**

**1. The claim depends on an input recon does not otherwise need.** `analyze` reads a
TSV and an mzML. Sage's TSV has no start-position column, so proving a peptide sits at
protein position 0 requires the search database as well. Without `--fasta`,
protein-terminal candidates — protein N-terminal acetylation, and every Met-loss form
— are NOT TESTABLE and fall to the abundance floor. The report records which mode it
ran in (`recommendations.protein_context`), because "not tested" and "not supported"
are different claims. `recon run` supplies it automatically; `analyze` does not.

A FASTA that is not the search database is the dangerous case: nothing resolves, every
protein-terminal candidate fails its acceptor test, and the output reads "no protein
N-terminal modifications present" with no symptom. A 95% accession-resolution floor is
asserted and hard-stops. All three test files resolve 100.00%.

**2. The cross-check EXISTS and corroborates. ⚠ An earlier version of this section
said "no reference tool was run for this class." That was wrong — corrected in place
2026-08-27, the same day it was written.** The claim was made without opening
`fragger.params`, which was committed the whole time. This is the "a summary is not a
source" failure applied to a config file.

MSFragger's committed strictTryp config carries `clip_nTerm_M = 1` (protein N-terminal
methionine trimmed as a variable modification) together with
`variable_mod_02 = 42.0106 [^ 1` (protein N-term acetyl). **No custom modification was
added to obtain this** — it is the config the 2026-08-24 reference run already used,
which is what makes it a fair out-of-the-box comparison rather than a hand-built one.

Splitting bcell's `psm.tsv` against the search FASTA:

| bcell, MSFragger strictTryp | PSMs |
|---|---|
| N-term acetyl, total | 1016 |
| **Met-CLIPPED protein N-term** | **767** |
| Met-RETAINED protein N-term | 248 |

So an independent tool finds the Met-loss+acetyl population in bcell, and finds it
DOMINANT over the Met-retained form. Recon reports 209 PSMs at its −89.0289 peak
(delta band 194, of which 188 at protein position 0).

**The magnitudes are not expected to match, and the difference is not a defect.**
Recon reports an un-localized delta-mass population from an open search that cannot
build Met-clipped peptides; MSFragger reports per-PSM localization from a search space
that can. This is the same quantity boundary that retired Gate 5, so it corroborates
at the rate level and must not be turned into a pass/fail without a fresh
pre-commitment. What it does establish: the population is real, it is in this file,
and recon is not inventing it.

**MetaMorpheus is a possible SECOND cross-check, not yet done.** Its committed search
config has `InitiatorMethionineBehavior = "Variable"`, so Met-clipped peptides are in
its search space too. Whether acetylation was available as a variable modification in
that run has NOT been checked — the GPTMD task may or may not have added it. Check
before citing it.

**Still one file of three.** serum and b1906 have no −89.03 peak, though b1906 carries
22 band PSMs, 21 at protein position 0, all NME-permissive. MSFragger's own N-term
acetyl counts order the same way: bcell 1016, b1906 137, serum 3.

The remaining support is the band's residue composition, textbook NatA (A 119, S 57,
T 6, V 2, G 2, M 2; 98.9% NME-permissive at protein residue 2 against a 55.3%
background). That is an independently known answer and it is why the result was
believed before the MSFragger split was measured.

**Four of the five Met-loss curated entries fire on nothing here.**
`Met-loss` (Unimod 765) and `Met-loss+{Methylation,Succinylation,Myristoylation}` were
added for chemical completeness, not on evidence from this data set. Only the acetyl
form is observed. The bare `Met-loss` absence is the odd one: −131.04 has no peak on
any of the three files while −89.03 carries 209 PSMs on bcell. Unexplained, and
recorded rather than guessed at.

**PROTEIN N-TERMINOMICS IS A FEATURE THIS TOOL ATTEMPTS, NOT ONE IT CLOSES.**
Deliberate scope for v0.1.0, recorded 2026-08-27. Recon detects protein-terminal
modifications and can decide them statistically against the search FASTA. It does NOT
claim parity with a dedicated N-terminomics workflow, and the gap is measurable on the
three test files.

Recon's open search only sees what survives as a delta-mass PEAK. MSFragger's committed
strictTryp run, with `clip_nTerm_M = 1` and protein N-term acetyl enabled, sees the
same populations directly:

| file | MSFragger N-term acetyl PSMs | recon protein-N-term PSMs in RECOMMENDED peaks |
|---|---|---|
| serum | 3 | 52 |
| bcell | 1016 | 329 |
| b1906 | **137** | **0** |

Two honest readings. On bcell recon captures roughly a third of the population. On
**b1906 recon recommends none at all** while MSFragger finds 137 PSMs — the population
is there and never forms a peak that clears detection. So "protein N-term modifications
are minor in these files" is true of RECON'S VIEW, not of the samples.

**Why this is not being pushed further before v0.1.0.** Closing the gap means
understanding what PTM-Shepherd and MetaMorpheus actually do for N-terminomics and how
their quantities relate to recon's un-localized delta-mass populations — the same
quantity boundary that retired Gate 5. That is a research question, not an
implementation task, and it is out of scope for a reconnaissance tool's first release.
**It MUST appear in the write-up** as a named limitation with the table above, not be
left for a reader to discover.

**A `TG=X` protein N-terminal entry is not "unspecific", and reading it that way was a
real defect.** Until 2026-08-27 recon reported `no_residue_support` for bcell's
+42.0109 — a peak that is 63.8% protein N-terminal against a 1.09% background — because
the empty acceptor set short-circuited the test before the position was ever checked.
The rule was correct while protein position was unknowable and was not revisited when
the FASTA lookup made it knowable. Two recommendations were being suppressed on the
three test files. This is worth stating because it is the failure mode the tool is
supposed to avoid: a confident answer that reads like a measurement.

**The tool encodes no enzymology, deliberately.** `TG=M` states the one hard chemical
requirement and the position does the discriminating. Whether protein residue 2 makes
Met excision or acetylation plausible is left to the reader, who knows their organism
and its enzymes. The NatA numbers above are a FINDING reported to the reader, not a
rule any decision path consults.

---

## Scope and coverage limitations

**n = 3 files.** Every threshold and every agreement statistic rests on three
files: a human serum digest, a naive B-cell line, and b1906 (293T). Thresholds
overfit at n=3. This is why the PTM tiers derive their floor from each file's
own distribution rather than from an absolute percentage, and why two tiers ship
instead of five.

**Rank agreement is good but not uniform.** Spearman rho against MetaMorpheus:
serum 0.820, b1906 0.630, bcell 0.491. Against PTM-Shepherd: bcell 0.648,
b1906 0.625, serum 0.284. Serum's weak agreement with PTM-Shepherd is the
expected regime for a high-drift, high-adduct sample, but it is not explained in
detail.

**Two files disagree with the ground truth on mass error, unresolved.** Serum
agrees tightly across recon, MetaMorpheus, and MSFragger (MS1 within 0.16 ppm
three ways). bcell's MS1 bias flips sign between recon (+0.65) and MetaMorpheus
(−0.295), with no third source to break the tie. b1906's MS2 bias is the
sharpest miss (recon +0.98 versus ~0 from two independent sources). Two
hypotheses are named and neither is verified: different PSM populations
(wide-open versus narrow-closed), and per-PSM versus per-datapoint aggregation.

**No glycopeptide search.** Oxonium ion screening flags glyco presence; it does
not identify glycopeptides. Byonic Preview has the same limitation.

**Residue-mass degeneracy is not handled programmatically.** +57.0215 is both
Carbamidomethyl and the glycine residue mass; the same degeneracy exists at Ala
+71, Ser +87, Pro +97, Val +99. A dedicated pass was designed and deferred by
evidence: serum's +57 peak was the one real candidate and was run down manually
with a flanking check (0 of 26 unique non-Cys +57 peptides had Gly flanking
context), proving over-alkylation rather than added glycine. The manual method
exists; the automated pass does not.

**Unimod annotation is ambiguous at some masses.** Nearest-Unimod matching at
±0.01 Da returns plausible but non-authoritative labels where several
modifications share a mass (+43 carbamyl versus trimethyl; +28 formyl versus
dimethyl). The mass is reported faithfully; the name is a best match.

**Sage version pinned.** SAGE_VERSION 0.14.7, with a hard error on mismatch and
a TSV schema validator. A known upgrade risk is recorded: v0.15.0 flips the sign
of `precursor_ppm`, which the schema validator cannot catch because the column
name is unchanged.

---

## Future work

**Resolve the magnitude gap mechanism.** Two candidates remain after
recalibration was eliminated. Needs per-stage PSM lists from a FragPipe run with
intermediate outputs retained.

**Grow the file panel past three.** Unlocks: five tiers instead of two,
absolute rather than file-relative thresholds if they prove stable, and the
per-run ranking confidence flag currently banked as beta.

**Per-run ranking confidence flag.** Designed, using intrinsic inputs only
(apex offset, adduct fraction, satellite fraction, modification complexity), so
no reference run is needed. Three design holes recorded: threshold overfitting
at n=3, biology-versus-artifact discrimination, and no reference dependence.

**Satellite fraction as a quantified metric.** Currently discussed rather than
computed. Would quantify how much of a peak's population is split into
neighbouring bins.

**Byonic Preview benchmark.** Data-gated. The adapter design is settled; the
comparison would complete the four-tool panel with the tool recon is most
directly a successor to.

**Stratified FDR by terminus class.** Would address the flat-q limitation in
Pass 2 and let semi-tryptic rates be reported with proper error control.

**Automated residue-mass degeneracy pass.** Activate if a future file's flanking
check returns Gly-context peptides. The Crystal-C approach is the method
reference.

**Delta-mass on a subset FASTA.** Would test whether the magnitude gap is
candidate-pool reduction. Deliberately not adopted for v0.1.0: a subset selected
from pass-1 confident IDs is biased toward proteins that already matched, which
is exactly the population a new-species or new-tissue user is not asking about.
Compare peak lists, not counts, if it is ever run.

**Non-Windows validation.** Builds are feasible on all three platforms; only
Windows has been tested.

---

## Explicitly rejected — do not revisit

**Prevalence calibration factor.** Scaling recon's percentages to match the
platforms cannot work. On serum the two reference platforms disagree with each
other by 1.37x on the same modification in the same file (PTM-Shepherd 17.92%,
MetaMorpheus 24.51%). A correction factor cannot be more precise than the spread
of its target. Reporting a tier instead sidesteps the problem, because a tier is
ordinal and needs no calibrated magnitude.

**m/z-dependent mass calibration (C1/C2).** Closed as a negative result across
three arms: ppm-constant calibration did not beat the Da scalar; the
three-file carpet that motivated it turned out to be a detector artifact; and
the Option-C ceiling POC — gold-standard MSFragger-calibrated mzML fed straight
into Sage — produced fewer PSMs on all three files and left the one symptomatic
file flat. No gain at the ceiling means no recalibrator should be built.
