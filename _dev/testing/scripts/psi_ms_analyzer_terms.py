#!/usr/bin/env python3
"""Derive the mass-analyzer term set from the HUPO-PSI PSI-MS CV, and check that
recon's fragment-tolerance table covers it exactly.

WHY THIS SCRIPT EXISTS. The set of mass analyzers a converter may legally write
into an mzML is defined by the PSI-MS controlled vocabulary, not by any one
converter and not by any Rust crate's generated copy of it. For the methods
write-up the set has to be derived from the spec itself, from a pinned snapshot,
by a script anyone can re-run. Reading the terms off a web page or a crate is
exactly the "a summary is not a source" failure AGENTS.md forbids — and it did
fail here: a first pass that read the OBO through a summariser returned 6 of the
7 direct children of MS:1000443 and silently omitted `orbitrap`.

WHAT IT DOES
  1. Verifies the pinned snapshot (data-version + SHA256).
  2. Parses every [Term] stanza and builds the `is_a` graph.
  3. Walks MS:1000443 "mass analyzer type" transitively.
  4. Emits the descendant set, flagging obsolete terms.
  5. TRIPWIRE: cross-checks against MASS_ANALYZER_TERMS in
     recon-tool/src/mzml.rs. Hard-stops if the CV holds an analyzer recon does
     not classify, if recon names a term absent from the CV, or if a name drifts.

Usage:  python3 testing/scripts/psi_ms_analyzer_terms.py [--markdown]
"""

import hashlib
import re
import sys
from pathlib import Path

# parents[2] is _dev/ (this file sits at _dev/testing/scripts/); parents[3] is the
# repo root. Development material lives under _dev/, the crate does not, so the
# two are resolved separately.
DEV = Path(__file__).resolve().parents[2]
ROOT = Path(__file__).resolve().parents[3]
OBO = DEV / "reference-notes/psi-ms-CV/psi-ms.obo"
RUST = ROOT / "recon-tool/src/mzml.rs"

# --- the pin ---------------------------------------------------------------
# Snapshot taken 2026-08-28 from
#   https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo
# Re-running against a newer CV is a change-regenerate: it needs an explicit
# request and a downstream-impact trace (AGENTS.md, "Reproducibility is locked").
PINNED_DATA_VERSION = "4.1.259"
PINNED_SHA256 = "4a615f3bf11685c7225b83bf0e182fc7591065dc5c241c22e5777fb3ccc7b532"

ROOT = "MS:1000443"  # mass analyzer type


def load_obo(path):
    text = path.read_text(encoding="utf-8", errors="replace")
    header = text.split("[Term]", 1)[0]
    data_version = re.search(r"^data-version:\s*(\S+)", header, re.M)
    date = re.search(r"^date:\s*(.+)$", header, re.M)
    terms = {}
    for stanza in text.split("\n[")[0:]:
        if not stanza.startswith("Term]"):
            continue
        body = stanza[len("Term]"):]
        tid = re.search(r"^id:\s*(\S+)", body, re.M)
        if not tid:
            continue
        name = re.search(r"^name:\s*(.+)$", body, re.M)
        parents = re.findall(r"^is_a:\s*(MS:\d+)", body, re.M)
        obsolete = re.search(r"^is_obsolete:\s*true", body, re.M) is not None
        defn = re.search(r'^def:\s*"(.*?)"\s*\[', body, re.M | re.S)
        terms[tid.group(1)] = {
            "id": tid.group(1),
            "name": name.group(1).strip() if name else "",
            "parents": parents,
            "obsolete": obsolete,
            "def": (defn.group(1).strip() if defn else ""),
        }
    return terms, (data_version.group(1) if data_version else "?"), (date.group(1).strip() if date else "?")


def descendants(terms, root):
    """Every term reachable upward-to-`root` through is_a, transitively."""
    children = {}
    for t in terms.values():
        for p in t["parents"]:
            children.setdefault(p, []).append(t["id"])
    out, stack, seen = [], list(children.get(root, [])), set()
    while stack:
        tid = stack.pop()
        if tid in seen:
            continue
        seen.add(tid)
        out.append(tid)
        stack.extend(children.get(tid, []))
    return sorted(out)


def rust_table(path):
    """Parse MASS_ANALYZER_TERMS out of the Rust source."""
    text = path.read_text(encoding="utf-8")
    block = re.search(
        r"pub const MASS_ANALYZER_TERMS:\s*&\[MassAnalyzerTerm\]\s*=\s*&\[(.*?)\n\];",
        text, re.S)
    if not block:
        sys.exit("FAIL: could not find MASS_ANALYZER_TERMS in " + str(path))
    entries = re.findall(
        r'accession:\s*"([^"]+)"\s*,\s*name:\s*"([^"]+)"\s*,\s*class:\s*AnalyzerClass::(\w+)',
        block.group(1))
    return {acc: {"name": name, "class": cls} for acc, name, cls in entries}


# Bucket -> (unit, Rust constant name). Asserted against the Rust source below:
# a class in MASS_ANALYZER_TERMS but not here is a hard failure, so this cannot
# silently fall behind the code.
CLASS_TOLERANCE = {
    "Orbitrap":     ("ppm", "ORBITRAP_MS2_HALF_WIDTH_PPM"),
    "AstralTof":    ("ppm", "ASTRAL_MS2_HALF_WIDTH_PPM"),
    "LegacyTof":    ("ppm", "LEGACY_TOF_MS2_HALF_WIDTH_PPM"),
    "IonTrap":      ("Da",  "ION_TRAP_MS2_HALF_WIDTH_DA"),
    "Unclassified": (None,  "UNKNOWN_MS2_FALLBACK_PPM"),
}

# Typical real-world MS2 error per bucket, and why the pass-1 window sits where
# it does. Curated in reference-notes/analyzer-tolerances/.
#
# NOTE THE PHILOSOPHY: these are LOOSE pass-1 windows, not performance figures.
# Pass 1 must assume the instrument could be well out of calibration, so each
# window sits far above typical performance rather than at it.
CLASS_BASIS = {
    "Orbitrap": "CURATED. Typical 1-5 ppm, up to ~20 ppm poorly calibrated; "
                "window set generously above the 20 ppm high-res ceiling.",
    "AstralTof": "CURATED. <5 ppm RMS drift over 24h external cal, ~3 ppm internal. "
                 "Same bucket as Orbitrap: Astral is high-res, not a legacy QTOF.",
    "LegacyTof": "CURATED. Typical ~10-30 ppm; 30 ppm is already a common default, "
                 "so the pass-1 window is set well above it.",
    "IonTrap": "CURATED. 0.3-0.8 Da typical at unit resolution. A ppm figure is "
               "meaningless here.",
    "Unclassified": "FALLBACK. No bucket. recon does not halt: it searches at the "
                    "fallback window and reports that it assumed.",
}

# Which buckets the curated source actually named, vs assigned here by parentage.
CURATED_ACCESSIONS = {"MS:1000484", "MS:1003379", "MS:1000084",
                      "MS:1000082", "MS:1000078", "MS:1000083"}


def rust_constants(path):
    text = path.read_text(encoding="utf-8")
    out = {}
    for name in ("ORBITRAP_MS2_HALF_WIDTH_PPM", "ASTRAL_MS2_HALF_WIDTH_PPM",
                 "LEGACY_TOF_MS2_HALF_WIDTH_PPM", "ION_TRAP_MS2_HALF_WIDTH_DA",
                 "UNKNOWN_MS2_FALLBACK_PPM"):
        m = re.search(rf"pub const {name}:\s*f64\s*=\s*([0-9.]+);", text)
        if not m:
            sys.exit(f"FAIL: constant {name} not found in {path}")
        out[name] = m.group(1).rstrip(".")
    return out


def tolerance_for(cls, consts):
    unit, const = CLASS_TOLERANCE[cls]
    if unit is None:
        return f"fallback +/-{consts[const]} ppm"
    return f"+/-{consts[const]} {unit}"


def emit_markdown(terms, live, rust, consts, data_version, date, digest):
    print("| CV accession | CV name | bucket | pass-1 MS2 `fragment_tol` | bucket from |")
    print("|---|---|---|---|---|")
    for tid in live:
        cls = rust[tid]["class"]
        src = "curated" if tid in CURATED_ACCESSIONS else "extended here"
        print(f"| `{tid}` | {terms[tid]['name']} | {cls} | "
              f"{tolerance_for(cls, consts)} | {src} |")
    print()
    print("| bucket | pass-1 tolerance | typical real-world MS2 error, and rationale |")
    print("|---|---|---|")
    for cls in ["Orbitrap", "AstralTof", "LegacyTof", "IonTrap", "Unclassified"]:
        print(f"| {cls} | {tolerance_for(cls, consts)} | {CLASS_BASIS[cls]} |")
    print()
    print(f"Derived from HUPO-PSI psi-ms.obo data-version {data_version} "
          f"(released {date}), sha256 `{digest}`,")
    print("by `testing/scripts/psi_ms_analyzer_terms.py`.")


def main():
    if not OBO.exists():
        sys.exit(f"FAIL: pinned CV snapshot missing: {OBO}")

    digest = hashlib.sha256(OBO.read_bytes()).hexdigest()
    terms, data_version, date = load_obo(OBO)

    print("PSI-MS controlled vocabulary — mass analyzer terms")
    print("=" * 78)
    print(f"  source      : HUPO-PSI psi-ms-CV, psi-ms.obo")
    print(f"  data-version: {data_version}")
    print(f"  released    : {date}")
    print(f"  sha256      : {digest}")
    print(f"  root term   : {ROOT} {terms.get(ROOT, {}).get('name', '?')}")

    failures = []
    if data_version != PINNED_DATA_VERSION:
        failures.append(f"data-version drift: pinned {PINNED_DATA_VERSION}, found {data_version}")
    if digest != PINNED_SHA256:
        failures.append(f"sha256 drift: pinned {PINNED_SHA256}, found {digest}")
    print(f"  pin         : {'OK' if not failures else 'MISMATCH'}")
    print()

    kids = descendants(terms, ROOT)
    live = [k for k in kids if not terms[k]["obsolete"]]
    dead = [k for k in kids if terms[k]["obsolete"]]

    rust = rust_table(RUST)
    consts = rust_constants(RUST)

    unknown_classes = sorted({v["class"] for v in rust.values()} - set(CLASS_TOLERANCE))
    if unknown_classes:
        failures.append("Rust declares analyzer classes this script has no tolerance "
                        "mapping for: " + ", ".join(unknown_classes))

    if "--markdown" in sys.argv:
        if failures:
            print("RESULT: FAIL"); [print("  - " + f) for f in failures]; sys.exit(1)
        emit_markdown(terms, live, rust, consts, data_version, date, digest)
        return

    print(f"{'accession':<13} {'CV name':<52} {'recon class':<14}")
    print("-" * 82)
    for tid in live:
        cls = rust.get(tid, {}).get("class", "*** UNCLASSIFIED ***")
        print(f"{tid:<13} {terms[tid]['name']:<52} {cls:<14}")
    print("-" * 82)
    print(f"{len(live)} live descendants of {ROOT}"
          + (f", {len(dead)} obsolete (excluded)" if dead else ", 0 obsolete"))
    for tid in dead:
        print(f"  obsolete: {tid} {terms[tid]['name']}")
    print()

    # --- tripwires ---------------------------------------------------------
    missing = [t for t in live if t not in rust]
    if missing:
        failures.append(
            "CV holds analyzer terms recon does not classify: "
            + ", ".join(f"{t} ({terms[t]['name']})" for t in missing))

    extra = [a for a in rust if a not in terms]
    if extra:
        failures.append("recon names accessions absent from the CV: " + ", ".join(extra))

    not_analyzer = [a for a in rust if a in terms and a not in live and a != ROOT]
    if not_analyzer:
        failures.append(
            "recon classifies terms that are NOT mass analyzers: "
            + ", ".join(f"{a} ({terms[a]['name']})" for a in not_analyzer))

    drift = [f"{a}: CV '{terms[a]['name']}' vs recon '{rust[a]['name']}'"
             for a in rust if a in terms and terms[a]["name"] != rust[a]["name"]]
    if drift:
        failures.append("name drift between CV and recon: " + "; ".join(drift))

    print("=" * 78)
    if failures:
        print("RESULT: FAIL")
        for f in failures:
            print("  - " + f)
        sys.exit(1)
    print(f"RESULT: PASS — recon classifies all {len(live)} CV mass analyzer terms, "
          f"names match, no extras.")


if __name__ == "__main__":
    main()
