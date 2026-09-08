# Mod-discovery cross-comparison: Sage-Recon vs MetaMorpheus (GPTMD-confirmed, reallyOpen tier)

OBJECTIVE tool-vs-tool benchmark (NOT a gate; NOT a comparison to the tool's author). Sage-Recon is the reference column. Divergence is expected — see the methodology deltas in `testing/reference-data/ptm-shepherd/README.md` (recalibrated two-stage search, per-file instruments, different FDR, wider window, higher peak floor, speed). Percentages are the currency; PSM totals differ by FDR.

Match tolerance 0.015 Da; shared window [-100.0, 500.0] Da (Sage delta range −100..+500, PTM-Shepherd −150..+500; true overlap −100..+500).

### bcell  (Sage-Recon vs MetaMorpheus (GPTMD-confirmed, reallyOpen tier))

- Matched (both, within 0.015 Da): **16**
- Sage-Recon only: **31**
- MetaMorpheus (GPTMD-confirmed, reallyOpen tier) only: **28**
- Spearman ρ (% matched): **0.442** (p=0.0869, n=16)
- Top-10 overlap (by % in matched set): **3/10**
- _Window coverage check: Sage-Recon peaks >+100 Da in window: 4 (Sage total >+100: 4, other tool total >+100: 6)_

| mass (Da) | Sage-Recon label | Recon % | Recon rank | MetaMorpheus (GPTMD-confirmed, reallyOpen tier) label | MetaMorpheus (GPTMD-confirmed, reallyOpen tier) % | MetaMorpheus (GPTMD-confirmed, reallyOpen tier) rank |
|---|---|---|---|---|---|---|
| +0.0000 | Unmodified | 58.29 | 1 | Unmodified | 77.88 | 1 |
| +57.0220 | Carbamidomethyl | 4.50 | 2 | Carbamidomethyl on C | 11.46 | 2 |
| +15.9953 | Oxidation | 1.92 | 3 | Oxidation on M | 5.35 | 3 |
| +0.9821 | Deamidated | 1.03 | 4 | Deamidation on N | 1.28 | 5 |
| -1.0290 | Lys-&gt;Allysine | 0.51 | 5 | Ammonia loss on N, Oxidation on M | 0.02 | 14 |
| +52.9113 | Cation:Fe[III] | 0.49 | 6 | Fe[III] on E | 0.65 | 6 |
| +1.9691 | UNANNOTATED | 0.40 | 7 | Deamidation on N, Deamidation on N | 0.05 | 13 |
| -17.0257 | Gln-&gt;pyro-Glu | 0.34 | 8 | Ammonia loss on N | 0.28 | 9 |
| +114.0432 | GG | 0.28 | 9 | Carbamidomethyl on C, Carbamidomethyl on C | 0.01 | 15 |
| +53.9139 | Cation:Fe[II] | 0.25 | 10 | Fe[II] on E | 0.12 | 12 |
| +73.0181 | UNANNOTATED | 0.25 | 11 | Carbamidomethyl on C, Oxidation on M | 0.01 | 16 |
| +31.9901 | Dioxidation | 0.21 | 12 | Oxidation on M, Oxidation on M | 0.46 | 7 |
| +42.0109 | Acetyl | 0.16 | 13 | Acetylation on X | 1.34 | 4 |
| +79.9661 | Phospho | 0.11 | 14 | Phosphorylation on S | 0.34 | 8 |
| -18.0099 | Dehydrated | 0.11 | 15 | Water Loss on E | 0.14 | 11 |
| +43.0082 | Carbamyl | 0.10 | 16 | Carbamyl on X | 0.21 | 10 |

**Sage-Recon only (in window, no match):**
- +58.0243  UNANNOTATED  (1.28%, 938 PSMs)
- +16.9975  UNANNOTATED  (0.53%, 389 PSMs)
- +0.9509  UNANNOTATED  (0.52%, 383 PSMs)
- -0.9804  Glu-&gt;pyro-Glu+Methyl:2H(2)13C(1)  (0.41%, 303 PSMs)
- +59.0280  AEC-MAEC  (0.40%, 292 PSMs)
- +0.9303  UNANNOTATED  (0.37%, 272 PSMs)
- +1.9855  UNANNOTATED  (0.33%, 241 PSMs)
- -0.0788  Unmodified  (0.31%, 226 PSMs)
- -89.0289  Met-loss+Acetyl  (0.28%, 209 PSMs)
- +0.9093  UNANNOTATED  (0.28%, 209 PSMs)
- +1.9196  UNANNOTATED  (0.28%, 205 PSMs)
- -1.0601  UNANNOTATED  (0.28%, 204 PSMs)
- -0.9599  UNANNOTATED  (0.27%, 201 PSMs)
- +1.9028  UNANNOTATED  (0.23%, 166 PSMs)
- -0.0995  Unmodified  (0.22%, 164 PSMs)
- +0.8899  UNANNOTATED  (0.21%, 152 PSMs)
- +2.0499  UNANNOTATED  (0.21%, 151 PSMs)
- +18.0012  Pro-&gt;HAVA  (0.20%, 148 PSMs)
- +2.0305  UNANNOTATED  (0.20%, 148 PSMs)
- -1.0871  UNANNOTATED  (0.20%, 147 PSMs)

**MetaMorpheus (GPTMD-confirmed, reallyOpen tier) only (in window, no match):**
- +114.0317  Glutarylation on K  (0.07%, 46 PSMs)
- +14.0157  Methylation on K  (0.05%, 36 PSMs)
- +16.9789  Deamidation on N, Oxidation on M  (0.05%, 33 PSMs)
- +203.0794  HexNAc on T  (0.04%, 26 PSMs)
- +44.9851  Nitrosylation on Y  (0.04%, 26 PSMs)
- +28.0313  Dimethylation on R  (0.03%, 24 PSMs)
- +42.9946  Acetylation on X, Deamidation on N  (0.02%, 15 PSMs)
- +68.9064  Fe[III] on D, Oxidation on M  (0.02%, 14 PSMs)
- +58.0055  Carbamidomethyl on C, Deamidation on Q  (0.01%, 7 PSMs)
- +121.9769  Acetylation on X, Phosphorylation on S  (0.01%, 6 PSMs)
- +21.9819  Sodium on E  (0.01%, 6 PSMs)
- +59.0007  Carbamyl on X, Oxidation on M  (0.01%, 4 PSMs)
- -2.0157  Oxidation on M, Water Loss on E  (0.01%, 4 PSMs)
- +43.9898  Carboxylation on D  (0.00%, 3 PSMs)
- +27.9949  Formylation on K  (0.00%, 3 PSMs)
- +87.9909  Carbamyl on X, Nitrosylation on Y  (0.00%, 3 PSMs)
- +69.9142  Fe[II] on D, Oxidation on M  (0.00%, 3 PSMs)
- +95.9612  Oxidation on M, Phosphorylation on S  (0.00%, 2 PSMs)
- +406.1587  HexNAc on S, HexNAc on T  (0.00%, 2 PSMs)
- +37.9469  Calcium on D  (0.00%, 2 PSMs)

_MetaMorpheus PSMs excluded as ambiguous (Full Sequence had '|', not dropped silently): 1335/71896 (1.86%)._

_MetaMorpheus mass error (Mass Diff ppm), Phase 8.6 population check — all confident targets: n=70561, median=0.810 ppm, MAD=0.690; unmodified-only: n=54952, median=0.820 ppm, MAD=0.680._

### serum  (Sage-Recon vs MetaMorpheus (GPTMD-confirmed, reallyOpen tier))

- Matched (both, within 0.015 Da): **17**
- Sage-Recon only: **31**
- MetaMorpheus (GPTMD-confirmed, reallyOpen tier) only: **31**
- Spearman ρ (% matched): **0.820** (p=5.65e-05, n=17)
- Top-10 overlap (by % in matched set): **2/10**
- _Window coverage check: Sage-Recon peaks >+100 Da in window: 11 (Sage total >+100: 11, other tool total >+100: 3)_

| mass (Da) | Sage-Recon label | Recon % | Recon rank | MetaMorpheus (GPTMD-confirmed, reallyOpen tier) label | MetaMorpheus (GPTMD-confirmed, reallyOpen tier) % | MetaMorpheus (GPTMD-confirmed, reallyOpen tier) rank |
|---|---|---|---|---|---|---|
| +0.0000 | Unmodified | 32.24 | 1 | Unmodified | 56.54 | 1 |
| +57.0237 | Carbamidomethyl | 7.31 | 2 | Carbamidomethyl on C | 24.51 | 2 |
| +52.9130 | Cation:Fe[III] | 1.90 | 3 | Fe[III] on E | 2.82 | 5 |
| +15.9951 | Oxidation | 1.85 | 4 | Oxidation on M | 6.27 | 3 |
| +0.9845 | Deamidated | 1.20 | 5 | Deamidation on N | 3.17 | 4 |
| +31.9907 | Dioxidation | 1.17 | 6 | Oxidation on M, Oxidation on M | 0.50 | 9 |
| +73.0186 | UNANNOTATED | 0.94 | 7 | Carbamidomethyl on C, Oxidation on M | 1.35 | 6 |
| +114.0457 | GG | 0.86 | 8 | Carbamidomethyl on C, Carbamidomethyl on C | 0.24 | 12 |
| +58.0128 | Carboxymethyl | 0.60 | 9 | Carbamidomethyl on C, Citrullination on R | 0.09 | 14 |
| -18.0101 | Dehydrated | 0.51 | 10 | Water Loss on E | 0.17 | 13 |
| -17.0285 | Gln-&gt;pyro-Glu | 0.38 | 11 | Ammonia loss on N | 0.65 | 7 |
| +53.9156 | Cation:Fe[II] | 0.38 | 12 | Fe[II] on D | 0.44 | 11 |
| +14.0149 | Methyl | 0.34 | 13 | Methylation on R | 0.58 | 8 |
| -1.0296 | Lys-&gt;Allysine | 0.29 | 14 | Ammonia loss on N, Hydroxylation on P | 0.03 | 16 |
| +21.9815 | Cation:Na | 0.21 | 15 | Sodium on E | 0.48 | 10 |
| +37.9492 | Cation:Ca[II] | 0.18 | 16 | Calcium on E | 0.09 | 15 |
| +71.0379 | Propionamide | 0.16 | 17 | Carbamidomethyl on C, Methylation on R | 0.03 | 17 |

**Sage-Recon only (in window, no match):**
- +58.0260  UNANNOTATED  (2.27%, 350 PSMs)
- +209.0205  CarbamidomethylDTT  (0.90%, 138 PSMs)
- +23.9593  Cation:Al[III]  (0.80%, 123 PSMs)
- +109.9354  UNANNOTATED  (0.77%, 119 PSMs)
- +16.9990  UNANNOTATED  (0.47%, 73 PSMs)
- +59.0276  AEC-MAEC  (0.47%, 73 PSMs)
- +13.9783  Pro-&gt;pyro-Glu  (0.45%, 70 PSMs)
- +115.0479  UNANNOTATED  (0.40%, 61 PSMs)
- +162.0542  UNANNOTATED  (0.38%, 58 PSMs)
- +80.9812  Arg-&gt;Npo  (0.35%, 54 PSMs)
- -0.9786  Glu-&gt;pyro-Glu+Methyl:2H(2)13C(1)  (0.34%, 53 PSMs)
- +110.9386  UNANNOTATED  (0.34%, 52 PSMs)
- -14.0159  UNANNOTATED  (0.32%, 50 PSMs)
- +89.0121  Gly+O(2)  (0.32%, 49 PSMs)
- +171.0690  UNANNOTATED  (0.30%, 46 PSMs)
- +74.0213  UNANNOTATED  (0.29%, 44 PSMs)
- +32.9925  UNANNOTATED  (0.29%, 44 PSMs)
- +55.0110  UNANNOTATED  (0.27%, 42 PSMs)
- +210.0244  UNANNOTATED  (0.24%, 37 PSMs)
- -1.0568  UNANNOTATED  (0.24%, 37 PSMs)

**MetaMorpheus (GPTMD-confirmed, reallyOpen tier) only (in window, no match):**
- +43.0058  Carbamyl on K  (0.30%, 29 PSMs)
- +27.9949  Formylation on K  (0.24%, 23 PSMs)
- +44.9851  Nitrosylation on Y  (0.20%, 19 PSMs)
- +1.9680  Deamidation on N, Deamidation on N  (0.17%, 16 PSMs)
- +53.8955  Deamidation on N, Fe[III] on D  (0.12%, 11 PSMs)
- +114.0317  Glutarylation on K  (0.10%, 10 PSMs)
- +42.0106  Acetylation on K  (0.10%, 10 PSMs)
- +16.9789  Deamidation on N, Oxidation on M  (0.10%, 10 PSMs)
- +43.9898  Carboxylation on E  (0.09%, 9 PSMs)
- +100.0273  Carbamidomethyl on C, Carbamyl on M  (0.06%, 6 PSMs)
- -16.0425  Ammonia loss on N, Deamidation on N  (0.05%, 5 PSMs)
- +68.9064  Fe[III] on E, Oxidation on M  (0.05%, 5 PSMs)
- +28.0313  Dimethylation on R  (0.05%, 5 PSMs)
- +79.9663  Phosphorylation on S  (0.04%, 4 PSMs)
- +21.9694  Magnesium on D  (0.04%, 4 PSMs)
- +37.9769  Hydroxylation on N, Sodium on E  (0.04%, 4 PSMs)
- +45.9691  Deamidation on Q, Nitrosylation on Y  (0.04%, 4 PSMs)
- +79.0034  Carbamidomethyl on C, Sodium on E  (0.03%, 3 PSMs)
- +14.9997  Deamidation on Q, Methylation on K  (0.03%, 3 PSMs)
- +99.0320  Acetylation on K, Carbamidomethyl on C  (0.02%, 2 PSMs)

_MetaMorpheus PSMs excluded as ambiguous (Full Sequence had '|', not dropped silently): 251/9816 (2.56%)._

_MetaMorpheus mass error (Mass Diff ppm), Phase 8.6 population check — all confident targets: n=9565, median=0.500 ppm, MAD=0.510; unmodified-only: n=5408, median=0.530 ppm, MAD=0.480._

### b1906  (Sage-Recon vs MetaMorpheus (GPTMD-confirmed, reallyOpen tier))

- Matched (both, within 0.015 Da): **17**
- Sage-Recon only: **31**
- MetaMorpheus (GPTMD-confirmed, reallyOpen tier) only: **24**
- Spearman ρ (% matched): **0.630** (p=0.00674, n=17)
- Top-10 overlap (by % in matched set): **5/10**
- _Window coverage check: Sage-Recon peaks >+100 Da in window: 8 (Sage total >+100: 8, other tool total >+100: 6)_

| mass (Da) | Sage-Recon label | Recon % | Recon rank | MetaMorpheus (GPTMD-confirmed, reallyOpen tier) label | MetaMorpheus (GPTMD-confirmed, reallyOpen tier) % | MetaMorpheus (GPTMD-confirmed, reallyOpen tier) rank |
|---|---|---|---|---|---|---|
| +0.0000 | Unmodified | 51.17 | 1 | Unmodified | 70.95 | 1 |
| +57.0219 | Carbamidomethyl | 4.47 | 2 | Carbamidomethyl on C | 11.58 | 2 |
| +15.9949 | Oxidation | 2.67 | 3 | Oxidation on M | 7.98 | 3 |
| +43.0058 | Carbamyl | 1.98 | 4 | Carbamyl on X | 3.55 | 4 |
| +53.9186 | Cation:Fe[II] | 1.22 | 5 | Fe[II] on E | 1.64 | 5 |
| +0.9818 | Deamidated | 0.81 | 6 | Deamidation on N | 1.13 | 6 |
| +27.9946 | Formyl | 0.69 | 7 | Formylation on K | 0.46 | 7 |
| +73.0161 | UNANNOTATED | 0.41 | 8 | Carbamidomethyl on C, Hydroxylation on P | 0.00 | 17 |
| -1.0225 | Lys-&gt;Allysine | 0.40 | 9 | Ammonia loss on N, Oxidation on M | 0.01 | 16 |
| +1.9676 | UNANNOTATED | 0.32 | 10 | Deamidation on Q, Deamidation on Q | 0.02 | 13 |
| +114.0426 | GG | 0.27 | 11 | Carbamidomethyl on C, Carbamidomethyl on C | 0.02 | 14 |
| +110.9403 | UNANNOTATED | 0.24 | 12 | Carbamidomethyl on C, Fe[II] on D | 0.02 | 15 |
| +31.9895 | Dioxidation | 0.21 | 13 | Oxidation on M, Oxidation on M | 0.41 | 8 |
| +37.9463 | Cation:Ca[II] | 0.21 | 14 | Calcium on D | 0.07 | 12 |
| +21.9798 | Cation:Na | 0.16 | 15 | Sodium on E | 0.21 | 10 |
| -18.0113 | Dehydrated | 0.15 | 16 | Water Loss on E | 0.08 | 11 |
| -17.0266 | Gln-&gt;pyro-Glu | 0.15 | 17 | Ammonia loss on N | 0.23 | 9 |

**Sage-Recon only (in window, no match):**
- +58.0237  UNANNOTATED  (0.89%, 248 PSMs)
- +16.9978  UNANNOTATED  (0.67%, 187 PSMs)
- +301.9863  Unknown:302  (0.66%, 186 PSMs)
- -1.0621  UNANNOTATED  (0.39%, 109 PSMs)
- +0.9505  UNANNOTATED  (0.38%, 107 PSMs)
- +0.9300  UNANNOTATED  (0.36%, 102 PSMs)
- +1.9484  UNANNOTATED  (0.29%, 80 PSMs)
- +183.0353  AEBS  (0.28%, 79 PSMs)
- -1.0793  UNANNOTATED  (0.28%, 78 PSMs)
- -1.9715  UNANNOTATED  (0.27%, 76 PSMs)
- +59.0277  AEC-MAEC  (0.27%, 75 PSMs)
- -2.0404  UNANNOTATED  (0.27%, 75 PSMs)
- -0.0997  Unmodified  (0.27%, 75 PSMs)
- +54.9208  UNANNOTATED  (0.25%, 71 PSMs)
- +128.0949  Lys  (0.24%, 68 PSMs)
- -2.0871  UNANNOTATED  (0.24%, 66 PSMs)
- -1.0983  UNANNOTATED  (0.22%, 63 PSMs)
- -1.9492  UNANNOTATED  (0.22%, 63 PSMs)
- +1.9197  UNANNOTATED  (0.20%, 56 PSMs)
- +44.0092  Delta:H(4)C(2)O(-1)S(1)  (0.20%, 55 PSMs)

**MetaMorpheus (GPTMD-confirmed, reallyOpen tier) only (in window, no match):**
- +42.0106  Acetylation on X  (0.63%, 173 PSMs)
- +79.9663  Phosphorylation on S  (0.40%, 110 PSMs)
- +52.9115  Fe[III] on E  (0.11%, 31 PSMs)
- +16.9789  Deamidation on N, Oxidation on M  (0.11%, 30 PSMs)
- +114.0317  Glutarylation on K  (0.06%, 16 PSMs)
- +28.0313  Dimethylation on R  (0.05%, 14 PSMs)
- +43.9898  Carboxylation on D  (0.05%, 13 PSMs)
- +14.0157  Methylation on R  (0.04%, 11 PSMs)
- +203.0794  HexNAc on T  (0.04%, 10 PSMs)
- +69.9142  Fe[II] on D, Oxidation on M  (0.04%, 10 PSMs)
- +59.0007  Carbamyl on X, Oxidation on M  (0.03%, 8 PSMs)
- +37.9559  Potassium on D  (0.02%, 5 PSMs)
- +109.9329  Carbamidomethyl on C, Fe[III] on E  (0.02%, 5 PSMs)
- +21.9694  Magnesium on D  (0.01%, 4 PSMs)
- +38.9399  Deamidation on Q, Potassium on E  (0.01%, 2 PSMs)
- +121.9769  Acetylation on X, Phosphorylation on S  (0.00%, 1 PSMs)
- +71.0007  Carbamyl on X, Formylation on K  (0.00%, 1 PSMs)
- +86.0116  Carbamyl on K, Carbamyl on X  (0.00%, 1 PSMs)
- +22.9660  Deamidation on Q, Sodium on E  (0.00%, 1 PSMs)
- +95.9173  Carbamyl on X, Fe[III] on E  (0.00%, 1 PSMs)

_MetaMorpheus PSMs excluded as ambiguous (Full Sequence had '|', not dropped silently): 941/28479 (3.30%)._

_MetaMorpheus mass error (Mass Diff ppm), Phase 8.6 population check — all confident targets: n=27538, median=1.060 ppm, MAD=0.850; unmodified-only: n=19538, median=1.110 ppm, MAD=0.835._
