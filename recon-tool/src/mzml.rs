//! mzML file parsing for spectrum counting and intensity extraction.
//!
//! This module provides functions to parse mzML files and extract:
//! - Total MS1/MS2 spectrum counts
//! - Total MS2 intensity (TIC sum)
//! - MS1 spectrum data for polymer detection
//! - MS2 spectrum data for oxonium ion screening
//! - MS1 precursor intensity extraction
//!
//! Uses the `mzdata` crate for parsing.

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use mzdata::io::mzml::MzMLReader;
use mzdata::io::DetailLevel;
use mzdata::meta::ComponentType;
use mzdata::prelude::*;
use mzdata::spectrum::MultiLayerSpectrum;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

/// Statistics extracted from an mzML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MzmlStats {
    /// Total number of spectra in the file
    pub total_spectra: usize,
    /// Number of MS1 spectra
    pub ms1_spectra: usize,
    /// Number of MS2 spectra
    pub ms2_spectra: usize,
    /// Sum of TIC (total ion current) for all MS2 spectra
    pub total_ms2_tic: f64,
    /// File path (for reference)
    pub file_path: String,
}

/// Parse an mzML file and extract basic statistics.
///
/// This function performs a single pass through the mzML file, counting
/// spectra by MS level and summing MS2 TIC values.
///
/// # Arguments
/// * `path` - Path to the mzML file (supports .mzML and .mzML.gz)
///
/// # Returns
/// Statistics about the mzML file contents
///
/// # Example
/// ```no_run
/// use recon_tool::mzml::get_mzml_stats;
/// use std::path::Path;
///
/// let stats = get_mzml_stats(Path::new("data.mzML")).unwrap();
/// println!("MS2 spectra: {}", stats.ms2_spectra);
/// ```
pub fn get_mzml_stats(path: &Path) -> Result<MzmlStats> {
    let file_path = path.display().to_string();
    let is_gzipped = path.extension().map(|e| e == "gz").unwrap_or(false);

    // Open the file
    let file = File::open(path)
        .with_context(|| format!("Failed to open mzML file: {}", path.display()))?;

    // Handle gzip vs plain mzML
    // mzdata requires Seek for iteration, so we need to read gzipped files into memory
    if is_gzipped {
        let mut decoder = GzDecoder::new(file);
        let mut data = Vec::new();
        decoder
            .read_to_end(&mut data)
            .with_context(|| format!("Failed to decompress gzipped mzML: {}", path.display()))?;
        let cursor = Cursor::new(data);
        parse_mzml_reader(cursor, file_path)
    } else {
        let reader = BufReader::new(file);
        parse_mzml_reader(reader, file_path)
    }
}

/// Internal function to parse mzML from any reader that supports Seek
fn parse_mzml_reader<R: std::io::Read + std::io::Seek>(
    reader: R,
    file_path: String,
) -> Result<MzmlStats> {
    // Use MzMLReader::new_indexed to build the index for iteration
    let mut mzml_reader = MzMLReader::new_indexed(reader);

    let mut total_spectra = 0usize;
    let mut ms1_spectra = 0usize;
    let mut ms2_spectra = 0usize;
    let mut total_ms2_tic = 0.0f64;

    // Iterate through all spectra using the indexed reader
    for spectrum in mzml_reader.iter() {
        total_spectra += 1;

        // Get MS level from spectrum description
        let ms_level = spectrum.ms_level();

        match ms_level {
            1 => {
                ms1_spectra += 1;
            }
            2 => {
                ms2_spectra += 1;
                // Get TIC from spectrum - use the signal's total intensity
                // The TIC is typically stored in the spectrum description
                if let Some(tic) = get_spectrum_tic(&spectrum) {
                    total_ms2_tic += tic;
                }
            }
            _ => {
                // MS3+ spectra - count but don't process
                // Could add ms3_spectra field if needed
            }
        }
    }

    Ok(MzmlStats {
        total_spectra,
        ms1_spectra,
        ms2_spectra,
        total_ms2_tic,
        file_path,
    })
}

/// Extract TIC (total ion current) from a spectrum.
///
/// Tries multiple approaches:
/// 1. Look for TIC in spectrum params
/// 2. Sum all peak intensities if TIC not available
fn get_spectrum_tic<C: CentroidLike + Default, D: DeconvolutedCentroidLike + Default>(
    spectrum: &MultiLayerSpectrum<C, D>,
) -> Option<f64> {
    // First try to get TIC from spectrum description params
    // The TIC is typically stored as a CV param
    let desc = spectrum.description();

    // Check for TIC in params - mzdata stores this in the spectrum description
    // Look for MS:1000285 (total ion current)
    for param in desc.params().iter() {
        if param.name() == "total ion current"
            || param.accession().map(|a| a == 1000285).unwrap_or(false)
        {
            // ⚠ `clippy::unnecessary_fallible_conversions` says to use `.into()`
            // here. DO NOT. It would turn a fallback into a PANIC.
            //
            // Checked against the pinned mzdata source, not assumed
            // (mzdata-0.65.5, `src/params.rs:1196`):
            //
            //     impl From<ValueRef<'_>> for f64 {
            //         fn from(value: ValueRef<'_>) -> Self { value.to_f64().unwrap() }
            //     }
            //
            // `to_f64` returns `Result<f64, ParamValueParseError>`, so the `From`
            // impl is infallible to the TYPE SYSTEM and panicking in FACT. A
            // `total ion current` param that does not parse as a number would
            // abort the run. The fallible form keeps the intended behaviour: a
            // non-numeric TIC reads as 0.0 and the code below falls through to
            // summing peak intensities instead.
            #[allow(clippy::unnecessary_fallible_conversions)]
            let tic: f64 = param.value().try_into().unwrap_or(0.0);
            if tic > 0.0 {
                return Some(tic);
            }
        }
    }

    // Fallback: sum peak intensities if we have centroid data
    // RefPeakDataLevel provides access to peaks
    let peaks = spectrum.peaks();
    if !peaks.is_empty() {
        let sum: f64 = peaks.iter().map(|p| p.intensity() as f64).sum();
        if sum > 0.0 {
            return Some(sum);
        }
    }

    None
}

/// Print a summary of mzML statistics
pub fn print_mzml_stats(stats: &MzmlStats) {
    println!("=== mzML File Statistics ===");
    println!("File: {}", stats.file_path);
    println!();
    println!("Spectrum Counts:");
    println!("  Total spectra: {}", stats.total_spectra);
    println!("  MS1 spectra: {}", stats.ms1_spectra);
    println!("  MS2 spectra: {}", stats.ms2_spectra);
    if stats.total_spectra > 0 {
        let other = stats.total_spectra - stats.ms1_spectra - stats.ms2_spectra;
        if other > 0 {
            println!("  Other (MS3+): {}", other);
        }
    }
    println!();
    println!("MS2 Intensity:");
    println!("  Total MS2 TIC: {:.2e}", stats.total_ms2_tic);
    if stats.ms2_spectra > 0 {
        println!(
            "  Average MS2 TIC: {:.2e}",
            stats.total_ms2_tic / stats.ms2_spectra as f64
        );
    }
}

// ============================================================================
// MS1 Spectrum Data Extraction (for polymer detection)
// ============================================================================

/// MS1 spectrum data for polymer detection
#[derive(Debug, Clone)]
pub struct Ms1Spectrum {
    /// Retention time in minutes
    pub rt: f64,
    /// Total ion current
    pub tic: f64,
    /// m/z values
    pub mz: Vec<f64>,
    /// Intensity values
    pub intensity: Vec<f64>,
}

/// Extract all MS1 spectra from an mzML file.
///
/// Returns a vector of MS1 spectra with RT, TIC, and peak arrays.
/// This is used for polymer detection.
///
/// # Arguments
/// * `path` - Path to the mzML file (supports .mzML and .mzML.gz)
///
/// # Returns
/// Vector of MS1 spectra with their data
pub fn extract_ms1_spectra(path: &Path) -> Result<Vec<Ms1Spectrum>> {
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
        extract_ms1_from_reader(cursor)
    } else {
        let reader = BufReader::new(file);
        extract_ms1_from_reader(reader)
    }
}

/// Internal function to extract MS1 spectra from any reader
fn extract_ms1_from_reader<R: std::io::Read + std::io::Seek>(
    reader: R,
) -> Result<Vec<Ms1Spectrum>> {
    let mut mzml_reader = MzMLReader::new_indexed(reader);
    let mut ms1_spectra = Vec::new();

    for spectrum in mzml_reader.iter() {
        if spectrum.ms_level() != 1 {
            continue;
        }

        // Get retention time (in minutes)
        let rt = spectrum
            .description()
            .acquisition
            .first_scan()
            .map(|s| s.start_time)
            .unwrap_or(0.0);

        // Get TIC
        let tic = get_spectrum_tic(&spectrum).unwrap_or(0.0);

        // Get peak data
        let peaks = spectrum.peaks();
        let mut mz = Vec::with_capacity(peaks.len());
        let mut intensity = Vec::with_capacity(peaks.len());

        for peak in peaks.iter() {
            mz.push(peak.mz());
            intensity.push(peak.intensity() as f64);
        }

        ms1_spectra.push(Ms1Spectrum {
            rt,
            tic,
            mz,
            intensity,
        });
    }

    Ok(ms1_spectra)
}

/// Get the maximum m/z value across all MS1 spectra
pub fn get_max_mz(ms1_spectra: &[Ms1Spectrum]) -> f64 {
    ms1_spectra
        .iter()
        .flat_map(|s| s.mz.iter())
        .copied()
        .fold(0.0f64, f64::max)
}

// ============================================================================
// MS2 Spectrum Data Extraction (for oxonium ion screening)
// ============================================================================

/// MS2 spectrum data for oxonium ion screening
#[derive(Debug, Clone)]
pub struct Ms2Spectrum {
    /// Scan number
    pub scan: u32,
    /// Retention time in minutes
    pub rt: f64,
    /// Precursor m/z
    pub precursor_mz: f64,
    /// Precursor charge (0 if unknown)
    pub precursor_charge: u32,
    /// m/z values
    pub mz: Vec<f64>,
    /// Intensity values
    pub intensity: Vec<f64>,
}

/// Extract all MS2 spectra from an mzML file.
///
/// Returns a vector of MS2 spectra with precursor info and peak arrays.
/// This is used for oxonium ion screening.
///
/// # Arguments
/// * `path` - Path to the mzML file (supports .mzML and .mzML.gz)
///
/// # Returns
/// Vector of MS2 spectra with their data
pub fn extract_ms2_spectra(path: &Path) -> Result<Vec<Ms2Spectrum>> {
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
        extract_ms2_from_reader(cursor)
    } else {
        let reader = BufReader::new(file);
        extract_ms2_from_reader(reader)
    }
}

/// Internal function to extract MS2 spectra from any reader
fn extract_ms2_from_reader<R: std::io::Read + std::io::Seek>(
    reader: R,
) -> Result<Vec<Ms2Spectrum>> {
    let mut mzml_reader = MzMLReader::new_indexed(reader);
    let mut ms2_spectra = Vec::new();

    for spectrum in mzml_reader.iter() {
        if spectrum.ms_level() != 2 {
            continue;
        }

        let desc = spectrum.description();

        // Extract scan number from native ID (e.g., "controllerType=0 controllerNumber=1 scan=9681")
        let scan = extract_scan_from_id(&desc.id);

        // Get retention time (in minutes)
        let rt = desc
            .acquisition
            .first_scan()
            .map(|s| s.start_time)
            .unwrap_or(0.0);

        // Precursor m/z, with the same fallback Sage v0.15 applies: when the
        // selected ion carries no m/z, use the isolation-window target
        // (sage-cloudpath/src/mzml.rs). Without it recon and Sage disagree about
        // the precursor of any scan whose converter omitted the selected ion,
        // and oxonium screening sees m/z 0.
        //
        // ⚠ UNEXERCISED on the four committed files. All are DDA Thermo and
        // carry a proper selected-ion m/z, so this branch never runs here and no
        // number moves. Check it against the first ion-trap or DIA file.
        let (precursor_mz, precursor_charge) = desc
            .precursor
            .first()
            .map(|p| {
                let charge = p.ions.first().and_then(|ion| ion.charge()).unwrap_or(0) as u32;
                let mz = match p.ions.first().map(|ion| ion.mz()) {
                    Some(m) if m > 0.0 => m,
                    _ => p.isolation_window.target as f64,
                };
                (mz, charge)
            })
            .unwrap_or((0.0, 0));

        // Get peak data
        let peaks = spectrum.peaks();
        let mut mz = Vec::with_capacity(peaks.len());
        let mut intensity = Vec::with_capacity(peaks.len());

        for peak in peaks.iter() {
            mz.push(peak.mz());
            intensity.push(peak.intensity() as f64);
        }

        ms2_spectra.push(Ms2Spectrum {
            scan,
            rt,
            precursor_mz,
            precursor_charge,
            mz,
            intensity,
        });
    }

    Ok(ms2_spectra)
}

/// Extract scan number from native ID string
fn extract_scan_from_id(id: &str) -> u32 {
    // Look for "scan=" followed by digits
    if let Some(pos) = id.find("scan=") {
        let after_scan = &id[pos + 5..];
        let num_str: String = after_scan
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        return num_str.parse().unwrap_or(0);
    }
    // Fallback: try parsing the whole string as a number
    id.parse().unwrap_or(0)
}

// ============================================================================
// MS1 Precursor Intensity Extraction (fixed RT window approach)
// ============================================================================

/// Proton mass for m/z calculation (Da).
///
/// This is the ONLY definition in the crate. `mod_discovery` and `polymer`
/// import it. Do not add a local copy: the constant was declared three times
/// before, and `polymer` held 1.007276466879 while this file held
/// 1.007276466812. The test `proton_mass_has_exactly_one_definition` reads the
/// source files and fails if a second declaration comes back.
pub const PROTON_MASS: f64 = 1.007276466812;

// Import C13_C12_DIFF from sage_results for isotope spacing calculations
use crate::sage_results::C13_C12_DIFF;

/// Precursor query for MS1 intensity extraction
#[derive(Debug, Clone)]
pub struct PrecursorQuery {
    /// Precursor m/z
    pub mz: f64,
    /// Retention time in minutes
    pub rt: f64,
    /// Scan number (for matching back to PSMs)
    pub scan: u32,
    /// Charge state (for isotope spacing calculation)
    pub charge: u32,
}

/// Build precursor queries from PSMs for MS1 intensity extraction.
///
/// Converts PSM experimental mass and charge to precursor m/z.
/// Formula: precursor_mz = (expmass + proton_mass) / charge
///
/// # Arguments
/// * `psms` - Slice of PSMs with expmass, charge, rt, and scannr fields
///
/// # Returns
/// Vector of PrecursorQuery for MS1 lookup
pub fn build_precursor_queries_from_psms(psms: &[crate::sage_results::Psm]) -> Vec<PrecursorQuery> {
    psms.iter()
        .map(|psm| {
            // Convert experimental mass to m/z
            // expmass is the neutral mass, so we add one proton and divide by charge
            let precursor_mz = (psm.expmass + PROTON_MASS) / psm.charge as f64;
            PrecursorQuery {
                mz: precursor_mz,
                rt: psm.rt,
                scan: psm.scannr,
                charge: psm.charge,
            }
        })
        .collect()
}

/// Result of MS1 precursor intensity extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecursorIntensityResult {
    /// Scan number
    pub scan: u32,
    /// Summed MS1 intensity at precursor m/z within RT window
    pub intensity: f64,
    /// Number of MS1 scans contributing to the intensity
    pub ms1_scans_used: usize,
}

/// Extract MS1 precursor intensities for a list of queries.
///
/// For each query (precursor m/z + RT), finds MS1 scans within the RT window
/// and sums the intensity at the precursor m/z (within ppm tolerance).
///
/// # Arguments
/// * `ms1_spectra` - Pre-loaded MS1 spectra
/// * `queries` - List of precursor queries (m/z, RT, scan)
/// * `rt_window_minutes` - RT window half-width in minutes (default: 1.0)
/// * `mz_tolerance_ppm` - m/z tolerance in ppm (default: 10.0)
///
/// # Returns
/// Vector of intensity results, one per query
pub fn extract_precursor_intensities(
    ms1_spectra: &[Ms1Spectrum],
    queries: &[PrecursorQuery],
    rt_window_minutes: f64,
    mz_tolerance_ppm: f64,
) -> Vec<PrecursorIntensityResult> {
    queries
        .iter()
        .map(|query| {
            let mut total_intensity = 0.0;
            let mut scans_used = 0usize;

            // Find MS1 scans within RT window
            for ms1 in ms1_spectra {
                if (ms1.rt - query.rt).abs() > rt_window_minutes {
                    continue;
                }

                // Find max intensity at precursor m/z within tolerance
                let tol = mz_tolerance_ppm * query.mz / 1_000_000.0;
                let mut max_intensity = 0.0;

                for (&mz, &intensity) in ms1.mz.iter().zip(ms1.intensity.iter()) {
                    if (mz - query.mz).abs() <= tol && intensity > max_intensity {
                        max_intensity = intensity;
                    }
                }

                if max_intensity > 0.0 {
                    total_intensity += max_intensity;
                    scans_used += 1;
                }
            }

            PrecursorIntensityResult {
                scan: query.scan,
                intensity: total_intensity,
                ms1_scans_used: scans_used,
            }
        })
        .collect()
}

/// Compute total MS1 intensity explained by identified PSMs.
///
/// This is a higher-level function that takes PSM data and computes
/// the total MS1 signal explained by identified peptides.
///
/// # Arguments
/// * `ms1_spectra` - Pre-loaded MS1 spectra
/// * `queries` - Precursor queries from PSMs
/// * `rt_window_minutes` - RT window half-width (default: 1.0)
/// * `mz_tolerance_ppm` - m/z tolerance (default: 10.0)
///
/// # Returns
/// (total_explained_intensity, total_ms1_tic)
pub fn compute_ms1_signal_fate(
    ms1_spectra: &[Ms1Spectrum],
    queries: &[PrecursorQuery],
    rt_window_minutes: f64,
    mz_tolerance_ppm: f64,
) -> (f64, f64) {
    // Total MS1 TIC
    let total_ms1_tic: f64 = ms1_spectra.iter().map(|s| s.tic).sum();

    // Extract precursor intensities
    let results =
        extract_precursor_intensities(ms1_spectra, queries, rt_window_minutes, mz_tolerance_ppm);

    // Sum explained intensity
    let explained_intensity: f64 = results.iter().map(|r| r.intensity).sum();

    (explained_intensity, total_ms1_tic)
}

// ============================================================================
// Improved MS1 Precursor Intensity Extraction (Phase 6D v2)
// ============================================================================

/// Result of improved MS1 precursor intensity extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovedPrecursorResult {
    /// Scan number
    pub scan: u32,
    /// Summed isotope envelope intensity (M+0 to M+2)
    pub envelope_intensity: f64,
    /// Number of MS1 scans contributing
    pub ms1_scans_used: usize,
    /// Number of isotope peaks found (0-3)
    pub isotopes_found: u8,
}

/// Extract isotope envelope intensity from a single MS1 spectrum.
///
/// Sums M+0, M+1, M+2 peaks at charge-adjusted spacing.
/// Isotope spacing = C13_C12_DIFF / charge
fn extract_envelope_intensity(
    ms1: &Ms1Spectrum,
    mz: f64,
    charge: u32,
    mz_tolerance_ppm: f64,
) -> (f64, u8) {
    let isotope_spacing = C13_C12_DIFF / charge as f64;
    let mut total_intensity = 0.0;
    let mut isotopes_found = 0u8;

    // Look for M+0, M+1, M+2
    for k in 0..3 {
        let target_mz = mz + k as f64 * isotope_spacing;
        let tol = mz_tolerance_ppm * target_mz / 1_000_000.0;

        // Find max intensity peak within tolerance
        let mut max_intensity = 0.0;
        for (&peak_mz, &intensity) in ms1.mz.iter().zip(ms1.intensity.iter()) {
            if (peak_mz - target_mz).abs() <= tol && intensity > max_intensity {
                max_intensity = intensity;
            }
        }

        if max_intensity > 0.0 {
            total_intensity += max_intensity;
            isotopes_found += 1;
        }
    }

    (total_intensity, isotopes_found)
}

/// Unique precursor feature for deduplication
#[derive(Debug, Clone, PartialEq)]
struct PrecursorFeature {
    /// Representative m/z
    mz: f64,
    /// Representative RT
    rt: f64,
    /// Charge state
    charge: u32,
    /// Integrated envelope intensity
    intensity: f64,
    /// PSM scans that map to this feature
    psm_scans: Vec<u32>,
}

/// Deduplicate PSMs into unique precursor features.
///
/// Groups PSMs by (m/z, RT, charge) within tolerances.
/// Returns unique features with their integrated intensities.
fn deduplicate_precursors(
    queries: &[PrecursorQuery],
    mz_tolerance_ppm: f64,
    rt_tolerance_minutes: f64,
) -> Vec<PrecursorFeature> {
    let mut features: Vec<PrecursorFeature> = Vec::new();

    for query in queries {
        // Check if this query matches an existing feature
        let mut matched = false;
        for feature in &mut features {
            if feature.charge != query.charge {
                continue;
            }
            let mz_tol = mz_tolerance_ppm * feature.mz / 1_000_000.0;
            if (feature.mz - query.mz).abs() <= mz_tol
                && (feature.rt - query.rt).abs() <= rt_tolerance_minutes
            {
                // Add to existing feature
                feature.psm_scans.push(query.scan);
                matched = true;
                break;
            }
        }

        if !matched {
            // Create new feature
            features.push(PrecursorFeature {
                mz: query.mz,
                rt: query.rt,
                charge: query.charge,
                intensity: 0.0,
                psm_scans: vec![query.scan],
            });
        }
    }

    features
}

/// Improved MS1 signal fate result with multiple metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovedMs1SignalFate {
    /// Total MS1 TIC (all signal)
    pub total_ms1_tic: f64,
    /// Peptide-like MS1 TIC (400-1200 m/z range)
    pub peptide_like_tic: f64,
    /// Explained intensity (isotope envelope, deduplicated)
    pub explained_intensity: f64,
    /// Number of unique precursor features
    pub unique_features: usize,
    /// Number of PSMs mapped to features
    pub total_psms: usize,
    /// PSMs with detectable MS1 signal
    pub psms_with_signal: usize,
    /// Explained % of total TIC
    pub pct_total_tic: f64,
    /// Explained % of peptide-like TIC
    pub pct_peptide_like_tic: f64,
}

/// Compute improved MS1 signal fate with all enhancements:
/// 1. Isotope envelope summing (M+0, M+1, M+2)
/// 2. Precursor deduplication
/// 3. Scan-based RT window (±N scans)
/// 4. Peptide-like denominator metric
///
/// # Arguments
/// * `ms1_spectra` - Pre-loaded MS1 spectra (must be sorted by RT)
/// * `queries` - Precursor queries from PSMs
/// * `scan_window` - Number of MS1 scans to use (±N from center)
/// * `mz_tolerance_ppm` - m/z tolerance in ppm
///
/// # Returns
/// Improved signal fate metrics
pub fn compute_improved_ms1_signal_fate(
    ms1_spectra: &[Ms1Spectrum],
    queries: &[PrecursorQuery],
    scan_window: usize,
    mz_tolerance_ppm: f64,
) -> ImprovedMs1SignalFate {
    // 1. Compute total MS1 TIC
    let total_ms1_tic: f64 = ms1_spectra.iter().map(|s| s.tic).sum();

    // 2. Compute peptide-like TIC (400-1200 m/z range, typical DDA selection)
    let peptide_like_tic: f64 = ms1_spectra
        .iter()
        .map(|ms1| {
            ms1.mz
                .iter()
                .zip(ms1.intensity.iter())
                .filter(|(&mz, _)| (400.0..=1200.0).contains(&mz))
                .map(|(_, &intensity)| intensity)
                .sum::<f64>()
        })
        .sum();

    // 3. Deduplicate PSMs into unique precursor features
    // Use 10 ppm and 0.5 min for feature grouping
    let mut features = deduplicate_precursors(queries, 10.0, 0.5);

    // 4. For each feature, extract isotope envelope intensity
    let mut psms_with_signal = 0usize;

    for feature in &mut features {
        // Find the MS1 scan closest to the feature RT
        let center_idx = ms1_spectra
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                (a.rt - feature.rt)
                    .abs()
                    .partial_cmp(&(b.rt - feature.rt).abs())
                    .unwrap()
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Extract from ±scan_window scans
        let start_idx = center_idx.saturating_sub(scan_window);
        let end_idx = (center_idx + scan_window + 1).min(ms1_spectra.len());

        let mut total_envelope = 0.0;
        let mut scans_with_signal = 0;

        for ms1 in &ms1_spectra[start_idx..end_idx] {
            let (envelope, isotopes) =
                extract_envelope_intensity(ms1, feature.mz, feature.charge, mz_tolerance_ppm);
            if isotopes >= 2 {
                // Require at least 2 isotope peaks for confidence
                total_envelope += envelope;
                scans_with_signal += 1;
            }
        }

        feature.intensity = total_envelope;
        if scans_with_signal > 0 {
            psms_with_signal += feature.psm_scans.len();
        }
    }

    // 5. Sum explained intensity (deduplicated)
    let explained_intensity: f64 = features.iter().map(|f| f.intensity).sum();

    // 6. Compute percentages
    let pct_total_tic = if total_ms1_tic > 0.0 {
        100.0 * explained_intensity / total_ms1_tic
    } else {
        0.0
    };

    let pct_peptide_like_tic = if peptide_like_tic > 0.0 {
        100.0 * explained_intensity / peptide_like_tic
    } else {
        0.0
    };

    ImprovedMs1SignalFate {
        total_ms1_tic,
        peptide_like_tic,
        explained_intensity,
        unique_features: features.len(),
        total_psms: queries.len(),
        psms_with_signal,
        pct_total_tic,
        pct_peptide_like_tic,
    }
}

/// Print improved MS1 signal fate summary
pub fn print_improved_ms1_signal_fate(fate: &ImprovedMs1SignalFate) {
    println!();
    println!("=== Improved MS1 Signal Fate ===");
    println!();
    println!("Denominators:");
    println!("  Total MS1 TIC:        {:.2e}", fate.total_ms1_tic);
    println!(
        "  Peptide-like TIC:     {:.2e} (400-1200 m/z)",
        fate.peptide_like_tic
    );
    println!();
    println!("Numerator (isotope envelope, deduplicated):");
    println!("  Explained intensity:  {:.2e}", fate.explained_intensity);
    println!("  Unique features:      {}", fate.unique_features);
    println!("  Total PSMs:           {}", fate.total_psms);
    println!(
        "  PSMs with signal:     {} ({:.1}%)",
        fate.psms_with_signal,
        100.0 * fate.psms_with_signal as f64 / fate.total_psms.max(1) as f64
    );
    println!();
    println!("Explained Percentages:");
    println!("  % of total MS1 TIC:      {:.1}%", fate.pct_total_tic);
    println!(
        "  % of peptide-like TIC:   {:.1}%",
        fate.pct_peptide_like_tic
    );
}

// ============================================================================
// Mass analyzer detection — the pass-1 MS2 fragment tolerance
// ============================================================================
//
// WHY THIS EXISTS. `fragment_tol` is a pass-1 INPUT, not a report field. A
// search already run at a ppm fragment tolerance against ion-trap MS2 cannot be
// rescued downstream, because almost nothing matched in the first place. So the
// analyzer has to be known BEFORE Sage runs. It is: the mzML header and scan
// metadata carry it, with no PSMs involved.
//
// TWO SIGNALS CARRY THE ANALYZER, AND THEY ARE NOT EQUALLY GOOD.
//
//  1. The per-scan Thermo filter string (MS:1000512). Its first token names the
//     analyzer that acquired THAT scan. PREFERRED, because being per-scan it can
//     express "FT MS1, ion-trap MS2" inside one run — which is exactly the
//     hybrid case that matters.
//  2. The file-level instrumentConfiguration componentList analyzer term, a
//     child of MS:1000443. Fallback, used only when no filter string is present.
//
// MEASURED 2026-08-28 ON THIS REPO'S THREE FILES: the CV term alone is not
// trustworthy. `B.naive_01steady-state.mzML.gz` is a Q Exactive Plus, and its
// mzML declares the analyzer as MS:1000079 "fourier transform ion cyclotron
// resonance". A Q Exactive Plus has no ICR cell. This is a converter artifact,
// not a mystery: ThermoRawFileParser mapped every Thermo `MassAnalyzerFTMS` to
// MS:1000079 until 1.4.4, whose release notes read "Using CVTerm 'Orbitap'
// instead of 'FTICR' for Orbitrap-based instruments (closes #177)" (2024-05-10).
// B.naive was converted with 1.4.2; the other two with 1.4.4, and they declare
// MS:1000484 "orbitrap".
//
// The filter string is IMMUNE to that particular bug — Orbitrap and FT-ICR both
// report `FTMS`, and both are high resolution, so the tolerance is unchanged
// either way. That is the core reason it is preferred here.
//
// ONE MORE TRAP, recorded so it is not re-derived: ThermoRawFileParser builds
// the componentList from the analyzers ACTUALLY USED by the scans, not from the
// instrument's full capability. `2019-4-9_909c_0311` is an Orbitrap Fusion
// Lumos — a genuine hybrid — but the run used the Orbitrap for every scan, so
// only `orbitrap` is declared and that is CORRECT, not incomplete. Do not read a
// single declared analyzer as proof the instrument has only one.

/// Which tolerance bucket an analyzer falls in.
///
/// Buckets, not individual analyzers, because the pass-1 tolerance is a coarse
/// "could this detector plausibly be this far out" number, not a performance
/// figure. See `_dev/reference-notes/ms2-analyzer-tolerance-table.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AnalyzerClass {
    /// Orbitrap and FT-ICR. High-resolution FT.
    Orbitrap,
    /// The Astral analyzer. TOF-class hardware, but Orbitrap-class accuracy, so
    /// it is deliberately NOT bucketed with legacy TOF despite being its CV child.
    AstralTof,
    /// Legacy TOF / QTOF. Looser than Orbitrap-class in practice.
    LegacyTof,
    /// Any ion trap, and a bare quadrupole. Unit resolution — daltons, not ppm.
    IonTrap,
    /// In the CV, but with no tolerance bucket. Triggers the reported fallback.
    Unclassified,
}

impl AnalyzerClass {
    pub fn label(&self) -> &'static str {
        match self {
            AnalyzerClass::Orbitrap => "Orbitrap / FT-ICR",
            AnalyzerClass::AstralTof => "Astral (TOF-class)",
            AnalyzerClass::LegacyTof => "TOF / QTOF (legacy)",
            AnalyzerClass::IonTrap => "ion trap / quadrupole (unit resolution)",
            AnalyzerClass::Unclassified => "unclassified analyzer",
        }
    }
}

/// Whether a term's bucket came from the curated table or was extended here.
///
/// The curated CSV names only 6 of the 12 analyzer terms in the CV. The other 6
/// are assigned by CV parentage, and are marked so the write-up can say which is
/// which rather than implying all 12 were specified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BucketSource {
    /// Named in `_dev/reference-notes/analyzer-tolerances/`.
    Curated,
    /// Assigned here, by parentage from a curated term.
    ExtendedHere,
}

/// One PSI-MS mass analyzer term, with its bucket.
#[derive(Debug, Clone, Copy)]
pub struct MassAnalyzerTerm {
    pub accession: &'static str,
    pub name: &'static str,
    pub class: AnalyzerClass,
    pub bucket_source: BucketSource,
}

/// Every descendant of MS:1000443 "mass analyzer type" in the PSI-MS CV.
///
/// The term set is DERIVED, not asserted: `_dev/testing/scripts/psi_ms_analyzer_terms.py`
/// walks `is_a` from MS:1000443 in a pinned `psi-ms.obo` snapshot and hard-fails
/// if this table gains, loses, or misnames a term. It runs as Tier 4 of
/// `run_validation.py`.
///
/// ⚠ NOTE THE TRAP IN MS:1000264. ThermoRawFileParser maps its `MassAnalyzerITMS`
/// to the GENERIC parent MS:1000264 "ion trap", not to MS:1000082 or MS:1000291.
/// A table listing only the specific children MISSES every ThermoRawFileParser-
/// converted ion-trap file — precisely the case this feature exists to catch.
/// The curated CSV names MS:1000082/78/83 but not MS:1000264 or MS:1000291;
/// both are added here for that reason.
pub const MASS_ANALYZER_TERMS: &[MassAnalyzerTerm] = &[
    // --- Orbitrap-class, 50 ppm --------------------------------------------
    MassAnalyzerTerm {
        accession: "MS:1000484",
        name: "orbitrap",
        class: AnalyzerClass::Orbitrap,
        bucket_source: BucketSource::Curated,
    },
    // FT-ICR is not in the curated CSV. It is high-resolution FT like the
    // Orbitrap and shares its accuracy regime, so it takes the same bucket.
    MassAnalyzerTerm {
        accession: "MS:1000079",
        name: "fourier transform ion cyclotron resonance",
        class: AnalyzerClass::Orbitrap,
        bucket_source: BucketSource::ExtendedHere,
    },
    // --- Astral, 50 ppm ----------------------------------------------------
    MassAnalyzerTerm {
        accession: "MS:1003379",
        name: "asymmetric track lossless time-of-flight analyzer",
        class: AnalyzerClass::AstralTof,
        bucket_source: BucketSource::Curated,
    },
    // --- legacy TOF, 100 ppm -----------------------------------------------
    MassAnalyzerTerm {
        accession: "MS:1000084",
        name: "time-of-flight",
        class: AnalyzerClass::LegacyTof,
        bucket_source: BucketSource::Curated,
    },
    // --- unit resolution, 1.0 Da -------------------------------------------
    MassAnalyzerTerm {
        accession: "MS:1000082",
        name: "quadrupole ion trap",
        class: AnalyzerClass::IonTrap,
        bucket_source: BucketSource::Curated,
    },
    MassAnalyzerTerm {
        accession: "MS:1000078",
        name: "axial ejection linear ion trap",
        class: AnalyzerClass::IonTrap,
        bucket_source: BucketSource::Curated,
    },
    MassAnalyzerTerm {
        accession: "MS:1000083",
        name: "radial ejection linear ion trap",
        class: AnalyzerClass::IonTrap,
        bucket_source: BucketSource::Curated,
    },
    // The generic parent, and the linear-ion-trap parent of the two above.
    // Neither is in the curated CSV; MS:1000264 is the one Thermo actually writes.
    MassAnalyzerTerm {
        accession: "MS:1000264",
        name: "ion trap",
        class: AnalyzerClass::IonTrap,
        bucket_source: BucketSource::ExtendedHere,
    },
    MassAnalyzerTerm {
        accession: "MS:1000291",
        name: "linear ion trap",
        class: AnalyzerClass::IonTrap,
        bucket_source: BucketSource::ExtendedHere,
    },
    // A bare quadrupole is unit-resolution too — the curated bucket is named
    // "Ion trap / linear ion trap (low-res, quad-like)".
    MassAnalyzerTerm {
        accession: "MS:1000081",
        name: "quadrupole",
        class: AnalyzerClass::IonTrap,
        bucket_source: BucketSource::ExtendedHere,
    },
    // --- no bucket: fall back and report ------------------------------------
    MassAnalyzerTerm {
        accession: "MS:1000080",
        name: "magnetic sector",
        class: AnalyzerClass::Unclassified,
        bucket_source: BucketSource::ExtendedHere,
    },
    MassAnalyzerTerm {
        accession: "MS:1000254",
        name: "electrostatic energy analyzer",
        class: AnalyzerClass::Unclassified,
        bucket_source: BucketSource::ExtendedHere,
    },
];

/// The analyzer token that opens a Thermo filter string, mapped to its bucket.
///
/// Source: `ThermoRawFileParser/Writer/OntologyMapping.cs` `MassAnalyzerTypes`.
/// `TQMS`/`SQMS` are triple and single quadrupole. Note `ASTMS` maps to the
/// Astral bucket, NOT to legacy TOF.
pub const FILTER_STRING_ANALYZERS: &[(&str, AnalyzerClass)] = &[
    ("FTMS", AnalyzerClass::Orbitrap),
    ("ITMS", AnalyzerClass::IonTrap),
    ("TOFMS", AnalyzerClass::LegacyTof),
    ("ASTMS", AnalyzerClass::AstralTof),
    ("TQMS", AnalyzerClass::IonTrap),
    ("SQMS", AnalyzerClass::IonTrap),
    ("SECTOR", AnalyzerClass::Unclassified),
];

/// Classify a CV accession from a `componentList` analyzer term.
pub fn class_from_accession(accession: &str) -> Option<AnalyzerClass> {
    MASS_ANALYZER_TERMS
        .iter()
        .find(|t| t.accession.eq_ignore_ascii_case(accession))
        .map(|t| t.class)
}

/// Classify a Thermo filter string by its leading analyzer token.
pub fn class_from_filter_string(filter: &str) -> Option<AnalyzerClass> {
    let token = filter.split_whitespace().next()?;
    FILTER_STRING_ANALYZERS
        .iter()
        .find(|(name, _)| token.eq_ignore_ascii_case(name))
        .map(|(_, class)| *class)
}

/// A symmetric fragment tolerance half-width, carrying its unit.
///
/// The unit is the whole point: a ppm number is meaningless on an ion trap and a
/// dalton number is absurdly wide on an Orbitrap. Keeping the unit in the type
/// makes the two impossible to merge by accident.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FragmentTolerance {
    Ppm(f64),
    Da(f64),
}

impl std::fmt::Display for FragmentTolerance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Trimmed to a precision a user can act on. A derived tolerance is
            // a float (5 x a measured median), and "±6.1261980000000005 ppm" is
            // a number nobody can type into a search engine. The full value
            // still reaches Sage through `to_sage_json`, which does not round.
            FragmentTolerance::Ppm(v) => write!(f, "±{} ppm", trim(*v, 2)),
            FragmentTolerance::Da(v) => write!(f, "±{} Da", trim(*v, 4)),
        }
    }
}

/// Format with at most `dp` decimals, dropping a trailing `.0`.
fn trim(v: f64, dp: usize) -> String {
    let s = format!("{v:.dp$}");
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    };
    if s.is_empty() || s == "-" {
        "0".to_string()
    } else {
        s
    }
}

impl FragmentTolerance {
    /// Render as the `fragment_tol` object a Sage config expects.
    pub fn to_sage_json(&self) -> serde_json::Value {
        match self {
            FragmentTolerance::Ppm(v) => serde_json::json!({ "ppm": [-v, v] }),
            FragmentTolerance::Da(v) => serde_json::json!({ "da": [-v, v] }),
        }
    }
}

// --- the pass-1 numbers -----------------------------------------------------
//
// THESE ARE DELIBERATELY LOOSE. Pass 1 must assume the instrument could be well
// out of calibration, so each bucket sits far above its typical performance
// rather than at it. Typical figures and the resulting choice are curated in
// `_dev/reference-notes/analyzer-tolerances/`; the derived table with citations is
// `_dev/reference-notes/ms2-analyzer-tolerance-table.md`.

/// Orbitrap / FT-ICR. Typical 1-5 ppm, up to ~20 ppm poorly calibrated.
///
/// **20, not 50.** The constant read 50 while this very comment gave ~20 ppm as
/// the poorly-calibrated worst case — 2.5x its own documented bound. It was also
/// WIDER than `UNKNOWN_MS2_FALLBACK_PPM`, so detecting an Orbitrap correctly gave
/// a worse search than failing to detect anything.
///
/// 20 is chosen as the documented worst case for the class, NOT as the empirical
/// optimum on our four files. All four are well-calibrated Orbitraps and would
/// keep improving below 20; tuning to them would overfit a default that has to
/// survive somebody else's poorly-calibrated instrument.
pub const ORBITRAP_MS2_HALF_WIDTH_PPM: f64 = 20.0;

/// Astral. Typical <5 ppm RMS on external calibration, ~3 ppm internal.
/// Same bucket as Orbitrap: Astral behaves like a high-res TOF, not a legacy QTOF.
///
/// **DEFINED AS the Orbitrap constant, not as a second literal.** It read `50.0`
/// while Orbitrap read `50.0`, so "same bucket as Orbitrap" was true only by
/// coincidence; the moment Orbitrap moved to 20 they silently disagreed, and the
/// doc comment above became false. `astral_is_not_bucketed_with_legacy_tof`
/// caught exactly that. Binding the two removes the way they can drift apart.
///
/// ⚠ No Astral file exists in the test set — all four are FTMS — so this is a
/// CURATED choice following the stated intent, not a measurement.
///
/// It stays a NUMERIC LITERAL rather than an alias of the Orbitrap constant
/// because `_dev/testing/scripts/psi_ms_analyzer_terms.py` pins each analyzer class to
/// a number and cannot resolve an alias. The tie is enforced instead by
/// `astral_is_not_bucketed_with_legacy_tof`, which asserts this equals
/// `ORBITRAP_MS2_HALF_WIDTH_PPM` — so the two still cannot drift apart unnoticed.
pub const ASTRAL_MS2_HALF_WIDTH_PPM: f64 = 20.0;

/// Legacy TOF / QTOF. Typical ~10-30 ppm, and 30 ppm is already a common default.
pub const LEGACY_TOF_MS2_HALF_WIDTH_PPM: f64 = 100.0;

/// Ion trap / quadrupole. Typical 0.3-0.8 Da at unit resolution.
pub const ION_TRAP_MS2_HALF_WIDTH_DA: f64 = 1.0;

/// Used when the MS2 analyzer cannot be determined, has no bucket, or changes
/// inside the run. This is the value every pass-1 template already hardcodes, so
/// the fallback is "behave as recon always has" — and say so loudly.
///
/// ⚠ **It now EQUALS the Orbitrap bucket.** It used to be tighter (20 against
/// Orbitrap's 50), and this comment used to claim that as a deliberate
/// conservatism. Orbitrap moved to 20 on 2026-08-31, so the two coincide and the
/// old claim is no longer true. The consequence is worth stating plainly: on a
/// file whose analyzer cannot be read, recon searches as if it were an Orbitrap.
/// That is right for the Thermo-dominated data this tool sees and WRONG for an
/// unreadable ion-trap file, which needs ~1 Da. It is reported every time it is
/// used, and `assumed: true` is set so the report says so.
pub const UNKNOWN_MS2_FALLBACK_PPM: f64 = 20.0;

/// The tolerance for a bucket, or `None` for a bucket with no number.
pub fn bucket_tolerance(class: AnalyzerClass) -> Option<FragmentTolerance> {
    match class {
        AnalyzerClass::Orbitrap => Some(FragmentTolerance::Ppm(ORBITRAP_MS2_HALF_WIDTH_PPM)),
        AnalyzerClass::AstralTof => Some(FragmentTolerance::Ppm(ASTRAL_MS2_HALF_WIDTH_PPM)),
        AnalyzerClass::LegacyTof => Some(FragmentTolerance::Ppm(LEGACY_TOF_MS2_HALF_WIDTH_PPM)),
        AnalyzerClass::IonTrap => Some(FragmentTolerance::Da(ION_TRAP_MS2_HALF_WIDTH_DA)),
        AnalyzerClass::Unclassified => None,
    }
}

/// Why the chosen tolerance is what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToleranceBasis {
    /// One analyzer bucket was detected and its number used.
    Detected,
    /// The MS2 detector changes inside the run. No single value can be right, so
    /// recon assumes the fallback and reports what it saw.
    DetectorSwitched,
    /// No analyzer could be read, or the analyzer has no bucket.
    Unknown,
}

/// The pass-1 MS2 fragment tolerance recon will actually search with.
///
/// ALWAYS yields a tolerance. Recon never refuses to search on the strength of
/// analyzer detection: an undetectable or switching detector falls back to
/// [`UNKNOWN_MS2_FALLBACK_PPM`] and says so, because a reconnaissance tool that
/// halts is less useful than one that proceeds with a stated assumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ms2TolDecision {
    pub tolerance: FragmentTolerance,
    pub basis: ToleranceBasis,
    /// The bucket detected, when there was exactly one.
    pub detected: Option<AnalyzerClass>,
    /// Human-readable analyzer labels seen at MS2, for the report.
    pub ms2_analyzers: Vec<String>,
    /// Human-readable analyzer labels seen at MS1, for the report.
    pub ms1_analyzers: Vec<String>,
    /// True when recon substituted an assumption for a real detection.
    pub assumed: bool,
    pub explanation: String,
}

/// Decide the pass-1 MS2 fragment tolerance from a census.
pub fn resolve_ms2_tolerance(census: &AnalyzerCensus) -> Ms2TolDecision {
    let ms1_analyzers = census
        .ms1
        .classes_present()
        .iter()
        .map(|c| c.label().to_string())
        .collect();
    let ms2_analyzers: Vec<String> = census
        .ms2
        .classes_present()
        .iter()
        .map(|c| c.label().to_string())
        .collect();
    let fallback = FragmentTolerance::Ppm(UNKNOWN_MS2_FALLBACK_PPM);

    // A run that changes MS2 detector cannot be served by one number. Say what
    // happened, assume the fallback, and carry on.
    if census.ms2.is_mixed() {
        return Ms2TolDecision {
            tolerance: fallback,
            basis: ToleranceBasis::DetectorSwitched,
            detected: None,
            explanation: format!(
                "The MS2 detector CHANGES inside this run ({}). One search runs at one \
                 fragment tolerance, so no single value is correct. recon assumed {fallback} \
                 and is telling you rather than guessing silently.",
                ms2_analyzers.join(" + ")
            ),
            ms1_analyzers,
            ms2_analyzers,
            assumed: true,
        };
    }

    match census.ms2.dominant().map(|(c, _)| c) {
        Some(class) => match bucket_tolerance(class) {
            Some(tol) => Ms2TolDecision {
                tolerance: tol,
                basis: ToleranceBasis::Detected,
                detected: Some(class),
                explanation: format!(
                    "MS2 analyzer is {}; pass-1 fragment tolerance {tol}.",
                    class.label()
                ),
                ms1_analyzers,
                ms2_analyzers,
                assumed: false,
            },
            None => Ms2TolDecision {
                tolerance: fallback,
                basis: ToleranceBasis::Unknown,
                detected: Some(class),
                explanation: format!(
                    "MS2 analyzer is {}, which has no tolerance bucket. recon assumed \
                     {fallback} and is telling you.",
                    class.label()
                ),
                ms1_analyzers,
                ms2_analyzers,
                assumed: true,
            },
        },
        None => Ms2TolDecision {
            tolerance: fallback,
            basis: ToleranceBasis::Unknown,
            detected: None,
            explanation: format!(
                "No MS2 analyzer could be read from this file. recon assumed {fallback} \
                 and is telling you."
            ),
            ms1_analyzers,
            ms2_analyzers,
            assumed: true,
        },
    }
}

/// How many scans per MS level to sample before deciding the analyzer.
///
/// An acquisition method does not swap the MS2 detector part-way through a run,
/// so the first handful of MS2 scans already answer the question. 100 is far
/// more than needed; it is cheap because detection stops reading as soon as the
/// MS2 sample is full.
pub const DEFAULT_ANALYZER_SAMPLE: usize = 100;

/// Spectrum counts by analyzer bucket at one MS level.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzerCounts {
    pub orbitrap: usize,
    pub astral_tof: usize,
    pub legacy_tof: usize,
    pub ion_trap: usize,
    pub unclassified: usize,
    /// A scan whose analyzer could not be read at all.
    pub unknown: usize,
}

impl AnalyzerCounts {
    fn add(&mut self, class: Option<AnalyzerClass>) {
        match class {
            Some(AnalyzerClass::Orbitrap) => self.orbitrap += 1,
            Some(AnalyzerClass::AstralTof) => self.astral_tof += 1,
            Some(AnalyzerClass::LegacyTof) => self.legacy_tof += 1,
            Some(AnalyzerClass::IonTrap) => self.ion_trap += 1,
            Some(AnalyzerClass::Unclassified) => self.unclassified += 1,
            None => self.unknown += 1,
        }
    }

    pub fn total(&self) -> usize {
        self.orbitrap
            + self.astral_tof
            + self.legacy_tof
            + self.ion_trap
            + self.unclassified
            + self.unknown
    }

    pub fn count_of(&self, class: AnalyzerClass) -> usize {
        match class {
            AnalyzerClass::Orbitrap => self.orbitrap,
            AnalyzerClass::AstralTof => self.astral_tof,
            AnalyzerClass::LegacyTof => self.legacy_tof,
            AnalyzerClass::IonTrap => self.ion_trap,
            AnalyzerClass::Unclassified => self.unclassified,
        }
    }

    fn pairs(&self) -> [(AnalyzerClass, usize); 5] {
        [
            (AnalyzerClass::Orbitrap, self.orbitrap),
            (AnalyzerClass::AstralTof, self.astral_tof),
            (AnalyzerClass::LegacyTof, self.legacy_tof),
            (AnalyzerClass::IonTrap, self.ion_trap),
            (AnalyzerClass::Unclassified, self.unclassified),
        ]
    }

    /// Every bucket that acquired at least one sampled scan.
    pub fn classes_present(&self) -> Vec<AnalyzerClass> {
        self.pairs()
            .into_iter()
            .filter(|(_, n)| *n > 0)
            .map(|(c, _)| c)
            .collect()
    }

    /// The bucket holding the most sampled scans, with its share of the sample.
    pub fn dominant(&self) -> Option<(AnalyzerClass, f64)> {
        let total = self.total();
        if total == 0 {
            return None;
        }
        let (class, n) = self.pairs().into_iter().max_by_key(|(_, n)| *n)?;
        if n == 0 {
            return None;
        }
        Some((class, n as f64 / total as f64))
    }

    /// True when more than one bucket appears in the sample — i.e. the detector
    /// changed inside the sampled stretch.
    pub fn is_mixed(&self) -> bool {
        self.classes_present().len() > 1
    }
}

/// What the mzML says about which analyzer acquired the scans.
///
/// ⚠ THIS IS A SAMPLE OF THE HEAD OF THE RUN, NOT A FULL CENSUS. Detection reads
/// until it has `sample_limit` MS2 scans and stops. That is sound because an
/// acquisition method does not switch the MS2 detector part-way through, but it
/// does mean a file that DOES switch late will not be caught. Recorded as a known
/// limitation rather than papered over.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerCensus {
    pub file_path: String,
    /// Instrument model CV term, e.g. "Orbitrap Fusion Lumos".
    pub instrument_model: Option<String>,
    /// Analyzer terms declared in `instrumentConfigurationList`, verbatim.
    pub declared_analyzers: Vec<String>,
    /// Which signal the per-scan classification came from.
    pub detection_source: String,
    pub ms1: AnalyzerCounts,
    pub ms2: AnalyzerCounts,
    /// How many MS2 scans detection was willing to read.
    pub sample_limit: usize,
    /// Total spectra actually read before detection stopped.
    pub spectra_examined: usize,
}

impl AnalyzerCensus {
    /// True when the sampled MS2 scans do not agree on one analyzer.
    pub fn ms2_switched(&self) -> bool {
        self.ms2.is_mixed()
    }

    /// The pass-1 MS2 fragment tolerance this file implies. Always answers.
    pub fn ms2_decision(&self) -> Ms2TolDecision {
        resolve_ms2_tolerance(self)
    }

    /// True when the declared componentList disagrees with what the scans say.
    /// Not fatal — the scans win — but worth printing, because it is how the
    /// ThermoRawFileParser FTICR mislabel becomes visible.
    pub fn declared_disagrees_with_scans(&self) -> bool {
        let declared: Vec<AnalyzerClass> = self
            .declared_analyzers
            .iter()
            .filter_map(|d| {
                d.rsplit_once(' ')
                    .map(|(_, acc)| acc.to_string())
                    .and_then(|acc| class_from_accession(&acc))
            })
            .collect();
        if declared.is_empty() {
            return false;
        }
        [self.ms1.dominant(), self.ms2.dominant()]
            .iter()
            .flatten()
            .any(|(c, _)| !declared.contains(c))
    }
}

/// Read an mzML and work out which analyzer acquired the MS1 and MS2 scans.
///
/// Samples the first [`DEFAULT_ANALYZER_SAMPLE`] MS2 scans. Prefers the per-scan
/// filter string; falls back to the file-level `instrumentConfiguration` analyzer
/// term. Needs no PSMs, so it can and must run before the pass-1 search.
pub fn detect_analyzers(path: &Path) -> Result<AnalyzerCensus> {
    detect_analyzers_sampled(path, DEFAULT_ANALYZER_SAMPLE)
}

/// [`detect_analyzers`] with an explicit per-level sample size.
pub fn detect_analyzers_sampled(path: &Path, sample_limit: usize) -> Result<AnalyzerCensus> {
    let file_path = path.display().to_string();
    let is_gzipped = path.extension().map(|e| e == "gz").unwrap_or(false);

    let file = File::open(path)
        .with_context(|| format!("Failed to open mzML file: {}", path.display()))?;

    // STREAMING, deliberately. Detection reads the head of the run and stops, so
    // it uses the non-indexed reader, which needs only `Read`. A gzipped input is
    // therefore decompressed as far as the sample and no further, and no spectrum
    // index is built. On the reference files that is the difference between ~19 s
    // and well under a second per file.
    if is_gzipped {
        detect_analyzers_from_reader(GzDecoder::new(file), file_path, sample_limit)
    } else {
        detect_analyzers_from_reader(BufReader::new(file), file_path, sample_limit)
    }
}

fn detect_analyzers_from_reader<R: std::io::Read>(
    reader: R,
    file_path: String,
    sample_limit: usize,
) -> Result<AnalyzerCensus> {
    let mut mzml_reader = MzMLReader::new(reader);
    // Analyzer detection never touches peaks, so skip decoding binary arrays.
    mzml_reader.detail_level = DetailLevel::MetadataOnly;

    // --- header: instrument model and declared analyzers ---------------------
    let mut instrument_model: Option<String> = None;
    let mut declared_analyzers: Vec<String> = Vec::new();
    // Per configuration id, the analyzer class it declares. Used as the fallback
    // when scans carry no filter string, and honoured per-scan so a multi-config
    // hybrid (which msconvert does emit) is read correctly.
    let mut class_by_config: HashMap<u32, AnalyzerClass> = HashMap::new();

    for (id, config) in mzml_reader.instrument_configurations().iter() {
        for param in config.params.iter() {
            // MS:1000529 is the serial number, not the model.
            if param.accession.is_some() && param.name != "instrument serial number" {
                instrument_model.get_or_insert_with(|| param.name.clone());
            }
        }
        for component in config.components.iter() {
            if component.component_type != ComponentType::Analyzer {
                continue;
            }
            for param in component.params.iter() {
                if let Some(acc) = param.accession {
                    let accession = format!("MS:{acc:07}");
                    declared_analyzers.push(format!("{} {}", param.name, accession));
                    if let Some(class) = class_from_accession(&accession) {
                        class_by_config.insert(*id, class);
                    }
                }
            }
        }
    }
    declared_analyzers.sort();
    declared_analyzers.dedup();

    // --- scans: sample the head of the run, then stop ------------------------
    let mut ms1 = AnalyzerCounts::default();
    let mut ms2 = AnalyzerCounts::default();
    let mut spectra_examined = 0usize;
    let mut saw_filter_string = false;

    // STOPPING RULE: read until the MS2 sample is full, however deep into the run
    // the MS2 scans start. A method that spends a long stretch on MS1 at the head
    // of a run must NOT cause detection to give up before it has seen any MS2 —
    // MS2 is the level whose analyzer decides `fragment_tol`. This is not
    // theoretical: in 2019-4-9_909c_0311 the first MS2 is at spectrum 140 and
    // 2063 MS1 scans precede the 100th MS2. The MS1 sample fills opportunistically
    // and never gates the stop.
    for spectrum in &mut mzml_reader {
        let level = spectrum.ms_level();
        if level != 1 && level != 2 {
            continue;
        }
        if ms2.total() >= sample_limit {
            break;
        }
        spectra_examined += 1;
        let counts = if level == 1 { &mut ms1 } else { &mut ms2 };
        if counts.total() >= sample_limit {
            continue;
        }

        let scan = spectrum.acquisition().first_scan();
        let class = match scan.and_then(|s| s.filter_string()) {
            Some(filter) => {
                saw_filter_string = true;
                class_from_filter_string(filter.as_ref())
            }
            // No filter string: fall back to the configuration this scan names.
            None => scan.and_then(|s| class_by_config.get(&s.instrument_configuration_id).copied()),
        };
        counts.add(class);
    }

    let detection_source = if saw_filter_string {
        "per-scan filter string (MS:1000512)".to_string()
    } else if !class_by_config.is_empty() {
        "instrumentConfiguration analyzer term".to_string()
    } else {
        "none — analyzer could not be determined".to_string()
    };

    Ok(AnalyzerCensus {
        file_path,
        instrument_model,
        declared_analyzers,
        detection_source,
        ms1,
        ms2,
        sample_limit,
        spectra_examined,
    })
}

/// Print the analyzer census and the tolerance it implies.
pub fn print_analyzer_census(census: &AnalyzerCensus) {
    println!("=== MS2 ANALYZER DETECTION ===");
    println!("  File:       {}", census.file_path);
    println!(
        "  Instrument: {}",
        census
            .instrument_model
            .as_deref()
            .unwrap_or("<not declared>")
    );
    println!(
        "  Declared:   {}",
        if census.declared_analyzers.is_empty() {
            "<none>".to_string()
        } else {
            census.declared_analyzers.join(", ")
        }
    );
    println!("  Source:     {}", census.detection_source);
    println!(
        "  Sampled:    {} scans read for the first {} MS2",
        census.spectra_examined, census.sample_limit
    );

    for (level, counts) in [("MS1", &census.ms1), ("MS2", &census.ms2)] {
        match counts.dominant() {
            Some((class, share)) => println!(
                "  {level}: {} ({}/{} sampled scans, {:.1}%){}",
                class.label(),
                counts.count_of(class),
                counts.total(),
                share * 100.0,
                if counts.is_mixed() {
                    "  ⚠ DETECTOR CHANGES"
                } else {
                    ""
                }
            ),
            None => println!("  {level}: no scans sampled"),
        }
    }

    if census.declared_disagrees_with_scans() {
        println!(
            "  ⚠ The declared componentList analyzer disagrees with the scans. \
             The scans win. This is how the ThermoRawFileParser <1.4.4 FTICR \
             mislabel shows up; it does not change the tolerance."
        );
    }

    let d = census.ms2_decision();
    println!();
    println!(
        "  → pass-1 fragment_tol: {}{}",
        d.tolerance,
        if d.assumed { "   (ASSUMED)" } else { "" }
    );
    println!("    {}", d.explanation);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `PROTON_MASS` was declared in three modules, with two different values.
    /// It is now declared here only. This test reads the crate source and fails
    /// if a second declaration comes back.
    ///
    /// The scan covers `src/*.rs`. Every module of this crate is a flat file
    /// there. Add a directory module and this test must be widened.
    #[test]
    fn proton_mass_has_exactly_one_definition() {
        // Built from two pieces so this line is not itself a match.
        let needle = concat!("const ", "PROTON_MASS");
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

        let mut found: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&src_dir).expect("the src directory must be readable") {
            let path = entry.expect("each directory entry must be readable").path();
            if path.extension().map(|e| e != "rs").unwrap_or(true) {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("each source file must be readable");
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            for _ in 0..text.matches(needle).count() {
                found.push(name.clone());
            }
        }
        found.sort();

        assert_eq!(
            found,
            vec!["mzml.rs".to_string()],
            "PROTON_MASS must be declared once, in mzml.rs. Import it elsewhere."
        );
    }

    #[test]
    fn test_mzml_stats_struct() {
        let stats = MzmlStats {
            total_spectra: 100,
            ms1_spectra: 20,
            ms2_spectra: 80,
            total_ms2_tic: 1e10,
            file_path: "test.mzML".to_string(),
        };

        assert_eq!(stats.total_spectra, 100);
        assert_eq!(stats.ms1_spectra, 20);
        assert_eq!(stats.ms2_spectra, 80);
    }

    #[test]
    fn test_stats_serialization() {
        let stats = MzmlStats {
            total_spectra: 100,
            ms1_spectra: 20,
            ms2_spectra: 80,
            total_ms2_tic: 1e10,
            file_path: "test.mzML".to_string(),
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"ms2_spectra\":80"));

        let parsed: MzmlStats = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.ms2_spectra, 80);
    }

    #[test]
    fn test_extract_scan_from_id() {
        assert_eq!(
            extract_scan_from_id("controllerType=0 controllerNumber=1 scan=9681"),
            9681
        );
        assert_eq!(extract_scan_from_id("scan=123"), 123);
        assert_eq!(extract_scan_from_id("456"), 456);
        assert_eq!(extract_scan_from_id("no_scan_here"), 0);
    }

    #[test]
    fn test_get_max_mz() {
        let spectra = vec![
            Ms1Spectrum {
                rt: 1.0,
                tic: 1000.0,
                mz: vec![100.0, 200.0],
                intensity: vec![10.0, 20.0],
            },
            Ms1Spectrum {
                rt: 2.0,
                tic: 2000.0,
                mz: vec![150.0, 500.0],
                intensity: vec![15.0, 50.0],
            },
        ];
        assert!((get_max_mz(&spectra) - 500.0).abs() < 0.001);
    }

    #[test]
    fn test_extract_precursor_intensities() {
        let ms1_spectra = vec![
            Ms1Spectrum {
                rt: 10.0,
                tic: 10000.0,
                mz: vec![500.0, 500.001, 600.0],
                intensity: vec![1000.0, 500.0, 2000.0],
            },
            Ms1Spectrum {
                rt: 10.5,
                tic: 12000.0,
                mz: vec![500.0005, 600.0],
                intensity: vec![1500.0, 2500.0],
            },
            Ms1Spectrum {
                rt: 15.0, // Outside RT window
                tic: 8000.0,
                mz: vec![500.0, 600.0],
                intensity: vec![800.0, 1600.0],
            },
        ];

        let queries = vec![PrecursorQuery {
            mz: 500.0,
            rt: 10.0,
            scan: 1,
            charge: 2u32,
        }];

        let results = extract_precursor_intensities(&ms1_spectra, &queries, 1.0, 10.0);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].scan, 1);
        // Should find peaks in first two MS1 scans (within RT window)
        // First scan: max at 500.0 = 1000.0
        // Second scan: max at 500.0005 = 1500.0
        // Total = 2500.0
        assert!((results[0].intensity - 2500.0).abs() < 0.1);
        assert_eq!(results[0].ms1_scans_used, 2);
    }

    #[test]
    fn test_compute_ms1_signal_fate() {
        let ms1_spectra = vec![
            Ms1Spectrum {
                rt: 10.0,
                tic: 10000.0,
                mz: vec![500.0],
                intensity: vec![1000.0],
            },
            Ms1Spectrum {
                rt: 20.0,
                tic: 20000.0,
                mz: vec![600.0],
                intensity: vec![2000.0],
            },
        ];

        let queries = vec![PrecursorQuery {
            mz: 500.0,
            rt: 10.0,
            scan: 1,
            charge: 2u32,
        }];

        let (explained, total) = compute_ms1_signal_fate(&ms1_spectra, &queries, 1.0, 10.0);

        assert!((total - 30000.0).abs() < 0.1);
        assert!((explained - 1000.0).abs() < 0.1);
    }
}

#[cfg(test)]
mod fragment_tolerance_display_tests {
    use super::*;

    /// A tolerance the user is shown must be typeable. The value handed to Sage
    /// is NOT rounded — only the display is.
    #[test]
    fn a_derived_tolerance_displays_at_usable_precision() {
        // The exact value the serum end-to-end run produced.
        let t = FragmentTolerance::Ppm(6.1261980000000005);
        assert_eq!(t.to_string(), "±6.13 ppm");
        // ...while Sage still receives full precision.
        assert_eq!(
            t.to_sage_json()["ppm"][1].as_f64().unwrap(),
            6.1261980000000005
        );
    }

    #[test]
    fn a_whole_number_tolerance_keeps_no_trailing_zeros() {
        assert_eq!(FragmentTolerance::Ppm(50.0).to_string(), "±50 ppm");
        assert_eq!(FragmentTolerance::Da(1.0).to_string(), "±1 Da");
    }

    /// An ion-trap tolerance is small; two decimals would round 0.0005 Da to
    /// zero and print "±0 Da", which reads as "no tolerance".
    #[test]
    fn a_small_da_tolerance_does_not_round_to_zero() {
        assert_eq!(FragmentTolerance::Da(0.5).to_string(), "±0.5 Da");
        assert_eq!(FragmentTolerance::Da(0.0625).to_string(), "±0.0625 Da");
    }
}
