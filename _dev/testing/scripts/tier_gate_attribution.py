#!/usr/bin/env python3
"""Attribute every gate-1 violation to the above-floor peak that causes it.

No new peak logic: imports build_tiers/gate1/lookup from tier_gates.py, so the
tiers are the SAME ones tier-gates-2026-08-25.txt reports. The only new thing is
grouping the violations by their above-floor member, and an optional exclusion
set to answer "if recon had not reported peak P, would gate 1 pass?".
"""
import json, sys
from collections import Counter, defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "testing/scripts"))
import tier_gates as TG
from compare_mod_discovery import load_ptmshepherd, load_mascot, load_unimod_title_mass, MASCOT_FILE_STEM

XS = [5, 10, 15, 20]
unimod = load_unimod_title_mass(TG.UNIMOD_XML)

PEAKS, REFS = {}, {}
for f in TG.FILES:
    src = json.loads(TG.RECON_JSON[f].read_text(encoding="utf-8"))
    PEAKS[f] = src["mod_discovery"]["peaks"] if "mod_discovery" in src else src["peaks"]
    REFS[f] = {"PTM-Shepherd": load_ptmshepherd(TG.PTMS_TSV, f),
               "Mascot": load_mascot(TG.MASCOT_DIR / MASCOT_FILE_STEM[f], unimod)[0]}


def run(exclude=(), tag=""):
    """exclude: list of (file, mass) pairs dropped from the peak list before tiering."""
    ex = defaultdict(list)
    for f, m in exclude:
        ex[f].append(m)
    print(f"\n{'='*88}\n{tag or 'BASELINE (no exclusions)'}\n{'='*88}")
    totals = {}
    for x in XS:
        line, tot = [], 0
        detail = {}
        for f in TG.FILES:
            pk = [p for p in PEAKS[f]
                  if not any(abs(p["delta_mass"] - m) < 5e-4 for m in ex[f])]
            t1, t2, bl, floor = TG.build_tiers(pk, x)
            v = TG.gate1(t1, t2, bl, REFS[f])
            tot += len(v)
            line.append(f"{f}={len(v):2d}")
            detail[f] = Counter(f"{hi['mass']:+.4f} {hi['label'][:26]}" for hi, lo, _ in v)
        totals[x] = tot
        print(f"  X={x:2g}%  total={tot:3d}   " + "  ".join(line))
        for f in TG.FILES:
            for k, n in detail[f].most_common():
                print(f"          {f:6s} {n:2d} x  {k}")
    return totals


base = run()

# --- candidate single suspects, by identity -------------------------------
SAT = [("serum", 58.0260), ("bcell", 58.0243), ("b1906", 58.0237)]
SAT2 = [("bcell", 59.0280), ("b1906", 59.0277), ("serum", 59.0276)]
CARPET_BCELL = [("bcell", -1.0290), ("bcell", -0.9804)]

run(SAT, "EXCLUDE the +58.02 satellite only (all three files)")
run(CARPET_BCELL, "EXCLUDE bcell's two contested +/-1 Da carpet peaks only")
run(SAT + SAT2, "EXCLUDE the +58/+59 satellite family (all three files)")
run(SAT + SAT2 + CARPET_BCELL, "EXCLUDE satellite family + bcell carpet pair")
