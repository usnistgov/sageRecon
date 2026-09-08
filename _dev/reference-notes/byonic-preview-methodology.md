Byonic Preview surveys a shotgun proteomics run before the real search, and emits search parameters rather than results. It is the closest published prior art to what recon does, and the tool recon is described as the open spiritual successor to. This note distils its method for comparison. [pmc.ncbi.nlm.nih.gov](https://pmc.ncbi.nlm.nih.gov/articles/PMC3134881/)

**Source:** Kil YJ, Becker C, Sandoval W, Goldberg D, Bern M. *Preview: A Program for Surveying Shotgun Proteomics Tandem Mass-Spectrometry Data.* Analytical Chemistry 2011 Jun 13; 83(13): 5259–5267. PMC3134881.

**⚠ Read from PMC on 2026-08-24. The PDF is NOT vendored in this repo.** Everything here is a web read and must be re-verified against a repo copy before the step-5 write-up cites it. Do not treat this note as a source in its own right — it is a summary, and a summary is not a source.

***

## The reporting floor is decoy-derived and per-run

This is the finding that matters most to recon, because it contradicts an assumption in our own step-2 design.

Preview gates what it reports with two score thresholds, both anchored to the decoy distribution of the run being surveyed. `THigh = max{ s + 1, 23 }`, where `s` is the highest score reached by any unmodified **decoy** peptide in the initial search. `TLow = max{ t + 1, 15 }`, where `t` is the highest initial-search score of any unmodified decoy of at least 9 residues. The higher threshold governs searches over a large peptide space; the lower one governs smaller searches. The constants 23 and 15 are described as empirically chosen. [pmc.ncbi.nlm.nih.gov](https://pmc.ncbi.nlm.nih.gov/articles/PMC3134881/)

Preview then corrects the modification hit counts themselves by decoy subtraction: target hits minus decoy hits, leaving an estimate of the true target hits. [pmc.ncbi.nlm.nih.gov](https://pmc.ncbi.nlm.nih.gov/articles/PMC3134881/)

**Consequence for recon.** `ptm-stratification-design.md` proposes `floor = X% of the top non-zero delta peak`, sourced to Mascot error-tolerant working practice. Preview does not do this. Its floor is a decoy-derived score threshold plus decoy subtraction of counts — a per-run statistical criterion rather than a ratio to another peak. Recon currently discards decoys at load (`sage_results.rs`), so the mod-discovery histogram never sees them. See NOTES: the ghost hypothesis, and the Preview entry.

***

## Isotope satellites are suppressed upstream, not folded downstream

Preview preprocesses each spectrum by extracting the 300 most intense peaks and downweighting isotope peaks. It states that matching a ¹³C₁ isotope peak to the theoretical monoisotopic peak is rare precisely because of this preprocessing, and that this matters when errors exceed 0.5 Da. [pmc.ncbi.nlm.nih.gov](https://pmc.ncbi.nlm.nih.gov/articles/PMC3134881/)

**Consequence for recon.** This is a third architecture for the isotope-satellite problem, distinct from both options recon has considered. Recon folds isotope satellites in the delta-mass histogram (fold-to-zero, on) or leaves them (satellite folding, disabled by design). Preview does neither: it prevents the satellite from reaching the mass calculation at all. Relevant to the open +57 / +58.02 question.

***

## Integer m/z binning — what it is, and what it is not

Preview converts observed floating-point m/z values to integer bins during spectrum preprocessing, rounding a value `M` to the nearest integer to `0.9995 × M`. The stated purpose is to remove mass defects — the fractional parts of elemental masses. [pmc.ncbi.nlm.nih.gov](https://pmc.ncbi.nlm.nih.gov/articles/PMC3134881/)

**This is not a coarse modification-mass resolution, and it should not be cited as one.** It applies to fragment ions and precursor m/z in preprocessing, not to the modification delta-mass axis. A hypothesis was raised on 2026-08-24 that Preview chose roughly 1 Da resolution for its modification choices in order to manage sub-Da peak splitting in a noisy delta region. The paper does not support that: different axis, different stated purpose. Recorded here so the guess is not carried forward as fact.

***

## Ordered searches, each constraining the next

Preview runs a sequence of searches by category rather than one permissive search, ordering them so that results most likely to affect later searches run first. Mass accuracy is checked first, so that the measured accuracy sets the tolerances used by the subsequent searches. [pmc.ncbi.nlm.nih.gov](https://pmc.ncbi.nlm.nih.gov/articles/PMC3134881/)

**Consequence for recon.** This is convergent with recon's own calibrate-then-recommend design, arrived at independently. Worth citing in the write-up as convergent design, not as borrowed method. Note the difference in cost: Preview runs many small ordered searches; recon's design is locked to one open search plus one Pass 2, with no re-run loop.

***

## Paired target-decoy propagation between passes

When Preview promotes a peptide into the peptide database for a later search, it adds the reversed counterpart alongside it — the reverse being the peptide with all residues except the last in reverse order. Semitryptic peptides scoring above 15 enter the database this way, each with its reverse. [pmc.ncbi.nlm.nih.gov](https://pmc.ncbi.nlm.nih.gov/articles/PMC3134881/)

**Consequence for recon.** This is a second, independent source for the "paired target-decoy selection in `subset_fasta.py`" item queued in PLAN step 3, which until now rested only on Mascot error-tolerant practice. Two published tools carry decoys forward paired, for the same reason: a subset database selected by pass-1 evidence has a biased decoy population unless the pairing is preserved. See `mascot-error-tolerant-methodology.md`.

***

## Scope limits worth carrying into the write-up

Preview evaluates on the order of thirty-odd modifications across seven assay categories, with individual searches restricted to the subset relevant to a sample type. Recon's differentiator — reporting delta-mass peaks with no Unimod match at all — has no counterpart here. `ptm-stratification-design.md` already records this as the capability both commercial tools lack; this note is the source for the Preview half of that claim. [pmc.ncbi.nlm.nih.gov](https://pmc.ncbi.nlm.nih.gov/articles/PMC3134881/)

***

## Open items against this note

- **Vendor the PDF.** Then re-verify every claim above against it and drop this warning.
- Confirm whether the decoy subtraction applies to the reported modification counts a user acts on, or only to internal estimates.
- Confirm whether `THigh` / `TLow` gate modification *reporting* or only database construction. The distinction decides how directly the threshold maps onto recon's tier floor.
