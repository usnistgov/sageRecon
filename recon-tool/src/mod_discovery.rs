//! Modification discovery engine: delta-mass histogram, peak detection, and annotation.
//!
//! This module implements the core PTM scouting functionality:
//! 1. Mass calibration (apex_offset correction)
//! 2. Neutron fold-to-zero (isotope misassignment removal)
//! 3. Build a delta-mass histogram from PSMs
//! 4. Prominence-based peak detection
//! 5. Satellite folding onto detected peaks
//! 6. Annotate peaks against Unimod with excluded classifications filter
//! 7. Compute confidence metrics from Sage output fields

use crate::sage_results::{Psm, SageResults, C13_C12_DIFF};
use crate::unimod::{UnimodDb, UnimodMatch};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

// Proton mass (Da) — for per-PSM m/z reconstruction in ppm-constant calibration.
// `mzml` holds the only definition. This module had its own copy, with a comment
// that called it a mirror, but nothing kept the two values equal.
use crate::mzml::PROTON_MASS;

/// Mass calibration mode for the delta-mass axis.
///
/// Parallel paths (NOT a swap) so `none` / `da-scalar` / `ppm-constant` can be
/// benchmarked on the same file without confounding "the ppm fix worked" with
/// "the code changed." `DaScalar` is the default and reproduces the historical
/// (Phase 7C) behavior byte-identically — do not change the default without
/// re-baselining the Tier 3 regression snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum CalibrationMode {
    /// No calibration — raw isotope-corrected deltas.
    None,
    /// Subtract a single Da constant (intensity-weighted median of near-zero pop).
    /// Corrects bias; cannot correct m/z-dependent (ppm) drift.
    #[default]
    DaScalar,
    /// Subtract a single ppm constant, applied per-PSM against each PSM's own m/z.
    /// Da magnitude of the correction scales with m/z (Orbitrap error ~const in ppm).
    PpmConstant,
}

/// How PSMs are grouped into peaks.
///
/// Parallel paths (NOT a swap), on the same pattern as `CalibrationMode`, so the
/// two answers can be benchmarked on the same file without confounding "the peak
/// fix worked" with "the code changed."
///
/// Both modes assign every PSM to its NEAREST accepted center and to that one
/// only, so both satisfy the exclusive-assignment invariant. They differ only in
/// which bins are allowed to be separate centers, which is the real question:
/// is a pair of adjacent bins one population or two?
///
/// The deamidation / isotope doublet is 19.3 mDa (0.984016 vs 1.003355), against
/// a 10 mDa bin. Whether that doublet is resolved in a given file depends on the
/// instrument's MS1 resolution, so neither mode is right for every input. See
/// NOTES, "Serum +1 Da conservation violation".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PeakAssignmentMode {
    /// Bins within the merge tolerance are ONE peak. Fewer, broader peaks.
    /// This is what the original `too_close` guard intended before f64 bin-center
    /// arithmetic defeated it.
    Merge,
    /// Every prominent bin is its own peak. More peaks, each narrower. Keeps a
    /// real doublet visible instead of averaging it away.
    Split,
}

impl Default for PeakAssignmentMode {
    /// `Merge` restores the intended pre-defect grouping. It is a DECISION, not a
    /// historical default — the previous behaviour double-counted PSMs and cannot
    /// be reproduced by either mode.
    fn default() -> Self {
        PeakAssignmentMode::Merge
    }
}

/// Default histogram bin width in Daltons
pub const DEFAULT_BIN_WIDTH_DA: f64 = 0.01;

/// Default minimum count for a bin to be considered a peak candidate
pub const DEFAULT_MIN_PEAK_COUNT: usize = 5;

/// Default tolerance for merging adjacent bins into a single peak
pub const DEFAULT_PEAK_MERGE_TOLERANCE_DA: f64 = 0.01;

/// Threshold for "near zero" classification (unmodified)
pub const NEAR_ZERO_THRESHOLD_DA: f64 = 0.1;

/// Threshold for unmodified roll-up (wider than merge, tighter than first real peak)
/// Must capture the full zero smear (fragments out to ~0.06 Da) but stay under 0.1 Da
pub const UNMODIFIED_ROLLUP_THRESHOLD_DA: f64 = 0.075;

/// Default base fold tolerance for neutron folding (Da)
/// k-scaled: tolerance(k) = base + (|k| - 1) × per_step
pub const DEFAULT_FOLD_TOLERANCE_BASE_DA: f64 = 0.012;

/// Per-step tolerance increase for higher k values (Da)
pub const DEFAULT_FOLD_TOLERANCE_PER_STEP_DA: f64 = 0.0045;

/// Compute k-scaled fold tolerance: base + (|k| - 1) × per_step
/// k=1: 12 mDa, k=2: 16.5 mDa, k=3: 21 mDa
#[inline]
fn fold_tolerance_for_k(k: i32, base: f64, per_step: f64) -> f64 {
    base + (k.abs() - 1).max(0) as f64 * per_step
}

/// Default prominence threshold (fraction of bin count)
pub const DEFAULT_PROMINENCE_THRESHOLD: f64 = 0.3;

/// Default excluded classifications for annotation
pub const DEFAULT_EXCLUDED_CLASSIFICATIONS: &[&str] = &[
    "AA substitution",     // Glu→Met, Xle→Asn, Thr→Cys, Trp→Pro — not PTMs
    "Other glycosylation", // too broad for recon
    "Isotopic label",      // ¹⁵N, ¹³C, ¹⁸O labels (all use this classification in Unimod)
];

/// Configuration for mod discovery
#[derive(Debug, Clone)]
pub struct ModDiscoveryConfig {
    /// Histogram bin width in Daltons
    pub bin_width_da: f64,
    /// Minimum PSM count for a bin to be considered a peak candidate
    pub min_peak_count: usize,
    /// Tolerance for merging adjacent bins into a single peak
    pub peak_merge_tolerance_da: f64,
    /// Maximum number of peaks to report
    pub max_peaks: usize,
    /// Fold tolerance for neutron folding (Da)
    pub fold_tolerance_da: f64,
    /// Prominence threshold (fraction of bin count that must rise above baseline)
    pub prominence_threshold: f64,
    /// Mass calibration mode (None / DaScalar / PpmConstant)
    pub calibration_mode: CalibrationMode,
    /// Enable neutron fold-to-zero
    pub enable_neutron_folding: bool,
    /// Enable satellite folding onto detected peaks
    pub enable_satellite_folding: bool,
    /// Classifications to exclude from annotation
    pub excluded_classifications: Vec<String>,
    /// How PSMs are grouped into peaks (Merge / Split)
    pub peak_assignment_mode: PeakAssignmentMode,
}

impl Default for ModDiscoveryConfig {
    fn default() -> Self {
        Self {
            bin_width_da: DEFAULT_BIN_WIDTH_DA,
            min_peak_count: DEFAULT_MIN_PEAK_COUNT,
            peak_merge_tolerance_da: DEFAULT_PEAK_MERGE_TOLERANCE_DA,
            max_peaks: 50,
            fold_tolerance_da: DEFAULT_FOLD_TOLERANCE_BASE_DA,
            prominence_threshold: DEFAULT_PROMINENCE_THRESHOLD,
            calibration_mode: CalibrationMode::default(),
            enable_neutron_folding: true,
            enable_satellite_folding: false,
            excluded_classifications: DEFAULT_EXCLUDED_CLASSIFICATIONS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            peak_assignment_mode: PeakAssignmentMode::default(),
        }
    }
}

/// A single histogram bin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBin {
    /// Bin center in Daltons
    pub bin_center: f64,
    /// Intensity-weighted apex delta mass within this bin
    /// Falls back to bin_center if intensity_sum == 0
    pub weighted_apex: f64,
    /// Number of PSMs in this bin
    pub count: usize,
    /// Sum of ms2_intensity for PSMs in this bin
    pub intensity_sum: f64,
    /// Percentage of total intensity
    pub intensity_pct: f64,
    /// Whether this bin was folded (isotope misassignment)
    #[serde(default)]
    pub folded: bool,
    /// Fold target (delta mass it was folded to, if folded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fold_target: Option<f64>,
}

/// Confidence metrics for a peak, computed from Sage output fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeakConfidence {
    /// Mean Sage hyperscore
    pub mean_hyperscore: f64,
    /// Median Sage hyperscore
    pub median_hyperscore: f64,
    /// Mean fraction of intensity explained by matched ions
    pub mean_matched_intensity_pct: f64,
    /// Mean longest consecutive b-ion series
    pub mean_longest_b: f64,
    /// Mean longest consecutive y-ion series
    pub mean_longest_y: f64,
    /// Ratio of mean hyperscore to unmodified peak (null for unmodified)
    pub hyperscore_vs_unmodified: Option<f64>,
}

/// A Unimod annotation for a peak
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeakAnnotation {
    /// Unimod record ID (None for intrinsic like "Unmodified")
    pub unimod_id: Option<u32>,
    /// Modification name
    pub name: String,
    /// Unimod monoisotopic mass in Daltons
    pub delta_mass: f64,
    /// Mass error in Daltons (observed - theoretical)
    pub mass_error_da: f64,
    /// Mass error in ppm
    pub mass_error_ppm: f64,
    /// Possible modification sites from Unimod
    pub sites: Vec<String>,
    /// Unimod classification
    pub classification: String,
    /// Source: "unimod", "intrinsic", or "combination"
    pub source: String,
}

/// A detected peak in the delta-mass distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peak {
    /// Rank by count (1 = most frequent)
    pub rank: u32,
    /// Peak center delta mass in Daltons
    pub delta_mass: f64,
    /// Number of PSMs in this peak
    pub count: usize,
    /// Percentage of total PSMs
    pub count_pct: f64,
    /// Sum of ms2_intensity for PSMs in peak
    pub intensity_sum: f64,
    /// Percentage of total intensity
    pub intensity_pct: f64,
    /// Prominence value (height above local baseline)
    #[serde(default)]
    pub prominence: usize,
    /// Confidence metrics from Sage output
    pub confidence: PeakConfidence,
    /// Unimod annotations (may be multiple if ambiguous)
    pub annotations: Vec<PeakAnnotation>,
    /// True if 2+ Unimod matches within tolerance
    pub ambiguous: bool,
    /// True if no Unimod match within tolerance
    pub unannotated: bool,
    /// Count of PSMs folded into this peak from satellites
    #[serde(default)]
    pub folded_count: usize,
    /// Representative m/z of this peak: intensity-weighted MEAN m/z of its
    /// constituent PSMs (reconstructed via `psm_mz`). Threaded through so
    /// `mass_error_ppm` can be computed at the peak's actual m/z instead of the
    /// old hardcoded 1000-Da assumption. Mean (not median) matches the
    /// intensity-weighting used everywhere else in the pipeline (dual-readout
    /// lock); the choice barely matters as a peak's PSMs form a tight m/z cluster
    /// for a given delta. INVARIANT: must fall within [min, max] m/z of the
    /// peak's constituent PSMs — asserted at construction.
    #[serde(default)]
    pub representative_mz: f64,
    /// Indices of the PSMs assigned to this peak, into the run's PSM slice.
    ///
    /// Not serialized: the committed result schema does not change, and the
    /// determinism guard keeps comparing the same bytes. This exists so a caller
    /// can ask what a peak actually contains WITHOUT reconstructing membership
    /// from `delta_mass` and a guessed tolerance. Reconstructing it is how two
    /// earlier check scripts produced false answers.
    #[serde(skip)]
    pub psm_indices: Vec<usize>,
}

/// Summary statistics for mod discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDiscoverySummary {
    /// Total PSMs analyzed
    pub total_psms: usize,
    /// PSMs with delta mass near zero (unmodified)
    pub psms_near_zero: usize,
    /// Percentage near zero
    pub psms_near_zero_pct: f64,
    /// PSMs with significant delta mass (modified)
    pub psms_modified: usize,
    /// Percentage modified
    pub psms_modified_pct: f64,
    /// Number of unique histogram bins with count > 0
    pub unique_bins: usize,
    /// Bin width used
    pub bin_width_da: f64,
}

/// Calibration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationResult {
    /// Calibration mode used
    #[serde(default)]
    pub mode: CalibrationMode,
    /// Apex offset in Daltons (DaScalar mode: subtracted from all deltas; 0.0 otherwise)
    pub apex_offset_da: f64,
    /// Fitted ppm offset (PpmConstant mode only; applied per-PSM against each PSM's m/z)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ppm_offset: Option<f64>,
    /// Number of PSMs used for calibration (near-zero population)
    pub calibration_psm_count: usize,
    /// Total intensity of calibration PSMs
    pub calibration_intensity: f64,
    /// Whether calibration was applied
    pub applied: bool,
    /// Intensity-weighted median of the near-zero population AFTER calibration (Da).
    /// Reported per-mode rather than asserted: DaScalar centers by construction, but a
    /// ppm-linear correction pivots around m/z and may leave the zero apex slightly off —
    /// which is a finding, not a failure. Compare across modes.
    #[serde(default)]
    pub post_calibration_zero_center_da: f64,
    /// Warning if offset is large (possible instrument drift)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// Folding statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldingStats {
    /// PSMs folded to zero (isotope misassignments)
    pub folded_to_zero_count: usize,
    /// Intensity folded to zero
    pub folded_to_zero_intensity: f64,
    /// PSMs folded onto other peaks (satellites)
    pub satellite_fold_count: usize,
    /// Intensity folded onto other peaks
    pub satellite_fold_intensity: f64,
    /// Breakdown by k value (k -> count). BTreeMap (not HashMap) so JSON key order
    /// is deterministic — this field is serialized into the discover output and a
    /// HashMap's randomized iteration order made byte-identical output impossible,
    /// breaking Tier 3 regression snapshots. See NOTES Phase 8 determinism entry.
    pub fold_by_k: BTreeMap<i32, usize>,
}

/// Isotope error distribution view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsotopeErrorView {
    /// Description of this view
    pub description: String,
    /// Distribution of isotope errors
    pub distribution: Vec<IsotopeErrorBin>,
}

/// A bin in the isotope error distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsotopeErrorBin {
    /// Isotope error value
    pub isotope_error: i32,
    /// Count of PSMs
    pub count: usize,
    /// Percentage of total
    pub pct: f64,
}

/// Annotation settings used
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationSettings {
    /// Match tolerance in Daltons
    pub match_tolerance_da: f64,
    /// Classifications excluded from matching
    pub excluded_classifications: Vec<String>,
}

/// The discovery settings that shaped the peaks, recorded in the output.
///
/// Without this, a committed JSON cannot be audited back to the run that made it.
/// That gap is not hypothetical: `nofixedmods/` and `full-run/` were produced by
/// `discover` and `analyze`, whose `min_peak_count` defaults differ (10 vs 5),
/// neither JSON recorded the value, and a session was spent working out whether
/// the difference came from the config or the build. `peak_assignment_mode` would
/// have been the second such gap. See NOTES, "Config-provenance gap".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverySettings {
    /// Histogram bin width in Daltons
    pub bin_width_da: f64,
    /// Minimum PSM count for a bin to be a peak candidate
    pub min_peak_count: usize,
    /// Tolerance for merging bins into one peak
    pub peak_merge_tolerance_da: f64,
    /// Prominence threshold (fraction of bin count above baseline)
    pub prominence_threshold: f64,
    /// How PSMs were grouped into peaks
    pub peak_assignment_mode: PeakAssignmentMode,
    /// Mass calibration mode
    pub calibration_mode: CalibrationMode,
    /// Whether neutron fold-to-zero ran
    pub enable_neutron_folding: bool,
    /// Whether satellite folding ran
    pub enable_satellite_folding: bool,
}

impl From<&ModDiscoveryConfig> for DiscoverySettings {
    fn from(c: &ModDiscoveryConfig) -> Self {
        Self {
            bin_width_da: c.bin_width_da,
            min_peak_count: c.min_peak_count,
            peak_merge_tolerance_da: c.peak_merge_tolerance_da,
            prominence_threshold: c.prominence_threshold,
            peak_assignment_mode: c.peak_assignment_mode,
            calibration_mode: c.calibration_mode,
            enable_neutron_folding: c.enable_neutron_folding,
            enable_satellite_folding: c.enable_satellite_folding,
        }
    }
}

/// Complete mod discovery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDiscoveryResult {
    /// Summary statistics
    pub summary: ModDiscoverySummary,
    /// Calibration result
    pub calibration: CalibrationResult,
    /// Folding statistics
    pub folding: FoldingStats,
    /// Histogram bins (sparse - only non-zero bins)
    pub histogram: Vec<HistogramBin>,
    /// Detected peaks, ranked by count
    pub peaks: Vec<Peak>,
    /// Isotope error distribution view
    pub isotope_error_view: IsotopeErrorView,
    /// Annotation settings used
    pub annotation_settings: AnnotationSettings,
    /// Discovery settings used — provenance for every number above
    pub discovery_settings: DiscoverySettings,
}

/// Run mod discovery on Sage results
pub fn run_mod_discovery(
    results: &SageResults,
    unimod: &UnimodDb,
    config: &ModDiscoveryConfig,
) -> ModDiscoveryResult {
    // Step 1: Mass calibration (mode selects the parallel path)
    let (mut calibrated_deltas, calibration) =
        compute_calibration(&results.psms, config.calibration_mode);

    // Invariant 1: PSM-count conservation through calibration.
    assert_eq!(
        calibrated_deltas.len(),
        results.psms.len(),
        "calibration must not add or drop PSMs"
    );

    // Step 2: PSM-level neutron fold-to-zero (BEFORE histogram building)
    // Each PSM folds iff |psm_delta − k×spacing| < fold_tolerance(k) for some k
    let mut folding = FoldingStats {
        folded_to_zero_count: 0,
        folded_to_zero_intensity: 0.0,
        satellite_fold_count: 0,
        satellite_fold_intensity: 0.0,
        fold_by_k: BTreeMap::new(),
    };

    // Count PSMs at exactly 0.0 BEFORE fold (pre-fold zero population)
    let pre_fold_zero_count = calibrated_deltas.iter().filter(|d| **d == 0.0).count();
    let pre_fold_near_zero_count = calibrated_deltas
        .iter()
        .filter(|d| d.abs() < NEAR_ZERO_THRESHOLD_DA)
        .count();

    if config.enable_neutron_folding {
        fold_isotope_psms_to_zero(
            &mut calibrated_deltas,
            &results.psms,
            &mut folding,
            config.fold_tolerance_da,
        );
    }

    // Count PSMs at exactly 0.0 AFTER fold
    let post_fold_zero_count = calibrated_deltas.iter().filter(|d| **d == 0.0).count();
    let post_fold_near_zero_count = calibrated_deltas
        .iter()
        .filter(|d| d.abs() < NEAR_ZERO_THRESHOLD_DA)
        .count();

    // DIAGNOSTIC: Print the four raw values
    log::info!(
        "DIAGNOSTIC: calibrated_deltas.len() = {} (invariant)",
        calibrated_deltas.len()
    );
    log::info!(
        "DIAGNOSTIC: PSMs with delta exactly 0.0: pre-fold={}, post-fold={}, diff={}",
        pre_fold_zero_count,
        post_fold_zero_count,
        post_fold_zero_count - pre_fold_zero_count
    );
    log::info!(
        "DIAGNOSTIC: folded_to_zero_count (counter) = {}",
        folding.folded_to_zero_count
    );
    log::info!(
        "DIAGNOSTIC: PSMs near zero (|d|<0.1): pre-fold={}, post-fold={}",
        pre_fold_near_zero_count,
        post_fold_near_zero_count
    );

    // Invariant 3: fold-to-zero conservation — the counter must equal the net PSMs
    // that arrived at exactly 0.0. (Was a DIAGNOSTIC log; promoted to a gating assert
    // per verification discipline: an invariant checked in prose can report any number.)
    assert_eq!(
        folding.folded_to_zero_count,
        post_fold_zero_count - pre_fold_zero_count,
        "fold-to-zero counter must equal net PSMs moved to Δ=0"
    );

    // Step 3: Build histogram with post-fold deltas
    // Δ=0 bin now naturally includes folded PSMs
    let (mut histogram, total_intensity) =
        build_histogram_from_deltas(&calibrated_deltas, &results.psms, config.bin_width_da);

    // Step 4: Prominence-based peak detection
    // Note: With PSM-level folding, no bins are marked as folded, so no PSM exclusion needed
    let mut peaks = detect_peaks_with_prominence(
        &histogram,
        &calibrated_deltas,
        &results.psms,
        config,
        total_intensity,
    );

    // Step 4.5: Satellite folding onto detected peaks
    if config.enable_satellite_folding && !peaks.is_empty() {
        fold_satellites_onto_peaks(
            &mut histogram,
            &mut peaks,
            &mut folding,
            config.fold_tolerance_da,
        );
    }

    // Step 5: Annotate peaks with Unimod (with excluded classifications filter)
    let unmodified_hyperscore =
        annotate_peaks(&mut peaks, unimod, &config.excluded_classifications);

    // DIAGNOSTIC: Print fine-grained delta distribution for deamidation peak
    // Find the deamidation peak (should be around 0.98 Da)
    let deamidation_peak = peaks
        .iter()
        .find(|p| p.delta_mass > 0.97 && p.delta_mass < 0.99);
    if let Some(peak) = deamidation_peak {
        let merge_tol = config.peak_merge_tolerance_da.max(config.bin_width_da);
        let peak_center = peak.delta_mass;

        // Collect PSMs within merge tolerance of peak center (what the peak actually contains)
        let peak_psms: Vec<f64> = calibrated_deltas
            .iter()
            .filter(|d| (**d - peak_center).abs() <= merge_tol)
            .copied()
            .collect();

        // Also collect the wider region for context
        let wide_region: Vec<f64> = calibrated_deltas
            .iter()
            .filter(|d| **d > 0.92 && **d < 1.00)
            .copied()
            .collect();

        // Build 1 mDa bins for wide region
        let mut fine_bins: std::collections::BTreeMap<i32, usize> =
            std::collections::BTreeMap::new();
        for d in &wide_region {
            let bin = (d * 1000.0).round() as i32;
            *fine_bins.entry(bin).or_insert(0) += 1;
        }

        log::info!("DIAGNOSTIC: Deamidation peak analysis:");
        log::info!(
            "  Peak center: {:.4} Da, reported count: {}",
            peak_center,
            peak.count
        );
        log::info!("  Merge tolerance: {:.4} Da", merge_tol);
        log::info!(
            "  PSMs within merge window [{:.4}, {:.4}]: {}",
            peak_center - merge_tol,
            peak_center + merge_tol,
            peak_psms.len()
        );
        log::info!("  Wide region (0.92-1.00 Da) distribution:");
        for (bin_mda, count) in &fine_bins {
            log::info!("    {:.3} Da: {} PSMs", *bin_mda as f64 / 1000.0, count);
        }
        let total_wide: usize = fine_bins.values().sum();
        log::info!("  Total in wide region: {}", total_wide);

        // Find max delta in peak population
        if !peak_psms.is_empty() {
            let max_delta = peak_psms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let min_delta = peak_psms.iter().cloned().fold(f64::INFINITY, f64::min);
            log::info!("  Peak PSMs range: [{:.4}, {:.4}] Da", min_delta, max_delta);
        }
        log::info!(
            "  k=1 fold window: [{:.4}, {:.4}] Da",
            C13_C12_DIFF - config.fold_tolerance_da,
            C13_C12_DIFF + config.fold_tolerance_da
        );
    }

    // Step 6: Roll up all near-zero peaks into one "Unmodified" row
    rollup_unmodified_peaks(
        &mut peaks,
        &calibrated_deltas,
        &results.psms,
        total_intensity,
    );

    // Compute hyperscore_vs_unmodified for modified peaks
    if let Some(unmod_score) = unmodified_hyperscore {
        for peak in &mut peaks {
            if peak.delta_mass.abs() > NEAR_ZERO_THRESHOLD_DA {
                peak.confidence.hyperscore_vs_unmodified =
                    Some(peak.confidence.mean_hyperscore / unmod_score);
            }
        }
    }

    // Build summary (using calibrated deltas)
    let psms_near_zero = calibrated_deltas
        .iter()
        .filter(|d| d.abs() < NEAR_ZERO_THRESHOLD_DA)
        .count();
    let total_psms = results.psms.len();

    let summary = ModDiscoverySummary {
        total_psms,
        psms_near_zero,
        psms_near_zero_pct: 100.0 * (psms_near_zero as f64) / (total_psms as f64),
        psms_modified: total_psms - psms_near_zero,
        psms_modified_pct: 100.0 * ((total_psms - psms_near_zero) as f64) / (total_psms as f64),
        unique_bins: histogram.iter().filter(|b| !b.folded).count(),
        bin_width_da: config.bin_width_da,
    };

    // Build isotope error view
    let isotope_error_view =
        build_isotope_error_view(&results.isotope_error_distribution, total_psms);

    ModDiscoveryResult {
        summary,
        calibration,
        folding,
        histogram,
        peaks,
        isotope_error_view,
        annotation_settings: AnnotationSettings {
            match_tolerance_da: unimod.tolerance_da,
            excluded_classifications: config.excluded_classifications.clone(),
        },
        discovery_settings: DiscoverySettings::from(config),
    }
}

/// Reconstruct a PSM's precursor m/z (Da) from expmass + charge.
/// m/z = (M + z·proton) / z. Falls back to expmass for z==0 (should not occur).
#[inline]
fn psm_mz(psm: &Psm) -> f64 {
    if psm.charge == 0 {
        psm.expmass
    } else {
        (psm.expmass + psm.charge as f64 * PROTON_MASS) / psm.charge as f64
    }
}

/// Intensity-weighted MEAN m/z of a set of PSMs — the peak's representative m/z
/// for ppm computation. Mean (not median) matches the intensity-weighting used
/// elsewhere in the pipeline (dual-readout lock).
///
/// INVARIANT (lead-with-the-invariant discipline): a weighted mean cannot land
/// outside the [min, max] of its inputs. If it does, the weighting or the
/// `psm_mz` reconstruction is wrong — this catches a whole class of threading
/// bugs the per-site ppm formula test cannot. Asserted here in debug builds.
/// Falls back to unweighted mean when total intensity is 0; returns 0.0 for an
/// empty slice (no PSMs → no meaningful m/z).
fn representative_mz(psms: &[&Psm]) -> f64 {
    if psms.is_empty() {
        return 0.0;
    }
    let mut min_mz = f64::INFINITY;
    let mut max_mz = f64::NEG_INFINITY;
    let mut sum_mz_int = 0.0;
    let mut sum_int = 0.0;
    let mut sum_mz = 0.0;
    for p in psms {
        let mz = psm_mz(p);
        min_mz = min_mz.min(mz);
        max_mz = max_mz.max(mz);
        sum_mz_int += mz * p.ms2_intensity;
        sum_int += p.ms2_intensity;
        sum_mz += mz;
    }
    let rep = if sum_int > 0.0 {
        sum_mz_int / sum_int
    } else {
        sum_mz / psms.len() as f64
    };
    // Conservation invariant: weighted mean stays within the input range.
    // Small epsilon absorbs floating-point rounding at the boundary.
    debug_assert!(
        rep >= min_mz - 1e-6 && rep <= max_mz + 1e-6,
        "representative_mz {} outside PSM m/z range [{}, {}] — weighting or psm_mz reconstruction is wrong",
        rep, min_mz, max_mz
    );
    rep
}

/// True ppm mass error at a peak's actual m/z: `error_da / mz × 1e6`.
///
/// Replaces the historical `error_da × 1000.0` shortcut, which equals true ppm
/// only at exactly 1000 Da and was off by ~1.5–3× for real tryptic peptides
/// (~800–3000 Da). See NOTES "Deferred enhancements" (mass_error_ppm bug).
/// Falls back to the 1000-Da form when `mz` is non-positive (no reconstructed
/// m/z available), which keeps output finite rather than producing NaN/Inf.
#[inline]
fn ppm_at_mz(error_da: f64, mz: f64) -> f64 {
    if mz > 0.0 {
        error_da / mz * 1e6
    } else {
        error_da * 1000.0
    }
}

/// Intensity-weighted median of `(value, intensity)` pairs. Input need not be sorted.
/// Returns 0.0 for an empty slice.
fn intensity_weighted_median(pairs: &mut [(f64, f64)]) -> f64 {
    if pairs.is_empty() {
        return 0.0;
    }
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let total: f64 = pairs.iter().map(|(_, i)| i).sum();
    let half = total / 2.0;
    let mut cumulative = 0.0;
    for (value, intensity) in pairs.iter() {
        cumulative += intensity;
        if cumulative >= half {
            return *value;
        }
    }
    pairs.last().map(|(v, _)| *v).unwrap_or(0.0)
}

/// Compute mass calibration for the selected mode.
///
/// Returns `(calibrated_deltas, result)`. All modes conserve PSM count (asserted).
/// The near-zero population (|Δ|<0.1 Da) defines the offset in every fitted mode.
fn compute_calibration(psms: &[Psm], mode: CalibrationMode) -> (Vec<f64>, CalibrationResult) {
    // Near-zero population used to fit the offset (both fitted modes).
    let near_zero: Vec<&Psm> = psms
        .iter()
        .filter(|p| p.delta_mass_corrected.abs() < NEAR_ZERO_THRESHOLD_DA)
        .collect();
    let calibration_intensity: f64 = near_zero.iter().map(|p| p.ms2_intensity).sum();

    // No calibration, or no population to fit from → identity transform.
    if mode == CalibrationMode::None || near_zero.is_empty() {
        let calibrated_deltas: Vec<f64> = psms.iter().map(|p| p.delta_mass_corrected).collect();
        let center = post_calibration_zero_center(&calibrated_deltas, psms);
        let warning = if mode != CalibrationMode::None && near_zero.is_empty() {
            Some("No near-zero PSMs for calibration".to_string())
        } else {
            None
        };
        let calibration = CalibrationResult {
            mode,
            apex_offset_da: 0.0,
            ppm_offset: None,
            calibration_psm_count: near_zero.len(),
            calibration_intensity,
            applied: false,
            post_calibration_zero_center_da: center,
            warning,
        };
        return (calibrated_deltas, calibration);
    }

    let (calibrated_deltas, apex_offset_da, ppm_offset, warning) = match mode {
        CalibrationMode::None => unreachable!("handled above"),

        CalibrationMode::DaScalar => {
            // Existing behavior: intensity-weighted median of near-zero deltas (Da),
            // subtracted from every PSM as a constant.
            let mut pairs: Vec<(f64, f64)> = near_zero
                .iter()
                .map(|p| (p.delta_mass_corrected, p.ms2_intensity))
                .collect();
            let apex_offset = intensity_weighted_median(&mut pairs);
            let calibrated: Vec<f64> = psms
                .iter()
                .map(|p| p.delta_mass_corrected - apex_offset)
                .collect();
            let warning = if apex_offset.abs() > 0.005 {
                Some(format!(
                    "Large apex_offset ({:.3} mDa) may indicate instrument drift",
                    apex_offset * 1000.0
                ))
            } else {
                None
            };
            (calibrated, apex_offset, None, warning)
        }

        CalibrationMode::PpmConstant => {
            // Fit ONE ppm offset from the near-zero population using each PSM's own m/z,
            // then apply it per-PSM: correction_i (Da) = ppm_offset * mz_i / 1e6.
            let mut pairs: Vec<(f64, f64)> = near_zero
                .iter()
                .map(|p| {
                    let mz = psm_mz(p);
                    let ppm = if mz != 0.0 {
                        p.delta_mass_corrected / mz * 1e6
                    } else {
                        0.0
                    };
                    (ppm, p.ms2_intensity)
                })
                .collect();
            let ppm_offset = intensity_weighted_median(&mut pairs);
            let calibrated: Vec<f64> = psms
                .iter()
                .map(|p| p.delta_mass_corrected - ppm_offset * psm_mz(p) / 1e6)
                .collect();
            let warning = if ppm_offset.abs() > 5.0 {
                Some(format!(
                    "Large ppm_offset ({:.2} ppm) may indicate instrument drift",
                    ppm_offset
                ))
            } else {
                None
            };
            (calibrated, 0.0, Some(ppm_offset), warning)
        }
    };

    // Invariant 1: PSM-count conservation.
    assert_eq!(
        calibrated_deltas.len(),
        psms.len(),
        "calibration must not add or drop PSMs"
    );

    // Invariant 2 (measured, not asserted): where the zero apex lands after calibration.
    // Reported per-mode; DaScalar is ~0 by construction, PpmConstant may pivot slightly.
    let center = post_calibration_zero_center(&calibrated_deltas, psms);

    let calibration = CalibrationResult {
        mode,
        apex_offset_da,
        ppm_offset,
        calibration_psm_count: near_zero.len(),
        calibration_intensity,
        applied: true,
        post_calibration_zero_center_da: center,
        warning,
    };

    (calibrated_deltas, calibration)
}

/// Intensity-weighted median of the near-zero (|Δ|<0.1 Da) population of the
/// CALIBRATED deltas — the "where did the zero apex end up" diagnostic (invariant 2).
fn post_calibration_zero_center(calibrated_deltas: &[f64], psms: &[Psm]) -> f64 {
    let mut pairs: Vec<(f64, f64)> = calibrated_deltas
        .iter()
        .zip(psms.iter())
        .filter(|(d, _)| d.abs() < NEAR_ZERO_THRESHOLD_DA)
        .map(|(d, p)| (*d, p.ms2_intensity))
        .collect();
    intensity_weighted_median(&mut pairs)
}

/// Build histogram from calibrated delta masses
fn build_histogram_from_deltas(
    deltas: &[f64],
    psms: &[Psm],
    bin_width: f64,
) -> (Vec<HistogramBin>, f64) {
    // Track: (count, intensity_sum, sum_delta_times_intensity)
    let mut bins: HashMap<i64, (usize, f64, f64)> = HashMap::new();
    let mut total_intensity = 0.0;

    for (delta, psm) in deltas.iter().zip(psms.iter()) {
        let bin_idx = (delta / bin_width).round() as i64;
        let entry = bins.entry(bin_idx).or_insert((0, 0.0, 0.0));
        entry.0 += 1;
        entry.1 += psm.ms2_intensity;
        entry.2 += delta * psm.ms2_intensity; // For weighted apex
        total_intensity += psm.ms2_intensity;
    }

    // Convert to sorted vector (sparse representation - only non-zero bins)
    let mut histogram: Vec<HistogramBin> = bins
        .into_iter()
        .map(|(bin_idx, (count, intensity_sum, sum_delta_intensity))| {
            let bin_center = (bin_idx as f64) * bin_width;
            // Compute weighted apex; guard against intensity_sum == 0 (fall back to bin_center)
            let weighted_apex = if intensity_sum > 0.0 {
                sum_delta_intensity / intensity_sum
            } else {
                bin_center
            };
            HistogramBin {
                bin_center,
                weighted_apex,
                count,
                intensity_sum,
                intensity_pct: if total_intensity > 0.0 {
                    100.0 * intensity_sum / total_intensity
                } else {
                    0.0
                },
                folded: false,
                fold_target: None,
            }
        })
        .collect();

    // Sort by bin center
    histogram.sort_by(|a, b| a.bin_center.partial_cmp(&b.bin_center).unwrap());

    (histogram, total_intensity)
}

/// PSM-level isotope fold-to-zero (monoisotope misassignment removal)
/// Each PSM folds iff |psm_delta − k×spacing| < fold_tolerance(k) for some k
/// This is the correct approach: operates on individual PSMs, not bins
fn fold_isotope_psms_to_zero(
    calibrated_deltas: &mut [f64],
    psms: &[Psm],
    folding: &mut FoldingStats,
    base_tolerance: f64,
) {
    for (i, delta) in calibrated_deltas.iter_mut().enumerate() {
        // Skip PSMs already near zero
        if delta.abs() < NEAR_ZERO_THRESHOLD_DA {
            continue;
        }

        // Check each k value
        for k in [-3, -2, -1, 1, 2, 3] {
            let target = k as f64 * C13_C12_DIFF;
            let k_tolerance =
                fold_tolerance_for_k(k, base_tolerance, DEFAULT_FOLD_TOLERANCE_PER_STEP_DA);
            let distance = (*delta - target).abs();

            if distance < k_tolerance {
                // This PSM is an isotope misassignment - fold to zero
                folding.folded_to_zero_count += 1;
                folding.folded_to_zero_intensity += psms[i].ms2_intensity;
                *folding.fold_by_k.entry(k).or_insert(0) += 1;

                log::debug!(
                    "Folded PSM at {:.4} Da (k={}, dist={:.1} mDa, tol={:.1} mDa) to zero",
                    *delta,
                    k,
                    distance * 1000.0,
                    k_tolerance * 1000.0
                );

                // Move this PSM to Δ=0
                *delta = 0.0;
                break; // Only fold once per PSM
            }
        }
    }

    log::info!(
        "PSM-level fold-to-zero: {} PSMs folded to Δ=0",
        folding.folded_to_zero_count
    );
}

/// Detect peaks using prominence-based filtering
fn detect_peaks_with_prominence(
    histogram: &[HistogramBin],
    calibrated_deltas: &[f64],
    psms: &[Psm],
    config: &ModDiscoveryConfig,
    total_intensity: f64,
) -> Vec<Peak> {
    // Filter to bins above minimum count
    // With PSM-level fold-to-zero, no bins are marked as folded at detection time
    let candidate_bins: Vec<&HistogramBin> = histogram
        .iter()
        .filter(|b| b.count >= config.min_peak_count)
        .collect();

    // Compute prominence for each candidate
    let mut peaks_with_prominence: Vec<(usize, &HistogramBin, usize)> = Vec::new();

    for (idx, bin) in candidate_bins.iter().enumerate() {
        let prominence = compute_prominence(bin, &candidate_bins, 0.5); // Search ±0.5 Da

        // Accept if prominence > threshold * count
        let threshold = (config.prominence_threshold * bin.count as f64) as usize;
        if prominence > threshold {
            peaks_with_prominence.push((idx, bin, prominence));
        }
    }

    // Sort by count descending
    peaks_with_prominence.sort_by_key(|b| std::cmp::Reverse(b.1.count));

    // Select peak CENTERS, then assign PSMs to them exclusively.
    //
    // These are two separate decisions, and the peak-assignment defect came from
    // conflating them: centers were chosen with one tolerance while PSMs were
    // collected with a window twice as wide, so two centers could take the same
    // PSM. Assignment is now nearest-center and exclusive in BOTH modes, so the
    // modes differ only in which bins are allowed to be separate centers.
    let total_psms = psms.len();

    // Use 1.0 × bin_width for merge tolerance - tight to keep real peaks separate
    // The unmodified roll-up handles the zero smear by classification, not merge
    let effective_merge_tolerance = config
        .peak_merge_tolerance_da
        .max(1.0 * config.bin_width_da);

    // Bin centers are `bin_idx as f64 * bin_width`, so comparing them as floats is
    // unreliable: `99.0*0.01 - 98.0*0.01` is 0.010000000000000009, which is greater
    // than 0.01, and an adjacent-bin pair escapes a `<= 0.01` test. Compare bin
    // INDICES, which are exact.
    let bin_index = |center: f64| (center / config.bin_width_da).round() as i64;
    let max_merge_gap: i64 = match config.peak_assignment_mode {
        // Merge: bins within the merge tolerance are ONE peak (tolerance in bins).
        PeakAssignmentMode::Merge => {
            (effective_merge_tolerance / config.bin_width_da).round() as i64
        }
        // Split: every prominent bin is its own center. Only an exact repeat merges.
        PeakAssignmentMode::Split => 0,
    };

    let mut centers: Vec<(i64, usize)> = Vec::new(); // (bin index, prominence)
    for (_, bin, prominence) in peaks_with_prominence {
        let idx = bin_index(bin.bin_center);
        if centers
            .iter()
            .any(|(c, _)| (idx - c).abs() <= max_merge_gap)
        {
            continue;
        }
        centers.push((idx, prominence));
        if centers.len() >= config.max_peaks {
            break;
        }
    }

    // Exclusive assignment. Every PSM within the tolerance of at least one center
    // goes to its NEAREST center, and to that one only. `centers` is in descending
    // count order, and the `<=` tie-break keeps the earlier entry, so an exactly
    // equidistant PSM goes to the larger peak. Deterministic — the determinism
    // guard depends on it.
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); centers.len()];
    for (i, delta) in calibrated_deltas.iter().enumerate() {
        let mut best: Option<(usize, f64)> = None;
        for (ci, (idx, _)) in centers.iter().enumerate() {
            let center = *idx as f64 * config.bin_width_da;
            let distance = (delta - center).abs();
            if distance > effective_merge_tolerance {
                continue;
            }
            match best {
                Some((_, best_distance)) if best_distance <= distance => {}
                _ => best = Some((ci, distance)),
            }
        }
        if let Some((ci, _)) = best {
            members[ci].push(i);
        }
    }

    // Invariant 4: exclusive PSM assignment. A PSM belongs to at most one peak.
    // Nearest-center assignment makes this true by construction, so this is a gate
    // on the construction, not a diagnostic. It is kept because the previous code
    // also looked correct. `claimed_by` maps a PSM index to its owning bin index.
    let mut claimed_by: HashMap<usize, i64> = HashMap::new();
    let mut collisions: Vec<(f64, f64, usize)> = Vec::new();
    for (ci, member_indices) in members.iter().enumerate() {
        let idx = centers[ci].0;
        let mut collided_with: BTreeMap<i64, usize> = BTreeMap::new();
        for &i in member_indices {
            match claimed_by.get(&i) {
                Some(&owner_idx) => *collided_with.entry(owner_idx).or_insert(0) += 1,
                None => {
                    claimed_by.insert(i, idx);
                }
            }
        }
        for (owner_idx, shared) in collided_with {
            let owner_center = owner_idx as f64 * config.bin_width_da;
            let center = idx as f64 * config.bin_width_da;
            log::error!(
                "PEAK OVERLAP: peak at {:.4} Da shares {} PSMs with peak at {:.4} Da \
                 (centers {} bins apart, merge tolerance {:.9} Da)",
                center,
                shared,
                owner_center,
                (idx - owner_idx).abs(),
                effective_merge_tolerance
            );
            collisions.push((owner_center, center, shared));
        }
    }
    let duplicate_claims: usize = collisions.iter().map(|&(_, _, n)| n).sum();
    assert!(
        collisions.is_empty(),
        "exclusive PSM assignment violated: {} duplicate PSM claims across {} peak pairs; \
         first pair {:.4} Da / {:.4} Da shares {} PSMs (merge tolerance {:.9} Da)",
        duplicate_claims,
        collisions.len(),
        collisions[0].0,
        collisions[0].1,
        collisions[0].2,
        effective_merge_tolerance
    );

    // Build the peaks from their member sets.
    let mut peaks: Vec<Peak> = Vec::new();
    for (ci, (idx, prominence)) in centers.iter().enumerate() {
        let peak_psm_indices = &members[ci];
        if peak_psm_indices.is_empty() {
            continue;
        }
        let bin_center = *idx as f64 * config.bin_width_da;
        let peak_psms: Vec<&Psm> = peak_psm_indices.iter().map(|&i| &psms[i]).collect();

        // Compute peak statistics
        let count = peak_psms.len();
        let intensity_sum: f64 = peak_psms.iter().map(|p| p.ms2_intensity).sum();

        // Compute intensity-weighted average delta mass
        let delta_mass = if intensity_sum > 0.0 {
            peak_psm_indices
                .iter()
                .map(|&i| calibrated_deltas[i] * psms[i].ms2_intensity)
                .sum::<f64>()
                / intensity_sum
        } else {
            bin_center
        };

        // Compute confidence metrics
        let confidence = compute_confidence(&peak_psms);

        peaks.push(Peak {
            rank: 0, // Will be set after sorting
            delta_mass,
            count,
            count_pct: 100.0 * (count as f64) / (total_psms as f64),
            intensity_sum,
            intensity_pct: if total_intensity > 0.0 {
                100.0 * intensity_sum / total_intensity
            } else {
                0.0
            },
            prominence: *prominence,
            confidence,
            annotations: Vec::new(),
            ambiguous: false,
            unannotated: false,
            folded_count: 0,
            representative_mz: representative_mz(&peak_psms),
            psm_indices: peak_psm_indices.clone(),
        });
    }

    // Sort by count and assign ranks
    peaks.sort_by_key(|b| std::cmp::Reverse(b.count));
    for (i, peak) in peaks.iter_mut().enumerate() {
        peak.rank = (i + 1) as u32;
    }

    peaks
}

/// Compute prominence for a bin (height above local baseline)
fn compute_prominence(bin: &HistogramBin, all_bins: &[&HistogramBin], search_range: f64) -> usize {
    let bin_center = bin.bin_center;
    let bin_count = bin.count;

    // Find left base: minimum count between this bin and next higher bin to the left
    let mut left_base = 0usize;
    for other in all_bins.iter() {
        if other.bin_center < bin_center
            && other.bin_center > bin_center - search_range
            && other.count > bin_count
        {
            // Found a higher bin to the left, find minimum between
            for between in all_bins.iter() {
                if between.bin_center > other.bin_center
                    && between.bin_center < bin_center
                    && (left_base == 0 || between.count < left_base)
                {
                    left_base = between.count;
                }
            }
            break;
        }
    }

    // Find right base: minimum count between this bin and next higher bin to the right
    let mut right_base = 0usize;
    for other in all_bins.iter() {
        if other.bin_center > bin_center
            && other.bin_center < bin_center + search_range
            && other.count > bin_count
        {
            // Found a higher bin to the right, find minimum between
            for between in all_bins.iter() {
                if between.bin_center < other.bin_center
                    && between.bin_center > bin_center
                    && (right_base == 0 || between.count < right_base)
                {
                    right_base = between.count;
                }
            }
            break;
        }
    }

    // Prominence = count - max(left_base, right_base)
    let base = left_base.max(right_base);
    bin_count.saturating_sub(base)
}

/// Fold satellite peaks onto their parent peaks
/// Uses k-scaled tolerance: base + (|k| - 1) × per_step
/// CRITICAL: A satellite bin is ONLY folded onto a non-zero peak if it is NOT
/// also within tolerance of a zero k-offset. Zero takes priority.
fn fold_satellites_onto_peaks(
    histogram: &mut [HistogramBin],
    peaks: &mut [Peak],
    folding: &mut FoldingStats,
    base_tolerance: f64,
) {
    // DIAGNOSTIC: Track what gets folded onto deamidation
    let mut deamidation_satellites: Vec<(f64, usize, i32)> = Vec::new(); // (bin_apex, count, k)

    // For each peak P (excluding zero), check for satellites at P + k*C13_C12_DIFF
    // CRITICAL: Only POSITIVE k values (M+1, M+2, M+3) — isotope envelopes only go UP
    // There is no M−1 peak; the monoisotope is the lightest peak in any envelope
    for peak in peaks.iter_mut() {
        if peak.delta_mass.abs() < NEAR_ZERO_THRESHOLD_DA {
            continue; // Skip unmodified peak (already handled by fold-to-zero)
        }

        let is_deamidation_peak = peak.delta_mass > 0.97 && peak.delta_mass < 0.99;
        let parent_count_before_folding = peak.count;
        let mut satellites_folded_this_peak = 0usize;

        // ONLY positive k: isotope envelopes go M+1, M+2, M+3 — never M−1
        for k in [1, 2, 3] {
            // POPULATION-LEVEL ABUNDANCE CAP: total satellites folded must be < parent count
            // Isotope envelopes: M+1 + M+2 + M+3 < M+0 for peptides at typical masses
            if satellites_folded_this_peak >= parent_count_before_folding {
                log::info!(
                    "ABUNDANCE CAP: Peak at {:.4} Da already has {} satellites >= {} parent count. Stopping satellite fold.",
                    peak.delta_mass, satellites_folded_this_peak, parent_count_before_folding
                );
                break; // Stop folding more satellites onto this peak
            }

            let satellite_delta = peak.delta_mass + k as f64 * C13_C12_DIFF;
            // k-scaled tolerance: k=1 → 12 mDa, k=2 → 16.5 mDa, k=3 → 21 mDa
            let k_tolerance =
                fold_tolerance_for_k(k, base_tolerance, DEFAULT_FOLD_TOLERANCE_PER_STEP_DA);

            // Find bins within k_tolerance of satellite_delta
            for bin in histogram.iter_mut() {
                if bin.folded {
                    continue; // Already folded
                }

                if bin.count == 0 {
                    continue; // Already drained (by fold-to-zero)
                }

                // Use weighted_apex for fold comparison (not bin_center)
                // This gives the actual PSM apex, not the discretized bin center
                let distance = (bin.weighted_apex - satellite_delta).abs();
                if distance < k_tolerance {
                    // CRITICAL CHECK: Is this bin ALSO within tolerance of a zero k-offset?
                    // If so, it's an unmodified satellite and should NOT be claimed by this peak.
                    let mut is_zero_satellite = false;
                    for zero_k in [-3, -2, -1, 1, 2, 3] {
                        let zero_target = zero_k as f64 * C13_C12_DIFF;
                        let zero_k_tolerance = fold_tolerance_for_k(
                            zero_k,
                            base_tolerance,
                            DEFAULT_FOLD_TOLERANCE_PER_STEP_DA,
                        );
                        if (bin.weighted_apex - zero_target).abs() < zero_k_tolerance {
                            is_zero_satellite = true;
                            break;
                        }
                    }

                    if is_zero_satellite {
                        // This bin belongs to zero, not to this peak
                        // It should have been folded by PSM-level fold-to-zero
                        // If it wasn't, that's a bug in PSM-level folding
                        log::warn!(
                            "Bin at {:.4} Da (apex={:.4}) is within tolerance of BOTH zero k-offset AND peak at {:.4} Da. Skipping satellite fold (zero takes priority).",
                            bin.bin_center, bin.weighted_apex, peak.delta_mass
                        );
                        continue;
                    }

                    // POPULATION-LEVEL CHECK: Would this bin push us over the abundance cap?
                    if satellites_folded_this_peak + bin.count > parent_count_before_folding {
                        log::info!(
                            "ABUNDANCE CAP: Bin at {:.4} Da ({} PSMs) would push peak at {:.4} Da over cap ({} + {} > {}). Skipping.",
                            bin.weighted_apex, bin.count, peak.delta_mass,
                            satellites_folded_this_peak, bin.count, parent_count_before_folding
                        );
                        continue;
                    }

                    // DIAGNOSTIC: Track deamidation satellites
                    if is_deamidation_peak {
                        deamidation_satellites.push((bin.weighted_apex, bin.count, k));
                    }

                    // This is a satellite of this peak - fold it
                    bin.folded = true;
                    bin.fold_target = Some(peak.delta_mass);

                    peak.folded_count += bin.count;
                    peak.count += bin.count;
                    peak.intensity_sum += bin.intensity_sum;
                    satellites_folded_this_peak += bin.count;

                    folding.satellite_fold_count += bin.count;
                    folding.satellite_fold_intensity += bin.intensity_sum;
                    *folding.fold_by_k.entry(k).or_insert(0) += bin.count;

                    log::debug!(
                        "Folded satellite at {:.4} Da (apex={:.4}, k={}, dist={:.1} mDa) onto peak at {:.4} Da: {} PSMs",
                        bin.bin_center, bin.weighted_apex, k, distance * 1000.0, peak.delta_mass, bin.count
                    );

                    // Drain the source bin after folding
                    bin.count = 0;
                    bin.intensity_sum = 0.0;
                }
            }
        }
    }

    // Re-sort peaks by count after folding
    peaks.sort_by_key(|b| std::cmp::Reverse(b.count));
    for (i, peak) in peaks.iter_mut().enumerate() {
        peak.rank = (i + 1) as u32;
    }

    // DIAGNOSTIC: Print what was folded onto deamidation
    if !deamidation_satellites.is_empty() {
        log::info!("DIAGNOSTIC: Satellites folded onto deamidation peak:");
        let total_count: usize = deamidation_satellites.iter().map(|(_, c, _)| c).sum();
        log::info!("  Total satellite PSMs: {}", total_count);
        for (apex, count, k) in &deamidation_satellites {
            let zero_k1_target = C13_C12_DIFF;
            let zero_k2_target = 2.0 * C13_C12_DIFF;
            log::info!(
                "    Bin apex={:.4} Da, count={}, k={}, dist_from_zero_k1={:.1} mDa, dist_from_zero_k2={:.1} mDa",
                apex, count, k,
                (apex - zero_k1_target).abs() * 1000.0,
                (apex - zero_k2_target).abs() * 1000.0
            );
        }
    }
}

/// Compute confidence metrics for a set of PSMs
fn compute_confidence(psms: &[&Psm]) -> PeakConfidence {
    if psms.is_empty() {
        return PeakConfidence {
            mean_hyperscore: 0.0,
            median_hyperscore: 0.0,
            mean_matched_intensity_pct: 0.0,
            mean_longest_b: 0.0,
            mean_longest_y: 0.0,
            hyperscore_vs_unmodified: None,
        };
    }

    let n = psms.len() as f64;

    let mean_hyperscore: f64 = psms.iter().map(|p| p.hyperscore).sum::<f64>() / n;
    let mean_matched_intensity_pct: f64 =
        psms.iter().map(|p| p.matched_intensity_pct).sum::<f64>() / n;
    let mean_longest_b: f64 = psms.iter().map(|p| p.longest_b as f64).sum::<f64>() / n;
    let mean_longest_y: f64 = psms.iter().map(|p| p.longest_y as f64).sum::<f64>() / n;

    // Compute median hyperscore
    let mut scores: Vec<f64> = psms.iter().map(|p| p.hyperscore).collect();
    scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_hyperscore = if scores.len().is_multiple_of(2) {
        (scores[scores.len() / 2 - 1] + scores[scores.len() / 2]) / 2.0
    } else {
        scores[scores.len() / 2]
    };

    PeakConfidence {
        mean_hyperscore,
        median_hyperscore,
        mean_matched_intensity_pct,
        mean_longest_b,
        mean_longest_y,
        hyperscore_vs_unmodified: None, // Set later
    }
}

/// Annotate peaks with Unimod matches, filtering by excluded classifications
/// Returns the mean hyperscore of the unmodified peak (if found)
fn annotate_peaks(
    peaks: &mut [Peak],
    unimod: &UnimodDb,
    excluded_classifications: &[String],
) -> Option<f64> {
    let mut unmodified_hyperscore: Option<f64> = None;

    for peak in peaks.iter_mut() {
        // Special case: near-zero is "Unmodified"
        if peak.delta_mass.abs() < NEAR_ZERO_THRESHOLD_DA {
            peak.annotations.push(PeakAnnotation {
                unimod_id: None,
                name: "Unmodified".to_string(),
                delta_mass: 0.0,
                mass_error_da: peak.delta_mass,
                // True ppm at the peak's actual m/z, not the old 1000-Da form.
                mass_error_ppm: ppm_at_mz(peak.delta_mass, peak.representative_mz),
                sites: vec![],
                classification: "None".to_string(),
                source: "intrinsic".to_string(),
            });
            peak.ambiguous = false;
            peak.unannotated = false;
            unmodified_hyperscore = Some(peak.confidence.mean_hyperscore);
            continue;
        }

        // Look up in Unimod
        let all_matches: Vec<UnimodMatch> = unimod.find_matches(peak.delta_mass);

        // Filter out excluded classifications
        let matches: Vec<UnimodMatch> = all_matches
            .into_iter()
            .filter(|m| {
                let classification = m.entry.classification();
                !excluded_classifications
                    .iter()
                    .any(|exc| classification.contains(exc))
            })
            .collect();

        if matches.is_empty() {
            peak.unannotated = true;
            peak.ambiguous = false;
        } else {
            peak.unannotated = false;
            peak.ambiguous = matches.len() > 1;

            for m in matches {
                peak.annotations.push(PeakAnnotation {
                    unimod_id: Some(m.entry.record_id),
                    name: m.entry.title.clone(),
                    delta_mass: m.entry.mono_mass,
                    mass_error_da: m.mass_error_da,
                    // Recompute ppm at the peak's actual m/z. Do NOT trust
                    // UnimodMatch.mass_error_ppm — it uses the old 1000-Da
                    // shortcut (unimod.rs find_matches). See NOTES ppm bug.
                    mass_error_ppm: ppm_at_mz(m.mass_error_da, peak.representative_mz),
                    sites: m.entry.sites(),
                    classification: m.entry.classification(),
                    source: "unimod".to_string(),
                });
            }
        }
    }

    unmodified_hyperscore
}

/// Roll up all near-zero peaks into one "Unmodified" row
/// Uses UNMODIFIED_ROLLUP_THRESHOLD_DA (0.075 Da) to capture the full zero smear
fn rollup_unmodified_peaks(
    peaks: &mut Vec<Peak>,
    calibrated_deltas: &[f64],
    psms: &[Psm],
    total_intensity: f64,
) {
    // Find all peaks within the rollup threshold
    let (near_zero_peaks, other_peaks): (Vec<Peak>, Vec<Peak>) = peaks
        .drain(..)
        .partition(|p| p.delta_mass.abs() < UNMODIFIED_ROLLUP_THRESHOLD_DA);

    if near_zero_peaks.is_empty() {
        *peaks = other_peaks;
        return;
    }

    // Collect all PSMs within the rollup threshold for recomputation
    let rollup_psm_indices: Vec<usize> = calibrated_deltas
        .iter()
        .enumerate()
        .filter(|(_, d)| d.abs() < UNMODIFIED_ROLLUP_THRESHOLD_DA)
        .map(|(i, _)| i)
        .collect();

    let rollup_psms: Vec<&Psm> = rollup_psm_indices.iter().map(|&i| &psms[i]).collect();

    // Compute combined statistics
    let count = rollup_psms.len();
    let intensity_sum: f64 = rollup_psms.iter().map(|p| p.ms2_intensity).sum();

    // Compute intensity-weighted median delta mass (the calibration figure)
    let weighted_apex = if intensity_sum > 0.0 {
        // Sort by delta mass for weighted median
        let mut weighted_deltas: Vec<(f64, f64)> = rollup_psm_indices
            .iter()
            .map(|&i| (calibrated_deltas[i], psms[i].ms2_intensity))
            .collect();
        weighted_deltas.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let half_intensity = intensity_sum / 2.0;
        let mut cumulative = 0.0;
        let mut apex = 0.0;
        for (delta, intensity) in &weighted_deltas {
            cumulative += intensity;
            if cumulative >= half_intensity {
                apex = *delta;
                break;
            }
        }
        apex
    } else {
        0.0
    };

    // Compute confidence metrics from all rollup PSMs
    let confidence = compute_confidence(&rollup_psms);

    // Representative m/z of the rolled-up unmodified peak (for correct ppm).
    let rollup_mz = representative_mz(&rollup_psms);

    // Sum prominence from constituent peaks
    let total_prominence: usize = near_zero_peaks.iter().map(|p| p.prominence).sum();

    // Create the single rolled-up Unmodified peak
    let unmodified_peak = Peak {
        rank: 0, // Will be set after sorting
        delta_mass: weighted_apex,
        count,
        count_pct: 100.0 * (count as f64) / (psms.len() as f64),
        intensity_sum,
        intensity_pct: if total_intensity > 0.0 {
            100.0 * intensity_sum / total_intensity
        } else {
            0.0
        },
        prominence: total_prominence,
        confidence,
        annotations: vec![PeakAnnotation {
            unimod_id: None,
            name: "Unmodified".to_string(),
            delta_mass: 0.0,
            mass_error_da: weighted_apex,
            // True ppm at the peak's actual m/z, not the old hardcoded 1000-Da form.
            mass_error_ppm: ppm_at_mz(weighted_apex, rollup_mz),
            sites: vec![],
            classification: "None".to_string(),
            source: "intrinsic".to_string(),
        }],
        ambiguous: false,
        unannotated: false,
        folded_count: near_zero_peaks.iter().map(|p| p.folded_count).sum(),
        representative_mz: rollup_mz,
        // The roll-up recounts over its own wider window, so its membership is
        // the roll-up window's, not the sum of the constituent peaks'.
        psm_indices: rollup_psm_indices.clone(),
    };

    // Rebuild peaks list with rolled-up unmodified + other peaks
    *peaks = vec![unmodified_peak];
    peaks.extend(other_peaks);

    // Re-sort by count and assign ranks
    peaks.sort_by_key(|b| std::cmp::Reverse(b.count));
    for (i, peak) in peaks.iter_mut().enumerate() {
        peak.rank = (i + 1) as u32;
    }

    log::info!(
        "Rolled up {} near-zero peaks into one Unmodified row: {} PSMs, weighted apex = {:.4} mDa",
        near_zero_peaks.len(),
        count,
        weighted_apex * 1000.0
    );
}

/// Build isotope error distribution view
fn build_isotope_error_view(
    distribution: &HashMap<i32, usize>,
    total_psms: usize,
) -> IsotopeErrorView {
    let mut bins: Vec<IsotopeErrorBin> = distribution
        .iter()
        .map(|(&isotope_error, &count)| IsotopeErrorBin {
            isotope_error,
            count,
            pct: 100.0 * (count as f64) / (total_psms as f64),
        })
        .collect();

    bins.sort_by_key(|b| b.isotope_error);

    IsotopeErrorView {
        description: "PSMs grouped by isotope_error value before correction".to_string(),
        distribution: bins,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_psm(delta: f64, intensity: f64, hyperscore: f64) -> Psm {
        Psm {
            scannr: 1,
            rank: 1,
            peptide: "PEPTIDE".to_string(),
            proteins: "PROTEIN".to_string(),
            expmass: 1000.0 + delta,
            calcmass: 1000.0,
            isotope_error: 0,
            delta_mass: delta,
            delta_mass_corrected: delta,
            hyperscore,
            matched_intensity_pct: 0.5,
            longest_b: 5,
            longest_y: 6,
            ms2_intensity: intensity,
            peptide_q: 0.001,
            spectrum_q: 0.001,
            is_decoy: false,
            charge: 2,
            rt: 30.0,
            missed_cleavages: 0,
            semi_enzymatic: false,
            precursor_ppm: 2.0,
            fragment_ppm: 5.0,
            peptide_len: 7,
        }
    }

    #[test]
    fn test_calibration() {
        // PSMs with slight positive bias
        let psms = vec![
            make_test_psm(0.003, 1000.0, 40.0),
            make_test_psm(0.004, 1000.0, 42.0),
            make_test_psm(0.002, 1000.0, 41.0),
        ];

        let (calibrated, result) = compute_calibration(&psms, CalibrationMode::DaScalar);

        assert!(result.applied);
        assert_eq!(result.mode, CalibrationMode::DaScalar);
        assert!(result.apex_offset_da > 0.002 && result.apex_offset_da < 0.004);

        // Calibrated deltas should be centered closer to zero
        let mean_calibrated: f64 = calibrated.iter().sum::<f64>() / calibrated.len() as f64;
        assert!(mean_calibrated.abs() < 0.002);
    }

    /// Build a PSM at a chosen precursor m/z. `expmass` (neutral mass) is derived from
    /// the target m/z and charge so `psm_mz()` returns `mz` back: expmass = mz*z − z*proton.
    fn make_psm_at_mz(delta: f64, mz: f64, charge: u32, intensity: f64) -> Psm {
        let expmass = mz * charge as f64 - charge as f64 * PROTON_MASS;
        Psm {
            expmass: expmass + delta,
            calcmass: expmass,
            charge,
            delta_mass: delta,
            delta_mass_corrected: delta,
            ms2_intensity: intensity,
            ..make_test_psm(delta, intensity, 40.0)
        }
    }

    #[test]
    fn test_ppm_constant_calibration() {
        // A +2 ppm bias present at TWO different m/z. In Da that is a DIFFERENT offset at
        // each m/z (2 ppm of 800 = 1.6 mDa; 2 ppm of 2000 = 4.0 mDa). A single Da scalar
        // cannot recenter both; the ppm-constant path must, because it scales with m/z.
        const PPM_BIAS: f64 = 2.0;
        let mz_lo = 800.0;
        let mz_hi = 2000.0;
        let delta_lo = PPM_BIAS * mz_lo / 1e6; // 0.0016 Da
        let delta_hi = PPM_BIAS * mz_hi / 1e6; // 0.0040 Da

        let psms = vec![
            make_psm_at_mz(delta_lo, mz_lo, 1, 1000.0),
            make_psm_at_mz(delta_hi, mz_hi, 1, 1000.0),
        ];

        let (calibrated, result) = compute_calibration(&psms, CalibrationMode::PpmConstant);

        assert!(result.applied);
        assert_eq!(result.mode, CalibrationMode::PpmConstant);

        // Fitted offset recovers the injected +2 ppm. (Not exact: m/z is reconstructed
        // from expmass, which carries the delta, so the ppm denominator is shifted by
        // <1e-5 relative — recovery to ~5e-6 ppm is the expected precision, not bit-exact.)
        let ppm = result.ppm_offset.expect("ppm_offset must be set");
        assert!(
            (ppm - PPM_BIAS).abs() < 1e-4,
            "fitted ppm {} != {}",
            ppm,
            PPM_BIAS
        );

        // The specific corrected value at EACH m/z must land near 0 — testing that the
        // correction scales with m/z, not merely that the two differ. Tolerance is a few
        // µDa: the fitted offset is the iw-median of the two per-PSM ppm values (which
        // differ by <1e-5 ppm because m/z is reconstructed from the delta-bearing
        // expmass), so applying it back leaves a µDa-scale residual, not bit-zero.
        assert!(
            calibrated[0].abs() < 5e-6,
            "low-m/z corrected delta not ~0: {}",
            calibrated[0]
        );
        assert!(
            calibrated[1].abs() < 5e-6,
            "high-m/z corrected delta not ~0: {}",
            calibrated[1]
        );

        // Invariant 1: count conserved.
        assert_eq!(calibrated.len(), psms.len());
    }

    #[test]
    fn test_ppm_constant_beats_da_scalar_across_mz() {
        // Same two-m/z +2 ppm bias. A Da scalar fit from this population subtracts one
        // constant (the iw-median Da offset) — which leaves a residual at BOTH m/z since
        // neither equals the median. Assert the ppm path leaves a strictly smaller
        // max-residual than the Da path.
        const PPM_BIAS: f64 = 2.0;
        let psms = vec![
            make_psm_at_mz(PPM_BIAS * 800.0 / 1e6, 800.0, 1, 1000.0),
            make_psm_at_mz(PPM_BIAS * 2000.0 / 1e6, 2000.0, 1, 1000.0),
        ];

        let (cal_da, _) = compute_calibration(&psms, CalibrationMode::DaScalar);
        let (cal_ppm, _) = compute_calibration(&psms, CalibrationMode::PpmConstant);

        let max_abs = |v: &[f64]| v.iter().map(|d| d.abs()).fold(0.0f64, f64::max);
        assert!(
            max_abs(&cal_ppm) < max_abs(&cal_da),
            "ppm max-residual {} not < da max-residual {}",
            max_abs(&cal_ppm),
            max_abs(&cal_da)
        );
    }

    #[test]
    fn test_calibration_psm_count_conserved() {
        let psms = vec![
            make_test_psm(0.003, 1000.0, 40.0),
            make_test_psm(0.984, 1500.0, 38.0),
            make_test_psm(15.995, 900.0, 39.0),
        ];
        for mode in [
            CalibrationMode::None,
            CalibrationMode::DaScalar,
            CalibrationMode::PpmConstant,
        ] {
            let (calibrated, result) = compute_calibration(&psms, mode);
            assert_eq!(
                calibrated.len(),
                psms.len(),
                "count not conserved in {:?}",
                mode
            );
            assert_eq!(result.mode, mode);
        }
    }

    #[test]
    fn test_calibration_none_is_identity() {
        let psms = vec![
            make_test_psm(0.003, 1000.0, 40.0),
            make_test_psm(0.984, 1500.0, 38.0),
        ];
        let (calibrated, result) = compute_calibration(&psms, CalibrationMode::None);
        assert!(!result.applied);
        assert_eq!(calibrated[0], 0.003);
        assert_eq!(calibrated[1], 0.984);
    }

    #[test]
    fn test_fold_to_zero() {
        // Create PSMs: some at 0, some at +1.003 (isotope misassignment)
        let psms = vec![
            make_test_psm(0.0, 10000.0, 40.0),
            make_test_psm(1.003, 2000.0, 35.0), // Should fold - within 12 mDa of k=1 target
            make_test_psm(0.984, 1500.0, 38.0), // Should NOT fold - 19 mDa from k=1 target
        ];

        let mut calibrated_deltas: Vec<f64> = psms.iter().map(|p| p.delta_mass_corrected).collect();

        let mut folding = FoldingStats {
            folded_to_zero_count: 0,
            folded_to_zero_intensity: 0.0,
            satellite_fold_count: 0,
            satellite_fold_intensity: 0.0,
            fold_by_k: BTreeMap::new(),
        };

        fold_isotope_psms_to_zero(&mut calibrated_deltas, &psms, &mut folding, 0.012);

        // The +1.003 PSM should be folded to 0
        assert_eq!(calibrated_deltas[0], 0.0); // Was already 0
        assert_eq!(calibrated_deltas[1], 0.0); // Folded from 1.003
        assert_eq!(calibrated_deltas[2], 0.984); // NOT folded - deamidation preserved

        assert_eq!(folding.folded_to_zero_count, 1); // Only the 1.003 PSM
        assert_eq!(*folding.fold_by_k.get(&1).unwrap_or(&0), 1);
    }

    /// Why peak centers are compared as bin INDICES, not as float distances.
    ///
    /// This was the root cause of the serum +1 Da conservation violation. Bin
    /// centers are `bin_idx as f64 * bin_width`. Two ADJACENT bins are one bin
    /// width apart in exact arithmetic, and the old `too_close` guard used `<=`,
    /// so they should have merged. In f64 they do not: 99*0.01 - 98*0.01 is
    /// larger than 0.01. The guard missed, both bins became peaks, and their PSM
    /// collection windows overlapped.
    ///
    /// The arithmetic below is still true — that is the point. It is why
    /// `detect_peaks_with_prominence` compares `bin_index()` values. If anyone
    /// reverts to a float comparison, the defect returns.
    /// This test needs no fixture — it is arithmetic, not tool behaviour.
    #[test]
    fn test_bin_grid_spacing_defeats_merge_tolerance() {
        let bin_width = DEFAULT_BIN_WIDTH_DA;
        let lower = 98.0 * bin_width; // 0.98 Da — the serum deamidation bin
        let upper = 99.0 * bin_width; // 0.99 Da — the bin immediately above it
        let gap = upper - lower;
        let tolerance = DEFAULT_PEAK_MERGE_TOLERANCE_DA.max(bin_width);

        // Exact arithmetic says the gap IS the tolerance. f64 says it is larger.
        assert!(
            gap > tolerance,
            "gap {:.20} vs tolerance {:.20}",
            gap,
            tolerance
        );
        // ⚠ `clippy::neg_cmp_op_on_partial_ord` fires on the negation below and
        // must NOT be applied. The negation is the point: this assertion and the
        // one above are DIFFERENT statements on a partially ordered type, and
        // asserting both is what pins the float behaviour. Rewriting it as
        // `gap > tolerance` would duplicate the assertion above and delete the
        // second check; rewriting it with `partial_cmp` would hide the very
        // comparison the surrounding doc comment tells the reader to watch.
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        {
            assert!(
                !(gap <= tolerance),
                "adjacent bins would merge; the defect described here is gone"
            );
        }
    }

    #[test]
    fn test_prominence_calculation() {
        // Create bins with a clear peak
        let bins: Vec<HistogramBin> = vec![
            HistogramBin {
                bin_center: 0.0,
                weighted_apex: 0.0,
                count: 10,
                intensity_sum: 1000.0,
                intensity_pct: 10.0,
                folded: false,
                fold_target: None,
            },
            HistogramBin {
                bin_center: 0.01,
                weighted_apex: 0.01,
                count: 50,
                intensity_sum: 5000.0,
                intensity_pct: 50.0,
                folded: false,
                fold_target: None,
            },
            HistogramBin {
                bin_center: 0.02,
                weighted_apex: 0.02,
                count: 100,
                intensity_sum: 10000.0,
                intensity_pct: 100.0,
                folded: false,
                fold_target: None,
            },
            HistogramBin {
                bin_center: 0.03,
                weighted_apex: 0.03,
                count: 40,
                intensity_sum: 4000.0,
                intensity_pct: 40.0,
                folded: false,
                fold_target: None,
            },
            HistogramBin {
                bin_center: 0.04,
                weighted_apex: 0.04,
                count: 5,
                intensity_sum: 500.0,
                intensity_pct: 5.0,
                folded: false,
                fold_target: None,
            },
        ];

        let bin_refs: Vec<&HistogramBin> = bins.iter().collect();

        // The peak at 0.02 should have high prominence
        let prominence = compute_prominence(&bins[2], &bin_refs, 0.5);
        assert!(prominence > 50); // Should be at least 100 - 50 = 50
    }

    #[test]
    fn test_excluded_classifications() {
        // Test that AA substitution annotations are filtered
        let excluded = ["AA substitution".to_string()];

        // This would need a mock UnimodDb to fully test
        // For now, just verify the filter logic compiles
        assert!(excluded.iter().any(|e| "AA substitution".contains(e)));
        assert!(!excluded.iter().any(|e| "Post-translational".contains(e)));
    }

    #[test]
    fn test_near_zero_classification() {
        // PSMs at delta ~0 should be classified as unmodified
        let psms = [
            make_test_psm(0.0, 1000.0, 40.0),
            make_test_psm(0.001, 1000.0, 42.0),
            make_test_psm(-0.002, 1000.0, 41.0),
        ];

        let near_zero_count = psms
            .iter()
            .filter(|p| p.delta_mass_corrected.abs() < NEAR_ZERO_THRESHOLD_DA)
            .count();

        assert_eq!(near_zero_count, 3);
    }

    #[test]
    fn test_ppm_at_mz_formula() {
        // Per-site gate: mass_error_ppm must equal error_da / mz * 1e6.
        // A +5 mDa error at m/z 1500 is 5e-3 / 1500 * 1e6 = 3.333 ppm — NOT the
        // old 1000-Da shortcut's 5.0 ppm. This is the bug the fix closes.
        let ppm = ppm_at_mz(0.005, 1500.0);
        assert!((ppm - 3.333333).abs() < 1e-4, "got {}", ppm);

        // At exactly 1000 Da the true form and the old shortcut agree — sanity.
        assert!((ppm_at_mz(0.005, 1000.0) - 5.0).abs() < 1e-9);

        // Non-positive m/z falls back to the 1000-Da form (finite, not NaN/Inf).
        assert!((ppm_at_mz(0.005, 0.0) - 5.0).abs() < 1e-9);
        assert!(ppm_at_mz(0.005, -1.0).is_finite());
    }

    #[test]
    fn test_representative_mz_within_input_range() {
        // Conservation invariant: the intensity-weighted mean m/z cannot fall
        // outside the [min, max] of its constituent PSMs' m/z. This catches
        // threading/weighting bugs the ppm-formula test cannot.
        let p_lo = make_psm_at_mz(0.0, 600.0, 2, 1000.0);
        let p_mid = make_psm_at_mz(0.0, 900.0, 2, 5000.0); // heaviest weight
        let p_hi = make_psm_at_mz(0.0, 1400.0, 2, 1000.0);
        let refs: Vec<&Psm> = vec![&p_lo, &p_mid, &p_hi];

        let rep = representative_mz(&refs);
        assert!(
            (600.0..=1400.0).contains(&rep),
            "rep {} outside [600, 1400]",
            rep
        );
        // Intensity weighting pulls it toward the 900 m/z peak, not the plain mean (966.7).
        assert!(
            rep < 966.7,
            "weighting not applied: rep {} >= unweighted mean",
            rep
        );

        // Empty input → 0.0, no panic.
        assert_eq!(representative_mz(&[]), 0.0);

        // Zero total intensity → unweighted mean, still within range.
        let z1 = make_psm_at_mz(0.0, 700.0, 2, 0.0);
        let z2 = make_psm_at_mz(0.0, 1100.0, 2, 0.0);
        let zrefs: Vec<&Psm> = vec![&z1, &z2];
        let zrep = representative_mz(&zrefs);
        assert!(
            (zrep - 900.0).abs() < 1e-6,
            "unweighted mean expected 900, got {}",
            zrep
        );
    }
}
