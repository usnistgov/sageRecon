#!/usr/bin/env python3
"""
psm_count_sensitivity.py — empirical test of "how many clean PSMs do we
actually need" for the MS1/MS2 bias+MAD numbers recon reports.

Why this exists (2026-08-19 conversation, see NOTES.md once decided):
recon's hyperscore-guard code currently gates on an unexplained 200-PSM
floor. MetaMorpheus's own engineered sufficiency floor for a harder job
(per-scan drift correction, not just a summary number) is >=16 PSMs,
>=40 MS1 datapoints, >=80 MS2 datapoints (reference-notes/
metamorpheus-mass-error-calibration.md). No number in either camp is
backed by a specific published citation — this script exists so we pick
a floor from evidence on our own three files instead of another guess.

What it does:
  1. Loads a Sage results TSV, applies the same clean-subset filter as
     `select_clean_subset` in calibration.rs (rank==1, target, q < q_threshold,
     |delta_mass_corrected| < delta_da_threshold). No hyperscore trim here —
     this script is testing the floor itself, not the optional 60% trim
     that sits on top of it.
  2. Reports the full clean-subset size and its median/MAD MS1 (precursor_ppm)
     and MS2 (fragment_ppm) — this is the file's best-available estimate,
     the number every smaller N gets compared against.
  3. Bootstrap: for a grid of N, draws random subsamples (no replacement)
     many times, computes median/MAD MS1+MS2 per draw, and reports how much
     those estimates wobble around the full-data value at each N.
  4. Flags the smallest N where the 90% spread of the bootstrap draws falls
     within a tolerance you supply (default 0.5 ppm) of the full-data value.

Usage:
  python3 psm_count_sensitivity.py results.sage.tsv \
      --q-threshold 0.01 --delta-da-threshold 0.02 \
      --tolerance-ppm 0.5 --draws 500 --seed 42 \
      --out testing/recon-output/psm-sensitivity/<file>.json

Ground truth notes for interpreting output (fill in from NOTES.md once you
have it open — not hardcoded here since these are per-file, not universal):
  - b1906: independently confirmed +0.53 ppm MS1 (FragPipe MS1-Old match).
    Good anchor — full-data output here should already read ~+0.53.
  - serum: FragPipe-agreed ~+2.46 to +2.51 ppm. Good anchor.
  - bcell: recon and FragPipe currently DISAGREE (-0.22 vs ~0.00, unresolved).
    Not a trustworthy "truth" anchor yet — use it to check whether small-N
    estimates get noisier, not whether they get "more correct."
"""

import argparse
import csv
import json
import random
import statistics
import sys
from pathlib import Path


C13_C12_DIFF = 1.003354835  # recon-tool/src/sage_results.rs C13_C12_DIFF, not the free-neutron mass
DECOY_PREFIX = "rev_"  # recon-tool/src/sage_results.rs DEFAULT_DECOY_PREFIX


def load_clean_subset(tsv_path, q_threshold, delta_da_threshold, decoy_prefix=DECOY_PREFIX):
    """Mirrors select_clean_subset() in calibration.rs, minus the hyperscore trim.

    Sage's raw results.sage.tsv has no `is_decoy` or `delta_mass_corrected`
    columns -- those are computed by recon's own parser
    (recon-tool/src/sage_results.rs: parse_sage_results). Reproduced here
    from `proteins`/`expmass`/`calcmass`/`isotope_error` so this script reads
    the same TSV recon itself consumes.
    """
    rows = []
    with open(tsv_path, newline="") as f:
        reader = csv.DictReader(f, delimiter="\t")
        for row in reader:
            try:
                is_decoy = row["proteins"].startswith(decoy_prefix)
                rank = int(float(row["rank"]))
                q_value = float(row["peptide_q"])
                isotope_error = round(float(row["isotope_error"]))
                delta_mass = float(row["expmass"]) - float(row["calcmass"])
                delta_da = delta_mass - (isotope_error * C13_C12_DIFF)
                precursor_ppm = float(row["precursor_ppm"])
                fragment_ppm = float(row["fragment_ppm"])
            except (KeyError, ValueError) as e:
                raise SystemExit(
                    f"Column/parse error on a row: {e}. Expected raw Sage TSV columns "
                    "(proteins, rank, peptide_q, expmass, calcmass, isotope_error, "
                    "precursor_ppm, fragment_ppm) -- see recon-tool/src/sage_results.rs's "
                    "REQUIRED_COLUMNS."
                )
            if is_decoy or rank != 1:
                continue
            if q_value >= q_threshold:
                continue
            if abs(delta_da) >= delta_da_threshold:
                continue
            rows.append({"precursor_ppm": precursor_ppm, "fragment_ppm": fragment_ppm})
    return rows


def median_mad(values):
    if not values:
        return None, None
    med = statistics.median(values)
    mad = statistics.median([abs(v - med) for v in values])
    return med, mad


def bootstrap_at_n(clean, n, draws, rng, key):
    """Returns list of `draws` median values, each from an N-sized random
    subsample (without replacement) of clean[key]."""
    values = [row[key] for row in clean]
    if n > len(values):
        return None
    out = []
    for _ in range(draws):
        sample = rng.sample(values, n)
        med, _ = median_mad(sample)
        out.append(med)
    return out


def spread_90(draws_list):
    """90% interval width (5th to 95th percentile) of a list of draw values."""
    s = sorted(draws_list)
    n = len(s)
    lo = s[int(0.05 * n)]
    hi = s[min(int(0.95 * n), n - 1)]
    return hi - lo, lo, hi


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("tsv_path", help="Path to Sage results TSV for the WIDE (open) search")
    ap.add_argument("--q-threshold", type=float, default=0.01)
    ap.add_argument("--delta-da-threshold", type=float, default=0.02,
                     help="Matches calibration.rs unit-test default; check against your actual production constant before trusting results")
    ap.add_argument("--tolerance-ppm", type=float, default=0.5,
                     help="90%% spread must fall within this many ppm of the full-data value to flag a floor as 'stable enough'")
    ap.add_argument("--draws", type=int, default=500)
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--grid", type=int, nargs="+",
                     default=[16, 30, 50, 75, 100, 150, 200, 300, 500, 750, 1000])
    ap.add_argument("--out", type=str, default=None, help="Optional path to write JSON summary")
    args = ap.parse_args()

    rng = random.Random(args.seed)

    clean = load_clean_subset(args.tsv_path, args.q_threshold, args.delta_da_threshold)
    n_full = len(clean)
    if n_full == 0:
        raise SystemExit("Clean subset is empty at these thresholds — nothing to test.")

    ms1_full_med, ms1_full_mad = median_mad([r["precursor_ppm"] for r in clean])
    ms2_full_med, ms2_full_mad = median_mad([r["fragment_ppm"] for r in clean])

    print(f"File: {args.tsv_path}")
    print(f"Full clean subset: N = {n_full}")
    print(f"  MS1 (precursor_ppm): median = {ms1_full_med:.3f} ppm, MAD = {ms1_full_mad:.3f}")
    print(f"  MS2 (fragment_ppm):  median = {ms2_full_med:.3f} ppm, MAD = {ms2_full_mad:.3f}")
    print()
    print(f"{'N':>6} | {'MS1 90% spread':>16} | {'MS1 stable?':>11} | {'MS2 90% spread':>16} | {'MS2 stable?':>11}")
    print("-" * 74)

    results = {
        "file": str(args.tsv_path),
        "q_threshold": args.q_threshold,
        "delta_da_threshold": args.delta_da_threshold,
        "n_full": n_full,
        "ms1_full_median_ppm": ms1_full_med,
        "ms1_full_mad": ms1_full_mad,
        "ms2_full_median_ppm": ms2_full_med,
        "ms2_full_mad": ms2_full_mad,
        "tolerance_ppm": args.tolerance_ppm,
        "draws": args.draws,
        "seed": args.seed,
        "grid": [],
    }

    smallest_stable_n = None
    for n in sorted(set(args.grid)):
        ms1_draws = bootstrap_at_n(clean, n, args.draws, rng, "precursor_ppm")
        ms2_draws = bootstrap_at_n(clean, n, args.draws, rng, "fragment_ppm")
        if ms1_draws is None:
            print(f"{n:>6} | {'N > N_full, skipped':>16} |")
            continue
        ms1_spread, ms1_lo, ms1_hi = spread_90(ms1_draws)
        ms2_spread, ms2_lo, ms2_hi = spread_90(ms2_draws)
        ms1_stable = ms1_spread <= 2 * args.tolerance_ppm
        ms2_stable = ms2_spread <= 2 * args.tolerance_ppm
        print(f"{n:>6} | {ms1_spread:>13.3f} ppm | {'yes' if ms1_stable else 'no':>11} | "
              f"{ms2_spread:>13.3f} ppm | {'yes' if ms2_stable else 'no':>11}")
        if ms1_stable and ms2_stable and smallest_stable_n is None:
            smallest_stable_n = n
        results["grid"].append({
            "n": n, "ms1_spread_90": ms1_spread, "ms1_stable": ms1_stable,
            "ms2_spread_90": ms2_spread, "ms2_stable": ms2_stable,
        })

    print()
    if smallest_stable_n:
        print(f"Smallest N with both MS1 and MS2 90% spread <= 2x{args.tolerance_ppm} ppm tolerance: {smallest_stable_n}")
    else:
        print(f"No N in the grid reached the {args.tolerance_ppm} ppm tolerance band both ways — "
              "widen --grid, loosen --tolerance-ppm, or this file just doesn't support a small floor.")
    results["smallest_stable_n"] = smallest_stable_n

    if args.out:
        out_path = Path(args.out)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(json.dumps(results, indent=2))
        print(f"\nWrote {out_path}")


if __name__ == "__main__":
    main()
