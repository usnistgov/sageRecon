# Digestion Efficiency: Python to Rust Port Plan

## Current State

### Rust (`digestion.rs`) — Implemented
- Reads `missed_cleavages` from Sage TSV
- Reads `semi_enzymatic` from Sage TSV  
- Computes statistics (distribution, mean, percentages)
- **Does NOT parse FASTA or classify terminus specificity**

### Python (testing/) — Prototype
Three scripts that need to be ported:

#### 1. `subset_fasta.py`
Subsets a FASTA file to only proteins identified in Sage results.

```python
# Key functionality:
- Parse Sage TSV to get unique protein IDs
- Parse FASTA file
- Write subset FASTA with only identified proteins
```

#### 2. `annotate_termini.py`
Classifies peptide termini as tryptic, semi-tryptic (N-ragged or C-ragged), or non-tryptic.

```python
# Key functionality:
- Parse Sage TSV to get peptide sequences and protein mappings
- Parse FASTA to get full protein sequences
- For each peptide:
  - Find position in protein
  - Check N-terminus: is preceding residue K/R or is it protein N-term?
  - Check C-terminus: does peptide end in K/R or is it protein C-term?
  - Classify: fully_tryptic, n_ragged, c_ragged, non_tryptic
```

#### 3. `digestion_efficiency.py`
Orchestrates the two-pass workflow for digestion efficiency analysis.

```python
# Key functionality:
- Run Sage pass 1 (open search)
- Subset FASTA to identified proteins
- Run Sage pass 2 (semi-enzymatic search on subset)
- Compute digestion efficiency metrics
```

## Port Plan

### Phase 1: FASTA Parser Module (`fasta.rs`)

```rust
pub struct FastaEntry {
    pub id: String,           // e.g., "sp|P12345|PROT_HUMAN"
    pub description: String,  // Full header line
    pub sequence: String,     // Amino acid sequence
}

pub fn parse_fasta(path: &Path) -> Result<Vec<FastaEntry>>;
pub fn write_fasta(entries: &[FastaEntry], path: &Path) -> Result<()>;
pub fn subset_fasta(fasta: &[FastaEntry], protein_ids: &HashSet<String>) -> Vec<FastaEntry>;
```

### Phase 2: Terminus Classification (`digestion.rs` extension)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminusType {
    Tryptic,      // K/R before cleavage site (or protein terminus)
    NonTryptic,   // Not K/R before cleavage site
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeptideSpecificity {
    FullyTryptic,   // Both termini tryptic
    NRagged,        // N-terminus non-tryptic, C-terminus tryptic
    CRagged,        // N-terminus tryptic, C-terminus non-tryptic
    NonTryptic,     // Both termini non-tryptic
}

pub fn classify_terminus(
    peptide: &str,
    protein_sequence: &str,
    position: usize,
) -> (TerminusType, TerminusType);

pub fn classify_peptide_specificity(
    peptide: &str,
    protein_sequence: &str,
) -> PeptideSpecificity;
```

### Phase 3: Extended Digestion Stats

```rust
pub struct ExtendedDigestionResult {
    // Existing fields from DigestionResult
    pub total_psms: usize,
    pub missed_cleavages: MissedCleavagesStats,
    pub semi_tryptic: SemiTrypticStats,
    pub peptide_length: PeptideLengthStats,
    
    // New fields from terminus classification
    pub terminus_specificity: TerminusSpecificityStats,
}

pub struct TerminusSpecificityStats {
    pub fully_tryptic: CountWithPct,
    pub n_ragged: CountWithPct,
    pub c_ragged: CountWithPct,
    pub non_tryptic: CountWithPct,
}
```

### Phase 4: CLI Integration

```rust
Commands::DigestionStats {
    tsv: PathBuf,
    fasta: Option<PathBuf>,  // NEW: for terminus classification
    q_threshold: f64,
    output: Option<PathBuf>,
    summary_only: bool,
}
```

## Implementation Notes

### Protein ID Matching
Sage's `proteins` column may contain multiple proteins (shared peptides). Need to handle:
- Single protein: `sp|P12345|PROT_HUMAN`
- Multiple proteins: `sp|P12345|PROT_HUMAN;sp|P67890|PROT2_HUMAN`

### Peptide Position Finding
- Use exact string matching: `protein_sequence.find(peptide)`
- Handle multiple occurrences (same peptide at different positions)
- Handle I/L ambiguity if needed

### Edge Cases
- Peptide not found in protein (decoy, contaminant, or sequence mismatch)
- Peptide at protein N-terminus (always "tryptic" at N-term)
- Peptide at protein C-terminus (always "tryptic" at C-term)
- Proline after K/R (missed cleavage site, but still tryptic terminus)

## Testing

1. Compare Rust output to Python output on same input files
2. Verify terminus classification matches manual inspection
3. Test edge cases (protein termini, shared peptides, etc.)

## Estimated Effort

- Phase 1 (FASTA parser): 2 hours
- Phase 2 (Terminus classification): 3 hours
- Phase 3 (Extended stats): 1 hour
- Phase 4 (CLI integration): 1 hour
- Testing: 2 hours

**Total: ~9 hours**

## Dependencies

- Requires FASTA file for terminus classification
- Sage TSV must have `proteins` column with valid protein IDs
- FASTA protein IDs must match Sage protein IDs

## Phase 7 Integration

The extended digestion stats will be included in the final report:

```json
{
  "digestion_efficiency": {
    "missed_cleavages": { ... },
    "semi_tryptic": { ... },
    "terminus_specificity": {
      "fully_tryptic": { "count": 50000, "pct": 75.0 },
      "n_ragged": { "count": 8000, "pct": 12.0 },
      "c_ragged": { "count": 7000, "pct": 10.5 },
      "non_tryptic": { "count": 1666, "pct": 2.5 }
    }
  }
}
```
