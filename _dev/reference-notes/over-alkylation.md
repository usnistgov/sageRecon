# Over-alkylation & alkylation artefacts

> **NOTE (2026-08-17):** the over-alkylation *annotation feature* this note was
> written to support was **DROPPED** — see NOTES "Default recon = two searches"
> (locked). The tool runs alkylation-agnostic (no fixed C+57), so an abundant
> alkylation surfaces as a delta and IS the recommendation ("fix this in your
> real search"); a fixed-C mode that would need artefact-cluster disambiguation
> is not part of the default. This file is kept as **domain reference** (the
> chemistry is still real and correct), NOT as a build spec.

Distilled reference for mod-discovery annotation and QC flagging. Purpose: let
the annotator recognise alkylation artefact clusters (+57 off-site, −48, +58,
+114, +12) and label them "probable alkylation artefact" rather than dumping
them as UNANNOTATED or forcing a wrong nearest-mass guess. Several of these
masses are **degenerate with real modifications or residue masses** — those are
flagged below and must NOT be auto-labelled as artefacts.

## Summary

Iodine-containing alkylation reagents (iodoacetamide / IAM, iodoacetic acid /
IAC) produce extensive off-site and over-alkylation, especially in in-gel
digests, with large losses of methionine-containing PSMs. Beyond canonical
Carbamidomethyl on Cys, Müller & Winter observed abundant mono- and
di-alkylation at the peptide N-terminus and side chains (Lys, Glu, His, Ser,
Thr, Tyr), plus Met-specific artefacts that dominate the spectra. Off-site CAM
is "the rule rather than the exception" with IAM (Boja & Fales 2001). These
species sit largely outside a fixed-Cys CAM search space, reduce Met peptide
IDs, and inflate search space / quantitative noise if added as many variable
mods. Non-iodine reagents (e.g. acrylamide → Propionamide Cys) give better ID
rates; where iodine must be tolerated, treat off-site CAM/CM and Dethiomethyl(M)
as artefact clusters, not biological PTMs.

## Artefact delta masses

Masses are monoisotopic. **Confirm each against the vendored `unimod.xml`** —
the XML is authoritative; the values below are for orientation and any
disagreement resolves to the XML.

| Label | Unimod ID | Site(s) | Δ mass (mono) | Notes / degeneracy |
| :-- | :-- | :-- | :-- | :-- |
| Carbamidomethyl (Cys) | 4 | C | +57.02146 | Standard IAM/IAA carbamidomethylation; IAC primarily yields carboxymethyl (+58); normally FIXED on Cys, so should NOT appear as a delta peak. If it does → incomplete alkylation or off-site. |
| CAM off-site | 4 | N-term, K, E, H, S, T, Y | +57.02146 | Over-alkylation. **DEGENERATE with Glycine residue mass (57.02146) — do NOT auto-label; flag ambiguous (artefact vs. adds-Gly digestion artefact vs. real off-site CAM). Resolving requires flanking-sequence context — this is the Phase 7D question.** |
| Carboxymethyl (Cys) | (CM) | C | +58.00548 | IAC alkylation; often fixed when IAC used. A +58 peak alongside +57 is a strong second-alkylation signal. |
| CM off-site | (CM) | N-term, K, E, H, S, T, Y | +58.00548 | Over-alkylation from IAC on non-Cys sites. |
| CAM dimer | – | N-term, K, E, H, S, T, Y | +114.04293 | Two CAMs on one peptide/region. **DEGENERATE with GlyGly (Unimod 121, ubiquitin remnant) — a marquee PTM. Do NOT auto-label as CAM dimer; flag ambiguous. Auto-labelling artefact would bury real ubiquitination.** |
| Propionamide (Cys, acrylamide) | 24 | C | +71.03711 | Acrylamide Cys alkylation; best ID rates in Müller & Winter. **DEGENERATE with Alanine residue mass (71.03711) — same artefact-vs-adds-Ala ambiguity as +57/Gly; flag, don't force.** |
| Dethiomethyl (Met side-chain loss) | 526 | M | **−48.00337 (PRECURSOR delta relative to the expected CAM/CM Met)** | In-source, near-quantitative loss of the alkylated Met side chain — the **precursor mass is ~48 lighter**, so it appears as a −48 delta peak in open search (NOT an MS2-only neutral loss). In in-gel IAA data outnumbers Carbamidomethyl(M) ~16×. Dominant cause of Met peptide under-identification with iodine reagents. |
| Thiazolidine (formaldehyde adduct) | 1009 | C (N-term Cys) | +11.99999 (~+12) | Formaldehyde condensation on N-terminal Cys; seen at high levels in some solution IAM data. Confirm exact mass against unimod.xml. |

## Residue-mass / artefact degeneracy family

Several alkylation-artefact masses coincide with amino-acid residue masses,
creating an "artefact vs. add-a-residue digestion artefact vs. real
modification" ambiguity that mass alone cannot resolve. These must be flagged
ambiguous, never auto-labelled, and resolving them requires flanking-sequence
context (Phase 7D scope):

- **+57.0215** — Carbamidomethyl / off-site CAM  ≡  Glycine residue (adds-Gly)
- **+71.0371** — Propionamide (acrylamide Cys)  ≡  Alanine residue (adds-Ala)
- **+114.0429** — CAM dimer  ≡  GlyGly / ubiquitin remnant (Unimod 121)

The correct handling for all three is the same, and it is the project-wide rule
established in Phase 7C: **preserve the peak, suppress/flag the misleading
annotation, mark ambiguous with counts intact — do not force one label and do
not drop the peak.** 7D, if built, should treat this as a *category* (masses
degenerate between artefact and real PTM), not a per-mass special case.

## Cluster summary for annotation

- **Over-alkylation cluster (IAM/IAC):** +57.02146 / +58.00548 on N-term, K, E,
  H, S, T, Y (mono) and ~+114.043 (dimer, degenerate w/ GG). Treat mono off-site
  as non-specific alkylation artefacts; treat +114 as ambiguous.
- **Met alkylation cluster:** +57/+58 on M with dominant −48.00337 precursor
  delta (Dethiomethyl, Unimod 526). Responsible for major Met peptide
  under-identification with iodine reagents.
- **Formaldehyde-related Cys artefact:** +12 (Thiazolidine, Unimod 1009),
  N-terminal Cys, high in some solution IAM data.

## Practical note

For routine workflows, non-iodine reagents (acrylamide → Propionamide,
+71.03711) minimise these artefacts. When iodine reagents must be tolerated,
Dethiomethyl(M) and off-site CAM/CM at N-term/Lys/etc. should be treated as
artefact clusters rather than biological PTMs. Adding all of them as variable
mods in a closed search inflates search space and quant noise — for recon, it's
better to recognise them as artefact clusters at annotation time.

## References

- Müller T. & Winter D. "Systematic Evaluation of Protein Reduction and
  Alkylation Reveals Massive Unspecific Side Effects by Iodine-Containing
  Reagents." Mol Cell Proteomics 2017. PMID 28539326.
  https://pubmed.ncbi.nlm.nih.gov/28539326/
- Cottrell J. "Step away from the iodoacetamide." Matrix Science blog, 2017.
  http://www.matrixscience.com/blog/step-away-from-the-iodoacetamide.html
- Boja E.S. & Fales H.M. "Overalkylation of a protein digest with
  iodoacetamide." Anal Chem 2001. (over-alkylation "the rule rather than the
  exception")
- Unimod entries: Carbamidomethyl (4), Propionamide (24), GlyGly (121),
  Dethiomethyl (526), Thiazolidine (1009).
  http://www.unimod.org/modifications_view.php?editid1=4  (and by ID)

<!-- Vendored during Phase 8 validation (file: over-alkylation.md, ASCII hyphen).
     Corroborates the serum +57 triage conclusion (off-site CAM = "rule not
     exception" with IAM). Corrections applied vs. the source draft: Dethiomethyl
     is a −48.0034 PRECURSOR delta not a −48.0 neutral loss; +114 flagged for the
     GlyGly degeneracy; +71 flagged for the Ala degeneracy; all masses
     subordinated to unimod.xml. -->
