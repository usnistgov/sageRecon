Common polymer contaminants in LC-MS/MS proteomics are often recognized by **regular repeating-unit mass differences** and by a small set of common precursor m/z values. For the polymers you named, the hallmark series are PEG at 44.0262 Da repeating units, PPG at 58.0419 Da, and polysiloxanes at 74.0188 Da; PEG/Triton/Tween-family signals often show ethoxylate ladders, while some detergents also show sodium/ammonium adduct variants. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC3714598/)

## Characteristic series

- **PEG / ethoxylates / Tritons / Tweens**: repeating unit \([C_2H_4O]\) with a mass difference of **44.02622 Da**; this is the classic PEG ladder and is also used for PEG-based detergents such as Tritons and Tween buffers. [chromatographyonline](https://www.chromatographyonline.com/view/contaminants-everywhere-tips-and-tricks-reducing-background-signals-when-using-lc-ms)
- **PPG**: repeating unit \([C_3H_6O]\) with a mass difference of **58.04187 Da**. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC3714598/)
- **Polysiloxanes**: repeating unit \([O-Si(CH_3)_2]\) with a mass difference of **74.01879 Da**. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC3714598/)
- **Phthalates**: commonly show strong ions near **391** and **419** with related homologs separated by alkyl-chain increments, rather than a single simple repeat like PEG/PPG. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC3714598/)

## mzsniffer detection

`mzsniffer` is MS1-only: it scans mzML files and **extracts the intensities for predefined polymer precursor masses from MS1 spectra**, then reports each polymer’s share of total ion current. In other words, it does not infer contaminants from MS/MS fragmentation; it matches known contaminant precursor masses in the survey scans within a user-set tolerance. The GitHub page explicitly says it “merely extracts the intensities for common polymer precursors from the MS1 spectra,” and by default it logs %TIC for each matched polymer. [github](https://github.com/wfondrie/mzsniffer)

## Reference masses

Here are some useful reference masses from the contaminant lists and repeat-unit tables:

| Contaminant / motif | Reference m/z or exact mass | Note |
|---|---:|---|
| PEG repeat unit | 44.02622 | Ethoxylate ladder; Tritons/Tweens  [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC3714598/) |
| PPG repeat unit | 58.04187 | Propoxylate ladder  [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC3714598/) |
| Polysiloxane repeat unit | 74.01879 | Silicone contamination  [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC3714598/) |
| PEG mono-isomers in positive mode | 75.02607, 149.04486, 223.06366, 297.08245, 371.10124, 445.12003, 519.13883, 593.15762, 667.17641, 741.19521 | Polysiloxane-related series table; these appear in a standard contaminant reference list  [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC3714598/) |
| PPG-related example | 447.2934 | Listed as PPG in a contaminant brochure  [sigmaaldrich](https://www.sigmaaldrich.com/deepweb/assets/sigmaaldrich/product/documents/210/830/lc-ms-contaminants-brochure-ms.pdf) |
| Triton-related example | 251.20056 | Listed as a Triton-related contaminant entry  [chem.uzh](https://www.chem.uzh.ch/dam/jcr:0abfca1e-40b4-4082-aee3-578634f1a542/Contaminants_+-MS.pdf) |
| PEG-related example | 189.0529 | PEG-related entry in the contaminant sheet; often seen with ethoxylate series  [sigmaaldrich](https://www.sigmaaldrich.com/deepweb/assets/sigmaaldrich/product/documents/210/830/lc-ms-contaminants-brochure-ms.pdf) |
| Phthalate examples | 279.1591, 391.2843, 419.3156 | Common plasticizer contaminants  [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC3714598/) |

## Practical pattern checks

A quick way to distinguish these in proteomics data is to look for repeated peaks separated by one of the hallmark deltas: **44.0262**, **58.0419**, or **74.0188** Da. PEG-like contaminants often appear as a ladder with multiple charge/adduct states, so the same polymer can show several nearby precursor m/z values. For PPG, the same logic applies but with the 58.0419 Da spacing, and polysiloxanes often produce a more rigid 74.0188 Da series. [github](https://github.com/wfondrie/mzsniffer)