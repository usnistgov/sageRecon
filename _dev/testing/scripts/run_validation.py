#!/usr/bin/env python3
"""
run_validation.py — Sage-Recon regression and anchor gate harness.

Philosophy (from PLAN.md validationPlan + Phase 8):
  An open search has no perfect ground truth for every delta. Validation is:
    Gate 1 (anchor): Δ≈0 dominant peak present and >= 40% of PSMs.
    Gate 2 (ratio):  open-search Oxidation count is consistent with the
                     closed-reference count from step0_expected_anchors.json
                     (ratio must be in a stated band; open search finds fewer
                     because it can't localize confidently — expected).
    Gate 3 (absent): no +57 Da peak above a small count floor in a fixed-C
                     Carbamidomethylated search (its absence as a large delta
                     IS the check; small residual from off-site is accepted).
    Tier 2 (polymer): PEG-spiked file PEG+1H %TIC above threshold, other
                      polymers low — validates the polymer port against the
                      vendored mzSniffer reference.
    Tier 3 (snapshot):byte-equal JSON output for all three full-run files,
                      locking committed outputs as the regression baseline.

This harness does NOT re-run Sage or recon — it works entirely from committed
JSON outputs. That is the design: the outputs are already verified; this script
turns that verification into a permanent automated gate. Re-running would require
gitignored TSV/mzML files and would slow CI for no correctness benefit.

Usage:
    python _dev/testing/scripts/run_validation.py
    python _dev/testing/scripts/run_validation.py --verbose

Exit code 0 = all gates pass. Nonzero = at least one gate failed.

Paths are relative to the repository root. Run from the repo root.
"""

import argparse
import io
import json
import re
import sys
from pathlib import Path

# Force UTF-8 stdout on Windows so delta/approx symbols print cleanly.
if hasattr(sys.stdout, "buffer"):
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8")

# ---------------------------------------------------------------------------
# Gate parameters — documented thresholds with provenance
# ---------------------------------------------------------------------------

# Gate 1: Δ≈0 must be the DOMINANT peak (a rank claim) and clear a loose floor.
#
# RE-BASELINED 2026-08-25, deliberate recorded edit: 35.0 -> 25.0.
# The old 35.0 was fitted just under serum's fixed-C value (42.1%). Step 1 moved
# `full-run/` to an alkylation-agnostic search, where Carbamidomethyl PSMs carry a
# +57 delta instead of sitting at Δ≈0 — serum's 1125 CAM PSMs are 7.3% of 15386.
# Serum fell to 32.2% and the gate failed on a correct result.
#
# Observed, both families:
#   fixed-C   serum 42.1  bcell 64.5  b1906 56.5
#   agnostic  serum 32.2  bcell 58.3  b1906 51.2
#
# 25.0 is NOT fitted to 32.2. The real assertion is now the rank check below —
# Δ≈0 must be the largest peak — which is the axis this project trusts
# (ptm-stratification-design.md: recon agrees with other platforms on rank, not
# magnitude). This percentage is only a backstop against total collapse of the
# unmodified population, so it sits well clear of every observation in both
# families rather than just under the lowest one.
GATE1_MIN_ZERO_PCT = 25.0

# Gate 2: open/closed Oxidation count ratio must fall in this band.
# Observed: b1906 0.45, bcell 0.35, serum 0.61.
# Open search finds fewer because the wide window admits more competing deltas
# (lower specificity at the Ox mass). 0.2–2.0 is deliberately wide because
# different samples have different Ox levels — the check is "roughly consistent,"
# not "exactly X." A ratio outside this band indicates something broke badly
# (e.g. Ox annotator stopped firing, or closed-search anchor file is mismatched).
GATE2_RATIO_LO = 0.2
GATE2_RATIO_HI = 2.0

# Gate 3 is TWO-SIDED, and which side applies depends on the search that made
# the artifact. This was the bug: the gate assumed every artifact came from a
# fixed-C search and asserted "+57 must be small". Step 1 moved `full-run/` to an
# alkylation-agnostic search (`step1-open-*/results.json` records
# `static_mods: {}`), where a LARGE +57 is the tool's headline result. The gate
# then failed on the correct answer for months while PLAN claimed 14/14.
#
# Fixed-C side: +57 must stay small. Observed 86 / 187 / 60 (serum/bcell/b1906).
#   Small residual is real over-alkylation — Phase 8 serum triage proved it is not
#   adds-Gly. Blowing past this means the fixed mod fell out of the Sage config.
# Agnostic side: +57 must be LARGE. Observed 1125 / 3311 / 1253. This is the
#   product claim — an alkylation-agnostic open search surfaces the alkylation
#   chemistry as a delta. If it stops firing, recon's headline result is gone.
# The two observed bands are separated by more than 6x (187 vs 1125), so neither
# threshold is finely tuned.
GATE3_MAX_C57_COUNT = 200
GATE3_MIN_C57_COUNT_AGNOSTIC = 500

# Tier 2: PEG+1H must be the top polymer and above this %TIC.
# Observed: 14.26%. Threshold set at 5% — well below observed, generous for
# instrument/sample variance.
TIER2_PEG_MIN_PCT = 5.0
# All non-PEG polymers must each be below this %TIC.
TIER2_OTHER_MAX_PCT = 2.0

# ---------------------------------------------------------------------------
# Paths (relative to repo root)
# ---------------------------------------------------------------------------

ANCHORS_PATH      = Path("_dev/testing/recon-output/step0_expected_anchors.json")
FULL_RUN_DIR      = Path("_dev/testing/recon-output/full-run")
PEG_RECON_PATH    = Path("_dev/testing/recon-output/tier2_peg_recon.json")
# ⚠ LIVER ADDED 2026-09-01. The standing tripwire had never run on the file the
# entire write-up rests on — the gap was recorded in PLAN and NOTES for three
# sessions. It is fixed by ADDING liver, never by removing the other three:
# serum, bcell and b1906 are BEHAVIOUR CHECKS and each is the only file where
# some behaviour is visible (see NOTES "LIVER IS THE ACTIVE FILE").
FILES             = ["b1906", "bcell", "serum", "liver"]

# Files carrying a step-0 anchor, which Gate 2 compares against. Liver has NO
# anchor: the anchors are an INDEPENDENT measurement of a file's own TSV, and
# inventing one from the report Gate 2 checks would make the gate compare a
# number against itself. Gate 2 therefore reports NOT CHECKED for liver rather
# than passing vacuously. Producing a real liver anchor is its own task.
ANCHORED_FILES    = ["b1906", "bcell", "serum"]

# Tier 3 snapshots exist for these only. Liver's pass-1 TSV lives under
# full-run/liver_search/, not SNAPSHOT_SOURCE, so `_regenerate_discover` cannot
# reach it and a liver snapshot would be a weak-mode self-comparison.
SNAPSHOT_FILES    = ["b1906", "bcell", "serum"]

# Which alkylation family each gated artifact came from. Declared explicitly
# rather than sniffed: the report records the fixed mod recon ASSUMES, not the
# one the Sage search actually used, so it cannot answer this question.
# Verified from the searches themselves:
#   step1-open-*/results.json    -> static_mods {}              -> "agnostic"
#   open-*-full/results.json     -> static_mods {'C': 57.0215}  -> "fixed-c"
FULL_RUN_FAMILY   = "agnostic"

# Tier 3 baselines. Regenerated 2026-08-25 from step1-open-* at
# --min-peak-count 5, matching what `analyze` uses for full-run.
SNAPSHOT_DIR      = Path("_dev/testing/regression-snapshots")
SNAPSHOT_SOURCE   = Path("_dev/testing/search-output")   # gitignored; absent in CI
RECON_BIN         = Path("recon-tool/target/release/recon")
UNIMOD_PATH       = Path("recon-tool/resources/unimod.xml")
SNAPSHOT_MIN_PEAK_COUNT = 5


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def load(path):
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


class Results:
    def __init__(self, verbose):
        self.verbose = verbose
        self.failures = []
        self.passes   = []
        self.skipped  = []

    def ok(self, tag, msg):
        self.passes.append(f"PASS  {tag}: {msg}")
        if self.verbose:
            print(f"  PASS  {tag}: {msg}")

    def fail(self, tag, msg):
        self.failures.append(f"FAIL  {tag}: {msg}")
        print(f"  FAIL  {tag}: {msg}")

    def not_checked(self, tag, msg):
        """A gate that could NOT run, recorded so it cannot pass silently.

        Deliberately counted as neither a pass nor a failure: counting it as a
        pass is the silent-skip trap this project already carries in the Rust
        suite, and counting it as a failure would make an absent input look like
        a regression. It is printed every run and listed in the summary.
        """
        self.skipped.append(f"NOT CHECKED  {tag}: {msg}")
        print(f"  NOT CHECKED  {tag}: {msg}")

    def summary(self):
        total = len(self.passes) + len(self.failures)
        print(f"\n{'='*60}")
        print(f"Results: {len(self.passes)}/{total} passed, "
              f"{len(self.failures)} failed")
        if self.skipped:
            print(f"\n{len(self.skipped)} gate(s) NOT CHECKED (counted as neither):")
            for s_ in self.skipped:
                print(f"  {s_}")
        if self.failures:
            print("\nFailed gates:")
            for f in self.failures:
                print(f"  {f}")
        return len(self.failures)


# ---------------------------------------------------------------------------
# Gate 1 — Δ≈0 dominant
# ---------------------------------------------------------------------------

def gate1_zero_dominant(r, data, fkey):
    peaks = data["mod_discovery"]["peaks"]
    if not peaks:
        r.fail(f"G1/{fkey}", "no peaks at all")
        return
    top = peaks[0]
    delta = top["delta_mass"]
    pct   = top["count_pct"]
    # Top peak must be near-zero
    if abs(delta) > 0.1:
        r.fail(f"G1/{fkey}",
               f"dominant peak is {delta:+.4f} Da (expected ~0.0) — "
               f"Δ≈0 unmodified peak is missing or dethroned")
        return
    # "Dominant" is a RANK claim, and rank is the axis this project trusts
    # (see ptm-stratification-design.md: recon agrees with other platforms on
    # rank, not on magnitude). The old 35% magnitude threshold was baselined on
    # fixed-C runs (42.1 / 64.5 / 56.5%) and failed serum once the search went
    # alkylation-agnostic (32.2%) — a correct result flagged as a fault.
    # The percentage stays only as a LOOSE sanity floor, not a tuned threshold.
    if top is not max(peaks, key=lambda p: p["count"]):
        r.fail(f"G1/{fkey}", "Δ≈0 peak is not the largest peak by count")
        return
    if pct < GATE1_MIN_ZERO_PCT:
        r.fail(f"G1/{fkey}",
               f"Δ≈0 peak is only {pct:.1f}% (sanity floor {GATE1_MIN_ZERO_PCT}%) — "
               f"unusually low unmodified fraction")
        return
    r.ok(f"G1/{fkey}", f"Δ≈0 dominant and rank 1 at {pct:.1f}%")


# ---------------------------------------------------------------------------
# Gate 2 — Oxidation open/closed ratio
# ---------------------------------------------------------------------------

def gate2_oxidation_ratio(r, data, fkey, anchors):
    peaks   = data["mod_discovery"]["peaks"]
    ox_open = next(
        (p["count"] for p in peaks
         if any("Oxidation" in a["name"] for a in p.get("annotations", []))),
        None,
    )
    if ox_open is None:
        r.fail(f"G2/{fkey}", "Oxidation peak absent from mod_discovery output")
        return
    ox_closed = anchors[fkey]["oxidation_M"]
    ratio     = ox_open / ox_closed if ox_closed else float("inf")
    if not (GATE2_RATIO_LO <= ratio <= GATE2_RATIO_HI):
        r.fail(f"G2/{fkey}",
               f"open/closed Ox ratio {ratio:.2f} outside [{GATE2_RATIO_LO}, "
               f"{GATE2_RATIO_HI}] "
               f"(open {ox_open}, closed {ox_closed})")
        return
    r.ok(f"G2/{fkey}",
         f"open/closed Ox ratio {ratio:.2f} in [{GATE2_RATIO_LO}, {GATE2_RATIO_HI}] "
         f"(open {ox_open}, closed {ox_closed})")


# ---------------------------------------------------------------------------
# Gate 3 — +57 absent as large delta (fixed-C search)
# ---------------------------------------------------------------------------

def gate3_c57(r, data, fkey, family):
    """+57 must be SMALL in a fixed-C search and LARGE in an alkylation-agnostic one.

    The old version asserted only the "small" side and read `full-run/`, which step 1
    moved to an alkylation-agnostic search. It therefore failed on the tool's own
    headline result for months while PLAN claimed 14/14 gates passed.
    """
    peaks = data["mod_discovery"]["peaks"]
    c57 = next((p for p in peaks if abs(p["delta_mass"] - 57.0215) < 0.05), None)
    count = c57["count"] if c57 else 0

    if family == "fixed-c":
        if count > GATE3_MAX_C57_COUNT:
            r.fail(f"G3/{fkey}",
                   f"fixed-C search: +57 has {count} PSMs (max {GATE3_MAX_C57_COUNT}) — "
                   f"the Carbamidomethyl fixed mod may have fallen out of the Sage config")
            return
        r.ok(f"G3/{fkey}", f"fixed-C search: +57 small ({count} <= {GATE3_MAX_C57_COUNT})")
        return

    if family == "agnostic":
        if count < GATE3_MIN_C57_COUNT_AGNOSTIC:
            r.fail(f"G3/{fkey}",
                   f"alkylation-agnostic search: +57 has only {count} PSMs "
                   f"(min {GATE3_MIN_C57_COUNT_AGNOSTIC}) — recon's headline result, that an "
                   f"agnostic open search surfaces the alkylation chemistry as a delta, "
                   f"is no longer reproducing")
            return
        r.ok(f"G3/{fkey}", f"agnostic search: +57 surfaced ({count} >= {GATE3_MIN_C57_COUNT_AGNOSTIC})")
        return

    r.fail(f"G3/{fkey}", f"unknown alkylation family {family!r} — cannot choose a side")


# ---------------------------------------------------------------------------
# Tier 2 — PEG polymer spike
# ---------------------------------------------------------------------------

def tier2_polymer(r, peg_recon):
    polymers = sorted(peg_recon["polymers"], key=lambda p: -p["pct_tic"])
    if not polymers:
        r.fail("T2/PEG", "no polymer entries in tier2_peg_recon.json")
        return
    top = polymers[0]
    if "PEG" not in top["name"]:
        r.fail("T2/PEG",
               f"top polymer is '{top['name']}' not PEG — "
               f"polymer port may be misidentifying dominant species")
        return
    if top["pct_tic"] < TIER2_PEG_MIN_PCT:
        r.fail("T2/PEG",
               f"PEG+1H %TIC {top['pct_tic']:.2f}% < threshold {TIER2_PEG_MIN_PCT}%")
        return
    r.ok("T2/PEG", f"{top['name']} %TIC {top['pct_tic']:.2f}% (threshold {TIER2_PEG_MIN_PCT}%)")
    # Other polymers must be low
    for p in polymers[1:]:
        if "PEG" in p["name"]:
            continue  # PEG+2H, PEG+3H are fine if present
        if p["pct_tic"] > TIER2_OTHER_MAX_PCT:
            r.fail("T2/PEG",
                   f"non-PEG polymer '{p['name']}' at {p['pct_tic']:.3f}% "
                   f"> threshold {TIER2_OTHER_MAX_PCT}%")
            return
    r.ok("T2/PEG_others", f"all non-PEG polymers below {TIER2_OTHER_MAX_PCT}%")


# ---------------------------------------------------------------------------
# Tier 3 — snapshot byte-equality
# ---------------------------------------------------------------------------

VOLATILE_KEYS = ("generated_at", "git_commit", "tool_version")


def _strip_volatile(obj):
    if isinstance(obj, dict):
        return {k: _strip_volatile(v) for k, v in obj.items() if k not in VOLATILE_KEYS}
    if isinstance(obj, list):
        return [_strip_volatile(v) for v in obj]
    return obj


def _first_difference(a, b, path=""):
    """Return a human-readable path to the first difference, or None."""
    if type(a) is not type(b):
        return f"{path}: type {type(a).__name__} vs {type(b).__name__}"
    if isinstance(a, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a:
                return f"{path}.{k}: missing in regenerated"
            if k not in b:
                return f"{path}.{k}: missing in snapshot"
            d = _first_difference(a[k], b[k], f"{path}.{k}")
            if d:
                return d
        return None
    if isinstance(a, list):
        if len(a) != len(b):
            return f"{path}: length {len(a)} vs {len(b)}"
        for i, (u, v) in enumerate(zip(a, b)):
            d = _first_difference(u, v, f"{path}[{i}]")
            if d:
                return d
        return None
    if a != b:
        return f"{path}: {a!r} vs {b!r}"
    return None


def _regenerate_discover(root, fkey):
    """Re-run `discover` on the pinned TSV. Returns (obj, note) or (None, why-not).

    This is the strong mode. The previous Tier 3 never did this — it re-parsed the
    same string twice and compared it to itself, so it could only fail on malformed
    JSON. A peak count of 999999 passed it.
    """
    import subprocess, tempfile
    binary = root / RECON_BIN
    tsv    = root / SNAPSHOT_SOURCE / f"step1-open-{fkey}" / "results.sage.tsv"
    unimod = root / UNIMOD_PATH
    if not binary.exists():
        return None, f"{RECON_BIN} not built"
    if not tsv.exists():
        return None, "source TSV absent (gitignored)"
    if not unimod.exists():
        return None, "unimod.xml absent"
    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as tmp:
        out = Path(tmp.name)
    try:
        proc = subprocess.run(
            [str(binary), "discover", "--tsv", str(tsv), "--unimod", str(unimod),
             "--min-peak-count", str(SNAPSHOT_MIN_PEAK_COUNT), "--output", str(out)],
            capture_output=True, text=True, timeout=900,
        )
        if proc.returncode != 0:
            return None, f"discover exited {proc.returncode}: {proc.stderr.strip()[:200]}"
        return json.loads(out.read_text(encoding="utf-8")), "regenerated from TSV"
    except Exception as e:
        return None, f"regeneration failed: {e}"
    finally:
        out.unlink(missing_ok=True)


def tier3_snapshot(r, root):
    """Compare the committed regression baselines against freshly generated output.

    Strong mode: re-run `discover` on the pinned `step1-open-*` TSV and require the
    result to equal the snapshot exactly (volatile keys excluded). Same input and
    same code must give the same bytes, so any difference is a real regression.

    Weak mode (TSV or binary absent, e.g. CI): compare the snapshot against the
    committed `04-discover-min5-*.json`, which is generated from the same TSV at the
    same peak floor. That still catches silent mutation of either file, but it
    cannot catch a code regression. The mode is always printed — a gate that
    silently degrades is how the previous one went unnoticed.
    """
    for fkey in SNAPSHOT_FILES:
        snap_path = root / SNAPSHOT_DIR / f"discover_{fkey}.snapshot.json"
        if not snap_path.exists():
            r.fail(f"T3/{fkey}", f"snapshot missing: {snap_path}")
            continue
        try:
            snapshot = _strip_volatile(json.loads(snap_path.read_text(encoding="utf-8")))
        except Exception as e:
            r.fail(f"T3/{fkey}", f"could not read snapshot: {e}")
            continue

        current, note = _regenerate_discover(root, fkey)
        mode = "regenerated"
        if current is None:
            fallback = root / "_dev/testing/recon-output/2026-08-25-checks" / f"04-discover-min5-{fkey}.json"
            if not fallback.exists():
                r.fail(f"T3/{fkey}", f"cannot verify: {note}, and no committed artifact at {fallback}")
                continue
            try:
                current = json.loads(fallback.read_text(encoding="utf-8"))
            except Exception as e:
                r.fail(f"T3/{fkey}", f"could not read {fallback}: {e}")
                continue
            mode = f"committed artifact ({note})"
        current = _strip_volatile(current)

        diff = _first_difference(current, snapshot)
        if diff:
            r.fail(f"T3/{fkey}",
                   f"output differs from regression baseline [{mode}] — first difference at {diff}")
        else:
            r.ok(f"T3/{fkey}", f"matches regression baseline [{mode}]")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def tier4_psi_ms_cv(r, root):
    """Every mass analyzer term in the pinned PSI-MS CV must be classified by
    recon, with no name drift and no invented accessions.

    Delegates to psi_ms_analyzer_terms.py, which derives the term set from the
    spec rather than restating it. A new analyzer entering the CV, or a rename,
    or the pinned snapshot changing under us, must fail here rather than surface
    as a wrong fragment tolerance later.
    """
    import subprocess
    script = root / "_dev/testing/scripts/psi_ms_analyzer_terms.py"
    if not script.exists():
        r.fail("psi-ms-cv", f"{script} missing")
        return
    proc = subprocess.run([sys.executable, str(script)],
                          capture_output=True, text=True)
    tail = (proc.stdout.strip().splitlines() or ["<no output>"])[-1]
    if proc.returncode != 0:
        r.fail("psi-ms-cv", tail)
        return
    count = re.search(r"all (\d+) CV mass analyzer terms", proc.stdout)
    r.ok("psi-ms-cv",
         f"all {count.group(1) if count else '?'} PSI-MS mass analyzer terms classified, "
         f"pin verified")


def main():
    ap = argparse.ArgumentParser(description="Sage-Recon validation gate harness")
    ap.add_argument("--verbose", "-v", action="store_true",
                    help="print PASS lines in addition to FAIL lines")
    ap.add_argument("--root", default=".",
                    help="repo root (default: current directory)")
    args = ap.parse_args()

    root = Path(args.root)
    r    = Results(args.verbose)

    print("Sage-Recon validation harness")
    print(f"Repo root: {root.resolve()}")
    print()

    # Load shared inputs
    anchors_path = root / ANCHORS_PATH
    if not anchors_path.exists():
        sys.exit(f"FATAL: anchors file not found: {anchors_path}")
    anchors = load(anchors_path)

    peg_path = root / PEG_RECON_PATH
    if not peg_path.exists():
        sys.exit(f"FATAL: PEG recon file not found: {peg_path}")
    peg_recon = load(peg_path)

    # Per-file gates
    for fkey in FILES:
        json_path = root / FULL_RUN_DIR / f"{fkey}.json"
        if not json_path.exists():
            r.fail(f"load/{fkey}", f"full-run JSON not found: {json_path}")
            continue
        data = load(json_path)

        print(f"--- {fkey} ---")
        gate1_zero_dominant(r, data, fkey)
        if fkey in ANCHORED_FILES:
            gate2_oxidation_ratio(r, data, fkey, anchors)
        else:
            r.not_checked(
                f"G2/{fkey}",
                "no step-0 anchor for this file; an anchor derived from the same "
                "report would compare it against itself",
            )
        gate3_c57(r, data, fkey, FULL_RUN_FAMILY)
        print()

    # Tier 2 — polymer
    print("--- Tier 2: PEG polymer spike ---")
    tier2_polymer(r, peg_recon)
    print()

    # Tier 3 — snapshots
    print("--- Tier 3: regression baseline ---")
    tier3_snapshot(r, root)
    print()

    # Tier 4 — the PSI-MS analyzer vocabulary recon claims to cover
    print("--- Tier 4: PSI-MS mass analyzer CV coverage ---")
    tier4_psi_ms_cv(r, root)
    print()

    failures = r.summary()
    sys.exit(failures)


if __name__ == "__main__":
    main()
