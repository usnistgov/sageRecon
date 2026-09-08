#!/usr/bin/env python3
"""
Compare predicted +57 (Carbamidomethyl) from Recon mod-discovery
vs verified +57 variable-mod PSMs from tight Sage searches.

Per file (serum / b1906 / bcell):
- Predicted: Recon mod-discovery peak at ~57.0215 Da (mass, count, pct).
- Verified: tight results.sage.tsv PSMs with +57.021465 in peptide,
           passing standard PSM FDR thresholds (peptide_q <= 0.01,
           protein_q <= 0.01).

Usage example (from repo root):

python testing/scripts/compare_predicted_vs_verified_cam57.py \
  --recon-json testing/recon-output/full-run/bcell.json:bcell \
                testing/recon-output/full-run/serum.json:serum \
                testing/recon-output/full-run/b1906.json:b1906 \
  --tight-root testing/search-output/ptmRecovery
"""

import argparse
import json
from pathlib import Path
import re

import pandas as pd

# --- Recon adapter (same shape as compare_mod_discovery) -------------------

def load_recon(json_path: Path):
    """
    Sage-Recon report JSON -> list of {mass, count, pct, label}.

    Accepts both:
    - analyze unified report: report["mod_discovery"]["peaks"]
    - discover standalone JSON: report["peaks"]
    """
    with json_path.open(encoding="utf-8") as fh:
        report = json.load(fh)
    if "mod_discovery" in report:
        peaks = report["mod_discovery"]["peaks"]
    else:
        peaks = report["peaks"]
    rows = []
    for peak in peaks:
        anns = peak.get("annotations", [])
        label = anns[0]["name"] if anns else "UNANNOTATED"
        if peak.get("unannotated"):
            label = "UNANNOTATED"
        rows.append({
            "mass": float(peak["delta_mass"]),
            "count": int(peak["count"]),
            "pct": float(peak["count_pct"]),
            "label": label,
        })
    return rows

# --- Tight results adapter -------------------------------------------------

CAM_MASS = 57.0215
CAM_MASS_TOL = 0.01  # 56.99–57.03 window for Recon peak

CAM_TAG_RE = re.compile(r"\[\+57\.021465\]")

def load_tight_results(tight_dir: Path, sample_key: str):
    """
    Load tight results.sage.tsv for a given sample_key.

    We assume folder names:
      serum  -> tight909c
      b1906  -> tightB1906
      bcell  -> tightBcell
    Adjust mapping below if you change folder names.
    """
    key_to_folder = {
        "serum": "tight909c",
        "b1906": "tightB1906",
        "bcell": "tightBcell",
    }
    folder = key_to_folder[sample_key]
    path = tight_dir / folder / "results.sage.tsv"
    df = pd.read_csv(path, sep="\t")
    return df

def count_verified_cam(df):
    """
    Count verified +57 PSMs in tight results.sage.tsv:

    - Confident PSMs: peptide_q <= 0.01 AND protein_q <= 0.01
    - Peptide sequence contains [+57.021465] (Carbamidomethyl Cys)
    """
    df_conf = df[(df["peptide_q"] <= 0.01) & (df["protein_q"] <= 0.01)].copy()
    mask_cam = df_conf["peptide"].str.contains(CAM_TAG_RE)
    return int(mask_cam.sum())

# --- Recon +57 helpers -----------------------------------------------------

def find_recon_cam_peak(rows):
    """
    Given Recon mod-discovery rows for one file, find the +57 peak.

    Strategy:
    - First, look for any row whose label contains 'Carbamidomethyl'
      and mass is near CAM_MASS.
    - If none, just take the highest-count peak within the [CAM_MASS ± tol] window.
    """
    candidates = [r for r in rows
                  if abs(r["mass"] - CAM_MASS) <= CAM_MASS_TOL]

    if not candidates:
        return None

    labeled = [r for r in candidates if "Carbamidomethyl" in r["label"]]
    if labeled:
        # Take the one with the highest count among labeled candidates
        return max(labeled, key=lambda r: r["count"])

    # Fall back: highest-count candidate in mass window
    return max(candidates, key=lambda r: r["count"])

# --- Main ------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(
        description="Compare Recon +57 peak vs tight variable-mod +57 PSMs."
    )
    ap.add_argument(
        "--recon-json", nargs="+", required=True,
        help="Path:file_key pairs (e.g. testing/recon-output/full-run/bcell.json:bcell)"
    )
    ap.add_argument(
        "--tight-root", required=True,
        help="Root directory containing tight* folders (e.g. testing/search-output/ptmRecovery)"
    )
    args = ap.parse_args()

    tight_root = Path(args.tight_root)

    results = []  # list of dicts per file_key

    for pair in args.recon_json:
        path_str, _, file_key = pair.partition(":")
        if not file_key:
            raise SystemExit(f"--recon-json needs path:file_key, got {pair!r}")
        json_path = Path(path_str)

        recon_rows = load_recon(json_path)
        cam_peak = find_recon_cam_peak(recon_rows)
        if cam_peak is None:
            print(f"[WARN] No Recon +57 peak found for {file_key} "
                  f"(mass window {CAM_MASS} ± {CAM_MASS_TOL} Da).")
            continue

        # Tight results: verified +57
        tight_df = load_tight_results(tight_root, file_key)
        verified_cam = count_verified_cam(tight_df)

        results.append({
            "file_key": file_key,
            "recon_cam_mass": cam_peak["mass"],
            "recon_cam_count": cam_peak["count"],
            "recon_cam_pct": cam_peak["pct"],
            "verified_cam_psms": verified_cam,
            "verified_vs_predicted_ratio":
                (verified_cam / cam_peak["count"]) if cam_peak["count"] > 0 else None,
        })

    # Print summary table
    print("=== Predicted vs verified +57 (Carbamidomethyl) ===")
    print("file_key\trecon_mass\trecon_count\trecon_pct\tverified_cam_psms\tverified/predicted")
    for r in results:
        ratio = r["verified_vs_predicted_ratio"]
        ratio_str = f"{ratio:.2f}" if ratio is not None else "NA"
        print(
            f"{r['file_key']}\t"
            f"{r['recon_cam_mass']:.4f}\t"
            f"{r['recon_cam_count']}\t"
            f"{r['recon_cam_pct']:.2f}\t"
            f"{r['verified_cam_psms']}\t"
            f"{ratio_str}"
        )

if __name__ == "__main__":
    main()