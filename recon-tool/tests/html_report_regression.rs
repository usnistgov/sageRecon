//! The committed HTML reports must equal what the CURRENT renderer produces.
//!
//! ⚠ **Until this file existed, `_dev/testing/recon-output/full-run/*.html` was an
//! UNGATED artifact.** `run_validation.py` byte-compares the JSON and never
//! looks at the HTML, and `generate_html_report` is called only from
//! `main.rs` during a real run, so nothing checked that the committed pages
//! still matched the code that renders them. A rendering change could stale
//! all four pages silently, and a reader opening one would be looking at
//! output no longer produced by the tool.
//!
//! This closes that gap the cheap way: the inputs are committed JSON, so the
//! check needs no mzML, no FASTA and no Sage run.
//!
//! **To regenerate after a deliberate rendering change**, set
//! `RECON_REGENERATE_HTML=1` and run this test. It then WRITES each rendered
//! page over the committed file instead of asserting. Always read `git diff` on
//! the four pages afterwards: re-rendering is a presentation-only step, so any
//! line that is not a presentation change is a defect, not an update.

use recon_tool::report::{generate_html_report, Pass2Report, ReconReport};
use std::path::{Path, PathBuf};

/// The four committed reports, by output name.
const FILES: &[&str] = &["b1906", "bcell", "liver", "serum"];

fn full_run_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("recon-tool has a parent directory")
        .join("_dev/testing/recon-output/full-run")
}

fn regenerating() -> bool {
    std::env::var("RECON_REGENERATE_HTML").is_ok_and(|v| v == "1")
}

/// Where the two pages first diverge: the line, and a WINDOW around the exact
/// character, from each side.
///
/// ⚠ The window matters as much as the line number. A bare `assert_eq!` on a
/// 16 KB page prints two walls of HTML; but so does printing the whole differing
/// line, because this renderer puts an entire table on ONE line — the cut table
/// alone is over 3 KB. Measured while falsifying this test: the line number was
/// right and the message was still unreadable. Show the neighbourhood instead.
fn first_difference(want: &str, got: &str) -> String {
    const CONTEXT: usize = 60;

    let at = want
        .bytes()
        .zip(got.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| want.len().min(got.len()));

    if at == want.len() && at == got.len() {
        return "no byte differs, but the strings compared unequal".to_string();
    }

    let line = want[..at].matches('\n').count() + 1;
    let col = at - want[..at].rfind('\n').map_or(0, |i| i + 1) + 1;

    // Clamp to char boundaries so a multi-byte character cannot panic the slice.
    let window = |s: &str| {
        let lo = (at.saturating_sub(CONTEXT)..=at)
            .find(|i| s.is_char_boundary(*i))
            .unwrap_or(at);
        let hi = ((at + CONTEXT).min(s.len())..=s.len())
            .find(|i| s.is_char_boundary(*i))
            .unwrap_or(s.len());
        s[lo..hi.max(lo)].replace('\n', "\\n")
    };

    format!(
        "first difference at line {line}, column {col} (byte {at}); \
         committed {} bytes, rendered {} bytes\n  \
         committed: ...{}...\n  rendered : ...{}...",
        want.len(),
        got.len(),
        window(want),
        window(got)
    )
}

#[test]
fn committed_html_equals_what_the_current_renderer_produces() {
    let dir = full_run_dir();
    let mut checked = 0usize;

    for name in FILES {
        let json = dir.join(format!("{name}.json"));
        let pass2_json = dir.join(format!("{name}_pass2.json"));
        let html = dir.join(format!("{name}.html"));

        if !json.exists() || !html.exists() {
            eprintln!("SKIP: {name} report not present at {}", json.display());
            continue;
        }

        let report: ReconReport = serde_json::from_str(
            &std::fs::read_to_string(&json).expect("committed report is readable"),
        )
        .unwrap_or_else(|e| panic!("{name}.json does not deserialize into ReconReport: {e}"));

        // Pass 2 is optional in the renderer, and the digestion section says so
        // plainly when it is absent. Pass it when the file is there so the test
        // exercises the same shape a real run produces.
        let pass2: Option<Pass2Report> = if pass2_json.exists() {
            Some(
                serde_json::from_str(
                    &std::fs::read_to_string(&pass2_json).expect("committed pass2 is readable"),
                )
                .unwrap_or_else(|e| {
                    panic!("{name}_pass2.json does not deserialize into Pass2Report: {e}")
                }),
            )
        } else {
            None
        };

        let rendered = generate_html_report(&report, pass2.as_ref());

        if regenerating() {
            std::fs::write(&html, &rendered).expect("committed HTML is writable");
            eprintln!("REGENERATED {}", html.display());
            checked += 1;
            continue;
        }

        let committed = std::fs::read_to_string(&html).expect("committed HTML is readable");
        assert!(
            committed == rendered,
            "{name}.html is not what the current renderer produces.\n{}\n\n\
             If the rendering change was deliberate, regenerate with:\n  \
             RECON_REGENERATE_HTML=1 cargo test --test html_report_regression\n\
             then READ `git diff` on the four pages before committing.",
            first_difference(&committed, &rendered)
        );
        checked += 1;
    }

    if checked == 0 {
        eprintln!("SKIP: no committed full-run reports found; nothing gated");
    } else {
        eprintln!("checked {checked} committed HTML report(s)");
    }
}
