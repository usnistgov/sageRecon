# Unimod Mass-Shift Decomposition & Ambiguity

Sources: PTM-Shepherd methods (Molecular & Cellular Proteomics, Kong et al.,
mcponline.org), PTMVision (J. Proteome Res., pubs.acs.org), verified July 2026.

## PTM-Shepherd's actual matching tolerance and process
Detected mass-shift peaks are iteratively annotated against Unimod (their
build used a Unimod snapshot retrieved Oct 2, 2019 — Unimod itself has been
updated since, worth using a current pull rather than an old cached copy).
Mass differences within **0.01 Da by default** of a known mass shift are
annotated immediately. If a shift doesn't match within that tolerance, it's
then tested against combinations of user-defined mass shifts and known
annotations, and finally against combinations of two known modifications.
**Each peak is allowed to decompose into at most two modifications** — not
an unbounded combinatorial search. Some rare/protocol-specific entries
(O18 labeling, N15 labeling) were deliberately excluded from their
annotation process because they regularly caused confounded/incorrect
matches — worth considering a similar exclusion list rather than matching
against the full raw Unimod database unfiltered.

## Concrete example of real Unimod ambiguity
A 15.0107 Da mass shift is annotated in Unimod as only one thing: 
conversion of carboxylic acid to hydroxamic acid, with Asp and Glu as the
only listed possible sites. But PTM-Shepherd's actual data showed this
mass shift occurring more frequently on Met than on Asp/Glu — meaning the
single Unimod annotation didn't match the dominant observed chemistry.
This is a real, citable example of why residue-context/enrichment checking
against the Unimod-suggested site matters, rather than trusting the
Unimod name+site pairing blindly once a mass match is found.

## Another real example: unannotatable shifts near known PTMs
PTMVision (analyzing open-search results from a phospho-enriched dataset)
found a 103.0683 Da mass shift sharing many of the same sites as
phosphorylation, but this specific mass could not be matched to any Unimod
modification at all — it gets bucketed as "Ambiguous mass shift" in their
system rather than forced into a wrong Unimod label. Worth building this
same escape hatch into the scouting report: a cluster that's frequent,
has tight RT/spectral behavior, and clearly co-locates with a known PTM's
site pattern, but doesn't cleanly match any Unimod entry, should be
reported as "unannotated, but evidence-backed" rather than either
suppressed or force-matched to the nearest wrong Unimod name.

## Practical takeaway for the scoring engine (Phase 3)
- Use ~0.01 Da (or a ppm-equivalent at your typical precursor mass) as the
  default Unimod match tolerance, matching PTM-Shepherd's default rather
  than inventing a new threshold
- Allow decomposition into at most two combined modifications, not more
- Flag ambiguous matches (2+ Unimod candidates within tolerance) explicitly
  rather than silently picking one
- Keep an explicit "no clean Unimod match" bucket in the report rather than
  forcing a best-guess label — this is a known, published pattern, not an
  edge case being invented here
