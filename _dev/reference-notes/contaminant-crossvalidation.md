# Contaminant Detection: Keller List vs. polymer.rs Cross-Validation

## Overview

Two sources of MS1 contaminant data:
1. **polymer.rs** — 16 polymer series (ported from mzSniffer), generates m/z ladders from formulas
2. **positiveContaminants.txt** — 831 discrete m/z values from Keller 2008 review

## Comparison

### polymer.rs Coverage

| Polymer Type | Charge States | Series Generation |
|--------------|---------------|-------------------|
| PEG | +1H, +2H, +3H | H₂O + n×C₂H₄O |
| PPG | +1H | H₂O + n×C₃H₆O |
| Triton X-100 | +1H, +Na | C₁₄H₂₂O + n×C₂H₄O |
| Triton X-100 (Reduced) | +1H, +Na | C₁₄H₂₈O + n×C₂H₄O |
| Triton X-101 | +1H | C₁₅H₂₄O + n×C₂H₄O |
| Triton X-101 (Reduced) | +1H | C₁₅H₃₀O + n×C₂H₄O |
| Polysiloxane | +1H | n×C₂H₆SiO |
| Tween-20/40/60/80 | +Na | Core + n×C₂H₄O |
| IGEPAL CA-630 (NP-40) | +1H | C₁₅H₂₄O + n×C₂H₄O |

**Strengths:**
- Generates complete series up to max m/z
- Handles multiple charge states
- Calculates %TIC (quantitative)

**Weaknesses:**
- Only 16 polymer types
- No solvents, buffers, or non-polymer contaminants

### Keller List Coverage

| Category | Count | Examples |
|----------|-------|----------|
| PEG series | ~50 | [M+H]+, [M+Na]+, [M+K]+, [M+NH4]+ |
| PPG series | ~30 | [M+H]+, [M+Na]+, [M+K]+ |
| Solvents | ~100 | Methanol, ACN, DMSO, DMF |
| Buffers | ~50 | TRIS, TEA, DIPEA, TFA |
| Phthalates | ~40 | DMP, DEP, DBP, DEHP |
| Detergents | ~20 | Triton, Tween (discrete masses) |
| Calibrants | ~30 | Cesium, PEG calibrants |
| Misc | ~500 | Plasticizers, antioxidants, etc. |

**Strengths:**
- 831 discrete m/z values
- Covers non-polymer contaminants (solvents, buffers, phthalates)
- Multiple adduct forms (+H, +Na, +K, +NH4, +Cu)
- Literature-validated

**Weaknesses:**
- Discrete masses only (doesn't generate series)
- No intensity/TIC calculation
- Some entries are MS2 fragments, not MS1 precursors

## Overlap Analysis

### PEG Comparison

Keller PEG entries (first 10):
```
63.044056   [A1B+H]+    n=1
85.026000   [A1B+Na]+   n=1
100.999937  [A1B+K]+    n=1
107.070271  [A2B+H]+    n=2
129.052215  [A2B+Na]+   n=2
145.026152  [A2B+K]+    n=2
151.096486  [A3B+H]+    n=3
173.078430  [A3B+Na]+   n=3
189.052367  [A3B+K]+    n=3
195.122701  [A4B+H]+    n=4
```

polymer.rs PEG+1H series (first 10):
```
19.018   n=0 (H2O + H)
63.044   n=1
107.070  n=2
151.096  n=3
195.123  n=4
239.149  n=5
283.175  n=6
327.201  n=7
371.228  n=8
415.254  n=9
```

**Match:** polymer.rs PEG+1H matches Keller [AnB+H]+ series exactly.

**Gap:** polymer.rs doesn't have PEG+Na or PEG+K series. Keller has these.

### PPG Comparison

Similar pattern — polymer.rs has PPG+1H, Keller has +H, +Na, +K adducts.

### Triton/Tween Comparison

polymer.rs generates full series. Keller has only a few discrete masses for detergents.

**Winner:** polymer.rs for detergent coverage.

### Non-Polymer Contaminants

Keller has extensive coverage that polymer.rs completely lacks:
- Solvents: Methanol (33.03, 65.06), ACN (42.03, 83.06), DMSO (79.02, 157.04)
- Buffers: TRIS (122.08), TEA (102.13), TFA (158.96)
- Phthalates: DMP (195.07), DEP (223.10), DBP (279.16)
- Calibrants: Cesium (132.90), PEG calibrants

**Winner:** Keller for non-polymer contaminants.

## Recommendation

**Keep both, use for different purposes:**

### polymer.rs — Quantitative Polymer %TIC
- Use for: "How much of my MS1 signal is polymer contamination?"
- Output: %TIC per polymer type
- Best for: QC metrics, contamination level assessment

### Keller List — Broad Contaminant Screening
- Use for: "Is this specific m/z a known contaminant?"
- Output: Yes/no match with compound ID
- Best for: Peak annotation, troubleshooting unknown peaks

## Implementation Plan

### Phase 1: Parse Keller List (New Module)

```rust
// contaminants.rs
pub struct ContaminantEntry {
    pub mz: f64,
    pub ion_type: String,      // e.g., "[M+H]+"
    pub formula: String,       // e.g., "CH3OH"
    pub compound_id: String,   // e.g., "Methanol"
    pub origin: String,        // e.g., "solvent"
    pub is_esi: bool,
    pub is_maldi: bool,
}

pub fn load_keller_list(path: &Path) -> Result<Vec<ContaminantEntry>>;
pub fn find_contaminant(mz: f64, tolerance_ppm: f64, list: &[ContaminantEntry]) -> Vec<&ContaminantEntry>;
```

### Phase 2: Add Sodium/Potassium Adducts to polymer.rs

```rust
// Add to default_polymers():
Polymer::new("PEG+Na", formula_mass("H2O") + NA, formula_mass("C2H4O"), 1, false),
Polymer::new("PEG+K", formula_mass("H2O") + K, formula_mass("C2H4O"), 1, false),
Polymer::new("PPG+Na", formula_mass("H2O") + NA, formula_mass("C3H6O"), 1, false),
Polymer::new("PPG+K", formula_mass("H2O") + K, formula_mass("C3H6O"), 1, false),
```

### Phase 3: Unified Contaminant Report

```json
{
  "polymer_contamination": {
    "total_pct_tic": 0.36,
    "by_polymer": [
      { "name": "PEG+2H", "pct_tic": 0.11 },
      { "name": "PEG+1H", "pct_tic": 0.08 }
    ],
    "contamination_level": "Moderate"
  },
  "known_contaminants_detected": [
    { "mz": 102.13, "compound": "TEA", "origin": "buffer" },
    { "mz": 195.07, "compound": "DMP", "origin": "plasticizer" }
  ]
}
```

## Test Results on B.naive File

### polymer.rs Results (Current)
```
Total polymer %TIC: 0.36%
PEG+2H: 0.11%
PEG+1H: 0.08%
PEG+3H: 0.05%
Tween-40: 0.04%
Tween-20: 0.03%
...
Contamination level: Moderate (0.1-1%)
```

### Expected Keller List Results
Would additionally detect:
- Solvent peaks (ACN, methanol clusters)
- Buffer peaks (if present)
- Phthalate peaks (common lab contaminants)

## Priority

| Task | Priority | Effort |
|------|----------|--------|
| Add Na/K adducts to polymer.rs | High | 1 hour |
| Parse Keller list | Medium | 2 hours |
| Unified contaminant report | Low | 2 hours |

**Recommendation:** Add Na/K adducts to polymer.rs first (quick win), then parse Keller list for Phase 7 report.
