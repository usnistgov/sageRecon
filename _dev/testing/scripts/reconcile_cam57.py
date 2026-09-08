#!/usr/bin/env python3
"""Reconcile recon's open-search +57 (Carbamidomethyl-C) peak count against
an independent MSFragger tight search with Cys+57 as a VARIABLE mod.

Ground truth for PLAN step 1's "+57 PSM count reconciliation" item. Reads
testing/reference-data/msfragger/strictTrypVarCAM/*/psm.tsv (variable
Cys+57, strict trypsin, current as of 2026-08-24) and counts confident PSMs
carrying at least one Carbamidomethyl(C) tag in "Assigned Modifications".
Compares against recon's mod_discovery peak counts already recorded in
testing/recon-output/full-run/{file}.json (NOTES "+57 PSM count
reconciliation — resolved for the recon side").

Usage: python3 testing/scripts/reconcile_cam57.py
"""
import csv
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MSFRAGGER_DIR = ROOT / "reference-data" / "msfragger" / "strictTrypVarCAM"
RECON_DIR = ROOT / "recon-output" / "full-run"

FILES = {
    "serum": "2019_4_9_909c_0311_1",
    "bcell": "B_naive_01steady_state_1",
    "b1906": "b1906_293T_proteinID_01A_QE3_122212_1",
}


def count_cam57(psm_path: Path):
    total = 0
    cam = 0
    with psm_path.open(newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh, delimiter="\t")
        for row in reader:
            total += 1
            if "C(57.02" in row["Assigned Modifications"]:
                cam += 1
    return total, cam


def recon_peak_count(label: str):
    with (RECON_DIR / f"{label}.json").open(encoding="utf-8") as fh:
        report = json.load(fh)
    for peak in report["mod_discovery"]["peaks"]:
        if abs(peak["delta_mass"] - 57.02) < 0.1:
            return peak["count"], peak["count_pct"]
    return None, None


def main():
    print(f"{'file':<8} {'MSFragger total':>16} {'MSFragger +57':>14} {'MSFragger %':>12} "
          f"{'recon +57':>10} {'recon %':>9} {'ratio (MSF/recon)':>18}")
    for label, folder in FILES.items():
        psm_path = MSFRAGGER_DIR / folder / "psm.tsv"
        total, cam = count_cam57(psm_path)
        msf_pct = 100.0 * cam / total if total else 0.0
        recon_count, recon_pct = recon_peak_count(label)
        ratio = cam / recon_count if recon_count else float("nan")
        print(f"{label:<8} {total:>16} {cam:>14} {msf_pct:>11.2f}% "
              f"{recon_count:>10} {recon_pct:>8.2f}% {ratio:>17.2f}x")


if __name__ == "__main__":
    main()
