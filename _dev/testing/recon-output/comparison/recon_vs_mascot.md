# Mod-discovery cross-comparison: Sage-Recon vs Mascot (error-tolerant)

OBJECTIVE tool-vs-tool benchmark (NOT a gate; NOT a comparison to the tool's author). Sage-Recon is the reference column. Divergence is expected — see the methodology deltas in `testing/reference-data/ptm-shepherd/README.md` (recalibrated two-stage search, per-file instruments, different FDR, wider window, higher peak floor, speed). Percentages are the currency; PSM totals differ by FDR.

Match tolerance 0.015 Da; shared window [-100.0, 500.0] Da (Sage delta range −100..+500, PTM-Shepherd −150..+500; true overlap −100..+500).

### bcell  (Sage-Recon vs Mascot (error-tolerant))

- Matched (both, within 0.015 Da): **27**
- Sage-Recon only: **20**
- Mascot (error-tolerant) only: **373**
- Spearman ρ (% matched): **0.053** (p=0.795, n=27)
- Top-10 overlap (by % in matched set): **1/10**
- _Window coverage check: Sage-Recon peaks >+100 Da in window: 4 (Sage total >+100: 4, other tool total >+100: 122)_

| mass (Da) | Sage-Recon label | Recon % | Recon rank | Mascot (error-tolerant) label | Mascot (error-tolerant) % | Mascot (error-tolerant) rank |
|---|---|---|---|---|---|---|
| +57.0220 | Carbamidomethyl | 4.50 | 1 | Carbamidomethyl [A,C,D,E,G,H,K,N-term,S,T,U,Y] | 33.76 | 1 |
| +15.9953 | Oxidation | 1.92 | 2 | Oxidation [A,C,D,F,K,M,N,P,R,W,Y] | 13.32 | 2 |
| +58.0243 | UNANNOTATED | 1.28 | 3 | Gln->Trp [Q] | 0.21 | 15 |
| +0.9821 | Deamidated | 1.03 | 4 | Deamidated [N,Q,R] | 3.45 | 3 |
| +16.9975 | UNANNOTATED | 0.53 | 5 | Asn->Met [N] | 0.30 | 12 |
| +0.9509 | UNANNOTATED | 0.52 | 6 | Lys->Glu [K] | 0.04 | 23 |
| -1.0290 | Lys-&gt;Allysine | 0.51 | 7 | Lys->Allysine [K] | 0.02 | 25 |
| -0.9804 | Glu-&gt;pyro-Glu+Methyl:2H(2)13C(1) | 0.41 | 8 | Glu->pyro-Glu+Methyl:2H(2)13C(1) [N-term] | 0.01 | 26 |
| +1.9691 | UNANNOTATED | 0.40 | 9 | Thr->Cys [T] | 0.04 | 24 |
| +59.0280 | AEC-MAEC | 0.40 | 10 | Propionyl:13C(3) [K,N-term] | 0.53 | 10 |
| -17.0257 | Gln-&gt;pyro-Glu | 0.34 | 11 | Gln->pyro-Glu [N,N-term] | 2.04 | 5 |
| +1.9855 | UNANNOTATED | 0.33 | 12 | Val->Thr [V] | 0.09 | 21 |
| -89.0289 | Met-loss+Acetyl | 0.28 | 13 | Met-loss+Acetyl [N-term] | 0.25 | 13 |
| +114.0432 | GG | 0.28 | 14 | GG [C,K,N-term,S,T] | 0.35 | 11 |
| -0.9599 | UNANNOTATED | 0.27 | 15 | Asn->Xle [N] | 0.01 | 27 |
| +53.9139 | Cation:Fe[II] | 0.25 | 16 | Cation:Fe[II] [D,E] | 0.25 | 14 |
| +31.9901 | Dioxidation | 0.21 | 17 | Dioxidation [C,F,M,R,W,Y] | 1.04 | 6 |
| +18.0012 | Pro-&gt;HAVA | 0.20 | 18 | Fluoro [A,F,Y] | 0.19 | 17 |
| +2.0305 | UNANNOTATED | 0.20 | 19 | Pro->Val [P] | 0.15 | 18 |
| +17.0257 | Ammonium | 0.19 | 20 | Ammonium [D,E] | 0.09 | 22 |
| +42.0109 | Acetyl | 0.16 | 21 | Acetyl [C,H,K,N-term,S,T,Y] | 3.18 | 4 |
| +18.0302 | UNANNOTATED | 0.15 | 22 | Glu->Phe [E] | 0.19 | 16 |
| +115.0481 | UNANNOTATED | 0.13 | 23 | Ala->Trp [A] | 0.13 | 19 |
| +74.0195 | UNANNOTATED | 0.11 | 24 | Gly->Met [G] | 0.12 | 20 |
| +79.9661 | Phospho | 0.11 | 25 | Phospho [S,T,Y] | 0.57 | 9 |
| -18.0099 | Dehydrated | 0.11 | 26 | Dehydrated [D,N-term,S,T,Y] | 0.85 | 7 |
| +43.0082 | Carbamyl | 0.10 | 27 | Carbamyl [A,C,K,M,N-term,R,S,T,Y] | 0.65 | 8 |

**Sage-Recon only (in window, no match):**
- +0.0000  Unmodified  (58.29%, 42861 PSMs)
- +52.9113  Cation:Fe[III]  (0.49%, 359 PSMs)
- +0.9303  UNANNOTATED  (0.37%, 272 PSMs)
- -0.0788  Unmodified  (0.31%, 226 PSMs)
- +0.9093  UNANNOTATED  (0.28%, 209 PSMs)
- +1.9196  UNANNOTATED  (0.28%, 205 PSMs)
- -1.0601  UNANNOTATED  (0.28%, 204 PSMs)
- +73.0181  UNANNOTATED  (0.25%, 181 PSMs)
- +1.9028  UNANNOTATED  (0.23%, 166 PSMs)
- -0.0995  Unmodified  (0.22%, 164 PSMs)
- +0.8899  UNANNOTATED  (0.21%, 152 PSMs)
- +2.0499  UNANNOTATED  (0.21%, 151 PSMs)
- -1.0871  UNANNOTATED  (0.20%, 147 PSMs)
- +1.8798  UNANNOTATED  (0.16%, 117 PSMs)
- -1.1068  UNANNOTATED  (0.15%, 111 PSMs)
- +270.1099  UNANNOTATED  (0.14%, 102 PSMs)
- -0.1276  UNANNOTATED  (0.13%, 96 PSMs)
- +78.0131  UNANNOTATED  (0.11%, 80 PSMs)
- +2.9108  UNANNOTATED  (0.11%, 80 PSMs)
- +284.1256  UNANNOTATED  (0.09%, 67 PSMs)

**Mascot (error-tolerant) only (in window, no match):**
- +0.9970  Label:15N(1) [A,D,E,F,G,I,L,M,P,S,T,V,Y]  (20.36%, 3479 PSMs)
- +58.0419  Delta:H(6)C(3)O(1) [C,H,K]  (1.64%, 280 PSMs)
- +58.0055  Carboxymethyl [A,C,G,K,N-term]  (1.37%, 234 PSMs)
- +39.9949  Pyro-carbamidomethyl [N-term,R]  (0.92%, 157 PSMs)
- +2.0042  Label:18O(1) [C-term,S,T,Y]  (0.74%, 127 PSMs)
- +1.9941  Label:15N(2) [K,N,Q,W]  (0.63%, 107 PSMs)
- +14.0157  Val->Xle [D,E,G,H,I,K,L,N-term,R,S,T,V]  (0.43%, 73 PSMs)
- +17.0345  Methyl:2H(3) [C-term,D,E,K]  (0.40%, 69 PSMs)
- +156.1011  Arg [N-term]  (0.35%, 59 PSMs)
- -2.0156  Didehydro [C-term,S,T,V,Y]  (0.34%, 58 PSMs)
- +16.9902  Pro->Asn [P]  (0.33%, 56 PSMs)
- +59.0194  AEC-MAEC [S,T]  (0.29%, 49 PSMs)
- +42.0218  Guanidinyl [C,K,N-term]  (0.27%, 47 PSMs)
- +28.0313  Ethyl [A,D,E,H,K,N,N-term,R]  (0.23%, 39 PSMs)
- +128.0950  Lys [N-term]  (0.19%, 33 PSMs)
- +0.9589  Xle->Asn [I,L]  (0.19%, 32 PSMs)
- +18.0378  Methyl:2H(3)13C(1) [K,N-term,R]  (0.18%, 31 PSMs)
- +1.9979  Glu->Met [E]  (0.17%, 29 PSMs)
- +29.0153  Ethyl+Deamidated [N,Q]  (0.16%, 28 PSMs)
- -99.0473  Trp->Ser [W]  (0.13%, 23 PSMs)

_Mascot rows with no Unimod mass (skipped, not dropped silently): 10 (1137 ET matches)._

### serum  (Sage-Recon vs Mascot (error-tolerant))

- Matched (both, within 0.015 Da): **33**
- Sage-Recon only: **15**
- Mascot (error-tolerant) only: **346**
- Spearman ρ (% matched): **0.634** (p=7.36e-05, n=33)
- Top-10 overlap (by % in matched set): **1/10**
- _Window coverage check: Sage-Recon peaks >+100 Da in window: 11 (Sage total >+100: 11, other tool total >+100: 117)_

| mass (Da) | Sage-Recon label | Recon % | Recon rank | Mascot (error-tolerant) label | Mascot (error-tolerant) % | Mascot (error-tolerant) rank |
|---|---|---|---|---|---|---|
| +57.0237 | Carbamidomethyl | 7.31 | 1 | Carbamidomethyl [A,C,D,E,G,H,K,N-term,S,T,Y] | 34.16 | 1 |
| +58.0260 | UNANNOTATED | 2.27 | 2 | Gln->Trp [Q] | 0.22 | 24 |
| +15.9951 | Oxidation | 1.85 | 3 | Oxidation [A,D,F,K,M,N,P,R,W,Y] | 6.40 | 2 |
| +0.9845 | Deamidated | 1.20 | 4 | Deamidated [N,Q,R] | 3.36 | 3 |
| +31.9907 | Dioxidation | 1.17 | 5 | Dioxidation [C,F,M,P,R,W,Y] | 2.19 | 5 |
| +209.0205 | CarbamidomethylDTT | 0.90 | 6 | CarbamidomethylDTT [C] | 2.55 | 4 |
| +114.0457 | GG | 0.86 | 7 | GG [C,K,N-term,S,T] | 0.41 | 16 |
| +58.0128 | Carboxymethyl | 0.60 | 8 | Carboxymethyl [A,C,G,K,N-term,W] | 1.80 | 6 |
| -18.0101 | Dehydrated | 0.51 | 9 | Dehydrated [D,N-term,S,T,Y] | 1.57 | 7 |
| +16.9990 | UNANNOTATED | 0.47 | 10 | Asn->Met [N] | 0.29 | 21 |
| +59.0276 | AEC-MAEC | 0.47 | 11 | AEC-MAEC [S,T] | 0.68 | 13 |
| +13.9783 | Pro-&gt;pyro-Glu | 0.45 | 12 | Trp->Oxolactone [P,T,W] | 0.79 | 11 |
| +115.0479 | UNANNOTATED | 0.40 | 13 | Ala->Trp [A] | 0.32 | 19 |
| -17.0285 | Gln-&gt;pyro-Glu | 0.38 | 14 | Gln->pyro-Glu [N,N-term] | 1.23 | 9 |
| +53.9156 | Cation:Fe[II] | 0.38 | 15 | Cation:Fe[II] [D,E] | 0.45 | 15 |
| +162.0542 | UNANNOTATED | 0.38 | 16 | Hex [K,N,N-term,S,T,W,Y] | 0.69 | 12 |
| +80.9812 | Arg-&gt;Npo | 0.35 | 17 | Arg->Npo [R] | 0.03 | 30 |
| -0.9786 | Glu-&gt;pyro-Glu+Methyl:2H(2)13C(1) | 0.34 | 18 | Glu->pyro-Glu+Methyl:2H(2)13C(1) [N-term] | 0.01 | 32 |
| +14.0149 | Methyl | 0.34 | 19 | Methyl [C,D,E,G,I,K,L,N,N-term,Q,R,S,T,V] | 1.50 | 8 |
| -14.0159 | UNANNOTATED | 0.32 | 20 | Xle->Val [A,E,I,L,T] | 0.95 | 10 |
| +89.0121 | Gly+O(2) | 0.32 | 21 | Pro->Trp [P] | 0.12 | 27 |
| -1.0296 | Lys-&gt;Allysine | 0.29 | 22 | Lys->Allysine [K] | 0.09 | 28 |
| +74.0213 | UNANNOTATED | 0.29 | 23 | Gly->Met [G] | 0.06 | 29 |
| +39.9970 | Pyro-carbamidomethyl | 0.23 | 24 | Pyro-carbamidomethyl [N-term,R] | 0.32 | 20 |
| +47.9889 | Trioxidation | 0.23 | 25 | Trioxidation [C,W] | 0.33 | 18 |
| +128.0950 | Lys | 0.23 | 26 | Lys [N-term] | 0.17 | 26 |
| +17.0275 | Ammonium | 0.22 | 27 | Ammonium [D,E] | 0.27 | 22 |
| +14.9840 | UNANNOTATED | 0.21 | 28 | Xle->Gln [I,L,V] | 0.26 | 23 |
| +21.9815 | Cation:Na | 0.21 | 29 | Cation:Na [C-term,D,E] | 0.59 | 14 |
| +1.9861 | UNANNOTATED | 0.19 | 30 | Val->Thr [V] | 0.01 | 33 |
| +37.9492 | Cation:Ca[II] | 0.18 | 31 | Cation:Ca[II] [E] | 0.03 | 31 |
| +71.0379 | Propionamide | 0.16 | 32 | Propionamide [C,G,K,N-term] | 0.40 | 17 |
| +55.9228 | Cation:Ni[II] | 0.16 | 33 | Cation:Ni[II] [D,E] | 0.22 | 25 |

**Sage-Recon only (in window, no match):**
- +0.0000  Unmodified  (32.24%, 4960 PSMs)
- +52.9130  Cation:Fe[III]  (1.90%, 292 PSMs)
- +73.0186  UNANNOTATED  (0.94%, 145 PSMs)
- +23.9593  Cation:Al[III]  (0.80%, 123 PSMs)
- +109.9354  UNANNOTATED  (0.77%, 119 PSMs)
- +110.9386  UNANNOTATED  (0.34%, 52 PSMs)
- +171.0690  UNANNOTATED  (0.30%, 46 PSMs)
- +32.9925  UNANNOTATED  (0.29%, 44 PSMs)
- +55.0110  UNANNOTATED  (0.27%, 42 PSMs)
- +210.0244  UNANNOTATED  (0.24%, 37 PSMs)
- -1.0568  UNANNOTATED  (0.24%, 37 PSMs)
- +71.0003  UNANNOTATED  (0.20%, 31 PSMs)
- +210.1634  Unknown:210  (0.18%, 28 PSMs)
- +56.0068  UNANNOTATED  (0.18%, 27 PSMs)
- +185.1192  UNANNOTATED  (0.16%, 25 PSMs)

**Mascot (error-tolerant) only (in window, no match):**
- +0.9970  Label:15N(1) [A,C,D,E,F,G,I,L,M,P,S,T,V,Y]  (9.12%, 713 PSMs)
- +58.0419  Delta:H(6)C(3)O(1) [C,H,K]  (3.20%, 250 PSMs)
- +23.9748  Xle->His [I,L]  (0.59%, 46 PSMs)
- -33.9877  Cys->Dha [C]  (0.58%, 45 PSMs)
- -15.9949  Tyr->Phe [D,S,T,Y]  (0.49%, 38 PSMs)
- +44.9851  Nitro [W,Y]  (0.47%, 37 PSMs)
- +27.9949  Formyl [K,N-term,S,T]  (0.47%, 37 PSMs)
- +133.0197  HCysteinyl [C]  (0.45%, 35 PSMs)
- +15.0109  Hydroxamic_acid [D,E,I,L,Y]  (0.43%, 34 PSMs)
- +210.0020  CarboxymethylDTT [C]  (0.42%, 33 PSMs)
- +156.1011  Arg [N-term]  (0.41%, 32 PSMs)
- +26.0157  Delta:H(2)C(2) [K,N-term]  (0.37%, 29 PSMs)
- +56.0262  Delta:H(4)C(3)O(1) [C,H,K,N-term,S,T]  (0.37%, 29 PSMs)
- +28.0313  Ala->Val [A,E,K,N-term]  (0.35%, 27 PSMs)
- -1.0078  Dehydro [C]  (0.32%, 25 PSMs)
- +27.0109  Ser->Asn [S,T]  (0.32%, 25 PSMs)
- +17.0345  Methyl:2H(3) [C-term,D,E,K]  (0.32%, 25 PSMs)
- +72.9952  Xle->Trp [I,L]  (0.32%, 25 PSMs)
- -30.0106  Ser->Gly [P,S,T]  (0.32%, 25 PSMs)
- +43.0058  Carbamyl [A,C,K,M,N-term,R,S,Y]  (0.31%, 24 PSMs)

_Mascot rows with no Unimod mass (skipped, not dropped silently): 13 (1611 ET matches)._

### b1906  (Sage-Recon vs Mascot (error-tolerant))

- Matched (both, within 0.015 Da): **28**
- Sage-Recon only: **20**
- Mascot (error-tolerant) only: **280**
- Spearman ρ (% matched): **0.370** (p=0.0527, n=28)
- Top-10 overlap (by % in matched set): **4/10**
- _Window coverage check: Sage-Recon peaks >+100 Da in window: 8 (Sage total >+100: 8, other tool total >+100: 68)_

| mass (Da) | Sage-Recon label | Recon % | Recon rank | Mascot (error-tolerant) label | Mascot (error-tolerant) % | Mascot (error-tolerant) rank |
|---|---|---|---|---|---|---|
| +57.0219 | Carbamidomethyl | 4.47 | 1 | Carbamidomethyl [A,C,D,E,G,H,K,N-term,S,T,Y] | 25.64 | 1 |
| +15.9949 | Oxidation | 2.67 | 2 | Oxidation [A,D,F,K,M,N,P,R,W,Y] | 18.48 | 2 |
| +43.0058 | Carbamyl | 1.98 | 3 | Carbamyl [A,K,M,N-term,R,S,T,Y] | 7.25 | 3 |
| +53.9186 | Cation:Fe[II] | 1.22 | 4 | Cation:Fe[II] [D,E] | 2.16 | 7 |
| +58.0237 | UNANNOTATED | 0.89 | 5 | Gln->Trp [Q] | 0.06 | 20 |
| +0.9818 | Deamidated | 0.81 | 6 | Deamidated [N,Q,R] | 2.23 | 6 |
| +27.9946 | Formyl | 0.69 | 7 | Formyl [K,N-term,S,T] | 4.12 | 4 |
| +16.9978 | UNANNOTATED | 0.67 | 8 | Asn->Met [N] | 0.33 | 14 |
| -1.0225 | Lys-&gt;Allysine | 0.40 | 9 | Lys->Allysine [K] | 0.05 | 24 |
| +0.9505 | UNANNOTATED | 0.38 | 10 | Lys->Glu [K] | 0.02 | 27 |
| +1.9676 | UNANNOTATED | 0.32 | 11 | Thr->Cys [T] | 0.06 | 21 |
| +1.9484 | UNANNOTATED | 0.29 | 12 | Xle->Asp [I,L] | 0.13 | 18 |
| +183.0353 | AEBS | 0.28 | 13 | AEBS [K,N-term,S,Y] | 3.30 | 5 |
| +114.0426 | GG | 0.27 | 14 | GG [C,K,N-term,R,S,T] | 0.36 | 13 |
| -1.9715 | UNANNOTATED | 0.27 | 15 | Thr->Val [T] | 0.02 | 28 |
| +59.0277 | AEC-MAEC | 0.27 | 16 | AEC-MAEC [S,T] | 0.13 | 19 |
| +128.0949 | Lys | 0.24 | 17 | Lys [N-term] | 0.62 | 11 |
| -1.9492 | UNANNOTATED | 0.22 | 18 | Asp->Xle [D] | 0.06 | 22 |
| +31.9895 | Dioxidation | 0.21 | 19 | Dioxidation [W] | 0.95 | 9 |
| +37.9463 | Cation:Ca[II] | 0.21 | 20 | Cation:Ca[II] [D,E] | 0.05 | 25 |
| +44.0092 | Delta:H(4)C(2)O(-1)S(1) | 0.20 | 21 | Delta:H(4)C(2)O(-1)S(1) [S] | 0.16 | 16 |
| +17.9982 | Fluoro | 0.19 | 22 | Fluoro [A,F,Y] | 0.16 | 17 |
| +21.9798 | Cation:Na | 0.16 | 23 | Cation:Na [D,E] | 0.43 | 12 |
| -18.0113 | Dehydrated | 0.15 | 24 | Glu->pyro-Glu [D,N-term,S,T,Y] | 0.82 | 10 |
| -17.0266 | Gln-&gt;pyro-Glu | 0.15 | 25 | Gln->pyro-Glu [N,N-term] | 1.37 | 8 |
| +74.0190 | UNANNOTATED | 0.14 | 26 | Gly->Met [G] | 0.03 | 26 |
| +100.0264 | UNANNOTATED | 0.12 | 27 | Succinyl [N-term,S] | 0.21 | 15 |
| +28.9983 | Nitrosyl | 0.12 | 28 | Val->Gln [V] | 0.06 | 23 |

**Sage-Recon only (in window, no match):**
- +0.0000  Unmodified  (51.17%, 14330 PSMs)
- +301.9863  Unknown:302  (0.66%, 186 PSMs)
- +73.0161  UNANNOTATED  (0.41%, 115 PSMs)
- -1.0621  UNANNOTATED  (0.39%, 109 PSMs)
- +0.9300  UNANNOTATED  (0.36%, 102 PSMs)
- -1.0793  UNANNOTATED  (0.28%, 78 PSMs)
- -2.0404  UNANNOTATED  (0.27%, 75 PSMs)
- -0.0997  Unmodified  (0.27%, 75 PSMs)
- +54.9208  UNANNOTATED  (0.25%, 71 PSMs)
- +110.9403  UNANNOTATED  (0.24%, 66 PSMs)
- -2.0871  UNANNOTATED  (0.24%, 66 PSMs)
- -1.0983  UNANNOTATED  (0.22%, 63 PSMs)
- +1.9197  UNANNOTATED  (0.20%, 56 PSMs)
- +0.8970  UNANNOTATED  (0.18%, 51 PSMs)
- -1.1176  UNANNOTATED  (0.18%, 50 PSMs)
- +0.8793  UNANNOTATED  (0.18%, 50 PSMs)
- -1.9200  UNANNOTATED  (0.16%, 44 PSMs)
- -2.1091  UNANNOTATED  (0.12%, 34 PSMs)
- +249.9807  Unknown:250  (0.12%, 33 PSMs)
- +302.9880  UNANNOTATED  (0.10%, 29 PSMs)

**Mascot (error-tolerant) only (in window, no match):**
- +0.9970  Label:15N(1) [A,D,E,F,G,I,L,M,P,S,T,V,Y]  (11.16%, 707 PSMs)
- +79.9663  Phospho [D,S,T]  (0.88%, 56 PSMs)
- +58.0419  Delta:H(6)C(3)O(1) [C,H,K]  (0.79%, 50 PSMs)
- +42.0106  Acetyl [K,N-term,S,T]  (0.66%, 42 PSMs)
- +58.0055  Carboxymethyl [A,C,G,N-term]  (0.60%, 38 PSMs)
- +156.1011  Arg [N-term]  (0.58%, 37 PSMs)
- +2.0042  Label:18O(1) [S,T,Y]  (0.54%, 34 PSMs)
- +39.9949  Pyro-carbamidomethyl [N-term,R]  (0.51%, 32 PSMs)
- +14.0157  Methyl [C,D,E,H,I,L,N,N-term,Q,R,S]  (0.49%, 31 PSMs)
- -2.0156  Val->Pro [C-term,S,T,V,Y]  (0.47%, 30 PSMs)
- +125.8966  Iodo [H,Y]  (0.44%, 28 PSMs)
- +16.9902  Pro->Asn [P]  (0.38%, 24 PSMs)
- +1.9941  Label:15N(2) [K,N,Q,W]  (0.35%, 22 PSMs)
- +28.0313  Dimethyl [A,D,K,N-term,R]  (0.28%, 18 PSMs)
- -0.9840  Glu->Gln [C-term,D,E]  (0.27%, 17 PSMs)
- +156.1150  HNE [A,H,K,L]  (0.27%, 17 PSMs)
- +72.9952  Xle->Trp [I,L]  (0.25%, 16 PSMs)
- +29.0153  Ethyl+Deamidated [N,Q]  (0.25%, 16 PSMs)
- +37.9559  Cation:K [D,E]  (0.24%, 15 PSMs)
- +42.0470  Ala->Xle [A,D,K,N-term,R]  (0.24%, 15 PSMs)

_Mascot rows with no Unimod mass (skipped, not dropped silently): 7 (644 ET matches)._
