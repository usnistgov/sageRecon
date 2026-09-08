#!/usr/bin/env python3
"""Residue-enrichment support test for delta-mass annotations.

Produces every enrichment number quoted in NOTES 2026-08-25:
  * +57 resolved to Carbamidomethyl (Cys 2.85 / 9.98 / 9.41x; Gly 1.01x)
  * position-aware terminal tests (Gln->pyro-Glu 13.2 / 22.3 / 24.1x)
  * the +58/+59 satellite fingerprint and PSM identity overlap

Reads the committed `testing/search-output/step1-open-*/results.sage.tsv`.
NO re-search is required -- the peptide sequences are already on disk.

Two rules this script exists to enforce, both learned the hard way:
  1. ENRICHMENT, never presence. Gly's T/S/K sit in 91-96% of all peptides, so a
     presence test says "possible" every time. Background is per-file.
  2. Score residues INDIVIDUALLY. Pooling a candidate's Unimod specificity list
     (Carbamidomethyl = C plus hidden Y/T/S/E/D/H/K/U/M) puts the union in ~100%
     of peptides and collapses the signal to 1.0x.

⚠ The verdicts below use the ENRICHMENT RATIO and a 1/p_background ceiling.
That ceiling is real for the ratio but WRONG as a general power guard -- the
ODDS RATIO is not bounded by it. Deamidation at 69-74% background reads
"untestable" here and is OR 3.1-11.6 at BH q < 1e-9. For the decision path use
`tier_report_prototype.py`, which runs Fisher exact + odds ratio + BH and
accepts on OR >= 2 AND q < 0.05. This script is kept because it produces the
per-residue band/background numbers those tests are built from, and the
position-aware terminal comparison. See NOTES 2026-08-25.

Mandatory zeroth step (NOTES "reusable methods"): rank-1 filter before any
delta-band count.
"""
import csv
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
C13_C12_DIFF = 1.003354835          # sage_results.rs:89
BAND_TOL_DA = 0.010
Q_MAX = 0.01
POWER_CEILING_MIN = 2.0             # below this, the test has no dynamic range

FILES = {"serum": "step1-open-serum", "bcell": "step1-open-bcell",
         "b1906": "step1-open-b1906"}


def load_rank1(file_key):
    """Rank-1 target PSMs at spectrum_q < Q_MAX, as (delta, peptide, charge)."""
    tsv = REPO / "testing/search-output" / FILES[file_key] / "results.sage.tsv"
    rows = []
    with open(tsv, newline="", encoding="utf-8") as fh:
        for r in csv.DictReader(fh, delimiter="\t"):
            if r["label"] != "1" or r["rank"] != "1":
                continue
            if float(r["spectrum_q"]) >= Q_MAX:
                continue
            delta = (float(r["expmass"]) - float(r["calcmass"])
                     - float(r["isotope_error"]) * C13_C12_DIFF)
            rows.append((delta, r["peptide"], r["charge"]))
    return rows


def band(rows, delta, tol=BAND_TOL_DA):
    return [(p, c) for dm, p, c in rows if abs(dm - delta) <= tol]


def enrichment(rows, delta, residue, position=None):
    """Enrichment of `residue` in the band vs the file's background.

    position=None  -> "peptide contains residue"
    position="N"   -> "first residue is this"   (use for Any N-term specificities)
    position="C"   -> "last residue is this"
    Returns dict, or None if the band is empty.
    """
    b = band(rows, delta)
    if not b:
        return None
    if position == "N":
        hit = lambda p: p.startswith(residue)
    elif position == "C":
        hit = lambda p: p.endswith(residue)
    else:
        hit = lambda p: any(a in p for a in residue)
    bg = sum(1 for _, p, _ in rows if hit(p)) / len(rows)
    fg = sum(1 for p, _ in b if hit(p)) / len(b)
    ceiling = 1 / bg if bg else 0.0
    if ceiling < POWER_CEILING_MIN:
        verdict = "UNTESTABLE"
    elif fg / bg >= 1.5 if bg else False:
        verdict = "SUPPORTED"
    else:
        verdict = "NO SUPPORT"
    return {"n": len(b), "band_pct": 100 * fg, "bg_pct": 100 * bg,
            "enrichment": (fg / bg if bg else 0.0), "ceiling": ceiling,
            "verdict": verdict}


def satellite_overlap(rows, parent_delta, n_neutrons=1):
    """Do the satellite band's peptides overlap the parent band's, above chance?"""
    sat_delta = parent_delta + n_neutrons * C13_C12_DIFF
    par = {p for p, _ in band(rows, parent_delta)}
    sat = {p for p, _ in band(rows, sat_delta)}
    allp = {p for _, p, _ in rows}
    if not sat or not par:
        return None
    obs = len(sat & par) / len(sat)
    chance = len(par) / len(allp)
    return {"parent_uniq": len(par), "sat_uniq": len(sat),
            "overlap_pct": 100 * obs, "chance_pct": 100 * chance,
            "ratio": obs / chance if chance else 0.0}


def main():
    CAM = 57.02146
    probes = [
        ("Carbamidomethyl", CAM, "C", None),
        ("Gly (competitor)", CAM, "TSK", None),
        ("Carbofuran (competitor)", CAM, "S", None),
        ("Oxidation", 15.994915, "M", None),
        ("Deamidated", 0.984016, "N", None),
        ("Carbamyl (K anywhere)", 43.005814, "K", None),
        ("Gln->pyro-Glu (N-term Q)", -17.026549, "Q", "N"),
        ("Glu->pyro-Glu (N-term E)", -18.010565, "E", "N"),
    ]
    for fkey in FILES:
        rows = load_rank1(fkey)
        print(f"\n===== {fkey}: {len(rows)} rank-1 target PSMs at q<{Q_MAX} =====")
        for label, delta, res, pos in probes:
            r = enrichment(rows, delta, res, pos)
            if r is None:
                print(f"  {label:28s} band empty")
                continue
            print(f"  {label:28s} n={r['n']:5d}  band={r['band_pct']:5.1f}%  "
                  f"bg={r['bg_pct']:5.1f}%  enrich={r['enrichment']:6.2f}x  "
                  f"ceiling={r['ceiling']:6.1f}x  {r['verdict']}")
        print("  -- satellite fingerprint (same Cys enrichment as the +57 parent?)")
        for n in (1, 2):
            r = enrichment(rows, CAM + n * C13_C12_DIFF, "C", None)
            if r:
                print(f"     +{n} C13 ({CAM + n * C13_C12_DIFF:+.4f}) n={r['n']:5d}  "
                      f"Cys enrich={r['enrichment']:6.2f}x")
        print("  -- satellite PSM identity overlap with the parent band")
        for n in (1, 2):
            o = satellite_overlap(rows, CAM, n)
            if o:
                print(f"     +{n} C13  {o['overlap_pct']:5.1f}% of satellite peptides "
                      f"also in +57 band (chance {o['chance_pct']:4.1f}%) "
                      f"-> {o['ratio']:5.1f}x")


if __name__ == "__main__":
    main()
