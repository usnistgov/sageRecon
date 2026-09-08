#!/usr/bin/env python3
"""Decide between the Merge and Split peak-assignment modes, from their own output.

The question: when Split reports two peaks where Merge reports one, are those two
REAL modes, or one population cut at a bin edge?

The test is mechanical, and it does not need composition, annotation, or any
external reference.

  Peaks sit on a bin grid of `bin_width`. If two adjacent bins hold two genuinely
  distinct populations, each peak's intensity-weighted mean sits near its own bin
  centre, so the two means are separated by ABOUT ONE BIN WIDTH.

  If instead a single population straddles a bin edge, nearest-centre assignment
  cuts it in half. Each half's mean is pulled toward the shared cut, so the two
  means come out CLOSER TOGETHER THAN ONE BIN WIDTH.

So: a consecutive gap well below the bin width PROVES Split is shredding one peak,
and Merge is right. A gap at or above the bin width does NOT prove a doublet - one
broad population smeared over adjacent bins gives the same signature. That case is
INCONCLUSIVE here and needs a sub-bin histogram. See the power limit in `main`.

The deamidation / isotope doublet that motivated Split at all is 19.3 mDa
(1.003355 - 0.984016), which is ~2 bin widths. If Split were resolving that, the
separations would show it.

Usage: mode_sibling_separation.py <09-modes-*.txt> [...]
"""

import itertools
import re
import sys

BIN_WIDTH_MDA = 10.0
DOUBLET_MDA = 19.3

ROW = re.compile(
    r"^\s+(-?\d+\.\d+)\s+(\d+)\s+(\d+)\s+(.{1,24}?)\s\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s*$"
)


def parse(path):
    modes, cur = {}, None
    for line in open(path, encoding="utf-8"):
        header = re.match(r"^--- (\w+) ---", line)
        if header:
            cur = header.group(1)
            modes[cur] = []
            continue
        if cur is None:
            continue
        m = ROW.match(line)
        if m:
            modes[cur].append(
                dict(
                    dm=float(m.group(1)),
                    n=int(m.group(2)),
                    ann=m.group(4).strip(),
                    bg=m.group(7),
                    enr=m.group(8),
                )
            )
    return modes


def ceiling(bg):
    """Max achievable enrichment is 1/background. Without it, an enrichment
    number cannot be read: 1.3 may be at ceiling or may be nothing."""
    try:
        v = float(bg.rstrip("%")) / 100.0
        return f"{1 / v:.2f}" if v > 0 else "-"
    except ValueError:
        return "-"


def main(paths):
    """Group Split peaks by the Merge peak they belong to, then measure the gaps.

    CORRECTED TWICE. Both corrections are recorded because the shape of each
    error is the lesson.

    1. 2026-08-25 (first). The original selected candidate pairs with
       `if not 0 < sep <= BIN_WIDTH_MDA: continue`, i.e. it discarded every pair
       wider than one bin width BEFORE counting how many were at or above one bin
       width. That count could only ever be zero and the "real doublet" branch was
       unreachable. Its headline, "0 of 24 pairs reach a full bin width", was
       circular.

    2. 2026-08-25 (second). The repair then gated on group SPREAD with no upper
       cap. Spread is wrong for a group of more than two members: a group that
       occupies k adjacent bins has a spread of about (k-1) bin widths BY
       CONSTRUCTION, whether it holds k real populations or one broad one. On the
       real files this over-triggered — `bcell` host 0.93035 has consecutive gaps
       of 9.6 and 9.7 mDa, both BELOW one bin width, which is the shredding
       verdict, but its 19.2 mDa spread flagged it as a candidate doublet. Spread
       contradicted the mechanism this script is built on.

       The gate is now on CONSECUTIVE GAPS, evaluated pair by pair. Spread is
       still printed, but it decides nothing.

    KNOWN POWER LIMIT — state it before quoting any verdict. This test can PROVE
    shredding and cannot PROVE a doublet. A single broad population smeared over
    three bins also puts each bin's weighted mean near its own bin centre, so it
    produces gaps of about one bin width, exactly like three real populations.
    A gap at or above one bin width is therefore INCONCLUSIVE here and needs a
    sub-bin histogram of the raw delta masses. Demonstrated by fixture C in
    `mode_gate_fixtures.py`.
    """
    print(f"{'file':8} {'annotation':22} {'k':>3} {'spread':>7} {'gaps (mDa)':>22} "
          f"{'n(first,last)':>14} {'verdict':>12}")
    print("=" * 100)
    all_gaps, spreads, shredded, inconclusive = [], [], 0, 0
    for path in paths:
        name = re.sub(r".*09-modes-|\.txt$", "", path)
        modes = parse(path)
        merge = modes.get("Merge", [])
        split = modes.get("Split", [])
        if not merge or not split:
            print(f"{name:8} could not parse both modes from {path}")
            continue
        # Assign each Split peak to its nearest Merge peak.
        groups = {}
        for sp in split:
            host = min(merge, key=lambda m: abs(m["dm"] - sp["dm"]))
            groups.setdefault(round(host["dm"], 6), []).append(sp)
        for host_dm, members in sorted(groups.items()):
            if len(members) < 2:
                continue
            members.sort(key=lambda p: p["dm"])
            gaps = [(members[i + 1]["dm"] - members[i]["dm"]) * 1000.0
                    for i in range(len(members) - 1)]
            spread = (members[-1]["dm"] - members[0]["dm"]) * 1000.0
            a, b = members[0], members[-1]
            wide = [g for g in gaps if g >= BIN_WIDTH_MDA]
            if wide:
                verdict = "INCONCLUSIVE"
                inconclusive += 1
            else:
                verdict = "shredded"
                shredded += 1
            gap_txt = " ".join(f"{g:.1f}" for g in gaps)
            print(f"{name:8} {a['ann'][:22]:22} {len(members):3} {spread:7.1f} "
                  f"{gap_txt:>22} {a['n']:6},{b['n']:<7} {verdict:>12}")
            all_gaps.extend(gaps)
            spreads.append(spread)
    print("=" * 100)
    if not all_gaps:
        print("No Merge peak holds two or more Split peaks. Nothing to decide.")
        return 0
    all_gaps.sort()
    spreads.sort()
    wide_gaps = sum(1 for g in all_gaps if g >= BIN_WIDTH_MDA)
    near_doublet = sum(1 for g in all_gaps if g >= DOUBLET_MDA * 0.75)
    print(f"  split groups                    {len(spreads)}")
    print(f"  consecutive gaps                {len(all_gaps)}")
    print(f"  gap min/median/max              "
          f"{all_gaps[0]:.1f} / {all_gaps[len(all_gaps)//2]:.1f} / {all_gaps[-1]:.1f} mDa")
    print(f"  spread min/median/max           "
          f"{spreads[0]:.1f} / {spreads[len(spreads)//2]:.1f} / {spreads[-1]:.1f} mDa  (not a gate)")
    print(f"  bin width                       {BIN_WIDTH_MDA:.1f} mDa")
    print(f"  deamidation/isotope doublet     {DOUBLET_MDA:.1f} mDa")
    print(f"  gaps >= one bin width           {wide_gaps}")
    print(f"  gaps >= 75% of the doublet      {near_doublet}")
    print(f"  groups: shredded / inconclusive {shredded} / {inconclusive}")
    print()
    if inconclusive == 0:
        print("  VERDICT: every consecutive gap is closer than one bin width. Split is")
        print("  cutting single populations at bin edges, not resolving doublets.")
        print("  Merge is the correct mode.")
    else:
        print("  VERDICT: some consecutive gaps reach a full bin width. This test")
        print("  CANNOT tell those from one broad population smeared over adjacent")
        print("  bins - see the power limit in the docstring. Resolve them with a")
        print("  sub-bin histogram of the raw delta masses before choosing a mode.")
    return 0


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    sys.exit(main(sys.argv[1:]))
