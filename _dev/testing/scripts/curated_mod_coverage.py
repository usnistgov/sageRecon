#!/usr/bin/env python3
"""Can MetaMorpheus's curated mod list replace Unimod for annotation?

Masses are computed from each entry's CF using the element table inside our pinned
unimod.xml (authoritative local source, not recalled constants), then matched
against our detected peaks.
"""
import re, json, html
import xml.etree.ElementTree as ET
from pathlib import Path
from collections import Counter

# parents[2] is _dev/ (this file sits at _dev/testing/scripts/); parents[3] is
# the repo root. The curated mod files and unimod.xml are shipped product data
# and live with the crate, so they resolve from the root, not from _dev.
REPO = Path(__file__).resolve().parents[2]
ROOT = Path(__file__).resolve().parents[3]
MM = ROOT / "recon-tool/resources/mods"
root = ET.parse(ROOT / "recon-tool/resources/unimod.xml").getroot()
NS = f"{{{root.tag.split('}')[0].strip('{')}}}" if "}" in root.tag else ""

EL = {}
for e in root.iter(f"{NS}elem"):
    EL[e.get("title")] = float(e.get("mono_mass"))
print(f"element table from unimod.xml: {len(EL)} entries "
      f"(H={EL.get('H')}, C={EL.get('C')}, O={EL.get('O')})")

def cf_mass(cf):
    total = 0.0
    for tok in re.findall(r"([A-Z][a-z]?(?:\[\d+\])?)\s*(-?\d*)", cf.replace(" ", " ")):
        sym, n = tok
        if not sym: continue
        if sym not in EL: return None
        total += EL[sym] * (int(n) if n not in ("", "-") else 1)
    return total

def parse(path):
    out, cur = [], {}
    for line in open(path, encoding="utf-8-sig"):
        if line.strip() == "//":
            if cur: out.append(cur); cur = {}
            continue
        m = re.match(r"^([A-Z]{2})\s\s+(.*)$", line.rstrip("\n"))
        if m: cur.setdefault(m.group(1), m.group(2).strip())
    if cur: out.append(cur)
    return out

entries = []
for f in ["Mods.txt", "aListOfmods.txt", "ProteaseMods.txt", "surfactants.txt"]:
    for e in parse(MM / f):
        m = cf_mass(e.get("CF", ""))
        if m is None:
            print(f"  !! could not compute mass for {e.get('ID')} CF={e.get('CF')!r}")
            continue
        e["mass"] = m
        entries.append(e)
print(f"curated entries with computable mass: {len(entries)}")
print("  spot check:", [(e["ID"], round(e["mass"], 5)) for e in entries
                        if e["ID"].startswith(("Carbamidomethyl on C", "Oxidation on M"))])

TOL = 0.01
for fkey in ["serum", "bcell", "b1906"]:
    peaks = json.loads((REPO / f"testing/recon-output/full-run/{fkey}.json").read_text())["mod_discovery"]["peaks"]
    nz = sorted((p for p in peaks if abs(p["delta_mass"]) >= 0.1), key=lambda p: -p["count"])
    cov = Counter()
    print(f"\n===== {fkey}: top 12 non-zero peaks vs the curated list =====")
    for p in nz[:12]:
        hits = [e for e in entries if abs(e["mass"] - p["delta_mass"]) <= TOL]
        uni = [html.unescape(a["name"]) for a in (p.get("annotations") or [])]
        cats = sorted({e.get("MT", "?") for e in hits})
        names = sorted({e["ID"] for e in hits})[:3]
        cov["covered" if hits else "not in curated list"] += 1
        print(f"  {p['delta_mass']:+9.4f} {p['count']:6d}  unimod={str(uni)[:34]:34s} "
              f"curated={('; '.join(names))[:44]:44s} {cats if cats else ''}")
    print("  ", dict(cov))
