//! Pass 2 — the semi-enzymatic subset search that measures digestion.
//!
//! Pass 1 is a wide open search over the whole database. Pass 2 is narrow in
//! mass and wide in enzyme specificity: it searches ONLY the proteins Pass 1
//! identified, with `semi_enzymatic: true`, inside the mass window Pass 1
//! measured. Its only job is digestion efficiency and missed cleavages.
//!
//! ## The sign convention, MEASURED not assumed
//!
//! AGENTS.md locks the inversion for `precursor_tol.da`. It does NOT say the
//! same holds for `.ppm`, and the Pass 2 window is ASYMMETRIC (centred on the
//! measured bias), so getting the sign wrong would mis-centre the window by
//! twice the bias without any symptom in the output.
//!
//! Probed on serum, 2026-08-29, with `precursor_tol: {"ppm": [-30, 5]}`:
//! the observed delta `(expmass - calcmass)/calcmass` ran **-5.047 to +30.007
//! ppm** over 12746 target PSMs. The walls sit on -5 and +30, so `.ppm` inverts
//! exactly as `.da` does.
//!
//! > To search delta in `[lo, hi]`, the config must read `[-hi, -lo]`.
//!
//! `precursor_tol_json` is the single place that conversion happens, and
//! `pass2_precursor_window_is_sign_inverted` is its control.

use crate::calibration::Ms1Pass2Window;
use crate::mzml::FragmentTolerance;
use anyhow::{anyhow, Context, Result};

/// Pass 2 isotope offsets. See `write_pass2_params_from_text` for the measurement that
/// dropped the templates' `-1`.
pub const PASS2_ISOTOPE_ERRORS: [i32; 2] = [0, 3];
use std::path::{Path, PathBuf};

/// The Sage `precursor_tol` object for a Pass 2 window.
///
/// `window` is in DELTA space (`expmass - calcmass`), which is how
/// `ms1_pass2_window` reports it and how every measurement in this repo reads.
/// The returned config is in Sage's INVERTED space. See the module docs.
pub fn precursor_tol_json(window: &Ms1Pass2Window) -> serde_json::Value {
    serde_json::json!({ "ppm": [-window.high_ppm, -window.low_ppm] })
}

/// Everything Pass 2 needs that Pass 1 had to measure first.
#[derive(Debug, Clone)]
pub struct Pass2Plan {
    /// MS1 window in DELTA space, from `calibration::ms1_pass2_window`.
    pub ms1_window: Ms1Pass2Window,
    /// MS2 tolerance in the unit Pass 1 used, from
    /// `calibration::ms2_pass2_tolerance`. `None` when Pass 1 measured no MS2
    /// error — Pass 2 then keeps the template's own `fragment_tol` and says so.
    pub ms2_tolerance: Option<FragmentTolerance>,
    /// The Pass-1 fragment tolerance the MS2 number is clamped against, carried
    /// for the provenance block.
    pub pass1_fragment_tol: FragmentTolerance,
    /// Subset FASTA written from the Pass-1 identifications.
    pub subset_fasta: PathBuf,
    /// How many proteins that subset holds.
    pub subset_proteins: usize,
}

/// The real implementation. Takes the template TEXT plus a LABEL naming its
/// origin, so a bundled default and a `--pass2-params` file share one code path.
pub fn write_pass2_params_from_text(
    template_text: &str,
    template_label: &str,
    plan: &Pass2Plan,
    out_dir: &Path,
) -> Result<PathBuf> {
    let mut json: serde_json::Value = serde_json::from_str(template_text)
        .with_context(|| format!("failed to parse Pass 2 template as JSON: {template_label}"))?;

    let template_precursor = json.get("precursor_tol").cloned();
    let template_fragment = json.get("fragment_tol").cloned();
    let template_isotope = json.get("isotope_errors").cloned();
    let template_static = json
        .get("database")
        .and_then(|d| d.get("static_mods"))
        .cloned();
    let template_variable = json
        .get("database")
        .and_then(|d| d.get("variable_mods"))
        .cloned();

    // The enzyme block is the whole point of Pass 2. A template that is not
    // semi-enzymatic would measure nothing about ragged termini, and the report
    // would read "0 % semi-tryptic" as a finding rather than as a
    // misconfiguration. Fail loudly instead.
    let semi = json
        .get("database")
        .and_then(|d| d.get("enzyme"))
        .and_then(|e| e.get("semi_enzymatic"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !semi {
        return Err(anyhow!(
            "Pass 2 template {} does not set database.enzyme.semi_enzymatic = true. \
             Pass 2 exists to measure ragged termini; a fully-tryptic search cannot \
             find one, and would report 0 % semi-tryptic as if it were a result.",
            template_label
        ));
    }

    let obj = json
        .as_object_mut()
        .ok_or_else(|| anyhow!("Pass 2 template is not a JSON object"))?;

    obj.insert(
        "precursor_tol".to_string(),
        precursor_tol_json(&plan.ms1_window),
    );
    if let Some(tol) = plan.ms2_tolerance {
        obj.insert("fragment_tol".to_string(), tol.to_sage_json());
    }
    obj.insert(
        "isotope_errors".to_string(),
        serde_json::json!(PASS2_ISOTOPE_ERRORS),
    );
    if let Some(db) = obj.get_mut("database").and_then(|d| d.as_object_mut()) {
        db.insert(
            "fasta".to_string(),
            serde_json::json!(plan.subset_fasta.display().to_string()),
        );
        db.insert("static_mods".to_string(), serde_json::json!({}));
        db.insert("variable_mods".to_string(), serde_json::json!({}));
        db.insert("max_variable_mods".to_string(), serde_json::json!(0));
    }

    // ASSERTED, exactly as pass 1 asserts it. NEITHER PASS EVER SEARCHES WITH A
    // MODIFICATION. The templates under _dev/testing/configs/ are not trustworthy on
    // this point — 17 of them carry `static_mods {C: 57.0215}`, and any can be
    // passed with --pass2-params — so the guard lives in the code that writes the
    // effective config, not in the data it reads.
    let leftover = obj
        .get("database")
        .and_then(|d| d.get("static_mods"))
        .and_then(|m| m.as_object())
        .map(|m| m.len())
        .unwrap_or(0);
    if leftover != 0 {
        return Err(anyhow!(
            "Pass 2 would search with {leftover} static modification(s). Neither pass \
             searches with mods: a fixed Cys mod deletes the unmodified form of the \
             residue from the search space, which makes an alkylation-agnostic tool \
             silently blind to any sample alkylated another way."
        ));
    }

    obj.insert(
        "_recon_pass2_provenance".to_string(),
        serde_json::json!({
            "template": template_label,
            "template_precursor_tol": template_precursor,
            "template_fragment_tol": template_fragment,
            "template_isotope_errors": template_isotope,
            "template_static_mods": template_static,
            "template_variable_mods": template_variable,
            "applied_isotope_errors": PASS2_ISOTOPE_ERRORS,
            "mods_note": "Pass 2 searches with NO fixed and NO variable mods. It \
                          measures where trypsin cut, not what is modified, and \
                          pass 1 is an OPEN search whose modification set is \
                          unknown by design.",
            "applied_precursor_tol": precursor_tol_json(&plan.ms1_window),
            "applied_fragment_tol": plan.ms2_tolerance.map(|t| t.to_sage_json()),
            "pass1_fragment_tol": plan.pass1_fragment_tol.to_sage_json(),
            "ms1_window_delta_ppm": {
                "low": plan.ms1_window.low_ppm,
                "high": plan.ms1_window.high_ppm,
            },
            "sign_note": "precursor_tol is INVERTED relative to the delta mass it \
                          produces; measured on serum 2026-08-29. See src/pass2.rs.",
            "subset_fasta": plan.subset_fasta.display().to_string(),
            "subset_proteins": plan.subset_proteins,
        }),
    );

    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;
    let path = out_dir.join("pass2-effective-params.json");
    std::fs::write(&path, serde_json::to_string_pretty(&json)?)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(low: f64, high: f64) -> Ms1Pass2Window {
        Ms1Pass2Window {
            low_ppm: low,
            high_ppm: high,
        }
    }

    /// THE control for the module docs' measurement.
    ///
    /// If someone "fixes" the sign because it reads backwards, this fails and
    /// points at the probe that settled it.
    #[test]
    fn pass2_precursor_window_is_sign_inverted() {
        // serum: bias +2.4215, rung ±10 -> delta window -7.5785 .. +12.4215.
        let j = precursor_tol_json(&window(-7.5785, 12.4215));
        let arr = j["ppm"].as_array().unwrap();
        assert_eq!(arr[0].as_f64().unwrap(), -12.4215);
        assert_eq!(arr[1].as_f64().unwrap(), 7.5785);
    }

    /// A symmetric window is the degenerate case where the bug is INVISIBLE.
    /// Kept so nobody "verifies" the sign with a symmetric example.
    #[test]
    fn a_symmetric_window_cannot_detect_the_sign_error() {
        let j = precursor_tol_json(&window(-10.0, 10.0));
        let arr = j["ppm"].as_array().unwrap();
        assert_eq!(arr[0].as_f64().unwrap(), -10.0);
        assert_eq!(arr[1].as_f64().unwrap(), 10.0);
        // Same input under a NON-inverted rule gives the identical object,
        // which is exactly why the asymmetric test above is the real control.
    }

    fn plan(fasta: &Path) -> Pass2Plan {
        Pass2Plan {
            ms1_window: window(-7.5785, 12.4215),
            ms2_tolerance: Some(FragmentTolerance::Ppm(5.6)),
            pass1_fragment_tol: FragmentTolerance::Ppm(50.0),
            subset_fasta: fasta.to_path_buf(),
            subset_proteins: 6245,
        }
    }

    /// The template as TEXT. Production holds it as text too — `defaults::PASS2`
    /// is compiled in — so writing it to a file and reading it back would add a
    /// step the shipped binary does not take.
    fn template(semi: bool) -> String {
        let j = serde_json::json!({
            "database": {
                "enzyme": { "semi_enzymatic": semi, "cleave_at": "KR" },
                "fasta": "PLACEHOLDER"
            },
            "precursor_tol": { "ppm": [-10.0, 10.0] },
            "fragment_tol": { "ppm": [-20.0, 20.0] }
        });
        serde_json::to_string_pretty(&j).unwrap()
    }

    /// Write the Pass 2 params THE WAY `main.rs` DOES: template TEXT, then
    /// `enzyme::apply_to_params_text`, then `write_pass2_params_from_text`
    /// (`main.rs:2223-2228`).
    ///
    /// ⚠ These tests used to call `write_pass2_params`, the path-taking wrapper
    /// nothing in the shipped binary calls. It skips the enzyme step that sits
    /// between the template and the writer, so the guards below were proved on
    /// a config production never writes. `--enzyme` has no default in
    /// production; trypsin is what every pinned number here came from.
    fn write_it(text: &str, plan: &Pass2Plan, out_dir: &Path) -> Result<PathBuf> {
        let enzyme = crate::enzyme::parse("trypsin").expect("the trypsin preset must parse");
        let text = crate::enzyme::apply_to_params_text(text, &enzyme)?;
        write_pass2_params_from_text(&text, "pass2-unit-test", plan, out_dir)
    }

    #[test]
    fn measured_values_replace_the_templates_hardcoded_ones() {
        let d = tempfile::tempdir().unwrap();
        let t = template(true);
        let out = write_it(&t, &plan(Path::new("/tmp/subset.fasta")), d.path()).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();

        // The enzyme step production runs first must land, and must leave
        // `semi_enzymatic` alone — it writes into the same `database.enzyme`.
        // `restrict` and `c_terminal` falsify: the template carries neither.
        assert_eq!(v["database"]["enzyme"]["c_terminal"], true);
        assert_eq!(v["database"]["enzyme"]["restrict"], "P");
        assert_eq!(v["database"]["enzyme"]["cleave_at"], "KR");
        assert_eq!(v["database"]["enzyme"]["semi_enzymatic"], true);

        // The template's ±10 ppm and ±20 ppm are GONE, not merely warned about.
        assert_eq!(v["precursor_tol"]["ppm"][0].as_f64().unwrap(), -12.4215);
        assert_eq!(v["fragment_tol"]["ppm"][1].as_f64().unwrap(), 5.6);
        assert_eq!(
            v["database"]["fasta"].as_str().unwrap(),
            "/tmp/subset.fasta"
        );
        // ...and what they used to be is still recoverable.
        assert_eq!(
            v["_recon_pass2_provenance"]["template_fragment_tol"]["ppm"][1]
                .as_f64()
                .unwrap(),
            20.0
        );
    }

    #[test]
    fn a_fully_tryptic_pass2_template_is_refused() {
        let d = tempfile::tempdir().unwrap();
        let t = template(false);
        let err = write_it(&t, &plan(Path::new("/tmp/s.fasta")), d.path()).unwrap_err();
        assert!(
            err.to_string().contains("semi_enzymatic"),
            "the error must name the field that is wrong, got: {err}"
        );
    }

    #[test]
    fn no_measured_ms2_leaves_the_template_fragment_tol_alone() {
        let d = tempfile::tempdir().unwrap();
        let t = template(true);
        let mut p = plan(Path::new("/tmp/s.fasta"));
        p.ms2_tolerance = None;
        let out = write_it(&t, &p, d.path()).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(v["fragment_tol"]["ppm"][1].as_f64().unwrap(), 20.0);
        assert!(v["_recon_pass2_provenance"]["applied_fragment_tol"].is_null());
    }

    /// A Da-unit Pass 1 must produce a Da-unit Pass 2. A ppm number reaching an
    /// ion trap is the unit-safety failure `calibration` also guards.
    #[test]
    fn an_ion_trap_pass1_yields_a_da_pass2_fragment_tol() {
        let d = tempfile::tempdir().unwrap();
        let t = template(true);
        let mut p = plan(Path::new("/tmp/s.fasta"));
        p.ms2_tolerance = Some(FragmentTolerance::Da(0.5));
        p.pass1_fragment_tol = FragmentTolerance::Da(1.0);
        let out = write_it(&t, &p, d.path()).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert!(
            v["fragment_tol"]["ppm"].is_null(),
            "a Da tolerance must not be written under a ppm key"
        );
        assert_eq!(v["fragment_tol"]["da"][1].as_f64().unwrap(), 0.5);
    }

    #[test]
    fn pass2_searches_with_no_mods_at_all() {
        let d = tempfile::tempdir().unwrap();
        // A template carrying BOTH kinds of mod, as all four shipped ones do.
        let j = serde_json::json!({
            "database": {
                "enzyme": { "semi_enzymatic": true, "cleave_at": "KR" },
                "fasta": "PLACEHOLDER",
                "static_mods": { "C": 57.0215 },
                "variable_mods": { "M": [15.9949] },
                "max_variable_mods": 1
            },
            "precursor_tol": { "ppm": [-10.0, 10.0] },
            "fragment_tol": { "ppm": [-20.0, 20.0] },
            "isotope_errors": [-1, 3]
        });
        let t = serde_json::to_string_pretty(&j).unwrap();
        let out = write_it(&t, &plan(Path::new("/tmp/s.fasta")), d.path()).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();

        assert_eq!(v["database"]["static_mods"], serde_json::json!({}));
        assert_eq!(v["database"]["variable_mods"], serde_json::json!({}));
        assert_eq!(v["database"]["max_variable_mods"].as_i64().unwrap(), 0);
        // The carbamidomethyl the tool exists to SURFACE must not be fixed out.
        assert!(
            v["database"]["static_mods"]["C"].is_null(),
            "a fixed Cys mod contradicts the alkylation-agnostic design"
        );
        // What the template asked for is still recoverable.
        assert_eq!(
            v["_recon_pass2_provenance"]["template_static_mods"]["C"]
                .as_f64()
                .unwrap(),
            57.0215
        );
    }

    #[test]
    fn pass2_drops_the_minus_one_isotope_offset() {
        let d = tempfile::tempdir().unwrap();
        let t = template(true);
        let out = write_it(&t, &plan(Path::new("/tmp/s.fasta")), d.path()).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(v["isotope_errors"], serde_json::json!([0, 3]));
        assert_eq!(
            v["isotope_errors"][0].as_i64().unwrap(),
            0,
            "the -1 offset the templates shipped must be gone"
        );
    }
}
