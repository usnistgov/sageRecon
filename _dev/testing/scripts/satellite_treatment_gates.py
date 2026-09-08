#!/usr/bin/env python3
"""Regenerate satellite-memo.md section 1's table on the FIXED build.

The memo's table (2026-08-24) was computed before the peak-assignment fix, so its
baseline row reads 47/25/31/16. The committed tier-gates file on the fixed build
reads 55/20/15/8. The table must be re-measured, not carried forward.

Treatments, matching the memo's names:
  satellite demote  = option B. Peak stays in the table, forced BELOW the floor.
  satellite fold    = SIMULATION ONLY, to reproduce the memo row. Satellite count
                      is added to the +57 parent and the satellite peak removed.
                      Folding stays disabled-by-design; this is measurement, not
                      a proposal.
  forest removed    = the +/-1 and +/-2 Da region peaks dropped. Regions are the
                      project's own (decoy_delta_histogram.REGIONS).
"""
import json, sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "testing/scripts"))
import tier_gates as TG
from compare_mod_discovery import (load_ptmshepherd, load_mascot,
                                   load_unimod_title_mass, MASCOT_FILE_STEM)

XS = [5, 10, 15, 20]
FOREST = [(0.85, 1.15), (-1.15, -0.85), (1.85, 2.15), (-2.15, -1.85)]
SATBAND = [(57.90, 58.15), (58.90, 59.15)]

def inband(m, bands): return any(lo <= m <= hi for lo, hi in bands)

unimod = load_unimod_title_mass(TG.UNIMOD_XML)
PEAKS, REFS = {}, {}
for f in TG.FILES:
    src = json.loads(TG.RECON_JSON[f].read_text(encoding="utf-8"))
    PEAKS[f] = src["mod_discovery"]["peaks"] if "mod_discovery" in src else src["peaks"]
    REFS[f] = {"PTM-Shepherd": load_ptmshepherd(TG.PTMS_TSV, f),
               "Mascot": load_mascot(TG.MASCOT_DIR / MASCOT_FILE_STEM[f], unimod)[0]}

def variant_peaks(f, drop_forest=False, keep_deamidated=False, fold_satellite=False):
    out = []
    sat_psm = 0
    for p in PEAKS[f]:
        m, ann = p["delta_mass"], (p.get("annotations") or [])
        name = ann[0]["name"] if ann else ""
        if fold_satellite and inband(m, SATBAND) and not name:
            sat_psm += p["count"]          # unannotated satellite only
            continue
        if drop_forest and abs(m) >= 0.1 and inband(m, FOREST):
            if keep_deamidated and "Deamidated" in name:
                out.append(p)
            continue
        out.append(dict(p))
    if fold_satellite and sat_psm:
        top = max((p for p in out if abs(p["delta_mass"]) >= 0.1), key=lambda p: p["count"])
        top["count"] += sat_psm
    return out

def gate1_count(f, x, peaks, demote_satellite=False):
    t1, t2, bl, floor = TG.build_tiers(peaks, x)
    if demote_satellite:
        keep1 = [e for e in t1 if not inband(e["mass"], SATBAND)]
        keep2 = [e for e in t2 if not inband(e["mass"], SATBAND)]
        moved = [e for e in t1 + t2 if inband(e["mass"], SATBAND)]
        t1, t2, bl = keep1, keep2, bl + moved
    return len(TG.gate1(t1, t2, bl, REFS[f]))

VARIANTS = [
    ("baseline (fixed build)",                 dict(), False),
    ("satellite fold (simulated)",             dict(fold_satellite=True), False),
    ("satellite flag-and-demote (option B)",   dict(), True),
    ("forest removed, satellites left",        dict(drop_forest=True), False),
    ("forest removed + satellite demote",      dict(drop_forest=True), True),
    ("forest removed (Deamidated kept) + demote", dict(drop_forest=True, keep_deamidated=True), True),
]

print(f"{'variant':45s} " + "".join(f"{'X='+str(x)+'%':>9}" for x in XS))
print("-" * 82)
for label, kw, demote in VARIANTS:
    row = []
    for x in XS:
        tot = sum(gate1_count(f, x, variant_peaks(f, **kw), demote) for f in TG.FILES)
        row.append(f"{tot:9d}" + ("" if tot else ""))
    print(f"{label:45s} " + "".join(row))

print("\nper-file detail for the two treatments that matter:")
for label, kw, demote in VARIANTS[2:]:
    print(f"\n  {label}")
    for x in XS:
        per = {f: gate1_count(f, x, variant_peaks(f, **kw), demote) for f in TG.FILES}
        print(f"    X={x:2g}%  total={sum(per.values()):2d}   " +
              "  ".join(f"{f}={n}" for f, n in per.items()))
