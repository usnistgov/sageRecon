# Satellite-recovery check (Recon-normalized percentages)

Read-only accounting. Percentages are kept on Recon's supplied scale: satellite % is calculated from the denominator inferred from Recon's own count/pct fields, and augmented % = raw % + satellite %.

### bcell

Recon percentage denominator inferred from count/pct rows: 73527.00 PSMs. Largest per-row estimate deviation: 0.00% (expected from rounded displayed percentages).

| mass (Da) | label | raw % | satellite PSMs | satellite % | augmented % | satellites claimed | PTM-Shep % | Mascot % | MetaMorph % | raw ratio* | augmented ratio* |
|---|---|---:|---:|---:|---:|---|---:|---:|---:|---|---|
| +57.0220 | Carbamidomethyl | 4.50 | 938 | 1.28 | 5.78 | k=+1 +58.0243 (938 PSM; Δ=1.1 mDa) | 10.68 | 33.76 | 11.46 | 0.42/0.13/0.39 | 0.54/0.17/0.50 |
| +15.9953 | Oxidation | 1.92 | 389 | 0.53 | 2.45 | k=+1 +16.9975 (389 PSM; Δ=1.1 mDa) | 5.53 | 13.32 | 5.35 | 0.35/0.14/0.36 | 0.44/0.18/0.46 |
| +0.9821 | Deamidated | 1.03 | 241 | 0.33 | 1.36 | k=+1 +1.9855 (241 PSM; Δ=0.1 mDa) | 1.25 | 3.45 | 1.28 | 0.83/0.30/0.81 | 1.09/0.39/1.06 |
| -1.0290 | Lys-&gt;Allysine | 0.51 | 296 | 0.40 | 0.91 | k=+3 +1.9691 (296 PSM; Δ=12.0 mDa) | 0.17 | 0.02 | 0.02 | 2.94/21.85/27.76 | 5.25/39.05/49.61 |
| +52.9113 | Cation:Fe[III] | 0.49 | 0 | 0.00 | 0.49 | — | 0.53 | — | 0.65 | 0.92/0.75 | 0.92/0.75 |
| -0.9804 | Glu-&gt;pyro-Glu+Methyl:2H(2)13C(1) | 0.41 | 148 | 0.20 | 0.61 | k=+3 +2.0305 (148 PSM; Δ=0.8 mDa) | 0.01 | 0.01 | — | 34.34/35.22 | 51.12/52.42 |
| +59.0280 | AEC-MAEC | 0.40 | 0 | 0.00 | 0.40 | — | 0.20 | 0.53 | — | 1.95/0.75 | 1.95/0.75 |
| -17.0257 | Gln-&gt;pyro-Glu | 0.34 | 0 | 0.00 | 0.34 | — | 0.82 | 2.04 | 0.28 | 0.42/0.17/1.25 | 0.42/0.17/1.25 |
| -0.0788 | Unmodified | 0.31 | 704 | 0.96 | 1.26 | k=-1 -1.0871 (147 PSM; Δ=5.0 mDa); k=+1 +0.9303 (272 PSM; Δ=5.8 mDa); k=+2 +1.9196 (205 PSM; Δ=8.3 mDa); k=+3 +2.9108 (80 PSM; Δ=20.4 mDa) | — | — | — | — | — |
| -89.0289 | Met-loss+Acetyl | 0.28 | 0 | 0.00 | 0.28 | — | 0.03 | 0.25 | — | 9.80/1.13 | 9.80/1.13 |
| +114.0432 | GG | 0.28 | 99 | 0.13 | 0.41 | k=+1 +115.0481 (99 PSM; Δ=1.6 mDa) | 1.27 | 0.35 | 0.01 | 0.22/0.79/19.48 | 0.32/1.17/28.98 |
| +53.9139 | Cation:Fe[II] | 0.25 | 0 | 0.00 | 0.25 | — | 0.03 | 0.25 | 0.12 | 8.84/1.01/1.98 | 8.84/1.01/1.98 |
| -0.0995 | Unmodified | 0.22 | 486 | 0.66 | 0.88 | k=-1 -1.1068 (111 PSM; Δ=4.0 mDa); k=+1 +0.9093 (209 PSM; Δ=5.4 mDa); k=+2 +1.9028 (166 PSM; Δ=4.5 mDa) | — | — | — | — | — |
| +31.9901 | Dioxidation | 0.21 | 0 | 0.00 | 0.21 | — | 0.62 | 1.04 | 0.46 | 0.35/0.21/0.46 | 0.35/0.21/0.46 |
| +18.0012 | Pro-&gt;HAVA | 0.20 | 0 | 0.00 | 0.20 | — | 0.06 | 0.19 | — | 3.35/1.08 | 3.35/1.08 |

Claimed 14 UNANNOTATED peak(s), totaling 3408 PSMs. Total UNANNOTATED PSMs: 5226.

_*Ratio order: PTM-Shepherd / Mascot / MetaMorpheus. A comparator without a mass-window match is omitted. This is read-only sensitivity accounting, not proof that every claimed peak is chemically an isotope satellite._

### serum

Recon percentage denominator inferred from count/pct rows: 15386.00 PSMs. Largest per-row estimate deviation: 0.00% (expected from rounded displayed percentages).

| mass (Da) | label | raw % | satellite PSMs | satellite % | augmented % | satellites claimed | PTM-Shep % | Mascot % | MetaMorph % | raw ratio* | augmented ratio* |
|---|---|---:|---:|---:|---:|---|---:|---:|---:|---|---|
| +57.0237 | Carbamidomethyl | 7.31 | 392 | 2.55 | 9.86 | k=-2 +55.0110 (42 PSM; Δ=6.0 mDa); k=+1 +58.0260 (350 PSM; Δ=1.1 mDa) | 17.92 | 34.16 | 24.51 | 0.41/0.21/0.30 | 0.55/0.29/0.40 |
| +52.9130 | Cation:Fe[III] | 1.90 | 0 | 0.00 | 1.90 | — | 2.90 | — | 2.82 | 0.65/0.67 | 0.65/0.67 |
| +15.9951 | Oxidation | 1.85 | 106 | 0.69 | 2.53 | k=-1 +14.9840 (33 PSM; Δ=7.8 mDa); k=+1 +16.9990 (73 PSM; Δ=0.5 mDa) | 5.28 | 6.40 | 6.27 | 0.35/0.29/0.29 | 0.48/0.40/0.40 |
| +0.9845 | Deamidated | 1.20 | 30 | 0.19 | 1.39 | k=+1 +1.9861 (30 PSM; Δ=1.7 mDa) | 1.41 | 3.36 | 3.17 | 0.85/0.36/0.38 | 0.99/0.41/0.44 |
| +31.9907 | Dioxidation | 1.17 | 44 | 0.29 | 1.46 | k=+1 +32.9925 (44 PSM; Δ=1.5 mDa) | 1.82 | 2.19 | 0.50 | 0.64/0.53/2.33 | 0.80/0.67/2.90 |
| +209.0205 | CarbamidomethylDTT | 0.90 | 37 | 0.24 | 1.14 | k=+1 +210.0244 (37 PSM; Δ=0.5 mDa) | 2.09 | 2.55 | — | 0.43/0.35 | 0.54/0.45 |
| +114.0457 | GG | 0.86 | 61 | 0.40 | 1.25 | k=+1 +115.0479 (61 PSM; Δ=1.1 mDa) | 3.27 | 0.41 | 0.24 | 0.26/2.10/3.57 | 0.38/3.06/5.22 |
| +23.9593 | Cation:Al[III] | 0.80 | 0 | 0.00 | 0.80 | — | 0.88 | — | — | 0.91 | 0.91 |
| +58.0128 | Carboxymethyl | 0.60 | 27 | 0.18 | 0.78 | k=-2 +56.0068 (27 PSM; Δ=0.7 mDa) | 0.85 | 1.80 | 0.09 | 0.71/0.34/6.42 | 0.92/0.43/8.29 |
| -18.0101 | Dehydrated | 0.51 | 0 | 0.00 | 0.51 | — | 1.28 | 1.57 | 0.17 | 0.40/0.33/3.07 | 0.40/0.33/3.07 |
| +59.0276 | AEC-MAEC | 0.47 | 0 | 0.00 | 0.47 | — | 0.10 | 0.68 | — | 4.74/0.70 | 4.74/0.70 |
| +13.9783 | Pro-&gt;pyro-Glu | 0.45 | 0 | 0.00 | 0.45 | — | 1.10 | 0.79 | — | 0.41/0.57 | 0.41/0.57 |
| -17.0285 | Gln-&gt;pyro-Glu | 0.38 | 50 | 0.32 | 0.71 | k=+3 -14.0159 (50 PSM; Δ=2.6 mDa) | 1.20 | 1.23 | 0.65 | 0.32/0.31/0.59 | 0.59/0.58/1.09 |
| +53.9156 | Cation:Fe[II] | 0.38 | 0 | 0.00 | 0.38 | — | 0.24 | 0.45 | 0.44 | 1.57/0.84/0.86 | 1.57/0.84/0.86 |
| +80.9812 | Arg-&gt;Npo | 0.35 | 0 | 0.00 | 0.35 | — | 0.53 | 0.03 | — | 0.67/13.72 | 0.67/13.72 |

Claimed 10 UNANNOTATED peak(s), totaling 747 PSMs. Total UNANNOTATED PSMs: 1304.

_*Ratio order: PTM-Shepherd / Mascot / MetaMorpheus. A comparator without a mass-window match is omitted. This is read-only sensitivity accounting, not proof that every claimed peak is chemically an isotope satellite._

### b1906

Recon percentage denominator inferred from count/pct rows: 28005.00 PSMs. Largest per-row estimate deviation: 0.00% (expected from rounded displayed percentages).

| mass (Da) | label | raw % | satellite PSMs | satellite % | augmented % | satellites claimed | PTM-Shep % | Mascot % | MetaMorph % | raw ratio* | augmented ratio* |
|---|---|---:|---:|---:|---:|---|---:|---:|---:|---|---|
| +57.0219 | Carbamidomethyl | 4.47 | 248 | 0.89 | 5.36 | k=+1 +58.0237 (248 PSM; Δ=1.5 mDa) | 10.77 | 25.64 | 11.58 | 0.42/0.17/0.39 | 0.50/0.21/0.46 |
| +15.9949 | Oxidation | 2.67 | 187 | 0.67 | 3.34 | k=+1 +16.9978 (187 PSM; Δ=0.4 mDa) | 7.68 | 18.48 | 7.98 | 0.35/0.14/0.33 | 0.43/0.18/0.42 |
| +43.0058 | Carbamyl | 1.98 | 0 | 0.00 | 1.98 | — | 2.55 | 7.25 | 3.55 | 0.78/0.27/0.56 | 0.78/0.27/0.56 |
| +53.9186 | Cation:Fe[II] | 1.22 | 71 | 0.25 | 1.47 | k=+1 +54.9208 (71 PSM; Δ=1.1 mDa) | 1.97 | 2.16 | 1.64 | 0.62/0.56/0.74 | 0.75/0.68/0.89 |
| +0.9818 | Deamidated | 0.81 | 75 | 0.27 | 1.08 | k=-3 -2.0404 (75 PSM; Δ=12.1 mDa) | 0.78 | 2.23 | 1.13 | 1.05/0.37/0.72 | 1.39/0.49/0.96 |
| +27.9946 | Formyl | 0.69 | 0 | 0.00 | 0.69 | — | 1.52 | 4.12 | 0.46 | 0.45/0.17/1.49 | 0.45/0.17/1.49 |
| +301.9863 | Unknown:302 | 0.66 | 29 | 0.10 | 0.77 | k=+1 +302.9880 (29 PSM; Δ=1.6 mDa) | 0.84 | — | — | 0.79 | 0.91 |
| -1.0225 | Lys-&gt;Allysine | 0.40 | 89 | 0.32 | 0.72 | k=+3 +1.9676 (89 PSM; Δ=19.9 mDa) | 0.04 | 0.05 | 0.01 | 9.52/8.45/55.07 | 17.09/15.16/98.82 |
| +183.0353 | AEBS | 0.28 | 0 | 0.00 | 0.28 | — | 1.26 | 3.30 | — | 0.22/0.09 | 0.22/0.09 |
| +114.0426 | GG | 0.27 | 0 | 0.00 | 0.27 | — | 1.05 | 0.36 | 0.02 | 0.26/0.75/14.95 | 0.26/0.75/14.95 |
| -0.0997 | Unmodified | 0.27 | 204 | 0.73 | 1.00 | k=-2 -2.1091 (34 PSM; Δ=2.7 mDa); k=-1 -1.0983 (63 PSM; Δ=4.7 mDa); k=+1 +0.8970 (51 PSM; Δ=6.7 mDa); k=+2 +1.9197 (56 PSM; Δ=12.7 mDa) | — | — | — | — | — |
| +59.0277 | AEC-MAEC | 0.27 | 0 | 0.00 | 0.27 | — | 0.11 | 0.13 | — | 2.43/2.12 | 2.43/2.12 |
| +128.0949 | Lys | 0.24 | 0 | 0.00 | 0.24 | — | 0.08 | 0.62 | — | 2.93/0.39 | 2.93/0.39 |
| +31.9895 | Dioxidation | 0.21 | 0 | 0.00 | 0.21 | — | 0.65 | 0.95 | 0.41 | 0.32/0.22/0.51 | 0.32/0.22/0.51 |
| +37.9463 | Cation:Ca[II] | 0.21 | 0 | 0.00 | 0.21 | — | 0.33 | 0.05 | 0.07 | 0.65/4.45/3.22 | 0.65/4.45/3.22 |

Claimed 10 UNANNOTATED peak(s), totaling 903 PSMs. Total UNANNOTATED PSMs: 1966.

_*Ratio order: PTM-Shepherd / Mascot / MetaMorpheus. A comparator without a mass-window match is omitted. This is read-only sensitivity accounting, not proof that every claimed peak is chemically an isotope satellite._
