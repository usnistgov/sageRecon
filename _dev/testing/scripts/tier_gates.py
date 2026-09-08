#!/usr/bin/env python3
"""Step 2 tier gates: pick the floor multiplier X empirically, not by judgement.

Runs the three validation gates from `reference-notes/ptm-stratification-design.md`
Step 5 that are computable from committed artifacts with no re-search, sweeps X,
and reports which X values pass. The decision rule is stated up front so it cannot
be adjusted after seeing the numbers:

    Pick the SMALLEST X that passes every gate.

Smallest = most inclusive = most useful to a user, so the gates are what removes
candidates, not preference.

Reads recon peaks from `analyze` or `discover` JSON via compare_mod_discovery's
loaders (single source for the reference-tool adapters — do not duplicate them).

TIER CONSTRUCTION (design Step 3, plus one correction measured 2026-08-24):
  1. Exclude |delta| < NEAR_ZERO_THRESHOLD_DA (0.1). Covers the Delta~0 population,
     the 0.075-0.100 Da roll-up gap, and every fold-to-zero target (folding rewrites
     the PSM delta to exactly 0.0 before binning).
  2. MERGE peaks that share an annotation name and sit within MERGE_SPAN_DA of each
     other. The detector emits peaks closer together than its own 0.01 Da bin width
     in the +/-1 and +/-2 Da region; the members of every such group are
     indistinguishable on every confidence metric (hyperscore, matched intensity,
     longest b/y). Without this the same chemistry is recommended several times.
  3. floor = X% of the top non-zero peak count (+57 on all three test files).
  4. Tier 1 = above floor AND annotated AND not ambiguous. Tier 2 = above floor,
     otherwise. Below floor = listed, not recommended.

GATES:
  Gate 1 (rank agreement). Recon's only ordering claim is ABOVE FLOOR vs BELOW
    FLOOR. Tier 1 vs tier 2 is an annotation-confidence split, not a magnitude
    claim -- an unannotated peak is not "smaller" than an annotated one -- so
    tier1/tier2 pairs are NOT compared. For every above/below pair where the
    available reference tools agree with each other that the BELOW-floor mass is
    the larger one, that is a violation.
  Gate 2 (tier stability). A chemistry present in more than one file should not be
    tier 1 on one and below the floor on another without a stated reason. Reported
    per X as the count of unstable chemistries; read, not auto-failed, because real
    sample differences are expected (serum is a biofluid; bcell and b1906 are not).
  Gate 4 (negative control). Serum's +57 is proven over-alkylation, not added
    glycine (Phase 8 Gate 3: 0 of 26 peptides with Gly flanking context). A tier
    that recommends Gly on serum fails.

MetaMorpheus is not read here: its AllPSMs.psmtsv is gitignored and absent from a
checkout without the raw data. Gate 1 therefore runs on PTM-Shepherd + Mascot. With
only two tools, "the tools agree" means both, which is the strictest reading.

Usage:
    python testing/scripts/tier_gates.py
    python testing/scripts/tier_gates.py --x 5 10 15 20
"""

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from compare_mod_discovery import (  # noqa: E402
    MASCOT_FILE_STEM,
    MATCH_TOL_DA,
    load_mascot,
    load_ptmshepherd,
    load_recon,
    load_unimod_title_mass,
)

REPO = Path(__file__).resolve().parents[2]

# Mirrors recon_tool::mod_discovery::NEAR_ZERO_THRESHOLD_DA. Kept in sync by hand;
# if that constant moves, this must move with it.
NEAR_ZERO_THRESHOLD_DA = 0.1

# Max span across a same-annotation group for it to be treated as one split peak.
# The observed groups span at most 14.7 mDa (bcell Lys->Allysine, 3 peaks); 0.05 Da
# is comfortably above that and still far below the gap to any distinct chemistry.
MERGE_SPAN_DA = 0.05

FILES = ["serum", "bcell", "b1906"]
RECON_JSON = {f: REPO / "testing/recon-output/full-run" / f"{f}.json" for f in FILES}
PTMS_TSV = REPO / "testing/reference-data/ptm-shepherd/reallyOpen/global.modsummary.tsv"
MASCOT_DIR = REPO / "testing/reference-data/mascot/error-tolerant"
UNIMOD_XML = REPO / "testing/reference-data/unimod.xml"


def build_tiers(peaks, x_pct):
    """Apply the exclusion, the merge, and the floor. Returns (tier1, tier2, below, floor)."""
    nz = [p for p in peaks if abs(p["delta_mass"]) >= NEAR_ZERO_THRESHOLD_DA]
    if not nz:
        return [], [], [], 0.0
    floor = max(p["count"] for p in nz) * x_pct / 100.0

    groups = defaultdict(list)
    singles = []
    for p in nz:
        anns = p.get("annotations", [])
        if anns and not p.get("unannotated"):
            groups[anns[0]["name"]].append(p)
        else:
            singles.append(p)

    entries = []
    for name, members in groups.items():
        members.sort(key=lambda p: p["delta_mass"])
        run = [members[0]]
        for p in members[1:]:
            if p["delta_mass"] - run[-1]["delta_mass"] <= MERGE_SPAN_DA:
                run.append(p)
            else:
                entries.append(_merge(run, name))
                run = [p]
        entries.append(_merge(run, name))
    for p in singles:
        entries.append({
            "mass": p["delta_mass"], "count": p["count"], "label": "UNANNOTATED",
            "ambiguous": p.get("ambiguous", False), "n_merged": 1,
        })

    tier1, tier2, below = [], [], []
    for e in entries:
        if e["count"] < floor:
            below.append(e)
        elif e["label"] != "UNANNOTATED" and not e["ambiguous"]:
            tier1.append(e)
        else:
            tier2.append(e)
    for lst in (tier1, tier2, below):
        lst.sort(key=lambda e: -e["count"])
    return tier1, tier2, below, floor


def _merge(run, name):
    total = sum(p["count"] for p in run)
    mass = sum(p["delta_mass"] * p["count"] for p in run) / total
    return {"mass": mass, "count": total, "label": name,
            "ambiguous": any(p.get("ambiguous", False) for p in run), "n_merged": len(run)}


def lookup(rows, mass, tol=MATCH_TOL_DA):
    """Best count for `mass` in a reference table, or None if the tool has no such mass."""
    hits = [r for r in rows if abs(r["mass"] - mass) <= tol]
    return max(r["count"] for r in hits) if hits else None


def gate1(tier1, tier2, below, refs):
    """Cross-tier contradictions of a reference consensus. Returns list of violations."""
    # Rank 1 = above the floor (tier 1 and tier 2 alike), rank 2 = below it.
    # The floor is the only ordering claim recon makes.
    tiered = [(1, e) for e in tier1] + [(1, e) for e in tier2] + [(2, e) for e in below]
    violations = []
    for i, (ta, a) in enumerate(tiered):
        for tb, b in tiered[i + 1:]:
            if ta == tb:
                continue                      # no ordering claim at the same rank
            hi, lo = (a, b) if ta < tb else (b, a)
            votes = []
            for name, rows in refs.items():
                ca, cb = lookup(rows, hi["mass"]), lookup(rows, lo["mass"])
                if ca is None or cb is None:
                    continue                  # tool lacks one of the masses
                votes.append((name, ca, cb))
            if len(votes) < 2:
                continue                      # need agreement between tools
            if all(cb > ca for _, ca, cb in votes):
                violations.append((hi, lo, votes))
    return violations


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--x", type=float, nargs="+", default=[5, 10, 15, 20])
    args = ap.parse_args()

    unimod_title_mass = load_unimod_title_mass(UNIMOD_XML)
    results = {}

    for f in FILES:
        peaks_src = json.loads(RECON_JSON[f].read_text(encoding="utf-8"))
        peaks = peaks_src["mod_discovery"]["peaks"] if "mod_discovery" in peaks_src else peaks_src["peaks"]
        refs = {
            "PTM-Shepherd": load_ptmshepherd(PTMS_TSV, f),
            "Mascot": load_mascot(MASCOT_DIR / MASCOT_FILE_STEM[f], unimod_title_mass)[0],
        }
        for x in args.x:
            t1, t2, bl, floor = build_tiers(peaks, x)
            results[(f, x)] = {
                "tier1": t1, "tier2": t2, "below": bl, "floor": floor,
                "g1": gate1(t1, t2, bl, refs),
                "g4": [e for e in t1 + t2 if "gly" in e["label"].lower()],
            }

    print("=" * 92)
    print("STEP 2 TIER GATES — decision rule fixed in advance: smallest X that passes every gate")
    print("=" * 92)

    for x in args.x:
        print(f"\n### X = {x:g}%")
        g1_tot = g4_tot = 0
        seen = defaultdict(dict)
        for f in FILES:
            r = results[(f, x)]
            g1_tot += len(r["g1"])
            g4_tot += len(r["g4"])
            print(f"  {f:6s} floor={r['floor']:7.1f} PSM   tier1={len(r['tier1']):2d}  "
                  f"tier2={len(r['tier2']):2d}  below={len(r['below']):2d}   "
                  f"gate1 violations={len(r['g1'])}   gate4 Gly in tiers={len(r['g4'])}")
            for e in r["tier1"]:
                seen[e["label"]][f] = 1
            for e in r["tier2"]:
                seen[e["label"]][f] = 2
            for e in r["below"]:
                seen[e["label"]][f] = 3
            for hi, lo, votes in r["g1"]:
                v = ", ".join(f"{n}: {ca} vs {cb}" for n, ca, cb in votes)
                print(f"      !! GATE1 {hi['mass']:+.4f} ({hi['label'][:22]}) placed above "
                      f"{lo['mass']:+.4f} ({lo['label'][:22]}); references say otherwise -> {v}")

        unstable = {k: v for k, v in seen.items()
                    if k != "UNANNOTATED" and len(v) > 1 and 1 in v.values() and 3 in v.values()}
        print(f"  GATE 2 — chemistries tier-1 on one file and below floor on another: {len(unstable)}")
        for k, v in sorted(unstable.items()):
            print(f"      {k[:44]:44s} " + "  ".join(f"{f}=T{v[f]}" for f in FILES if f in v))

        verdict = "PASS" if (g1_tot == 0 and g4_tot == 0) else "FAIL"
        print(f"  ==> X={x:g}%  gate1={g1_tot} violations, gate4={g4_tot} -> {verdict}"
              f"   (gate2 is read, not auto-failed)")


if __name__ == "__main__":
    main()
