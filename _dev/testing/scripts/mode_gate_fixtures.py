#!/usr/bin/env python3
"""Falsification fixtures for `mode_sibling_separation.py`.

AGENTS.md rule: a gate is not done until a deliberately wrong input has made it
fail. This script builds synthetic `09-modes-*.txt` files with a KNOWN answer,
runs the gate on each, and asserts the verdict it must give. It exits non-zero if
any fixture is classified wrongly.

Three fixtures, and each one exists for a reason:

  A  narrow pair, 4.0 mDa apart          -> must report "shredded"
     One population cut at a bin edge. The halves are pulled toward the cut.

  B  true doublet, 19.3 mDa apart        -> must report "INCONCLUSIVE"
     The deamidation / +1 neutron doublet that motivated `Split` at all. The
     ORIGINAL script could not report this: its `sep <= BIN_WIDTH_MDA` filter
     discarded the pair before counting it. This fixture is the regression case
     for that defect.

  C  one broad population over 3 bins    -> must report "INCONCLUSIVE"
     THE POWER LIMIT, made visible. C is a SINGLE population by construction, yet
     it is indistinguishable from B by this test, because each bin's weighted mean
     still sits near its own bin centre. A gate that called C "a real doublet"
     would be lying. "INCONCLUSIVE" is the correct and only honest verdict, and it
     is why the seven wide groups on the real files need a sub-bin histogram.

Usage: mode_gate_fixtures.py
"""

import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
GATE = os.path.join(HERE, "mode_sibling_separation.py")


def row(dm, n, prom, ann):
    """One peak line in the `compare-peak-assignment` output format."""
    return f"  {dm:>10.5f}  {n:>6}  {prom:>6}  {ann:<24} {'-':>7} {'-':>7} {'-':>7} {'-':>6}"


def fixture(path, merge_peaks, split_peaks):
    with open(path, "w", encoding="utf-8") as fh:
        fh.write("--- Merge ---\n")
        for dm, n, ann in merge_peaks:
            fh.write(row(dm, n, n, ann) + "\n")
        fh.write("\n--- Split ---\n")
        for dm, n, ann in split_peaks:
            fh.write(row(dm, n, n, ann) + "\n")


# (name, merge, split, required verdict, what it proves)
CASES = [
    (
        "A-narrow-pair",
        [(10.00000, 400, "Fixture")],
        [(9.99800, 220, "Fixture"), (10.00200, 180, "Fixture")],
        "shredded",
        "4.0 mDa pair: one population cut at a bin edge",
    ),
    (
        "B-true-doublet",
        [(1.00000, 400, "Fixture")],
        [(0.98402, 220, "Fixture"), (1.00336, 180, "Fixture")],
        "INCONCLUSIVE",
        "19.3 mDa deamidation/neutron doublet: the old filter DISCARDED this",
    ),
    (
        "C-broad-single",
        [(0.00000, 900, "Fixture")],
        [(-0.01000, 300, "Fixture"), (0.00000, 300, "Fixture"),
         (0.01000, 300, "Fixture")],
        "INCONCLUSIVE",
        "one broad population over 3 bins: the power limit, not a doublet",
    ),
]


def main():
    failures = 0
    with tempfile.TemporaryDirectory() as tmp:
        for name, merge, split, want, why in CASES:
            path = os.path.join(tmp, f"09-modes-{name}.txt")
            fixture(path, merge, split)
            out = subprocess.run(
                [sys.executable, GATE, path],
                capture_output=True, text=True, check=True,
            ).stdout
            # Read the verdict off the group ROW, not by searching the whole
            # output. Searching the summary text would match the explanatory
            # prose and could pass on an empty table - a could-not-fail check
            # inside the check that exists to prevent could-not-fail checks.
            body = [ln for ln in out.splitlines() if ln.startswith(name)]
            got = body[0].split()[-1] if body else "NO-ROW"
            ok = got == want
            failures += 0 if ok else 1
            print(f"{'PASS' if ok else 'FAIL'}  {name:16} want {want:13} got {got:13}  {why}")
            for ln in body:
                print(f"        {ln.strip()}")
    print()
    if failures:
        print(f"{failures} fixture(s) misclassified. The gate is NOT trustworthy.")
        return 1
    print("All 3 fixtures classified correctly. The gate can report both verdicts.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
