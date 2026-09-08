# Unimod Classification Filtering for Biological Samples

**Source:** Perplexity AI response, 2026-07-07  
**Context:** Guidance on which Unimod modification classifications to include/exclude when annotating delta masses from biological samples.

---

## Summary

For proteomics delta-mass annotation focused on biological samples, you typically **exclude** Unimod modifications classified as chemical labels, tags, or other explicitly artificial/synthetic processes (e.g. SILAC, TMT, iTRAQ), and retain classes that correspond to natural PTMs or routine sample-prep chemistry (e.g. oxidation, carbamidomethylation, enzymatic PTMs).

## Unimod Classification Concept

Unimod is designed to capture mass differences for "all types of natural and artificial modifications" without itself enforcing a biological vs artefactual distinction. Each modification has a "Classification" field chosen from a controlled vocabulary specifically intended to group modifications by chemical/biological category.

Because Unimod's goal is mass spectrometry support rather than biological curation, downstream users decide which classes to treat as biological vs synthetic when interpreting experimental data.

**Reference:** https://www.unimod.org/unimod_help.html

---

## Classes Typically Treated as Biological/Natural

These classification types are generally **kept** when annotating delta masses from biological samples (PTM discovery, proteoform characterization, etc.), acknowledging that some may also arise during sample handling:

- **Post-translational modification**
- **Glycosylation**
- **Lipidation**
- **Phosphorylation**
- **Acetylation, Methylation, Ubiquitination, Sumoylation** (often grouped under PTM-related classes)
- **Proteolytic processing, Signal peptide removal** (when present)
- **Disulfide formation, Cross-linking** (endogenous)
- **Oxidation and reduction** (can be biological or artefactual but usually not considered "synthetic labels")
- **Deamidation, Isomerization, Cyclization** (e.g. pyro-Glu)
- **Non-enzymatic glycation** (e.g. Amadori products)

These all represent naturally occurring PTMs or plausible in vivo or ex vivo chemistry intrinsic to the biological sample rather than deliberate synthetic labels.

### Sample Preparation Chemistry (Usually Retained)

For routine proteomics workflows, you also usually retain:

- **Alkylation classes** such as "Alkylation" and specific entries like Carbamidomethyl (iodoacetamide), Iodoacetic acid derivatives etc., because they are systematic and necessary to interpret Cys-containing peptides, even though they are artificial.
- **Enzymatic digestion artefacts** (missed cleavages, N-terminal cyclization, etc.) where captured as classes, since they reflect realistic peptide chemistry in the pipeline.

---

## Classes Typically Excluded as Synthetic/Artificial Labels

When annotating unknown delta masses and restricting to "biological" chemistry, the convention is to **exclude** modifications whose Unimod classification clearly reflects deliberate synthetic labeling, reporter tags, or instrument-specific artifacts:

### Isotopic Label / Stable Isotope Label
- Includes SILAC-type metabolic labels and heavy isotope coding (e.g. ¹³C, ¹⁵N, ¹⁸O)
- These represent designed labels, not intrinsic biology
- You already know if you used SILAC or heavy labeling, so they're not part of de novo biological mass-annotation

### Isobaric Tag / Reporter Ion Tag
- TMT, iTRAQ, DiLeu, ICAT, etc., often classified under "Isobaric label", "Reporter tag", or similar
- These introduce large, defined mass shifts on N-termini or Lys residues
- Strictly synthetic reagents for quantitation

### Chemical Tag / Derivatisation / Affinity Tag
- Biotin tags, fluorescent tags (dansyl, fluorescein), click-chemistry tags
- Other affinity/fluorescent labels classified as "Derivatisation", "Affinity label", or "Chemical tag"
- These do not occur in vivo; their presence indicates a specific experimental strategy

### Cross-linking Agents (Synthetic)
- BS3, DSS, EDC, and other cross-linking reagents used in XL-MS
- Usually given dedicated classification reflecting "Cross-linker" or "Chemical cross-link"
- While cross-links can be biological, these reagent-specific modifications are treated as synthetic unless explicitly analyzing XL-MS data

### Artifact / Instrument Artifact / Fragmentation-specific Labels
- Certain instrument- or fragmentation-induced artefacts may have Unimod classes indicating "Artifact" or "Fragmentation product"
- Usually not used as biological explanations for unknown precursor delta masses

### Protection Group / Blocking Group
- Modifications representing synthetic protecting groups from chemical synthesis (e.g. Boc, Fmoc)
- Should be excluded from biological sample interpretation

---

## Practical Inclusion/Exclusion Table

| Conceptual Class | Examples | Include for Biological Annotation? |
|------------------|----------|-----------------------------------|
| Post-translational modification / PTM | Phosphorylation, Glycosylation, Acetylation | **Yes** |
| Glycosylation | N-linked, O-linked glycans | **Yes** |
| Lipidation | Palmitoylation, Myristoylation | **Yes** |
| Oxidation / Reduction | Oxidation (M), Dioxidation, reduction | **Yes** (often artefactual but not synthetic) |
| Deamidation / Cyclization / Isomerization | Deamidated N/Q, Pyro-glu | **Yes** |
| Non-enzymatic glycation | Carboxymethyl-lysine, Amadori products | **Yes** |
| Sample-prep alkylation / Derivatisation (Cys) | Carbamidomethyl, Iodoacetamide | **Usually yes** (known pipeline chemistry) |
| Proteolytic processing / terminal processing | N-term acetyl, signal peptide removal | **Yes** |
| Isotopic / stable isotope label | SILAC labels, 13C/15N labeling | **No** (synthetic label) |
| Isobaric tag / reporter tag / quantitative label | TMT, iTRAQ, DiLeu | **No** (synthetic quantitation tags) |
| Chemical tag / affinity label / fluorescent tag | Biotin tag, Dansyl, Fluorescein | **No** |
| Synthetic cross-linker | BS3, DSS, EDC cross-links | **No** (unless XL-MS analysis) |
| Protection / blocking group | Boc, Fmoc | **No** |
| Instrument artifact / fragmentation artifact | Reporter-specific artefacts | **No** |

---

## Implementation Notes

In practice, implement this by:

1. Parsing the Unimod "Classification" field for each candidate modification
2. Keeping modifications whose classification contains PTM/biological/sample-prep terms
3. Discarding those whose classification clearly indicates labels, tags, or synthetic reagents

Because exact classification names in Unimod can vary and evolve, the usual practice is to treat broad families as biological vs synthetic labels, rather than rely on a single string.

---

## Specific Examples

| Modification | Unimod Classification | Decision |
|--------------|----------------------|----------|
| SILAC Label:13C(6) on Lys/Arg | Isotopic label / stable isotope label | **Exclude** |
| TMT6plex | Isobaric tag / reporter tag / chemical label | **Exclude** |
| iTRAQ4plex | Isobaric tag / reporter tag | **Exclude** |
| Oxidation (M) | Post-translational / Artefact | **Include** |
| Carbamidomethyl (C) | Chemical derivative | **Include** (known sample prep) |
| Phospho (STY) | Post-translational | **Include** |
| Deamidated (NQ) | Artefact | **Include** |

---

## Application to This Project

For Phase 3 mod discovery, the current implementation stores `excluded_classifications` in the output but does not yet filter by it. Phase 8 (Validation Pass) should implement:

1. Parse classification from Unimod XML (already captured in `ModSpecificity.classification`)
2. Filter matches based on a configurable exclusion list
3. Default exclusions: `["Isotopic label", "O18 label", "Isobaric label", "Reporter tag"]`
