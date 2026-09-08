# Step 3.5 — liver, four sources, one rule

NIST RM 8461 liver `10mg_1_A_1`. Four sources: **recon, Byonic Preview,
PTM-Shepherd, MetaMorpheus**. All on the SAME 2018 database
(`uniprot_sprot_iso_human-2018_06.fasta`). Provenance for every run:
`testing/reference-data/liver-reference-runs.md`.

**No tool's own class, terminus, or missed-cleavage column is read anywhere in
this document.** Every side is reclassified from raw sequence context with a
mirror of the shipped Rust rule (`digestion.rs::is_tryptic_nterm` /
`is_tryptic_cterm`), including the initiator-Met excision branch.
Producer: `testing/scripts/liver_four_tool_digestion.py`.

---

## 1. Digestion

Basis: DISTINCT PEPTIDES. Denominator for missed cleavage and ragged rates is
tryptic + semitryptic (non-tryptic excluded), which is Preview's own convention.

| source | peptides | den | missed cl | ragged-N | ragged-C | nontryp | N:C |
|---|---|---|---|---|---|---|---|
| recon Pass 2 (semi) | 10621 | 10621 | **17.12 %** | 6.75 % | 3.01 % | 0.00 % | 2.24 |
| PTM-Shepherd open (FULLY) | 15998 | 15954 | **19.64 %** | 1.64 % | 0.51 % | 0.02 % | 3.18 |
| MetaMorpheus (FULLY) | 15256 | 15253 | **17.84 %** | 0.54 % | 0.45 % | 0.01 % | 1.21 |
| Byonic Preview v3.2.0 | 2009 | 2008 | **15.90 %** | 8.60 % | 1.30 % | 0.10 % | 6.62 |

⚠ An MSFragger closed semi-tryptic run (`liverFragger`, 16.16 % missed cleavage)
was an INTERMEDIATE CHECK from a previous session and is **not part of this
comparison**. Archived to `_archive/liverFragger-2026-08-31/` on 2026-09-01.
Do not cite it and do not re-add it.

Preview's row is quoted from `result_summary.html`; its peptide list is not
shipped, so it is the one row that cannot be recomputed.

Excluded as unresolvable against this FASTA: PTM-Shepherd 41, MetaMorpheus 1,
recon 0.

### What it says

**Missed cleavage — four independent tools span 15.90 → 19.64 %, a 3.74 pp
spread.** Against the between-sample spread of 15.9 pp recorded in NOTES
"ACCEPTANCE CRITERION", the biological signal is **4.3x** the method
disagreement. The criterion is met. Missed cleavage remains the weakest link, as
that entry already says — this is the worst ratio in the table, and it is the
headline number.

**Ragged termini cannot be compared across enzyme conventions, and this table
shows why.** The two FULLY tryptic searches report ragged-N of 1.64 % and
0.54 %. A fully tryptic search cannot generate a ragged peptide, so those are a
control near zero, not a measurement. **Comparing them to Preview's 8.60 % would
be the exact trap NOTES warns about.** Only recon (semi) and Preview (fully
specific, but reporting ragged classes) are comparable here: 6.75 % vs 8.60 %,
a 1.85 pp gap against a 24.7 pp between-sample range — **13x**. Comfortable.

**✅ The N >> C direction holds on all four sources.** N:C ratios 2.24, 3.18,
1.21, 6.62 — every one above 1. This is a fifth independent confirmation
that Davis Table 3's ragged N/C headers are transposed relative to the tool that
produced them. The magnitude of the ratio is NOT reproducible across tools
(1.21 to 6.62); the direction is.

⚠ The residual fully-tryptic ragged-N (1.64 % on Shepherd, 0.54 % on
MetaMorpheus) is not zero because classification uses the FIRST listed protein.
A peptide that is tryptic in its true parent can be ragged in the razor protein.
recon carries the identical limitation, documented at `digestion.rs:390`.

---

## 2. Mod discovery

Matched by DELTA MASS at 0.01 Da, never by name — the same chemistry is
"Carbamidomethyl" to one tool and "Iodoacetamide derivative" to another.
Producer: `testing/scripts/liver_mod_rank_comparison.py`.

All three ran alkylation-agnostic: recon strips and asserts empty mods in both
passes; PTM-Shepherd had every fixed and variable mod removed; MetaMorpheus had
Common Fixed / Common Variable moved OUT of the mod options and INTO the G-PTM-D
discovery list.

| | recon | PTM-Shepherd | MetaMorpheus | Mascot |
|---|---|---|---|---|
| distinct masses (delta ~ 0 removed) | 48 | 102 | 15 | 330 |

| pair | n | Spearman rho | p |
|---|---|---|---|
| recon vs PTM-Shepherd | 38 | **+0.616** | 2.5e-05 (permutation, seed 42) |
| recon vs MetaMorpheus | 6 | **+1.000** | 0.002778 (exact) |
| recon vs Mascot | 26 | **+0.502** | 0.01005 (permutation, seed 42) |

**Mascot error-tolerant is the fourth agnostic tool**, added 2026-09-01. Its
`Human_ertol-2018.par` has `MODS=` and `IT_MODS=` both EMPTY with
`ERRORTOLERANT=1`, so its Carbamidomethyl is a discovery, not an assumption —
verified in the file, not assumed from the name.

⚠ Mascot reports by NAME + SITE and splits one mass across many site rows, so
the site rows are **rolled up by mass** before comparison. Without that, its
C-only row understates the true +57. The roll-up already existed and is reused
verbatim (`compare_mod_discovery.load_mascot`); 16 rows had no clean Unimod mass
("Non-specific cleavage", unresolved substitutions) and are SKIPPED and counted,
never silently dropped.

⚠ Mascot's `PFA=1` allows ONE missed cleavage against recon's two, and its
counts are its own resolved-ET fraction, not PSMs. It contributes to the mod
comparison ONLY — it is not in the digestion table and must not be.

⚠ n = 6 for MetaMorpheus is small and expected: G-PTM-D searches a curated
candidate list, not an open window, so it has far fewer masses to share. A
perfect rho on six points is real but thin — report it with its n, never alone.

### Top shared masses

| delta | recon | PTM-Shepherd | MetaMorpheus | Mascot |
|---|---|---|---|---|
| 15.9945 | 1688 | 3290 | 1973 | 4585 |
| 57.0207 | 1291 | 2048 | 1398 | 2479 |
| 0.9833 | 562 | 447 | 516 | 687 |
| 162.0522 | 310 | 302 | — | 180 |
| 16.9976 | 286 | 136 | — | 137 |
| 73.0157 | 248 | 459 | — | — |
| 31.9894 | 235 | 455 | — | 67 |
| 58.0225 | 204 | 123 | — | 3 |
| 52.9105 | 157 | 123 | 117 | — |

**All FOUR tools independently put Oxidation first and Carbamidomethyl second
among real modifications, and not one of them was told the sample was
alkylated.** recon reports +57.0207 at rank 3 overall (behind the unmodified
population and Oxidation), 1291 PSMs, 3.97 %.

⚠ **Counts are not commensurable and no claim is made on them** — Mascot's are
resolved-ET fractions, Shepherd's run ~1.5-2x recon's, — NOTES
"Prevalence currency" records that these tools report four different quantities.
Shepherd's counts run roughly 1.5–2x recon's throughout. The claim is rank.

---

## 3. Mass accuracy — and the signed MS2 number recon does not have

**Preview reports measured mass error in `result_summary.html`, before and after
its own recalibration.** Quoted verbatim (found 2026-09-01 by parsing the HTML
tables, which is where this lives — not in the numbers the vendored README
already carried):

```
PRECURSOR   before recal: median signed  0.0 ppm | |error| 0.5 ppm | 943 high / 850 low
            after  recal: median signed -0.0 ppm | |error| 0.5 ppm | 878 high / 980 low
            off-by-one (nominal mass is +1 isotope): 6.0% (119/1973)

FRAGMENT    before recal: median signed -3.1 ppm | |error| 3.5 ppm | 307 high / 9820 low
            after  recal: median signed  0.1 ppm | |error| 0.7 ppm | 4783 high / 5354 low
```

Compare recon on the same file (`full-run/liver.json`). **Pre-recalibration is
the right side of Preview's ledger to use** — recon reports what the instrument
delivered, it does not recalibrate the spectra.

| quantity | recon | Preview (pre-recal) | agreement |
|---|---|---|---|
| fragment / MS2, **absolute** median | **3.4206 ppm** (`fragment_median_ppm`) | **3.5 ppm** | **0.08 ppm apart** |
| fragment / MS2, **signed** median | *not produced* — recon's MS2 is absolute by design | **−3.1 ppm** | — |
| precursor / MS1, signed median | **−1.4193 ppm** (`bias_ppm`) | **0.0 ppm** | 1.42 ppm apart |
| precursor / MS1, spread | MAD 0.6231 ppm | \|error\| median 0.5 ppm | same scale |

### Three things this settles or opens

**✅ recon's MS2 accuracy number is corroborated to 0.08 ppm.** recon reports
3.4206 ppm absolute; Preview independently measures 3.5 ppm absolute on the same
raw file. Same quantity (median of |error|), same instrument state.

**✅ The signed MS2 gap has an external answer.** NOTES "MS2 stays ABSOLUTE"
records that recon has no signed MS2 number for the benchmark table and says to
settle it in Step 5. Preview supplies it: **−3.1 ppm**, corroborated by its own
directional count, **307 fragments high against 9820 low**. A 32:1 imbalance is
not compatible with a centred distribution. So the true liver MS2 bias is
negative and of order −3 ppm, and recon's absolute 3.42 ppm is consistent with
it. recon still does not MEASURE the sign; it now has a reference value for the
write-up.

**⚠ MS1 disagrees by 1.42 ppm, and that is NOT resolved.** recon says the
precursor bias is −1.4193 ppm; Preview says 0.0 ppm with a balanced 943/850
split. Both cannot be describing the same population. Candidate explanations,
none tested: Preview measured on the `.mgf` after its own conversion from
`.raw`; the populations differ by an order of magnitude (recon's clean subset is
7519 PSMs, Preview's precursor count is ~1793); and the two use different
peptide sets entirely. **Recorded as an open discrepancy — do not present recon's
MS1 bias as corroborated by Preview.**

⚠ Preview's `off-by-one 6.0 % (119/1973)` is a fifth quantity again — it is an
isotope-assignment rate, not a mass error. Do not fold it into a tolerance
comparison.

---

## 4. What this does NOT establish

* ⚠ **(CORRECTED 2026-09-01) Preview DOES report mass accuracy.** An earlier
  version of this document said there was no Preview number to compare against.
  That was true of the UI's INPUTS but wrong about its OUTPUT — see section 3.
* **Nothing about localization.** recon reports an un-localized delta mass plus
  a population enrichment; the reference tools localize per PSM. Same boundary
  that retired Gate 5.
* **Nothing outside one instrument class.** This is one Orbitrap file. The
  ion-trap, TOF and Astral tolerance buckets remain unexercised.
* ⚠ **The MetaMorpheus arm is not reproducible from a clean clone.** Both
  producers read `Task3-SearchTask/AllPeptides.psmtsv`, which is gitignored
  (`.gitignore:62`), as MetaMorpheus's PSM files have always been in this repo.
  Both scripts HARD-STOP with a named missing input rather than quietly
  dropping the arm — falsified 2026-09-01 by removing an input and confirming
  the stop.
* **Not equal populations.** Preview kept ProtScore >= 20 representative
  proteins (2009 peptides); recon Pass 2 searches its own >= 2-peptide subset;
  the FragPipe runs searched the whole database. Direction and scale, not
  equality.
