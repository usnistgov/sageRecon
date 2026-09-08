The fragment ion intensities in an MGF exported by msconvert are, in general, direct copies of the peak-picked per-spectrum intensity values from the mzML, but precursor intensity in the `PEPMASS=` line is often missing, instrument‑dependent, and not reliably comparable to what you would extract yourself from MS1 scans in mzML. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC4113728/)

Below is how this plays out in practice and what to watch for when porting an intensity‑weighting analysis.

## Fragment ion intensities

- mzML stores each spectrum’s m/z and intensity arrays as binary data (32‑ or 64‑bit float), optionally zlib/Numpress‑compressed, with no semantic constraint beyond “intensity” being an arbitrary instrument unit. [mzmine.github](https://mzmine.github.io/mzmine_documentation/data_conversion.html)
- MGF is just a text peak list; for each MS/MS spectrum, msconvert writes `m/z intensity` lines that represent the peak‑picked fragment list it derived from the underlying data. [fiehnlab.ucdavis](https://fiehnlab.ucdavis.edu/projects/lipidblast/mgf-files)
- The MGF specification does not define whether intensities are peak height vs area or isotope‑collapsed vs monoisotopic; that is entirely determined by the peak‑picking/conversion settings. [mascot.biotech.illinois](https://mascot.biotech.illinois.edu/mascot/help/data_file_help.html)

Implications for porting your analysis:

- If you exported MGF with centroiding or threshold filters in msconvert, the fragment intensities in the MGF already reflect those processing choices (e.g., noise removal, deisotoping). [proteowizard.sourceforge](https://proteowizard.sourceforge.io/tools/filters.html)
- If you now read mzML “directly” and simply sum or read raw profile intensities, you may get systematically different magnitudes or peak lists because you are skipping those same filters. [github](https://github.com/ProteoWizard/pwiz/issues/1301)
- For a like‑for‑like intensity weighting, you should either:
  - Reproduce in your mzML‑based pipeline the same centroiding and filters that msconvert used to produce the MGF, or  
  - Re-run msconvert with known, controlled options (e.g., `--peakPicking`, `--threshold`) and treat that as the reference definition of intensity. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC4113728/)

A concrete example: if your MGF was created with aggressive noise filtering, fragment peaks below a certain intensity never appear, so any spectrum‑level weighting scheme trained on MGF will implicitly assume that low‑intensity tails are absent. That same scheme applied to mzML without filtering will “see” more small peaks and can change relative weights even if high‑intensity peaks match.

## Precursor intensity in `PEPMASS`

- MGF’s `PEPMASS=` field allows an optional intensity value after the precursor m/z, but the format doesn’t say what that number represents or how it is computed. [mascot.biotech.illinois](https://mascot.biotech.illinois.edu/mascot/help/data_file_help.html)
- In practice, many vendor formats do not provide a precursor intensity, and msconvert often cannot fill this reliably; the ProteoWizard developers explicitly note that “not all vendor formats provide that value” and that even when available “it really shouldn’t be relied on for serious analysis.” [sourceforge](https://sourceforge.net/p/proteowizard/mailman/message/34953397/)
- For some vendors (e.g., Bruker BAF), the precursor intensity may simply be missing; msconvert leaves it out, so MGF has no MS1 intensity information beyond the fragment peak list. [sourceforge](https://sourceforge.net/p/proteowizard/mailman/message/34953397/)

Compared with mzML‑based extraction:

- In mzML, you can compute your own precursor intensity from the MS1 spectrum(s): integrate a window around the precursor m/z, perform XIC across scans, handle co‑isolated species, etc. [mzmine.github](https://mzmine.github.io/mzmine_documentation/data_conversion.html)
- The msconvert team explicitly avoided embedding such derived intensities in MGF in a general way, preferring that more complex derivations be done in post‑processing tools rather than in the export step. [sourceforge](https://sourceforge.net/p/proteowizard/mailman/message/34953397/)
- As a result, an intensity‑weighting scheme that used `PEPMASS` intensity from MGF is basing its weights on a vendor‑dependent and often poorly defined value, whereas mzML‑based weighting can be made precise but will not necessarily reproduce those MGF numbers. [sourceforge](https://sourceforge.net/p/proteowizard/mailman/message/58777358/)

If your previous analysis relied on precursor intensity in MGF (e.g., spectrum weight ∝ `PEPMASS` intensity), you should expect discrepancies when you switch to mzML and recompute precursor intensities from MS1. The differences are not just numeric noise; they can be conceptual (peak height vs area, single scan vs XIC, co‑isolation, etc.). [mascot.biotech.illinois](https://mascot.biotech.illinois.edu/mascot/help/data_file_help.html)

## Known approximations and caveats in msconvert MGF export

Several specific issues are worth accounting for:

- **Loss of MS1 detail**: MGF is designed around MS/MS peak lists; many workflows and exports omit MS1 spectra entirely, so any MS1‑based intensity weighting cannot be replicated from a plain MGF. [fiehnlab.ucdavis](https://fiehnlab.ucdavis.edu/projects/lipidblast/mgf-files)
- **Vendor‑dependent precursor intensity**: For some vendors (e.g., certain Sciex/Bruker formats), precursor intensity either isn’t exposed or is exposed in a way that differs from typical MS1 peak extraction; msconvert does not attempt multi‑scan XIC‑style corrections during export. [sourceforge](https://sourceforge.net/p/proteowizard/mailman/message/58777358/)
- **MGF flavor variability**: There is no globally agreed‑upon MGF standard; different tools write slightly different header fields, and may or may not include precursor charge, intensity, or additional annotations. [fiehnlab.ucdavis](https://fiehnlab.ucdavis.edu/projects/lipidblast/mgf-files)
- **Peak‑picking semantics**: Whether msconvert writes profile data, centroided peaks, or filtered peaks depends on filters you set; the MGF format itself doesn’t capture those semantics, so downstream code cannot know whether intensities are area‑integrated or just centroid heights. [proteowizard.sourceforge](https://proteowizard.sourceforge.io/tools/filters.html)

From a porting perspective, the main approximations to consider are:

- Fragment intensities in MGF are already processed; raw mzML intensities are not. You must match processing if you want comparable numbers. [github](https://github.com/ProteoWizard/pwiz/issues/1301)
- Precursor intensities in MGF, when present, are optional and loosely defined; they should not be treated as ground truth when designing intensity‑based models. [sourceforge](https://sourceforge.net/p/proteowizard/mailman/message/58777358/)

## Practical recommendations for your analysis

For an intensity‑weighting analysis you’re porting from MGF to direct mzML:

- Document the exact msconvert settings used to generate the original MGF (centroiding, thresholds, filters), and mirror those steps explicitly in your mzML pipeline before peak list extraction. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC4113728/)
- Treat MGF `PEPMASS` intensities, if you used them, as a legacy approximation; redesign the weighting to compute precursor intensities consistently from MS1 data in mzML (e.g., standardized XIC over ±ppm window, scan aggregation rules). [sourceforge](https://sourceforge.net/p/proteowizard/mailman/message/34953397/)
- Validate by selecting a small dataset, exporting both MGF and mzML with known settings, and comparing per‑peak and per‑spectrum intensity distributions after your new mzML‑based peak extraction to the historical MGF numbers. [mzmine.github](https://mzmine.github.io/mzmine_documentation/data_conversion.html)

If you can share the exact msconvert command line you’ve been using for MGF export, I can help you outline a matching mzML‑based extraction workflow that preserves your intensity scale and weighting behavior as closely as possible.