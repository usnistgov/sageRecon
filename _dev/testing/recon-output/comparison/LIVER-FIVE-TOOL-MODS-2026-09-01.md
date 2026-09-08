# Liver mod discovery — five sources, one mass axis

NIST RM 8461 liver `10mg_1_A_1`. The `fourway_comparison.md` treatment, run on
liver — which that file does not cover — and with **Byonic Preview added as a
fifth arm**. All on the same 2018 database. Provenance for every run:
`testing/reference-data/liver-reference-runs.md`.

Producers, both re-runnable:
* `testing/scripts/liver_5way_mods.py` — the tables below
* `testing/scripts/curated_mass_collisions.py` — section 4

Clustering, tolerance and window are `compare_4way.py`'s, reused verbatim:
single global greedy pass, largest-count-first, 0.015 Da, window [-100, 500] Da.

---

## 1. Why Preview was missing until now

Not a decision. `VariableMods` appears nowhere in NOTES, JOURNAL, PLAN or any
script, so no locked entry excluded it. Two causes:

1. **Catalogued as configuration.** The vendored README lists
   `objs/VariableMods*.txt` under "mod settings". The file is only half
   settings: it also carries `nmods`, `n16`, `prop` and `prop2`, which are
   Preview's MEASURED counts. Filed as config, it was never read as output.
   This is the same failure that hid Preview's mass accuracy in
   `result_summary.html` until 2026-09-01.
2. **The alkylation-agnostic frame.** The mod comparison is built on four tools
   that DISCOVERED +57. Preview had `CysMod=+57.021464` pre-set by the operator.
   That argument excludes Preview from the +57-on-C row only; it was applied to
   the whole arm.

### Two Preview conventions must be undone first

⚠ **`(-fixed mod)` rows are deltas RELATIVE TO THE FIXED +57 ON C.** On the raw
axis they match nothing. Verified three ways against Preview's own detail text,
not assumed:

```
Trioxidation  -9.036720 + 57.021464 = +47.984744   detail: "C[+48]"
Propionamide +14.015650 + 57.021464 = +71.037114   detail: "C[+71]"
Dehydro      -58.029289 + 57.021464 =  -1.007825   detail: "unmodified_C[-2]"

3 x oxidation = 47.984745                          agrees to 1e-6
```

Uncorrected, a mass join silently DROPS every cysteine modification Preview
found.

⚠ **`VariableMods.txt` repeats ONE GROUP TOTAL across several site rows** —
exactly the Mascot multi-site trap in NOTES. `Deamidated` shows 237 at N and 237
at Q because 237 is the combined total (194 N + 43 Q); summing gives 474. One
group can also span two MASSES: pyro-glu lists 66 against both -17.0265 and
-18.0106, where `result_detail.html` gives the true split as 61 and 5. Deduped
on (mod, mass, count, denominator): 9 repeated rows dropped, 4 group totals
replaced by the detail split.

⚠ One group the detail page never splits: "Carbamidomethylation artifacts
(H,K,N-terminus[+57/+114]): 2.3% (54/2359)". Its 54 is repeated on +57.0215 and
+114.0429, marked `g` below. Do not read those two cells as independent.

⚠ **Preview's list is CLOSED** — 19 masses off a fixed menu of 44 candidate
rows, not an open window. A Preview blank means "not on the menu", NOT "searched
and not found". A recon or Shepherd blank means the opposite.

---

## 2. Preview arm — how it was put on the mass axis

* rows read from `objs/VariableMods.txt`, **9 repeated group-total rows dropped** (same mod, same mass, same population listed once per site).
* **3 `(-fixed mod)` rows shifted back onto the absolute axis** by +57.021464:
  * `Dehydro (-fixed mod)` -58.029289 -> **-1.007825**
  * `Trioxidation (-fixed mod)` -9.036720 -> **+47.984744**
  * `Propionamide (-fixed mod)` +14.015650 -> **+71.037114**
* **4 group totals replaced by the finer split in `result_detail.html`**:
  * `Gln->pyro-Glu` @N-Term Q at -17.0265: 66 (group total) -> **61** (per-mass)
  * `Glu->pyro-Glu` @N-Term E at -18.0106: 66 (group total) -> **5** (per-mass)
  * `Ammonia-loss` @N-Term C at -17.0265: 66 (group total) -> **0** (per-mass)
  * `Dimethyl` @N-term at +28.0313: 9 (group total) -> **5** (per-mass)
* Preview % is its own `prop2` = nmods / 2667 baseline, the same number its summary page prints.

### Counts entering the comparison

* Recon: 49 rows
* Preview: 19 rows
* PTM-Shepherd: 65 rows
* Mascot: 351 rows
* MetaMorpheus: 41 rows
* Mascot: 16 rows had no clean Unimod mass — skipped and counted

## 3. The five-way table

| mass (Da) | recon rec | Recon % | Preview % | PTM-Shepherd % | Mascot % | MetaMorpheus % | Recon label | Preview label | PTM-Shepherd label | Mascot label | MetaMorpheus label |
|---|---|---|---|---|---|---|---|---|---|---|---|
| +0.0000 | — | 49.54 (#1) | — | 50.23 (#1) | — | 72.49 (#1) | Unmodified | — | None | — | Unmodified |
| +15.9949 | variable — Oxidation on M | 5.19 (#2) | 20.85 (#1) | 13.67 (#2) | 39.24 (#1) | 11.67 (#2) | Oxidation | Oxidation @M/Oxidation @H, W/Oxidation @P | Oxidation or Hydroxylation | Oxidation [A,D,F,H,K,M,N,P,R,W,Y] | Oxidation on M |
| +57.0215 | **FIXED** — Carbamidomethyl on C | 3.97 (#3) | 2.34 (#4) ᵍ | 8.58 (#3) | 21.22 (#2) | 8.27 (#3) | Carbamidomethyl | Carbamidomethyl @N-term, H, K | Iodoacetamide derivative/Addition of Glycine/Addition of G | Carbamidomethyl [A,C,D,E,G,H,K,N-term,S,T,Y] | Carbamidomethyl on C |
| +0.9840 | variable — Deamidation | 1.73 (#4) | 8.89 (#2) | 2.35 (#5) | 5.88 (#4) | 3.05 (#4) | Deamidated | Deamidated @N | Deamidation | Deamidated [N,Q,R] | Deamidation on N |
| +0.9970 | variable — Deamidation | — | — | 2.42 (#4) | 8.05 (#3) | — | — | — | First isotopic peak | Label:15N(1) [A,D,E,F,G,I,L,M,P,S,T,V,Y] | — |
| +114.0429 | _no_ (no_residue_support) | 0.39 (#18) | 2.34 (#5) ᵍ | 1.22 (#7) | 0.18 (#32) | 0.07 (#15) | GG | Dicarbamidomethyl @N-term | ubiquitinylation residue/Double Carbamidomethylation/Addition of N | GG [C,K,N-term,R,S,T] | Glutarylation on K |
| -17.0265 | variable — Gln->pyro-Glu | 0.43 (#14) | 2.29 (#3) ᵈ | 0.86 (#10) | 1.82 (#6) | 0.44 (#8) | Gln->pyro-Glu | Gln->pyro-Glu @N-Term Q | Pyro-glu from Q/Loss of ammonia | Gln->pyro-Glu [N,N-term] | Ammonia loss on N |
| +58.0055 | variable — Carboxymethylation | 0.28 (#22) | — | 0.79 (#11) | 1.97 (#5) | 0.07 (#14) | Carboxymethyl | — | Iodoacetic acid derivative | Carboxymethyl [A,C,G,K,N-term,W] | Carbamidomethyl on C, Deamidation on N |
| +26.0157 | _no_ (not_curated) | 0.23 (#26) | 1.72 (#6) | 0.36 (#21) | 0.97 (#9) | — | Delta:H(2)C(2) | Delta:H(2)C(2) @N-term, H, K | Acetaldehyde +26 | Delta:H(2)C(2) [A,H,K,N-term] | — |
| +31.9898 | _no_ (below_floor) | 0.72 (#8) | 0.97 (#8) | 1.72 (#6) | 0.57 (#14) | 0.89 (#6) | Dioxidation | Dioxidation @M/Dioxidation @W | dihydroxy | Dioxidation [C,F,M,P,R,W,Y] | Oxidation on M, Oxidation on M |
| +162.0528 | _no_ (not_curated) | 0.95 (#5) | — | 1.17 (#8) | 1.54 (#7) | — | UNANNOTATED | — | Hexose | Hex [K,N,N-term,S,T,W,Y] | — |
| +14.0157 | — | — | 1.31 (#7) | 0.06 (#54) | 0.25 (#24) | 0.04 (#22) | — | Methyl @N-term/Methyl @E/Methyl @R | Methylation | Methyl [C,D,E,H,I,K,L,N-term,Q,R,S,T,V] | Methylation on R |
| +42.0106 | — | — | 0.67 (#9) | 0.57 (#13) | 1.12 (#8) | 0.93 (#5) | — | Acetyl @Protein N-term/Acetyl @K | Acetylation | Acetyl [H,K,N-term,S,T] | Acetylation on X |
| -1.0024 | — | — | — | 1.07 (#9) | 0.02 (#160) | — | — | — | Isotopic peak error | Dehydro [C] | — |
| +16.9976 | _no_ (satellite (+1 C13 of +15.9945)) | 0.88 (#6) | — | — | 0.67 (#12) | — | UNANNOTATED | — | — | Pro->Asn [P] | — |
| +47.9847 | variable — Trioxidation | 0.40 (#17) | 0.52 (#10) ⁺ | 0.68 (#12) | 0.87 (#10) | — | Trioxidation | Trioxidation (-fixed mod) @C | cysteine oxidation to cysteic acid | Trioxidation [C,W,Y] | — |
| +73.0157 | _no_ (not_curated) | 0.76 (#7) | — | — | — | 0.09 (#13) | UNANNOTATED | — | — | — | Carbamidomethyl on C, Oxidation on M |
| +58.0419 | — | — | — | 0.10 (#44) | 0.72 (#11) | — | — | — | Reduced acrolein addition +58 | Delta:H(6)C(3)O(1) [C,H,K] | — |
| +52.9115 | variable — Fe[III] | 0.48 (#12) | — | 0.54 (#16) | — | 0.69 (#7) | Cation:Fe[III] | — | Replacement of 3 protons by iron | — | Fe[III] on E |
| +58.0225 | _no_ (not_curated) | 0.63 (#9) | — | 0.40 (#18) | 0.03 (#121) | — | UNANNOTATED | — | 2,3-dihydro-2,2-dimethyl-7-benzofuranol N-methyl carbamate | Gln->Trp [Q] | — |
| +72.9952 | — | — | — | — | 0.60 (#13) | — | — | — | — | Xle->Trp [I,L] | — |
| +0.9630 | — | — | — | 0.57 (#14) | 0.05 (#92) | — | — | — | Unannotated mass-shift 0.9630 | Xle->Asn [I,L] | — |
| +2.0047 | — | — | — | 0.55 (#15) | 0.15 (#35) | — | — | — | Second isotopic peak | Label:15N(2) [N,Q] | — |
| -1.0297 | _no_ (not_curated) | 0.53 (#10) | — | 0.26 (#22) | 0.11 (#52) | 0.04 (#21) | Lys->Allysine | — | Lysine oxidation to aminoadipic semialdehyde | Lys->Allysine [K] | Ammonia loss on N, Oxidation on M |
| -0.0800 | — | 0.53 (#11) | — | — | — | — | Unmodified | — | — | — | — |
| +16.9976 | _no_ (satellite (+1 C13 of +15.9945)) | — | — | — | 0.50 (#15) | — | — | — | — | Asn->Met [N] | — |
| -0.9588 | _no_ (not_curated) | 0.41 (#16) | — | 0.48 (#17) | 0.26 (#23) | — | UNANNOTATED | — | Unannotated mass-shift -0.9588 | Glu->Lys [E] | — |
| +151.9966 | — | — | 0.48 (#12) | — | — | — | — | DTT @C | — | — | — |
| +28.0313 | — | — | 0.45 (#11) ᵈ | — | 0.13 (#44) | — | — | Dimethyl @N-term/Dimethyl @R | — | Ethyl [A,K,N-term] | — |
| -33.9877 | variable — Dehydroalanine | 0.11 (#42) | — | 0.19 (#28) | 0.45 (#16) | — | Cys->Dha | — | Dehydroalanine (from Cysteine) | Cys->Dha [C] | — |

_374 total clusters in window; 299 found by exactly one tool (shown above only if in the top 30 by max %)._
_⁺ = Preview mass shifted off its fixed +57 C. ᵈ = count taken from `result_detail.html`, not the group total._

## 4. Do recon's recommendations line up?

| # | delta | recon calls it | Preview | PTM-Shepherd | Mascot | MetaMorpheus | corroborated by |
|---|---|---|---|---|---|---|---|
| 1 | +15.9945 | variable — Oxidation on M (Common Variable) | 20.85 | 13.67 | 39.24 | 11.67 | **4/4** |
| 2 | +57.0207 | **FIXED** — Carbamidomethyl on C (Common Fixed) | 2.34 | 8.58 | 21.22 | 8.27 | **4/4** |
| 3 | +0.9833 | variable — Deamidation (Common Artifact) | 8.89 | 2.35 | 5.88 | 3.05 | **4/4** |
| 4 | +52.9105 | variable — Fe[III] (Metal) | — | 0.54 | — | 0.69 | **2/4** |
| 5 | -17.0261 | variable — Gln->pyro-Glu (Common Biological) | 2.29 | 0.86 | 1.82 | 0.44 | **4/4** |
| 6 | +47.9844 | variable — Trioxidation (Less Common) | 0.52 | 0.68 | 0.87 | — | **3/4** |
| 7 | +58.0039 | variable — Carboxymethylation (Less Common) | — | 0.79 | 1.97 | 0.07 | **3/4** |
| 8 | -89.0302 | variable — Met-loss+Acetylation (Common Biological) | — | — | 0.17 | — | **1/4** |
| 9 | -33.9890 | variable — Dehydroalanine (Less Common) | — | 0.19 | 0.45 | — | **2/4** |

### recon's tail — the biggest peaks it declined to recommend

| delta | recon PSMs | reason | Preview | PTM-Shepherd | Mascot | MetaMorpheus |
|---|---|---|---|---|---|---|
| +162.0522 | 310 | not_curated | — | 1.17 | 1.54 | — |
| +16.9976 | 286 | satellite (+1 C13 of +15.9945) | — | — | 0.50 | — |
| +73.0157 | 248 | not_curated | — | — | — | 0.09 |
| +31.9894 | 235 | below_floor — Dioxidation | 0.97 | 1.72 | 0.57 | 0.89 |
| +58.0225 | 204 | not_curated | — | 0.40 | 0.03 | — |
| -1.0297 | 172 | not_curated — Lys->Allysine | — | 0.26 | 0.11 | 0.04 |
| -0.9798 | 144 | below_floor — Amidation | 0.15 | 0.36 | 0.03 | — |
| -1.0581 | 137 | not_curated | — | 0.21 | — | — |
| -0.9597 | 134 | not_curated | — | 0.48 | 0.12 | — |
| +114.0427 | 128 | no_residue_support — GG (Ubiquitination Site) | 2.34 | 1.22 | 0.18 | 0.07 |

⚠ **+57 is three discoveries and one assumption.** Preview's 2.34 % at +57.0215
is NOT cysteine carbamidomethylation — it is the H/K/N-term ARTIFACT. Preview's
cysteine +57 was pre-set and is reported separately as a fixed mod at 100.0 %
(226/226), a confirmation of the input, not a detection. The row is corroborated
by PTM-Shepherd, Mascot and MetaMorpheus, with Preview counted apart.

⚠ **-89.0302 Met-loss+Acetylation is a SINGLE-SOURCE recommendation.** Mascot
alone reports it, at 0.17 %. recon's internal evidence is strong (odds ratio
8578, q = 6.5e-152, 72 PSMs) and neither PTM-Shepherd nor MetaMorpheus reports
the mass at all. Label it as single-source wherever the nine are quoted.

⚠ **Counts are not commensurable, and no claim is made on them.** recon and
PTM-Shepherd report % of PSMs; MetaMorpheus % of q <= 0.01 peptides; Mascot its
own resolved-ET fraction; Preview `nmods` over a 2667-ID baseline. Five
currencies. NOTES "Prevalence currency" governs. **The claim is presence and
rank.**

---

## 5. Proline oxidation — the recommendation recon cannot make

Preview reports "Hydroxyproline: 1.5% (18/1166) of peptides containing P" as a
line of its own. recon has no such recommendation. Three questions, three
answers from files.

### 5.1 It IS in the curated list

Not a curation gap. `reference-notes/metaMorpheusMods/Mods.txt`:

```
ID   Hydroxylation
TG   P
PP   Anywhere.
MT   Common Biological
CF   O1                      -> 15.994915 Da
```

⚠ **Only four of the eight files in that directory are compiled into the
binary** — `Mods.txt`, `aListOfmods.txt`, `ProteaseMods.txt`, `surfactants.txt`.
`ptmlist.txt`, `glyco.txt`, `substitutions.txt` and `tmt.txt` are NOT. Read from
the build dependency graph (`recon-tool/target/*/deps/recon_tool-*.d`), not
assumed. Hydroxyproline is in `ptmlist.txt` (unloaded) AND, as `Hydroxylation`
on P, in `Mods.txt` (loaded), so it does reach the classifier.

### 5.2 All four other tools found it

| tool | its name for it | on P | on M | basis |
|---|---|---|---|---|
| Byonic Preview | Hydroxyproline | 18 | 509 | peptides; 1.5 % of 1166 P-containing |
| Mascot ET | Oxidation @ P | 1314 | 3086 | its own site rows, before roll-up |
| MetaMorpheus | Hydroxylation on P | 192 | 2127 | q <= 0.01 peptides |
| PTM-Shepherd | Oxidation or Hydroxylation | 69 | 1889 | unambiguous localizations only |
| **recon** | **not reported** | **—** | 1688 | one peak at +15.9945, un-localized |

⚠ The PTM-Shepherd figure needs care. `psm.tsv` `MSFragger Localization`
lower-cases the modified residue, but **1260 of the 3319 PSMs at this delta are
AMBIGUOUS** (more than one lower-case residue). Counting every candidate of an
ambiguous row inflates P to 366 and puts implausible residues alongside it
(G 356, E 314, A 297) — which is how you can tell that count is a candidate set,
not an assignment. Restricted to the 2059 unambiguous rows: M 91.7 %, P 3.4 %.

### 5.3 Why recon misses it: the test is CONTAINMENT, not localization

recon's open search assigns a delta per PSM, not a site. Everything at +15.9945
is one peak. Four curated entries fall within `CURATED_TOL_DA` = 0.010 Da of it.
`peptide_hits` asks **"does this peptide CONTAIN an acceptor residue"** — so a
candidate is compared against the background rate at which peptides contain that
residue at all.

Reproduced with `curated_mass_collisions.py --peak 15.994472`:

| candidate | acceptors | band containment | background | odds ratio | verdict |
|---|---|---|---|---|---|
| Oxidation on M | M | 91.7 % | 19.7 % | 59.42 | **passes** OR >= 2 |
| Hydroxylation | KN | 78.5 % | 76.5 % | 1.12 | **fails** |
| Hydroxylation | P | 62.7 % | 56.0 % | **1.34** | **fails** |
| Oxidation | CDEFHILQRSTUVWY | 100.0 % | 100.0 % | — | background saturated, untestable |

⚠ **APPROXIMATE.** A flat +/- 0.010 Da window gives a band of 1717 PSMs against
recon's reported 1688, and OR 59.42 for Oxidation on M against recon's 63.53.
Same conclusion, different number. **Do not quote these as recon's own values.**

**Hydroxylation on P does not lose a `max_by` tie-break. It fails the OR >= 2
gate outright**, because proline is present in 56.0 % of all liver peptides. The
containment test cannot see enrichment for a residue that common, and the band
is 92 % M-oxidised peptides that merely happen to also contain a proline.

**This is not a missing entry and not a missed peak.** recon found the mass,
ranked it second overall and recommended it. It is a reporting-shape limit: the
recommendation is keyed to a MASS, and a mass can carry more than one real
modification. Liver's collagen and ECM content is where that bites hardest,
which is why all four reference tools list proline oxidation separately.

---

## 6. How general is this? The curated list's own collisions

From `curated_mass_collisions.py`, over the four compiled-in files:

* curated entries parsed: **99**
* distinct mass clusters at 0.010 Da: **62**
* clusters holding more than one candidate: **21**
* entries sitting in a contested cluster: **58** (**58.6 %** of all entries)
* largest cluster: **5** candidates
* contested masses with two or more residue-testable candidates: **15**

**Background containment in the 32497 liver PSMs, and the band fraction a
candidate needs to reach OR >= 2** (`x = 2f / (1 + f)`):

| residue | in this % of peptides | needs |
|---|---|---|
| L | 79.3 % | 88.5 % |
| K | 60.6 % | 75.5 % |
| T | 57.5 % | 73.0 % |
| **P** | **56.0 %** | **71.8 %** |
| Q | 46.3 % | 63.3 % |
| Y | 35.4 % | 52.3 % |
| H | 31.6 % | 48.0 % |
| **M** | **19.7 %** | **33.0 %** |
| **C** | **11.5 %** | **20.6 %** |
| W | 11.4 % | 20.5 % |

⚠ **The reachability number is difficulty, NOT a prediction.** Whether a
candidate clears its threshold depends on the actual band. Both outcomes occur
on this one file:

* `+15.9949` Hydroxylation on P needs 71.8 %, band has 62.7 % -> **fails**
* `-17.0265` Gln->pyro-Glu on Q needs 63.3 %, band is nearly all Q -> **OR 650,
  passes** and is recon's recommendation

**So the test works when one modification DOMINATES its mass band. It cannot
surface a SECOND, co-occurring modification at the same mass.** The rare-acceptor
candidate wins by construction: at +57.0215, `Carbamidomethyl on C` needs 20.6 %
while the broad `Carbamidomethyl DEHKSTY` needs 99.7 %.

---

## 7. What this does NOT establish

* **Nothing about localization.** recon reports an un-localized delta mass plus a
  population enrichment. Same boundary that retired Gate 5.
* **No claim that Hydroxylation on P SHOULD be recommended.** Its measured odds
  ratio here is ~1.34, below the gate. Whether the gate or the test is the right
  design is a separate question this document does not answer.
* **Not equal populations.** Preview kept ProtScore >= 20 representative
  proteins; recon Pass 1 is the whole database; the FragPipe and MetaMorpheus
  runs are their own. Same file, same FASTA, different peptide sets.
* **One file, one instrument class.** One Orbitrap run.
* **The MetaMorpheus arm needs a gitignored input**
  (`Task3-SearchTask/AllPeptides.psmtsv`). Both producers hard-stop with a named
  missing input rather than quietly dropping the arm.
