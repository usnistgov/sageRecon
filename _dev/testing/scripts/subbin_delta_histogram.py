#!/usr/bin/env python3
"""Sub-bin histogram of the delta-mass axis, to settle the Merge/Split mode.

WHY THIS EXISTS. `mode_sibling_separation.py` can prove that Split is shredding a
single population (consecutive gap below one bin width). It CANNOT prove the
opposite: a gap of about one bin width looks the same whether it is two real
populations or one broad population smeared across adjacent bins. The committed
`discover` JSONs are binned at 10 mDa, the same grid, so they cannot answer it
either. This script goes back to the PSMs and re-bins finer.

LEAD WITH THE INVARIANT. The Delta~0 `Unmodified` population is ONE population -
that is not an assumption, it is what "unmodified" means. Its width on the
delta-mass axis is therefore a direct measurement of THIS FILE's delta-mass
scatter, with an independently known correct answer. If that scatter is as wide as
a bin, then two apparent populations one bin apart cannot be resolved by this data,
and Split is shredding. The reference peak and the test peaks are measured by the
same code on the same axis.

EQUIVALENCE TRIPWIRE - the reason this script can be trusted at all. It
reimplements the Rust delta pipeline (isotope correction, da-scalar calibration,
neutron fold-to-zero) in Python. A reimplementation is a re-derivation and can
drift silently. So `--verify` rebuilds the histogram at the SHIPPED 10 mDa bin
width and diffs it bin-for-bin against the committed `discover` JSON. HARD-STOP on
mismatch. Only after that match is the finer binning trustworthy.

Ported from `recon-tool/src/{sage_results,mod_discovery}.rs`. If either changes,
`--verify` fails, which is the point.

Usage:
  subbin_delta_histogram.py --tsv <results.sage.tsv> --verify <discover.json>
  subbin_delta_histogram.py --tsv <results.sage.tsv> --bin-width 0.001 \\
      --region <lo> <hi> [--region <lo> <hi> ...]
"""

import argparse
import csv
import json
import sys

# --- constants, ported verbatim. Source file and line are the provenance. ---
C13_C12_DIFF = 1.003354835            # sage_results.rs:89
NEAR_ZERO_THRESHOLD_DA = 0.1          # mod_discovery.rs:93
FOLD_TOL_BASE_DA = 0.012              # mod_discovery.rs:101
FOLD_TOL_PER_STEP_DA = 0.0045         # mod_discovery.rs:104
DECOY_PREFIX = "rev_"                 # sage_results.rs DEFAULT_DECOY_PREFIX
SHIPPED_BIN_WIDTH_DA = 0.01


def fold_tolerance_for_k(k):
    """mod_discovery.rs:109. k=1: 12 mDa, k=2: 16.5 mDa, k=3: 21 mDa."""
    return FOLD_TOL_BASE_DA + max(abs(k) - 1, 0) * FOLD_TOL_PER_STEP_DA


def load_psms(tsv_path, q_threshold=0.01):
    """sage_results.rs:parse_sage_results. Decoy by protein prefix, then
    peptide_q (NOT spectrum_q), then isotope correction. isotope_error_zero_only
    defaults to false, so no isotope filter is applied."""
    deltas, intensities = [], []
    with open(tsv_path, newline="", encoding="utf-8") as fh:
        for rec in csv.DictReader(fh, delimiter="\t"):
            if rec["proteins"].startswith(DECOY_PREFIX):
                continue
            if float(rec["peptide_q"]) >= q_threshold:
                continue
            isotope_error = round(float(rec["isotope_error"]))
            delta = float(rec["expmass"]) - float(rec["calcmass"])
            deltas.append(delta - isotope_error * C13_C12_DIFF)
            intensities.append(float(rec["ms2_intensity"]))
    return deltas, intensities


def intensity_weighted_median(pairs):
    """mod_discovery.rs:intensity_weighted_median."""
    if not pairs:
        return 0.0
    pairs = sorted(pairs, key=lambda t: t[0])
    half = sum(i for _, i in pairs) / 2.0
    cum = 0.0
    for value, intensity in pairs:
        cum += intensity
        if cum >= half:
            return value
    return pairs[-1][0]


def calibrate_da_scalar(deltas, intensities):
    """mod_discovery.rs:compute_calibration, DaScalar branch."""
    near_zero = [(d, i) for d, i in zip(deltas, intensities)
                 if abs(d) < NEAR_ZERO_THRESHOLD_DA]
    if not near_zero:
        return list(deltas), 0.0
    offset = intensity_weighted_median(near_zero)
    return [d - offset for d in deltas], offset


def fold_to_zero(deltas):
    """mod_discovery.rs:fold_isotope_psms_to_zero. First matching k wins."""
    out, folded = list(deltas), 0
    for idx, delta in enumerate(out):
        if abs(delta) < NEAR_ZERO_THRESHOLD_DA:
            continue
        for k in (-3, -2, -1, 1, 2, 3):
            if abs(delta - k * C13_C12_DIFF) < fold_tolerance_for_k(k):
                out[idx] = 0.0
                folded += 1
                break
    return out, folded


def rust_round(x):
    """Rust `f64::round` rounds half AWAY FROM ZERO. Python `round` rounds half to
    EVEN. They differ only for a value exactly on a .5 boundary, which is rare
    enough to look like agreement and common enough to happen: on bcell exactly one
    PSM sits on the 284.125 Da boundary, and it moved one count between two bins.
    The equivalence check caught it. Do not replace this with `round`."""
    import math
    return math.copysign(math.floor(abs(x) + 0.5), x)


def histogram(deltas, intensities, bin_width):
    """mod_discovery.rs:build_histogram_from_deltas. Bin index by ROUND, not
    floor - a floor grid would shift every centre by half a bin."""
    bins = {}
    for delta, intensity in zip(deltas, intensities):
        idx = rust_round(delta / bin_width)
        c, isum, wsum = bins.get(idx, (0, 0.0, 0.0))
        bins[idx] = (c + 1, isum + intensity, wsum + delta * intensity)
    out = []
    for idx in sorted(bins):
        c, isum, wsum = bins[idx]
        out.append(dict(bin_center=idx * bin_width, count=c, intensity_sum=isum,
                        weighted_apex=(wsum / isum) if isum > 0 else idx * bin_width))
    return out


def pipeline(tsv_path, bin_width, q_threshold=0.01):
    deltas, intensities = load_psms(tsv_path, q_threshold)
    calibrated, offset = calibrate_da_scalar(deltas, intensities)
    folded_deltas, folded = fold_to_zero(calibrated)
    return histogram(folded_deltas, intensities, bin_width), dict(
        psms=len(deltas), apex_offset_da=offset, folded_to_zero=folded)


def verify(tsv_path, json_path, q_threshold):
    """HARD-STOP equivalence check against the shipped Rust output."""
    mine, meta = pipeline(tsv_path, SHIPPED_BIN_WIDTH_DA, q_threshold)
    ref = json.load(open(json_path, encoding="utf-8"))
    theirs = ref["histogram"]
    print(f"  python PSMs kept       {meta['psms']}")
    print(f"  json   total_psms      {ref['summary'].get('total_psms')}")
    print(f"  python apex_offset     {meta['apex_offset_da'] * 1000:.4f} mDa")
    print(f"  json   apex_offset     {ref['calibration']['apex_offset_da'] * 1000:.4f} mDa")
    print(f"  python folded_to_zero  {meta['folded_to_zero']}")
    print(f"  json   folded_to_zero  {ref['folding']['folded_to_zero_count']}")
    print(f"  python bins            {len(mine)}")
    print(f"  json   bins            {len(theirs)}")
    bad = 0
    if len(mine) != len(theirs):
        print(f"  BIN COUNT MISMATCH: {len(mine)} vs {len(theirs)}")
        bad += 1
    for a, b in zip(mine, theirs):
        if abs(a["bin_center"] - b["bin_center"]) > 1e-9 or a["count"] != b["count"]:
            if bad < 10:
                print(f"  MISMATCH at {b['bin_center']:.5f}: "
                      f"count {a['count']} vs {b['count']}")
            bad += 1
    if bad:
        print(f"\n  FAIL: {bad} differing bin(s). The Python port is NOT equivalent.")
        print("  Do not read any finer histogram from it until this is zero.")
        return 1
    print(f"\n  PASS: all {len(mine)} bins identical (centre and count).")
    return 0


def show_region(hist, lo, hi, bin_width, label=""):
    sel = [b for b in hist if lo <= b["bin_center"] <= hi]
    if not sel:
        print(f"  no bins in [{lo}, {hi}]")
        return
    peak = max(b["count"] for b in sel)
    print(f"  region [{lo:+.4f}, {hi:+.4f}] {label}  bin {bin_width * 1000:.1f} mDa  "
          f"max count {peak}")
    for b in sel:
        bar = "#" * int(60 * b["count"] / peak) if peak else ""
        print(f"    {b['bin_center']:+9.4f}  {b['count']:6}  {bar}")


def fwhm(hist, center, half_span, bin_width, bg=0.0):
    """Full width at half maximum around `center`, in mDa, measured ABOVE `bg`.

    The half-maximum level is `bg + (apex - bg) / 2`, NOT `apex / 2`.

    CORRECTED. Using `apex / 2` biases every low-contrast feature narrow, and it
    did: Carbamidomethyl sits on a background of 2, so `apex / 2` is far above
    the carpet and the width is honest, while a feature in the +/-1 Da forest sits
    on a background of ~14, where `apex / 2` lands just above the carpet and the
    profile crosses it almost immediately. Real peaks and carpet features were
    being measured against different levels, and the resulting "the contested
    features are narrower than real peaks" comparison was an artifact of that,
    not a property of the data.

    Returns None if the half-maximum is reached only at the window edge, or if the
    excess over background is too small for a crossing to mean anything
    (< 4 sigma of the background). A width measured on noise is not a width."""
    sel = [b for b in hist if abs(b["bin_center"] - center) <= half_span]
    if not sel:
        return None
    apex = max(sel, key=lambda b: b["count"])
    excess = apex["count"] - bg
    if excess < 4.0 * max(bg, 1.0) ** 0.5:
        return None
    half = bg + excess / 2.0
    idx = sel.index(apex)
    left = right = None
    for i in range(idx, -1, -1):
        if sel[i]["count"] < half:
            left = sel[i]["bin_center"]
            break
    for i in range(idx, len(sel)):
        if sel[i]["count"] < half:
            right = sel[i]["bin_center"]
            break
    if left is None or right is None:
        return None
    return (right - left) * 1000.0


def peak_test(hist, center, bin_width, core_mda=8.0, bg_mda=40.0):
    """Is there a PEAK at `center`, or is this flat carpet?

    A peak detector that runs on a 10 mDa grid cannot answer this: a chunk of
    featureless background containing more PSMs than its neighbours becomes a
    "peak" with a count and a rank, and it looks exactly like a real one in the
    peak table. The difference is visible only below the grid.

    Measures the apex inside +/-`core_mda` against the local background, taken as
    the MEDIAN bin count in the surrounding annulus out to +/-`bg_mda`. Median,
    not mean, so a real neighbouring peak inside the annulus does not inflate the
    background and hide a true signal.

    z is a Poisson deviate, (apex - bg) / sqrt(bg). Counting statistics only. It
    does NOT include the systematic that matters most here - whether the carpet is
    genuinely flat - so read it with the FWHM, not instead of it. A real narrow
    population has both a high z AND a width near the reference peaks' 3-7 mDa. A
    carpet chunk has a low z and no measurable width.
    """
    core = [b for b in hist if abs(b["bin_center"] - center) <= core_mda / 1000.0]
    annulus = [b for b in hist
               if core_mda / 1000.0 < abs(b["bin_center"] - center) <= bg_mda / 1000.0]
    if not core or not annulus:
        return None
    apex = max(b["count"] for b in core)
    counts = sorted(b["count"] for b in annulus)
    bg = counts[len(counts) // 2]
    z = (apex - bg) / (bg ** 0.5) if bg > 0 else float("inf")
    # Width is measured above the LOCAL background, so a peak on the +/-1 Da
    # carpet and a peak on clean baseline are measured the same way. `fwhm`
    # returns None when the excess is too small for the crossing to mean
    # anything.
    width = fwhm(hist, center, bg_mda / 1000.0, bin_width, bg=bg)
    return dict(apex=apex, bg=bg, z=z, width=width)


STABILITY_BIN_WIDTHS_DA = (0.001, 0.002, 0.004)
STABILITY_MIN_Z = 5.0


def stability_test(tsv_path, centers, q_threshold):
    """Classify each center as a REAL population or CARPET, by re-measuring it at
    several bin widths.

    WHY STABILITY, AND NOT A SINGLE MEASUREMENT. `peak_test` at one bin width is
    not safe to read on its own. Measured at 2 mDa, the sibling at -0.97116 on
    bcell gives z=6.4 with a 6.0 mDa width - the signature of a real narrow
    population, and the evidence that would have made that group a genuine
    doublet. At 1 mDa and at 4 mDa the same feature gives z=3.2 and z=4.1 with no
    measurable width. It is a background fluctuation that a particular grid
    flattered. A real population does not do this: the controls hold their z
    across all three widths, and their FWHM tracks the grid, as quantization must.

    REAL requires z >= 5 at EVERY bin width and a measurable width at two of the
    three. Anything else is CARPET. Both criteria can fail, and on the real files
    both do.
    """
    per_width = {}
    for bw in STABILITY_BIN_WIDTHS_DA:
        hist, meta = pipeline(tsv_path, bw, q_threshold)
        per_width[bw] = {c: peak_test(hist, float(c), bw) for c, _ in centers}
    rows = []
    for center, label in centers:
        zs, widths = [], []
        for bw in STABILITY_BIN_WIDTHS_DA:
            r = per_width[bw][center]
            zs.append(float("nan") if r is None else r["z"])
            widths.append(None if r is None else r["width"])
        measurable = sum(1 for w in widths if w is not None)
        verdict = ("REAL" if all(z >= STABILITY_MIN_Z for z in zs) and measurable >= 2
                   else "carpet")
        rows.append(dict(center=float(center), label=label, zs=zs,
                         widths=widths, verdict=verdict))
    return rows


def print_stability(rows):
    print(f"  {'center':>10} {'z@1mDa':>7} {'z@2mDa':>7} {'z@4mDa':>7} "
          f"{'FWHM 1/2/4 mDa':>18}  {'verdict':>7}  label")
    for r in rows:
        w = "/".join("-" if x is None else f"{x:.0f}" for x in r["widths"])
        print(f"  {r['center']:>10.5f} {r['zs'][0]:>7.1f} {r['zs'][1]:>7.1f} "
              f"{r['zs'][2]:>7.1f} {w:>18}  {r['verdict']:>7}  {r['label']}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tsv", required=True)
    ap.add_argument("--verify")
    ap.add_argument("--bin-width", type=float, default=0.001)
    ap.add_argument("--q-threshold", type=float, default=0.01)
    ap.add_argument("--region", nargs=2, type=float, action="append", default=[])
    ap.add_argument("--fwhm", nargs=2, type=float, action="append", default=[],
                    help="CENTER HALFSPAN - report FWHM around CENTER")
    ap.add_argument("--peak-test", nargs=2, action="append", default=[],
                    metavar=("CENTER", "LABEL"),
                    help="peak-or-carpet test at CENTER, tagged LABEL")
    ap.add_argument("--stability", nargs=2, action="append", default=[],
                    metavar=("CENTER", "LABEL"),
                    help="REAL-or-carpet verdict at CENTER, across bin widths")
    args = ap.parse_args()

    if args.verify:
        print(f"EQUIVALENCE CHECK  {args.tsv}")
        return verify(args.tsv, args.verify, args.q_threshold)

    if args.stability:
        print(f"{args.tsv}")
        rows = stability_test(args.tsv, args.stability, args.q_threshold)
        print_stability(rows)
        return 0

    hist, meta = pipeline(args.tsv, args.bin_width, args.q_threshold)
    print(f"{args.tsv}  PSMs {meta['psms']}  apex_offset "
          f"{meta['apex_offset_da'] * 1000:.4f} mDa  folded {meta['folded_to_zero']}")
    for center, half_span in args.fwhm:
        w = fwhm(hist, center, half_span, args.bin_width)
        print(f"  FWHM around {center:+.5f} (+/-{half_span * 1000:.0f} mDa): "
              f"{'ran off the window edge' if w is None else f'{w:.1f} mDa'}")
    if args.peak_test:
        print(f"  {'center':>10} {'apex':>6} {'bg':>5} {'z':>7} {'FWHM':>10}  label")
        for center, label in args.peak_test:
            r = peak_test(hist, float(center), args.bin_width)
            if r is None:
                print(f"  {float(center):>10.5f}  no bins in window          {label}")
                continue
            w = "none" if r["width"] is None else f"{r['width']:.1f} mDa"
            print(f"  {float(center):>10.5f} {r['apex']:>6} {r['bg']:>5} "
                  f"{r['z']:>7.1f} {w:>10}  {label}")
    for lo, hi in args.region:
        show_region(hist, lo, hi, args.bin_width)
    return 0


if __name__ == "__main__":
    sys.exit(main())
