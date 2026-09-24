//! Proteomics reconnaissance tool using Sage open search.
//!
//! This library provides modules for:
//! - Running Sage in process, as a pinned library dependency
//! - Parsing and filtering Sage results
//! - Computing isotope-corrected delta masses
//! - Modification discovery via delta-mass analysis
//! - Unimod database parsing and annotation
//! - MS1/MS2 self-calibration and tolerance recommendation
//! - Polymer contamination detection (MS1)
//! - Oxonium ion screening for glycopeptides (MS2)
//! - Pass 2 digestion composition (missed cleavage, ragged termini)
//! - Unified report generation (JSON, text, HTML)
//!
//! # Example
//!
//! ```no_run
//! use recon_tool::sage_results::{parse_sage_results, FilterOptions};
//! use recon_tool::unimod::UnimodDb;
//! use recon_tool::mod_discovery::{run_mod_discovery, ModDiscoveryConfig};
//! use std::path::Path;
//!
//! // Parse Sage results
//! let results = parse_sage_results(
//!     Path::new("results.sage.tsv"),
//!     &FilterOptions::default()
//! ).unwrap();
//!
//! // Load Unimod database
//! let unimod = UnimodDb::from_xml(Path::new("unimod.xml")).unwrap();
//!
//! // Run mod discovery
//! let discovery = run_mod_discovery(&results, &unimod, &ModDiscoveryConfig::default());
//!
//! println!("Found {} peaks", discovery.peaks.len());
//! ```

pub mod calibration;
pub mod curated_mods;
pub mod defaults;
pub mod digestion;
pub mod enzyme;
pub mod mod_discovery;
pub mod mzml;
pub mod oxonium;
pub mod pass2;
pub mod peak_composition;
pub mod polymer;
pub mod protein_index;
pub mod provenance;
pub mod report;
pub mod sage_results;
pub mod sage_runner;
pub mod stats;
pub mod tier_assignment;
pub mod unimod;

// Re-export commonly used types
pub use calibration::{
    compute_ms1_stats, compute_ms2_tolerance, hyperscore_guard_would_apply, ms1_pass2_window,
    ms1_user_recommendation, select_clean_subset, MassErrorStats, Ms1Pass2Window,
    Ms1UserRecommendation, Ms2Tolerance, PsmSummary,
};
pub use mod_discovery::{
    run_mod_discovery, CalibrationResult, DiscoverySettings, FoldingStats, HistogramBin,
    ModDiscoveryConfig, ModDiscoveryResult, Peak, PeakAssignmentMode,
};
pub use mzml::{
    bucket_tolerance, class_from_accession, class_from_filter_string, detect_analyzers,
    detect_analyzers_sampled, extract_ms1_spectra, extract_ms2_spectra, get_max_mz, get_mzml_stats,
    print_analyzer_census, resolve_ms2_tolerance, AnalyzerCensus, AnalyzerClass, AnalyzerCounts,
    BucketSource, FragmentTolerance, MassAnalyzerTerm, Ms1Spectrum, Ms2Spectrum, Ms2TolDecision,
    MzmlStats, ToleranceBasis, ASTRAL_MS2_HALF_WIDTH_PPM, DEFAULT_ANALYZER_SAMPLE,
    FILTER_STRING_ANALYZERS, ION_TRAP_MS2_HALF_WIDTH_DA, LEGACY_TOF_MS2_HALF_WIDTH_PPM,
    MASS_ANALYZER_TERMS, ORBITRAP_MS2_HALF_WIDTH_PPM, UNKNOWN_MS2_FALLBACK_PPM,
};
pub use oxonium::{
    compute_screening_summary, screen_spectra, screen_spectrum, OxoniumIon, OxoniumScreeningConfig,
    OxoniumScreeningResult, OxoniumScreeningSummary, OXONIUM_IONS,
};
pub use pass2::{precursor_tol_json, Pass2Plan};
pub use polymer::{
    default_polymers, search_polymers, Polymer, PolymerResult, PolymerSearchResults,
};
pub use protein_index::{
    target_accessions, write_subset_fasta, ProteinIndex, MIN_PEPTIDES_PER_PROTEIN,
    MIN_RESOLVED_FRACTION,
};
pub use provenance::{Provenance, ProvenanceInput, GIT_COMMIT, TOOL_VERSION};
pub use report::{generate_html_report, print_report_summary, Ms1CalibrationReport, ReconReport};
pub use sage_results::{FilterOptions, FilterStats, Psm, SageResults, C13_C12_DIFF};
pub use sage_runner::{SageConfig, SageRunResult};
pub use unimod::{UnimodDb, UnimodEntry, UnimodMatch};
