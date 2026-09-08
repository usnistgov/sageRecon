#!/usr/bin/env python3
"""Check 4 — the ghost test. Build the delta-mass histogram WITH decoys retained.

Tests the hypothesis in NOTES ("⚠ HYPOTHESIS — the ±1 Da peak forest may be
decoy-floor noise"). Recon drops decoys at load (`sage_results.rs`), so mod
discovery never sees the noise model. This rebuilds the histogram from the same
TSV without that filter and reports target:decoy enrichment per bin.

WHY NOT FILTER ON q-VALUE. `peptide_q` comes from target/decoy competition, so
filtering at q<0.01 removes essentially the whole decoy population by
construction — that is what FDR control does. Comparing targets and decoys after
a q filter measures nothing. This script therefore filters on a matched
SCORE threshold applied identically to both populations, which is what Wilmarth's
guide and Preview's THigh/TLow both do. Default is no score filter at all; use
--min-hyperscore to match a threshold.

DELTA CONVENTION — mirrors sage_results.rs exactly:
    delta_mass            = expmass - calcmass
    delta_mass_corrected  = delta_mass - isotope_error * 1.003354835
The corrected value is the one mod discovery bins. Both are reported here; they
are identical whenever isotope_error is 0, which our pinned config forces (see
`"isotope_errors": [0, 0]`).

READ THE OUTPUT LIKE THIS. A bin that is real chemistry should be strongly
target-enriched. A bin that is the incorrect-match floor should sit near the
background target:decoy ratio. The +/-1 and +/-2 Da region is the region under
test; +57.02 is the positive control and should be sharply enriched.

Usage (from repo root):
    python testing/scripts/decoy_delta_histogram.py \
        --tsv testing/search-output/step1-open-serum/results.sage.tsv --name serum
    # all three, plus a score-matched variant:
    python testing/scripts/decoy_delta_histogram.py --tsv ... --name ... --min-hyperscore 20
"""

import argparse
import csv
import math
import sys
from collections import defaultdict
from pathlib import Path

C13_C12_DIFF = 1.003354835
DECOY_PREFIX = "rev_"
BIN_WIDTH_DA = 0.01

# Regions reported in the summary table. The +/-1 and +/-2 Da rows are the test;
# the others are controls with a known answer.
REGIONS = [
    ("zero (control: real)", -0.10, 0.10),
    ("+1 Da region (TEST)", 0.85, 1.15),
    ("-1 Da region (TEST)", -1.15, -0.85),
    ("+2 Da region (TEST)", 1.85, 2.15),
    ("-2 Da region (TEST)", -2.15, -1.85),
    ("deamidation +0.984 (control: real)", 0.974, 0.994),
    ("+57.02 (control: real)", 56.98, 57.06),
    ("+58.02 satellite (TEST)", 57.98, 58.06),
    ("+15.995 oxidation (control: real)", 15.96, 16.03),
]


def load(tsv, min_hyperscore):
    rows = []
    with open(tsv, encoding="utf-8", newline="") as fh:
        r = csv.DictReader(fh, delimiter="\t")
        need = {"proteins", "expmass", "calcmass", "isotope_error", "charge", "hyperscore", "peptide_q"}
        missing = need - set(r.fieldnames or [])
        if missing:
            raise SystemExit(f"TSV missing columns: {sorted(missing)}")
        for row in r:
            try:
                exp = float(row["expmass"]); calc = float(row["calcmass"])
                iso = int(round(float(row["isotope_error"])))
                z = int(float(row["charge"]))
                hs = float(row["hyperscore"])
                q = float(row["peptide_q"])
            except (ValueError, KeyError):
                continue
            if min_hyperscore is not None and hs < min_hyperscore:
                continue
            d = exp - calc
            rows.append({
                "decoy": row["proteins"].startswith(DECOY_PREFIX),
                "delta": d,
                "delta_corr": d - iso * C13_C12_DIFF,
                "charge": z, "hyperscore": hs, "q": q, "iso": iso,
            })
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tsv", required=True)
    ap.add_argument("--name", required=True)
    ap.add_argument("--min-hyperscore", type=float, default=None)
    ap.add_argument("--per-charge", action="store_true",
                    help="also break every region down by charge state (Wilmarth calls this mandatory)")
    ap.add_argument("--field", choices=["delta", "delta_corr"], default="delta_corr")
    args = ap.parse_args()

    rows = load(args.tsv, args.min_hyperscore)
    t = [r for r in rows if not r["decoy"]]
    d = [r for r in rows if r["decoy"]]
    if not d:
        raise SystemExit("No decoy rows found. Is this TSV already decoy-filtered, "
                         f"or is the prefix not {DECOY_PREFIX!r}?")

    print("=" * 88)
    print(f"{args.name}: {len(rows)} rows  ->  {len(t)} target, {len(d)} decoy"
          f"   (score filter: {args.min_hyperscore if args.min_hyperscore is not None else 'none'})")
    print(f"binning field: {args.field}")
    bg = len(t) / len(d)
    print(f"BACKGROUND target:decoy ratio for this file = {bg:.2f}  <-- the number every region is judged against")
    print("=" * 88)
    print(f"{'region':38s} {'target':>8} {'decoy':>7} {'T:D':>8} {'vs bg':>8}   verdict")

    for label, lo, hi in REGIONS:
        tt = sum(1 for r in t if lo <= r[args.field] <= hi)
        dd = sum(1 for r in d if lo <= r[args.field] <= hi)
        if tt == 0 and dd == 0:
            print(f"{label:38s} {'-':>8} {'-':>7} {'-':>8} {'-':>8}   empty")
            continue
        ratio = (tt / dd) if dd else math.inf
        rel = (ratio / bg) if bg else math.inf
        verdict = ("ENRICHED" if rel >= 2 else "at background" if rel <= 1.25 else "weak")
        print(f"{label:38s} {tt:8d} {dd:7d} {ratio:8.2f} {rel:7.2f}x   {verdict}")

    if args.per_charge:
        print("\nPer charge state (resolution and monoisotope-calling error both vary with z):")
        charges = sorted({r["charge"] for r in rows if 1 <= r["charge"] <= 6})
        for label, lo, hi in REGIONS:
            if "TEST" not in label and "deamidation" not in label:
                continue
            print(f"  {label}")
            for z in charges:
                tz = [r for r in t if r["charge"] == z]
                dz = [r for r in d if r["charge"] == z]
                if not dz:
                    continue
                bgz = len(tz) / len(dz)
                tt = sum(1 for r in tz if lo <= r[args.field] <= hi)
                dd = sum(1 for r in dz if lo <= r[args.field] <= hi)
                if tt == 0 and dd == 0:
                    continue                  # region empty at this charge; nothing to judge
                if dd == 0:
                    print(f"     z={z}: target {tt:6d}  decoy     0  T:D      inf  "
                          f"(no decoys in region -- strongly enriched, ratio undefined)")
                    continue
                ratio = tt / dd
                rel = (ratio / bgz) if bgz else float("inf")
                print(f"     z={z}: target {tt:6d}  decoy {dd:5d}  T:D {ratio:7.2f}  "
                      f"vs bg({bgz:.2f}) {rel:6.2f}x")

    # Per-bin dump for the +/-1 Da region, which is where the forest lives.
    print("\nPer-bin detail, -1.15 .. +1.15 Da, bins with >= 20 target PSMs:")
    tb = defaultdict(int); db = defaultdict(int)
    for r in t:
        if -1.15 <= r[args.field] <= 1.15:
            tb[round(r[args.field] / BIN_WIDTH_DA)] += 1
    for r in d:
        if -1.15 <= r[args.field] <= 1.15:
            db[round(r[args.field] / BIN_WIDTH_DA)] += 1
    print(f"   {'bin centre':>11} {'target':>7} {'decoy':>6} {'T:D':>8} {'vs bg':>8}")
    for b in sorted(set(tb) | set(db)):
        if tb[b] < 20:
            continue
        ratio = (tb[b] / db[b]) if db[b] else math.inf
        rel = (ratio / bg) if bg else math.inf
        print(f"   {b * BIN_WIDTH_DA:+11.2f} {tb[b]:7d} {db[b]:6d} {ratio:8.2f} {rel:7.2f}x")


if __name__ == "__main__":
    main()
