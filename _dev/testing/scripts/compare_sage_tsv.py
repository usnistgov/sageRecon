#!/usr/bin/env python3
"""Compare two Sage results.sage.tsv files column by column.

WHY THIS EXISTS. NOTES "Sage output is NOT bit-reproducible across builds"
records the rule this script enforces: never byte-compare Sage output across
builds. Every RAW measurement must be bit-identical; the five rescoring outputs
may move, and q-derived PSM counts are reproducible only to about 0.1%.

Used as the gate for A1 landing 2 (subprocess -> embedded library). Pass it the
committed baseline and a fresh run.

    python3 testing/scripts/compare_sage_tsv.py BASELINE.tsv PROBE.tsv

Exit status is 1 if any RAW column differs, which is a hard failure. Movement in
the rescoring columns is reported, not failed.
"""
import sys
import csv

# Bit-identical across builds, per NOTES. A difference here is a real defect.
RAW = [
    "expmass", "calcmass", "isotope_error", "precursor_ppm", "fragment_ppm",
    "hyperscore", "delta_next", "delta_best", "rt", "matched_peaks",
    "longest_b", "longest_y", "matched_intensity_pct", "scored_candidates",
    "poisson", "ms2_intensity",
]
# Model-based, may move between builds.
RESCORE = [
    "sage_discriminant_score", "posterior_error", "spectrum_q", "peptide_q",
    "protein_q",
]
KEY = ["filename", "scannr", "peptide", "rank"]


def load(path):
    with open(path, newline="", encoding="utf-8") as fh:
        rows = list(csv.DictReader(fh, delimiter="\t"))
    return rows


def key_of(row):
    return tuple(row.get(k, "") for k in KEY)


def main(baseline_path, probe_path):
    base = load(baseline_path)
    probe = load(probe_path)
    print(f"baseline rows: {len(base)}")
    print(f"probe rows:    {len(probe)}")

    bmap = {key_of(r): r for r in base}
    pmap = {key_of(r): r for r in probe}
    shared = set(bmap) & set(pmap)
    print(f"shared PSM keys: {len(shared)}")
    print(f"baseline-only:   {len(set(bmap) - set(pmap))}")
    print(f"probe-only:      {len(set(pmap) - set(bmap))}")

    cols = base[0].keys() if base else []
    missing = [c for c in RAW + RESCORE if c not in cols]
    if missing:
        print(f"\nWARNING: columns absent from the TSV: {missing}")

    failed = False
    print("\n--- RAW columns (must be bit-identical) ---")
    for col in RAW:
        if col not in cols:
            continue
        diffs = sum(1 for k in shared if bmap[k][col] != pmap[k][col])
        status = "OK" if diffs == 0 else "DIFFERS"
        if diffs:
            failed = True
        print(f"  {status:8} {col:24} {diffs} of {len(shared)} rows differ")

    print("\n--- RESCORING columns (movement expected) ---")
    for col in RESCORE:
        if col not in cols:
            continue
        worst = 0.0
        diffs = 0
        for k in shared:
            if bmap[k][col] == pmap[k][col]:
                continue
            diffs += 1
            try:
                worst = max(worst, abs(float(bmap[k][col]) - float(pmap[k][col])))
            except ValueError:
                pass
        print(f"  {col:26} {diffs} of {len(shared)} rows differ, max |delta| {worst:.6g}")

    print("\n--- q <= 0.01 membership ---")
    for col in ("spectrum_q", "peptide_q"):
        if col not in cols:
            continue
        bset = {key_of(r) for r in base if float(r[col]) <= 0.01}
        pset = {key_of(r) for r in probe if float(r[col]) <= 0.01}
        moved_out = len(bset - pset)
        moved_in = len(pset - bset)
        denom = len(bset) or 1
        pct = 100.0 * (moved_out + moved_in) / denom
        print(f"  {col:12} baseline {len(bset)}  probe {len(pset)}  "
              f"out {moved_out}  in {moved_in}  ({pct:.3f}% of baseline)")

    print()
    if failed:
        print("RESULT: FAIL — a raw measurement column changed between builds.")
        return 1
    print("RESULT: PASS — every raw measurement is bit-identical.")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(__doc__)
        sys.exit(2)
    sys.exit(main(sys.argv[1], sys.argv[2]))
