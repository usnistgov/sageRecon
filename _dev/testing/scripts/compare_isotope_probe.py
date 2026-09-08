#!/usr/bin/env python3
"""Compare the isotope_errors probe against the current b1906 output.

Prints the four numbers that decide whether Sage's own multi-notch precursor
handling collapses the +58 / +59 satellites at the search layer. If it does, the
satellite question in `reference-notes/satellite-memo.md` retires and step 2 gets
simpler.

THE CONTROL ROW IS THE ISOTOPE-ERROR DISTRIBUTION. Our pinned config sets
`isotope_errors: [0, 0]`, which forces that column to 0 for every PSM. If the probe
still reports 100% at zero, the config edit did not take effect and every other row
is meaningless -- that is exactly what happened on the first attempt (2026-08-25),
where the edit was a comment in the run sheet and got skipped.

Usage (from repo root):
    python testing/scripts/compare_isotope_probe.py
    python testing/scripts/compare_isotope_probe.py --probe <path> --current <path>
"""

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
DEFAULT_CURRENT = REPO / "testing/recon-output/nofixedmods/b1906.json"
DEFAULT_PROBE = REPO / "testing/recon-output/2026-08-25-checks/06-isotope-probe-b1906.json"

REGIONS = [
    ("+57.02 parent", 56.95, 57.10),
    ("+58.02 (M+1 of +57)", 57.95, 58.10),
    ("+59.02 (M+2 of +57)", 58.95, 59.10),
]


def peaks_of(doc):
    return doc["mod_discovery"]["peaks"] if "mod_discovery" in doc else doc["peaks"]


def region_count(doc, lo, hi):
    return sum(p["count"] for p in peaks_of(doc) if lo <= p["delta_mass"] <= hi)


def iso_summary(doc):
    dist = doc.get("isotope_error_view", {}).get("distribution", [])
    total = sum(b["count"] for b in dist) or 1
    nonzero = sum(b["count"] for b in dist if b["isotope_error"] != 0)
    return nonzero, total, {b["isotope_error"]: b["count"] for b in dist}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--current", default=str(DEFAULT_CURRENT))
    ap.add_argument("--probe", default=str(DEFAULT_PROBE))
    args = ap.parse_args()

    cur = json.loads(Path(args.current).read_text(encoding="utf-8"))
    prb = json.loads(Path(args.probe).read_text(encoding="utf-8"))

    print("=" * 78)
    print("isotope_errors probe -- b1906")
    print(f"  current: {args.current}")
    print(f"  probe:   {args.probe}")
    print("=" * 78)

    # Control first. Nothing below it means anything if this fails.
    nz_c, tot_c, dist_c = iso_summary(cur)
    nz_p, tot_p, dist_p = iso_summary(prb)
    print("\nCONTROL -- did the config edit take effect?")
    print(f"  current: {nz_c}/{tot_c} PSMs with isotope_error != 0   dist={dist_c}")
    print(f"  probe:   {nz_p}/{tot_p} PSMs with isotope_error != 0   dist={dist_p}")
    if nz_p == 0:
        print("\n  ** PROBE DID NOT TAKE EFFECT. isotope_error is still 100% zero, which is")
        print("     what isotope_errors [0, 0] forces. Re-check that the probe config really")
        print("     contains [-1, 2] and that Sage was pointed at it. Everything below is")
        print("     meaningless until this row changes.")
        effective = False
    else:
        print("\n  Probe took effect -- isotope_error is now populated.")
        effective = True

    print(f"\n{'metric':32s} {'current':>10} {'probe':>10}   verdict")
    for label, lo, hi in REGIONS:
        a = region_count(cur, lo, hi)
        b = region_count(prb, lo, hi)
        if a == 0:
            verdict = "n/a"
        elif b < a * 0.5:
            verdict = "COLLAPSED"
        elif abs(b - a) < a * 0.1:
            verdict = "unchanged"
        else:
            verdict = "moved"
        print(f"{label:32s} {a:10d} {b:10d}   {verdict}")

    for key in ("folded_to_zero_count", "satellite_fold_count"):
        a = cur.get("folding", {}).get(key, 0)
        b = prb.get("folding", {}).get(key, 0)
        verdict = "COLLAPSED" if a and b < a * 0.5 else "unchanged" if abs(b - a) < max(a, 1) * 0.1 else "moved"
        print(f"{key:32s} {a:10d} {b:10d}   {verdict}")

    for key in ("total_psms", "psms_near_zero"):
        a = cur["summary"][key]
        b = prb["summary"][key]
        print(f"{key:32s} {a:10d} {b:10d}   {'same' if a == b else f'{b - a:+d}'}")

    print("\nHOW TO READ IT:")
    print("  +58 and +59 COLLAPSED and folded_to_zero_count down sharply -> Sage handles")
    print("  monoisotope misassignment upstream; the satellite memo retires and the")
    print("  full change-regenerate becomes worth proposing (with its impact trace).")
    print("  Unchanged -> Sage's multi-notch does not help here at our window width, the")
    print("  satellite stays a recon-side problem, and no regenerate is justified.")
    if not effective:
        print("\n  As run, this probe answered nothing. See the CONTROL block above.")
        sys.exit(2)


if __name__ == "__main__":
    main()
