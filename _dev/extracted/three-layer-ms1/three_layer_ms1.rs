//! Three-layer MS1 signal fate — EXTRACTED FROM recon, NOT COMPILED.
//!
//! This file is not part of any crate. It is kept as source for the other tool
//! (sageGUI) that still wants the algorithm. See README.md in this directory.
//!
//! Removed from `recon-tool/src/mzml.rs` on 2026-09-02 on Ben's instruction.
//! Extracted verbatim; only this header and the `use` block below were added.
//!
//! ## What recon supplied that is NOT in this file
//! These items stayed in `recon-tool/src/mzml.rs` because live, non-three-layer
//! code still calls them. A host crate must provide equivalents:
//!   * `Ms1Spectrum`                     (mzml.rs) — `{ scan, rt, mz: Vec<f64>, intensity: Vec<f64>, tic }`
//!   * `PrecursorQuery`                  (mzml.rs) — `{ mz, rt, charge, .. }`
//!   * `build_precursor_queries_from_psms` (mzml.rs)
//!   * `extract_precursor_intensities`   (mzml.rs)
//!   * `extract_scan_from_id`            (mzml.rs) — PRIVATE to that module
//!   * `crate::sage_results::Psm`        — field `delta_mass_corrected: f64`
//!
//! External crates used: anyhow, flate2, mzdata, serde.

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use mzdata::io::mzml::MzMLReader;
use mzdata::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

// The five items below stayed in recon. Point these at the host crate.
use crate::mzml::{
    build_precursor_queries_from_psms, extract_precursor_intensities, extract_scan_from_id,
    Ms1Spectrum, PrecursorQuery,
};

// ============================================================================
// MS2 Precursor Info Extraction (for three-layer MS1 signal fate)
// ============================================================================

/// MS2 precursor information extracted from mzML (independent of identification)
#[derive(Debug, Clone)]
pub struct Ms2PrecursorInfo {
    /// Scan number
    pub scan: u32,
    /// Precursor m/z
    pub precursor_mz: f64,
    /// Precursor charge (0 if unknown)
    pub charge: u32,
    /// Retention time in minutes
    pub rt: f64,
}

/// Extract MS2 precursor information from mzML file.
///
/// This extracts precursor m/z, charge, and RT for ALL MS2 spectra,
/// independent of whether they were identified. Used for three-layer
/// MS1 signal fate analysis.
///
/// # Arguments
/// * `path` - Path to the mzML file (supports .mzML and .mzML.gz)
///
/// # Returns
/// Vector of MS2 precursor info
pub fn extract_ms2_precursor_info(path: &Path) -> Result<Vec<Ms2PrecursorInfo>> {
    let is_gzipped = path.extension().map(|e| e == "gz").unwrap_or(false);

    let file = File::open(path)
        .with_context(|| format!("Failed to open mzML file: {}", path.display()))?;

    if is_gzipped {
        let mut decoder = GzDecoder::new(file);
        let mut data = Vec::new();
        decoder
            .read_to_end(&mut data)
            .with_context(|| format!("Failed to decompress gzipped mzML: {}", path.display()))?;
        let cursor = Cursor::new(data);
        extract_ms2_precursor_info_from_reader(cursor)
    } else {
        let reader = BufReader::new(file);
        extract_ms2_precursor_info_from_reader(reader)
    }
}

/// Internal function to extract MS2 precursor info from any reader
fn extract_ms2_precursor_info_from_reader<R: std::io::Read + std::io::Seek>(
    reader: R,
) -> Result<Vec<Ms2PrecursorInfo>> {
    let mut mzml_reader = MzMLReader::new_indexed(reader);
    let mut precursor_info = Vec::new();

    for spectrum in mzml_reader.iter() {
        if spectrum.ms_level() != 2 {
            continue;
        }

        let desc = spectrum.description();

        // Extract scan number from native ID
        let scan = extract_scan_from_id(&desc.id);

        // Get retention time (in minutes)
        let rt = desc
            .acquisition
            .first_scan()
            .map(|s| s.start_time)
            .unwrap_or(0.0);

        // Get precursor info.
        //
        // Sage uses the isolation window target m/z when the selected ion m/z is
        // absent or zero. Some conversion software writes no selected ion tag,
        // and DIA files frequently do not have one. recon must agree with Sage
        // on which MS2 spectra exist, because signal-fate divides by that count.
        // Source: sage-cloudpath/src/mzml.rs at the pinned rev df92199,
        // lines 225-231 (isolation window target) and 244-249 (selected ion m/z,
        // which is written only when it is not zero).
        let precursor = desc.precursor.first();
        let (mut precursor_mz, charge) = precursor
            .and_then(|p| p.ions.first())
            .map(|ion| (ion.mz(), ion.charge().unwrap_or(0) as u32))
            .unwrap_or((0.0, 0));
        if precursor_mz <= 0.0 {
            // `IsolationWindow::target` is 0.0 when the file has no window, so
            // the guard below still rejects a spectrum with no usable m/z.
            if let Some(p) = precursor {
                precursor_mz = p.isolation_window.target as f64;
            }
        }

        // Skip if no valid precursor m/z
        if precursor_mz > 0.0 {
            precursor_info.push(Ms2PrecursorInfo {
                scan,
                precursor_mz,
                charge,
                rt,
            });
        }
    }

    Ok(precursor_info)
}

// ============================================================================
// Three-Layer MS1 Signal Fate
// ============================================================================

/// Three-layer MS1 signal fate result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeLayerMs1Fate {
    /// Total MS1 TIC (all signal)
    pub total_ms1_tic: f64,

    // Layer 1: Non-peptidic (outside 400-1200 m/z)
    /// Non-peptidic TIC (outside peptide-like m/z range)
    pub non_peptidic_tic: f64,
    /// Non-peptidic percentage of total
    pub non_peptidic_pct: f64,

    // Layer 2: Peptide-like breakdown
    /// Peptide-like TIC (400-1200 m/z)
    pub peptide_like_tic: f64,
    /// Peptide-like percentage of total
    pub peptide_like_pct: f64,

    // Layer 2a: Never sampled (no MS2 trigger)
    /// Never sampled TIC
    pub never_sampled_tic: f64,
    /// Never sampled percentage of peptide-like
    pub never_sampled_pct_of_peptide_like: f64,

    // Layer 2b: Sampled but not identified
    /// Sampled but not identified TIC
    pub sampled_not_id_tic: f64,
    /// Sampled but not identified percentage of peptide-like
    pub sampled_not_id_pct_of_peptide_like: f64,

    // Layer 2c: Identified
    /// Identified TIC
    pub identified_tic: f64,
    /// Identified percentage of peptide-like
    pub identified_pct_of_peptide_like: f64,

    // Derived metrics
    /// Sampling efficiency: (sampled + identified) / peptide-like
    pub sampling_efficiency_pct: f64,
    /// ID efficiency: identified / (sampled + identified)
    pub id_efficiency_pct: f64,

    // Counts
    /// Number of MS2 precursors (sampling events)
    pub ms2_precursor_count: usize,
    /// Number of identified PSMs
    pub identified_psm_count: usize,
    /// Number of unique identified precursor features
    pub identified_feature_count: usize,

    // Modification breakdown (optional, populated when PSMs have delta_mass)
    /// Identified TIC breakdown by modification status
    pub mod_breakdown: Option<ModificationBreakdown>,
}

/// Breakdown of identified TIC by modification status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationBreakdown {
    /// Unmodified TIC (|delta_mass| < 0.1 Da)
    pub unmodified_tic: f64,
    /// Unmodified percentage of identified
    pub unmodified_pct: f64,
    /// Modified TIC
    pub modified_tic: f64,
    /// Modified percentage of identified
    pub modified_pct: f64,
    /// Top modifications by TIC
    pub top_modifications: Vec<ModificationTic>,
}

/// A modification with its associated TIC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationTic {
    /// Delta mass bin (0.01 Da resolution)
    pub delta_mass: f64,
    /// TIC attributed to this modification
    pub tic: f64,
    /// Percentage of identified TIC
    pub pct: f64,
    /// PSM count
    pub count: usize,
}

/// Region in m/z-RT space that was sampled or identified
#[derive(Debug, Clone)]
struct MzRtRegion {
    mz: f64,
    rt: f64,
    charge: u32,
}

/// Sorted region index for fast RT-based lookup
struct SortedRegionIndex {
    /// Regions sorted by RT
    regions: Vec<MzRtRegion>,
    /// RT values for binary search
    rt_values: Vec<f64>,
}

impl SortedRegionIndex {
    /// Build a sorted index from regions
    fn new(mut regions: Vec<MzRtRegion>) -> Self {
        regions.sort_by(|a, b| a.rt.partial_cmp(&b.rt).unwrap_or(std::cmp::Ordering::Equal));
        let rt_values: Vec<f64> = regions.iter().map(|r| r.rt).collect();
        Self { regions, rt_values }
    }

    /// Check if a peak is within any region (optimized with RT pre-filtering)
    fn contains(&self, peak_mz: f64, peak_rt: f64, mz_tol_ppm: f64, rt_tol_minutes: f64) -> bool {
        if self.regions.is_empty() {
            return false;
        }

        // Binary search to find RT range
        let rt_min = peak_rt - rt_tol_minutes;
        let rt_max = peak_rt + rt_tol_minutes;

        // Find first region with RT >= rt_min
        let start_idx = match self
            .rt_values
            .binary_search_by(|r| r.partial_cmp(&rt_min).unwrap_or(std::cmp::Ordering::Equal))
        {
            Ok(i) => i,
            Err(i) => i,
        };

        // Scan only regions within RT window
        for i in start_idx..self.regions.len() {
            let region = &self.regions[i];
            if region.rt > rt_max {
                break; // Past RT window, done
            }

            // Check m/z match
            let mz_tol = mz_tol_ppm * region.mz / 1_000_000.0;
            if (peak_mz - region.mz).abs() <= mz_tol {
                return true;
            }
        }

        false
    }

    fn len(&self) -> usize {
        self.regions.len()
    }
}

/// Compute three-layer MS1 signal fate.
///
/// Separates MS1 TIC into:
/// 1. Non-peptidic (outside 400-1200 m/z)
/// 2. Peptide-like, never sampled (no MS2 trigger)
/// 3. Peptide-like, sampled but not identified (MS2 but no PSM)
/// 4. Peptide-like, identified (MS2 + PSM)
///
/// # Arguments
/// * `ms1_spectra` - Pre-loaded MS1 spectra
/// * `ms2_precursors` - All MS2 precursor info from mzML
/// * `identified_queries` - Precursor queries from identified PSMs
/// * `mz_tol_ppm` - m/z tolerance in ppm (default: 20.0)
/// * `rt_tol_minutes` - RT tolerance in minutes (default: 1.0)
///
/// # Returns
/// Three-layer MS1 signal fate breakdown
pub fn compute_three_layer_ms1_fate(
    ms1_spectra: &[Ms1Spectrum],
    ms2_precursors: &[Ms2PrecursorInfo],
    identified_queries: &[PrecursorQuery],
    mz_tol_ppm: f64,
    rt_tol_minutes: f64,
) -> ThreeLayerMs1Fate {
    use std::time::Instant;

    let start_time = Instant::now();

    // Build regions for sampled (all MS2 precursors) and identified (PSMs)
    eprintln!("  Building region indexes...");
    let sampled_regions: Vec<MzRtRegion> = ms2_precursors
        .iter()
        .map(|p| MzRtRegion {
            mz: p.precursor_mz,
            rt: p.rt,
            charge: p.charge,
        })
        .collect();

    let identified_regions: Vec<MzRtRegion> = identified_queries
        .iter()
        .map(|q| MzRtRegion {
            mz: q.mz,
            rt: q.rt,
            charge: q.charge,
        })
        .collect();

    // Build sorted indexes for fast lookup
    let sampled_index = SortedRegionIndex::new(sampled_regions.clone());
    let identified_index = SortedRegionIndex::new(identified_regions.clone());
    eprintln!("  Sampled index: {} regions", sampled_index.len());
    eprintln!("  Identified index: {} regions", identified_index.len());

    // Deduplicate identified regions for feature count
    let identified_feature_count = deduplicate_regions(&identified_regions, mz_tol_ppm, 0.5).len();
    eprintln!("  Unique identified features: {}", identified_feature_count);

    // Peptide-like m/z range
    const MZ_MIN: f64 = 400.0;
    const MZ_MAX: f64 = 1200.0;

    // Accumulate TIC into buckets
    let mut total_tic = 0.0;
    let mut non_peptidic_tic = 0.0;
    let mut never_sampled_tic = 0.0;
    let mut sampled_not_id_tic = 0.0;
    let mut identified_tic = 0.0;

    let total_spectra = ms1_spectra.len();
    let report_interval = (total_spectra / 20).max(100); // Report ~20 times or every 100 spectra

    eprintln!("  Processing {} MS1 spectra...", total_spectra);

    for (idx, ms1) in ms1_spectra.iter().enumerate() {
        // Progress reporting
        if idx > 0 && idx % report_interval == 0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let rate = idx as f64 / elapsed;
            let remaining = (total_spectra - idx) as f64 / rate;
            eprintln!(
                "    Progress: {}/{} spectra ({:.1}%), ETA: {:.0}s",
                idx,
                total_spectra,
                100.0 * idx as f64 / total_spectra as f64,
                remaining
            );
        }

        let spectrum_rt = ms1.rt;

        for (&mz, &intensity) in ms1.mz.iter().zip(ms1.intensity.iter()) {
            total_tic += intensity;

            // Check if peptide-like (400-1200 m/z)
            if mz < MZ_MIN || mz > MZ_MAX {
                non_peptidic_tic += intensity;
                continue;
            }

            // Check if identified (using optimized index)
            if identified_index.contains(mz, spectrum_rt, mz_tol_ppm, rt_tol_minutes) {
                identified_tic += intensity;
            }
            // Check if sampled (but not identified) (using optimized index)
            else if sampled_index.contains(mz, spectrum_rt, mz_tol_ppm, rt_tol_minutes) {
                sampled_not_id_tic += intensity;
            }
            // Never sampled
            else {
                never_sampled_tic += intensity;
            }
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    eprintln!("  Completed in {:.1}s", elapsed);

    // Compute peptide-like TIC
    let peptide_like_tic = never_sampled_tic + sampled_not_id_tic + identified_tic;

    // Compute percentages
    let non_peptidic_pct = if total_tic > 0.0 {
        100.0 * non_peptidic_tic / total_tic
    } else {
        0.0
    };
    let peptide_like_pct = if total_tic > 0.0 {
        100.0 * peptide_like_tic / total_tic
    } else {
        0.0
    };

    let never_sampled_pct = if peptide_like_tic > 0.0 {
        100.0 * never_sampled_tic / peptide_like_tic
    } else {
        0.0
    };
    let sampled_not_id_pct = if peptide_like_tic > 0.0 {
        100.0 * sampled_not_id_tic / peptide_like_tic
    } else {
        0.0
    };
    let identified_pct = if peptide_like_tic > 0.0 {
        100.0 * identified_tic / peptide_like_tic
    } else {
        0.0
    };

    // Derived metrics
    let sampled_total = sampled_not_id_tic + identified_tic;
    let sampling_efficiency = if peptide_like_tic > 0.0 {
        100.0 * sampled_total / peptide_like_tic
    } else {
        0.0
    };
    let id_efficiency = if sampled_total > 0.0 {
        100.0 * identified_tic / sampled_total
    } else {
        0.0
    };

    ThreeLayerMs1Fate {
        total_ms1_tic: total_tic,
        non_peptidic_tic,
        non_peptidic_pct,
        peptide_like_tic,
        peptide_like_pct,
        never_sampled_tic,
        never_sampled_pct_of_peptide_like: never_sampled_pct,
        sampled_not_id_tic,
        sampled_not_id_pct_of_peptide_like: sampled_not_id_pct,
        identified_tic,
        identified_pct_of_peptide_like: identified_pct,
        sampling_efficiency_pct: sampling_efficiency,
        id_efficiency_pct: id_efficiency,
        ms2_precursor_count: ms2_precursors.len(),
        identified_psm_count: identified_queries.len(),
        identified_feature_count,
        mod_breakdown: None, // Populated separately via compute_mod_breakdown()
    }
}

/// Compute modification breakdown for identified TIC.
///
/// Groups PSMs by delta mass and computes TIC for each modification.
/// Returns breakdown of unmodified vs modified, plus top N modifications.
///
/// # Arguments
/// * `psms` - PSMs with delta_mass_corrected field
/// * `ms1_spectra` - Pre-loaded MS1 spectra
/// * `mz_tol_ppm` - m/z tolerance for intensity extraction
/// * `rt_window_minutes` - RT window for intensity extraction
/// * `top_n` - Number of top modifications to return
///
/// # Returns
/// ModificationBreakdown with unmodified/modified TIC and top modifications
pub fn compute_mod_breakdown(
    psms: &[crate::sage_results::Psm],
    ms1_spectra: &[Ms1Spectrum],
    mz_tol_ppm: f64,
    rt_window_minutes: f64,
    top_n: usize,
) -> ModificationBreakdown {
    use std::collections::HashMap;

    const NEAR_ZERO_THRESHOLD: f64 = 0.1; // Da

    // Build precursor queries and extract intensities
    let queries = build_precursor_queries_from_psms(psms);
    let intensities =
        extract_precursor_intensities(ms1_spectra, &queries, rt_window_minutes, mz_tol_ppm);

    // Group by delta mass bin (0.01 Da resolution)
    let mut mod_tic: HashMap<i64, (f64, usize)> = HashMap::new(); // bin -> (tic, count)
    let mut unmodified_tic = 0.0;
    let mut modified_tic = 0.0;

    for (psm, result) in psms.iter().zip(intensities.iter()) {
        let delta = psm.delta_mass_corrected;
        let intensity = result.intensity;

        if delta.abs() < NEAR_ZERO_THRESHOLD {
            unmodified_tic += intensity;
        } else {
            modified_tic += intensity;
            // Bin to 0.01 Da
            let bin = (delta * 100.0).round() as i64;
            let entry = mod_tic.entry(bin).or_insert((0.0, 0));
            entry.0 += intensity;
            entry.1 += 1;
        }
    }

    let total_tic = unmodified_tic + modified_tic;

    // Sort modifications by TIC and take top N
    let mut mods: Vec<_> = mod_tic
        .into_iter()
        .map(|(bin, (tic, count))| {
            let delta_mass = bin as f64 / 100.0;
            let pct = if total_tic > 0.0 {
                100.0 * tic / total_tic
            } else {
                0.0
            };
            ModificationTic {
                delta_mass,
                tic,
                pct,
                count,
            }
        })
        .collect();
    mods.sort_by(|a, b| {
        b.tic
            .partial_cmp(&a.tic)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_modifications: Vec<_> = mods.into_iter().take(top_n).collect();

    ModificationBreakdown {
        unmodified_tic,
        unmodified_pct: if total_tic > 0.0 {
            100.0 * unmodified_tic / total_tic
        } else {
            0.0
        },
        modified_tic,
        modified_pct: if total_tic > 0.0 {
            100.0 * modified_tic / total_tic
        } else {
            0.0
        },
        top_modifications,
    }
}

/// Deduplicate regions by (m/z, RT) within tolerances
fn deduplicate_regions(
    regions: &[MzRtRegion],
    mz_tol_ppm: f64,
    rt_tol_minutes: f64,
) -> Vec<MzRtRegion> {
    let mut unique: Vec<MzRtRegion> = Vec::new();

    for region in regions {
        let mut found = false;
        for existing in &unique {
            let mz_tol = mz_tol_ppm * existing.mz / 1_000_000.0;
            if (existing.mz - region.mz).abs() <= mz_tol
                && (existing.rt - region.rt).abs() <= rt_tol_minutes
                && existing.charge == region.charge
            {
                found = true;
                break;
            }
        }
        if !found {
            unique.push(region.clone());
        }
    }

    unique
}

/// Print three-layer MS1 signal fate summary
pub fn print_three_layer_ms1_fate(fate: &ThreeLayerMs1Fate) {
    println!();
    println!("=== MS1 Signal Fate (Three-Layer) ===");
    println!();
    println!("Total MS1 TIC:           {:.2e} (100%)", fate.total_ms1_tic);
    println!(
        "├─ Non-peptidic:         {:.2e} ({:.1}%)  [outside 400-1200 m/z]",
        fate.non_peptidic_tic, fate.non_peptidic_pct
    );
    println!(
        "├─ Peptide-like:         {:.2e} ({:.1}%)",
        fate.peptide_like_tic, fate.peptide_like_pct
    );
    println!(
        "   ├─ Never sampled:     {:.2e} ({:.1}%)  [no MS2 trigger]",
        fate.never_sampled_tic, fate.never_sampled_pct_of_peptide_like
    );
    println!(
        "   ├─ Sampled, not ID'd: {:.2e} ({:.1}%)  [MS2 but no PSM]",
        fate.sampled_not_id_tic, fate.sampled_not_id_pct_of_peptide_like
    );
    println!(
        "   └─ Identified:        {:.2e} ({:.1}%)  [PSM at q<0.01]",
        fate.identified_tic, fate.identified_pct_of_peptide_like
    );
    println!();
    println!("Derived Metrics:");
    println!(
        "  Sampling efficiency: {:.1}% of peptide-like TIC was sampled by MS2",
        fate.sampling_efficiency_pct
    );
    println!(
        "  ID efficiency:       {:.1}% of sampled TIC was identified",
        fate.id_efficiency_pct
    );
    println!();
    println!("Counts:");
    println!(
        "  MS2 precursors (sampling events): {}",
        fate.ms2_precursor_count
    );
    println!("  Identified PSMs: {}", fate.identified_psm_count);
    println!(
        "  Unique identified features: {}",
        fate.identified_feature_count
    );

    // Comparison table
    let sampled_tic = fate.sampled_not_id_tic + fate.identified_tic;
    let id_pct_of_sampled = if sampled_tic > 0.0 {
        100.0 * fate.identified_tic / sampled_tic
    } else {
        0.0
    };
    let unid_pct_of_sampled = if sampled_tic > 0.0 {
        100.0 * fate.sampled_not_id_tic / sampled_tic
    } else {
        0.0
    };

    // For MS2 spectra, we need unique scans (not PSM count which can be > spectra due to chimeras)
    // Use feature count as proxy for identified spectra (deduplicated)
    let id_spectra_pct =
        100.0 * fate.identified_feature_count as f64 / fate.ms2_precursor_count.max(1) as f64;
    let unid_spectra_pct = 100.0 - id_spectra_pct.min(100.0);

    println!();
    println!("=== Signal Fate Comparison Table ===");
    println!();
    println!(
        "{:<20} {:>15} {:>20} {:>20}",
        "", "MS2 Spectra", "MS1 TIC (sampled)", "MS1 TIC (peptide-like)"
    );
    println!("{}", "-".repeat(78));
    println!(
        "{:<20} {:>15} {:>20} {:>20}",
        "Total",
        format!("{}", fate.ms2_precursor_count),
        format!("{:.2e}", sampled_tic),
        format!("{:.2e}", fate.peptide_like_tic)
    );
    println!(
        "{:<20} {:>14}% {:>19}% {:>19}%",
        "Identified",
        format!("{:.1}", id_spectra_pct.min(100.0)),
        format!("{:.1}", id_pct_of_sampled),
        format!("{:.1}", fate.identified_pct_of_peptide_like)
    );
    println!(
        "{:<20} {:>14}% {:>19}% {:>19}%",
        "Unidentified",
        format!("{:.1}", unid_spectra_pct.max(0.0)),
        format!("{:.1}", unid_pct_of_sampled),
        format!("{:.1}", fate.sampled_not_id_pct_of_peptide_like)
    );
    println!(
        "{:<20} {:>15} {:>20} {:>19}%",
        "Never sampled",
        "N/A",
        "N/A",
        format!("{:.1}", fate.never_sampled_pct_of_peptide_like)
    );
    println!();
    println!(
        "Note: 'MS2 Spectra' uses unique identified features (deduplicated by m/z, RT, charge)"
    );
    println!("      'MS1 TIC (sampled)' = signal that was selected for MS2 fragmentation");
    println!("      'MS1 TIC (peptide-like)' = all signal in 400-1200 m/z range");

    // Print modification breakdown if available
    if let Some(ref breakdown) = fate.mod_breakdown {
        print_mod_breakdown(breakdown);
    }
}

/// Print modification breakdown summary
pub fn print_mod_breakdown(breakdown: &ModificationBreakdown) {
    println!();
    println!("=== Identified TIC by Modification Status ===");
    println!();
    println!(
        "  Unmodified:  {:.2e} ({:.1}%)",
        breakdown.unmodified_tic, breakdown.unmodified_pct
    );
    println!(
        "  Modified:    {:.2e} ({:.1}%)",
        breakdown.modified_tic, breakdown.modified_pct
    );

    if !breakdown.top_modifications.is_empty() {
        println!();
        println!("Top Modifications by MS1 TIC:");
        println!(
            "{:<12} {:>15} {:>10} {:>10}",
            "Delta (Da)", "TIC", "% of ID'd", "PSM Count"
        );
        println!("{}", "-".repeat(50));
        for m in &breakdown.top_modifications {
            println!(
                "{:<12.2} {:>15.2e} {:>9.1}% {:>10}",
                m.delta_mass, m.tic, m.pct, m.count
            );
        }
    }
}
