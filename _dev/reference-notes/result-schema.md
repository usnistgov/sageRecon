# Reconnaissance Report JSON Schema

**Version:** 1.0.0
**Last updated:** 2026-07-15 (body carries content through Phase 7B)

> **Freshness note:** the schema body was extended through Phase 7B (see the
> mass-calibration, neutron-folding, and prominence peak-detection sections
> below), though it originated in Phase 1. Current project state lives in
> `PLAN.md`'s status block, not here. During Phase 8 validation, verify these
> field definitions against the actual `ReconResult` struct in
> `recon-tool/src/` — they have not been diffed field-for-field.

This document defines the complete JSON schema for the reconnaissance report output.
This is the contract between all analysis modules and both the CLI and future GUI.

---

## Design Principles

1. **Dual readouts everywhere** — Every metric that can be expressed as both spectral count AND intensity-weighted is reported both ways. Users can choose which view matters for their question.

2. **Explicit ambiguity** — When multiple interpretations are possible (e.g., multiple Unimod matches within tolerance), report ALL candidates and flag `ambiguous: true`. Never silently pick one.

3. **No clean match bucket** — Peaks that don't match any Unimod entry within tolerance are reported as `unannotated` rather than force-matched to the nearest wrong name.

4. **Confidence from existing fields only** — MVP uses Sage's existing output columns (`hyperscore`, `matched_intensity_pct`, `longest_b`, `longest_y`) for confidence metrics. No new fragment-ion computation.

5. **Isotope correction explicit** — Delta masses are reported both raw and isotope-corrected. The correction formula is documented in the schema.

---

## Top-Level Structure

```json
{
  "schema_version": "1.0.0",
  "generated_at": "2026-07-07T13:00:00Z",
  "tool_version": "0.1.0",
  
  "input": { ... },
  "mod_discovery": { ... },
  "signal_fate": { ... },
  "polymer": { ... },
  "diagnostic_ions": { ... },
  "digestion": { ... },
  "qc": { ... }
}
```

---

## Section: `input`

Metadata about the input files and search parameters used.

```json
{
  "input": {
    "mzml_files": [
      {
        "path": "path/to/file.mzML.gz",
        "basename": "file.mzML.gz",
        "ms1_count": 12345,
        "ms2_count": 45678,
        "total_tic": 1.23e12
      }
    ],
    "fasta_path": "path/to/database.fasta",
    "fasta_entries": 20000,
    "sage_params_path": "path/to/open-search-params.json",
    "sage_version": "0.14.6",
    "precursor_tolerance": {
      "type": "da",
      "low": -500,
      "high": 100
    },
    "fragment_tolerance": {
      "type": "ppm",
      "low": -20,
      "high": 20
    },
    "isotope_errors": [-1, 3],
    "chimera_enabled": true,
    "report_psms": 2
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `mzml_files` | array | List of input mzML files with basic stats |
| `fasta_path` | string | Path to FASTA database |
| `fasta_entries` | int | Number of protein entries in FASTA |
| `sage_params_path` | string | Path to Sage config JSON used |
| `sage_version` | string | Sage version reported at runtime |
| `precursor_tolerance` | object | Precursor mass tolerance used |
| `fragment_tolerance` | object | Fragment mass tolerance used |
| `isotope_errors` | array[int, int] | Isotope error range searched |
| `chimera_enabled` | bool | Whether chimeric search was enabled |
| `report_psms` | int | Number of PSMs reported per spectrum |

---

## Section: `mod_discovery`

Delta-mass analysis results — the core PTM scouting output.

### Isotope Correction Formula

```
corrected_delta = (expmass - calcmass) - (isotope_error × C13_C12_DIFF)
C13_C12_DIFF = 1.003354835 Da  (¹³C − ¹²C mass difference)
```

**Note:** The previous NEUTRON constant (1.0086649158849 Da, free neutron rest mass) was incorrect for isotope envelope spacing. The correct value is the ¹³C−¹²C mass difference (1.003354835 Da).

### Mass Calibration (Phase 7B)

Before histogram binning, an intensity-weighted median of the near-zero population (|Δ| < 0.1 Da) is computed as `apex_offset`. This scalar is subtracted from all delta masses to center the unmodified peak at zero.

### Neutron Folding (Phase 7B)

Bins at ±k × C13_C12_DIFF (k = 1,2,3) are identified as monoisotope misassignments and folded into the Δ=0 bin. This removes the +1.003, +2.007, etc. artifact peaks.

### Prominence-Based Peak Detection (Phase 7B)

Peaks are detected using prominence filtering (height above local baseline must exceed 30% of bin count) rather than simple threshold + merge. This eliminates diffuse noise while preserving real PTM peaks.

### Schema

```json
{
  "mod_discovery": {
    "summary": {
      "total_psms": 81966,
      "psms_near_zero": 44382,
      "psms_near_zero_pct": 54.2,
      "psms_modified": 37584,
      "psms_modified_pct": 45.8,
      "unique_bins": 6669,
      "bin_width_da": 0.01
    },
    
    "histogram": [
      {
        "bin_center": 0.00,
        "count": 39916,
        "intensity_sum": 1.23e11,
        "intensity_pct": 42.5
      },
      {
        "bin_center": 1.00,
        "count": 5251,
        "intensity_sum": 4.56e10,
        "intensity_pct": 15.7
      }
    ],
    
    "peaks": [
      {
        "rank": 1,
        "delta_mass": 0.000,
        "delta_mass_corrected": 0.000,
        "isotope_error_mode": 0,
        
        "count": 39916,
        "count_pct": 48.7,
        "intensity_sum": 1.23e11,
        "intensity_pct": 42.5,
        
        "confidence": {
          "mean_hyperscore": 45.2,
          "median_hyperscore": 42.1,
          "mean_matched_intensity_pct": 0.72,
          "mean_longest_b": 8.3,
          "mean_longest_y": 9.1,
          "hyperscore_vs_unmodified": null
        },
        
        "annotations": [
          {
            "unimod_id": null,
            "name": "Unmodified",
            "delta_mass": 0.0,
            "mass_error_da": 0.0,
            "mass_error_ppm": 0.0,
            "sites": [],
            "classification": "None",
            "source": "intrinsic"
          }
        ],
        "ambiguous": false,
        "unannotated": false
      },
      {
        "rank": 2,
        "delta_mass": 15.995,
        "delta_mass_corrected": 15.995,
        "isotope_error_mode": 0,
        
        "count": 1854,
        "count_pct": 2.3,
        "intensity_sum": 2.34e10,
        "intensity_pct": 8.1,
        
        "confidence": {
          "mean_hyperscore": 38.7,
          "median_hyperscore": 36.2,
          "mean_matched_intensity_pct": 0.65,
          "mean_longest_b": 7.1,
          "mean_longest_y": 8.4,
          "hyperscore_vs_unmodified": 0.86
        },
        
        "annotations": [
          {
            "unimod_id": 35,
            "name": "Oxidation",
            "delta_mass": 15.9949,
            "mass_error_da": 0.0001,
            "mass_error_ppm": 6.3,
            "sites": ["M", "W", "P", "F", "Y", "H", "C"],
            "classification": "Post-translational",
            "source": "unimod"
          }
        ],
        "ambiguous": false,
        "unannotated": false
      },
      {
        "rank": 3,
        "delta_mass": 0.984,
        "delta_mass_corrected": 0.984,
        "isotope_error_mode": 0,
        
        "count": 1203,
        "count_pct": 1.5,
        "intensity_sum": 1.12e10,
        "intensity_pct": 3.9,
        
        "confidence": {
          "mean_hyperscore": 35.2,
          "median_hyperscore": 33.1,
          "mean_matched_intensity_pct": 0.58,
          "mean_longest_b": 6.8,
          "mean_longest_y": 7.9,
          "hyperscore_vs_unmodified": 0.78
        },
        
        "annotations": [
          {
            "unimod_id": 7,
            "name": "Deamidated",
            "delta_mass": 0.9840,
            "mass_error_da": 0.0000,
            "mass_error_ppm": 0.0,
            "sites": ["N", "Q"],
            "classification": "Artefact",
            "source": "unimod"
          }
        ],
        "ambiguous": false,
        "unannotated": false
      },
      {
        "rank": 10,
        "delta_mass": 52.911,
        "delta_mass_corrected": 52.911,
        "isotope_error_mode": 0,
        
        "count": 398,
        "count_pct": 0.5,
        "intensity_sum": 3.45e9,
        "intensity_pct": 1.2,
        
        "confidence": {
          "mean_hyperscore": 28.4,
          "median_hyperscore": 26.1,
          "mean_matched_intensity_pct": 0.42,
          "mean_longest_b": 5.2,
          "mean_longest_y": 6.1,
          "hyperscore_vs_unmodified": 0.63
        },
        
        "annotations": [],
        "ambiguous": false,
        "unannotated": true
      }
    ],
    
    "isotope_error_view": {
      "description": "PSMs grouped by isotope_error value before correction",
      "distribution": [
        { "isotope_error": -1, "count": 1234, "pct": 1.5 },
        { "isotope_error": 0, "count": 72000, "pct": 87.8 },
        { "isotope_error": 1, "count": 6500, "pct": 7.9 },
        { "isotope_error": 2, "count": 1800, "pct": 2.2 },
        { "isotope_error": 3, "count": 432, "pct": 0.5 }
      ],
      "zero_only_peak_count": 45,
      "zero_only_peaks": "Available in full histogram with isotope_error==0 filter"
    },
    
    "annotation_settings": {
      "unimod_version": "2024-01-15",
      "match_tolerance_da": 0.01,
      "max_decomposition_depth": 2,
      "excluded_classifications": ["Isotopic label", "O18 label"]
    }
  }
}
```

### Field Definitions: `mod_discovery.peaks[]`

| Field | Type | Description |
|-------|------|-------------|
| `rank` | int | Rank by count (1 = most frequent) |
| `delta_mass` | float | Raw delta mass (expmass - calcmass), Da |
| `delta_mass_corrected` | float | Isotope-corrected delta mass, Da |
| `isotope_error_mode` | int | Most common isotope_error value for this peak |
| `count` | int | Number of PSMs in this peak |
| `count_pct` | float | Percentage of total PSMs |
| `intensity_sum` | float | Sum of ms2_intensity for PSMs in peak |
| `intensity_pct` | float | Percentage of total intensity |
| `confidence` | object | Quality metrics from Sage output |
| `annotations` | array | Unimod matches within tolerance |
| `ambiguous` | bool | True if 2+ Unimod matches within tolerance |
| `unannotated` | bool | True if no Unimod match within tolerance |

### Field Definitions: `mod_discovery.peaks[].confidence`

| Field | Type | Description |
|-------|------|-------------|
| `mean_hyperscore` | float | Mean Sage hyperscore for PSMs in peak |
| `median_hyperscore` | float | Median Sage hyperscore |
| `mean_matched_intensity_pct` | float | Mean fraction of intensity explained by matched ions |
| `mean_longest_b` | float | Mean longest consecutive b-ion series |
| `mean_longest_y` | float | Mean longest consecutive y-ion series |
| `hyperscore_vs_unmodified` | float\|null | Ratio of mean hyperscore to unmodified peak (null for unmodified) |

### Field Definitions: `mod_discovery.peaks[].annotations[]`

| Field | Type | Description |
|-------|------|-------------|
| `unimod_id` | int\|null | Unimod record ID (null for intrinsic like "Unmodified") |
| `name` | string | Modification name |
| `delta_mass` | float | Unimod monoisotopic mass, Da |
| `mass_error_da` | float | Difference from observed peak center, Da |
| `mass_error_ppm` | float | Difference in ppm (at typical precursor mass) |
| `sites` | array[string] | Possible modification sites from Unimod |
| `classification` | string | Unimod classification (Post-translational, Artefact, etc.) |
| `source` | string | "unimod", "intrinsic", or "combination" |

---

## Section: `signal_fate`

Where is the signal going? Explained vs. unexplained, by count AND intensity.

```json
{
  "signal_fate": {
    "summary": {
      "total_ms2_spectra": 98765,
      "total_ms2_intensity": 2.89e12
    },
    
    "by_count": {
      "identified": {
        "count": 74996,
        "pct": 75.9
      },
      "unidentified": {
        "count": 23769,
        "pct": 24.1
      }
    },
    
    "by_intensity": {
      "identified": {
        "intensity": 2.31e12,
        "pct": 79.9
      },
      "unidentified": {
        "intensity": 5.80e11,
        "pct": 20.1
      }
    },
    
    "identified_breakdown": {
      "by_count": {
        "unmodified": { "count": 44382, "pct": 59.2 },
        "modified_annotated": { "count": 25614, "pct": 34.2 },
        "modified_unannotated": { "count": 5000, "pct": 6.7 }
      },
      "by_intensity": {
        "unmodified": { "intensity": 1.23e12, "pct": 53.2 },
        "modified_annotated": { "intensity": 8.90e11, "pct": 38.5 },
        "modified_unannotated": { "intensity": 1.90e11, "pct": 8.2 }
      }
    },
    
    "chimera_stats": {
      "spectra_with_multiple_psms": 12345,
      "spectra_with_multiple_psms_pct": 16.5,
      "avg_psms_per_chimeric_spectrum": 1.8
    },
    
    "filtering": {
      "peptide_q_threshold": 0.01,
      "decoy_prefix": "rev_",
      "psms_before_filter": 125000,
      "psms_after_filter": 74996,
      "decoys_removed": 3500,
      "q_filtered": 46504
    }
  }
}
```

### Field Definitions: `signal_fate`

| Field | Type | Description |
|-------|------|-------------|
| `total_ms2_spectra` | int | Total MS2 spectra in input file(s) |
| `total_ms2_intensity` | float | Sum of precursor intensities for all MS2 |
| `by_count.identified` | object | Spectra with at least one passing PSM |
| `by_count.unidentified` | object | Spectra with no passing PSM |
| `by_intensity.identified` | object | Intensity explained by identified spectra |
| `by_intensity.unidentified` | object | Intensity from unidentified spectra |
| `identified_breakdown` | object | Further breakdown of identified signal |
| `chimera_stats` | object | Statistics on chimeric/co-fragmenting spectra |
| `filtering` | object | Details of PSM filtering applied |

---

## Section: `polymer`

Polymer contamination assessment from MS1 spectra (ported from mzSniffer).

```json
{
  "polymer": {
    "summary": {
      "total_ms1_tic": 1.23e13,
      "polymer_tic": 4.56e11,
      "polymer_pct_tic": 3.7,
      "contamination_level": "low"
    },
    
    "by_type": [
      {
        "polymer_type": "PEG",
        "repeat_unit_da": 44.0262,
        "matched_peaks": 234,
        "intensity_sum": 2.34e11,
        "pct_tic": 1.9,
        "representative_mz": [283.17, 327.20, 371.23, 415.25]
      },
      {
        "polymer_type": "PPG",
        "repeat_unit_da": 58.0419,
        "matched_peaks": 89,
        "intensity_sum": 1.12e11,
        "pct_tic": 0.9,
        "representative_mz": [447.29, 505.33, 563.38]
      },
      {
        "polymer_type": "Polysiloxane",
        "repeat_unit_da": 74.0188,
        "matched_peaks": 156,
        "intensity_sum": 1.10e11,
        "pct_tic": 0.9,
        "representative_mz": [371.10, 445.12, 519.14]
      }
    ],
    
    "detection_settings": {
      "mz_tolerance_ppm": 10.0,
      "min_series_length": 3,
      "polymer_definitions_source": "mzsniffer_defaults"
    },
    
    "contamination_thresholds": {
      "low": "<5% TIC",
      "moderate": "5-15% TIC",
      "high": ">15% TIC"
    }
  }
}
```

### Field Definitions: `polymer`

| Field | Type | Description |
|-------|------|-------------|
| `total_ms1_tic` | float | Total ion current from all MS1 spectra |
| `polymer_tic` | float | TIC attributed to polymer contaminants |
| `polymer_pct_tic` | float | Percentage of TIC from polymers |
| `contamination_level` | string | "low", "moderate", or "high" |
| `by_type` | array | Breakdown by polymer type |
| `by_type[].polymer_type` | string | PEG, PPG, Polysiloxane, etc. |
| `by_type[].repeat_unit_da` | float | Mass of repeating unit |
| `by_type[].matched_peaks` | int | Number of MS1 peaks matched |
| `by_type[].intensity_sum` | float | Sum of matched peak intensities |
| `by_type[].pct_tic` | float | Percentage of total MS1 TIC |
| `by_type[].representative_mz` | array[float] | Example m/z values matched |

---

## Section: `diagnostic_ions`

Diagnostic fragment ion detection in MS2 spectra — oxonium ions (glyco flagship) and contaminant masses.

```json
{
  "diagnostic_ions": {
    "summary": {
      "total_ms2_spectra": 98765,
      "spectra_with_oxonium": 8234,
      "spectra_with_oxonium_pct": 8.3,
      "spectra_with_contaminant_ions": 1234,
      "spectra_with_contaminant_ions_pct": 1.2
    },
    
    "oxonium_ions": {
      "description": "Glycan diagnostic ions in MS2 spectra",
      "detection_rule": "≥2 oxonium ions in top 10% of peaks, m/z 204 mandatory",
      
      "spectra_passing_rule": 6543,
      "spectra_passing_rule_pct": 6.6,
      
      "by_ion": [
        {
          "name": "HexNAc",
          "mz": 204.0867,
          "spectra_detected": 7890,
          "spectra_detected_pct": 8.0,
          "mean_relative_intensity": 0.15,
          "diagnostic_rank": 1
        },
        {
          "name": "Hex-HexNAc",
          "mz": 366.1395,
          "spectra_detected": 5432,
          "spectra_detected_pct": 5.5,
          "mean_relative_intensity": 0.08,
          "diagnostic_rank": 2
        },
        {
          "name": "NeuAc",
          "mz": 292.1027,
          "spectra_detected": 3210,
          "spectra_detected_pct": 3.2,
          "mean_relative_intensity": 0.06,
          "diagnostic_rank": 3
        },
        {
          "name": "NeuAc-H2O",
          "mz": 274.0921,
          "spectra_detected": 2987,
          "spectra_detected_pct": 3.0,
          "mean_relative_intensity": 0.05,
          "diagnostic_rank": 4
        },
        {
          "name": "HexNAc-H2O",
          "mz": 186.0761,
          "spectra_detected": 4321,
          "spectra_detected_pct": 4.4,
          "mean_relative_intensity": 0.04,
          "diagnostic_rank": 5
        },
        {
          "name": "HexNAc-2H2O",
          "mz": 168.0655,
          "spectra_detected": 3890,
          "spectra_detected_pct": 3.9,
          "mean_relative_intensity": 0.03,
          "diagnostic_rank": 6
        },
        {
          "name": "Hexose",
          "mz": 163.0601,
          "spectra_detected": 2100,
          "spectra_detected_pct": 2.1,
          "mean_relative_intensity": 0.02,
          "diagnostic_rank": 7
        },
        {
          "name": "HexNAc-internal",
          "mz": 138.0545,
          "spectra_detected": 1890,
          "spectra_detected_pct": 1.9,
          "mean_relative_intensity": 0.02,
          "diagnostic_rank": 8
        }
      ],
      
      "glyco_enrichment_estimate": {
        "description": "Rough estimate of glycopeptide content based on oxonium ion prevalence",
        "estimated_glyco_spectra_pct": 6.6,
        "confidence": "moderate",
        "note": "Based on screening rule only; not a quantitative glycoproteomics analysis"
      }
    },
    
    "contaminant_ions": {
      "description": "Known contaminant fragment masses in MS2 spectra",
      "source": "positiveContaminants.txt (Keller 2008)",
      
      "by_contaminant": [
        {
          "name": "Triethylamine",
          "mz": 102.1277,
          "spectra_detected": 456,
          "spectra_detected_pct": 0.5,
          "origin": "LC buffer, very persistent"
        },
        {
          "name": "DMSO",
          "mz": 79.0212,
          "spectra_detected": 234,
          "spectra_detected_pct": 0.2,
          "origin": "Solvent"
        }
      ]
    },
    
    "detection_settings": {
      "mz_tolerance_ppm": 20.0,
      "oxonium_top_n_pct": 10.0,
      "oxonium_min_ions": 2,
      "oxonium_mandatory_ion_mz": 204.0867
    }
  }
}
```

### Field Definitions: `diagnostic_ions.oxonium_ions`

| Field | Type | Description |
|-------|------|-------------|
| `spectra_passing_rule` | int | Spectra meeting glycopeptide screening rule |
| `spectra_passing_rule_pct` | float | Percentage of MS2 spectra |
| `by_ion` | array | Detection stats per oxonium ion |
| `by_ion[].name` | string | Ion name |
| `by_ion[].mz` | float | Exact m/z |
| `by_ion[].spectra_detected` | int | Spectra where ion was found |
| `by_ion[].mean_relative_intensity` | float | Mean intensity relative to base peak |
| `by_ion[].diagnostic_rank` | int | Diagnostic value ranking (1=most diagnostic) |

---

## Section: `digestion`

Digestion efficiency metrics from Sage PSM results.

```json
{
  "digestion": {
    "enzyme": {
      "name": "trypsin",
      "cleave_at": "KR",
      "restrict": "P",
      "c_terminal": true
    },
    
    "missed_cleavages": {
      "distribution": [
        { "missed": 0, "count": 52341, "pct": 69.8 },
        { "missed": 1, "count": 18234, "pct": 24.3 },
        { "missed": 2, "count": 4421, "pct": 5.9 }
      ],
      "mean": 0.36,
      "max_allowed": 2
    },
    
    "semi_tryptic": {
      "fully_tryptic_count": 71234,
      "fully_tryptic_pct": 95.0,
      "semi_tryptic_count": 3762,
      "semi_tryptic_pct": 5.0,
      "semi_tryptic_n_term": 1890,
      "semi_tryptic_c_term": 1872
    },
    
    "peptide_properties": {
      "length_distribution": {
        "min": 7,
        "max": 50,
        "mean": 14.2,
        "median": 13,
        "histogram": [
          { "length": 7, "count": 1234 },
          { "length": 8, "count": 2345 },
          { "length": 9, "count": 4567 }
        ]
      },
      "mass_distribution": {
        "min_da": 700.4,
        "max_da": 4800.2,
        "mean_da": 1623.5,
        "median_da": 1456.2
      }
    }
  }
}
```

### Field Definitions: `digestion`

| Field | Type | Description |
|-------|------|-------------|
| `enzyme` | object | Enzyme settings used in search |
| `missed_cleavages.distribution` | array | Count by number of missed cleavages |
| `missed_cleavages.mean` | float | Mean missed cleavages per peptide |
| `semi_tryptic.fully_tryptic_pct` | float | Percentage of fully tryptic peptides |
| `semi_tryptic.semi_tryptic_pct` | float | Percentage of semi-tryptic peptides |
| `peptide_properties` | object | Length and mass distributions |

---

## Section: `qc`

Minimal QC metrics — not a general QC suite (see non-goals in PLAN.md).

```json
{
  "qc": {
    "mass_accuracy": {
      "precursor_ppm": {
        "mean": 2.3,
        "median": 1.8,
        "std": 3.1,
        "percentile_95": 6.2
      },
      "fragment_ppm": {
        "mean": 4.5,
        "median": 3.9,
        "std": 5.2,
        "percentile_95": 12.1
      }
    },
    
    "identification_rates": {
      "psm_rate": {
        "value": 75.9,
        "description": "Percentage of MS2 spectra with passing PSM"
      },
      "peptide_rate": {
        "unique_peptides": 44462,
        "per_1000_spectra": 450.1
      },
      "protein_rate": {
        "unique_proteins": 6544,
        "per_1000_spectra": 66.3
      }
    },
    
    "score_distributions": {
      "hyperscore": {
        "mean": 38.5,
        "median": 35.2,
        "std": 15.3,
        "percentile_5": 18.2,
        "percentile_95": 68.4
      },
      "delta_hyperscore": {
        "mean": 12.3,
        "median": 10.1,
        "description": "Difference between rank 1 and rank 2 PSM scores"
      }
    },
    
    "retention_time": {
      "gradient_length_min": 120.5,
      "first_id_min": 2.3,
      "last_id_min": 118.2,
      "ids_per_minute_mean": 624.5
    }
  }
}
```

### Field Definitions: `qc`

| Field | Type | Description |
|-------|------|-------------|
| `mass_accuracy.precursor_ppm` | object | Precursor mass error statistics |
| `mass_accuracy.fragment_ppm` | object | Fragment mass error statistics |
| `identification_rates.psm_rate` | object | MS2-to-PSM conversion rate |
| `identification_rates.peptide_rate` | object | Unique peptide counts |
| `identification_rates.protein_rate` | object | Unique protein counts |
| `score_distributions` | object | Sage score statistics |
| `retention_time` | object | Chromatographic coverage |

---

## Validation Against Test Data

The schema was designed against exploratory analysis of `B.naive_01steady-state.mzML.gz`:

| Observed Peak | Schema Representation |
|---------------|----------------------|
| 0.00 Da (39,916 PSMs) | `peaks[0]` with `annotations[0].name = "Unmodified"` |
| 15.99-16.00 Da (1,854 PSMs) | `peaks[rank=2]` with Oxidation annotation |
| 0.98-1.01 Da (deamidation + isotope) | Multiple peaks, `isotope_error_view` separates them |
| 52.91 Da (398 PSMs, unknown) | `peaks[rank=10]` with `unannotated: true` |
| -17.03 Da (318 PSMs) | Peak with Ammonia loss annotation |

---

## Implementation Notes

1. **Histogram vs. Peaks**: The `histogram` array contains ALL bins at the configured resolution (0.01 Da default). The `peaks` array contains only significant peaks after peak-picking (local maxima above a count threshold).

2. **Intensity weighting**: All intensity values use `ms2_intensity` from Sage output, which is the summed intensity of matched fragment ions.

3. **Chimera handling**: When `chimera: true` and `report_psms: 2`, multiple PSMs can map to the same scan. Signal fate accounting deduplicates by scan for spectrum-level stats but counts all PSMs for modification discovery.

4. **Unimod matching**: Uses 0.01 Da tolerance by default (matching PTM-Shepherd). Decomposition into 2-mod combinations is supported but not in MVP.

5. **Polymer detection**: MS1-only, ported from mzSniffer. Does not use MS2 fragmentation patterns.

6. **Oxonium screening rule**: "≥2 oxonium ions in top 10% of peaks, m/z 204 mandatory" — this is a published heuristic, not a quantitative glycoproteomics analysis.

---

## Schema Versioning Policy

This schema follows semantic versioning:

- **Major version** (1.x.x → 2.x.x): Breaking changes — fields removed, renamed, or with changed semantics. Consumers must update.
- **Minor version** (1.0.x → 1.1.x): Backward-compatible additions — new optional fields, new sections. Existing consumers continue to work.
- **Patch version** (1.0.0 → 1.0.1): Documentation fixes, clarifications, example corrections. No schema changes.

**Compatibility guarantee:** A consumer written for schema version 1.0.0 should be able to read any 1.x.x output without modification (new fields are ignored).

---

## Changelog

- **3.2.0** (2026-09-03): `recommendations.not_recommended[]` and
  `recommendations.notable_unannotated[]` gained THREE optional fields:
  `count_pct`, `sites` and `position`. A 3.1.0 consumer keeps working.
  NO EXISTING VALUE CHANGES. Verified: one `recon run` before the change and one
  after, compared key by key with the new keys and the timestamps excluded.
  The "did not make the cut" table rendered an em dash for both Residue and %
  while Count was populated. The data existed in both cases and was dropped on
  the way out.
  `count_pct` uses the SAME denominator as `RecommendedMod.count_pct` —
  `mod_discovery.summary.total_psms`. The three tables sit next to each other, so
  a second denominator would make them incomparable.
  `sites` and `position` are read TOGETHER, exactly as on `RecommendedMod`.
  Their source depends on `name_source`:
  * `"curated"` — the acceptors the residue test ACTUALLY RAN AGAINST, carried
    from the tested candidate. A `failed_residue_test` row exists to say which
    residues failed; without them it cannot.
  * `"unimod"` — the acceptor sites of the Unimod entry the annotation already
    chose, keyed by its `unimod_id` and guarded on `mono_mass`. Single-letter
    residues only: Unimod's `N-term` and `C-term` are POSITIONS, not acceptors.
    A Unimod-named row also carries Unimod's own position, mapped onto the
    curated vocabulary ("Any N-term" -> "Peptide N-terminal.", "Protein N-term"
    -> "Protein N-terminal.", and so on), and only when every residue-bearing
    specificity agrees on one.
  ⚠ **CORRECTED 2026-09-03, the same day this entry was written.** It first said
  the sites came from `PeakAnnotation.sites` and that Unimod contributed no
  position. Both were wrong, and the first was a BUG: that field is
  `UnimodEntry::sites`, which drops every `hidden="1"` specificity and has no
  fallback for an entry whose specificities are ALL hidden. On serum that is 10
  of the 11 named entries, so the field filled 1 row of 10.
  The fix reads the entry's own `specificities` with the same all-hidden fallback
  `UnimodEntry::classification` already carries. `UnimodEntry::sites` is
  deliberately NOT changed: it feeds `mod_discovery.peaks[].annotations[].sites`,
  an existing published field, and widening it there would move values nothing
  asked to move.
  ⚠ 3.2.0 output made BEFORE that correction carries `sites` on the non-hidden
  subset only and no `position` on a Unimod-named row. The field set is identical
  either way, so no version distinguishes them. Regenerate to get the full set.
  ⚠ A SATELLITE row is left with an empty acceptor even when it carries a Unimod
  name. It has `decided_by: null` and never reached a residue test.
  An empty `sites` AND an empty `position` mean "not known", which is NOT the
  same statement as "unspecific".
  The HTML also merged the MS1 and MS2 tolerance recommendations into one stat.
  That adds NO field: the MS2 rung is derived at render time from
  `ms1_calibration.ms2_tolerance_high_ppm`, which the JSON already carries.
  ⚠ The MS2 recommendation is quantized onto the `{10,20,50,100}` ppm ladder
  ONLY when the MS2 analyzer is ppm-based. An ion trap or quadrupole gets
  DALTONS — the measured ppm converted at m/z 500 and doubled. A ppm rung on a
  unit-resolution trap is meaningless. All four committed test files are
  Orbitraps, so the dalton branch is covered by unit tests and by no real data.
- **3.1.0** (2026-09-03): `input.fasta_file` and `input.enzyme` ADDED. The report
  did not record which database was searched, nor which protease the digestion
  numbers were counted with. Both had to be read back from Sage's own
  `results.json`. Both fields are optional and are omitted when the run did not
  have them: `recon analyze` has no `--enzyme`, and its `--fasta` is optional.
  A 3.0.0 consumer keeps working.
  `input.fasta_file` is the path as it was given, like `mzml_file`. It is not
  made absolute.
  `input.enzyme` is IDENTITY ONLY: `name`, `cleave_at`, `restrict`,
  `c_terminal`. The tuning fields (`missed_cleavages`, `min_len`, `max_len`,
  `semi_enzymatic`) are NOT in this block. They belong to the search config, and
  Pass 1 and Pass 2 set them differently.
  NO EXISTING VALUE CHANGES. This was verified: one `recon run` before the change
  and one after, compared key by key with the two new keys and the timestamps
  excluded.
- **3.0.0** (2026-09-02): **BREAKING — `mass_accuracy.precursor_median_ppm` and
  `precursor_p95_ppm` are SIGNED, not absolute.** No field was added, renamed or
  removed. The MEANING changed, which this policy calls a major bump.
  The cause was upstream: the Sage pin moved to v0.15.0-beta.2 on 2026-09-01,
  and v0.15 stopped absoluting `precursor_ppm`. Nothing in recon changed, so
  nothing announced it. Measured on the committed v0.15 serum output: 17591 of
  68817 values negative, min -121568.77.
  `fragment_median_ppm` and `fragment_p95_ppm` are NOT affected. `fragment_ppm`
  stayed absolute: 0 of 68817 negative. **Do not merge the two conventions.**
  ⚠ `precursor_p95_ppm` also stopped being a coverage bound. Under v0.14 it read
  as "95 % of PSMs within X ppm". It is now the 95th percentile of a signed
  distribution.
  ⚠ Neither precursor field is a mass accuracy in an open search: the delta
  carries the modification mass. The usable MS1 number is
  `ms1_calibration.bias_ppm`.
  Versions 2.1.0 and earlier reported these two fields as `|error|`. Output from
  2026-09-01 onward is labelled 2.1.0 but already carries signed values; that
  window is unavoidable, because the change came from a dependency.
- **1.0.0** (2026-07-07): Initial schema definition for Phase 1
