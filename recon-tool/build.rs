//! Build script: bake the current git commit SHA (and a -dirty flag) into the
//! binary at compile time, exposed as env vars for `env!()` in the code.
//!
//! Why build-time, not runtime `git rev-parse`: a real user runs the recon
//! binary on their own machine, outside this git repo — a runtime lookup would
//! find no repo (or the WRONG repo) and report a misleading SHA. The commit the
//! binary was BUILT from is the honest provenance. Falls back to "unknown" when
//! git is unavailable or this isn't a checkout (e.g. a source tarball).

use std::process::Command;

fn main() {
    // Re-run this build script if HEAD moves (new commit / checkout).
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");

    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    // Dirty flag: any staged or unstaged change to tracked files. `git status
    // --porcelain` prints nothing on a clean tree. Untracked-only files do not
    // affect reproducibility of the build, so ignore them (-uno).
    let dirty = Command::new("git")
        .args(["status", "--porcelain", "-uno"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    let commit = if sha == "unknown" {
        "unknown".to_string()
    } else if dirty {
        format!("{sha}-dirty")
    } else {
        sha
    };

    println!("cargo:rustc-env=RECON_GIT_COMMIT={commit}");
}
