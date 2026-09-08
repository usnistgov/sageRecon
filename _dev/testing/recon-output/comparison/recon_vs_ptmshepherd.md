# Mod-discovery cross-comparison: Sage-Recon vs PTM-Shepherd (open)

OBJECTIVE tool-vs-tool benchmark (NOT a gate; NOT a comparison to the tool's author). Sage-Recon is the reference column. Divergence is expected — see the methodology deltas in `testing/reference-data/ptm-shepherd/README.md` (recalibrated two-stage search, per-file instruments, different FDR, wider window, higher peak floor, speed). Percentages are the currency; PSM totals differ by FDR.

Match tolerance 0.015 Da; shared window [-100.0, 500.0] Da (Sage delta range −100..+500, PTM-Shepherd −150..+500; true overlap −100..+500).

### bcell  (Sage-Recon vs PTM-Shepherd (open))

- Matched (both, within 0.015 Da): **22**
- Sage-Recon only: **25**
- PTM-Shepherd (open) only: **89**
- Spearman ρ (% matched): **0.237** (p=0.289, n=22)
- Top-10 overlap (by % in matched set): **3/10**
- _Window coverage check: Sage-Recon peaks >+100 Da in window: 4 (Sage total >+100: 4, other tool total >+100: 24)_

| mass (Da) | Sage-Recon label | Recon % | Recon rank | PTM-Shepherd (open) label | PTM-Shepherd (open) % | PTM-Shepherd (open) rank |
|---|---|---|---|---|---|---|
| +0.0000 | Unmodified | 58.29 | 1 | None | 77.31 | 1 |
| +57.0220 | Carbamidomethyl | 4.50 | 2 | Iodoacetamide derivative/Addition of Glycine/Addition of G | 0.09 | 18 |
| +15.9953 | Oxidation | 1.92 | 3 | Oxidation or Hydroxylation | 0.57 | 6 |
| +0.9821 | Deamidated | 1.03 | 4 | Deamidation | 1.82 | 2 |
| +0.9509 | UNANNOTATED | 0.52 | 5 | Unannotated mass-shift 0.9480 | 0.17 | 13 |
| -1.0290 | Lys-&gt;Allysine | 0.51 | 6 | Lysine oxidation to aminoadipic semialdehyde | 0.28 | 11 |
| +52.9113 | Cation:Fe[III] | 0.49 | 7 | Replacement of 3 protons by iron | 0.93 | 4 |
| -0.9804 | Glu-&gt;pyro-Glu+Methyl:2H(2)13C(1) | 0.41 | 8 | Pyro-Glu from E + Methylation Medium | 0.02 | 21 |
| -17.0257 | Gln-&gt;pyro-Glu | 0.34 | 9 | Pyro-glu from Q/Loss of ammonia | 1.32 | 3 |
| -89.0289 | Met-loss+Acetyl | 0.28 | 10 | Removal of initiator methionine from protein N-terminus, then acetylation of the new N-terminus | 0.02 | 22 |
| +1.9196 | UNANNOTATED | 0.28 | 11 | Unannotated mass-shift 1.9136 | 0.11 | 15 |
| +53.9139 | Cation:Fe[II] | 0.25 | 12 | Replacement of 2 protons by iron | 0.04 | 19 |
| +31.9901 | Dioxidation | 0.21 | 13 | dihydroxy | 0.55 | 7 |
| +18.0012 | Pro-&gt;HAVA | 0.20 | 14 | Proline oxidation to 5-hydroxy-2-aminovaleric acid | 0.04 | 20 |
| +17.0257 | Ammonium | 0.19 | 15 | deuterated methyl ester | 0.70 | 5 |
| +42.0109 | Acetyl | 0.16 | 16 | Acetylation | 0.29 | 10 |
| +18.0302 | UNANNOTATED | 0.15 | 17 | monomethylation | 0.10 | 16 |
| +270.1099 | UNANNOTATED | 0.14 | 18 | Unannotated mass-shift 270.1106 | 0.30 | 9 |
| +79.9661 | Phospho | 0.11 | 19 | Phosphorylation | 0.26 | 12 |
| +78.0131 | UNANNOTATED | 0.11 | 20 | Unannotated mass-shift 78.0138 | 0.13 | 14 |
| -18.0099 | Dehydrated | 0.11 | 21 | Dehydration/Pyro-glu from E | 0.40 | 8 |
| +43.0082 | Carbamyl | 0.10 | 22 | Carbamylation | 0.10 | 17 |

**Sage-Recon only (in window, no match):**
- +58.0243  UNANNOTATED  (1.28%, 938 PSMs)
- +16.9975  UNANNOTATED  (0.53%, 389 PSMs)
- +1.9691  UNANNOTATED  (0.40%, 296 PSMs)
- +59.0280  AEC-MAEC  (0.40%, 292 PSMs)
- +0.9303  UNANNOTATED  (0.37%, 272 PSMs)
- +1.9855  UNANNOTATED  (0.33%, 241 PSMs)
- -0.0788  Unmodified  (0.31%, 226 PSMs)
- +0.9093  UNANNOTATED  (0.28%, 209 PSMs)
- -1.0601  UNANNOTATED  (0.28%, 204 PSMs)
- +114.0432  GG  (0.28%, 203 PSMs)
- -0.9599  UNANNOTATED  (0.27%, 201 PSMs)
- +73.0181  UNANNOTATED  (0.25%, 181 PSMs)
- +1.9028  UNANNOTATED  (0.23%, 166 PSMs)
- -0.0995  Unmodified  (0.22%, 164 PSMs)
- +0.8899  UNANNOTATED  (0.21%, 152 PSMs)
- +2.0499  UNANNOTATED  (0.21%, 151 PSMs)
- +2.0305  UNANNOTATED  (0.20%, 148 PSMs)
- -1.0871  UNANNOTATED  (0.20%, 147 PSMs)
- +1.8798  UNANNOTATED  (0.16%, 117 PSMs)
- -1.1068  UNANNOTATED  (0.15%, 111 PSMs)

**PTM-Shepherd (open) only (in window, no match):**
- +1.0024  First isotopic peak  (4.21%, 2948 PSMs)
- +2.0047  Second isotopic peak  (1.17%, 821 PSMs)
- -1.0024  Isotopic peak error  (0.94%, 657 PSMs)
- +3.0071  Third isotopic peak  (0.46%, 322 PSMs)
- -2.0156  2-amino-3-oxo-butanoic_acid  (0.39%, 272 PSMs)
- -0.9840  Amidation  (0.37%, 257 PSMs)
- +41.0265  amidination of lysines or N-terminal amines with methyl acetimidate  (0.36%, 252 PSMs)
- +14.0157  Methylation  (0.28%, 193 PSMs)
- -15.0109  lactic acid from N-term Ser/ISD (z+2)-series  (0.16%, 110 PSMs)
- +28.0313  di-Methylation/Acetaldehyde +28/Ethylation  (0.15%, 102 PSMs)
- +0.0452  Unannotated mass-shift 0.0452  (0.12%, 82 PSMs)
- -15.9949  reduction  (0.10%, 72 PSMs)
- +3.9949  tryptophan oxidation to kynurenin  (0.09%, 64 PSMs)
- +386.0456  Unannotated mass-shift 386.0456  (0.09%, 59 PSMs)
- +13.9793  proline oxidation to pyroglutamic acid/Tryptophan oxidation to oxolactone/aldehyde and ketone modifications  (0.08%, 56 PSMs)
- +0.0594  Unannotated mass-shift 0.0594  (0.07%, 50 PSMs)
- -48.0034  Homoserine lactone/Prompt loss of side chain from oxidised Met  (0.07%, 48 PSMs)
- +28.9902  nitrosylation  (0.07%, 48 PSMs)
- +27.9949  Formylation  (0.06%, 44 PSMs)
- -26.0526  Unannotated mass-shift -26.0526  (0.06%, 42 PSMs)


### serum  (Sage-Recon vs PTM-Shepherd (open))

- Matched (both, within 0.015 Da): **25**
- Sage-Recon only: **23**
- PTM-Shepherd (open) only: **77**
- Spearman ρ (% matched): **0.562** (p=0.00347, n=25)
- Top-10 overlap (by % in matched set): **2/10**
- _Window coverage check: Sage-Recon peaks >+100 Da in window: 11 (Sage total >+100: 11, other tool total >+100: 16)_

| mass (Da) | Sage-Recon label | Recon % | Recon rank | PTM-Shepherd (open) label | PTM-Shepherd (open) % | PTM-Shepherd (open) rank |
|---|---|---|---|---|---|---|
| +0.0000 | Unmodified | 32.24 | 1 | None | 48.17 | 1 |
| +57.0237 | Carbamidomethyl | 7.31 | 2 | Iodoacetamide derivative/Addition of Glycine/Addition of G | 0.79 | 13 |
| +52.9130 | Cation:Fe[III] | 1.90 | 3 | Replacement of 3 protons by iron | 3.58 | 2 |
| +15.9951 | Oxidation | 1.85 | 4 | Oxidation or Hydroxylation | 1.72 | 8 |
| +0.9845 | Deamidated | 1.20 | 5 | Deamidation | 2.41 | 3 |
| +31.9907 | Dioxidation | 1.17 | 6 | dihydroxy | 2.01 | 5 |
| +209.0205 | CarbamidomethylDTT | 0.90 | 7 | Carbamidomethylated DTT modification of cysteine | 0.14 | 22 |
| +23.9593 | Cation:Al[III] | 0.80 | 8 | Replacement of 3 protons by aluminium | 1.81 | 6 |
| +58.0128 | Carboxymethyl | 0.60 | 9 | Iodoacetic acid derivative | 0.20 | 20 |
| -18.0101 | Dehydrated | 0.51 | 10 | Dehydration/Pyro-glu from E | 1.80 | 7 |
| +13.9783 | Pro-&gt;pyro-Glu | 0.45 | 11 | proline oxidation to pyroglutamic acid/Tryptophan oxidation to oxolactone/aldehyde and ketone modifications | 1.25 | 9 |
| -17.0285 | Gln-&gt;pyro-Glu | 0.38 | 12 | Pyro-glu from Q/Loss of ammonia | 2.02 | 4 |
| +53.9156 | Cation:Fe[II] | 0.38 | 13 | Replacement of 2 protons by iron | 0.25 | 19 |
| +162.0542 | UNANNOTATED | 0.38 | 14 | Hexose | 1.11 | 10 |
| -0.9786 | Glu-&gt;pyro-Glu+Methyl:2H(2)13C(1) | 0.34 | 15 | Pyro-Glu from E + Methylation Medium | 0.03 | 25 |
| +14.0149 | Methyl | 0.34 | 16 | Methylation | 0.98 | 11 |
| -1.0296 | Lys-&gt;Allysine | 0.29 | 17 | Lysine oxidation to aminoadipic semialdehyde | 0.28 | 18 |
| -1.0568 | UNANNOTATED | 0.24 | 18 | Unannotated mass-shift -1.0430 | 0.05 | 24 |
| +47.9889 | Trioxidation | 0.23 | 19 | cysteine oxidation to cysteic acid | 0.07 | 23 |
| +128.0950 | Lys | 0.23 | 20 | Addition of lysine due to transpeptidation/Addition of K | 0.15 | 21 |
| +17.0275 | Ammonium | 0.22 | 21 | deuterated methyl ester | 0.41 | 15 |
| +21.9815 | Cation:Na | 0.21 | 22 | Sodium adduct | 0.88 | 12 |
| +37.9492 | Cation:Ca[II] | 0.18 | 23 | Replacement of proton by potassium | 0.45 | 14 |
| +210.1634 | Unknown:210 | 0.18 | 24 | Unidentified modification of 210.1616 found in open search | 0.33 | 17 |
| +55.9228 | Cation:Ni[II] | 0.16 | 25 | Replacement of 2 protons by nickel | 0.35 | 16 |

**Sage-Recon only (in window, no match):**
- +58.0260  UNANNOTATED  (2.27%, 350 PSMs)
- +73.0186  UNANNOTATED  (0.94%, 145 PSMs)
- +114.0457  GG  (0.86%, 132 PSMs)
- +109.9354  UNANNOTATED  (0.77%, 119 PSMs)
- +16.9990  UNANNOTATED  (0.47%, 73 PSMs)
- +59.0276  AEC-MAEC  (0.47%, 73 PSMs)
- +115.0479  UNANNOTATED  (0.40%, 61 PSMs)
- +80.9812  Arg-&gt;Npo  (0.35%, 54 PSMs)
- +110.9386  UNANNOTATED  (0.34%, 52 PSMs)
- -14.0159  UNANNOTATED  (0.32%, 50 PSMs)
- +89.0121  Gly+O(2)  (0.32%, 49 PSMs)
- +171.0690  UNANNOTATED  (0.30%, 46 PSMs)
- +74.0213  UNANNOTATED  (0.29%, 44 PSMs)
- +32.9925  UNANNOTATED  (0.29%, 44 PSMs)
- +55.0110  UNANNOTATED  (0.27%, 42 PSMs)
- +210.0244  UNANNOTATED  (0.24%, 37 PSMs)
- +39.9970  Pyro-carbamidomethyl  (0.23%, 36 PSMs)
- +14.9840  UNANNOTATED  (0.21%, 33 PSMs)
- +71.0003  UNANNOTATED  (0.20%, 31 PSMs)
- +1.9861  UNANNOTATED  (0.19%, 30 PSMs)

**PTM-Shepherd (open) only (in window, no match):**
- +1.0024  First isotopic peak  (5.24%, 961 PSMs)
- +151.9966  DTT adduct of cysteine  (1.88%, 344 PSMs)
- -1.0024  Isotopic peak error  (1.32%, 242 PSMs)
- +2.0047  Second isotopic peak  (0.83%, 153 PSMs)
- +3.0071  Third isotopic peak  (0.81%, 148 PSMs)
- -15.9949  reduction  (0.67%, 123 PSMs)
- -25.0314  Unannotated mass-shift -25.0314  (0.54%, 98 PSMs)
- -91.0084  Unannotated mass-shift -91.0084  (0.44%, 81 PSMs)
- +31.9721  persulfide  (0.41%, 76 PSMs)
- +26.0157  Acetaldehyde +26  (0.41%, 75 PSMs)
- -15.0109  lactic acid from N-term Ser/ISD (z+2)-series  (0.38%, 69 PSMs)
- -2.0156  2-amino-3-oxo-butanoic_acid  (0.35%, 65 PSMs)
- +27.9949  Formylation  (0.35%, 64 PSMs)
- +152.9884  Shimadzu NBS-12C  (0.33%, 60 PSMs)
- +44.9851  Oxidation to nitro  (0.31%, 56 PSMs)
- -0.9840  Amidation  (0.29%, 54 PSMs)
- -30.0106  Proline oxidation to pyrrolidinone/Decarboxylation  (0.29%, 54 PSMs)
- -28.0306  Unannotated mass-shift -28.0306  (0.28%, 52 PSMs)
- +28.0313  di-Methylation/Acetaldehyde +28/Ethylation  (0.27%, 50 PSMs)
- -57.0034  Unannotated mass-shift -57.0034  (0.27%, 49 PSMs)


### b1906  (Sage-Recon vs PTM-Shepherd (open))

- Matched (both, within 0.015 Da): **22**
- Sage-Recon only: **26**
- PTM-Shepherd (open) only: **74**
- Spearman ρ (% matched): **0.533** (p=0.0107, n=22)
- Top-10 overlap (by % in matched set): **4/10**
- _Window coverage check: Sage-Recon peaks >+100 Da in window: 8 (Sage total >+100: 8, other tool total >+100: 14)_

| mass (Da) | Sage-Recon label | Recon % | Recon rank | PTM-Shepherd (open) label | PTM-Shepherd (open) % | PTM-Shepherd (open) rank |
|---|---|---|---|---|---|---|
| +0.0000 | Unmodified | 51.17 | 1 | None | 72.02 | 1 |
| +57.0219 | Carbamidomethyl | 4.47 | 2 | Iodoacetamide derivative/Addition of Glycine/Addition of G | 0.16 | 14 |
| +15.9949 | Oxidation | 2.67 | 3 | Oxidation or Hydroxylation | 0.36 | 12 |
| +43.0058 | Carbamyl | 1.98 | 4 | Carbamylation | 2.60 | 2 |
| +53.9186 | Cation:Fe[II] | 1.22 | 5 | Replacement of 2 protons by iron | 2.13 | 3 |
| +0.9818 | Deamidated | 0.81 | 6 | Deamidation | 1.51 | 5 |
| +27.9946 | Formyl | 0.69 | 7 | Formylation | 1.63 | 4 |
| +301.9863 | Unknown:302 | 0.66 | 8 | Unidentified modification of 301.9864 found in open search | 0.93 | 7 |
| -1.0225 | Lys-&gt;Allysine | 0.40 | 9 | Lysine oxidation to aminoadipic semialdehyde | 0.09 | 17 |
| +0.9505 | UNANNOTATED | 0.38 | 10 | Unannotated mass-shift 0.9480 | 0.03 | 20 |
| +183.0353 | AEBS | 0.28 | 11 | Aminoethylbenzenesulfonylation | 1.44 | 6 |
| +128.0949 | Lys | 0.24 | 12 | Addition of lysine due to transpeptidation/Addition of K | 0.13 | 16 |
| +31.9895 | Dioxidation | 0.21 | 13 | dihydroxy | 0.68 | 9 |
| +37.9463 | Cation:Ca[II] | 0.21 | 14 | Replacement of proton by potassium | 0.41 | 10 |
| +1.9197 | UNANNOTATED | 0.20 | 15 | Unannotated mass-shift 1.9136 | 0.01 | 22 |
| +44.0092 | Delta:H(4)C(2)O(-1)S(1) | 0.20 | 16 | S-Ethylcystine from Serine | 0.08 | 18 |
| +17.9982 | Fluoro | 0.19 | 17 | Proline oxidation to 5-hydroxy-2-aminovaleric acid | 0.01 | 21 |
| +21.9798 | Cation:Na | 0.16 | 18 | Sodium adduct | 0.39 | 11 |
| -18.0113 | Dehydrated | 0.15 | 19 | Dehydration/Pyro-glu from E | 0.36 | 13 |
| -17.0266 | Gln-&gt;pyro-Glu | 0.15 | 20 | Pyro-glu from Q/Loss of ammonia | 0.84 | 8 |
| +28.9983 | Nitrosyl | 0.12 | 21 | nitrosylation | 0.05 | 19 |
| +249.9807 | Unknown:250 | 0.12 | 22 | Unidentified modification of 249.981 found in open search | 0.14 | 15 |

**Sage-Recon only (in window, no match):**
- +58.0237  UNANNOTATED  (0.89%, 248 PSMs)
- +16.9978  UNANNOTATED  (0.67%, 187 PSMs)
- +73.0161  UNANNOTATED  (0.41%, 115 PSMs)
- -1.0621  UNANNOTATED  (0.39%, 109 PSMs)
- +0.9300  UNANNOTATED  (0.36%, 102 PSMs)
- +1.9676  UNANNOTATED  (0.32%, 89 PSMs)
- +1.9484  UNANNOTATED  (0.29%, 80 PSMs)
- -1.0793  UNANNOTATED  (0.28%, 78 PSMs)
- +114.0426  GG  (0.27%, 76 PSMs)
- -1.9715  UNANNOTATED  (0.27%, 76 PSMs)
- +59.0277  AEC-MAEC  (0.27%, 75 PSMs)
- -2.0404  UNANNOTATED  (0.27%, 75 PSMs)
- -0.0997  Unmodified  (0.27%, 75 PSMs)
- +54.9208  UNANNOTATED  (0.25%, 71 PSMs)
- +110.9403  UNANNOTATED  (0.24%, 66 PSMs)
- -2.0871  UNANNOTATED  (0.24%, 66 PSMs)
- -1.0983  UNANNOTATED  (0.22%, 63 PSMs)
- -1.9492  UNANNOTATED  (0.22%, 63 PSMs)
- +0.8970  UNANNOTATED  (0.18%, 51 PSMs)
- -1.1176  UNANNOTATED  (0.18%, 50 PSMs)

**PTM-Shepherd (open) only (in window, no match):**
- +1.0024  First isotopic peak  (2.96%, 887 PSMs)
- +2.0047  Second isotopic peak  (1.10%, 331 PSMs)
- -1.0024  Isotopic peak error  (0.76%, 227 PSMs)
- -0.9840  Amidation  (0.46%, 137 PSMs)
- -2.0156  2-amino-3-oxo-butanoic_acid  (0.45%, 134 PSMs)
- +79.9663  Phosphorylation  (0.36%, 109 PSMs)
- +3.0071  Third isotopic peak  (0.30%, 89 PSMs)
- +41.0265  amidination of lysines or N-terminal amines with methyl acetimidate  (0.20%, 61 PSMs)
- +28.0313  di-Methylation/Acetaldehyde +28/Ethylation  (0.18%, 54 PSMs)
- +125.8966  Iodination  (0.18%, 53 PSMs)
- +14.0157  Methylation  (0.15%, 44 PSMs)
- +0.0452  Unannotated mass-shift 0.0452  (0.13%, 38 PSMs)
- +0.0594  Unannotated mass-shift 0.0594  (0.11%, 32 PSMs)
- +14.9633  alpha-amino adipic acid  (0.11%, 32 PSMs)
- +2.0804  Unannotated mass-shift 2.0804  (0.10%, 30 PSMs)
- +42.0106  Acetylation  (0.09%, 28 PSMs)
- +44.0262  Ethanolation  (0.09%, 28 PSMs)
- -15.9949  reduction  (0.08%, 25 PSMs)
- +12.0000  formaldehyde adduct  (0.08%, 24 PSMs)
- +17.0345  deuterated methyl ester  (0.07%, 20 PSMs)

