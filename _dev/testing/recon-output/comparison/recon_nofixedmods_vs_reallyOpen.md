# Mod-discovery cross-comparison: Sage-Recon vs PTM-Shepherd (reallyOpen)

OBJECTIVE tool-vs-tool benchmark (NOT a gate; NOT a comparison to the tool's author). Sage-Recon is the reference column. Divergence is expected — see the methodology deltas in `testing/reference-data/ptm-shepherd/README.md` (recalibrated two-stage search, per-file instruments, different FDR, wider window, higher peak floor, speed). Percentages are the currency; PSM totals differ by FDR.

Match tolerance 0.015 Da; shared window [-100.0, 500.0] Da (Sage delta range −100..+500, PTM-Shepherd −150..+500; true overlap −100..+500).

### bcell  (Sage-Recon vs PTM-Shepherd (reallyOpen))

- Matched (both, within 0.015 Da): **24**
- Sage-Recon only: **23**
- PTM-Shepherd (reallyOpen) only: **93**
- Spearman ρ (% matched): **0.402** (p=0.0517, n=24)
- Top-10 overlap (by % in matched set): **3/10**
- _Window coverage check: Sage-Recon peaks >+100 Da in window: 4 (Sage total >+100: 4, other tool total >+100: 22)_

| mass (Da) | Sage-Recon label | Recon % | Recon rank | PTM-Shepherd (reallyOpen) label | PTM-Shepherd (reallyOpen) % | PTM-Shepherd (reallyOpen) rank |
|---|---|---|---|---|---|---|
| +0.0000 | Unmodified | 58.29 | 1 | None | 63.03 | 1 |
| +57.0220 | Carbamidomethyl | 4.50 | 2 | Iodoacetamide derivative/Addition of Glycine/Addition of G | 10.68 | 2 |
| +15.9953 | Oxidation | 1.92 | 3 | Oxidation or Hydroxylation | 5.53 | 3 |
| +58.0243 | UNANNOTATED | 1.28 | 4 | 2,3-dihydro-2,2-dimethyl-7-benzofuranol N-methyl carbamate | 0.41 | 10 |
| +0.9821 | Deamidated | 1.03 | 5 | Deamidation | 1.25 | 6 |
| -1.0290 | Lys-&gt;Allysine | 0.51 | 6 | Lysine oxidation to aminoadipic semialdehyde | 0.17 | 16 |
| +52.9113 | Cation:Fe[III] | 0.49 | 7 | Replacement of 3 protons by iron | 0.53 | 9 |
| -0.9804 | Glu-&gt;pyro-Glu+Methyl:2H(2)13C(1) | 0.41 | 8 | Pyro-Glu from E + Methylation Medium | 0.01 | 24 |
| +59.0280 | AEC-MAEC | 0.40 | 9 | Propionate labeling reagent heavy form (+3amu), N-term  K | 0.20 | 15 |
| -17.0257 | Gln-&gt;pyro-Glu | 0.34 | 10 | Pyro-glu from Q/Loss of ammonia | 0.82 | 7 |
| -89.0289 | Met-loss+Acetyl | 0.28 | 11 | Removal of initiator methionine from protein N-terminus, then acetylation of the new N-terminus | 0.03 | 22 |
| +1.9196 | UNANNOTATED | 0.28 | 12 | Unannotated mass-shift 1.9136 | 0.13 | 18 |
| +114.0432 | GG | 0.28 | 13 | ubiquitinylation residue/Double Carbamidomethylation/Addition of N | 1.27 | 5 |
| +53.9139 | Cation:Fe[II] | 0.25 | 14 | Replacement of 2 protons by iron | 0.03 | 23 |
| +31.9901 | Dioxidation | 0.21 | 15 | dihydroxy | 0.62 | 8 |
| +2.0499 | UNANNOTATED | 0.21 | 16 | Unannotated mass-shift 2.0428 | 0.06 | 20 |
| +18.0012 | Pro-&gt;HAVA | 0.20 | 17 | Proline oxidation to 5-hydroxy-2-aminovaleric acid | 0.06 | 21 |
| +17.0257 | Ammonium | 0.19 | 18 | deuterated methyl ester | 0.39 | 11 |
| +42.0109 | Acetyl | 0.16 | 19 | Acetylation | 1.27 | 4 |
| +270.1099 | UNANNOTATED | 0.14 | 20 | Unannotated mass-shift 270.1104 | 0.23 | 13 |
| +79.9661 | Phospho | 0.11 | 21 | Phosphorylation | 0.21 | 14 |
| +78.0131 | UNANNOTATED | 0.11 | 22 | Unannotated mass-shift 78.0138 | 0.12 | 19 |
| -18.0099 | Dehydrated | 0.11 | 23 | Dehydration/Pyro-glu from E | 0.35 | 12 |
| +43.0082 | Carbamyl | 0.10 | 24 | Carbamylation | 0.14 | 17 |

**Sage-Recon only (in window, no match):**
- +16.9975  UNANNOTATED  (0.53%, 389 PSMs)
- +0.9509  UNANNOTATED  (0.52%, 383 PSMs)
- +1.9691  UNANNOTATED  (0.40%, 296 PSMs)
- +0.9303  UNANNOTATED  (0.37%, 272 PSMs)
- +1.9855  UNANNOTATED  (0.33%, 241 PSMs)
- -0.0788  Unmodified  (0.31%, 226 PSMs)
- +0.9093  UNANNOTATED  (0.28%, 209 PSMs)
- -1.0601  UNANNOTATED  (0.28%, 204 PSMs)
- -0.9599  UNANNOTATED  (0.27%, 201 PSMs)
- +73.0181  UNANNOTATED  (0.25%, 181 PSMs)
- +1.9028  UNANNOTATED  (0.23%, 166 PSMs)
- -0.0995  Unmodified  (0.22%, 164 PSMs)
- +0.8899  UNANNOTATED  (0.21%, 152 PSMs)
- +2.0305  UNANNOTATED  (0.20%, 148 PSMs)
- -1.0871  UNANNOTATED  (0.20%, 147 PSMs)
- +1.8798  UNANNOTATED  (0.16%, 117 PSMs)
- -1.1068  UNANNOTATED  (0.15%, 111 PSMs)
- +18.0302  UNANNOTATED  (0.15%, 107 PSMs)
- +115.0481  UNANNOTATED  (0.13%, 99 PSMs)
- -0.1276  UNANNOTATED  (0.13%, 96 PSMs)

**PTM-Shepherd (reallyOpen) only (in window, no match):**
- +1.0024  First isotopic peak  (3.37%, 2174 PSMs)
- +0.0186  Unannotated mass-shift 0.0186  (0.79%, 511 PSMs)
- +2.0047  Second isotopic peak  (0.61%, 395 PSMs)
- -1.0024  Isotopic peak error  (0.41%, 261 PSMs)
- -2.0156  2-amino-3-oxo-butanoic_acid  (0.36%, 232 PSMs)
- +39.9949  S-carbamoylmethylcysteine cyclization (N-terminus)/Glyoxal-derived hydroimiadazolone  (0.34%, 218 PSMs)
- +3.0071  Third isotopic peak  (0.32%, 206 PSMs)
- +58.0055  Iodoacetic acid derivative  (0.23%, 145 PSMs)
- +14.0157  Methylation  (0.19%, 122 PSMs)
- +28.0313  di-Methylation/Acetaldehyde +28/Ethylation  (0.13%, 81 PSMs)
- +74.0559  Acrylamide d3  (0.11%, 72 PSMs)
- +13.9793  proline oxidation to pyroglutamic acid/Tryptophan oxidation to oxolactone/aldehyde and ketone modifications  (0.10%, 66 PSMs)
- -0.9840  Amidation  (0.08%, 54 PSMs)
- +300.1576  Unannotated mass-shift 300.1576  (0.06%, 41 PSMs)
- +72.0211  carboxyethyl/Dihydroxy methylglyoxal adduct/Ethoxyformylation  (0.06%, 38 PSMs)
- +386.0456  Unannotated mass-shift 386.0456  (0.06%, 36 PSMs)
- +55.9898  DST crosslinker cleaved by sodium periodate  (0.05%, 34 PSMs)
- +3.9949  tryptophan oxidation to kynurenin  (0.05%, 34 PSMs)
- +203.0794  N-Acetylhexosamine  (0.04%, 28 PSMs)
- +89.0113  Photo-induced Glycine Adduct  (0.04%, 27 PSMs)


### serum  (Sage-Recon vs PTM-Shepherd (reallyOpen))

- Matched (both, within 0.015 Da): **31**
- Sage-Recon only: **17**
- PTM-Shepherd (reallyOpen) only: **78**
- Spearman ρ (% matched): **0.798** (p=7.6e-08, n=31)
- Top-10 overlap (by % in matched set): **2/10**
- _Window coverage check: Sage-Recon peaks >+100 Da in window: 11 (Sage total >+100: 11, other tool total >+100: 14)_

| mass (Da) | Sage-Recon label | Recon % | Recon rank | PTM-Shepherd (reallyOpen) label | PTM-Shepherd (reallyOpen) % | PTM-Shepherd (reallyOpen) rank |
|---|---|---|---|---|---|---|
| +0.0000 | Unmodified | 32.24 | 1 | None | 31.28 | 1 |
| +57.0237 | Carbamidomethyl | 7.31 | 2 | Iodoacetamide derivative/Addition of Glycine/Addition of G | 17.92 | 2 |
| +58.0260 | UNANNOTATED | 2.27 | 3 | 2,3-dihydro-2,2-dimethyl-7-benzofuranol N-methyl carbamate | 1.30 | 9 |
| +52.9130 | Cation:Fe[III] | 1.90 | 4 | Replacement of 3 protons by iron | 2.90 | 5 |
| +15.9951 | Oxidation | 1.85 | 5 | Oxidation or Hydroxylation | 5.28 | 3 |
| +0.9845 | Deamidated | 1.20 | 6 | Deamidation | 1.41 | 8 |
| +31.9907 | Dioxidation | 1.17 | 7 | dihydroxy | 1.82 | 7 |
| +209.0205 | CarbamidomethylDTT | 0.90 | 8 | Carbamidomethylated DTT modification of cysteine | 2.09 | 6 |
| +114.0457 | GG | 0.86 | 9 | ubiquitinylation residue/Double Carbamidomethylation/Addition of N | 3.27 | 4 |
| +23.9593 | Cation:Al[III] | 0.80 | 10 | Replacement of 3 protons by aluminium | 0.88 | 13 |
| +58.0128 | Carboxymethyl | 0.60 | 11 | Iodoacetic acid derivative | 0.85 | 14 |
| -18.0101 | Dehydrated | 0.51 | 12 | Dehydration/Pyro-glu from E | 1.28 | 10 |
| +59.0276 | AEC-MAEC | 0.47 | 13 | aminoethylcysteine | 0.10 | 29 |
| +13.9783 | Pro-&gt;pyro-Glu | 0.45 | 14 | proline oxidation to pyroglutamic acid/Tryptophan oxidation to oxolactone/aldehyde and ketone modifications | 1.10 | 12 |
| -17.0285 | Gln-&gt;pyro-Glu | 0.38 | 15 | Pyro-glu from Q/Loss of ammonia | 1.20 | 11 |
| +53.9156 | Cation:Fe[II] | 0.38 | 16 | Replacement of 2 protons by iron | 0.24 | 25 |
| +162.0542 | UNANNOTATED | 0.38 | 17 | Hexose | 0.64 | 17 |
| +80.9812 | Arg-&gt;Npo | 0.35 | 18 | Arginine replacement by Nitropyrimidyl ornithine | 0.53 | 18 |
| -0.9786 | Glu-&gt;pyro-Glu+Methyl:2H(2)13C(1) | 0.34 | 19 | Pyro-Glu from E + Methylation Medium | 0.03 | 31 |
| +14.0149 | Methyl | 0.34 | 20 | Methylation | 0.71 | 16 |
| +89.0121 | Gly+O(2) | 0.32 | 21 | Photo-induced Glycine Adduct | 0.44 | 19 |
| -1.0296 | Lys-&gt;Allysine | 0.29 | 22 | Lysine oxidation to aminoadipic semialdehyde | 0.07 | 30 |
| +39.9970 | Pyro-carbamidomethyl | 0.23 | 23 | S-carbamoylmethylcysteine cyclization (N-terminus)/Glyoxal-derived hydroimiadazolone | 0.40 | 20 |
| +47.9889 | Trioxidation | 0.23 | 24 | cysteine oxidation to cysteic acid | 0.21 | 26 |
| +128.0950 | Lys | 0.23 | 25 | Addition of lysine due to transpeptidation/Addition of K | 0.19 | 27 |
| +17.0275 | Ammonium | 0.22 | 26 | deuterated methyl ester | 0.28 | 23 |
| +21.9815 | Cation:Na | 0.21 | 27 | Sodium adduct | 0.77 | 15 |
| +37.9492 | Cation:Ca[II] | 0.18 | 28 | Replacement of proton by potassium | 0.35 | 21 |
| +210.1634 | Unknown:210 | 0.18 | 29 | Unidentified modification of 210.1616 found in open search | 0.29 | 22 |
| +71.0379 | Propionamide | 0.16 | 30 | Acrylamide adduct/Addition of A | 0.19 | 28 |
| +55.9228 | Cation:Ni[II] | 0.16 | 31 | Replacement of 2 protons by nickel | 0.27 | 24 |

**Sage-Recon only (in window, no match):**
- +73.0186  UNANNOTATED  (0.94%, 145 PSMs)
- +109.9354  UNANNOTATED  (0.77%, 119 PSMs)
- +16.9990  UNANNOTATED  (0.47%, 73 PSMs)
- +115.0479  UNANNOTATED  (0.40%, 61 PSMs)
- +110.9386  UNANNOTATED  (0.34%, 52 PSMs)
- -14.0159  UNANNOTATED  (0.32%, 50 PSMs)
- +171.0690  UNANNOTATED  (0.30%, 46 PSMs)
- +74.0213  UNANNOTATED  (0.29%, 44 PSMs)
- +32.9925  UNANNOTATED  (0.29%, 44 PSMs)
- +55.0110  UNANNOTATED  (0.27%, 42 PSMs)
- +210.0244  UNANNOTATED  (0.24%, 37 PSMs)
- -1.0568  UNANNOTATED  (0.24%, 37 PSMs)
- +14.9840  UNANNOTATED  (0.21%, 33 PSMs)
- +71.0003  UNANNOTATED  (0.20%, 31 PSMs)
- +1.9861  UNANNOTATED  (0.19%, 30 PSMs)
- +56.0068  UNANNOTATED  (0.18%, 27 PSMs)
- +185.1192  UNANNOTATED  (0.16%, 25 PSMs)

**PTM-Shepherd (reallyOpen) only (in window, no match):**
- +1.0024  First isotopic peak  (3.22%, 485 PSMs)
- +0.0186  Unannotated mass-shift 0.0186  (0.74%, 111 PSMs)
- +3.0071  Third isotopic peak  (0.69%, 103 PSMs)
- -2.0156  2-amino-3-oxo-butanoic_acid  (0.67%, 100 PSMs)
- -1.0024  Isotopic peak error  (0.59%, 89 PSMs)
- -15.9949  reduction  (0.52%, 78 PSMs)
- -33.9877  Dehydroalanine (from Cysteine)  (0.45%, 67 PSMs)
- +43.0058  Carbamylation  (0.42%, 63 PSMs)
- +72.0211  carboxyethyl/Dihydroxy methylglyoxal adduct/Ethoxyformylation  (0.39%, 59 PSMs)
- +2.0047  Second isotopic peak  (0.36%, 54 PSMs)
- +27.9949  Formylation  (0.36%, 54 PSMs)
- +44.9851  Oxidation to nitro  (0.28%, 42 PSMs)
- +28.0313  di-Methylation/Acetaldehyde +28/Ethylation  (0.24%, 37 PSMs)
- +31.9721  persulfide  (0.23%, 35 PSMs)
- -28.0306  Unannotated mass-shift -28.0306  (0.22%, 33 PSMs)
- -30.0106  Proline oxidation to pyrrolidinone/Decarboxylation  (0.21%, 32 PSMs)
- +130.0266  S-(2-monomethylsuccinyl) cysteine  (0.20%, 30 PSMs)
- +88.9965  C13 label (Phosphotyrosine)  (0.20%, 30 PSMs)
- +59.0363  Propionate labeling reagent heavy form (+3amu), N-term  K  (0.19%, 28 PSMs)
- +55.9898  DST crosslinker cleaved by sodium periodate  (0.18%, 27 PSMs)


### b1906  (Sage-Recon vs PTM-Shepherd (reallyOpen))

- Matched (both, within 0.015 Da): **25**
- Sage-Recon only: **23**
- PTM-Shepherd (reallyOpen) only: **80**
- Spearman ρ (% matched): **0.700** (p=9.76e-05, n=25)
- Top-10 overlap (by % in matched set): **5/10**
- _Window coverage check: Sage-Recon peaks >+100 Da in window: 8 (Sage total >+100: 8, other tool total >+100: 16)_

| mass (Da) | Sage-Recon label | Recon % | Recon rank | PTM-Shepherd (reallyOpen) label | PTM-Shepherd (reallyOpen) % | PTM-Shepherd (reallyOpen) rank |
|---|---|---|---|---|---|---|
| +0.0000 | Unmodified | 51.17 | 1 | None | 56.99 | 1 |
| +57.0219 | Carbamidomethyl | 4.47 | 2 | Iodoacetamide derivative/Addition of Glycine/Addition of G | 10.77 | 2 |
| +15.9949 | Oxidation | 2.67 | 3 | Oxidation or Hydroxylation | 7.68 | 3 |
| +43.0058 | Carbamyl | 1.98 | 4 | Carbamylation | 2.55 | 4 |
| +53.9186 | Cation:Fe[II] | 1.22 | 5 | Replacement of 2 protons by iron | 1.97 | 5 |
| +58.0237 | UNANNOTATED | 0.89 | 6 | 2,3-dihydro-2,2-dimethyl-7-benzofuranol N-methyl carbamate | 0.16 | 16 |
| +0.9818 | Deamidated | 0.81 | 7 | Deamidation | 0.78 | 10 |
| +27.9946 | Formyl | 0.69 | 8 | Formylation | 1.52 | 6 |
| +301.9863 | Unknown:302 | 0.66 | 9 | Unidentified modification of 301.9864 found in open search | 0.84 | 9 |
| -1.0225 | Lys-&gt;Allysine | 0.40 | 10 | Lysine oxidation to aminoadipic semialdehyde | 0.04 | 24 |
| +183.0353 | AEBS | 0.28 | 11 | Aminoethylbenzenesulfonylation | 1.26 | 7 |
| +114.0426 | GG | 0.27 | 12 | ubiquitinylation residue/Double Carbamidomethylation/Addition of N | 1.05 | 8 |
| +59.0277 | AEC-MAEC | 0.27 | 13 | Propionate labeling reagent heavy form (+3amu), N-term  K | 0.11 | 18 |
| -2.0404 | UNANNOTATED | 0.27 | 14 | Unannotated mass-shift -2.0478 | 0.08 | 20 |
| +128.0949 | Lys | 0.24 | 15 | Addition of lysine due to transpeptidation/Addition of K | 0.08 | 19 |
| +31.9895 | Dioxidation | 0.21 | 16 | dihydroxy | 0.65 | 11 |
| +37.9463 | Cation:Ca[II] | 0.21 | 17 | Replacement of proton by potassium | 0.33 | 14 |
| +1.9197 | UNANNOTATED | 0.20 | 18 | Unannotated mass-shift 1.9136 | 0.07 | 21 |
| +44.0092 | Delta:H(4)C(2)O(-1)S(1) | 0.20 | 19 | S-Ethylcystine from Serine | 0.07 | 22 |
| +17.9982 | Fluoro | 0.19 | 20 | Proline oxidation to 5-hydroxy-2-aminovaleric acid | 0.05 | 23 |
| +21.9798 | Cation:Na | 0.16 | 21 | Sodium adduct | 0.38 | 13 |
| -18.0113 | Dehydrated | 0.15 | 22 | Dehydration/Pyro-glu from E | 0.29 | 15 |
| -17.0266 | Gln-&gt;pyro-Glu | 0.15 | 23 | Pyro-glu from Q/Loss of ammonia | 0.42 | 12 |
| +28.9983 | Nitrosyl | 0.12 | 24 | nitrosylation | 0.02 | 25 |
| +249.9807 | Unknown:250 | 0.12 | 25 | Unidentified modification of 249.981 found in open search | 0.12 | 17 |

**Sage-Recon only (in window, no match):**
- +16.9978  UNANNOTATED  (0.67%, 187 PSMs)
- +73.0161  UNANNOTATED  (0.41%, 115 PSMs)
- -1.0621  UNANNOTATED  (0.39%, 109 PSMs)
- +0.9505  UNANNOTATED  (0.38%, 107 PSMs)
- +0.9300  UNANNOTATED  (0.36%, 102 PSMs)
- +1.9676  UNANNOTATED  (0.32%, 89 PSMs)
- +1.9484  UNANNOTATED  (0.29%, 80 PSMs)
- -1.0793  UNANNOTATED  (0.28%, 78 PSMs)
- -1.9715  UNANNOTATED  (0.27%, 76 PSMs)
- -0.0997  Unmodified  (0.27%, 75 PSMs)
- +54.9208  UNANNOTATED  (0.25%, 71 PSMs)
- +110.9403  UNANNOTATED  (0.24%, 66 PSMs)
- -2.0871  UNANNOTATED  (0.24%, 66 PSMs)
- -1.0983  UNANNOTATED  (0.22%, 63 PSMs)
- -1.9492  UNANNOTATED  (0.22%, 63 PSMs)
- +0.8970  UNANNOTATED  (0.18%, 51 PSMs)
- -1.1176  UNANNOTATED  (0.18%, 50 PSMs)
- +0.8793  UNANNOTATED  (0.18%, 50 PSMs)
- +74.0190  UNANNOTATED  (0.14%, 39 PSMs)
- +100.0264  UNANNOTATED  (0.12%, 34 PSMs)

**PTM-Shepherd (reallyOpen) only (in window, no match):**
- +1.0024  First isotopic peak  (2.23%, 588 PSMs)
- +0.0186  Unannotated mass-shift 0.0186  (0.98%, 260 PSMs)
- +2.0047  Second isotopic peak  (0.51%, 135 PSMs)
- -1.0024  Isotopic peak error  (0.43%, 114 PSMs)
- +42.0106  Acetylation  (0.42%, 110 PSMs)
- -2.0156  2-amino-3-oxo-butanoic_acid  (0.39%, 104 PSMs)
- +79.9663  Phosphorylation  (0.32%, 85 PSMs)
- +39.9949  S-carbamoylmethylcysteine cyclization (N-terminus)/Glyoxal-derived hydroimiadazolone  (0.24%, 64 PSMs)
- +125.8966  Iodination  (0.20%, 53 PSMs)
- +28.0313  di-Methylation/Acetaldehyde +28/Ethylation  (0.18%, 49 PSMs)
- +3.0071  Third isotopic peak  (0.17%, 45 PSMs)
- +58.0055  Iodoacetic acid derivative  (0.14%, 38 PSMs)
- +13.9793  proline oxidation to pyroglutamic acid/Tryptophan oxidation to oxolactone/aldehyde and ketone modifications  (0.14%, 37 PSMs)
- +89.0113  Photo-induced Glycine Adduct  (0.11%, 30 PSMs)
- +14.0157  Methylation  (0.10%, 26 PSMs)
- +12.0000  formaldehyde adduct  (0.08%, 22 PSMs)
- +156.1011  Addition of arginine due to transpeptidation/Addition of R  (0.08%, 21 PSMs)
- +26.0157  Acetaldehyde +26  (0.07%, 19 PSMs)
- -3.9949  Pyro-Glu from E + Methylation  (0.07%, 18 PSMs)
- -15.9949  reduction  (0.06%, 17 PSMs)

