# Reporting Mass Error in PPM and Da for MS1 and MS2

## Overview
Mass error in mass spectrometry is most commonly reported in parts per million (ppm), especially for high-resolution MS1 and MS2 data, because ppm normalizes the error to the observed ion m/z and makes values comparable across the mass range.[cite:7][cite:10] A Dalton (Da) error can also be reported, but it is only meaningful when paired with a specific reference m/z because the Da equivalent changes linearly with m/z for a fixed ppm value.[cite:7][cite:10]

## Core relationship
The conversion between ppm and Da follows these equations:

\[
\text{error (ppm)} = \frac{\text{error (Da)} \times 10^{6}}{\text{m/z}}
\]

\[
\text{error (Da)} = \text{error (ppm)} \times \frac{\text{m/z}}{10^{6}}
\]

These equations show that a fixed ppm tolerance corresponds to a wider absolute Da window at higher m/z, while a fixed Da tolerance corresponds to a larger ppm error at lower m/z.[cite:7][cite:10]

## Choosing a reference m/z
There is no single correct m/z at which a Da error must be reported.[cite:7][cite:10] Using m/z 500 is not inherently more right or wrong than using m/z 200; it is simply a reporting choice, provided the ppm value is given and the reference m/z is stated explicitly.[cite:7][cite:10]

For proteomics datasets, a representative precursor m/z such as the median or typical precursor m/z is often more informative than a generic instrument-specification point.[cite:7] For example, if precursor ions in a dataset cluster around m/z 450 to 650, quoting an equivalent Da error at m/z 500 gives readers an intuitive scale that better matches the data than m/z 200.[cite:7][cite:10]

## Why resolution is often quoted at m/z 200
Instrument vendors commonly specify resolving power at m/z 200, particularly for Orbitrap instruments and related high-resolution platforms.[cite:9][cite:15] Resolution is defined as \(R = (m/z)/\Delta m\), where \(\Delta m\) is the peak width measured under a defined convention such as full width at half maximum.[cite:4][cite:15]

The use of m/z 200 is a convention for instrument specification and comparison, not a rule for mass error reporting.[cite:9][cite:15] A method can therefore report resolution at m/z 200 while separately expressing mass accuracy in ppm and, when useful, giving the corresponding Da value at m/z 500 or another representative m/z.[cite:9][cite:10][cite:15]

## Practical examples
If MS1 mass accuracy is 2 ppm, the equivalent absolute error depends on the reference m/z:[cite:10]

| Reference m/z | Equivalent Da error at 2 ppm |
|---|---|
| 200 | 0.0004 Da [cite:10] |
| 500 | 0.0010 Da [cite:10] |
| 800 | 0.0016 Da [cite:10] |

Likewise, 0.02 Da in MS2 corresponds to very different ppm values depending on fragment m/z: 100 ppm at m/z 200 and 20 ppm at m/z 1000.[cite:10] This is one reason ppm is often the cleaner primary metric for high-resolution fragment data, while Da is more common for lower-resolution fragment matching or for explicitly defined search tolerances.[cite:7][cite:10]

## Recommended wording
For method documentation, the clearest approach is to report ppm as the primary mass accuracy metric and treat Da as a contextual translation at a stated m/z.[cite:7][cite:10] Suitable wording includes the following:

- "Mass accuracy is reported in ppm; equivalent absolute errors in Da are given at a stated reference m/z for interpretability."[cite:7][cite:10]
- "For reference, an error of 2 ppm corresponds to 0.0010 Da at m/z 500."[cite:10]
- "Instrument resolution is reported at m/z 200 according to vendor convention, whereas Da error examples are given at a representative precursor m/z."[cite:9][cite:15]

## Guidance for MS1 and MS2
For MS1, reporting median or mean precursor mass error in ppm is generally the most portable and interpretable choice across instruments and datasets.[cite:7][cite:10] When a Da value is included, it should be described as the equivalent error at a representative precursor m/z, such as m/z 500.[cite:7][cite:10]

For MS2, the same logic applies if fragment accuracy is being discussed as a performance metric.[cite:7][cite:10] If fragment matching is instead controlled by a fixed Da tolerance in the search engine or processing pipeline, it is helpful to note the implied ppm range across the fragment m/z values of interest.[cite:7][cite:10]
