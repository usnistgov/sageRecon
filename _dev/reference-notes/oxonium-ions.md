In HCD glycopeptide MS2 spectra, the most common diagnostic oxonium ions are HexNAc at **m/z 204.0867**, Hex-HexNAc at **m/z 366.1395**, NeuAc at **m/z 292.1027**, NeuAc-H2O at **m/z 274.0921**, HexNAc-H2O at **m/z 186.0761**, HexNAc-2H2O at **m/z 168.0655**, Hexose at **m/z 163.0601**, and the HexNAc-related ion at **m/z 138.0545**; some workflows also track HexNAcHexNAc-related internal fragments around **m/z 495** only in broader glycan profiling, but 204 and 366 are the core screening ions. A practical screening rule used in published glycoproteomics software is **at least 2 oxonium ions among the top 5–10% of peaks**, with **m/z 204 mandatory** in one common GPQuest-based workflow. [sciencedirect](https://www.sciencedirect.com/science/article/pii/S1535947625001847)

## Common ions

| Ion | Exact m/z | Typical interpretation |
|---|---:|---|
| HexNAc | 204.0867 | Universal glycopeptide marker.  [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC10308332/) |
| Hex-HexNAc | 366.1395 | Common disaccharide oxonium ion.  [tandfonline](https://www.tandfonline.com/doi/full/10.1080/19420862.2018.1494106) |
| NeuAc | 292.1027 | Sialylation marker.  [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC10308332/) |
| NeuAc-H2O | 274.0921 | Sialylation marker, often strong in acidic glycans.  [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC10308332/) |
| HexNAc-H2O | 186.0761 | Helpful for O-glycopeptides and glycan typing.  [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC10308332/) |
| HexNAc-2H2O | 168.0655 | Supporting HexNAc fragment.  [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC10308332/) |
| Hexose | 163.0601 | More composition-dependent, less specific.  [tandfonline](https://www.tandfonline.com/doi/full/10.1080/19420862.2018.1494106) |
| HexNAc-related internal fragment | 138.0545 | Often observed, but not as universal as 204.  [nature](https://www.nature.com/articles/srep37189) |

## Screening thresholds

Published screening rules are usually intensity-based rather than absolute m/z cutoffs. One study required **at least two oxonium ions in the top 5% of peaks, with 204 mandatory** for glycopeptide classification, and another used **at least two oxonium ions in the top 10% of peaks** to qualify an MS/MS spectrum for glycopeptide search.  In a later glycoproteomics workflow, practical absolute thresholds were also used for triggering: **5 × 10^3** for NeuAc-H2O and **1 × 10^4** for HexNAc, Hex-HexNAc, and NeuAcHexHexNAc ions. [nature](https://www.nature.com/articles/srep37189)

## Most diagnostic

The **most diagnostic** single ion is **HexNAc at m/z 204.0867**, because it is widely treated as the universal glycopeptide marker and is often required in screening logic.  The next most informative ions are **366.1395** for Hex-HexNAc and the sialic-acid ions **292.1027/274.0921**, because they strongly support glycan-containing spectra and help distinguish sialylated species. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC10308332/)

## More ambiguous ions

The more **ambiguous** ions are **163.0601** (Hex) and **138.0545**, because they can occur in glycan-rich contexts but are less specific to glycopeptides than 204 or 366.  Likewise, **186.0761** and **168.0655** are useful supporting ions, but their diagnostic value is strongest when interpreted together with 204 and 366 rather than alone. [tandfonline](https://www.tandfonline.com/doi/full/10.1080/19420862.2018.1494106)

## Practical ranking

For routine HCD screening, a reasonable hierarchy is: **204 > 366 > 292/274 > 186/168 > 163/138**. That ranking reflects both how commonly the ions appear in glycopeptide HCD spectra and how often tools use them as explicit markers for glycopeptide detection or N/O-type classification. [sciencedirect](https://www.sciencedirect.com/science/article/pii/S1535947625001847)