//! Runs Sage IN PROCESS.
//!
//! Sage is a Cargo git dependency, pinned by `rev`. `run_sage` calls
//! `sage_cli::runner::Runner` directly.
//!
//! ⚠ This doc used to say "Sage subprocess invocation module. Locates the Sage
//! binary and invokes it." That has been false since 2026-09-01, when Sage
//! became a library. There is no binary to locate, no `SAGE_PATH`, and no
//! version handshake. AGENTS.md forbids reintroducing any of them.
//!
//! Nothing here spawns a process. The crate contains no `Command::new` and no
//! `env::var`.

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

/// The version string Sage ACTUALLY REPORTS, which is what the guard compares.
///
/// **UPGRADED 2026-09-01: v0.14.7 -> v0.15.0-beta.2.** Verified by asking the
/// built binary: `sage --version` prints `sage 0.15.0-beta.2`, matching the crate
/// version at the tag. ⚠ Unlike v0.14.7 this release is HONEST about its version.
///
/// ⚠ **THE v0.14.7 HISTORY, kept because it explains the guard's shape.** That
/// release reported `0.14.6`, because upstream never bumped the crate versions at
/// the tag and `--version` is `clap::crate_version!()`. Confirmed four ways on
/// 2026-08-28. So this constant tracks WHAT THE BINARY SAYS, not the tag name,
/// and the two are not required to agree.
///
/// **A BETA IS PINNED DELIBERATELY (Ben, 2026-09-01).** No `v0.15.0` final exists
/// upstream — measured with `git ls-remote --tags`, newest are beta.1 and beta.2.
/// A NOTES instruction to "wait for the stable release" was an agent's invention,
/// never a project decision, and is corrected there. Reproducibility does not
/// depend on tag stability: `SAGE_COMMIT` pins an exact commit.
///
/// **`SAGE_COMMIT` is the real identity of the pin. This constant only has to
/// match what the binary says about itself.**
///
/// Update this constant (and SAGE_COMMIT) whenever the pinned Sage rev changes.
/// Also update: the schema validator column list, any affected config templates,
/// and bump recon's Cargo.toml version.
pub const SAGE_VERSION: &str = "0.15.0-beta.2";

/// Exact git commit hash of the pinned Sage. Must equal the `rev` on the
/// `sage-core`/`sage-cli`/`sage-cloudpath` git dependencies in `Cargo.toml`;
/// a test asserts `Cargo.lock` names this same rev.
/// Tag `v0.15.0-beta.2` on github.com/lazear/sage points to this commit —
/// verified 2026-09-01 from a CLEAN clone of upstream at
/// `reference/sage-src/` (gitignored), not from a working tree carrying local
/// edits. The previous pin was `99407db6e3754b31a9b88b7316a0aee67293c93f`
/// (v0.14.7).
pub const SAGE_COMMIT: &str = "df9219951cc9a54cf4cd55d76541af24b687bd3d";

/// Convert a local path into a `file://` URL string for Sage.
///
/// Sage's `to_url` calls `Url::parse` FIRST and returns early when it succeeds
/// (`sage-cloudpath/src/lib.rs:19-26`). A Windows path parses as a URL whose
/// SCHEME is the drive letter:
///
/// ```text
/// Url::parse("C:\\data\\x.mzML")  ->  Ok(scheme = "c")
/// ```
///
/// so `from_file_path` is never reached and the search fails. A Unix path has
/// no scheme, `parse` fails, and Sage falls through correctly -- which is why
/// this only bites Windows. v0.14.7 used `http::Uri` and was not affected.
///
/// Building the URL here gives Sage what it would have built itself, on every
/// platform. A real URL is passed through: a scheme of more than one character
/// cannot be a drive letter. A path that will not canonicalize is passed
/// through too, so Sage still raises its own not-found error.
fn to_sage_path(p: &std::path::Path) -> String {
    let raw = p.to_string_lossy().to_string();
    if url::Url::parse(&raw)
        .map(|u| u.scheme().len() > 1)
        .unwrap_or(false)
    {
        return raw;
    }
    p.canonicalize()
        .ok()
        .and_then(|c| url::Url::from_file_path(&c).ok())
        .map(|u| u.to_string())
        .unwrap_or(raw)
}

/// Configuration for one Sage search.
///
/// ⚠ **`sage_binary` IS GONE (A1 landing 2, 2026-09-01).** There is no external
/// binary to point at any more. So are `SAGE_PATH`, the vendored-layout search
/// and the runtime version guard: the pin now lives in `Cargo.lock` and cannot
/// drift between what we tested and what a user runs.
#[derive(Debug, Clone)]
pub struct SageConfig {
    /// Path to params JSON file
    pub params_path: PathBuf,
    /// Path to FASTA database (overrides the params file if provided)
    pub fasta_path: Option<PathBuf>,
    /// Output directory for Sage results
    pub output_dir: PathBuf,
    /// Path to mzML file(s) (overrides the params file if provided)
    pub mzml_paths: Vec<PathBuf>,
}

/// Result of a Sage run.
///
/// The `stdout`/`stderr` fields are gone with the subprocess: in-process there is
/// no child output to capture, and keeping empty strings around would be a field
/// that lies. `sage_version` is no longer scraped or optional — it is the pinned
/// constant, true by construction.
#[derive(Debug)]
pub struct SageRunResult {
    /// Path to results.sage.tsv
    pub results_tsv: PathBuf,
    /// Path to results.json (metadata)
    pub results_json: PathBuf,
    /// Pinned Sage version. Always `SAGE_VERSION`.
    pub sage_version: String,
}

/// Configure Rayon exactly as Sage's own `main` does, and only once.
///
/// Sage sets a 2 MiB worker stack before building its runner
/// (`crates/sage-cli/src/main.rs`, stack-size default 2). We mirror it so the
/// library route runs under the same thread configuration the binary did.
///
/// `build_global` can only succeed once per process, and recon runs TWO searches
/// (pass 1 and pass 2), so a second call returns an error that is expected and
/// must be ignored rather than propagated.
fn init_rayon_pool() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        const STACK_SIZE_MIB: usize = 2;
        if let Err(e) = rayon::ThreadPoolBuilder::new()
            .stack_size(STACK_SIZE_MIB * 1024 * 1024)
            .build_global()
        {
            // Not fatal: a global pool already exists, which is fine.
            log::debug!("Rayon global pool already configured: {e}");
        }
    });
}

/// Run one Sage search IN PROCESS.
///
/// ⚠ **THIS NO LONGER SHELLS OUT.** It calls `sage_cli::runner::Runner` directly,
/// against the commit pinned in `Cargo.lock`. What that changes and what it does
/// not:
///
/// * **Unchanged:** the params JSON is still the input, `results.sage.tsv` and
///   `results.json` are still the outputs in `output_dir`, and the effective
///   params are still written beside the search. A third party can still re-run
///   stock Sage with those params and get the same file.
/// * **Gone:** the binary lookup, `SAGE_PATH`, the `--version` handshake, and the
///   `--disable-telemetry-i-dont-want-to-improve-sage` flag. Telemetry is silent
///   BY CONSTRUCTION here: `Telemetry::send` is called only from Sage's own
///   `main.rs:138`, never from `Runner::run`, which merely returns the struct.
///   Verified against the pinned source, not assumed.
/// * **Expected to move:** q-values, by roughly 0.1% of PSM membership at
///   q <= 0.01. This build is not the build that produced the committed
///   reference data. Raw measurements stay bit-identical. See NOTES "Sage output
///   is NOT bit-reproducible across builds".
pub fn run_sage(config: &SageConfig) -> Result<SageRunResult> {
    use sage_cli::input::Input;
    use sage_cli::runner::Runner;

    init_rayon_pool();

    std::fs::create_dir_all(&config.output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            config.output_dir.display()
        )
    })?;

    let params = config.params_path.to_string_lossy().to_string();
    let mut input =
        Input::load(&params).with_context(|| format!("Failed to load Sage params: {params}"))?;

    // The same three overrides the subprocess passed as flags: -o, -f, and the
    // positional mzML paths. Everything else comes from the template.
    input.output_directory = Some(to_sage_path(&config.output_dir));

    if let Some(ref fasta) = config.fasta_path {
        input.database.fasta = Some(to_sage_path(fasta));
    }

    if !config.mzml_paths.is_empty() {
        input.mzml_paths = Some(config.mzml_paths.iter().map(|p| to_sage_path(p)).collect());
    }

    // Sage's own default: files searched in parallel = CPUs / 2.
    let parallel = (num_cpus::get() / 2).max(1);

    log::info!(
        "Running Sage {SAGE_VERSION} in process (commit {SAGE_COMMIT}), parallel={parallel}"
    );

    let search = input.build().context("Failed to build the Sage search")?;
    let runner = Runner::new(search, parallel).context("Failed to construct the Sage runner")?;

    // `parquet: false` -- recon parses the TSV, and the schema validator is
    // written against it.
    let _telemetry = runner.run(parallel, false).context("Sage search failed")?;

    let results_tsv = config.output_dir.join("results.sage.tsv");
    let results_json = config.output_dir.join("results.json");

    if !results_tsv.exists() {
        return Err(anyhow!(
            "Sage completed but results.sage.tsv not found at: {}",
            results_tsv.display()
        ));
    }

    log::info!("Sage completed successfully");

    Ok(SageRunResult {
        results_tsv,
        results_json,
        sage_version: SAGE_VERSION.to_string(),
    })
}

fn json_fasta(json: &serde_json::Value) -> Option<String> {
    json.get("database")
        .and_then(|d| d.get("fasta"))
        .and_then(|f| f.as_str())
        .map(|s| s.to_string())
}

/// The real implementation. Takes the template TEXT plus a LABEL naming where it
/// came from, so a bundled default and a `--params` file follow one code path and
/// the provenance record can say honestly which was used.
pub fn write_effective_params_from_text(
    template_text: &str,
    template_label: &str,
    decision: &crate::mzml::Ms2TolDecision,
    out_dir: &Path,
    fasta: &Path,
    mzml_paths: &[PathBuf],
) -> Result<PathBuf> {
    let mut json: serde_json::Value = serde_json::from_str(template_text)
        .with_context(|| format!("Failed to parse params template as JSON: {template_label}"))?;

    let previous = json.get("fragment_tol").cloned();
    let template_static = json
        .get("database")
        .and_then(|d| d.get("static_mods"))
        .cloned();
    let template_variable = json
        .get("database")
        .and_then(|d| d.get("variable_mods"))
        .cloned();
    let template_fasta = json_fasta(&json);
    let obj = json
        .as_object_mut()
        .ok_or_else(|| anyhow!("params template is not a JSON object: {template_label}"))?;
    obj.insert(
        "fragment_tol".to_string(),
        decision.tolerance.to_sage_json(),
    );

    // PASS 1 SEARCHES WITH NO MODIFICATIONS. This is the alkylation-agnostic
    // default, and it is the product claim: an open search that ASSUMES
    // carbamidomethyl cannot report whether the sample was alkylated, or with
    // what. It is stripped HERE rather than in the template because 17 configs
    // under _dev/testing/configs/ carry `static_mods {C: 57.0215}` and any of them can
    // be passed with --params.
    //
    // MEASURED on bcell, 2026-08-31: with `static_mods {C: 57.0215}` the +57.02
    // peak reads n=146; without it, n=3311 — and it is the TOP peak in the file.
    // A fixed Cys mod does not merely hide a peak, it removes the largest signal
    // recon exists to surface, and with it the abundance floor derived from it.
    // THE DATABASE AND THE SPECTRA, AS ACTUALLY SEARCHED.
    //
    // ⚠ These are overridden for the same reason the mods are. Sage takes the
    // FASTA from the `-f` CLI flag and the mzML from a positional argument, both
    // of which BEAT the JSON. So a template's `database.fasta` is not what runs —
    // and copying it here produced a file that named a database nobody searched.
    //
    // That is not hypothetical. On 2026-09-01 this file claimed
    // `UniProt-Human-UP000005640_canonical-2023_05.fasta` for a liver run made
    // against `uniprot_sprot_iso_human-2018_06.fasta`, and a session read the
    // claim, believed it, and drew a wrong conclusion about which database the
    // committed artifact used. An artifact that calls itself "effective" must
    // not carry a value the run overrode.
    if let Some(db) = obj.get_mut("database").and_then(|d| d.as_object_mut()) {
        db.insert("static_mods".to_string(), serde_json::json!({}));
        db.insert("variable_mods".to_string(), serde_json::json!({}));
        db.insert("max_variable_mods".to_string(), serde_json::json!(0));
        db.insert(
            "fasta".to_string(),
            serde_json::json!(fasta.display().to_string()),
        );
    }
    obj.insert(
        "mzml_paths".to_string(),
        serde_json::json!(mzml_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()),
    );
    // ASSERTED, not assumed. If a future edit reorders this, the run stops rather
    // than silently producing a fixed-C search that reports "no alkylation".
    let leftover = obj
        .get("database")
        .and_then(|d| d.get("static_mods"))
        .and_then(|m| m.as_object())
        .map(|m| m.len())
        .unwrap_or(0);
    if leftover != 0 {
        return Err(anyhow!(
            "pass 1 would search with {leftover} static modification(s). Pass 1 is the \
             alkylation-agnostic open search: a fixed Cys mod makes every alkylated \
             peptide match at delta 0, so the report cannot say what the sample was \
             alkylated with — the question recon exists to answer."
        ));
    }

    // ISOTOPE ERRORS ARE ASSERTED, NOT INHERITED.
    //
    // Pass 1 overrides fragment_tol, the mods, the FASTA and the mzML — but it
    // does NOT override precursor_tol or isotope_errors, because those two ARE
    // the open-search definition and must come from the template. That makes
    // isotope_errors a live ghost: any template passed with --params can carry a
    // value that silently ruins the search.
    //
    // MEASURED and recorded in NOTES "CLOSED-NEGATIVE — isotope_errors [-1, 2]
    // is harmful in an open search": it cut the +57 peak from 1253 to 394 and
    // FABRICATED a Propionyl peak at +56.018 = +57.022 - 1 neutron. An open
    // search must not let Sage try neighbouring isotopes, because the delta mass
    // IS the measurement.
    let isotopes = obj.get("isotope_errors").cloned();
    let ok = isotopes
        .as_ref()
        .and_then(|v| v.as_array())
        .map(|a| a.len() == 2 && a[0] == 0 && a[1] == 0)
        .unwrap_or(false);
    if !ok {
        return Err(anyhow!(
            "pass 1 template {} sets isotope_errors = {}, but the open search \
             requires [0, 0]. A non-zero isotope window lets Sage match a \
             neighbouring isotope peak, which moves the delta mass — and the \
             delta mass IS what recon measures. Measured cost of [-1, 2]: the \
             +57 peak falls from 1253 to 394 and a Propionyl peak at +56.018 is \
             invented out of +57.022 minus one neutron. See NOTES.",
            template_label,
            isotopes
                .map(|v| v.to_string())
                .unwrap_or_else(|| "absent".into())
        ));
    }

    // Record why, inside the artifact itself. Sage ignores unknown keys, and it
    // means the effective config explains its own provenance.
    obj.insert(
        "_recon_fragment_tol_provenance".to_string(),
        serde_json::json!({
            "template": template_label,
            "template_fragment_tol": previous,
            "applied": decision.tolerance.to_sage_json(),
            "basis": format!("{:?}", decision.basis),
            "assumed": decision.assumed,
            "ms1_analyzers": decision.ms1_analyzers,
            "ms2_analyzers": decision.ms2_analyzers,
            "explanation": decision.explanation,
            "template_static_mods": template_static,
            "template_variable_mods": template_variable,
            "template_fasta": template_fasta,
            "fasta_note": "database.fasta and mzml_paths are the values ACTUALLY \
                           searched, taken from --fasta/--mzml. Sage's -f flag and \
                           positional mzML beat the JSON, so the template's values \
                           are recorded separately and are NOT what ran.",
            "mods_note": "Pass 1 searches with NO fixed and NO variable mods. A fixed \
                          Cys mod would put every alkylated peptide at delta 0 and \
                          hide the alkylation state this tool exists to report.",
        }),
    );

    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("Failed to create output directory: {}", out_dir.display()))?;
    let path = out_dir.join("effective-params.json");
    std::fs::write(&path, serde_json::to_string_pretty(&json)?)
        .with_context(|| format!("Failed to write effective params: {}", path.display()))?;

    log::info!(
        "fragment_tol set to {} ({:?}); effective config at {}",
        decision.tolerance,
        decision.basis,
        path.display()
    );
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE PIN IS NOW A LOCKFILE ENTRY, SO GATE THE LOCKFILE.
    ///
    /// This replaces `the_vendored_binary_reports_exactly_sage_version`, which
    /// asked a binary that no longer exists. The failure it guards against is
    /// real and silent: someone runs `cargo update`, the git rev moves, and recon
    /// parses a TSV written by a different Sage than the one every pinned number
    /// came from.
    #[test]
    fn cargo_lock_pins_exactly_sage_commit() {
        let lock = include_str!("../Cargo.lock");
        let mut found = Vec::new();
        for (i, line) in lock.lines().enumerate() {
            if line.contains("lazear/sage") {
                found.push((i + 1, line.trim().to_string()));
            }
        }
        assert!(
            !found.is_empty(),
            "Cargo.lock names no lazear/sage dependency at all"
        );
        for (lineno, line) in &found {
            assert!(
                line.contains(SAGE_COMMIT),
                "Cargo.lock line {lineno} pins a different Sage than SAGE_COMMIT.\n\
                 expected rev: {SAGE_COMMIT}\n\
                 found:        {line}"
            );
        }
    }

    /// Falsification of the test above: a lockfile line for another rev must
    /// fail the same substring check. Guards against the assertion being
    /// vacuously true.
    #[test]
    fn a_different_rev_would_fail_the_pin_check() {
        let wrong = "source = \"git+https://github.com/lazear/sage.git?rev=99407db6e3754b31a9b88b7316a0aee67293c93f#99407db6\"";
        assert!(!wrong.contains(SAGE_COMMIT));
    }
}
