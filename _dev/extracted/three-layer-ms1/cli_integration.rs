//! CLI and plumbing for the three-layer MS1 feature — EXTRACTED, NOT COMPILED.
//!
//! This file is NOT valid standalone Rust and is NOT part of any crate. It is a
//! verbatim record of the fragments that wired `three_layer_ms1.rs` into recon's
//! `signal-fate` subcommand, so the feature can be rebuilt elsewhere without
//! re-deriving it. Removed 2026-09-02 on Ben's instruction.

// ---------------------------------------------------------------------------
// 1. recon-tool/src/signal_fate.rs — field on `SignalFateResult`
// ---------------------------------------------------------------------------

    /// Three-layer MS1 fate (only present when --three-layer flag used).
    ///
    /// ⚠ THIS IS THE PRESERVED HOME OF THE THREE-LAYER METRIC (2026-09-01).
    /// It was removed from the unified `run`/`analyze` report because it is a
    /// DDA-METHOD diagnostic, not a sample-reconnaissance one, and recon's
    /// non-goals say it is not a general QC suite. It lives on here as a
    /// standalone command so the algorithm stays compiled, tested and runnable
    /// for porting to sageGUI. See NOTES "THREE-LAYER MS1 REMOVED FROM RECON".
    ///
    /// It was computed and PRINTED but never serialised, so `--output` silently
    /// dropped it. Fixed at the same time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub three_layer_ms1: Option<crate::mzml::ThreeLayerMs1Fate>,

// ...and its initialiser in `compute_signal_fate`:

        three_layer_ms1: None,

// ---------------------------------------------------------------------------
// 2. recon-tool/src/main.rs — clap flag on the `SignalFate` subcommand
// ---------------------------------------------------------------------------

        /// Compute three-layer MS1 signal fate (requires --mzml)
        /// Separates: non-peptidic, never-sampled, sampled-not-ID'd, identified
        #[arg(long)]
        three_layer: bool,

// ...its entry in the `Commands::SignalFate { .. }` destructuring and in the
// `run_signal_fate_command(..)` call, both as the bare binding `three_layer,`.

// ---------------------------------------------------------------------------
// 3. recon-tool/src/main.rs — `run_signal_fate_command` parameter and imports
// ---------------------------------------------------------------------------

    three_layer: bool,          // parameter, after `ms1_intensity: bool`

    use recon_tool::mzml::{
        build_precursor_queries_from_psms, compute_mod_breakdown, compute_three_layer_ms1_fate,
        extract_ms2_precursor_info, extract_precursor_intensities, print_three_layer_ms1_fate,
    };

// NOTE: `build_precursor_queries_from_psms` and `extract_precursor_intensities`
// are ALSO used by the `--ms1-intensity` path and stayed imported in main.rs.
// Only the other three names were dropped.

// ---------------------------------------------------------------------------
// 4. recon-tool/src/main.rs — the body, run after the --ms1-intensity block
// ---------------------------------------------------------------------------

    // Compute three-layer MS1 signal fate if requested
    if three_layer {
        if let Some(ref mzml_file) = mzml_file_path {
            println!();
            println!("Computing three-layer MS1 signal fate...");
            println!("  m/z tolerance: {} ppm", mz_tol_ppm);
            println!("  RT tolerance: {:.1} min", rt_window);

            // Extract MS1 spectra
            let ms1_spectra = extract_ms1_spectra(mzml_file)?;
            println!("  Extracted {} MS1 spectra", ms1_spectra.len());

            // Extract ALL MS2 precursor info (not just identified)
            let ms2_precursors = extract_ms2_precursor_info(mzml_file)?;
            println!(
                "  Extracted {} MS2 precursors (sampling events)",
                ms2_precursors.len()
            );

            // Build precursor queries from identified PSMs
            let identified_queries = build_precursor_queries_from_psms(&results.psms);
            println!("  Identified PSMs: {}", identified_queries.len());

            // Compute three-layer fate
            let mut three_layer_fate = compute_three_layer_ms1_fate(
                &ms1_spectra,
                &ms2_precursors,
                &identified_queries,
                mz_tol_ppm,
                rt_window,
            );

            // Compute modification breakdown for identified TIC
            println!();
            println!("Computing modification breakdown...");
            let mod_breakdown = compute_mod_breakdown(
                &results.psms,
                &ms1_spectra,
                mz_tol_ppm,
                rt_window,
                5, // Top 5 modifications
            );
            three_layer_fate.mod_breakdown = Some(mod_breakdown);

            // Print the three-layer summary (includes mod breakdown)
            print_three_layer_ms1_fate(&three_layer_fate);

            // Serialise it too. It used to be printed and then dropped, so
            // `--output` produced JSON with no three-layer block in it at all.
            fate.three_layer_ms1 = Some(three_layer_fate);
        } else {
            println!();
            println!("Warning: --three-layer requires --mzml to be specified");
        }
    }
