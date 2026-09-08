#!/usr/bin/env python3
"""Check 2 — confirm or refute the serum +1 Da PSM-conservation violation, in ONE run.

WHAT IS BEING TESTED. On 2026-08-24 serum's two Deamidated peaks were measured as
claiming 360 PSMs (184 + 176) where the delta-mass histogram holds only 314 in the
region. That measurement crossed two artifacts produced by two different runs a
month apart -- `recon-output/full-run/serum.json` (analyze, 2026-08-24) for the
peaks, and `recon-output/nofixedmods/serum.json` (discover, 2026-07-24) for the
histogram. Their total PSMs and every peak count are identical, so the PSM set is
almost certainly the same, but "almost certainly" is not a measurement. This script
rebuilds the histogram from the Sage TSV directly so both sides come from one place.

THE INVARIANT, stated before the numbers:
    For any delta-mass region R, the PSMs claimed by all peaks whose apex lies in R
    must not exceed the PSMs the histogram holds in R widened by the merge margin.
    A peak cannot contain PSMs that do not exist at those masses.
Violation => peaks overlap and double-count PSMs. Exit code 1 on violation.

This script does NOT re-derive peak detection. It takes recon's reported peaks as
given and asks only whether the underlying PSMs can supply them. Re-deriving the
detector would test the detector against itself.

Filters mirror sage_results.rs exactly, in the same order: drop decoys (rev_),
then drop peptide_q >= threshold. Delta uses expmass - calcmass, corrected by
isotope_error * 1.003354835.

Usage (from repo root):
    python testing/scripts/verify_peak_conservation.py \
        --tsv testing/search-output/step1-open-serum/results.sage.tsv \
        --peaks testing/recon-output/full-run/serum.json --name serum
"""

import argparse
import csv
import json
import sys

C13_C12_DIFF = 1.003354835
DECOY_PREFIX = "rev_"

# Mirrors mod_discovery.rs. Fold-to-zero runs at PSM level before the histogram,
# so these must stay in sync with the Rust constants.
NEAR_ZERO_THRESHOLD_DA = 0.1
FOLD_TOL_BASE_DA = 0.012
FOLD_TOL_PER_STEP_DA = 0.0045

# Regions checked. Serum's +1 Da region is the one under test; the rest are context.
REGIONS = [
    ("+1 Da  (THE TEST)", 0.85, 1.15),
    ("-1 Da", -1.15, -0.85),
    ("+2 Da", 1.85, 2.15),
    ("-2 Da", -2.15, -1.85),
    ("+57 Da (control)", 56.90, 57.15),
    ("+58 Da (control)", 57.90, 58.15),
]

# Allowance for a peak legitimately merging bins just outside the region boundary.
MERGE_MARGIN_DA = 0.05


def load_deltas(tsv, q_threshold):
    deltas = []
    stats = {"rows": 0, "decoy": 0, "q_filtered": 0}
    with open(tsv, encoding="utf-8", newline="") as fh:
        r = csv.DictReader(fh, delimiter="\t")
        for row in r:
            stats["rows"] += 1
            if row["proteins"].startswith(DECOY_PREFIX):
                stats["decoy"] += 1
                continue
            try:
                q = float(row["peptide_q"])
            except (ValueError, KeyError):
                continue
            if q >= q_threshold:
                stats["q_filtered"] += 1
                continue
            try:
                exp = float(row["expmass"]); calc = float(row["calcmass"])
                iso = int(round(float(row["isotope_error"])))
            except (ValueError, KeyError):
                continue
            deltas.append((exp - calc) - iso * C13_C12_DIFF)
    return deltas, stats


def fold_to_zero(deltas):
    """Replicate mod_discovery's PSM-level neutron fold-to-zero.

    THIS IS LOAD-BEARING AND WAS MISSING IN THE FIRST VERSION OF THIS SCRIPT.
    Peaks are built from the histogram AFTER folding, so comparing peak claims
    against unfolded PSM counts compares against a supply that no longer exists.
    On serum that produced a false "ok": 894 unfolded PSMs in the +1 Da region,
    580 of them folded away, leaving 314 against a 360-PSM claim.

    Mirrors mod_discovery.rs `fold_isotope_psms_to_zero`: k in +/-1..3, tolerance
    base + (|k|-1) * per_step, PSMs already within NEAR_ZERO skipped.
    """
    out = []
    folded = 0
    for d in deltas:
        if abs(d) < NEAR_ZERO_THRESHOLD_DA:
            out.append(d)
            continue
        hit = False
        for k in (-3, -2, -1, 1, 2, 3):
            tol = FOLD_TOL_BASE_DA + (abs(k) - 1) * FOLD_TOL_PER_STEP_DA
            if abs(d - k * C13_C12_DIFF) < tol:
                hit = True
                break
        if hit:
            out.append(0.0)
            folded += 1
        else:
            out.append(d)
    return out, folded


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tsv", required=True)
    ap.add_argument("--peaks", required=True)
    ap.add_argument("--name", required=True)
    ap.add_argument("--q-threshold", type=float, default=0.01)
    args = ap.parse_args()

    raw, stats = load_deltas(args.tsv, args.q_threshold)
    deltas, n_folded = fold_to_zero(raw)
    src = json.load(open(args.peaks, encoding="utf-8"))
    md = src["mod_discovery"] if "mod_discovery" in src else src
    peaks = md["peaks"]
    reported_total = md.get("total_psms") or md.get("summary", {}).get("total_psms")

    print("=" * 86)
    print(f"{args.name}: TSV {stats['rows']} rows -> {stats['decoy']} decoys dropped, "
          f"{stats['q_filtered']} above q>={args.q_threshold}, {len(deltas)} PSMs kept")
    print(f"fold-to-zero applied: {n_folded} PSMs folded to 0.0 "
          f"(peaks are built AFTER this, so counts must be judged against it)")
    print(f"recon's reported total_psms = {reported_total}")
    if reported_total is not None and reported_total != len(deltas):
        print(f"  ** MISMATCH of {len(deltas) - reported_total:+d} PSMs. The two sides are not the "
              f"same population; every number below is suspect until this is explained.")
    else:
        print("  PSM totals agree -- peaks and histogram now come from one run.")
    print("=" * 86)

    failed = False
    print(f"{'region':22s} {'peaks':>6} {'claimed':>8} {'available':>10} {'verdict':>12}")
    for label, lo, hi in REGIONS:
        pk = [p for p in peaks if lo <= p["delta_mass"] <= hi]
        if not pk:
            continue
        claimed = sum(p["count"] for p in pk)
        avail = sum(1 for d in deltas if lo - MERGE_MARGIN_DA <= d <= hi + MERGE_MARGIN_DA)
        bad = claimed > avail
        failed = failed or bad
        print(f"{label:22s} {len(pk):6d} {claimed:8d} {avail:10d} "
              f"{('VIOLATED +' + str(claimed - avail)) if bad else 'ok':>12}")
        if bad:
            for p in sorted(pk, key=lambda q: -q["count"]):
                nm = p["annotations"][0]["name"] if p.get("annotations") else "UNANNOTATED"
                print(f"      {p['delta_mass']:+10.6f}  {p['count']:6d} PSM  rank {p.get('rank','?')}  {nm}")
            print(f"      widening window to find a source for {claimed} PSMs:")
            centre = sum(p["delta_mass"] * p["count"] for p in pk) / claimed
            for w in (0.05, 0.10, 0.20, 0.40, 0.80):
                n = sum(1 for d in deltas if abs(d - centre) <= w)
                print(f"        +/-{w:.2f} Da of {centre:+.4f}: {n:6d} PSMs"
                      + ("   <-- first window that can supply it" if n >= claimed else ""))

    print()
    if failed:
        print("RESULT: CONSERVATION VIOLATED. Peaks claim PSMs that do not exist at those")
        print("masses, so at least two peaks share PSMs. This is a peak-construction defect,")
        print("not a reporting artifact. Record it and stop before building on these counts.")
        sys.exit(1)
    print("RESULT: conservation holds in every region checked.")


if __name__ == "__main__":
    main()
