#!/usr/bin/env python3
"""Liver window-sign and fragment m/z check on a STOCK Sage search.

Reads the two TSVs that stock Sage v0.15.0-beta.2 writes with
`--annotate-matches`: `results.sage.tsv` and `matched_fragments.sage.tsv`.
The search config is `sage-window-check/results.json` (Sage's own record).

It reports two things.

1. Window sign. The config sets `precursor_tol.da = [-3.5, 1.25]`. Sage applies
   the tolerance to the OBSERVED mass, so the observed delta
   (expmass - calcmass) must fall in about -1.25 .. +3.5 Da. The script prints
   the actual min and max over every PSM row.
2. Fragment m/z. For rank-1 target PSMs with spectrum_q <= 0.01, it joins the
   matched fragments on psm_id and prints the median of
   `fragment_mz_experimental`, the intensity-weighted median (weights =
   `fragment_intensity`), and the percent of fragments with
   400 <= m/z <= 600.

Standard library only.

Usage:
    python3 _dev/liver-benchmark/window_and_fragments.py SAGE_OUTPUT_DIR
"""
import csv
import os
import sys

csv.field_size_limit(10 ** 9)


def median(values):
    v = sorted(values)
    n = len(v)
    if n == 0:
        return float("nan")
    mid = n // 2
    return v[mid] if n % 2 else (v[mid - 1] + v[mid]) / 2.0


def weighted_median(pairs):
    """pairs: (value, weight). The lowest value where the cumulative weight
    reaches half the total weight."""
    p = sorted(pairs)
    total = sum(w for _, w in p)
    acc = 0.0
    for v, w in p:
        acc += w
        if acc >= total / 2.0:
            return v
    return float("nan")


def main():
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    out = sys.argv[1]
    psm_path = os.path.join(out, "results.sage.tsv")
    frag_path = os.path.join(out, "matched_fragments.sage.tsv")

    n_rows = n_target = n_decoy = 0
    ranks = {}
    dmin = dmax = None
    keep = set()
    with open(psm_path, newline="") as fh:
        for r in csv.DictReader(fh, delimiter="\t"):
            n_rows += 1
            d = float(r["expmass"]) - float(r["calcmass"])
            dmin = d if dmin is None else min(dmin, d)
            dmax = d if dmax is None else max(dmax, d)
            ranks[r["rank"]] = ranks.get(r["rank"], 0) + 1
            if r["label"] == "1":
                n_target += 1
            else:
                n_decoy += 1
            if r["rank"] == "1" and r["label"] == "1" and float(r["spectrum_q"]) <= 0.01:
                keep.add(r["psm_id"])

    mz, pairs = [], []
    with open(frag_path, newline="") as fh:
        for r in csv.DictReader(fh, delimiter="\t"):
            if r["psm_id"] not in keep:
                continue
            m = float(r["fragment_mz_experimental"])
            mz.append(m)
            pairs.append((m, float(r["fragment_intensity"])))

    in_band = sum(1 for m in mz if 400.0 <= m <= 600.0)

    print("== window sign ==")
    print(f"PSM rows (all ranks, targets and decoys): {n_rows}")
    print(f"  targets {n_target}, decoys {n_decoy}, rows per rank {dict(sorted(ranks.items()))}")
    print(f"observed delta expmass - calcmass: min {dmin:+.4f} Da, max {dmax:+.4f} Da")
    print()
    print("== matched fragments, rank-1 target PSMs, spectrum_q <= 0.01 ==")
    print(f"PSMs: {len(keep)}")
    print(f"matched fragments: {len(mz)}")
    print(f"median fragment_mz_experimental: {median(mz):.4f}")
    print(f"intensity-weighted median: {weighted_median(pairs):.4f}")
    print(f"fragments with 400 <= m/z <= 600: {in_band} "
          f"({100.0 * in_band / len(mz):.2f} %)")


if __name__ == "__main__":
    main()
