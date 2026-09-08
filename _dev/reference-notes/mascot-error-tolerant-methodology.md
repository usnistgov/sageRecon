Mascot's Error Tolerant (ET) search is a two-pass method. It finds unsuspected modifications, non-specific cleavage, and single-residue sequence variants. Matrix Science holds the method as a trade secret. No patent covers it. The only public description is the vendor help pages, the 2021 workshop deck, and the company blog. This note distills those sources for comparison against recon's open-search approach. [matrixscience](https://www.matrixscience.com/help/error_tolerant_help.html)

***

## Two-pass architecture

Pass 1 is a standard search. It uses fully specific enzyme rules and the parameters the user set on the search form. Pass 1 has a reduced soft limit on the number of variable modifications. ET cannot combine with quantitation, crosslinking, machine-learning rescoring, or error tolerant sequence tags. ET applies to MS/MS data only. It is not possible to do an error tolerant peptide mass fingerprint. [matrixscience](https://www.matrixscience.com/help/error_tolerant_help.html)

Pass 1 selects the database entries for pass 2. Every protein with one or more significant peptide matches goes forward. Significance uses one of two tests. If the user sets no target FDR, the test is an expect value below 0.05. If the user sets a target FDR, Mascot adjusts the pass-1 threshold to hit that FDR. The same test decides which queries go forward: only queries **without** a significant pass-1 match are searched again. [matrixscience](https://www.matrixscience.com/help/error_tolerant_help.html)

Pass 2 searches the selected proteins only. This is the key cost control. The wide search space is never applied to the full database. It is applied to a small, evidence-selected protein subset.

***

## What pass 2 changes

Pass 2 relaxes the search in four ways at once:

- The enzyme becomes semi-specific. Only one peptide terminus must obey the cleavage rule. The missed cleavage limit increases by 1.
- The full Unimod modification list is tested serially. The user can limit this to selected Unimod modification classes. The search form shows the count per class. In the vendor example, two selected classes give 172 candidate modifications.
- All single amino acid substitutions are tested. Substitution masses come from Unimod entries, not from a BLOSUM-style scoring matrix. For nucleic acid databases, single base insertions and deletions are also tested. Insertions and deletions are not tested for protein databases because they cause a frame shift.
- Modifications with a mass delta below the smaller of the precursor and fragment tolerance are rejected. This removes meaningless near-isobaric candidates such as Q->K.

[matrixscience](https://www.matrixscience.com/help/error_tolerant_help.html)

***

## The combinatorial constraint

One rule keeps pass 2 tractable. A peptide can be semi-specific **OR** carry one unsuspected modification **OR** carry one sequence substitution. The three are mutually exclusive. Mascot never combines them on the same peptide. [matrixscience](https://www.matrixscience.com/help/error_tolerant_help.html)

Multiple instances of the *same* ET modification are permitted. The vendor example uses Methyl (DE) on a peptide with four candidate D/E sites and Oxidation (M) as a user variable mod. Pass 2 tries 1, 2, 3, and 4 methylations against 0 or 1 oxidations. Only the combinations that fit the precursor tolerance are scored. Every permutation within configured limits is tested. A modification affecting three serines on a peptide whose mass fits two modifications gives three permutations (110, 101, 011). [matrixscience](https://www.matrixscience.com/help/error_tolerant_help.html)

Matrix Science warns that specifying more than a few variable modifications causes a large loss of discrimination. The permutation count grows geometrically with the fractional abundance of modifiable residues. [matrixscience](https://www.matrixscience.com/help/error_tolerant_help.html)

***

## Disjoint search spaces

Pass 2 skips modification combinations already searched. It also skips fully specific peptides and tests semi-specific peptides only. The two pass search spaces are therefore disjoint by construction. [matrixscience](https://www.matrixscience.com/help/error_tolerant_help.html)

Disjointness is load-bearing for the statistics. It lets Mascot threshold each pass independently and then merge. First, pass-1 results are thresholded to the target FDR. Second, pass-2 results are thresholded at an independent significance level. The union of two disjoint sets at the same FDR has that same FDR. [matrixscience](https://www.matrixscience.com/help/error_tolerant_help.html)

***

## Paired target-decoy selection

The pass-2 decoy database is built in a specific way. Target and decoy proteins are treated as pairs. Any significant pass-1 match — target **or** decoy — pulls both members of the pair into pass 2. [matrixscience](https://www.matrixscience.com/help/error_tolerant_help.html)

This gives two properties. The pass-2 target and decoy databases have identical size. Together they cover all significant pass-1 PSMs. Without the pairing, decoy selection would be biased by pass-1 evidence and the pass-2 FDR estimate would not hold. [matrixscience](https://www.matrixscience.com/pdf/2021WKSHP2.pdf)

A query searched in pass 2 had no match or only a non-significant match in pass 1. Its identity threshold uses the trial count from pass 1 **plus** pass 2. This reflects the combined search space. ET matches therefore face a much stricter threshold than pass-1 matches. [matrixscience](https://www.matrixscience.com/help/error_tolerant_help.html)

Before Mascot Server 2.8, ET could not be combined with target-decoy, and expect values from trial counting were reported for first-pass matches only. Statistical significance for ET matches is a 2.8-era addition. [matrixscience](https://www.matrixscience.com/blog/error-tolerant-searches-now-show-statistical-significance.html)

***

## Reporting behavior

Only pass-1 matches are evidence for protein identity. Matrix Science states this directly. Pass-2 matches are the most likely assignments of otherwise unexplained spectra. They occasionally give useful biology, such as separating two isoforms. They are not protein-level evidence. [matrixscience](http://www.matrixscience.com/help/error_tolerant_example.html)

Mascot reports ET results by Unimod **name and site**, not by mass. One true mass delta is split across many site rows. Multiple candidate matches for one query are common, and the top-scoring match is often a less credible chemistry than a slightly lower-scoring alternative. The vendor example shows a succinylation delta scoring equal to a double-carbamidomethylation explanation of the same query. [matrixscience](http://www.matrixscience.com/help/error_tolerant_example.html)

***

## Relevance to recon

The comparison points that matter for this project:

- **Named-list versus unconstrained.** Mascot ET enumerates named Unimod deltas and substitutions. It tests a controlled vocabulary. It cannot report a mass that Unimod does not contain. Recon's open search has no such ceiling — it reports the observed delta whether or not a name exists for it. This is recon's main capability advantage and the reason the two tools disagree on the long tail.
- **Restricted database is the cost control.** Mascot buys its wide search space by shrinking the database first. Recon buys it by keeping the search space wide but the pass count low. Both are two-stage. The stages divide the work differently.
- **Mutual exclusivity is a shared constraint.** One modification, or one substitution, or semi-specificity — never combined. Byonic's Wildcard search reaches the same one-per-peptide limit independently. Two vendors converged on this. That is evidence the constraint is about tractability, not about either implementation.
- **Population definition differs by design.** Mascot ET reports a resolved-ET fraction, not a PSM count. Pass-1 and pass-2 populations are disjoint and separately thresholded. Any comparison against recon's percentages must state the currency, as the existing Mascot adapter already does.
- **The multi-site roll-up requirement follows from the name+site output.** See NOTES.md, "Mascot error-tolerant adapter" (2026-07-24). The adapter must sum site rows sharing one mass before aligning to recon's mass axis.

***

## Source status

All statements above come from vendor documentation. No peer-reviewed methods paper describes the ET algorithm. The primary Mascot scoring paper (Perkins, Pappin, Creasy, Cottrell, *Electrophoresis* 1999) predates ET and does not cover it. Treat implementation details here as vendor claims, not as independently verified behavior.
