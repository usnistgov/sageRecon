# Mass accuracy (mass error) — reference

General domain reference: what mass accuracy is, how it's expressed, what
governs it, and why it is NOT comparable across instruments or runs. This is
curated background, not observations from any particular data file (session
observations live in `JOURNAL.md`; the recon tool's own metric lives in
`qc.rs`).

## What it is

**Mass accuracy** = how close a measured m/z is to the true (theoretical) value.
Expressed two ways:

- **Absolute (Da / mDa / mmu):** `measured − theoretical`. Used at low resolution
  where error is roughly constant across the m/z range (e.g. ion traps).
- **Relative (ppm):** `(measured − theoretical) / theoretical × 1e6`. Used at
  high resolution because the error scales with m/z — a fixed ppm is a larger
  absolute error at higher mass. **ppm is m/z-dependent by construction**; a
  single scalar offset in Da cannot correct a ppm-proportional error (this is
  why scalar recalibration only partially works — see the recon tool's deferred
  m/z-dependent calibration).

**Signed vs. absolute:** the *sign* of the error carries information the
absolute value hides. A distribution centered on 0 ppm is well-calibrated; one
centered on +3 ppm is *precise but biased* (systematic calibration offset). Report
signed median (bias) separately from absolute median (headline accuracy) and
spread (precision) — an "accurate" absolute number can conceal a real bias.

**Precursor (MS1) vs. fragment (MS2) accuracy** are distinct metrics and can
differ within one run. In an open (wide-precursor) search, precursor mass error
is dominated by the *delta mass* (the modification being searched for), NOT
instrument calibration — so open-search precursor error is meaningless as an
accuracy metric. Fragment error, matched at tight tolerance regardless of
precursor delta, reflects true calibration; and precursor error from a *closed*
(narrow) search, where the delta is ~0 for correct IDs, is real MS1 accuracy.

## What governs it (and why cross-run comparison is invalid)

Mass accuracy depends on factors specific to the instrument and the individual
acquisition — **two runs' numbers are not comparable unless you know these match:**

- **Detector / analyzer class** sets the order of magnitude:
  - Linear ion trap: ~0.1–1 Da (low-res; unit-resolution nominal mass).
  - Orbitrap: typically < 10 ppm well-calibrated (often 1–5 ppm).
  - TOF: ~ 5–60 ppm depending on generation/calibration (older/simpler ~30–60).
  - FT-ICR: sub-ppm achievable.
  These are ballparks; a given instrument's spec and tune state dominate.
- **Calibration recency / drift.** Mass calibration drifts with time and
  environment. A well-behaved Orbitrap may hold < 10 ppm for a week and stay
  under ~15 ppm even after a month uncalibrated; other instruments drift faster.
  So the *same instrument* can report different accuracy on different dates.
- **Space-charge / overloading.** Injecting too many ions (or too many of one
  dominant species) into a trapping analyzer (e.g. Orbitrap) causes ion–ion
  interactions — **peak coalescence and m/z splitting** — producing mass errors
  *far* larger than the calibrated spec, even at a nominally acceptable total ion
  count if the population is dominated by one species. High mass error on a
  normally-accurate instrument can therefore be a *loading* symptom, not a
  calibration one.

**Consequence for any tool that reports mass accuracy:** the number describes
one acquisition on one instrument in one state. Do NOT compare mass accuracy
across files of unknown provenance, and do NOT grade a file's accuracy against
another's — report the number, note it's instrument/calibration/loading-
dependent, and let the analyst interpret it within that run. (This is the same
"present, don't judge" discipline as the digestion-score decision.)

## How it's determined in practice

- From confidently identified PSMs, take the near-zero-delta population
  (correct, unmodified IDs) and measure `(observed − theoretical)/theoretical`
  per PSM; report signed median (bias), absolute median (accuracy), and
  spread/MAD (precision).
- Search engines (e.g. MSFragger/FragPipe) perform a calibration pass that
  measures pre-calibration ("Old") error and applies a correction ("New") that
  drives the median toward zero — the "New" MS1/MS2 medians are the residual
  after their recalibration. An m/z-dependent recalibration tightens more than a
  scalar offset because it addresses the ppm-proportional term.

## References / see also

- `reference-notes/domain-primer.md` — delta mass, open search context.
- `qc.rs` — the recon tool's MS1 mass-accuracy metric (signed, from a closed
  reference search) and the fragment_ppm QC figure.
- NOTES.md "Deferred enhancements" — m/z-dependent calibration (why a scalar
  apex_offset can't fix ppm-proportional drift).
