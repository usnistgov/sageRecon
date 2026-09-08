#!/usr/bin/env python3
"""Summarize known-PTM prevalence from MSFragger/FragPipe psm.tsv files.

Ground-truth input for PLAN step 1 ("lock the truth"). Reads the psm.tsv
files under testing/reference-data/msfragger/{strictTryp,semiTryp}/, all
three raw files, and reports per-file/per-tier: total PSMs, Oxidation(M)
count and %, N-term Acetyl count and %, and (semiTryp only) the enzymatic
termini split (fully/semi/non-tryptic).

Search config for both tiers: fixed Cys+57.02146 (Carbamidomethyl), variable
Met oxidation (max 3/peptide) and protein N-term acetyl (max 1/peptide).
precursor_true_tolerance=20 ppm, fragment_mass_tolerance auto-calibrated
20->10 ppm (see fragger.params / log_*.txt in each tier folder). Cys+57 is
FIXED in this config, so this script cannot answer "how many PSMs carry
+57" as a discoverable delta -- that question needs a variable-+57 tight
search, a different config from this one. Do not conflate the two.

Usage: python3 testing/scripts/msfragger_prevalence_summary.py
"""
import csv
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "reference-data" / "msfragger"

FILES = {
    "serum (909c)": "2019_4_9_909c_0311_1",
    "bcell (B.naive)": "B_naive_01steady_state_1",
    "b1906": "b1906_293T_proteinID_01A_QE3_122212_1",
}

TIERS = ["strictTryp", "semiTryp"]


def load_psm(path: Path):
    with path.open(newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh, delimiter="\t")
        return list(reader)


def summarize(rows):
    total = len(rows)
    ox_m = sum(1 for r in rows if "(15.9949)" in r["Assigned Modifications"])
    nterm_ac = sum(1 for r in rows if "N-term(42.0106)" in r["Assigned Modifications"])
    termini = {"2": 0, "1": 0, "0": 0}
    for r in rows:
        t = r.get("Number of Enzymatic Termini", "").strip()
        if t in termini:
            termini[t] += 1
    return {
        "total": total,
        "ox_m": ox_m,
        "ox_m_pct": 100.0 * ox_m / total if total else 0.0,
        "nterm_ac": nterm_ac,
        "nterm_ac_pct": 100.0 * nterm_ac / total if total else 0.0,
        "termini": termini,
    }


def main():
    for tier in TIERS:
        print(f"\n=== {tier} ===")
        for label, folder in FILES.items():
            psm_path = ROOT / tier / folder / "psm.tsv"
            rows = load_psm(psm_path)
            s = summarize(rows)
            print(f"{label}: {s['total']} PSMs, "
                  f"Ox(M) {s['ox_m']} ({s['ox_m_pct']:.2f}%), "
                  f"N-term-Ac {s['nterm_ac']} ({s['nterm_ac_pct']:.2f}%)")
            if tier == "semiTryp":
                t = s["termini"]
                total = s["total"]
                fully = t["2"]
                semi = t["1"]
                non = t["0"]
                print(f"    termini: fully-tryptic {fully} ({100.0*fully/total:.2f}%), "
                      f"semi-tryptic {semi} ({100.0*semi/total:.2f}%), "
                      f"non-tryptic {non} ({100.0*non/total:.2f}%)")


if __name__ == "__main__":
    main()
