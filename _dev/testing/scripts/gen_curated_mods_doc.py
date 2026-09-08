#!/usr/bin/env python3
"""Generate docs/curated-modifications.md from the files the binary embeds.

WHY THIS IS GENERATED, NOT WRITTEN. The curated list is 99 entries. A table
typed by hand would be wrong the first time one of the four source files
changed, and nothing would catch it. This reads the same four files
`recon-tool/src/defaults.rs` embeds with `include_str!`, so the document and
the shipped binary cannot disagree without this script saying so.

Masses are computed from each entry's CF formula using the element table inside
the pinned unimod.xml, which is the same rule `curated_mods.rs` applies. The
source files carry no masses of their own.

TRIPWIRE. The entry count is asserted against 99, the number
`curated_mods_integration.rs` reports on every test run ("curated list: 99
entries, 0 skipped"). A mismatch is a hard stop, not a warning.

Run from anywhere:
    python3 _dev/testing/scripts/gen_curated_mods_doc.py
"""
import re
import sys
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
MODS_DIR = ROOT / "recon-tool/resources/mods"
UNIMOD = ROOT / "recon-tool/resources/unimod.xml"
OUT = ROOT / "docs/curated-modifications.md"

# The four files, in the order defaults.rs bundles them.
FILES = ["Mods.txt", "aListOfmods.txt", "ProteaseMods.txt", "surfactants.txt"]

EXPECTED_ENTRIES = 99

root = ET.parse(UNIMOD).getroot()
NS = f"{{{root.tag.split('}')[0].strip('{')}}}" if "}" in root.tag else ""
EL = {e.get("title"): float(e.get("mono_mass")) for e in root.iter(f"{NS}elem")}


def cf_mass(cf):
    """Monoisotopic mass from a MetaMorpheus CF string, or None on an unknown element."""
    total = 0.0
    for sym, n in re.findall(r"([A-Z][a-z]?(?:\[\d+\])?)\s*(-?\d*)", cf):
        if not sym:
            continue
        if sym not in EL:
            return None
        total += EL[sym] * (int(n) if n not in ("", "-") else 1)
    return total


def parse(path):
    """Split a MetaMorpheus mod file into blocks of two-letter tagged lines."""
    out, cur = [], {}
    for line in open(path, encoding="utf-8-sig"):
        if line.strip() == "//":
            if cur:
                out.append(cur)
                cur = {}
            continue
        m = re.match(r"^([A-Z]{2})\s\s+(.*)$", line.rstrip("\n"))
        if m:
            cur.setdefault(m.group(1), m.group(2).strip())
    if cur:
        out.append(cur)
    return out


entries = []
for fn in FILES:
    for e in parse(MODS_DIR / fn):
        mass = cf_mass(e.get("CF", ""))
        if mass is None:
            sys.exit(f"HARD STOP: no computable mass for {e.get('ID')!r} CF={e.get('CF')!r}")
        e["mass"] = mass
        e["file"] = fn
        entries.append(e)

if len(entries) != EXPECTED_ENTRIES:
    sys.exit(
        f"HARD STOP: parsed {len(entries)} entries, expected {EXPECTED_ENTRIES}. "
        "Either a source file changed or this parser drifted. Do not publish "
        "the document until this is understood."
    )

entries.sort(key=lambda e: e["mass"])
by_cat = Counter(e.get("MT", "(none)") for e in entries)
by_file = Counter(e["file"] for e in entries)


def esc(s):
    return s.replace("|", "\\|")


lines = []
w = lines.append
w("# Curated modifications")
w("")
w("This is the modification list `recon` uses to name a delta mass and to say")
w("which residues can carry it. It is compiled into the binary, so a release")
w("needs no extra file to annotate a peak.")
w("")
w(f"**{len(entries)} entries.** Generated from the four files the binary embeds.")
w("Do not edit this document by hand: it is produced by")
w("`_dev/testing/scripts/gen_curated_mods_doc.py`, which reads those same files.")
w("")
w("## Where the list comes from")
w("")
w("The list is a dated snapshot of the curated modification files from")
w("[MetaMorpheus](https://github.com/smith-chem-wisc/MetaMorpheus) (MIT licence),")
w("taken at commit `7e453540`. We adopted it rather than using all of Unimod")
w("because Unimod is a catalogue of everything ever reported, while this list is")
w("a working set with acceptor residues already curated. Attribution and the one")
w("modified file are recorded in `THIRD_PARTY_LICENSES.md`.")
w("")
w("Masses are not stored in the source files. We compute each one from the")
w("entry's chemical formula using the element table in the bundled `unimod.xml`,")
w("which is the same rule the tool applies at run time.")
w("")
w("| Source file | Entries |")
w("|---|---:|")
for fn in FILES:
    w(f"| `{fn}` | {by_file[fn]} |")
w("")
w("## Categories")
w("")
w("The category is MetaMorpheus's own `MT` field, carried through unchanged.")
w("It is an inherited label, not a measurement `recon` makes.")
w("")
w("| Category | Entries |")
w("|---|---:|")
for cat, n in by_cat.most_common():
    w(f"| {esc(cat)} | {n} |")
w("")
w("## The full list")
w("")
w("Sorted by monoisotopic mass, the order the tool holds them in. `Sites` is the")
w("set of residues the entry accepts; `X` means any residue, and the position")
w("column then carries the restriction (for example a protein N-terminus).")
w("")
w("| Δ mass (Da) | Name | Sites | Position | Category | Formula |")
w("|---:|---|---|---|---|---|")
for e in entries:
    w(
        f"| {e['mass']:.4f} | {esc(e.get('ID', ''))} | {esc(e.get('TG', ''))} "
        f"| {esc(e.get('PP', ''))} | {esc(e.get('MT', ''))} | `{esc(e.get('CF', ''))}` |"
    )
w("")

OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_text("\n".join(lines), encoding="utf-8")
print(f"element table: {len(EL)} elements from {UNIMOD.name}")
print(f"entries: {len(entries)} (expected {EXPECTED_ENTRIES}) — OK")
print(f"by file: {dict(by_file)}")
print(f"wrote {OUT.relative_to(ROOT)} ({OUT.stat().st_size} bytes)")
