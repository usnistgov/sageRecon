//! CLI for proteomics reconnaissance tool.
//!
//! Phase 7: Unified report output with JSON, text, and HTML formats.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use recon_tool::curated_mods::CuratedDb;
use recon_tool::digestion::{compute_digestion_stats, print_digestion_summary};
use recon_tool::mod_discovery::{
    run_mod_discovery, CalibrationMode, ModDiscoveryConfig, PeakAssignmentMode,
    NEAR_ZERO_THRESHOLD_DA,
};
use recon_tool::mzml::{
    self, extract_ms1_spectra, extract_ms2_spectra, get_max_mz, get_mzml_stats, print_mzml_stats,
};
use recon_tool::oxonium::{
    compute_screening_summary, print_screening_summary, screen_spectra, OxoniumScreeningConfig,
};
use recon_tool::polymer::search_polymers;
use recon_tool::qc::{compute_qc_stats, print_qc_summary};
use recon_tool::report::{
    compute_alkylation_check, generate_html_report, print_report_summary, ReconReport,
};
use recon_tool::sage_results::{parse_sage_results, FilterOptions};
use recon_tool::signal_fate::{
    compute_signal_fate, compute_signal_fate_with_mzml, print_signal_fate_summary,
};
use recon_tool::tier_assignment;
use recon_tool::unimod::UnimodDb;

/// Abundance floor as a percentage of the top non-zero peak, used ONLY on the
/// abundance path. Chosen 2026-08-25: gate 1 reads 0/0/0 and gate 4 reads 0 at
/// this value on all three test files. See NOTES "Step 2 unblocked".
const FLOOR_PCT_OF_TOP: f64 = 20.0;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "recon")]
#[command(about = "Proteomics reconnaissance tool using Sage open search")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// CLI-facing calibration mode (bridges to `mod_discovery::CalibrationMode`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum CalibrationArg {
    /// No calibration (raw isotope-corrected deltas)
    None,
    /// Da constant offset (default; historical Phase 7C behavior)
    DaScalar,
    /// ppm constant offset, applied per-PSM against each PSM's m/z
    PpmConstant,
}

impl From<CalibrationArg> for CalibrationMode {
    fn from(a: CalibrationArg) -> Self {
        match a {
            CalibrationArg::None => CalibrationMode::None,
            CalibrationArg::DaScalar => CalibrationMode::DaScalar,
            CalibrationArg::PpmConstant => CalibrationMode::PpmConstant,
        }
    }
}

/// CLI-facing peak assignment mode (bridges to `mod_discovery::PeakAssignmentMode`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum PeakAssignmentArg {
    /// Bins within the merge tolerance are one peak (default)
    Merge,
    /// Every prominent bin is its own peak
    Split,
}

impl From<PeakAssignmentArg> for PeakAssignmentMode {
    fn from(a: PeakAssignmentArg) -> Self {
        match a {
            PeakAssignmentArg::Merge => PeakAssignmentMode::Merge,
            PeakAssignmentArg::Split => PeakAssignmentMode::Split,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and validate Sage results TSV
    #[command(hide = true)]
    Parse {
        /// Path to results.sage.tsv
        #[arg(short, long)]
        tsv: PathBuf,

        /// Q-value threshold (default: 0.01)
        #[arg(short, long, default_value = "0.01")]
        q_threshold: f64,

        /// Only include PSMs with isotope_error == 0
        #[arg(long)]
        isotope_zero_only: bool,

        /// Show delta mass histogram
        #[arg(long)]
        histogram: bool,
    },

    /// Report which mass analyzer acquired the MS1 and MS2 scans, and the
    /// pass-1 fragment tolerance that implies. Reads only the mzML — no search,
    /// no PSMs — so it can run before the pass-1 search that it informs.
    #[command(hide = true)]
    DetectAnalyzer {
        /// Path to the mzML file (.mzML or .mzML.gz)
        #[arg(short, long, required_unless_present = "table")]
        mzml: Option<PathBuf>,

        /// Emit the census as JSON instead of a text summary
        #[arg(long)]
        json: bool,

        /// Print the CV term -> fragment tolerance table and exit
        #[arg(long)]
        table: bool,
    },

    /// Compare the two peak-assignment modes on one file (step-2 decision aid)
    #[command(hide = true)]
    ComparePeakAssignment {
        /// Path to results.sage.tsv
        #[arg(short, long)]
        tsv: PathBuf,

        /// Path to unimod.xml. Optional: a copy is compiled into this binary
        /// and is used when this is not given.
        #[arg(short, long)]
        unimod: Option<PathBuf>,

        /// Q-value threshold (default: 0.01)
        #[arg(short, long, default_value = "0.01")]
        q_threshold: f64,

        /// Minimum PSM count for a peak (default: 5, matching `analyze`)
        #[arg(long, default_value = "5")]
        min_peak_count: usize,

        /// Only report peaks whose delta mass is in this window, e.g. 0.8 1.2
        #[arg(long, num_args = 2, value_names = ["LOW", "HIGH"])]
        window: Option<Vec<f64>>,

        /// Output text file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Run modification discovery on Sage results
    #[command(hide = true)]
    Discover {
        /// Path to results.sage.tsv
        #[arg(short, long)]
        tsv: PathBuf,

        /// Path to unimod.xml. Optional: a copy is compiled into this binary
        /// and is used when this is not given.
        #[arg(short, long)]
        unimod: Option<PathBuf>,

        /// Q-value threshold (default: 0.01)
        #[arg(short, long, default_value = "0.01")]
        q_threshold: f64,

        /// Minimum PSM count for a peak (default: 10)
        #[arg(long, default_value = "10")]
        min_peak_count: usize,

        /// Maximum number of peaks to report (default: 50)
        #[arg(long, default_value = "50")]
        max_peaks: usize,

        /// Mass calibration mode for the delta-mass axis
        #[arg(long, value_enum, default_value_t = CalibrationArg::DaScalar)]
        calibration: CalibrationArg,

        /// How PSMs are grouped into peaks. Both modes assign each PSM to one
        /// peak only; they differ on whether adjacent bins are one peak or two.
        #[arg(long, value_enum, default_value_t = PeakAssignmentArg::Merge)]
        peak_assignment: PeakAssignmentArg,

        /// Output JSON file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show summary only (no JSON output)
        #[arg(long)]
        summary_only: bool,
    },

    /// Compute signal fate accounting (explained vs. unexplained signal)
    #[command(hide = true)]
    SignalFate {
        /// Path to results.sage.tsv
        #[arg(short, long)]
        tsv: PathBuf,

        /// Path to mzML file (optional, for unidentified signal calculation)
        #[arg(short, long)]
        mzml: Option<PathBuf>,

        /// Path to unimod.xml (optional, for annotation status)
        #[arg(short, long)]
        unimod: Option<PathBuf>,

        /// Q-value threshold (default: 0.01)
        #[arg(short, long, default_value = "0.01")]
        q_threshold: f64,

        /// Compute MS1 precursor intensity signal fate (requires --mzml)
        #[arg(long)]
        ms1_intensity: bool,

        /// RT window for MS1 precursor lookup in minutes (default: 1.0)
        #[arg(long, default_value = "1.0")]
        rt_window: f64,

        /// m/z tolerance for MS1 precursor lookup in ppm (default: 20.0)
        #[arg(long, default_value = "20.0")]
        mz_tol_ppm: f64,

        /// Output JSON file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show summary only (no JSON output)
        #[arg(long)]
        summary_only: bool,
    },

    /// Parse mzML file and show statistics (MS1/MS2 counts, TIC)
    #[command(hide = true)]
    MzmlStats {
        /// Path to mzML file (.mzML or .mzML.gz)
        #[arg(short, long)]
        mzml: PathBuf,

        /// Output JSON file (default: stdout summary)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Detect polymer contamination in MS1 spectra (PEG, PPG, Triton, etc.)
    #[command(hide = true)]
    PolymerStats {
        /// Path to mzML file (.mzML or .mzML.gz)
        #[arg(short, long)]
        mzml: PathBuf,

        /// m/z tolerance in ppm (default: 10.0)
        #[arg(long, default_value = "10.0")]
        tol_ppm: f64,

        /// Output JSON file (default: stdout summary)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show summary only (no JSON output)
        #[arg(long)]
        summary_only: bool,
    },

    /// Screen MS2 spectra for oxonium ions (glycopeptide detection)
    #[command(hide = true)]
    OxoniumScreen {
        /// Path to mzML file (.mzML or .mzML.gz)
        #[arg(short, long)]
        mzml: PathBuf,

        /// m/z tolerance in ppm (default: 20.0)
        #[arg(long, default_value = "20.0")]
        tol_ppm: f64,

        /// Minimum oxonium ions required (default: 2)
        #[arg(long, default_value = "2")]
        min_ions: usize,

        /// Top peak fraction to consider (default: 0.10 = 10%)
        #[arg(long, default_value = "0.10")]
        top_fraction: f64,

        /// Output JSON file (default: stdout summary)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show summary only (no JSON output)
        #[arg(long)]
        summary_only: bool,
    },

    /// Compute digestion efficiency metrics (missed cleavages, semi-tryptic)
    #[command(hide = true)]
    DigestionStats {
        /// Path to results.sage.tsv
        #[arg(short, long)]
        tsv: PathBuf,

        /// Q-value threshold (default: 0.01)
        #[arg(short, long, default_value = "0.01")]
        q_threshold: f64,

        /// Output JSON file (default: stdout summary)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show summary only (no JSON output)
        #[arg(long)]
        summary_only: bool,
    },

    /// Compute QC metrics (mass accuracy, ID rate)
    #[command(hide = true)]
    QcStats {
        /// Path to results.sage.tsv
        #[arg(short, long)]
        tsv: PathBuf,

        /// Path to mzML file (optional, for ID rate calculation)
        #[arg(short, long)]
        mzml: Option<PathBuf>,

        /// Q-value threshold (default: 0.01)
        #[arg(short, long, default_value = "0.01")]
        q_threshold: f64,

        /// Output JSON file (default: stdout summary)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show summary only (no JSON output)
        #[arg(long)]
        summary_only: bool,
    },

    /// Run full reconnaissance analysis and generate unified report
    #[command(hide = true)]
    Analyze {
        /// Path to mzML file (.mzML or .mzML.gz)
        #[arg(short, long)]
        mzml: PathBuf,

        /// Path to results.sage.tsv
        #[arg(short, long)]
        tsv: PathBuf,

        /// Path to unimod.xml. Optional: a copy is compiled into this binary
        /// and is used when this is not given.
        #[arg(short, long)]
        unimod: Option<PathBuf>,

        /// Optional path to the search FASTA. Supply the SAME database the
        /// search used. It is what makes protein-terminal modifications
        /// testable: Sage's TSV has no start-position column, so proving a
        /// peptide sits at protein position 0 needs the protein sequences.
        /// Without it those candidates are decided by abundance instead, and
        /// the report records that they were not testable.
        #[arg(long)]
        fasta: Option<PathBuf>,

        /// Q-value threshold (default: 0.01)
        #[arg(short, long, default_value = "0.01")]
        q_threshold: f64,

        /// Output base name (generates .json, .html)
        #[arg(short, long)]
        output: PathBuf,

        /// How PSMs are grouped into peaks. Recorded in the output JSON.
        #[arg(long, value_enum, default_value_t = PeakAssignmentArg::Merge)]
        peak_assignment: PeakAssignmentArg,
    },

    /// Run the full reconnaissance on one mzML file.
    ///
    /// Two Sage searches and one report: a wide open search that assumes no fixed
    /// modifications, then a semi-enzymatic Pass 2 over the identified proteins.
    /// Reports which modifications are present, where the signal went, how well
    /// the digest worked, and what mass tolerances the data supports.
    Run {
        /// The mzML file to analyse (.mzML or .mzML.gz).
        #[arg(value_name = "MZML")]
        mzml: PathBuf,

        /// The protein FASTA to search against.
        #[arg(value_name = "FASTA")]
        fasta: PathBuf,

        /// Protease used to digest the sample. REQUIRED — recon does not assume
        /// trypsin, because assuming it silently mis-reports every digestion
        /// number for any other enzyme.
        ///
        /// A preset name — trypsin, trypsin/p, arg-c, asp-n, asp-n/ambic,
        /// chymotrypsin, cnbr, glu-c, glu-c/de, lys-c, lys-c/p, lys-n, pepsin-a,
        /// trypchymo — or an explicit rule CLEAVE_AT[/RESTRICT][/n], such as
        /// "KR/P" for trypsin or "D//n" for an N-terminal cleaver. Presets are a
        /// convenience, not a ceiling: any protease can be given as a rule, and
        /// --cleave-at / --restrict / --c-terminal override single fields.
        ///
        /// Glu-C and Asp-N are buffer-dependent and ship as pairs: glu-c cleaves
        /// after E, glu-c/de after D and E in ammonium bicarbonate; asp-n cleaves
        /// before D, asp-n/ambic before D and E.
        #[arg(short, long, value_name = "ENZYME")]
        enzyme: String,

        /// Output base name; generates <NAME>.json and <NAME>.html. Defaults to
        /// the mzML basename + "_recon" in the current directory.
        #[arg(short, long, value_name = "NAME")]
        output: Option<PathBuf>,

        /// Path to unimod.xml. Optional: a copy is compiled into this binary and
        /// is used unless this points somewhere else.
        #[arg(short, long, help_heading = "Advanced")]
        unimod: Option<PathBuf>,

        /// Open-search params template. mzml_paths, database.fasta and the enzyme
        /// identity are OVERRIDDEN at runtime; everything else (tolerances,
        /// chimera, and so on) comes from this template. Defaults to the bundled
        /// open-search template.
        #[arg(long, help_heading = "Advanced")]
        params: Option<PathBuf>,

        /// Directory for Sage output (default: a temp dir next to --output).
        #[arg(long, help_heading = "Advanced")]
        search_out: Option<PathBuf>,

        /// Override the enzyme's cleavage residues, e.g. "KR".
        #[arg(long, help_heading = "Advanced")]
        cleave_at: Option<String>,

        /// Override the enzyme's restriction residues, e.g. "P". Empty means none.
        #[arg(long, help_heading = "Advanced")]
        restrict: Option<String>,

        /// Override whether cleavage is at the C-terminus of the matched residue.
        #[arg(long, help_heading = "Advanced")]
        c_terminal: Option<bool>,

        /// Q-value threshold. Changing this makes results incomparable with the
        /// pinned baselines, which are all at 0.01.
        #[arg(short, long, default_value = "0.01", help_heading = "Advanced")]
        q_threshold: f64,

        /// Pass-2 params template (semi-enzymatic subset search). Its
        /// precursor_tol, fragment_tol and database.fasta are OVERRIDDEN with what
        /// Pass 1 measured; everything else comes from the template.
        #[arg(long, help_heading = "Advanced")]
        pass2_params: Option<PathBuf>,

        /// Skip Pass 2. It roughly doubles wall-clock time, so this exists for the
        /// case where only the open-search report is wanted.
        #[arg(long, help_heading = "Advanced")]
        no_pass2: bool,
    },
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Parse {
            tsv,
            q_threshold,
            isotope_zero_only,
            histogram,
        } => {
            run_parse_command(tsv, q_threshold, isotope_zero_only, histogram)?;
        }
        Commands::DetectAnalyzer { mzml, json, table } => {
            if table {
                print_analyzer_tolerance_table();
            }
            if let Some(mzml) = mzml {
                let census = recon_tool::detect_analyzers(&mzml)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&census)?);
                } else {
                    recon_tool::print_analyzer_census(&census);
                }
            }
        }
        Commands::ComparePeakAssignment {
            tsv,
            unimod,
            q_threshold,
            min_peak_count,
            window,
            output,
        } => {
            run_compare_peak_assignment(tsv, unimod, q_threshold, min_peak_count, window, output)?;
        }
        Commands::Discover {
            tsv,
            unimod,
            q_threshold,
            min_peak_count,
            max_peaks,
            calibration,
            peak_assignment,
            output,
            summary_only,
        } => {
            run_discover_command(
                tsv,
                unimod,
                q_threshold,
                min_peak_count,
                max_peaks,
                calibration.into(),
                peak_assignment.into(),
                output,
                summary_only,
            )?;
        }
        Commands::SignalFate {
            tsv,
            mzml,
            unimod,
            q_threshold,
            ms1_intensity,
            rt_window,
            mz_tol_ppm,
            output,
            summary_only,
        } => {
            run_signal_fate_command(
                tsv,
                mzml,
                unimod,
                q_threshold,
                ms1_intensity,
                rt_window,
                mz_tol_ppm,
                output,
                summary_only,
            )?;
        }
        Commands::MzmlStats { mzml, output } => {
            run_mzml_stats_command(mzml, output)?;
        }
        Commands::PolymerStats {
            mzml,
            tol_ppm,
            output,
            summary_only,
        } => {
            run_polymer_stats_command(mzml, tol_ppm, output, summary_only)?;
        }
        Commands::OxoniumScreen {
            mzml,
            tol_ppm,
            min_ions,
            top_fraction,
            output,
            summary_only,
        } => {
            run_oxonium_screen_command(
                mzml,
                tol_ppm,
                min_ions,
                top_fraction,
                output,
                summary_only,
            )?;
        }
        Commands::DigestionStats {
            tsv,
            q_threshold,
            output,
            summary_only,
        } => {
            run_digestion_stats_command(tsv, q_threshold, output, summary_only)?;
        }
        Commands::QcStats {
            tsv,
            mzml,
            q_threshold,
            output,
            summary_only,
        } => {
            run_qc_stats_command(tsv, mzml, q_threshold, output, summary_only)?;
        }
        Commands::Analyze {
            mzml,
            tsv,
            unimod,
            fasta,
            q_threshold,
            output,
            peak_assignment,
        } => {
            run_analyze_command(
                mzml,
                tsv,
                unimod,
                fasta,
                None, // `analyze` has no --enzyme: the protease is not knowable here
                q_threshold,
                output,
                peak_assignment.into(),
            )?;
        }
        Commands::Run {
            mzml,
            fasta,
            unimod,
            output,
            params,
            search_out,
            enzyme,
            cleave_at,
            restrict,
            c_terminal,
            q_threshold,
            pass2_params,
            no_pass2,
        } => {
            run_run_command(
                mzml,
                fasta,
                unimod,
                output,
                params,
                search_out,
                enzyme,
                cleave_at,
                restrict,
                c_terminal,
                q_threshold,
                pass2_params,
                no_pass2,
            )?;
        }
    }

    Ok(())
}

fn run_parse_command(
    tsv: PathBuf,
    q_threshold: f64,
    isotope_zero_only: bool,
    histogram: bool,
) -> Result<()> {
    let options = FilterOptions {
        q_threshold,
        isotope_error_zero_only: isotope_zero_only,
        ..Default::default()
    };

    println!("Parsing: {}", tsv.display());
    println!("Q-value threshold: {}", q_threshold);
    println!("Isotope zero only: {}", isotope_zero_only);
    println!();

    let results = parse_sage_results(&tsv, &options)?;

    // Print filter stats
    println!("=== Filter Statistics ===");
    println!(
        "Total PSMs before filter: {}",
        results.filter_stats.total_before_filter
    );
    println!("Decoys removed: {}", results.filter_stats.decoys_removed);
    println!("Q-value filtered: {}", results.filter_stats.q_filtered);
    println!(
        "Isotope filtered: {}",
        results.filter_stats.isotope_filtered
    );
    println!(
        "PSMs after filter: {}",
        results.filter_stats.total_after_filter
    );
    println!();

    // Print isotope error distribution
    println!("=== Isotope Error Distribution (before filtering) ===");
    let mut isotope_vec: Vec<_> = results.isotope_error_distribution.iter().collect();
    isotope_vec.sort_by_key(|(k, _)| *k);
    for (error, count) in isotope_vec {
        let pct = 100.0 * (*count as f64) / (results.filter_stats.total_before_filter as f64);
        println!("  isotope_error {}: {} ({:.1}%)", error, count, pct);
    }
    println!();

    // Print chimera stats
    let by_scan = results.psms_by_scan();
    let chimeric_scans: usize = by_scan.values().filter(|v| v.len() > 1).count();
    println!("=== Chimera Statistics ===");
    println!("Unique scans: {}", by_scan.len());
    println!("Scans with multiple PSMs: {}", chimeric_scans);
    if !by_scan.is_empty() {
        println!(
            "Chimeric scan percentage: {:.1}%",
            100.0 * (chimeric_scans as f64) / (by_scan.len() as f64)
        );
    }
    println!();

    // Delta mass histogram
    if histogram {
        println!("=== Delta Mass Histogram (top 20, 0.01 Da bins) ===");
        let mut delta_counts: HashMap<i64, usize> = HashMap::new();
        for psm in &results.psms {
            // Use corrected delta mass, bin to 0.01 Da
            let bin = (psm.delta_mass_corrected * 100.0).round() as i64;
            *delta_counts.entry(bin).or_insert(0) += 1;
        }

        let mut counts_vec: Vec<_> = delta_counts.into_iter().collect();
        counts_vec.sort_by_key(|b| std::cmp::Reverse(b.1));

        println!("{:<15} {:<10} {:<10}", "Delta (Da)", "Count", "Pct");
        println!("{}", "-".repeat(35));
        for (bin, count) in counts_vec.iter().take(20) {
            let delta = (*bin as f64) / 100.0;
            let pct = 100.0 * (*count as f64) / (results.psms.len() as f64);
            println!("{:<15.2} {:<10} {:<10.1}%", delta, count, pct);
        }
        println!();
        println!("Unique delta bins: {}", counts_vec.len());
    }

    println!("=== Summary ===");
    println!("Total filtered PSMs: {}", results.psms.len());

    Ok(())
}

/// Run both peak-assignment modes on one file and print them side by side.
///
/// This exists to answer a step-2 decision, not to produce a deliverable: are a
/// pair of adjacent bins one population or two? Both modes satisfy the
/// exclusive-assignment invariant, so the peak COUNTS are trustworthy in both;
/// what differs is the grouping.
///
/// The composition column is a diagnostic only — see `peak_composition`. It is
/// not wired into annotation and does not change any number reported here.
/// Load Unimod: the CLI path when given, the compiled-in copy otherwise.
///
/// `unimod.xml` is embedded (`defaults::UNIMOD`), so no subcommand needs a path.
/// `run` already worked this way; `analyze`, `discover` and
/// `compare-peak-assignment` each declared a REQUIRED `--unimod` and so could
/// not run without the repo beside them. NOTES said the flag was an override
/// everywhere, which was true of `run` alone.
fn load_unimod(path: Option<PathBuf>) -> Result<UnimodDb> {
    match path {
        Some(p) => {
            println!("Loading Unimod database: {}", p.display());
            UnimodDb::from_xml(&p)
        }
        None => {
            println!(
                "Loading Unimod database: <compiled into recon {}>",
                env!("CARGO_PKG_VERSION")
            );
            UnimodDb::from_embedded()
        }
    }
}

fn run_compare_peak_assignment(
    tsv: PathBuf,
    unimod_path: Option<PathBuf>,
    q_threshold: f64,
    min_peak_count: usize,
    window: Option<Vec<f64>>,
    output: Option<PathBuf>,
) -> Result<()> {
    use recon_tool::peak_composition::site_support;
    use std::fmt::Write as _;

    let options = FilterOptions {
        q_threshold,
        ..Default::default()
    };
    let results = parse_sage_results(&tsv, &options)?;
    let unimod = load_unimod(unimod_path)?;

    let (low, high) = match window.as_deref() {
        Some([l, h]) => (*l, *h),
        _ => (f64::NEG_INFINITY, f64::INFINITY),
    };

    let mut out = String::new();
    writeln!(out, "{}", "=".repeat(96))?;
    writeln!(out, "PEAK ASSIGNMENT COMPARISON")?;
    writeln!(out, "  input          {}", tsv.display())?;
    writeln!(out, "  PSMs kept      {}", results.psms.len())?;
    writeln!(out, "  min_peak_count {min_peak_count}   q<={q_threshold}")?;
    if low.is_finite() {
        writeln!(out, "  window         [{low}, {high}] Da")?;
    }
    writeln!(out, "{}", "=".repeat(96))?;

    let mut summaries: Vec<(PeakAssignmentMode, usize, usize)> = Vec::new();

    for mode in [PeakAssignmentMode::Merge, PeakAssignmentMode::Split] {
        let config = ModDiscoveryConfig {
            min_peak_count,
            peak_assignment_mode: mode,
            ..Default::default()
        };
        let discovery = run_mod_discovery(&results, &unimod, &config);

        // Peak counts must never sum past the run. Print the numbers; do not assert
        // in prose.
        let claimed: usize = discovery.peaks.iter().map(|p| p.count).sum();

        writeln!(out)?;
        writeln!(out, "--- {mode:?} ---")?;
        writeln!(
            out,
            "  peaks {}   PSMs claimed by all peaks {} of {} total",
            discovery.peaks.len(),
            claimed,
            discovery.summary.total_psms
        )?;
        writeln!(
            out,
            "  {:>10}  {:>6}  {:>6}  {:<24}  {:>6}  {:>6}  {:>6}  {:>5}",
            "delta_Da", "count", "promin", "annotation", "sites", "in-pk", "bg", "enr"
        )?;

        for peak in discovery
            .peaks
            .iter()
            .filter(|p| p.delta_mass >= low && p.delta_mass <= high)
        {
            // Exactly the PSMs detection assigned to this peak. Not reconstructed
            // from delta_mass and a guessed tolerance — that is how two earlier
            // check scripts produced false answers.
            let peak_psms: Vec<&recon_tool::sage_results::Psm> =
                peak.psm_indices.iter().map(|&i| &results.psms[i]).collect();

            let (name, sites) = match peak.annotations.first() {
                Some(a) => (a.name.clone(), a.sites.clone()),
                None => ("(unannotated)".to_string(), vec![]),
            };
            let support = site_support(&peak_psms, &results.psms, &sites);
            let site_str: String = support.sites.iter().collect();
            // No testable residue sites (Unmodified, or a terminus-only mod) means
            // there is nothing to measure. Print "-" rather than a 0% that reads
            // like a measured absence of support.
            let testable = !support.sites.is_empty();
            let enr = match support.enrichment() {
                Some(e) if testable => format!("{e:.2}"),
                _ => "-".to_string(),
            };
            let in_pk = if testable {
                format!("{:.0}%", 100.0 * support.fraction())
            } else {
                "-".to_string()
            };
            let bg = if testable {
                format!("{:.0}%", 100.0 * support.background_fraction)
            } else {
                "-".to_string()
            };

            writeln!(
                out,
                "  {:>10.5}  {:>6}  {:>6}  {:<24}  {:>6}  {:>6}  {:>6}  {:>5}",
                peak.delta_mass,
                peak.count,
                peak.prominence,
                if name.len() > 24 {
                    name[..24].to_string()
                } else {
                    name
                },
                if site_str.is_empty() {
                    "-".to_string()
                } else {
                    site_str
                },
                in_pk,
                bg,
                enr
            )?;
        }

        summaries.push((mode, discovery.peaks.len(), claimed));
    }

    writeln!(out)?;
    writeln!(out, "{}", "=".repeat(96))?;
    for (mode, n_peaks, claimed) in &summaries {
        writeln!(
            out,
            "  {:<6?}  {:>3} peaks   {:>6} PSM claims",
            mode, n_peaks, claimed
        )?;
    }
    writeln!(
        out,
        "\n  'in-pk' is the share of the peak's PSMs whose peptide CONTAINS one of\n  \
         the listed residues. 'bg' is the same share over the whole run, and 'enr'\n  \
         is in-pk / bg. READ THE CEILING: enrichment cannot exceed 1/bg, so a\n  \
         common residue set gives a low ceiling and a weak test — N or Q occurs in\n  \
         most tryptic peptides by chance, capping enr near 1.2, while a rare\n  \
         residue like C or W discriminates strongly. Sequence membership only; it\n  \
         cannot say the mod SITS on that residue. A DIAGNOSTIC, not a gate."
    )?;

    match output {
        Some(path) => {
            std::fs::write(&path, &out)?;
            println!("Wrote {}", path.display());
        }
        None => print!("{out}"),
    }
    Ok(())
}

// `clippy::too_many_arguments`: these mirror the CLI flags one-for-one. A
// struct here would add an indirection whose only purpose is the lint, and the
// argument surface is FROZEN (see AGENTS.md), so the list cannot grow.
#[allow(clippy::too_many_arguments)]
fn run_discover_command(
    tsv: PathBuf,
    unimod_path: Option<PathBuf>,
    q_threshold: f64,
    min_peak_count: usize,
    max_peaks: usize,
    calibration_mode: CalibrationMode,
    peak_assignment_mode: PeakAssignmentMode,
    output: Option<PathBuf>,
    summary_only: bool,
) -> Result<()> {
    // Parse Sage results
    println!("Loading Sage results: {}", tsv.display());
    let options = FilterOptions {
        q_threshold,
        ..Default::default()
    };
    let results = parse_sage_results(&tsv, &options)?;
    println!(
        "  Loaded {} PSMs (q <= {})",
        results.psms.len(),
        q_threshold
    );

    let unimod = load_unimod(unimod_path)?;
    println!("  Loaded {} modifications", unimod.len());

    // Configure mod discovery
    let config = ModDiscoveryConfig {
        min_peak_count,
        max_peaks,
        calibration_mode,
        peak_assignment_mode,
        ..Default::default()
    };

    // Run mod discovery
    println!("Running modification discovery...");
    let discovery = run_mod_discovery(&results, &unimod, &config);

    // Print summary
    println!();
    println!("=== Modification Discovery Summary ===");
    println!("Total PSMs: {}", discovery.summary.total_psms);
    println!(
        "Unmodified (delta ~0): {} ({:.1}%)",
        discovery.summary.psms_near_zero, discovery.summary.psms_near_zero_pct
    );
    println!(
        "Modified: {} ({:.1}%)",
        discovery.summary.psms_modified, discovery.summary.psms_modified_pct
    );
    println!("Unique histogram bins: {}", discovery.summary.unique_bins);
    println!("Peaks detected: {}", discovery.peaks.len());

    // Calibration diagnostics (benchmark-facing: mode, fitted offset, where zero landed)
    {
        let c = &discovery.calibration;
        println!();
        println!("=== Calibration ===");
        println!("Mode: {:?} (applied: {})", c.mode, c.applied);
        match c.mode {
            CalibrationMode::DaScalar => {
                println!("apex_offset: {:.4} mDa", c.apex_offset_da * 1000.0);
            }
            CalibrationMode::PpmConstant => {
                if let Some(ppm) = c.ppm_offset {
                    println!("ppm_offset: {:.3} ppm", ppm);
                }
            }
            CalibrationMode::None => {}
        }
        println!(
            "Fit population (near-zero): {} PSMs",
            c.calibration_psm_count
        );
        println!(
            "Post-calibration zero center: {:.4} mDa",
            c.post_calibration_zero_center_da * 1000.0
        );
        if let Some(w) = &c.warning {
            println!("Warning: {}", w);
        }
    }
    println!();

    // Print top peaks
    println!("=== Top Peaks ===");
    println!(
        "{:<6} {:<12} {:<10} {:<10} {:<30} {:<10}",
        "Rank", "Delta (Da)", "Count", "Pct", "Annotation", "Ambiguous"
    );
    println!("{}", "-".repeat(80));

    for peak in discovery.peaks.iter().take(20) {
        let annotation = if peak.unannotated {
            "UNANNOTATED".to_string()
        } else if let Some(ann) = peak.annotations.first() {
            ann.name.clone()
        } else {
            "?".to_string()
        };

        let ambig = if peak.ambiguous { "Yes" } else { "No" };

        println!(
            "{:<6} {:<12.4} {:<10} {:<10.1} {:<30} {:<10}",
            peak.rank, peak.delta_mass, peak.count, peak.count_pct, annotation, ambig
        );
    }
    println!();

    // Print confidence comparison
    println!("=== Confidence Metrics (Top 10 Peaks) ===");
    println!(
        "{:<6} {:<12} {:<12} {:<12} {:<12} {:<12}",
        "Rank", "Delta", "Hyperscore", "Match%", "LongestB", "LongestY"
    );
    println!("{}", "-".repeat(72));

    for peak in discovery.peaks.iter().take(10) {
        println!(
            "{:<6} {:<12.4} {:<12.1} {:<12.2} {:<12.1} {:<12.1}",
            peak.rank,
            peak.delta_mass,
            peak.confidence.mean_hyperscore,
            // Already a percent. Sage scales it (scoring.rs); scaling again
            // printed 3436.66 for a stored 34.37.
            peak.confidence.mean_matched_intensity_pct,
            peak.confidence.mean_longest_b,
            peak.confidence.mean_longest_y
        );
    }
    println!();

    // Output JSON if requested
    if !summary_only {
        let json = serde_json::to_string_pretty(&discovery)?;

        if let Some(output_path) = output {
            std::fs::write(&output_path, &json)?;
            println!("Results written to: {}", output_path.display());
        } else {
            println!("=== JSON Output ===");
            println!("{}", json);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)] // see run_discover_command
fn run_signal_fate_command(
    tsv: PathBuf,
    mzml_path: Option<PathBuf>,
    unimod_path: Option<PathBuf>,
    q_threshold: f64,
    ms1_intensity: bool,
    rt_window: f64,
    mz_tol_ppm: f64,
    output: Option<PathBuf>,
    summary_only: bool,
) -> Result<()> {
    use recon_tool::mzml::{build_precursor_queries_from_psms, extract_precursor_intensities};
    use recon_tool::signal_fate::Ms1SignalFate;

    // Parse Sage results
    println!("Loading Sage results: {}", tsv.display());
    let options = FilterOptions {
        q_threshold,
        ..Default::default()
    };
    let results = parse_sage_results(&tsv, &options)?;
    println!(
        "  Loaded {} PSMs (q <= {})",
        results.psms.len(),
        q_threshold
    );

    // Track mzML path for MS1 extraction
    let mzml_file_path = mzml_path.clone();

    // Optionally load mzML stats for unidentified signal calculation
    let mzml_stats = if let Some(ref mzml_file) = mzml_path {
        println!("Loading mzML file: {}", mzml_file.display());
        let stats = get_mzml_stats(mzml_file)?;
        println!("  MS2 spectra: {}", stats.ms2_spectra);
        println!("  Total MS2 TIC: {:.2e}", stats.total_ms2_tic);
        Some(stats)
    } else {
        None
    };

    // Optionally load Unimod and run mod discovery for annotation status
    let mod_discovery = if let Some(unimod_file) = unimod_path {
        println!("Loading Unimod database: {}", unimod_file.display());
        let unimod = UnimodDb::from_xml(&unimod_file)?;
        println!("  Loaded {} modifications", unimod.len());

        println!("Running modification discovery for annotation status...");
        let config = ModDiscoveryConfig::default();
        Some(run_mod_discovery(&results, &unimod, &config))
    } else {
        println!("  (No Unimod provided - all modified PSMs will be marked unannotated)");
        None
    };

    // Compute signal fate (with or without mzML stats)
    println!("Computing signal fate accounting...");
    let mut fate = if let Some(ref stats) = mzml_stats {
        compute_signal_fate_with_mzml(&results, mod_discovery.as_ref(), stats)
    } else {
        compute_signal_fate(&results, mod_discovery.as_ref())
    };

    // Compute MS1 signal fate if requested
    if ms1_intensity {
        if let Some(ref mzml_file) = mzml_file_path {
            println!();
            println!("Computing MS1 precursor intensity signal fate...");
            println!("  RT window: ±{:.1} min", rt_window);
            println!("  m/z tolerance: {} ppm", mz_tol_ppm);

            // Extract MS1 spectra
            let ms1_spectra = extract_ms1_spectra(mzml_file)?;
            println!("  Extracted {} MS1 spectra", ms1_spectra.len());

            // Compute total MS1 TIC
            let total_ms1_tic: f64 = ms1_spectra.iter().map(|s| s.tic).sum();
            println!("  Total MS1 TIC: {:.2e}", total_ms1_tic);

            // Build precursor queries from PSMs
            let queries = build_precursor_queries_from_psms(&results.psms);
            println!("  Built {} precursor queries", queries.len());

            // Extract precursor intensities
            let intensity_results =
                extract_precursor_intensities(&ms1_spectra, &queries, rt_window, mz_tol_ppm);

            // Compute MS1 signal fate
            let explained_intensity: f64 = intensity_results.iter().map(|r| r.intensity).sum();
            let psms_with_ms1 = intensity_results
                .iter()
                .filter(|r| r.intensity > 0.0)
                .count();
            let psms_without_ms1 = intensity_results.len() - psms_with_ms1;

            let explained_pct = if total_ms1_tic > 0.0 {
                100.0 * explained_intensity / total_ms1_tic
            } else {
                0.0
            };

            let unexplained_intensity = if total_ms1_tic > explained_intensity {
                total_ms1_tic - explained_intensity
            } else {
                0.0
            };

            let unexplained_pct = if total_ms1_tic > 0.0 {
                100.0 * unexplained_intensity / total_ms1_tic
            } else {
                0.0
            };

            fate.ms1_signal_fate = Some(Ms1SignalFate {
                total_ms1_tic,
                explained_intensity,
                explained_pct,
                unexplained_intensity,
                unexplained_pct,
                psms_with_ms1,
                psms_without_ms1,
            });

            println!(
                "  PSMs with MS1 signal: {} / {}",
                psms_with_ms1,
                queries.len()
            );

            // Also compute improved MS1 signal fate with all enhancements
            println!();
            println!("Computing improved MS1 signal fate (isotope envelope, deduplicated)...");
            let improved_fate = mzml::compute_improved_ms1_signal_fate(
                &ms1_spectra,
                &queries,
                3,    // ±3 MS1 scans
                10.0, // 10 ppm tolerance
            );
            mzml::print_improved_ms1_signal_fate(&improved_fate);
        } else {
            println!();
            println!("Warning: --ms1-intensity requires --mzml to be specified");
        }
    }

    // Print summary
    println!();
    print_signal_fate_summary(&fate);

    // Output JSON if requested
    if !summary_only {
        let json = serde_json::to_string_pretty(&fate)?;

        if let Some(output_path) = output {
            std::fs::write(&output_path, &json)?;
            println!();
            println!("Results written to: {}", output_path.display());
        } else {
            println!();
            println!("=== JSON Output ===");
            println!("{}", json);
        }
    }

    Ok(())
}

fn run_mzml_stats_command(mzml_path: PathBuf, output: Option<PathBuf>) -> Result<()> {
    println!("Parsing mzML file: {}", mzml_path.display());
    let start = std::time::Instant::now();

    let stats = get_mzml_stats(&mzml_path)?;

    let elapsed = start.elapsed();
    println!("  Parsed in {:.2}s", elapsed.as_secs_f64());
    println!();

    // Print summary
    print_mzml_stats(&stats);

    // Output JSON if requested
    if let Some(output_path) = output {
        let json = serde_json::to_string_pretty(&stats)?;
        std::fs::write(&output_path, &json)?;
        println!();
        println!("Results written to: {}", output_path.display());
    }

    Ok(())
}

fn run_polymer_stats_command(
    mzml_path: PathBuf,
    tol_ppm: f64,
    output: Option<PathBuf>,
    summary_only: bool,
) -> Result<()> {
    println!("Extracting MS1 spectra from: {}", mzml_path.display());
    let start = std::time::Instant::now();

    let ms1_spectra = extract_ms1_spectra(&mzml_path)?;
    let max_mz = get_max_mz(&ms1_spectra);

    println!("  Extracted {} MS1 spectra", ms1_spectra.len());
    println!("  Max m/z: {:.1}", max_mz);
    println!("  Tolerance: {} ppm", tol_ppm);
    println!();

    println!("Searching for polymer contamination...");
    let spectra_iter = ms1_spectra
        .iter()
        .map(|s| (s.rt, s.tic, s.mz.as_slice(), s.intensity.as_slice()));

    let results = search_polymers(spectra_iter, max_mz, tol_ppm);

    let elapsed = start.elapsed();
    println!("  Completed in {:.2}s", elapsed.as_secs_f64());
    println!();

    println!("=== Polymer Contamination Summary ===");
    println!();
    println!("Total MS1 TIC: {:.2e}", results.total_tic);
    println!(
        "Total polymer %TIC: {:.2}%",
        results.total_polymer_pct_tic()
    );
    println!();

    println!("Polymer Detection (sorted by %TIC):");
    println!("{:<35} {:>12} {:>10}", "Polymer", "Intensity", "%TIC");
    println!("{}", "-".repeat(60));

    for (name, pct) in results.polymers_by_pct_tic() {
        if pct > 0.0 {
            let poly = results.polymers.iter().find(|p| p.name == name).unwrap();
            println!(
                "{:<35} {:>12.2e} {:>10.4}%",
                name, poly.total_intensity, pct
            );
        }
    }

    let detected_count = results
        .polymers
        .iter()
        .filter(|p| p.total_intensity > 0.0)
        .count();
    println!();
    println!(
        "Polymers detected: {} / {}",
        detected_count,
        results.polymers.len()
    );

    let total_pct = results.total_polymer_pct_tic();
    let level = if total_pct < 0.1 {
        "Low (< 0.1%)"
    } else if total_pct < 1.0 {
        "Moderate (0.1-1%)"
    } else if total_pct < 5.0 {
        "High (1-5%)"
    } else {
        "Very High (> 5%)"
    };
    println!("Contamination level: {}", level);

    if !summary_only {
        let json_output = serde_json::json!({
            "total_tic": results.total_tic,
            "total_polymer_pct_tic": results.total_polymer_pct_tic(),
            "contamination_level": level,
            "polymers": results.polymers_by_pct_tic().into_iter()
                .filter(|(_, pct)| *pct > 0.0)
                .map(|(name, pct)| {
                    let poly = results.polymers.iter().find(|p| p.name == name).unwrap();
                    serde_json::json!({ "name": name, "total_intensity": poly.total_intensity, "pct_tic": pct })
                })
                .collect::<Vec<_>>(),
        });
        let json = serde_json::to_string_pretty(&json_output)?;
        if let Some(output_path) = output {
            std::fs::write(&output_path, &json)?;
            println!("\nResults written to: {}", output_path.display());
        } else {
            println!("\n=== JSON Output ===\n{}", json);
        }
    }
    Ok(())
}

fn run_oxonium_screen_command(
    mzml_path: PathBuf,
    tol_ppm: f64,
    min_ions: usize,
    top_fraction: f64,
    output: Option<PathBuf>,
    summary_only: bool,
) -> Result<()> {
    println!("Extracting MS2 spectra from: {}", mzml_path.display());
    let start = std::time::Instant::now();

    let ms2_spectra = extract_ms2_spectra(&mzml_path)?;

    println!("  Extracted {} MS2 spectra", ms2_spectra.len());
    println!("  Tolerance: {} ppm", tol_ppm);
    println!("  Min oxonium ions: {}", min_ions);
    println!("  Top peak fraction: {:.0}%", top_fraction * 100.0);
    println!();

    let config = OxoniumScreeningConfig {
        mz_tolerance_ppm: tol_ppm,
        min_oxonium_ions: min_ions,
        top_peak_fraction: top_fraction,
        require_mandatory: true,
    };

    println!("Screening for oxonium ions...");
    let results = screen_spectra(&ms2_spectra, &config);
    let summary = compute_screening_summary(&results);

    let elapsed = start.elapsed();
    println!("  Completed in {:.2}s", elapsed.as_secs_f64());
    println!();

    print_screening_summary(&summary);

    if !summary_only {
        let json = serde_json::to_string_pretty(&summary)?;
        if let Some(output_path) = output {
            std::fs::write(&output_path, &json)?;
            println!("\nResults written to: {}", output_path.display());
        } else {
            println!("\n=== JSON Output ===\n{}", json);
        }
    }
    Ok(())
}

fn run_digestion_stats_command(
    tsv: PathBuf,
    q_threshold: f64,
    output: Option<PathBuf>,
    summary_only: bool,
) -> Result<()> {
    println!("Loading Sage results: {}", tsv.display());
    let options = FilterOptions {
        q_threshold,
        ..Default::default()
    };
    let results = parse_sage_results(&tsv, &options)?;
    println!(
        "  Loaded {} PSMs (q <= {})",
        results.psms.len(),
        q_threshold
    );
    println!();

    println!("Computing digestion efficiency metrics...");
    let digestion = compute_digestion_stats(&results);

    // Print summary
    print_digestion_summary(&digestion);

    // Output JSON if requested
    if !summary_only {
        let json = serde_json::to_string_pretty(&digestion)?;
        if let Some(output_path) = output {
            std::fs::write(&output_path, &json)?;
            println!("\nResults written to: {}", output_path.display());
        } else {
            println!("\n=== JSON Output ===\n{}", json);
        }
    }
    Ok(())
}

fn run_qc_stats_command(
    tsv: PathBuf,
    mzml_path: Option<PathBuf>,
    q_threshold: f64,
    output: Option<PathBuf>,
    summary_only: bool,
) -> Result<()> {
    println!("Loading Sage results: {}", tsv.display());
    // Capture input paths as strings for the provenance envelope before the
    // Option values get moved/borrowed below.
    let tsv_path_str = tsv.display().to_string();
    let mzml_path_str = mzml_path.as_ref().map(|p| p.display().to_string());
    let options = FilterOptions {
        q_threshold,
        ..Default::default()
    };
    let results = parse_sage_results(&tsv, &options)?;
    println!(
        "  Loaded {} PSMs (q <= {})",
        results.psms.len(),
        q_threshold
    );

    // Optionally compute signal fate for ID rate
    let signal_fate = if let Some(mzml_file) = mzml_path {
        println!("Loading mzML file: {}", mzml_file.display());
        let stats = get_mzml_stats(&mzml_file)?;
        println!("  MS2 spectra: {}", stats.ms2_spectra);
        let fate = compute_signal_fate_with_mzml(&results, None, &stats);
        Some(fate)
    } else {
        None
    };

    println!("Computing QC metrics...");
    let qc = compute_qc_stats(&results, signal_fate.as_ref());

    // Print summary
    print_qc_summary(&qc);

    // Output JSON if requested
    if !summary_only {
        // Provenance envelope: only qc-stats carries it. The comment here used to
        // say "analyze + qc-stats"; that is false. `Provenance` is built in this
        // function alone, and no committed report JSON has a `provenance` key.
        // The analyze and run reports carry flat tool_version and git_commit
        // fields instead. Discover and the snapshots carry neither, on purpose,
        // so their output stays byte-stable for regression comparison.
        let mut inputs: Vec<(&str, &str)> = vec![("tsv", tsv_path_str.as_str())];
        if let Some(ref m) = mzml_path_str {
            inputs.push(("mzml", m.as_str()));
        }
        let provenance =
            recon_tool::provenance::Provenance::new(chrono::Utc::now().to_rfc3339(), inputs);
        // Attach MS1 accuracy + provenance alongside the QC struct for the JSON view.
        let json_out = serde_json::json!({
            "provenance": provenance,
            "qc": qc,
        });
        let json = serde_json::to_string_pretty(&json_out)?;
        if let Some(output_path) = output {
            std::fs::write(&output_path, &json)?;
            println!("\nResults written to: {}", output_path.display());
        } else {
            println!("\n=== JSON Output ===\n{}", json);
        }
    }
    Ok(())
}

/// Write `<base>.json` and `<base>.html`.
///
/// Factored out so `run` can call it TWICE: once before Pass 2, which consumes
/// the report, and again afterwards to fill in the total runtime. Without the
/// second write the header could only ever show the time to first output, which
/// is not what a user means by how long the tool took.
///
/// `pass2` supplies the HTML report's Digestion section, whose numbers live ONLY
/// in `<base>_pass2.json` and are deliberately not folded into `ReconReport`
/// (see `report::Pass2Report`). It is therefore `None` on the FIRST write of a
/// run, on every `recon analyze`, and on `recon run --no-pass2`. The JSON is not
/// affected either way.
fn write_report_files(
    report: &ReconReport,
    pass2: Option<&recon_tool::report::Pass2Report>,
    output: &std::path::Path,
    announce: bool,
) -> Result<()> {
    let base = output.display().to_string();
    let json_path = format!("{base}.json");
    std::fs::write(&json_path, serde_json::to_string_pretty(report)?)?;
    let html_path = format!("{base}.html");
    std::fs::write(&html_path, generate_html_report(report, pass2))?;
    if announce {
        println!("JSON report written to: {json_path}");
        println!("HTML report written to: {html_path}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // see run_discover_command
fn run_analyze_command(
    mzml_path: PathBuf,
    tsv: PathBuf,
    unimod_path: Option<PathBuf>,
    fasta: Option<PathBuf>,
    // The protease, for the report's `input.enzyme` block. `recon analyze` has no
    // `--enzyme` and passes `None`; `recon run` passes the enzyme it resolved.
    // Analysis uses it nowhere — `analyze` classifies termini from Sage's own
    // columns — so this is a record, not a new input to a measurement.
    enzyme: Option<&recon_tool::enzyme::Enzyme>,
    q_threshold: f64,
    output: PathBuf,
    peak_assignment_mode: PeakAssignmentMode,
) -> Result<recon_tool::report::ReconReport> {
    use recon_tool::sage_results::distinct_filenames;
    use std::path::Path;

    let start = std::time::Instant::now();

    println!("================================================================================");
    println!("                    PROTEOMICS RECONNAISSANCE ANALYSIS");
    println!("================================================================================");
    println!();

    // --- Same-file provenance guard (three checks, hard error) ---------------
    // Runs FIRST, before the expensive mzML/TSV parse, so a mismatched-file
    // scripting error fails in milliseconds instead of after a full load. It
    // proves the open TSV, the --mzml argument, and the closed TSV all refer to
    // the SAME raw file by BASENAME (Sage's `filename` column) — a scripting-error
    // guard, NOT byte-identity. The one-report-per-file lock forbids blending two
    // different raw files' mass errors into one report; this enforces it.
    //
    // Basename of the raw file this report is about (the --mzml argument).
    let mzml_basename = Path::new(&mzml_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // Check 3 always applies (guards the report's OWN inputs): the report is
    // built from the open TSV; --mzml must be one of the files that open TSV
    // actually contains, else the report would be built on one file's PSMs while
    // labelled another. The open TSV MAY be multi-file — analyze does NOT
    // currently restrict PSMs to --mzml (see NOTES: multi-file open TSV is a
    // latent mixing bug tracked separately), so this check is the only thing
    // standing between a mislabelled --mzml and a wrong report.
    let open_files = distinct_filenames(&tsv).map_err(|e| {
        anyhow::anyhow!("reading filename column from open TSV for provenance guard: {e}")
    })?;
    if !open_files.iter().any(|f| f == &mzml_basename) {
        anyhow::bail!(
            "provenance guard (open TSV): --mzml basename '{}' is not present in the open TSV's filename set {:?}. \
             The report would be built on a different file's PSMs than it is labelled with. \
             (basename-level check — catches a mismatched-file scripting error, not byte-identity)",
            mzml_basename, open_files
        );
    }
    if open_files.len() > 1 {
        // Not fatal on its own, but the caller should know the report is drawn
        // from a multi-file open TSV that analyze does not restrict to --mzml.
        eprintln!(
            "WARNING: open TSV spans {} raw files {:?}; analyze does NOT restrict PSMs to --mzml '{}'. \
             Mod-discovery/signal-fate numbers mix all {} runs. See NOTES (multi-file open TSV).",
            open_files.len(), open_files, mzml_basename, open_files.len()
        );
    }

    // Step 1: Load mzML file
    println!("[1/8] Loading mzML file: {}", mzml_path.display());
    let mzml_stats = get_mzml_stats(&mzml_path)?;
    let ms1_spectra = extract_ms1_spectra(&mzml_path)?;
    let ms2_spectra = extract_ms2_spectra(&mzml_path)?;
    println!(
        "       MS1 spectra: {}, MS2 spectra: {}",
        mzml_stats.ms1_spectra, mzml_stats.ms2_spectra
    );

    // Step 2: Load Sage results
    println!("[2/8] Loading Sage results: {}", tsv.display());
    let options = FilterOptions {
        q_threshold,
        ..Default::default()
    };
    let results = parse_sage_results(&tsv, &options)?;
    println!(
        "       Loaded {} PSMs (q <= {})",
        results.psms.len(),
        q_threshold
    );

    // Step 3: Load Unimod
    let unimod_label = match unimod_path {
        Some(ref p) => p.display().to_string(),
        None => format!("<compiled into recon {}>", env!("CARGO_PKG_VERSION")),
    };
    println!("[3/8] Loading Unimod database: {unimod_label}");
    // A path if the user gave one, otherwise the copy compiled into the binary.
    // Both go through the same parser, so the two routes cannot drift.
    let unimod = match unimod_path {
        Some(ref p) => UnimodDb::from_xml(p)?,
        None => UnimodDb::from_embedded()?,
    };
    println!("       Loaded {} modifications", unimod.len());

    // Step 4: Run mod discovery
    println!("[4/8] Running modification discovery...");
    let config = ModDiscoveryConfig {
        peak_assignment_mode,
        ..Default::default()
    };
    let mod_discovery = run_mod_discovery(&results, &unimod, &config);
    println!("       Found {} peaks", mod_discovery.peaks.len());

    // Step 5: Compute signal fate
    println!("[5/8] Computing signal fate...");
    let signal_fate = compute_signal_fate_with_mzml(&results, Some(&mod_discovery), &mzml_stats);
    println!(
        "       ID rate: {:.1}% by count, {:.1}% by TIC",
        signal_fate.by_count.identified_pct.unwrap_or(0.0),
        signal_fate.by_intensity.identified_pct.unwrap_or(0.0)
    );

    // Step 6: Polymer detection
    println!("[6/8] Detecting polymer contamination...");
    let max_mz = get_max_mz(&ms1_spectra);
    let spectra_iter = ms1_spectra
        .iter()
        .map(|s| (s.rt, s.tic, s.mz.as_slice(), s.intensity.as_slice()));
    let polymer = search_polymers(spectra_iter, max_mz, 10.0);
    println!(
        "       Polymer %TIC: {:.2}%",
        polymer.total_polymer_pct_tic()
    );

    // Step 7: Oxonium screening
    println!("[7/8] Screening for glycopeptides (oxonium ions)...");
    let oxonium_config = OxoniumScreeningConfig::default();
    let oxonium_results = screen_spectra(&ms2_spectra, &oxonium_config);
    let oxonium = compute_screening_summary(&oxonium_results);
    println!(
        "       Glycopeptide candidates: {} ({:.1}%)",
        oxonium.glycopeptide_candidates, oxonium.glycopeptide_pct
    );

    // Step 8: Digestion and QC
    println!("[8/8] Computing digestion and QC metrics...");
    let digestion = compute_digestion_stats(&results);
    let qc = compute_qc_stats(&results, Some(&signal_fate));
    println!(
        "       Missed cleavage 0: {:.1}%, Semi-tryptic: {:.1}%",
        digestion
            .missed_cleavages
            .distribution
            .iter()
            .find(|m| m.missed == 0)
            .map(|m| m.pct)
            .unwrap_or(0.0),
        digestion.semi_enzymatic.semi_enzymatic_pct
    );

    // Compute alkylation check
    let alkylation = compute_alkylation_check(&results.psms);

    // Self-calibrated MS1/MS2 tolerance recommendation, measured from THIS
    // open search's own clean subset (near-zero-delta, rank-1, target,
    // q<0.01 PSMs — NOTES "MS1 error from the wide search's clean subset",
    // locked). This works on every open-search
    // report, not only when a closed reference search was also supplied.
    println!();
    println!("[CAL] Computing self-calibrated MS1/MS2 tolerance from open-search clean subset...");
    let ms1_calibration = {
        const DELTA_DA_THRESHOLD: f64 = 0.02;
        let psm_summaries: Vec<recon_tool::PsmSummary> = results
            .psms
            .iter()
            .map(recon_tool::PsmSummary::from_psm)
            .collect();
        // Pre-guard pass (no hyperscore filter) to learn the un-truncated
        // subset size, needed to know — via the SAME eligibility function
        // select_clean_subset uses internally — whether the guard fires,
        // without duplicating its threshold logic.
        let pre_guard_subset = recon_tool::select_clean_subset(
            &psm_summaries,
            DELTA_DA_THRESHOLD,
            q_threshold,
            false,
            mzml_stats.ms2_spectra,
        );
        let hyperscore_guard_applied = recon_tool::hyperscore_guard_would_apply(
            pre_guard_subset.len(),
            mzml_stats.ms2_spectra,
            psm_summaries.len(),
        );
        let clean_subset = recon_tool::select_clean_subset(
            &psm_summaries,
            DELTA_DA_THRESHOLD,
            q_threshold,
            true, // use_hyperscore_guard
            mzml_stats.ms2_spectra,
        );
        match recon_tool::compute_ms1_stats(&clean_subset) {
            Some(stats) => {
                let user_rec = recon_tool::ms1_user_recommendation(&stats);
                let pass2 = recon_tool::ms1_pass2_window(&stats);
                let fragment_ppm_values: Vec<f64> =
                    clean_subset.iter().map(|p| p.fragment_ppm).collect();
                let ms2_tol = recon_tool::compute_ms2_tolerance(&fragment_ppm_values);
                println!(
                    "       Clean subset: {} PSMs, bias {:+.2} ppm, MAD {:.2} ppm{}",
                    stats.n_psms,
                    stats.bias_ppm,
                    stats.mad_ppm,
                    if hyperscore_guard_applied {
                        " (hyperscore-guarded)"
                    } else {
                        ""
                    }
                );
                println!(
                    "       User recommendation: ±{:.0} ppm (quantized from |bias|+5×MAD = {:.2}) | Pass 2 window (±100 ppm cap): {:+.2} to {:+.2} ppm",
                    user_rec.recommended_tolerance_ppm,
                    stats.bias_ppm.abs() + 5.0 * stats.mad_ppm,
                    pass2.low_ppm, pass2.high_ppm
                );
                if let Some(ref t) = ms2_tol {
                    println!(
                        "       MS2 bias: {:+.2} ppm, MAD {:.2} ppm (from Sage fragment_ppm)",
                        t.median_ppm, t.mad_ppm
                    );
                }
                Some(recon_tool::report::Ms1CalibrationReport {
                    clean_subset_n_psms: stats.n_psms,
                    hyperscore_guard_applied,
                    bias_ppm: stats.bias_ppm,
                    spread_mad_ppm: stats.mad_ppm,
                    user_recommendation_tolerance_ppm: Some(user_rec.recommended_tolerance_ppm),
                    user_recommendation_requirement_ppm: Some(user_rec.measured_requirement_ppm),
                    user_recommendation_exceeds_ladder: Some(user_rec.requirement_exceeds_ladder),
                    user_recommendation_low_ppm: user_rec.low_ppm,
                    user_recommendation_high_ppm: user_rec.high_ppm,
                    pass2_window_low_ppm: pass2.low_ppm,
                    pass2_window_high_ppm: pass2.high_ppm,
                    ms2_tolerance_low_ppm: ms2_tol.as_ref().map(|t| t.low_ppm),
                    ms2_tolerance_high_ppm: ms2_tol.as_ref().map(|t| t.high_ppm),
                    ms2_median_abs_ppm: ms2_tol.as_ref().map(|t| t.median_ppm),
                    ms2_spread_mad_ppm: ms2_tol.as_ref().map(|t| t.mad_ppm),
                })
            }
            None => {
                eprintln!("       WARNING: no qualifying near-zero rank-1 target PSMs — self-calibrated MS1/MS2 recommendation unavailable");
                None
            }
        }
    };

    // Step-2 search-parameter recommendation.
    //
    // Candidate names and acceptor sites come from the curated mod list, not from
    // all of Unimod; Unimod supplies only the element masses. If the list is not
    // found the block is omitted rather than silently falling back to Unimod --
    // a missing curated list is a configuration problem, and a report built on the
    // wrong candidate set would reintroduce the +57 ambiguity without saying so.
    // The protein-sequence context, when a FASTA was supplied. Loaded BEFORE the
    // curated list so a bad FASTA fails before any of the expensive work below.
    //
    // THE GUARD IS THE POINT. A mismatched FASTA resolves no accessions, every
    // protein-terminal candidate then fails its acceptor test, and the report reads
    // "no protein N-terminal modifications present" — a confident wrong answer with
    // no symptom at all. Hard-stop instead, and print the actual fraction.
    let protein_index: Option<recon_tool::protein_index::ProteinIndex> = match &fasta {
        Some(path) => {
            println!();
            println!("[FASTA] Loading protein context: {}", path.display());
            let ix = recon_tool::protein_index::ProteinIndex::from_fasta(path)?;
            println!("        {} sequences indexed", ix.len());
            Some(ix)
        }
        None => {
            println!();
            println!("[FASTA] No --fasta given: protein-terminal modifications are NOT TESTABLE");
            println!("        and will be decided by abundance. This is recorded in the report.");
            None
        }
    };

    let recommendations = {
        // Compiled in, NOT read from disk. `defaults::CURATED_MODS` was bundled
        // and unit-tested, but this call site was never switched over, so it
        // still read `_dev/reference-notes/metaMorpheusMods/` relative to the WORKING
        // DIRECTORY. That resolves in a repo checkout and nowhere else, so every
        // released binary printed "recommendations omitted" and produced a report
        // with no tier recommendations at all. Found 2026-09-02 by the first
        // release smoke test. See _dev/testing/release-smoke/v0.1.0-apple-silicon.md.
        match CuratedDb::load_from_sources(recon_tool::defaults::CURATED_MODS, unimod.elements()) {
            Ok((curated, skipped)) => {
                if skipped > 0 {
                    println!(
                        "  Curated list: {} entries skipped (unknown element)",
                        skipped
                    );
                }
                // The statistical background is rank-1 targets at spectrum_q < 0.01,
                // the basis NOTES standardises delta-band populations on.
                //
                // Re-parsed rather than reusing `results.psms`, which the pipeline
                // has ALREADY filtered at peptide_q <= 0.01. Filtering that by
                // spectrum_q would give the INTERSECTION of the two criteria, not
                // the spectrum_q population -- 21736 rather than 22298 on b1906, a
                // 2.5% difference that silently deviates from both the stated basis
                // and the Python prototype the port is pinned against.
                let bg_opts = FilterOptions {
                    q_threshold: 1.0,
                    ..Default::default()
                };
                let background: Vec<_> = match parse_sage_results(&tsv, &bg_opts) {
                    Ok(all) => all
                        .psms
                        .into_iter()
                        .filter(|p| p.rank == 1 && p.spectrum_q < 0.01)
                        .collect(),
                    Err(e) => {
                        log::warn!("background re-parse failed, recommendations omitted: {e:#}");
                        Vec::new()
                    }
                };
                let peaks: Vec<(f64, usize)> = mod_discovery
                    .peaks
                    .iter()
                    .filter(|p| p.delta_mass.abs() >= NEAR_ZERO_THRESHOLD_DA)
                    .map(|p| (p.delta_mass, p.count))
                    .collect();
                let top = peaks.iter().map(|(_, c)| *c).max().unwrap_or(0) as f64;
                let floor = top * FLOOR_PCT_OF_TOP / 100.0;
                // Resolution guard + protein_context block. Runs on the SAME
                // background population the statistics use, so the fraction that
                // is reported is the fraction the test actually saw.
                let protein_context = match &protein_index {
                    Some(ix) => {
                        let (resolved, total) = ix.resolution(&background);
                        let frac = if total == 0 {
                            0.0
                        } else {
                            resolved as f64 / total as f64
                        };
                        println!(
                            "  Protein context: {}/{} target PSMs resolved ({:.2}%), \
                             {} sequences",
                            resolved,
                            total,
                            100.0 * frac,
                            ix.len()
                        );
                        if frac < recon_tool::protein_index::MIN_RESOLVED_FRACTION {
                            anyhow::bail!(
                                "FASTA does not match this search: only {}/{} target PSMs \
                                 ({:.2}%) resolve an accession in {}, below the {:.0}% floor. \
                                 Supply the SAME database the search used — a mismatched \
                                 FASTA silently reports 'no protein N-terminal modifications'.",
                                resolved,
                                total,
                                100.0 * frac,
                                fasta
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_default(),
                                100.0 * recon_tool::protein_index::MIN_RESOLVED_FRACTION
                            );
                        }
                        let nterm = background
                            .iter()
                            .filter(|p| {
                                !p.is_decoy
                                    && ix.starts_protein(
                                        &recon_tool::peak_composition::residues_of(&p.peptide),
                                        &p.proteins,
                                    )
                            })
                            .count();
                        println!(
                            "  Protein N-terminal PSMs: {} ({:.3}% of background)",
                            nterm,
                            if total == 0 {
                                0.0
                            } else {
                                100.0 * nterm as f64 / total as f64
                            }
                        );
                        Some(recon_tool::report::ProteinContext {
                            fasta: fasta
                                .as_ref()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default(),
                            proteins_indexed: ix.len(),
                            psms_resolved: resolved,
                            psms_total: total,
                            resolved_pct: 100.0 * frac,
                            protein_nterm_psms: nterm,
                        })
                    }
                    None => None,
                };
                let tiers = tier_assignment::assign(
                    &peaks,
                    &background,
                    &curated,
                    floor,
                    protein_index.as_ref(),
                );
                println!(
                    "  Recommendations: {} by statistics, {} by abundance ({} background PSMs)",
                    tiers
                        .iter()
                        .filter(|t| matches!(
                            t.decision,
                            tier_assignment::Decision::Statistics { .. }
                        ))
                        .count(),
                    tiers
                        .iter()
                        .filter(|t| matches!(
                            t.decision,
                            tier_assignment::Decision::Abundance { .. }
                        ))
                        .count(),
                    background.len()
                );
                // Informational Unimod names for peaks the curated list cannot name.
                let unimod_name = |delta: f64| -> Option<String> {
                    mod_discovery
                        .peaks
                        .iter()
                        .find(|p| (p.delta_mass - delta).abs() < 1e-9)
                        .and_then(|p| p.annotations.first())
                        .map(|a| a.name.clone())
                };
                Some(tier_assignment::to_report(
                    &tiers,
                    mod_discovery.summary.total_psms,
                    floor,
                    FLOOR_PCT_OF_TOP,
                    unimod_name,
                    protein_context,
                ))
            }
            Err(e) => {
                log::warn!("curated mod list not loaded, recommendations omitted: {e:#}");
                None
            }
        }
    };

    // Which analyzer acquired these scans. `recon run` has already used this to
    // set the pass-1 fragment tolerance BEFORE the search; recording it here is
    // what lets a reader tell whether the tolerance recommendation applies to
    // their instrument at all. A ppm ladder is meaningless for an ion trap.
    // Detection failure is NOT fatal: the report simply omits the block rather
    // than losing every other number in it.
    let analyzers = match recon_tool::detect_analyzers(&mzml_path) {
        Ok(census) => {
            let d = census.ms2_decision();
            println!();
            println!(
                "[ANALYZER] MS1 {:?} | MS2 {:?} -> pass-1 fragment_tol {}{}",
                d.ms1_analyzers,
                d.ms2_analyzers,
                d.tolerance,
                if d.assumed {
                    "  (ASSUMED, not detected)"
                } else {
                    ""
                }
            );
            Some(recon_tool::report::AnalyzerReport {
                instrument_model: census.instrument_model.clone(),
                ms1_analyzers: d.ms1_analyzers.clone(),
                ms2_analyzers: d.ms2_analyzers.clone(),
                ms2_switched: census.ms2_switched(),
                pass1_fragment_tol: d.tolerance.to_string(),
                basis: format!("{:?}", d.basis),
                assumed: d.assumed,
                explanation: d.explanation.clone(),
            })
        }
        Err(e) => {
            log::warn!("analyzer detection failed, block omitted: {e:#}");
            None
        }
    };

    // Build unified report
    let report = ReconReport::from_analyses(
        &mzml_path.display().to_string(),
        &tsv.display().to_string(),
        &unimod_label,
        fasta.as_ref().map(|p| p.display().to_string()).as_deref(),
        enzyme,
        &mzml_stats,
        &mod_discovery,
        &signal_fate,
        &polymer,
        &oxonium,
        &digestion,
        &qc,
        alkylation,
        ms1_calibration,
        recommendations,
        analyzers,
    );

    let elapsed = start.elapsed();
    println!();
    println!("Analysis completed in {:.2}s", elapsed.as_secs_f64());
    println!();

    // Print text summary to console
    print_report_summary(&report);

    // Write output files
    // No Pass 2 here by construction: `analyze` is one stage, not a run.
    write_report_files(&report, None, &output, true)?;

    println!();
    println!("================================================================================");

    Ok(report)
}

/// Stages 3-5 of the one-command flow: subset FASTA, semi-enzymatic Pass 2
/// search, terminus annotation, and the Pass-1-vs-Pass-2 comparison.
///
/// Returns the wall-clock time spent inside the Pass 2 Sage search, so the
/// caller can report it apart from Pass 1 and from recon's own work. Pass 2
/// roughly doubles runtime on a well-behaved file and the user is told which
/// half went where.
///
/// Also returns the Pass 2 report itself, so the FINAL HTML write can render the
/// Digestion section. Those numbers live only in `<base>_pass2.json` — folding
/// them into `ReconReport` would move a frozen schema, which is exactly what
/// `report::Pass2Report`'s own doc argues against. `None` means Pass 2 did not
/// run, and the HTML then says so.
///
/// EVERYTHING Pass 2 needs was MEASURED by Pass 1: the MS1 window from
/// `ms1_pass2_window` (the ladder rung centred on the measured bias), the MS2
/// tolerance from `ms2_pass2_tolerance` (clamped to Pass 1's own window, in
/// Pass 1's unit), and the protein set from Pass 1's identifications. Nothing
/// here is a constant that a user has to know to change.
#[allow(clippy::too_many_arguments)]
fn run_pass2(
    mzml: &std::path::Path,
    fasta: &std::path::Path,
    pass1_tsv: &std::path::Path,
    report: &recon_tool::report::ReconReport,
    decision: &recon_tool::mzml::Ms2TolDecision,
    search_dir: &std::path::Path,
    output_base: &std::path::Path,
    pass2_params: Option<PathBuf>,
    enzyme: &recon_tool::enzyme::Enzyme,
    q_threshold: f64,
    no_pass2: bool,
) -> Result<(std::time::Duration, Option<recon_tool::report::Pass2Report>)> {
    use recon_tool::sage_runner::{run_sage, SageConfig};

    let zero = std::time::Duration::from_secs(0);
    if no_pass2 {
        println!();
        println!("[PASS2] Skipped (--no-pass2). Digestion figures come from Pass 1 only,");
        println!("        which is a FULLY TRYPTIC search and cannot see a ragged terminus.");
        return Ok((zero, None));
    }

    // Pass 2 is driven entirely by Pass 1's calibration. Without it there is no
    // window to search in, so say that plainly rather than substituting a
    // default that would look like a measurement.
    let Some(cal) = report.ms1_calibration.as_ref() else {
        println!();
        println!("[PASS2] SKIPPED: Pass 1 produced no MS1 calibration, so there is no");
        println!("        measured window to search Pass 2 in. This is not a Pass 2");
        println!("        failure — the clean subset was empty.");
        return Ok((zero, None));
    };

    // The template is TEXT plus a LABEL, not a path. With no --pass2-params the
    // text is compiled into the binary, so `recon` does not need to be launched
    // from the repo root. See `defaults.rs`.
    let (template_text, template_label) = match pass2_params {
        Some(p) => {
            if !p.exists() {
                anyhow::bail!("Pass 2 params template not found: {}", p.display());
            }
            let t = std::fs::read_to_string(&p)
                .with_context(|| format!("failed to read Pass 2 template: {}", p.display()))?;
            (t, p.display().to_string())
        }
        None => (
            recon_tool::defaults::PASS2.to_string(),
            recon_tool::defaults::bundled_label("pass-2"),
        ),
    };

    println!();
    println!("================================================================================");
    println!("  PASS 2 — semi-enzymatic search on the identified subset");
    println!("================================================================================");

    // --- Stage 3: subset FASTA ----------------------------------------------
    let pass2_dir = search_dir.join("pass2");
    std::fs::create_dir_all(&pass2_dir)?;
    let subset = pass2_dir.join("subset_identified_proteins.fasta");
    // PROTEIN PARSIMONY, adopted 2026-08-31. The subset is the SMALLEST set of
    // accessions that explains every confident peptide, not every accession any
    // peptide mapped to. Measured on two files: liver 3909 -> 1721, bcell
    // 4719 -> 4214, Pass 2 Sage 40 % faster on BOTH, and no reported digestion
    // rate moves more than 0.14 pp — against a 0.77 pp recon-to-MSFragger
    // disagreement already accepted on the same quantity.
    let accessions = recon_tool::protein_index::parsimonious_accessions(
        pass1_tsv,
        q_threshold,
        recon_tool::protein_index::MIN_PEPTIDES_PER_PROTEIN,
    )?;
    let written = recon_tool::write_subset_fasta(fasta, &accessions, &subset)?;
    println!(
        "[PASS2] Subset FASTA (parsimony): {} proteins, each keeping >={} peptides \
         no other protein explains ({} requested) -> {}",
        written,
        recon_tool::protein_index::MIN_PEPTIDES_PER_PROTEIN,
        accessions.len(),
        subset.display()
    );
    if written == 0 {
        anyhow::bail!(
            "subset FASTA is empty: none of the {} Pass-1 accessions were found in {}. \
             Pass 2 would search against nothing.",
            accessions.len(),
            fasta.display()
        );
    }

    // --- Stage 4: build the Pass 2 config from Pass 1's measurements --------
    let ms1_window = recon_tool::calibration::Ms1Pass2Window {
        low_ppm: cal.pass2_window_low_ppm,
        high_ppm: cal.pass2_window_high_ppm,
    };
    // The MS2 number keeps Pass 1's UNIT. `ms2_median_abs_ppm` is the median of Sage's
    // absolute fragment_ppm — see its doc comment; it is the right input here
    // because the Pass 2 window is symmetric about zero and only needs a width.
    let ms2_tolerance = cal
        .ms2_median_abs_ppm
        .map(|m| recon_tool::calibration::ms2_pass2_tolerance(m, decision.tolerance));

    let plan = recon_tool::Pass2Plan {
        ms1_window: ms1_window.clone(),
        ms2_tolerance,
        pass1_fragment_tol: decision.tolerance,
        subset_fasta: subset.clone(),
        subset_proteins: written,
    };
    let template_text = recon_tool::enzyme::apply_to_params_text(&template_text, enzyme)?;
    let pass2_params_path = recon_tool::pass2::write_pass2_params_from_text(
        &template_text,
        &template_label,
        &plan,
        &pass2_dir,
    )?;
    println!(
        "[PASS2] MS1 window (delta): {:+.2} to {:+.2} ppm  |  MS2: {}",
        ms1_window.low_ppm,
        ms1_window.high_ppm,
        match ms2_tolerance {
            Some(t) => t.to_string(),
            None => "template default (Pass 1 measured no MS2 error)".to_string(),
        }
    );
    println!("[PASS2] Effective config: {}", pass2_params_path.display());

    // --- Stage 4b: the search ------------------------------------------------
    println!("[PASS2] Running semi-enzymatic search...");
    let t0 = std::time::Instant::now();
    let cfg = SageConfig {
        params_path: pass2_params_path.clone(),
        fasta_path: Some(subset.clone()),
        output_dir: pass2_dir.clone(),
        mzml_paths: vec![mzml.to_path_buf()],
    };
    let pass2_result = run_sage(&cfg)?;
    let elapsed = t0.elapsed();
    println!(
        "[PASS2] Done in {:.1}s -> {}",
        elapsed.as_secs_f64(),
        pass2_result.results_tsv.display()
    );

    // --- Stage 5: terminus annotation ---------------------------------------
    let index = recon_tool::ProteinIndex::from_fasta(&subset)?;
    let opts = recon_tool::FilterOptions {
        q_threshold,
        ..Default::default()
    };
    let pass2_results =
        recon_tool::sage_results::parse_sage_results(&pass2_result.results_tsv, &opts)?;
    let terminus =
        recon_tool::digestion::compute_terminus_stats(&pass2_results.psms, &index, enzyme);
    let digestion = recon_tool::digestion::compute_digestion_stats(&pass2_results);
    println!();
    recon_tool::digestion::print_terminus_summary(&terminus);

    // The reported composition needs DECOYS, to subtract them per specificity
    // class the way Preview does. It is loaded SEPARATELY rather than reusing
    // `pass2_results` with `keep_decoys` set, because `compute_digestion_stats`
    // does not filter decoys itself — feeding it a mixed population would put
    // decoy PSMs into a reported missed-cleavage distribution. Two loads of one
    // TSV is cheap; a decoy leaking into a user-facing count is not.
    let composition = {
        let with_decoys = recon_tool::sage_results::parse_sage_results(
            &pass2_result.results_tsv,
            &recon_tool::FilterOptions {
                q_threshold,
                keep_decoys: true,
                ..Default::default()
            },
        )?;
        recon_tool::digestion::compute_digestion_composition(&with_decoys.psms, &index, enzyme)
    };
    recon_tool::digestion::print_composition_summary(&composition);

    // --- Stage 5b: Pass-1 prediction vs Pass-2 observation -------------------
    // Pass 1 is fully tryptic, so its semi-tryptic rate is a CONTROL that should
    // sit near zero, not a prediction that Pass 2 refines. Saying so in the
    // artifact stops the two numbers being read as a before/after.
    let pass1_semi = report.digestion.ragged_ends_pct;
    let pass1_n = report.mod_discovery.total_psms;
    let semi_count = terminus.semi_enzymatic_total;
    let comparison = recon_tool::report::Pass2Comparison {
        pass1_psms: pass1_n,
        pass1_semi_enzymatic_pct: pass1_semi,
        pass2_psms: terminus.classified_psms,
        pass2_semi_enzymatic_pct: terminus.semi_enzymatic_pct,
        pass2_only_semi_enzymatic: semi_count,
        note: format!(
            "Pass 1 is a FULLY TRYPTIC search over the whole database and cannot \
             generate a semi-tryptic peptide, so its {pass1_semi:.2} % is a control, \
             not a prediction. Pass 2 is semi-enzymatic over {written} identified \
             proteins and measured {:.2} % ({semi_count} PSMs). The two rates have \
             DIFFERENT denominators ({pass1_n} vs {}) and different search spaces; \
             their difference is not a delta.",
            terminus.semi_enzymatic_pct, terminus.classified_psms
        ),
    };
    println!();
    println!("--- Pass 1 (control) vs Pass 2 (measurement) ---");
    println!(
        "  Pass 1 semi-tryptic: {:.2} % of {} PSMs  (fully-tryptic search — expected ~0)",
        pass1_semi, pass1_n
    );
    println!(
        "  Pass 2 semi-tryptic: {:.2} % of {} PSMs  ({} PSMs)",
        terminus.semi_enzymatic_pct, terminus.classified_psms, semi_count
    );
    println!("  Different denominators and different search spaces — not a delta.");

    // --- Stage 5c: write the artifact ---------------------------------------
    let pass2_report = recon_tool::report::Pass2Report {
        schema_version: recon_tool::report::PASS2_SCHEMA_VERSION.to_string(),
        generated_at: chrono::Utc::now(),
        tool_version: recon_tool::provenance::TOOL_VERSION.to_string(),
        git_commit: recon_tool::provenance::GIT_COMMIT.to_string(),
        source_file: mzml.display().to_string(),
        effective_params: pass2_params_path.display().to_string(),
        subset_fasta: subset.display().to_string(),
        subset_proteins: written,
        ms1_window_low_ppm: ms1_window.low_ppm,
        ms1_window_high_ppm: ms1_window.high_ppm,
        ms2_tolerance: ms2_tolerance.map(|t| t.to_string()),
        pass1_fragment_tol: decision.tolerance.to_string(),
        terminus,
        digestion,
        composition,
        comparison,
        pass2_search_seconds: elapsed.as_secs_f64(),
    };
    let path = format!("{}_pass2.json", output_base.display());
    std::fs::write(&path, serde_json::to_string_pretty(&pass2_report)?)?;
    println!();
    println!("[PASS2] Report written to: {path}");

    Ok((elapsed, Some(pass2_report)))
}

/// One-command recon: wide open Sage search on <mzml>+<fasta>, then analyze.
///
/// This orchestrates the two pure-Rust, already-proven pieces — `sage_runner`
/// to run the wide open search, then `run_analyze_command` to build the unified
/// report — behind a single `recon run <mzml> <fasta>` entry point. It does NOT
/// yet run the semi-tryptic Pass 2 or the self-calibrated MS1 tolerance; those
/// are separate follow-up passes (see PLAN "Default recon"). The FASTA and mzML
/// override whatever the params template names, so the template supplies only
/// the search settings (tolerances, enzyme, chimera), never the inputs.
#[allow(clippy::too_many_arguments)]
fn run_run_command(
    mzml: PathBuf,
    fasta: PathBuf,
    unimod: Option<PathBuf>,
    output: Option<PathBuf>,
    params: Option<PathBuf>,
    search_out: Option<PathBuf>,
    enzyme_spec: String,
    cleave_at: Option<String>,
    restrict: Option<String>,
    c_terminal: Option<bool>,
    q_threshold: f64,
    pass2_params: Option<PathBuf>,
    no_pass2: bool,
) -> Result<()> {
    use recon_tool::sage_runner::{run_sage, SageConfig};
    use std::path::Path;

    let overall_start = std::time::Instant::now();

    // Resolve the protease FIRST: a bad spec should fail in milliseconds, not
    // after a multi-minute search. `--enzyme` names it; the three explicit flags
    // override individual fields for a protease no preset covers.
    let enzyme = {
        let mut e =
            recon_tool::enzyme::parse(&enzyme_spec).map_err(|err| anyhow::anyhow!("{err}"))?;
        if let Some(c) = cleave_at {
            e.cleave_at = c.to_ascii_uppercase().into_bytes();
            e.name = format!("{} (cleave_at={c})", e.name);
        }
        if let Some(r) = restrict {
            e.restrict = r.to_ascii_uppercase().into_bytes();
            e.name = format!("{} (restrict={r})", e.name);
        }
        if let Some(ct) = c_terminal {
            e.c_terminal = ct;
            e.name = format!("{} (c_terminal={ct})", e.name);
        }
        if e.cleave_at.is_empty() {
            anyhow::bail!(
                "the enzyme has no cleavage residues, so recon cannot classify termini \
                 or count missed cleavages"
            );
        }
        e
    };
    println!(
        "[ENZYME] {} — cleave_at {:?}, restrict {:?}, {}",
        enzyme.name,
        String::from_utf8_lossy(&enzyme.cleave_at),
        String::from_utf8_lossy(&enzyme.restrict),
        if enzyme.c_terminal {
            "C-terminal"
        } else {
            "N-terminal"
        }
    );

    // Fail fast on missing inputs before we spawn Sage.
    for (label, p) in [("mzML", &mzml), ("FASTA", &fasta)] {
        if !p.exists() {
            anyhow::bail!("{} not found: {}", label, p.display());
        }
    }
    if let Some(ref u) = unimod {
        if !u.exists() {
            anyhow::bail!("unimod.xml not found: {}", u.display());
        }
    }

    // Resolve the params template: explicit --params, else the bundled default.
    // The default lives at _dev/testing/configs/open-search-params.json relative to
    // the repo root (cwd when run from the repo, as the timing tests were).
    let (params_text, params_label) = match params {
        Some(p) => {
            if !p.exists() {
                anyhow::bail!("params template not found: {}", p.display());
            }
            let t = std::fs::read_to_string(&p)
                .with_context(|| format!("failed to read params template: {}", p.display()))?;
            (t, p.display().to_string())
        }
        None => (
            recon_tool::defaults::OPEN_SEARCH.to_string(),
            recon_tool::defaults::bundled_label("open-search"),
        ),
    };

    // Resolve the output base name: --output, else <mzml-stem>_recon in cwd.
    // A .mzML.gz has a two-part extension; strip both so the stem is clean.
    let output_base = match output {
        Some(o) => o,
        None => {
            let name = Path::new(&mzml)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "recon".to_string());
            // Strip .gz then the mzML extension.
            let stem = name
                .strip_suffix(".gz")
                .unwrap_or(&name)
                .rsplit_once('.')
                .map(|(base, _)| base.to_string())
                .unwrap_or(name.clone());
            PathBuf::from(format!("{}_recon", stem))
        }
    };

    // Resolve the Sage output dir: --search-out, else a sibling of the output
    // base named "<output_base>_search". We do NOT auto-clean it — the TSV is a
    // real artifact the user may want to inspect or re-analyze.
    let search_dir = match search_out {
        Some(d) => d,
        None => PathBuf::from(format!("{}_search", output_base.display())),
    };

    println!("================================================================================");
    println!("                    RECON — ONE-COMMAND RUN (open search + analyze)");
    println!("================================================================================");
    println!("  mzML:      {}", mzml.display());
    println!("  FASTA:     {}", fasta.display());
    println!("  params:    {}", params_label);
    println!("  search out:{}", search_dir.display());
    println!("  report:    {}.json / .html", output_base.display());
    println!();

    // --- Stage 0: detector check ---------------------------------------------
    // Costs a fraction of a second and reads no PSMs. It runs BEFORE the search
    // because a fragment tolerance in the wrong unit for the MS2 detector cannot
    // be fixed afterwards — the search simply would not have matched anything.
    println!("[DETECT] Reading the MS2 analyzer from the mzML header...");
    let census = recon_tool::detect_analyzers(&mzml)?;
    recon_tool::print_analyzer_census(&census);
    let decision = census.ms2_decision();
    // The template's fragment_tol is ±20 ppm regardless of instrument, so it is
    // overridden rather than trusted. The effective config is written next to the
    // search output so what was actually searched stays recoverable.
    let params_text = recon_tool::enzyme::apply_to_params_text(&params_text, &enzyme)?;
    let effective_params = recon_tool::sage_runner::write_effective_params_from_text(
        &params_text,
        &params_label,
        &decision,
        &search_dir,
        &fasta,
        std::slice::from_ref(&mzml),
    )?;
    println!("  effective config: {}", effective_params.display());
    println!();

    // --- Stage 1: wide open Sage search --------------------------------------
    println!("[SAGE] Running wide open search (this is the main wall-clock cost)...");
    let sage_start = std::time::Instant::now();
    let cfg = SageConfig {
        params_path: effective_params.clone(),
        fasta_path: Some(fasta.clone()), // -f overrides the template's database.fasta
        output_dir: search_dir.clone(),
        mzml_paths: vec![mzml.clone()], // positional arg overrides the template's mzml_paths
    };
    let sage_result = run_sage(&cfg)?;
    let sage_elapsed = sage_start.elapsed();
    println!(
        "[SAGE] Done in {:.1}s → {}",
        sage_elapsed.as_secs_f64(),
        sage_result.results_tsv.display()
    );
    println!();

    // --- Stage 2: analyze the open results into the unified report -----------
    // Reuse the existing analyze path verbatim so the report is identical to a
    // manual `recon analyze` on the same TSV — no divergent second code path.
    println!("[ANALYZE] Building unified report from the open-search results...");
    let report = run_analyze_command(
        mzml.clone(),
        sage_result.results_tsv.clone(),
        unimod,
        Some(fasta.clone()), // the search database IS the protein context
        Some(&enzyme),       // recorded in the report; `run` resolved it above
        q_threshold,
        output_base.clone(),
        PeakAssignmentMode::default(), // Merge — settled 2026-08-25, see NOTES
    )?;

    // --- Stages 3-5: Pass 2 --------------------------------------------------
    let (pass2_elapsed, pass2_report) = run_pass2(
        &mzml,
        &fasta,
        &sage_result.results_tsv,
        &report,
        &decision,
        &search_dir,
        &output_base,
        pass2_params,
        &enzyme,
        q_threshold,
        no_pass2,
    )?;

    let overall = overall_start.elapsed();

    // Rewrite the report now that the total is actually known. The first write
    // happened before Pass 2, which consumes the report, so it could only have
    // carried the time to first output.
    let mut report = report;
    report.runtime_seconds = Some(overall.as_secs_f64());
    write_report_files(&report, pass2_report.as_ref(), &output_base, false)?;

    println!();
    println!("================================================================================");
    println!(
        "  TOTAL one-command time: {:.1}s  (Pass-1 Sage {:.1}s + Pass-2 Sage {:.1}s + recon {:.1}s)",
        overall.as_secs_f64(),
        sage_elapsed.as_secs_f64(),
        pass2_elapsed.as_secs_f64(),
        (overall - sage_elapsed - pass2_elapsed).as_secs_f64()
    );
    println!(
        "  Report: {}.json  {}.html",
        output_base.display(),
        output_base.display()
    );
    println!("================================================================================");

    Ok(())
}

/// Print every PSI-MS mass analyzer term recon recognises, and the pass-1 MS2
/// fragment tolerance each one implies. Generated from the shipped table, so it
/// cannot drift from what the code actually does.
fn print_analyzer_tolerance_table() {
    use recon_tool::{
        bucket_tolerance, BucketSource, FILTER_STRING_ANALYZERS, MASS_ANALYZER_TERMS,
        UNKNOWN_MS2_FALLBACK_PPM,
    };

    println!("PSI-MS mass analyzer terms (children of MS:1000443) -> pass-1 MS2 fragment_tol");
    println!("{:-<104}", "");
    println!(
        "{:<13} {:<52} {:<24} {:<12}",
        "accession", "CV name", "MS2 fragment_tol", "bucket from"
    );
    println!("{:-<104}", "");
    for term in MASS_ANALYZER_TERMS {
        let tol = match bucket_tolerance(term.class) {
            Some(t) => t.to_string(),
            None => format!("fallback ±{UNKNOWN_MS2_FALLBACK_PPM} ppm"),
        };
        println!(
            "{:<13} {:<52} {:<24} {:<12}",
            term.accession,
            term.name,
            tol,
            match term.bucket_source {
                BucketSource::Curated => "curated",
                BucketSource::ExtendedHere => "extended",
            }
        );
    }
    println!("{:-<104}", "");
    println!();
    println!("Thermo filter-string analyzer tokens (MS:1000512), the preferred per-scan signal:");
    for (token, class) in FILTER_STRING_ANALYZERS {
        let tol = match bucket_tolerance(*class) {
            Some(t) => t.to_string(),
            None => format!("fallback ±{UNKNOWN_MS2_FALLBACK_PPM} ppm"),
        };
        println!("  {:<8} -> {:<40} {}", token, class.label(), tol);
    }
    println!();
    println!("'curated' = bucket named in reference-notes/analyzer-tolerances/.");
    println!("'extended' = assigned by CV parentage; see ms2-analyzer-tolerance-table.md.");
    println!("An analyzer with no bucket does NOT halt recon: it falls back to");
    println!("±{UNKNOWN_MS2_FALLBACK_PPM} ppm and the run reports that it did.");
    println!();
}
