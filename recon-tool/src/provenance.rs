//! Output provenance — makes every emitted artifact self-describing.
//!
//! Two reports from different code both saying "recon-tool 0.1.0" are
//! indistinguishable except by mtime (which lies on re-run). Embedding the git
//! commit the binary was built from makes each output provably tied to a code
//! state. Both reports carry `tool_version` and `git_commit` as flat fields.
//!
//! A `Provenance` struct with an input-file list also lived here. Its only
//! user was the removed `qc-stats` subcommand, and it was removed with it
//! (2026-09-24).
//!
//! `git_commit` is baked at BUILD time (see `build.rs`), not looked up at
//! runtime: a user runs the binary outside this repo, where a runtime lookup
//! would find no repo or the wrong one. A `-dirty` suffix means the build had
//! uncommitted tracked changes → NOT reproducible from a clean checkout.

/// Git commit the binary was built from (short SHA, `-dirty` if the build tree
/// had uncommitted tracked changes, `"unknown"` if git was unavailable).
pub const GIT_COMMIT: &str = env!("RECON_GIT_COMMIT");

/// Cargo package version.
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
