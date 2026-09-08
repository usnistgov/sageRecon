# Four-tool mod-discovery comparison (Recon / PTM-Shepherd reallyOpen / Mascot ET / MetaMorpheus reallyOpen)

Single global clustering across all four tools (not recon-anchored pairwise) — a mass can be "discovered" by whichever tool has the biggest peak there. Match tolerance 0.015 Da, window [-100.0, 500.0] Da. Rank (#N) is each tool's own rank among ITS rows in-window, not the cluster rank.

### bcell

| mass (Da) | Recon % | PTM-Shepherd % | Mascot % | MetaMorpheus % | Recon label | PTM-Shepherd label | Mascot label | MetaMorpheus label |
|---|---|---|---|---|---|---|---|---|
| +0.0000 | 58.29 (#1) | 63.03 (#1) | — | 77.88 (#1) | Unmodified | None | — | Unmodified |
| +57.0215 | 4.50 (#2) | 10.68 (#2) | 33.76 (#1) | 11.46 (#2) | Carbamidomethyl | Iodoacetamide derivative/Addition of Glycine/Addition of G | Carbamidomethyl [A,C,D,E,G,H,K,N-term,S,T,U,Y] | Carbamidomethyl on C |
| +0.9970 | — | 3.37 (#4) | 20.36 (#2) | 1.28 (#5) | — | First isotopic peak | Label:15N(1) [A,D,E,F,G,I,L,M,P,S,T,V,Y] | Deamidation on N |
| +15.9949 | 1.92 (#3) | 5.53 (#3) | 13.32 (#3) | 5.35 (#3) | Oxidation | Oxidation or Hydroxylation | Oxidation [A,C,D,F,K,M,N,P,R,W,Y] | Oxidation on M |
| +0.9840 | 1.03 (#5) | 1.25 (#7) | 3.45 (#4) | — | Deamidated | Deamidation | Deamidated [N,Q,R] | — |
| +42.0106 | 0.16 (#34) | 1.27 (#5) | 3.18 (#5) | 1.34 (#4) | Acetyl | Acetylation | Acetyl [C,H,K,N-term,S,T,Y] | Acetylation on X |
| -17.0265 | 0.34 (#14) | 0.82 (#8) | 2.04 (#6) | 0.28 (#9) | Gln-&gt;pyro-Glu | Pyro-glu from Q/Loss of ammonia | Gln->pyro-Glu [N,N-term] | Ammonia loss on N |
| +58.0419 | — | 0.01 (#95) | 1.64 (#7) | — | — | Reduced acrolein addition +58 | Delta:H(6)C(3)O(1) [C,H,K] | — |
| +58.0055 | — | 0.23 (#20) | 1.37 (#8) | 0.01 (#25) | — | Iodoacetic acid derivative | Carboxymethyl [A,C,G,K,N-term] | Carbamidomethyl on C, Deamidation on Q |
| +58.0243 | 1.28 (#4) | 0.41 (#13) | 0.21 (#29) | — | UNANNOTATED | 2,3-dihydro-2,2-dimethyl-7-benzofuranol N-methyl carbamate | Gln->Trp [Q] | — |
| +114.0429 | 0.28 (#21) | 1.27 (#6) | 0.35 (#19) | 0.07 (#13) | GG | ubiquitinylation residue/Double Carbamidomethylation/Addition of N | GG [C,K,N-term,S,T] | Glutarylation on K |
| +31.9898 | 0.21 (#27) | 0.62 (#10) | 1.04 (#9) | 0.46 (#7) | Dioxidation | dihydroxy | Dioxidation [C,F,M,R,W,Y] | Oxidation on M, Oxidation on M |
| +39.9949 | — | 0.34 (#18) | 0.92 (#10) | — | — | S-carbamoylmethylcysteine cyclization (N-terminus)/Glyoxal-derived hydroimiadazolone | Pyro-carbamidomethyl [N-term,R] | — |
| -18.0106 | 0.11 (#45) | 0.35 (#17) | 0.85 (#11) | 0.14 (#11) | Dehydrated | Dehydration/Pyro-glu from E | Dehydrated [D,N-term,S,T,Y] | Water Loss on E |
| +0.0186 | — | 0.79 (#9) | — | — | — | Unannotated mass-shift 0.0186 | — | — |
| +2.0047 | — | 0.61 (#11) | 0.74 (#12) | — | — | Second isotopic peak | Label:18O(1) [C-term,S,T,Y] | — |
| +52.9115 | 0.49 (#9) | 0.53 (#12) | — | 0.65 (#6) | Cation:Fe[III] | Replacement of 3 protons by iron | — | Fe[III] on E |
| +43.0058 | 0.10 (#46) | 0.14 (#26) | 0.65 (#13) | 0.21 (#10) | Carbamyl | Carbamylation | Carbamyl [A,C,K,M,N-term,R,S,T,Y] | Carbamyl on X |
| +1.9855 | 0.33 (#15) | — | 0.63 (#14) | — | UNANNOTATED | — | Label:15N(2) [K,N,Q,W] | — |
| +79.9663 | 0.11 (#42) | 0.21 (#22) | 0.57 (#15) | 0.34 (#8) | Phospho | Phosphorylation | Phospho [S,T,Y] | Phosphorylation on S |
| +59.0280 | 0.40 (#12) | 0.20 (#23) | 0.53 (#16) | — | AEC-MAEC | Propionate labeling reagent heavy form (+3amu), N-term  K | Propionyl:13C(3) [K,N-term] | — |
| +16.9975 | 0.53 (#6) | — | 0.33 (#22) | — | UNANNOTATED | — | Pro->Asn [P] | — |
| +0.9509 | 0.52 (#7) | — | 0.19 (#32) | — | UNANNOTATED | — | Xle->Asn [I,L] | — |
| -1.0290 | 0.51 (#8) | 0.17 (#25) | 0.02 (#142) | 0.02 (#22) | Lys-&gt;Allysine | Lysine oxidation to aminoadipic semialdehyde | Lys->Allysine [K] | Ammonia loss on N, Oxidation on M |
| +14.0157 | — | 0.19 (#24) | 0.43 (#17) | 0.05 (#14) | — | Methylation | Val->Xle [D,E,G,H,I,K,L,N-term,R,S,T,V] | Methylation on K |
| -0.9804 | 0.41 (#10) | 0.08 (#32) | 0.08 (#64) | — | Glu-&gt;pyro-Glu+Methyl:2H(2)13C(1) | Amidation | Glu->Gln [C-term,D,E] | — |
| -1.0024 | — | 0.41 (#14) | — | — | — | Isotopic peak error | — | — |
| +17.0345 | 0.19 (#33) | 0.39 (#15) | 0.40 (#18) | — | Ammonium | deuterated methyl ester | Methyl:2H(3) [C-term,D,E,K] | — |
| +1.9691 | 0.40 (#11) | — | 0.09 (#60) | 0.05 (#16) | UNANNOTATED | — | Val->Thr [V] | Deamidation on N, Deamidation on N |
| +0.9303 | 0.37 (#13) | — | — | — | UNANNOTATED | — | — | — |

_460 total clusters in window; 358 found by exactly one tool (shown above only if in the top 30 by max %; full singleton list omitted here)._

### serum

| mass (Da) | Recon % | PTM-Shepherd % | Mascot % | MetaMorpheus % | Recon label | PTM-Shepherd label | Mascot label | MetaMorpheus label |
|---|---|---|---|---|---|---|---|---|
| +0.0000 | 32.24 (#1) | 31.28 (#1) | — | 56.54 (#1) | Unmodified | None | — | Unmodified |
| +57.0215 | 7.31 (#2) | 17.92 (#2) | 34.16 (#1) | 24.51 (#2) | Carbamidomethyl | Iodoacetamide derivative/Addition of Glycine/Addition of G | Carbamidomethyl [A,C,D,E,G,H,K,N-term,S,T,Y] | Carbamidomethyl on C |
| +0.9970 | — | 3.22 (#5) | 9.12 (#2) | 3.17 (#4) | — | First isotopic peak | Label:15N(1) [A,C,D,E,F,G,I,L,M,P,S,T,V,Y] | Deamidation on N |
| +15.9949 | 1.85 (#5) | 5.28 (#3) | 6.40 (#3) | 6.27 (#3) | Oxidation | Oxidation or Hydroxylation | Oxidation [A,D,F,K,M,N,P,R,W,Y] | Oxidation on M |
| +0.9840 | 1.20 (#6) | 1.41 (#9) | 3.36 (#4) | — | Deamidated | Deamidation | Deamidated [N,Q,R] | — |
| +114.0429 | 0.86 (#10) | 3.27 (#4) | 0.41 (#27) | 0.24 (#13) | GG | ubiquitinylation residue/Double Carbamidomethylation/Addition of N | GG [C,K,N-term,S,T] | Carbamidomethyl on C, Carbamidomethyl on C |
| +58.0419 | — | 0.04 (#91) | 3.20 (#5) | — | — | Reduced acrolein addition +58 | Delta:H(6)C(3)O(1) [C,H,K] | — |
| +52.9115 | 1.90 (#4) | 2.90 (#6) | — | 2.82 (#5) | Cation:Fe[III] | Replacement of 3 protons by iron | — | Fe[III] on E |
| +209.0180 | 0.90 (#9) | 2.09 (#7) | 2.55 (#6) | — | CarbamidomethylDTT | Carbamidomethylated DTT modification of cysteine | CarbamidomethylDTT [C] | — |
| +58.0260 | 2.27 (#3) | 1.30 (#10) | 0.22 (#53) | — | UNANNOTATED | 2,3-dihydro-2,2-dimethyl-7-benzofuranol N-methyl carbamate | Gln->Trp [Q] | — |
| +31.9898 | 1.17 (#7) | 1.82 (#8) | 2.19 (#7) | 0.50 (#9) | Dioxidation | dihydroxy | Dioxidation [C,F,M,P,R,W,Y] | Oxidation on M, Oxidation on M |
| +58.0055 | 0.60 (#13) | 0.85 (#15) | 1.80 (#8) | 0.09 (#22) | Carboxymethyl | Iodoacetic acid derivative | Carboxymethyl [A,C,G,K,N-term,W] | Carbamidomethyl on C, Citrullination on R |
| -18.0106 | 0.51 (#14) | 1.28 (#11) | 1.57 (#9) | 0.17 (#17) | Dehydrated | Dehydration/Pyro-glu from E | Dehydrated [D,N-term,S,T,Y] | Water Loss on E |
| +14.0157 | 0.34 (#25) | 0.71 (#18) | 1.50 (#10) | 0.58 (#8) | Methyl | Methylation | Methyl [C,D,E,G,I,K,L,N,N-term,Q,R,S,T,V] | Methylation on R |
| +73.0186 | 0.94 (#8) | — | — | 1.35 (#6) | UNANNOTATED | — | — | Carbamidomethyl on C, Oxidation on M |
| -17.0265 | 0.38 (#19) | 1.20 (#12) | 1.23 (#11) | 0.65 (#7) | Gln-&gt;pyro-Glu | Pyro-glu from Q/Loss of ammonia | Gln->pyro-Glu [N,N-term] | Ammonia loss on N |
| +13.9793 | 0.45 (#17) | 1.10 (#13) | 0.79 (#13) | — | Pro-&gt;pyro-Glu | proline oxidation to pyroglutamic acid/Tryptophan oxidation to oxolactone/aldehyde and ketone modifications | Trp->Oxolactone [P,T,W] | — |
| -14.0157 | 0.32 (#26) | — | 0.95 (#12) | — | UNANNOTATED | — | Xle->Val [A,E,I,L,T] | — |
| +23.9581 | 0.80 (#11) | 0.88 (#14) | — | — | Cation:Al[III] | Replacement of 3 protons by aluminium | — | — |
| +109.9354 | 0.77 (#12) | — | — | — | UNANNOTATED | — | — | — |
| +21.9819 | 0.21 (#40) | 0.77 (#16) | 0.59 (#17) | 0.48 (#10) | Cation:Na | Sodium adduct | Cation:Na [C-term,D,E] | Sodium on E |
| +0.0186 | — | 0.74 (#17) | — | — | — | Unannotated mass-shift 0.0186 | — | — |
| +162.0528 | 0.38 (#21) | 0.64 (#21) | 0.69 (#14) | — | UNANNOTATED | Hexose | Hex [K,N,N-term,S,T,W,Y] | — |
| +3.0071 | — | 0.69 (#19) | 0.01 (#267) | — | — | Third isotopic peak | Label:2H(3) [L] | — |
| +59.0276 | 0.47 (#16) | 0.19 (#46) | 0.68 (#15) | — | AEC-MAEC | Propionate labeling reagent heavy form (+3amu), N-term  K | AEC-MAEC [S,T] | — |
| -2.0156 | — | 0.67 (#20) | 0.26 (#49) | — | — | 2-amino-3-oxo-butanoic_acid | Didehydro [C-term,S,T,V,Y] | — |
| -1.0024 | — | 0.59 (#22) | 0.32 (#33) | — | — | Isotopic peak error | Dehydro [C] | — |
| +23.9748 | — | — | 0.59 (#16) | — | — | — | Xle->His [I,L] | — |
| -33.9877 | — | 0.45 (#25) | 0.58 (#18) | — | — | Dehydroalanine (from Cysteine) | Cys->Dha [C] | — |
| +80.9851 | 0.35 (#22) | 0.53 (#23) | 0.03 (#239) | — | Arg-&gt;Npo | Arginine replacement by Nitropyrimidyl ornithine | Arg->Npo [R] | — |

_429 total clusters in window; 327 found by exactly one tool (shown above only if in the top 30 by max %; full singleton list omitted here)._

### b1906

| mass (Da) | Recon % | PTM-Shepherd % | Mascot % | MetaMorpheus % | Recon label | PTM-Shepherd label | Mascot label | MetaMorpheus label |
|---|---|---|---|---|---|---|---|---|
| +0.0000 | 51.17 (#1) | 56.99 (#1) | — | 70.95 (#1) | Unmodified | None | — | Unmodified |
| +57.0215 | 4.47 (#2) | 10.77 (#2) | 25.64 (#1) | 11.58 (#2) | Carbamidomethyl | Iodoacetamide derivative/Addition of Glycine/Addition of G | Carbamidomethyl [A,C,D,E,G,H,K,N-term,S,T,Y] | Carbamidomethyl on C |
| +15.9949 | 2.67 (#3) | 7.68 (#3) | 18.48 (#2) | 7.98 (#3) | Oxidation | Oxidation or Hydroxylation | Oxidation [A,D,F,K,M,N,P,R,W,Y] | Oxidation on M |
| +0.9970 | — | 2.23 (#5) | 11.16 (#3) | 1.13 (#6) | — | First isotopic peak | Label:15N(1) [A,D,E,F,G,I,L,M,P,S,T,V,Y] | Deamidation on N |
| +43.0058 | 1.98 (#4) | 2.55 (#4) | 7.25 (#4) | 3.55 (#4) | Carbamyl | Carbamylation | Carbamyl [A,K,M,N-term,R,S,T,Y] | Carbamyl on X |
| +27.9949 | 0.69 (#8) | 1.52 (#7) | 4.12 (#5) | 0.46 (#8) | Formyl | Formylation | Formyl [K,N-term,S,T] | Formylation on K |
| +183.0354 | 0.28 (#18) | 1.26 (#8) | 3.30 (#6) | — | AEBS | Aminoethylbenzenesulfonylation | AEBS [K,N-term,S,Y] | — |
| +0.9818 | 0.81 (#7) | 0.78 (#12) | 2.23 (#7) | — | Deamidated | Deamidation | Deamidated [N,Q,R] | — |
| +53.9193 | 1.22 (#5) | 1.97 (#6) | 2.16 (#8) | 1.64 (#5) | Cation:Fe[II] | Replacement of 2 protons by iron | Cation:Fe[II] [D,E] | Fe[II] on E |
| -17.0265 | 0.15 (#41) | 0.42 (#16) | 1.37 (#9) | 0.23 (#11) | Gln-&gt;pyro-Glu | Pyro-glu from Q/Loss of ammonia | Gln->pyro-Glu [N,N-term] | Ammonia loss on N |
| +114.0429 | 0.27 (#20) | 1.05 (#9) | 0.36 (#25) | 0.06 (#17) | GG | ubiquitinylation residue/Double Carbamidomethylation/Addition of N | GG [C,K,N-term,R,S,T] | Glutarylation on K |
| +0.0186 | — | 0.98 (#10) | — | — | — | Unannotated mass-shift 0.0186 | — | — |
| +31.9898 | 0.21 (#31) | 0.65 (#13) | 0.95 (#10) | 0.41 (#9) | Dioxidation | dihydroxy | Dioxidation [W] | Oxidation on M, Oxidation on M |
| +58.0237 | 0.89 (#6) | 0.16 (#27) | 0.06 (#78) | — | UNANNOTATED | 2,3-dihydro-2,2-dimethyl-7-benzofuranol N-methyl carbamate | Gln->Trp [Q] | — |
| +79.9663 | — | 0.32 (#21) | 0.88 (#11) | 0.40 (#10) | — | Phosphorylation | Phospho [D,S,T] | Phosphorylation on S |
| +301.9865 | 0.66 (#10) | 0.84 (#11) | — | — | Unknown:302 | Unidentified modification of 301.9864 found in open search | — | — |
| -18.0106 | 0.15 (#40) | 0.29 (#22) | 0.82 (#12) | 0.08 (#15) | Dehydrated | Dehydration/Pyro-glu from E | Glu->pyro-Glu [D,N-term,S,T,Y] | Water Loss on E |
| +58.0419 | — | 0.01 (#78) | 0.79 (#13) | — | — | Reduced acrolein addition +58 | Delta:H(6)C(3)O(1) [C,H,K] | — |
| +16.9978 | 0.67 (#9) | — | 0.38 (#24) | — | UNANNOTATED | — | Pro->Asn [P] | — |
| +42.0106 | — | 0.42 (#17) | 0.66 (#14) | 0.63 (#7) | — | Acetylation | Acetyl [K,N-term,S,T] | Acetylation on X |
| +128.0949 | 0.24 (#26) | 0.08 (#34) | 0.62 (#15) | — | Lys | Addition of lysine due to transpeptidation/Addition of K | Lys [N-term] | — |
| +58.0055 | — | 0.14 (#28) | 0.60 (#16) | — | — | Iodoacetic acid derivative | Carboxymethyl [A,C,G,N-term] | — |
| +156.1011 | — | 0.08 (#36) | 0.58 (#17) | — | — | Addition of arginine due to transpeptidation/Addition of R | Arg [N-term] | — |
| +2.0047 | — | 0.51 (#14) | 0.54 (#18) | — | — | Second isotopic peak | Label:18O(1) [S,T,Y] | — |
| +39.9949 | — | 0.24 (#23) | 0.51 (#19) | — | — | S-carbamoylmethylcysteine cyclization (N-terminus)/Glyoxal-derived hydroimiadazolone | Pyro-carbamidomethyl [N-term,R] | — |
| +14.0157 | — | 0.10 (#33) | 0.49 (#20) | 0.04 (#20) | — | Methylation | Methyl [C,D,E,H,I,L,N,N-term,Q,R,S] | Methylation on R |
| -2.0156 | — | 0.39 (#18) | 0.47 (#21) | — | — | 2-amino-3-oxo-butanoic_acid | Val->Pro [C-term,S,T,V,Y] | — |
| +125.8966 | — | 0.20 (#24) | 0.44 (#22) | — | — | Iodination | Iodo [H,Y] | — |
| -1.0024 | — | 0.43 (#15) | — | — | — | Isotopic peak error | — | — |
| +21.9819 | 0.16 (#39) | 0.38 (#19) | 0.43 (#23) | 0.21 (#12) | Cation:Na | Sodium adduct | Cation:Na [D,E] | Sodium on E |

_365 total clusters in window; 269 found by exactly one tool (shown above only if in the top 30 by max %; full singleton list omitted here)._
