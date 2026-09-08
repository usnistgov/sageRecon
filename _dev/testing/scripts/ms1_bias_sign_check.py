#!/usr/bin/env python3
"""Test two open questions about recon's MS1/MS2 calibration numbers.

--------------------------------------------------------------------------
QUESTION 1 (primary): is recon's reported `bias_ppm` a SIGNED bias, or a
median of ABSOLUTE errors?

Why this matters. `compute_ms1_stats` (calibration.rs:168) takes the median
of Sage's raw `precursor_ppm` column as-is.

HISTORY (v0.14.x, retired 2026-09-01): under the Sage version pinned before
the v0.15.0-beta.2 upgrade, `precursor_ppm` was |error| -- an absolute value
that could never be negative. Under that pin, recon's "bias" could never be
negative either, and it read systematically HIGH whenever the true bias was
small relative to the scatter. That degeneracy was the leading explanation
for the largest unresolved discrepancy in NOTES ("MetaMorpheus workflow
review, part 2"): recon vs. MetaMorpheus/MSFragger bias on serum, b1906,
bcell.

CURRENT (v0.15.0-beta.2, the pin as of 2026-09-02): `precursor_ppm` is
SIGNED. Confirmed by Michael Lazear (Sage author) 2026-08-17 and by the
committed open-search output testing/recon-output/full-run/serum_search/
results.sage.tsv (17591 of 68817 values negative, min -121568.77). This is
now the EXPECTED, locked result -- see AGENTS.md "The two conventions that
keep getting re-litigated". A run of this script that reports the column
as absolute (zero negatives) on that pin is the anomaly and means something
is wrong (wrong TSV, wrong Sage build, a regression) -- not a rediscovery
of the old v0.14 behaviour.

Two dispositive checks, both run below:
  (a) Count negative values in the raw precursor_ppm column. A healthy
      fraction of negatives across tens of thousands of PSMs is what a
      SIGNED column looks like; zero negatives would mean the column is
      absolute, which should not happen on the current pin.
  (b) Independently reconstruct the SIGNED error from the mass columns:
          signed_ppm = (expmass - calcmass - isotope_error*1.003354835)
                       / calcmass * 1e6
      For near-zero-delta PSMs this IS the signed instrument error, and it
      does not depend on what precursor_ppm contains. Compare its median
      against median(precursor_ppm).

  If (a) finds a healthy fraction of negatives AND (b)'s signed median
  agrees with the raw median, the expected v0.15 behaviour is CONFIRMED.

  SURPRISE (state before running, per AGENTS.md): if the column has zero
  negatives, OR the two medians disagree materially, that contradicts the
  locked v0.15 convention. Report the disagreement and stop -- do not
  reinterpret it as the old |error| hypothesis being "confirmed" without
  first checking which Sage build actually produced the TSV.

--------------------------------------------------------------------------
QUESTION 2 (secondary): does the clean subset understate the mass-error
scatter of the wider population a real search must accommodate?

This is the mechanism claim behind
reference-notes/ms1-tolerance-recommendation-rationale.md section 4, which
is currently tagged [INFERRED]. NOTE: the test named in that note ("compare
clean-subset MAD vs all-confident-PSM MAD") is ILL-POSED for an OPEN search
-- a modified peptide's precursor_ppm reflects its modification mass, not
instrument error (NOTES "Precursor ppm in open search is huge/garbage",
median ~14,000 ppm). So:

  - From the OPEN TSV we can only isolate the HYPERSCORE-TRIM factor:
    near-zero-delta subset with vs without the top-60% trim.
  - The full comparison needs a CLOSED TSV, where mods are explicit variable
    mods and precursor_ppm is instrument error for every PSM. Pass one with
    --closed-tsv.

  ⚠ RETIRED / UNREACHABLE (2026-09-02): recon's closed-search support was
  retired (see AGENTS.md "argument surface is FROZEN" -- there is no closed
  search subcommand or template left to generate a closed TSV from). The
  --closed-tsv argument and the "[factor B: full population, closed search]"
  code path below are kept for reference only; nothing in the current tool
  can produce an input for them. Do not treat their absence from a run as
  informative, and do not try to wire up a new closed-search path to feed
  them -- that would be re-deriving something the project intentionally
  removed.

--------------------------------------------------------------------------
Usage:
  python3 ms1_bias_sign_check.py OPEN.tsv --label serum --closed-tsv CLOSED.tsv
"""
import argparse
import csv
import statistics
from pathlib import Path

C13_C12_DIFF = 1.003354835  # recon-tool/src/sage_results.rs
DECOY_PREFIX = "rev_"


def median_mad(values):
    if not values:
        return None, None
    med = statistics.median(values)
    mad = statistics.median([abs(v - med) for v in values])
    return med, mad


def load(tsv_path, q_threshold=0.01, delta_da_threshold=None):
    """Load confident target rank-1 PSMs. If delta_da_threshold is given,
    additionally restrict to the near-zero-delta (unmodified) population."""
    rows = []
    with open(tsv_path, newline="") as f:
        for row in csv.DictReader(f, delimiter="\t"):
            if row["proteins"].startswith(DECOY_PREFIX):
                continue
            if int(float(row["rank"])) != 1:
                continue
            if float(row["peptide_q"]) >= q_threshold:
                continue
            iso = round(float(row["isotope_error"]))
            calc = float(row["calcmass"])
            delta_da = (float(row["expmass"]) - calc) - iso * C13_C12_DIFF
            if delta_da_threshold is not None and abs(delta_da) >= delta_da_threshold:
                continue
            rows.append({
                "raw_precursor_ppm": float(row["precursor_ppm"]),
                "raw_fragment_ppm": float(row["fragment_ppm"]),
                # Independent signed reconstruction -- does NOT rely on the
                # sign convention of the precursor_ppm column.
                "signed_ppm": (delta_da / calc) * 1e6 if calc else 0.0,
                "hyperscore": float(row["hyperscore"]),
            })
    return rows


def sign_report(rows, field, name):
    vals = [r[field] for r in rows]
    neg = sum(1 for v in vals if v < 0)
    pos = sum(1 for v in vals if v > 0)
    zero = len(vals) - neg - pos
    med, mad = median_mad(vals)
    print(f"  {name}: n={len(vals)}  negatives={neg} ({100.0*neg/len(vals):.2f}%)  "
          f"positives={pos}  zeros={zero}")
    print(f"    median={med:+.4f} ppm   MAD={mad:.4f} ppm")
    return neg, med, mad


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("open_tsv")
    ap.add_argument("--label", default="?")
    ap.add_argument(
        "--closed-tsv", default=None,
        help="RETIRED/UNREACHABLE (2026-09-02): recon's closed-search support "
             "was retired, so nothing can produce a closed TSV to pass here. "
             "Kept only so the factor-B code path below still runs if you "
             "hand it a TSV from elsewhere.",
    )
    ap.add_argument("--q-threshold", type=float, default=0.01)
    ap.add_argument("--delta-da-threshold", type=float, default=0.02)
    args = ap.parse_args()

    print("=" * 78)
    print(f"FILE: {args.label}   open TSV: {args.open_tsv}")
    print("=" * 78)

    clean = load(args.open_tsv, args.q_threshold, args.delta_da_threshold)
    if not clean:
        raise SystemExit("Clean subset empty -- check thresholds/paths.")

    # ---------------- QUESTION 1 ----------------
    print("\n--- QUESTION 1: is precursor_ppm absolute or signed? ---")
    print("\n[open search, near-zero-delta clean subset]")
    neg_prec, med_raw, mad_raw = sign_report(clean, "raw_precursor_ppm",
                                             "Sage raw precursor_ppm  ")
    neg_frag, _, _ = sign_report(clean, "raw_fragment_ppm",
                                 "Sage raw fragment_ppm   ")
    _, med_signed, mad_signed = sign_report(clean, "signed_ppm",
                                            "reconstructed SIGNED ppm")

    print("\n  VERDICT:")
    col_is_abs = (neg_prec == 0)
    delta = abs(med_raw - med_signed)
    if not col_is_abs and delta <= 0.05:
        print(f"    CONFIRMED. precursor_ppm contains {neg_prec} negative values (signed),")
        print(f"    and recon's reported bias ({med_raw:+.4f}) agrees with the")
        print(f"    independently reconstructed signed bias ({med_signed:+.4f}) to")
        print(f"    {delta:.4f} ppm. This is the expected v0.15.0-beta.2 behaviour --")
        print("    see AGENTS.md 'precursor_ppm signed vs absolute'.")
    elif not col_is_abs:
        print(f"    ANOMALY. precursor_ppm contains {neg_prec} negative values (signed,")
        print("    as expected on v0.15), but recon's reported bias")
        print(f"    ({med_raw:+.4f}) differs from the reconstructed signed bias")
        print(f"    ({med_signed:+.4f}) by {delta:.4f} ppm -- more than the 0.05 ppm")
        print("    tolerance. The column's sign convention is right, but the two")
        print("    medians disagree for some other reason. Investigate before")
        print("    trusting recon's bias_ppm on this file -- do not wave this past.")
    else:
        print(f"    SURPRISE. precursor_ppm has ZERO negative values (absolute) on")
        print("    what should be the v0.15.0-beta.2 pin, where the column is signed")
        print("    (AGENTS.md, confirmed by Sage's author and by the committed serum")
        print("    TSV). This contradicts the locked convention -- do NOT read it as")
        print("    the old v0.14 |error| hypothesis being confirmed. Check which Sage")
        print("    build actually produced this TSV before concluding anything.")

    # ---------------- QUESTION 2 ----------------
    print("\n--- QUESTION 2: does the clean subset understate scatter? ---")
    print("\n[factor A: the top-60%-by-hyperscore trim, open search]")
    _, mad_untrimmed = median_mad([r["signed_ppm"] for r in clean])
    trimmed = sorted(clean, key=lambda r: r["hyperscore"], reverse=True)
    trimmed = trimmed[:max(1, round(len(trimmed) * 0.60))]
    _, mad_trimmed = median_mad([r["signed_ppm"] for r in trimmed])
    print(f"  untrimmed near-zero-delta: n={len(clean)}  MAD={mad_untrimmed:.4f} ppm")
    print(f"  top-60% by hyperscore:     n={len(trimmed)}  MAD={mad_trimmed:.4f} ppm")
    if mad_untrimmed:
        print(f"  => trim narrows scatter by {100.0*(1 - mad_trimmed/mad_untrimmed):.1f}%")

    if args.closed_tsv:
        print("\n[factor B: full population, closed search]")
        closed_all = load(args.closed_tsv, args.q_threshold, None)
        closed_near = load(args.closed_tsv, args.q_threshold, args.delta_da_threshold)
        _, mad_all = median_mad([r["signed_ppm"] for r in closed_all])
        _, mad_near = median_mad([r["signed_ppm"] for r in closed_near])
        print(f"  closed, ALL confident PSMs:   n={len(closed_all)}  MAD={mad_all:.4f} ppm")
        print(f"  closed, near-zero-delta only: n={len(closed_near)}  MAD={mad_near:.4f} ppm")
        print(f"  open clean subset (trimmed):  n={len(trimmed)}  MAD={mad_trimmed:.4f} ppm")
        if mad_all and mad_trimmed:
            ratio = mad_all / mad_trimmed
            print(f"\n  => wider population has {ratio:.2f}x the scatter of the clean subset")
            if ratio > 1.15:
                print("     MECHANISM SUPPORTED: clean subset understates real-search scatter.")
            else:
                print("     MECHANISM NOT SUPPORTED at this file. Do not claim it in the")
                print("     paper -- the tolerance conclusion stands on other evidence.")
    else:
        print("\n[factor B skipped -- pass --closed-tsv for the full comparison]")

    print()


if __name__ == "__main__":
    main()
