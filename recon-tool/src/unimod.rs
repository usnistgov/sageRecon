//! Unimod XML parser for modification annotation.
//!
//! Parses the Unimod XML database and provides mass-based lookup
//! for annotating delta masses with known modifications.

use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Decode the five XML predefined entities in an attribute value.
///
/// Unimod attribute values arrive escaped and were previously stored raw, so
/// `Lys-&gt;Allysine` reached the JSON verbatim. 381 titles in the pinned
/// unimod.xml carry an entity. In HTML that renders correctly by accident; in
/// JSON it is simply wrong. Longstanding since Phase 3, not a regression.
///
/// Numeric character references are NOT handled -- the pinned unimod.xml
/// contains none. If one ever appears it survives as literal text rather than
/// being silently mangled.
fn unescape_xml(value: &str) -> String {
    if !value.contains('&') {
        return value.to_string();
    }
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        // `&amp;` LAST, so "&amp;gt;" becomes "&gt;" and not ">".
        .replace("&amp;", "&")
}

/// Default match tolerance in Daltons (matches PTM-Shepherd default)
pub const DEFAULT_MATCH_TOLERANCE_DA: f64 = 0.01;

/// A single modification specificity (site + position)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModSpecificity {
    /// Amino acid site (e.g., "M", "K", "N-term")
    pub site: String,
    /// Position constraint (e.g., "Anywhere", "Any N-term", "Protein N-term")
    pub position: String,
    /// Classification for this specificity
    pub classification: String,
    /// Whether this specificity is hidden in Unimod UI
    pub hidden: bool,
}

/// A modification entry from Unimod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnimodEntry {
    /// Unimod record ID
    pub record_id: u32,
    /// Short title (e.g., "Oxidation", "Acetyl")
    pub title: String,
    /// Full descriptive name
    pub full_name: String,
    /// Monoisotopic mass delta in Daltons
    pub mono_mass: f64,
    /// Average mass delta in Daltons
    pub avge_mass: f64,
    /// Elemental composition string
    pub composition: String,
    /// All specificities (sites + positions)
    pub specificities: Vec<ModSpecificity>,
}

impl UnimodEntry {
    /// Get unique non-hidden sites as a sorted vector
    pub fn sites(&self) -> Vec<String> {
        let mut sites: Vec<String> = self
            .specificities
            .iter()
            .filter(|s| !s.hidden)
            .map(|s| s.site.clone())
            .collect();
        sites.sort();
        sites.dedup();
        sites
    }

    /// Get the primary classification (first non-hidden specificity, or first hidden if all hidden)
    pub fn classification(&self) -> String {
        // First try non-hidden specificities
        self.specificities
            .iter()
            .filter(|s| !s.hidden)
            .map(|s| s.classification.clone())
            .next()
            // Fall back to any specificity (including hidden) - important for AA substitutions
            // which are all marked hidden="1" in Unimod
            .or_else(|| self.specificities.first().map(|s| s.classification.clone()))
            .unwrap_or_else(|| "Unknown".to_string())
    }
}

/// A match result from Unimod lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnimodMatch {
    /// The matched Unimod entry
    pub entry: UnimodEntry,
    /// Mass error in Daltons (observed - theoretical)
    pub mass_error_da: f64,
    /// DEPRECATED — do NOT consume. Computed with the old 1000-Da shortcut
    /// (`mass_error_da / 1000 × 1e6`), correct only at exactly 1000 Da. All live
    /// callers now recompute true ppm at the peak's actual m/z via
    /// `mod_discovery::ppm_at_mz` using `Peak.representative_mz`. Kept only so the
    /// struct stays serialization-stable; carries a knowingly-wrong number for any
    /// mass ≠ 1000 Da. See NOTES "Deferred enhancements" (mass_error_ppm bug).
    pub mass_error_ppm: f64,
}

/// Unimod database with mass-indexed lookup
#[derive(Debug)]
pub struct UnimodDb {
    /// All entries, sorted by mono_mass for binary search
    entries: Vec<UnimodEntry>,
    /// Match tolerance in Daltons
    pub tolerance_da: f64,
    /// Monoisotopic element masses from the `<umod:elem>` block, keyed by title
    /// ("H", "C", "13C", ...). Unimod is the authority for these, so a formula
    /// summed here agrees with a Unimod mass by construction. Used by
    /// `curated_mods`, whose files carry formulas but no masses.
    elements: HashMap<String, f64>,
}

impl UnimodDb {
    /// Load the copy of Unimod compiled into the binary.
    ///
    /// This is the preferred constructor. `unimod.xml` is compiled in, so
    /// callers do not need a `--unimod` path.
    ///
    /// The parse is not cached: it runs once per invocation, like the file path
    /// route it replaces, so behaviour is identical whichever source is used.
    ///
    /// Only the `run` subcommand uses this today (main.rs). `compare-peak-assignment`,
    /// `discover`, and `signal-fate` still take a required `--unimod` path and
    /// read from disk. Migrating them to this constructor is future work.
    pub fn from_embedded() -> Result<Self> {
        Self::parse_xml(crate::defaults::UNIMOD)
            .context("failed to parse the Unimod database compiled into this binary")
    }

    pub fn from_xml(path: &Path) -> Result<Self> {
        let xml_content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read Unimod XML: {}", path.display()))?;

        Self::parse_xml(&xml_content)
    }

    /// Parse Unimod XML content
    fn parse_xml(xml: &str) -> Result<Self> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut entries: Vec<UnimodEntry> = Vec::new();
        let mut elements: HashMap<String, f64> = HashMap::new();
        let mut current_mod: Option<PartialMod> = None;
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let local_name = e.local_name();
                    let name = std::str::from_utf8(local_name.as_ref()).unwrap_or("");

                    match name {
                        "mod" => {
                            // Start of a new modification
                            let mut partial = PartialMod::default();
                            for attr in e.attributes().flatten() {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = std::str::from_utf8(&attr.value).unwrap_or("");
                                match key {
                                    "record_id" => partial.record_id = value.parse().unwrap_or(0),
                                    "title" => partial.title = unescape_xml(value),
                                    "full_name" => partial.full_name = unescape_xml(value),
                                    _ => {}
                                }
                            }
                            current_mod = Some(partial);
                        }
                        "specificity" => {
                            // Modification specificity
                            if let Some(ref mut m) = current_mod {
                                let mut spec = ModSpecificity {
                                    site: String::new(),
                                    position: String::new(),
                                    classification: String::new(),
                                    hidden: false,
                                };
                                for attr in e.attributes().flatten() {
                                    let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                    let value = std::str::from_utf8(&attr.value).unwrap_or("");
                                    match key {
                                        "site" => spec.site = value.to_string(),
                                        "position" => spec.position = value.to_string(),
                                        "classification" => spec.classification = value.to_string(),
                                        "hidden" => spec.hidden = value == "1",
                                        _ => {}
                                    }
                                }
                                m.specificities.push(spec);
                            }
                        }
                        "elem" => {
                            let mut title = String::new();
                            let mut mono = 0.0_f64;
                            for attr in e.attributes().flatten() {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = std::str::from_utf8(&attr.value).unwrap_or("");
                                match key {
                                    "title" => title = unescape_xml(value),
                                    "mono_mass" => mono = value.parse().unwrap_or(0.0),
                                    _ => {}
                                }
                            }
                            if !title.is_empty() && mono > 0.0 {
                                elements.insert(title, mono);
                            }
                        }
                        "delta" => {
                            // Mass delta
                            if let Some(ref mut m) = current_mod {
                                for attr in e.attributes().flatten() {
                                    let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                    let value = std::str::from_utf8(&attr.value).unwrap_or("");
                                    match key {
                                        "mono_mass" => m.mono_mass = value.parse().unwrap_or(0.0),
                                        "avge_mass" => m.avge_mass = value.parse().unwrap_or(0.0),
                                        "composition" => m.composition = value.to_string(),
                                        _ => {}
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let local_name = e.local_name();
                    let name = std::str::from_utf8(local_name.as_ref()).unwrap_or("");
                    if name == "mod" {
                        // End of modification - save it
                        if let Some(m) = current_mod.take() {
                            if !m.specificities.is_empty() {
                                entries.push(UnimodEntry {
                                    record_id: m.record_id,
                                    title: m.title,
                                    full_name: m.full_name,
                                    mono_mass: m.mono_mass,
                                    avge_mass: m.avge_mass,
                                    composition: m.composition,
                                    specificities: m.specificities,
                                });
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    log::warn!(
                        "XML parse error at position {}: {:?}",
                        reader.buffer_position(),
                        e
                    );
                    // Continue parsing despite errors
                }
                _ => {}
            }
            buf.clear();
        }

        // Sort by mono_mass for efficient lookup
        entries.sort_by(|a, b| a.mono_mass.partial_cmp(&b.mono_mass).unwrap());

        log::info!(
            "Loaded {} Unimod entries and {} element masses",
            entries.len(),
            elements.len()
        );

        Ok(Self {
            entries,
            tolerance_da: DEFAULT_MATCH_TOLERANCE_DA,
            elements,
        })
    }

    /// Monoisotopic element masses, keyed by Unimod's element title.
    pub fn elements(&self) -> &HashMap<String, f64> {
        &self.elements
    }

    /// Set match tolerance
    pub fn with_tolerance(mut self, tolerance_da: f64) -> Self {
        self.tolerance_da = tolerance_da;
        self
    }

    /// Find all modifications matching a delta mass within tolerance
    ///
    /// Returns all matches sorted by absolute mass error (closest first)
    pub fn find_matches(&self, delta_mass: f64) -> Vec<UnimodMatch> {
        let mut matches: Vec<UnimodMatch> = Vec::new();

        // Binary search to find starting point
        let low = delta_mass - self.tolerance_da;
        let high = delta_mass + self.tolerance_da;

        // Find first entry >= low
        let start_idx = self.entries.partition_point(|e| e.mono_mass < low);

        // Scan forward while within range
        for entry in self.entries[start_idx..].iter() {
            if entry.mono_mass > high {
                break;
            }

            let mass_error_da = delta_mass - entry.mono_mass;
            // Calculate ppm at reference mass of 1000 Da
            let mass_error_ppm = (mass_error_da / 1000.0) * 1_000_000.0;

            matches.push(UnimodMatch {
                entry: entry.clone(),
                mass_error_da,
                mass_error_ppm,
            });
        }

        // Sort by absolute mass error
        matches.sort_by(|a, b| {
            a.mass_error_da
                .abs()
                .partial_cmp(&b.mass_error_da.abs())
                .unwrap()
        });

        matches
    }

    /// Get total number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if database is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Check if a delta mass has any Unimod annotation within tolerance.
    ///
    /// This is a lightweight alternative to running full mod discovery
    /// when you only need to know if a mass is annotated.
    pub fn has_match(&self, delta_mass: f64) -> bool {
        let low = delta_mass - self.tolerance_da;
        let high = delta_mass + self.tolerance_da;

        let start_idx = self.entries.partition_point(|e| e.mono_mass < low);

        if start_idx < self.entries.len() {
            self.entries[start_idx].mono_mass <= high
        } else {
            false
        }
    }

    /// Get entry by record ID
    pub fn get_by_id(&self, record_id: u32) -> Option<&UnimodEntry> {
        self.entries.iter().find(|e| e.record_id == record_id)
    }
}

/// Partial modification during parsing
#[derive(Default)]
struct PartialMod {
    record_id: u32,
    title: String,
    full_name: String,
    mono_mass: f64,
    avge_mass: f64,
    composition: String,
    specificities: Vec<ModSpecificity>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_xml() -> &'static str {
        r#"<?xml version="1.0"?>
        <umod:unimod xmlns:umod="http://www.unimod.org/xmlns/schema/unimod_2">
           <umod:modifications>
              <umod:mod title="Oxidation" full_name="Oxidation or Hydroxylation"
                        record_id="35">
                 <umod:specificity hidden="0" site="M" position="Anywhere" classification="Post-translational"/>
                 <umod:specificity hidden="0" site="W" position="Anywhere" classification="Post-translational"/>
                 <umod:specificity hidden="1" site="C" position="Anywhere" classification="Chemical derivative"/>
                 <umod:delta mono_mass="15.994915" avge_mass="15.9994" composition="O"/>
              </umod:mod>
              <umod:mod title="Deamidated" full_name="Deamidation"
                        record_id="7">
                 <umod:specificity hidden="0" site="N" position="Anywhere" classification="Artefact"/>
                 <umod:specificity hidden="0" site="Q" position="Anywhere" classification="Artefact"/>
                 <umod:delta mono_mass="0.984016" avge_mass="0.9848" composition="H(-1) N(-1) O"/>
              </umod:mod>
              <umod:mod title="Gln->pyro-Glu" full_name="Cyclization of N-terminal glutamine"
                        record_id="28">
                 <umod:specificity hidden="0" site="Q" position="Any N-term" classification="Post-translational"/>
                 <umod:delta mono_mass="-17.026549" avge_mass="-17.0305" composition="H(-3) N(-1)"/>
              </umod:mod>
           </umod:modifications>
        </umod:unimod>"#
    }

    #[test]
    fn test_parse_sample_xml() {
        let db = UnimodDb::parse_xml(sample_xml()).unwrap();
        assert_eq!(db.len(), 3);
    }

    #[test]
    fn test_find_oxidation() {
        let db = UnimodDb::parse_xml(sample_xml()).unwrap();
        let matches = db.find_matches(15.995);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].entry.title, "Oxidation");
        assert_eq!(matches[0].entry.record_id, 35);
        assert!(matches[0].mass_error_da.abs() < 0.001);
    }

    #[test]
    fn test_find_deamidation() {
        let db = UnimodDb::parse_xml(sample_xml()).unwrap();
        let matches = db.find_matches(0.984);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].entry.title, "Deamidated");
        assert_eq!(matches[0].entry.record_id, 7);
    }

    #[test]
    fn test_find_pyroglu() {
        let db = UnimodDb::parse_xml(sample_xml()).unwrap();
        let matches = db.find_matches(-17.027);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].entry.title, "Gln->pyro-Glu");
        assert_eq!(matches[0].entry.record_id, 28);

        // Check position specificity
        let spec = &matches[0].entry.specificities[0];
        assert_eq!(spec.site, "Q");
        assert_eq!(spec.position, "Any N-term");
    }

    #[test]
    fn test_no_match() {
        let db = UnimodDb::parse_xml(sample_xml()).unwrap();
        let matches = db.find_matches(52.91); // Unknown mass

        assert!(matches.is_empty());
    }

    #[test]
    fn test_sites_method() {
        let db = UnimodDb::parse_xml(sample_xml()).unwrap();
        let oxidation = db.get_by_id(35).unwrap();

        // Should only include non-hidden sites
        let sites = oxidation.sites();
        assert!(sites.contains(&"M".to_string()));
        assert!(sites.contains(&"W".to_string()));
        assert!(!sites.contains(&"C".to_string())); // hidden
    }

    #[test]
    fn test_tolerance() {
        let db = UnimodDb::parse_xml(sample_xml())
            .unwrap()
            .with_tolerance(0.001); // Very tight tolerance

        // Should not match with tight tolerance
        let matches = db.find_matches(15.990);
        assert!(matches.is_empty());
    }
}

#[cfg(test)]
mod entity_tests {
    use super::*;

    /// Regression case with a KNOWN correct answer taken from the pinned
    /// unimod.xml itself, not from a synthetic fixture: 381 titles in that file
    /// carry an entity, and `Lys-&gt;Allysine` reached the committed reports raw.
    #[test]
    fn decodes_the_entities_unimod_actually_uses() {
        assert_eq!(unescape_xml("Lys-&gt;Allysine"), "Lys->Allysine");
        assert_eq!(unescape_xml("Gln-&gt;pyro-Glu"), "Gln->pyro-Glu");
        assert_eq!(
            unescape_xml("Glu-&gt;pyro-Glu+Methyl:2H(2)13C(1)"),
            "Glu->pyro-Glu+Methyl:2H(2)13C(1)"
        );
        // No entity: returned untouched.
        assert_eq!(unescape_xml("Oxidation"), "Oxidation");
        // `&amp;` is decoded last so a doubly-escaped value does not collapse
        // two levels in one pass.
        assert_eq!(unescape_xml("&amp;gt;"), "&gt;");
    }
}

#[cfg(test)]
mod embedded_tests {
    use super::*;

    /// THE EMBEDDED COPY MUST NOT DRIFT FROM THE COMMITTED FILE.
    ///
    /// `include_str!` takes a snapshot at compile time. If someone updates
    /// `recon-tool/resources/unimod.xml` and the binary is not rebuilt, or the
    /// file is moved, the shipped tool and the tested tool become two different
    /// tools silently. This is the same guard
    /// `defaults::tests::bundled_default_matches_committed_template` applies to
    /// the search templates.
    ///
    /// Compared entry for entry on the fields recon actually uses — not merely on
    /// the count, because a reordering or a dropped mass would hold the count and
    /// change the answers.
    #[test]
    fn embedded_unimod_matches_the_committed_file() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("recon-tool/resources/unimod.xml");
        let from_file = UnimodDb::from_xml(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let embedded = UnimodDb::from_embedded().expect("embedded Unimod must parse");

        assert_eq!(
            embedded.entries.len(),
            from_file.entries.len(),
            "embedded Unimod has {} entries, the committed file has {}",
            embedded.entries.len(),
            from_file.entries.len()
        );

        for (i, (a, b)) in embedded
            .entries
            .iter()
            .zip(from_file.entries.iter())
            .enumerate()
        {
            assert_eq!(a.record_id, b.record_id, "record_id differs at index {i}");
            assert_eq!(a.title, b.title, "title differs at index {i}");
            assert_eq!(
                a.mono_mass.to_bits(),
                b.mono_mass.to_bits(),
                "mono_mass differs for {:?}",
                a.title
            );
        }
    }
}
