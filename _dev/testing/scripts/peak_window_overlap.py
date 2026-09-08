#!/usr/bin/env python3
"""Show that recon peaks can claim the same PSM twice.

Two earlier scripts asked "do the peak counts in a region exceed the PSMs in
that region?" and both gave a false answer, because the region boundary is not
where the defect lives. This script asks a different question, and one that
needs no raw TSV: does the PSM collection WINDOW of one peak overlap the window
of another peak?

`detect_peaks_with_prominence` gives every accepted peak the PSMs within
+/- effective_merge_tolerance of its bin center. That window is two bin widths
wide, while two peaks only need to be more than one bin width apart to survive
the `too_close` guard. Overlapping windows means shared PSMs.

Input is a `discover` JSON (histogram + peaks), which is committed.
Usage: peak_window_overlap.py <discover.json> [<discover.json> ...]
"""

import json
import sys


def bin_center_of(peak, histogram, bin_width):
    """Recover the peak's originating bin.

    `delta_mass` is intensity weighted, so it is not the bin center. Prominence
    is carried straight from the originating bin's count, so match on that.
    """
    candidates = [
        b for b in histogram
        if b["count"] == peak["prominence"]
        and abs(b["bin_center"] - peak["delta_mass"]) <= 2 * bin_width
    ]
    if len(candidates) == 1:
        return candidates[0]["bin_center"], "prominence==bin count"
    # Fall back to the nearest bin.
    nearest = min(histogram, key=lambda b: abs(b["bin_center"] - peak["delta_mass"]))
    return nearest["bin_center"], "nearest bin (prominence ambiguous)"


def psms_in(histogram, lo, hi, bin_width):
    """PSMs held by bins whose span intersects [lo, hi]. Upper bound: bins are
    only known at bin resolution, so a partly covered bin counts in full."""
    half = bin_width / 2.0
    return sum(
        b["count"] for b in histogram
        if b["bin_center"] + half > lo and b["bin_center"] - half < hi
    )


def main(paths):
    bad = 0
    for path in paths:
        d = json.load(open(path))
        histogram = d["histogram"]
        peaks = d["peaks"]
        bin_width = d["summary"]["bin_width_da"]
        # Mirrors detect_peaks_with_prominence.
        merge_tol = max(bin_width, bin_width)

        print("=" * 78)
        print(f"{path}")
        print(f"  bin_width={bin_width} Da   collection window=+/-{merge_tol} Da "
              f"({2 * merge_tol / bin_width:.0f} bin widths wide)")

        centers = []
        for p in peaks:
            c, how = bin_center_of(p, histogram, bin_width)
            centers.append((c, p, how))
        centers.sort(key=lambda t: t[0])

        found = False
        for i in range(len(centers) - 1):
            c1, p1, how1 = centers[i]
            c2, p2, how2 = centers[i + 1]
            gap = c2 - c1
            shared_width = 2 * merge_tol - gap
            if shared_width <= bin_width / 2.0:
                continue  # windows are disjoint, or touch at a single point
            found = True
            bad += 1
            lo, hi = c1 - merge_tol, c2 + merge_tol
            available = psms_in(histogram, lo, hi, bin_width)
            claimed = p1["count"] + p2["count"]
            print(f"  OVERLAP  peaks at bin {c1:.4f} and {c2:.4f} Da")
            print(f"           gap {gap!r} Da vs merge tolerance {merge_tol!r} Da")
            print(f"           gap <= tolerance ? {gap <= merge_tol}   "
                  f"(exact arithmetic says the adjacent-bin gap IS the tolerance)")
            print(f"           windows [{c1 - merge_tol:.4f},{c1 + merge_tol:.4f}] and "
                  f"[{c2 - merge_tol:.4f},{c2 + merge_tol:.4f}] "
                  f"share [{c2 - merge_tol:.4f},{c1 + merge_tol:.4f}]")
            lower_bound = max(0, claimed - available)
            print(f"           claimed by the two peaks   {claimed}")
            print(f"           PSMs available in [{lo:.4f},{hi:.4f}]  {available} (upper bound)")
            if lower_bound:
                print(f"           duplicate PSM claims       >= {lower_bound}")
            else:
                print(f"           duplicate PSM claims       >= 0 "
                      f"(windows overlap, but bin-resolution bound is uninformative here)")
            flag = "" if "ambiguous" not in how1 + how2 else "   <-- UNCERTAIN"
            print(f"           bin recovered by: {how1} / {how2}{flag}")
        if not found:
            print("  no overlapping peak windows")
    print("=" * 78)
    print(f"RESULT: {bad} overlapping peak-window pair(s) across {len(paths)} file(s).")
    return 1 if bad else 0


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    sys.exit(main(sys.argv[1:]))
