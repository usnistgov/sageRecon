//! Output provenance — makes every emitted artifact self-describing.
//!
//! Two reports from different code both saying "recon-tool 0.1.0" are
//! indistinguishable except by mtime (which lies on re-run). Embedding the git
//! commit the binary was built from + the input file identities makes each
//! output provably tied to a code state and a set of inputs — so testing
//! artifacts stay unique and traceable across code updates.
//!
//! `git_commit` is baked at BUILD time (see `build.rs`), not looked up at
//! runtime: a user runs the binary outside this repo, where a runtime lookup
//! would find no repo or the wrong one. A `-dirty` suffix means the build had
//! uncommitted tracked changes → NOT reproducible from a clean checkout.

use serde::{Deserialize, Serialize};

/// Git commit the binary was built from (short SHA, `-dirty` if the build tree
/// had uncommitted tracked changes, `"unknown"` if git was unavailable).
pub const GIT_COMMIT: &str = env!("RECON_GIT_COMMIT");

/// Cargo package version.
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Provenance block embedded in every JSON output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    /// Tool version (Cargo pkg version).
    pub tool_version: String,
    /// Git commit the binary was built from (short SHA, may carry `-dirty`).
    pub git_commit: String,
    /// UTC timestamp when this output was generated (RFC3339).
    pub generated_at: String,
    /// Input files that produced this output, by role (e.g. "mzml", "tsv",
    /// "unimod", "closed_tsv"). Values are the paths as invoked.
    pub inputs: Vec<ProvenanceInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceInput {
    pub role: String,
    pub path: String,
}

impl Provenance {
    /// Build a provenance block. `generated_at` must be passed in by the caller
    /// (the library does not read the clock itself, keeping this deterministic
    /// and testable — the CLI supplies `chrono::Utc::now()`).
    pub fn new(generated_at: String, inputs: Vec<(&str, &str)>) -> Self {
        Provenance {
            tool_version: TOOL_VERSION.to_string(),
            git_commit: GIT_COMMIT.to_string(),
            generated_at,
            inputs: inputs
                .into_iter()
                .map(|(role, path)| ProvenanceInput {
                    role: role.to_string(),
                    path: path.to_string(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_is_populated() {
        let p = Provenance::new(
            "2026-07-16T00:00:00Z".to_string(),
            vec![("mzml", "a.mzML.gz"), ("tsv", "r.sage.tsv")],
        );
        // git_commit is baked at build time — never empty (at minimum "unknown").
        assert!(!p.git_commit.is_empty(), "git_commit must be baked in");
        assert!(!p.tool_version.is_empty());
        assert_eq!(p.inputs.len(), 2);
        assert_eq!(p.inputs[0].role, "mzml");
        assert_eq!(p.inputs[1].path, "r.sage.tsv");
        // round-trips through JSON
        let json = serde_json::to_string(&p).unwrap();
        let back: Provenance = serde_json::from_str(&json).unwrap();
        assert_eq!(back.git_commit, p.git_commit);
    }
}
