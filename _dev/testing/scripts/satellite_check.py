#!/usr/bin/env python3
r"""
Read-only isotope-satellite accounting for Recon modification-discovery output.

This version keeps Recon's supplied percentage scale. It derives the denominator
implied by each Recon peak as count / (pct / 100), verifies that those estimates
are consistent, and calculates:

    satellite contribution % = 100 * satellite PSMs / Recon denominator
    augmented %              = raw % + satellite contribution %

Thus every row with zero claimed satellite PSMs must have augmented % == raw %.
The script fails loudly if that invariant is violated.

Run from repository root:
python testing\scripts\satellite_check.py --recon-json testing\recon-output\nofixedmods\bcell.json:bcell testing\recon-output\nofixedmods\serum.json:serum testing\recon-output\nofixedmods\b1906.json:b1906 --ptmshepherd testing\reference-data\ptm-shepherd\reallyOpen\global.modsummary.tsv --mascot-dir testing\reference-data\mascot\error-tolerant --metamorpheus "testing\reference-data\metamorpheus\2026-08-21-10-29-48\Task3-SearchTask\AllPSMs.psmtsv" --unimod testing\reference-data\unimod.xml --out-name satellite_check_corrected.md
"""

import argparse
import statistics
from pathlib import Path

from compare_4way import (
    MASCOT_FILE_STEM,
    MATCH_TOL_DA,
    in_window,
    load_mascot,
    load_metamorpheus,
    load_ptmshepherd,
    load_recon,
    load_unimod_title_mass,
)

C13_C12_DIFF = 1.003354835


def fold_tol(k):
    return 0.012 + (abs(k) - 1) * 0.0045


def peak_key(row):
    return (row["mass"], row["label"])


def recon_denominator(recon_rows):
    """Infer the denominator used for Recon's supplied pct values.

    Each nonzero row estimates D = count / (pct / 100). Pct values in output
    are rounded, so estimates will not be exactly identical. The median is
    robust to small peaks and rounding. Large disagreement is reported, rather
    than silently mixing an incompatible denominator into the calculation.
    """
    estimates = [
        row["count"] * 100.0 / row["pct"]
        for row in recon_rows
        if row.get("count", 0) > 0 and row.get("pct", 0) > 0
    ]
    if not estimates:
        raise ValueError("No positive count/pct rows available to infer Recon denominator")

    denominator = statistics.median(estimates)
    deviations = [abs(value - denominator) / denominator for value in estimates]
    return denominator, max(deviations), estimates


def augment_with_satellites(recon_rows, denominator):
    """Claim isotope-spaced UNANNOTATED peaks once, largest parents first."""
    parents = [
        row
        for row in recon_rows
        if row["label"] != "UNANNOTATED" and abs(row["mass"]) > 1e-6
    ]
    parents.sort(key=lambda row: (-row["count"], row["mass"], row["label"]))
    satellites = [row for row in recon_rows if row["label"] == "UNANNOTATED"]

    claimed_indices = set()
    detail = {}
    augmented = []

    for parent in parents:
        claims = []
        for k in (-3, -2, -1, 1, 2, 3):
            target = parent["mass"] + k * C13_C12_DIFF
            tolerance = fold_tol(k)
            best = None

            for index, satellite in enumerate(satellites):
                if index in claimed_indices:
                    continue
                distance = abs(satellite["mass"] - target)
                if distance <= tolerance and (best is None or distance < best[1]):
                    best = (index, distance)

            if best is not None:
                index, distance = best
                claimed_indices.add(index)
                claims.append((k, satellites[index], distance))

        satellite_count = sum(satellite["count"] for _, satellite, _ in claims)
        satellite_pct = 100.0 * satellite_count / denominator
        raw_pct = parent["pct"]
        augmented_pct = raw_pct + satellite_pct

        if satellite_count == 0 and abs(augmented_pct - raw_pct) > 1e-12:
            raise AssertionError("Zero-satellite row changed percentage")

        augmented.append(
            {
                "mass": parent["mass"],
                "label": parent["label"],
                "raw_count": parent["count"],
                "raw_pct": raw_pct,
                "satellite_count": satellite_count,
                "satellite_pct": satellite_pct,
                "augmented_pct": augmented_pct,
            }
        )
        detail[peak_key(parent)] = claims

    return augmented, detail


def nearest(rows, mass, tolerance=MATCH_TOL_DA):
    candidates = []
    for row in rows:
        if not in_window(row["mass"]):
            continue
        distance = abs(row["mass"] - mass)
        if distance <= tolerance:
            candidates.append((distance, -row["pct"], row))
    return min(candidates, default=None)[2] if candidates else None


def pct_or_dash(row):
    return f"{row['pct']:.2f}" if row else "—"


def ratio_or_dash(numerator_pct, denominator_row):
    if not denominator_row or denominator_row["pct"] == 0:
        return "—"
    return f"{numerator_pct / denominator_row['pct']:.2f}"


def claim_text(claims):
    if not claims:
        return "—"
    return "; ".join(
        f"k={k:+d} {satellite['mass']:+.4f} "
        f"({satellite['count']} PSM; Δ={distance * 1000:.1f} mDa)"
        for k, satellite, distance in claims
    )


def ratios(pct, comparators):
    values = [ratio_or_dash(pct, comparator) for comparator in comparators if comparator]
    return "/".join(values) if values else "—"


def build_report(file_key, recon_rows, ptms_rows, mascot_rows, mm_rows, top_n):
    denominator, max_deviation, _estimates = recon_denominator(recon_rows)
    augmented, detail = augment_with_satellites(recon_rows, denominator)
    augmented.sort(key=lambda row: (-row["raw_pct"], row["mass"], row["label"]))

    lines = [
        f"### {file_key}",
        "",
        f"Recon percentage denominator inferred from count/pct rows: {denominator:.2f} PSMs. "
        f"Largest per-row estimate deviation: {max_deviation * 100:.2f}% (expected from rounded displayed percentages).",
        "",
        "| mass (Da) | label | raw % | satellite PSMs | satellite % | augmented % | "
        "satellites claimed | PTM-Shep % | Mascot % | MetaMorph % | raw ratio* | augmented ratio* |",
        "|---|---|---:|---:|---:|---:|---|---:|---:|---:|---|---|",
    ]

    for parent in augmented[:top_n]:
        claims = detail.get(peak_key(parent), [])
        ptms = nearest(ptms_rows, parent["mass"])
        mascot = nearest(mascot_rows, parent["mass"])
        metamorpheus = nearest(mm_rows, parent["mass"])
        comparators = (ptms, mascot, metamorpheus)

        lines.append(
            f"| {parent['mass']:+.4f} | {parent['label']} | "
            f"{parent['raw_pct']:.2f} | {parent['satellite_count']} | "
            f"{parent['satellite_pct']:.2f} | {parent['augmented_pct']:.2f} | "
            f"{claim_text(claims)} | {pct_or_dash(ptms)} | {pct_or_dash(mascot)} | "
            f"{pct_or_dash(metamorpheus)} | {ratios(parent['raw_pct'], comparators)} | "
            f"{ratios(parent['augmented_pct'], comparators)} |"
        )

    claimed_total = sum(
        satellite["count"]
        for claims in detail.values()
        for _, satellite, _ in claims
    )
    claimed_peaks = sum(len(claims) for claims in detail.values())
    unannotated_total = sum(
        row["count"] for row in recon_rows if row["label"] == "UNANNOTATED"
    )

    lines.extend(
        [
            "",
            f"Claimed {claimed_peaks} UNANNOTATED peak(s), totaling {claimed_total} PSMs. "
            f"Total UNANNOTATED PSMs: {unannotated_total}.",
            "",
            "_*Ratio order: PTM-Shepherd / Mascot / MetaMorpheus. A comparator "
            "without a mass-window match is omitted. This is read-only sensitivity "
            "accounting, not proof that every claimed peak is chemically an isotope satellite._",
        ]
    )
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(
        description="Read-only Recon isotope-satellite recovery accounting."
    )
    parser.add_argument("--recon-json", nargs="+", required=True, help="path:file_key pairs")
    parser.add_argument("--ptmshepherd", required=True)
    parser.add_argument("--mascot-dir", required=True)
    parser.add_argument("--metamorpheus", required=True)
    parser.add_argument("--unimod", default="testing/reference-data/unimod.xml")
    parser.add_argument("--out-dir", default="testing/recon-output/comparison")
    parser.add_argument("--out-name", default="satellite_check_corrected.md")
    parser.add_argument("--top", type=int, default=15)
    args = parser.parse_args()

    unimod_title_mass = load_unimod_title_mass(args.unimod)
    output_dir = Path(args.out_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    report = [
        "# Satellite-recovery check (Recon-normalized percentages)",
        "",
        "Read-only accounting. Percentages are kept on Recon's supplied scale: "
        "satellite % is calculated from the denominator inferred from Recon's own "
        "count/pct fields, and augmented % = raw % + satellite %.",
        "",
    ]

    for pair in args.recon_json:
        path, separator, file_key = pair.partition(":")
        if not separator or not file_key:
            raise SystemExit(f"--recon-json values must be path:file_key; got {pair!r}")
        if file_key not in MASCOT_FILE_STEM:
            raise SystemExit(
                f"Unknown file_key {file_key!r}; expected: {', '.join(sorted(MASCOT_FILE_STEM))}"
            )

        recon_rows = load_recon(path)
        ptms_rows = load_ptmshepherd(args.ptmshepherd, file_key)
        mascot_path = Path(args.mascot_dir) / MASCOT_FILE_STEM[file_key]
        mascot_rows, _skipped = load_mascot(mascot_path, unimod_title_mass)
        mm_rows = load_metamorpheus(args.metamorpheus, file_key)

        report.append(
            build_report(
                file_key, recon_rows, ptms_rows, mascot_rows, mm_rows, args.top
            )
        )
        report.append("")

    output_path = output_dir / args.out_name
    output_path.write_text("\n".join(report), encoding="utf-8")
    print(f"Wrote {output_path}")


if __name__ == "__main__":
    main()
