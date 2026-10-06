#!/usr/bin/env python3
"""Why the Deamidation peak reads low on bcell and b1906.

The report gives each peak a `delta_mass`. For Deamidation (Unimod 0.984016 Da)
it reads about 2.1 mDa low on bcell and b1906. This script rebuilds that number
from the pass-1 Sage TSV and shows where the difference comes from.

What recon does (`mod_discovery.rs`, `detect_peaks_with_prominence`):
  - bins are 0.01 Da wide, so the Deamidation centre is the 0.98 bin;
  - a PSM is a member of the peak if |delta - 0.98| <= 0.01, that is, the
    window [0.97, 0.99];
  - `delta_mass` is the ms2-intensity-weighted MEAN of the members.

INVARIANT, asserted below: the rebuilt count and `delta_mass` equal the
committed report. If they do not, the TSV is a different run and every number
after that point is meaningless, so the script stops.

The rebuild applies NO isotope fold. If it still matches the report, the fold
stage does not set this number.

Inputs (the TSVs are gitignored; the script stops if one is missing):
  _dev/testing/recon-output/full-run/<file>_search/results.sage.tsv
  _dev/testing/recon-output/full-run/<file>.json

Standard library only. Run from the repository root.
"""
import csv
import json
import statistics
import sys
from pathlib import Path

C13_C12_DIFF = 1.003355
DEAMIDATED = 0.984016
BIN_CENTRE = 0.98
MERGE_TOLERANCE = 0.01
NEAR_ZERO = 0.1
Q_THRESHOLD = 0.01
FULL_RUN = Path("_dev/testing/recon-output/full-run")
FILES = ["bcell", "b1906", "liver", "serum"]


def weighted_median(pairs):
    """Same rule as `intensity_weighted_median` in mod_discovery.rs."""
    pairs = sorted(pairs)
    total = sum(w for _, w in pairs)
    acc = 0.0
    for value, weight in pairs:
        acc += weight
        if acc >= total / 2:
            return value
    raise ValueError("empty population")


def weighted_mean(pairs):
    total = sum(w for _, w in pairs)
    return sum(d * w for d, w in pairs) / total


def load_calibrated(name):
    tsv = FULL_RUN / f"{name}_search" / "results.sage.tsv"
    if not tsv.exists():
        sys.exit(f"missing input: {tsv}")
    rows = []
    with open(tsv, newline="") as handle:
        for rec in csv.DictReader(handle, delimiter="\t"):
            if rec["proteins"].startswith("rev_"):
                continue
            if float(rec["peptide_q"]) > Q_THRESHOLD:
                continue
            delta = float(rec["expmass"]) - float(rec["calcmass"])
            delta -= int(float(rec["isotope_error"])) * C13_C12_DIFF
            rows.append((delta, float(rec["ms2_intensity"])))
    offset = weighted_median([(d, w) for d, w in rows if abs(d) < NEAR_ZERO])
    return [(d - offset, w) for d, w in rows], offset


def density(cal, low, high):
    """PSMs per mDa in [low, high)."""
    return sum(1 for d, _ in cal if low <= d < high) / ((high - low) * 1000)


def main():
    for name in FILES:
        report = json.loads((FULL_RUN / f"{name}.json").read_text())
        peak = next(
            p for p in report["mod_discovery"]["peaks"] if 0.97 < p["delta_mass"] < 0.99
        )
        cal, offset = load_calibrated(name)

        window = [(d, w) for d, w in cal if abs(d - BIN_CENTRE) <= MERGE_TOLERANCE]
        rebuilt = weighted_mean(window)
        assert len(cal) == report["mod_discovery"]["total_psms"], (
            f"{name}: {len(cal)} PSMs rebuilt, report has "
            f"{report['mod_discovery']['total_psms']}"
        )
        assert len(window) == peak["count"], (
            f"{name}: window holds {len(window)} PSMs, report peak has {peak['count']}"
        )
        assert abs(rebuilt - peak["delta_mass"]) < 1e-9, (
            f"{name}: rebuilt {rebuilt:.9f}, report {peak['delta_mass']:.9f}"
        )

        # 1 mDa histogram of the window. The mode is the apex, and it does not
        # depend on where the window is cut.
        fine = {}
        for d, _ in window:
            key = int(round(d * 1000))
            fine[key] = fine.get(key, 0) + 1
        mode = max(fine, key=lambda k: (fine[k], -k))

        low_side = [(d, w) for d, w in window if d < 0.981]
        flank = density(cal, 0.955, 0.970)

        print(f"\n{name}  (PSMs {len(cal)}, apex_offset {offset * 1000:+.3f} mDa)")
        print(
            f"  report peak, rebuilt with no fold: count {len(window)}, "
            f"delta_mass {rebuilt:.5f}, residual {(rebuilt - DEAMIDATED) * 1000:+.2f} mDa"
        )
        print(
            f"  window [0.970, 0.990] against the true mass: "
            f"{(0.970 - DEAMIDATED) * 1000:+.1f} to {(0.990 - DEAMIDATED) * 1000:+.1f} mDa"
        )
        print(
            f"  apex (mode of the 1 mDa histogram): {mode / 1000:.3f} "
            f"({fine[mode]} PSMs), residual {(mode / 1000 - DEAMIDATED) * 1000:+.1f} mDa"
        )
        print(
            f"  median of the window: "
            f"{statistics.median(d for d, _ in window):.5f}"
        )
        print(
            f"  window members below 0.981: {len(low_side)} "
            f"({100 * len(low_side) / len(window):.0f} % of the count, "
            f"{100 * sum(w for _, w in low_side) / sum(w for _, w in window):.0f} % "
            f"of the intensity)"
        )
        print(
            f"  background, PSMs per mDa: 0.900-0.950 {density(cal, 0.90, 0.95):.1f}, "
            f"0.955-0.970 {flank:.1f}, 0.970-0.981 {density(cal, 0.970, 0.981):.1f}, "
            f"1.030-1.080 {density(cal, 1.03, 1.08):.1f}"
        )
        print(
            f"  ESTIMATE: a flat background at the 0.955-0.970 level fills the "
            f"20 mDa window with {flank * 20:.0f} of its {len(window)} PSMs"
        )
        print(
            "  1 mDa histogram 0.970-0.990: "
            + " ".join(str(fine.get(k, 0)) for k in range(970, 991))
        )


if __name__ == "__main__":
    main()
