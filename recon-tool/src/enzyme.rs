//! The digestion rule recon applies when it classifies termini ITSELF.
//!
//! WHY THIS EXISTS. Before this module, `digestion.rs` hardcoded trypsin in four
//! places: `is_enzymatic_nterm`, `is_enzymatic_cterm`, `internal_missed_cleavages`
//! and `decoy_ragged_side`. Sage would honour any enzyme in the params template,
//! but recon's digestion report kept counting K/R. Choose Lys-C and the SEARCH
//! followed while the REPORT did not — a silently wrong number, and the digestion
//! number is the one validated against Byonic Preview and MSFragger.
//!
//! ⚠ **NOT everything in the digestion report needs this.** `compute_digestion_stats`
//! reads Sage's OWN `missed_cleavages` and `semi_enzymatic` columns, so it follows
//! the configured enzyme for free. Only recon's own protein-context classification
//! needs the rule restated here.
//!
//! # The boundary rule, taken from Sage's source, not from the docs
//!
//! `crates/sage/src/enzyme.rs` at the pinned commit does exactly this:
//!
//! ```text
//! let right = match c_terminal { true => mat.end(), false => mat.start() };
//! if sequence.as_bytes().get(right).map_or(false, |b| skip_suffix[b]) { continue; }
//! ```
//!
//! So there is ONE rule covering both enzyme families:
//!
//! * the cleavage boundary `right` is AFTER the matched residue for a C-terminal
//!   enzyme (trypsin), and AT the matched residue for an N-terminal one (Asp-N);
//! * the restriction always tests the residue **at** `right` — the first residue
//!   of the next peptide — in both cases;
//! * `get(right)` returning `None` at the sequence end means a boundary at the
//!   protein C-terminus is never suppressed by `restrict`.
//!
//! The DOCS are ambiguous about the N-terminal case ("Do not cleave if one of
//! these AAs follows the cleavage site"), which is why this was read off the
//! implementation instead.

use serde::{Deserialize, Serialize};

/// A protease specification, in the same terms Sage's config uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enzyme {
    /// Name as the user gave it. Recorded in the report; never used for logic.
    pub name: String,
    /// Residues to cleave at. Sage's `database.enzyme.cleave_at`.
    pub cleave_at: Vec<u8>,
    /// Do not cleave when one of these sits AT the boundary. Sage's `restrict`.
    pub restrict: Vec<u8>,
    /// Cleave at the C-terminus of the matched residue. Sage's `c_terminal`.
    pub c_terminal: bool,
}

impl Enzyme {
    /// Is `right` a cleavage boundary inside `seq`?
    ///
    /// `right` is an index INTO `seq`, and means "the first residue of the next
    /// peptide". Positions 0 and `seq.len()` are sequence ends, not internal
    /// boundaries; callers handle protein termini separately, because a peptide
    /// terminus that IS the protein terminus is enzymatic by definition.
    pub fn is_boundary(&self, seq: &[u8], right: usize) -> bool {
        if right == 0 || right >= seq.len() {
            return false;
        }
        let matched = if self.c_terminal {
            seq[right - 1]
        } else {
            seq[right]
        };
        if !self.cleave_at.contains(&matched) {
            return false;
        }
        !self.restrict.contains(&seq[right])
    }

    /// Count cleavage boundaries strictly INSIDE `seq` — the missed cleavages.
    ///
    /// The peptide's own two ends are excluded: they are the termini, not misses.
    pub fn internal_boundaries(&self, seq: &[u8]) -> usize {
        (1..seq.len()).filter(|&r| self.is_boundary(seq, r)).count()
    }

    /// True when this enzyme leaves the cleaved residue at the peptide's C-terminus.
    ///
    /// Used only by the decoy ragged-side PROXY, whose accuracy was measured for
    /// trypsin alone — see `digestion::decoy_ragged_side`.
    pub fn is_c_terminal(&self) -> bool {
        self.c_terminal
    }
}

/// Why a name was rejected, so the CLI can say something useful.
#[derive(Debug)]
pub enum EnzymeError {
    Unknown(String),
    NonSpecific,
    NoDigest,
    InvalidResidue { field: String, residue: char },
}

impl std::fmt::Display for EnzymeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnzymeError::Unknown(n) => write!(
                f,
                "unknown enzyme {n:?}. Known: {}.\n\
                 Or give an explicit rule as CLEAVE_AT[/RESTRICT][/n], e.g. \"KR/P\" \
                 for trypsin, \"D//n\" for an N-terminal cleaver.",
                known_names().join(", ")
            ),
            EnzymeError::NonSpecific => write!(
                f,
                "a non-specific digest has no cleavage rule, so recon cannot classify \
                 peptide termini or count missed cleavages. Sage can still search it: \
                 pass a template with cleave_at \"\" via --params."
            ),
            EnzymeError::InvalidResidue { field, residue } => write!(
                f,
                "{field} contains {residue:?}, which Sage does not accept. Valid residues \
                 are the 20 standard ones plus U and O.\n\
                 Note: Mascot's table uses the ambiguity codes B (Asx) and Z (Glx) for \
                 Asp-N, V8-DE and V8-E. Sage cannot represent them; recon's presets drop \
                 them, which changes nothing for a modern FASTA."
            ),
            EnzymeError::NoDigest => write!(
                f,
                "\"$\" means no digestion (FASTA entries used as-is), so there are no \
                 cleavage sites to report. Sage can still search it: pass a template \
                 with cleave_at \"$\" via --params."
            ),
        }
    }
}

impl std::error::Error for EnzymeError {}

/// Residues Sage will accept in `cleave_at` / `restrict`.
///
/// ⚠ **THIS IS A CRASH GUARD, NOT A STYLE CHECK.** Sage's `Enzyme::new` validates
/// with `assert!`, not a `Result`, against its own `VALID_AA` — the 20 standard
/// residues plus `U` (selenocysteine) and `O` (pyrrolysine). It does NOT accept
/// the ambiguity codes `B`, `Z`, `J` or `X`. Since A1 landing 2 put Sage in our
/// process, that assert would ABORT recon instead of printing an error. So every
/// residue is checked here first, and rejected with something a user can act on.
/// Read from `crates/sage/src/mass.rs` at the pinned commit.
const SAGE_VALID_AA: &[u8] = b"ACDEFGHIKLMNPQRSTVWYUO";

/// Preset table: the COMMON proteases, from Mascot's published enzyme list.
///
/// **SOURCE (cited, supplied by Ben 2026-09-01):**
/// <https://www.matrixscience.com/help/enzyme_help.html>, rows vendored at
/// `_dev/reference-notes/mascot-enzymes.md`.
///
/// **SCOPE, decided by Ben 2026-09-01: the big ones only.** Anything else is
/// reachable with `--enzyme "CLEAVE_AT/RESTRICT[/n]"` or the `--cleave-at` /
/// `--restrict` / `--c-terminal` flags, so the table does not chase Mascot's
/// full list.
///
/// ⚠ **TWO PROTEASES ARE BUFFER-DEPENDENT, so each ships as a PAIR.** Mascot
/// splits them and so does recon, because a single name would silently pick one
/// condition for the user:
/// * **Glu-C / V8** cleaves after E in phosphate buffer (`glu-c`, Mascot `V8-E`)
///   but after both D and E in ammonium bicarbonate (`glu-c/de`, Mascot `V8-DE`).
/// * **Asp-N** cleaves before D (`asp-n`) but before both D and E in ammonium
///   bicarbonate (`asp-n/ambic`, Mascot `Asp-N_ambic`).
///
/// Two consequences of that scope, both deliberate:
/// * **Ambiguity codes are out.** Mascot writes Asp-N as `BD` and the V8 pair as
///   `BDEZ` / `EZ`. Sage rejects `B`, `Z`, `J` and `X` outright, so `asp-n` here
///   is `D` and `glu-c` is `E`. B and Z appear in essentially no modern FASTA,
///   so the search is unchanged; the deviation is noted rather than hidden.
/// * **Multi-rule enzymes are out.** `CNBr+Trypsin`, `LysC+AspN`, `Formic_acid`
///   and `TrypsinMSIPI` need two or three rules each, mixing N- and C-terminal
///   cleavage. Sage's config carries ONE triple, so neither Sage nor recon can
///   express them at all.
const PRESETS: &[(&str, &str, &str, bool)] = &[
    // name            cleave_at  restrict  c_terminal   (Mascot: Cleave / Don't cleave / term)
    ("trypsin", "KR", "P", true),        // KR     / P / C
    ("trypsin/p", "KR", "", true),       // KR     /   / C
    ("arg-c", "R", "P", true),           // R      / P / C
    ("asp-n", "D", "", false),           // BD     /   / N   B dropped
    ("asp-n/ambic", "DE", "", false),    // Asp-N_ambic: DE / / N
    ("chymotrypsin", "FYWL", "P", true), // FYWL   / P / C
    ("cnbr", "M", "", true),             // M      /   / C
    ("lys-c", "K", "P", true),           // K      / P / C
    ("lys-c/p", "K", "", true),          // K      /   / C
    ("lys-n", "K", "", false),           // K      /   / N
    ("pepsin-a", "FL", "", true),        // FL     /   / C
    ("trypchymo", "FYWLKR", "P", true),  // FYWLKR / P / C
    ("glu-c", "E", "P", true),           // V8-E:  EZ   / P / C   Z dropped
    ("glu-c/de", "DE", "P", true),       // V8-DE: BDEZ / P / C   B,Z dropped
];

/// Preset names, for error messages and `--help`.
pub fn known_names() -> Vec<&'static str> {
    PRESETS.iter().map(|(n, _, _, _)| *n).collect()
}

/// Trypsin WITH the proline rule, for TESTS ONLY.
///
/// This is what every pinned number in this repo was produced with, so tests
/// use it as a fixture. Production code must NOT call this. The enzyme is a
/// required `--enzyme` parameter with no default — assuming trypsin would
/// silently mis-report every digestion number for any other protease.
pub fn trypsin_for_tests() -> Enzyme {
    parse("trypsin").expect("the trypsin preset must parse")
}

/// Reject residues Sage would `assert!` on. See `SAGE_VALID_AA`.
pub fn validate_residues(field: &str, residues: &[u8]) -> Result<(), EnzymeError> {
    for r in residues {
        if !SAGE_VALID_AA.contains(r) {
            return Err(EnzymeError::InvalidResidue {
                field: field.to_string(),
                residue: *r as char,
            });
        }
    }
    Ok(())
}

/// Parse a preset name, or an explicit `CLEAVE_AT[/RESTRICT][/n]` rule.
///
/// The explicit form exists so an enzyme this table does not know is still
/// reachable without editing the code: `"KR/P"` is trypsin, `"K//n"` is Lys-N.
/// A trailing `/n` means N-terminal cleavage; the default is C-terminal.
pub fn parse(spec: &str) -> Result<Enzyme, EnzymeError> {
    let lower = spec.trim().to_ascii_lowercase();

    if let Some(&(name, cleave, restrict, c_term)) = PRESETS.iter().find(|(n, _, _, _)| *n == lower)
    {
        let e = Enzyme {
            name: name.to_string(),
            cleave_at: cleave.bytes().collect(),
            restrict: restrict.bytes().collect(),
            c_terminal: c_term,
        };
        // The presets are checked too, not just user input: a typo in the table
        // above would otherwise reach Sage's assert and abort the process.
        validate_residues("cleave_at", &e.cleave_at)?;
        validate_residues("restrict", &e.restrict)?;
        return Ok(e);
    }

    // Sage's two special cases, refused with an explanation rather than a
    // misleading digestion report.
    if lower.is_empty() {
        return Err(EnzymeError::NonSpecific);
    }
    if lower == "$" {
        return Err(EnzymeError::NoDigest);
    }

    // Explicit rule: CLEAVE_AT[/RESTRICT][/n]
    let parts: Vec<&str> = spec.trim().split('/').collect();
    let cleave = parts[0].to_ascii_uppercase();
    if !cleave.is_empty() && cleave.bytes().all(|b| b.is_ascii_uppercase()) {
        let restrict = parts
            .get(1)
            .map(|s| s.to_ascii_uppercase())
            .unwrap_or_default();
        if !restrict.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err(EnzymeError::Unknown(spec.to_string()));
        }
        let c_terminal = !matches!(parts.get(2), Some(&"n") | Some(&"N"));
        let e = Enzyme {
            name: spec.trim().to_string(),
            cleave_at: cleave.bytes().collect(),
            restrict: restrict.bytes().collect(),
            c_terminal,
        };
        validate_residues("cleave_at", &e.cleave_at)?;
        validate_residues("restrict", &e.restrict)?;
        return Ok(e);
    }

    Err(EnzymeError::Unknown(spec.to_string()))
}

/// Apply this enzyme's IDENTITY to a Sage params template, returning new JSON text.
///
/// ⚠ **IDENTITY ONLY, AND THAT IS A LOCK.** Exactly three fields are written:
/// `cleave_at`, `restrict` and `c_terminal`. The tuning fields —
/// `missed_cleavages`, `min_len`, `max_len`, `semi_enzymatic` — are LEFT ALONE,
/// because Pass 1 and Pass 2 set them differently on purpose (pass 1 is
/// `missed_cleavages: 2, min_len: 7`; pass 2 is `1`, `8`, `semi_enzymatic: true`).
/// An `--enzyme` flag that quietly reset those would silently change the search
/// depth of both passes.
///
/// Applied to the template TEXT before the existing effective-params writers run,
/// so those keep their signatures and their guards.
pub fn apply_to_params_text(text: &str, enzyme: &Enzyme) -> anyhow::Result<String> {
    use anyhow::Context;
    let mut json: serde_json::Value =
        serde_json::from_str(text).context("params template is not valid JSON")?;

    let db = json
        .get_mut("database")
        .and_then(|d| d.as_object_mut())
        .ok_or_else(|| anyhow::anyhow!("params template has no `database` object"))?;

    let enz = db
        .entry("enzyme")
        .or_insert_with(|| serde_json::Value::Object(Default::default()))
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("`database.enzyme` is not an object"))?;

    enz.insert(
        "cleave_at".into(),
        serde_json::Value::String(String::from_utf8_lossy(&enzyme.cleave_at).into_owned()),
    );
    enz.insert(
        "restrict".into(),
        if enzyme.restrict.is_empty() {
            // Sage reads a missing/null restrict as "no restriction". An empty
            // string would also work; null is what its own default config uses.
            serde_json::Value::Null
        } else {
            serde_json::Value::String(String::from_utf8_lossy(&enzyme.restrict).into_owned())
        },
    );
    enz.insert(
        "c_terminal".into(),
        serde_json::Value::Bool(enzyme.c_terminal),
    );

    Ok(serde_json::to_string_pretty(&json)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_trypsin_with_the_proline_rule() {
        let e = trypsin_for_tests();
        assert_eq!(e.cleave_at, b"KR".to_vec());
        assert_eq!(e.restrict, b"P".to_vec());
        assert!(e.c_terminal);
    }

    /// The rule recon used to hardcode, restated as a boundary test.
    /// PEPTIDEKPEPTIDE: the K is followed by P, so it is NOT a boundary.
    #[test]
    fn the_proline_rule_suppresses_a_boundary() {
        let e = trypsin_for_tests();
        let seq = b"AAAKPAAA";
        assert!(!e.is_boundary(seq, 4), "K followed by P must not cleave");
        let seq2 = b"AAAKAAAA";
        assert!(e.is_boundary(seq2, 4), "K followed by A must cleave");
    }

    /// trypsin/p is the same enzyme without the restriction, so the boundary
    /// the proline rule suppressed above must now fire. This is the difference
    /// Ben named: "KR on C-term with or without P".
    #[test]
    fn strict_trypsin_cleaves_where_trypsin_does_not() {
        let e = parse("trypsin/p").unwrap();
        assert!(e.is_boundary(b"AAAKPAAA", 4));
    }

    /// An N-terminal cleaver puts the boundary AT the matched residue, not after
    /// it. Asp-N on AAADAAA cleaves before the D, i.e. at index 3, and NOT at 4.
    #[test]
    fn an_n_terminal_enzyme_cleaves_before_its_residue() {
        let e = parse("asp-n").unwrap();
        let seq = b"AAADAAA";
        assert!(e.is_boundary(seq, 3), "Asp-N cleaves before D");
        assert!(!e.is_boundary(seq, 4), "Asp-N does not cleave after D");
    }

    /// Sequence ends are not internal boundaries.
    #[test]
    fn sequence_ends_are_not_boundaries() {
        let e = trypsin_for_tests();
        let seq = b"KAAAK";
        assert!(!e.is_boundary(seq, 0));
        assert!(!e.is_boundary(seq, seq.len()));
    }

    /// Missed cleavages counted the way the hardcoded version did: internal K/R
    /// not followed by P.
    #[test]
    fn internal_boundaries_match_the_old_missed_cleavage_rule() {
        let e = trypsin_for_tests();
        // AAKAARAA: internal boundaries after K (index 3) and after R (index 6).
        assert_eq!(e.internal_boundaries(b"AAKAARAA"), 2);
        // Trailing K is the peptide C-terminus, not a missed cleavage.
        assert_eq!(e.internal_boundaries(b"AAAAK"), 0);
        // K followed by P is not a cleavage site at all.
        assert_eq!(e.internal_boundaries(b"AAKPAAR"), 0);
    }

    #[test]
    fn explicit_rules_parse() {
        let t = parse("KR/P").unwrap();
        assert_eq!(t.cleave_at, b"KR".to_vec());
        assert_eq!(t.restrict, b"P".to_vec());
        assert!(t.c_terminal);

        let n = parse("D//n").unwrap();
        assert_eq!(n.cleave_at, b"D".to_vec());
        assert!(n.restrict.is_empty());
        assert!(!n.c_terminal);
    }

    #[test]
    fn the_two_sage_special_cases_are_refused_with_a_reason() {
        assert!(matches!(parse(""), Err(EnzymeError::NonSpecific)));
        assert!(matches!(parse("$"), Err(EnzymeError::NoDigest)));
        assert!(matches!(parse("nonsense!"), Err(EnzymeError::Unknown(_))));
    }

    /// The identity lock: applying an enzyme must not touch the tuning fields.
    #[test]
    fn applying_an_enzyme_leaves_the_tuning_fields_alone() {
        let template = r#"{
          "database": {
            "enzyme": {
              "missed_cleavages": 2,
              "min_len": 7,
              "max_len": 50,
              "cleave_at": "KR",
              "restrict": "P"
            },
            "bucket_size": 8192
          },
          "chimera": true
        }"#;
        let out = apply_to_params_text(template, &parse("lys-n").unwrap()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let e = &v["database"]["enzyme"];
        // identity changed
        assert_eq!(e["cleave_at"], "K");
        assert_eq!(e["restrict"], serde_json::Value::Null);
        assert_eq!(e["c_terminal"], false);
        // tuning untouched
        assert_eq!(e["missed_cleavages"], 2);
        assert_eq!(e["min_len"], 7);
        assert_eq!(e["max_len"], 50);
        // everything else untouched
        assert_eq!(v["database"]["bucket_size"], 8192);
        assert_eq!(v["chimera"], true);
    }

    /// Trypsin here must reproduce the template it replaces, field for field.
    /// This is what makes "trypsin changes nothing" checkable.
    #[test]
    fn the_default_enzyme_reproduces_the_committed_identity() {
        let template =
            r#"{"database":{"enzyme":{"cleave_at":"KR","restrict":"P","missed_cleavages":2}}}"#;
        let out = apply_to_params_text(template, &trypsin_for_tests()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["database"]["enzyme"]["cleave_at"], "KR");
        assert_eq!(v["database"]["enzyme"]["restrict"], "P");
        assert_eq!(v["database"]["enzyme"]["c_terminal"], true);
        assert_eq!(v["database"]["enzyme"]["missed_cleavages"], 2);
    }

    /// MONOTONICITY: dropping the restriction can only ADD boundaries, never
    /// remove one. So for ANY peptide, trypsin/p counts at least as many missed
    /// cleavages as trypsin. This is the invariant behind Ben's spot check --
    /// "KR" versus "KR but not if P follows" -- and it holds per-peptide
    /// regardless of what the search returns.
    #[test]
    fn dropping_the_restriction_never_removes_a_boundary() {
        let strict = parse("trypsin").unwrap();
        let loose = parse("trypsin/p").unwrap();
        let cases: &[&[u8]] = &[
            b"PEPTIDEKPEPTIDER",
            b"AAKPAARPAA",
            b"KKKKKK",
            b"KPKPKPKP",
            b"NOCLEAVAGEHERE",
            b"AAAAAAAAAA",
            b"MKPRPKAAA",
            b"K",
            b"",
        ];
        for seq in cases {
            let a = strict.internal_boundaries(seq);
            let b = loose.internal_boundaries(seq);
            assert!(
                b >= a,
                "trypsin/p must not count fewer boundaries than trypsin on {:?}: {a} vs {b}",
                String::from_utf8_lossy(seq)
            );
        }
        // And it must be STRICTLY greater somewhere, or the two presets would be
        // the same enzyme and the spot check would prove nothing.
        assert!(
            loose.internal_boundaries(b"AAKPAARPAA") > strict.internal_boundaries(b"AAKPAARPAA"),
            "K|P and R|P must be boundaries for trypsin/p and not for trypsin"
        );
    }

    /// THE CRASH GUARD. Sage validates cleave_at with `assert!`, so an ambiguity
    /// code would abort the process now that Sage is linked in. Every preset and
    /// every user-supplied residue must be caught here first.
    #[test]
    fn ambiguity_codes_are_rejected_before_they_reach_sage() {
        for bad in ["BD", "BDEZ", "EZ", "J", "X"] {
            let r = parse(bad);
            assert!(
                matches!(r, Err(EnzymeError::InvalidResidue { .. })),
                "{bad:?} must be refused, not passed to Sage's assert"
            );
        }
        // And the residues Sage DOES allow must survive.
        assert!(parse("U").is_ok());
        assert!(parse("O").is_ok());
    }

    /// Every shipped preset must be Sage-representable. A typo in the table would
    /// otherwise only surface as a panic mid-search.
    #[test]
    fn every_preset_parses_and_is_sage_valid() {
        for name in known_names() {
            let e = parse(name).unwrap_or_else(|e| panic!("preset {name:?} failed: {e}"));
            validate_residues("cleave_at", &e.cleave_at).unwrap();
            validate_residues("restrict", &e.restrict).unwrap();
            assert!(
                !e.cleave_at.is_empty(),
                "preset {name:?} has no cleavage residues"
            );
        }
    }

    /// Spot-check the presets against the cited Mascot rows, including the two
    /// N-terminal cleavers and the restriction on the V8 pair — the two things
    /// my first pass got wrong before the source was supplied.
    #[test]
    fn presets_match_the_cited_mascot_rows() {
        let asp = parse("asp-n").unwrap();
        assert!(!asp.c_terminal, "Asp-N cleaves N-terminal");
        assert!(asp.restrict.is_empty());

        let lysn = parse("lys-n").unwrap();
        assert!(!lysn.c_terminal, "Lys-N cleaves N-terminal");

        // Mascot's V8-E is EZ / P / C. The Z is dropped; the P restriction is
        // NOT -- my first pass at this table omitted it, before Ben supplied
        // the source.
        let v8e = parse("glu-c").unwrap();
        assert_eq!(v8e.cleave_at, b"E".to_vec());
        assert_eq!(v8e.restrict, b"P".to_vec());
        assert!(v8e.c_terminal);

        let tc = parse("trypchymo").unwrap();
        assert_eq!(tc.cleave_at, b"FYWLKR".to_vec());
        assert_eq!(tc.restrict, b"P".to_vec());
    }

    /// THE BUFFER-DEPENDENT PAIRS MUST DIFFER, or shipping two names is a lie.
    ///
    /// Glu-C cleaves after E in phosphate but after D and E in ammonium
    /// bicarbonate; Asp-N cleaves before D, or before D and E in ambic. Mascot
    /// encodes each as two rows (V8-E / V8-DE, Asp-N / Asp-N_ambic) and so do we,
    /// because one name would silently pick a condition for the user.
    #[test]
    fn buffer_dependent_pairs_are_actually_different() {
        let phosphate = parse("glu-c").unwrap();
        let ambic = parse("glu-c/de").unwrap();
        assert_eq!(phosphate.cleave_at, b"E".to_vec());
        assert_eq!(ambic.cleave_at, b"DE".to_vec());
        assert_eq!(
            phosphate.restrict,
            b"P".to_vec(),
            "V8-E carries the P restriction"
        );
        assert_eq!(ambic.restrict, b"P".to_vec(), "V8-DE carries it too");
        assert!(phosphate.c_terminal && ambic.c_terminal);

        let aspn = parse("asp-n").unwrap();
        let aspn_ambic = parse("asp-n/ambic").unwrap();
        assert_eq!(aspn.cleave_at, b"D".to_vec());
        assert_eq!(aspn_ambic.cleave_at, b"DE".to_vec());
        assert!(
            !aspn.c_terminal && !aspn_ambic.c_terminal,
            "both cleave N-terminal"
        );

        // The ambic forms must be STRICTLY more permissive. The probe peptide has
        // to contain BOTH a D and an E, or the comparison proves nothing: an
        // earlier version of this test used "AAAEAAA", where glu-c and glu-c/de
        // see the same single site and the assertion failed for the right reason.
        const BOTH: &[u8] = b"AADAEAA";
        assert!(
            ambic.internal_boundaries(BOTH) > phosphate.internal_boundaries(BOTH),
            "glu-c/de must see the D that glu-c does not: {} vs {}",
            ambic.internal_boundaries(BOTH),
            phosphate.internal_boundaries(BOTH)
        );
        assert!(
            aspn_ambic.internal_boundaries(BOTH) > aspn.internal_boundaries(BOTH),
            "asp-n/ambic must see the E that asp-n does not: {} vs {}",
            aspn_ambic.internal_boundaries(BOTH),
            aspn.internal_boundaries(BOTH)
        );
    }
}
