# Fallback Common-Modification Table

Salvaged from the original handoff spec (`testing/sagePreview.md`, now retired).
This is the hard-coded common-modification table the tool can fall back on when
a Unimod export isn't available — enough to annotate the most frequent deltas so
the tool works immediately. The real annotation path uses `testing/unimod.xml`
(see [unimod-decomposition.md](unimod-decomposition.md)); this is the minimum
viable default only.

| delta_mass | name | sites | note |
|-----------|------|-------|------|
| 15.9949 | Oxidation | M | also P (hydroxyproline — collagen/ECM) |
| 79.9663 | Phospho | STY | |
| 42.0106 | Acetyl | K, N-term | |
| 0.9840 | Deamidation | NQ | |
| 57.0215 | Carbamidomethyl | C | sanity check (fixed mod) |
| 14.0157 | Methyl | KR | |
| 28.0313 | Dimethyl | KR | |
| 114.0429 | GG (ubiquitin remnant) | K | |
| 43.0058 | Carbamyl | K, N-term | |
| -17.0265 | Loss of ammonia | N-term Q/C | |

**Annotation rules (from the spec):**
- Match by nearest mass within tolerance. Default tolerance tied to instrument
  mass accuracy (±0.005 Da, or a few ppm). Note: the implemented path uses the
  0.01 Da Unimod tolerance — see the `(locked)` decision in NOTES.
- Ambiguity handling: if multiple known mods fall within tolerance (e.g. +80
  phospho vs. sulfation; multiple +16 candidates), report ALL candidates and
  flag as ambiguous — do NOT force a single guess.

**Isotope correction (for reference):**
```text
corrected_delta = (expmass − calcmass) − (isotope_error × NEUTRON)
NEUTRON = 1.0086649158849
```
