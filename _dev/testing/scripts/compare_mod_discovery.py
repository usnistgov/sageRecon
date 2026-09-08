#!/usr/bin/env python3
"""
Mod-discovery cross-comparison: Sage-Recon vs other tools.

OBJECTIVE benchmark (tool-vs-tool), NOT a gate and NOT a comparison to the
tool's author. Sage-Recon is the fixed reference column; each other tool is
adapted into a generic mod-table and compared against it. See NOTES "Software
comparison is tool-vs-tool" (locked). Divergence is expected (different engines,
FDR, windows) — the six methodology deltas are in
testing/reference-data/ptm-shepherd/README.md.

N-tool shape: add a new adapter function to compare another tool (Mascot,
Preview). The alignment + metric core does not change.

Metric family (per file, in the shared mass window, percentages as currency):
  - presence: is a peak/mod found by both tools at the same mass (+/- tol)?
  - rank:     ordering by PSM count among matched mods.
  - prevalence: per-file percent-of-PSMs for each matched mod, side by side.

Usage (paths relative to repo root):
    # vs PTM-Shepherd (open OR reallyOpen — just point at the modsummary)
    python testing/scripts/compare_mod_discovery.py \
        --recon-json testing/recon-output/full-run/bcell.json:bcell \
                     testing/recon-output/full-run/serum.json:serum \
                     testing/recon-output/full-run/b1906.json:b1906 \
        --ptmshepherd testing/reference-data/ptm-shepherd/reallyOpen/global.modsummary.tsv \
        --tool-name "PTM-Shepherd (reallyOpen)" \
        --out-name recon_vs_ptmshepherd_reallyOpen.md

    # vs Mascot error-tolerant (name+site output -> resolved to mass, sites rolled up)
    python testing/scripts/compare_mod_discovery.py \
        --recon-json .../bcell.json:bcell .../serum.json:serum .../b1906.json:b1906 \
        --mascot-dir testing/reference-data/mascot/error-tolerant \
        --tool-name "Mascot (error-tolerant)" --out-name recon_vs_mascot.md

The --recon-json args are path:file_key pairs; file_key must match the per-file
column/filename mapping below.
"""

import argparse
import csv
import json
import os
import xml.etree.ElementTree as ET
from pathlib import Path
from scipy.stats import spearmanr

# --- Configuration ---------------------------------------------------------

# Match tolerance for calling two masses "the same mod".
# 0.015 Da (vs prior 0.01) adds ~5 mDa headroom for residual m/z-dependent drift
# after scalar apex_offset correction (serum pre-cal +2.5 ppm ≈ 2.5 mDa at 1000 Da).
MATCH_TOL_DA = 0.015

# Shared comparison window: true delta axis overlap of both tools' searches.
# Our Sage open config da:[-500,100] → delta window is −100..+500 (Lazear confirmed,
# 2026-08-17). PTM-Shepherd window is −150/+500. True overlap = −100..+500.
# Prior script had WINDOW_LO=-150, WINDOW_HI=100 — both wrong; the bug that silently
# dropped +100..+500 from the comparison. Verified fix: output tables MUST show peaks
# above +100 Da when both tools have them (the new spot-check below asserts this).
WINDOW_LO, WINDOW_HI = -100.0, 500.0

# Map our file_key -> PTM-Shepherd modsummary column stem (their shortened names).
# One report per file; never blended (NOTES locked).
PTMS_COLUMN_STEM = {
    "serum": "2019_4_9_909c_0311_1",
    "bcell": "Bnaive_01steady_state_1",
    "b1906": "b1906_1",
}

# Map our file_key -> Mascot error-tolerant summary filename stem (Ben's hand-copied
# per-file exports). One file per key; never blended (NOTES locked).
MASCOT_FILE_STEM = {
    "serum": "MascotErrorTol-909c.txt",
    "bcell": "MascotErrorTol-Bcell.txt",
    "b1906": "MascotErrorTol-b1906.txt",
}


# --- Generic mod-table -----------------------------------------------------
# A tool's output adapts into: list of {mass, count, pct, label} per file.

def load_recon(json_path):
    """Adapter: Sage-Recon report JSON -> generic mod-table (single file).

    Accepts BOTH output shapes: the `analyze` unified report
    (peaks under report["mod_discovery"]["peaks"]) and the `discover`
    standalone JSON (peaks at the top level report["peaks"]). Same peak
    fields either way.
    """
    with open(json_path, encoding="utf-8") as fh:
        report = json.load(fh)
    if "mod_discovery" in report:
        peaks = report["mod_discovery"]["peaks"]
    else:
        peaks = report["peaks"]
    rows = []
    for peak in peaks:
        # Prefer the first annotation name; fall back to UNANNOTATED.
        anns = peak.get("annotations", [])
        label = anns[0]["name"] if anns else "UNANNOTATED"
        if peak.get("unannotated"):
            label = "UNANNOTATED"
        rows.append({
            "mass": peak["delta_mass"],
            "count": peak["count"],
            "pct": peak["count_pct"],
            "label": label,
        })
    return rows


def load_ptmshepherd(tsv_path, file_key):
    """Adapter: PTM-Shepherd global.modsummary.tsv -> generic mod-table (one file)."""
    stem = PTMS_COLUMN_STEM[file_key]
    psm_col = f"{stem}_PSMs"
    pct_col = f"{stem}_percent_PSMs"
    rows = []
    with open(tsv_path, encoding="utf-8", newline="") as fh:
        r = csv.DictReader(fh, delimiter="\t")
        if psm_col not in r.fieldnames or pct_col not in r.fieldnames:
            raise SystemExit(f"PTM-Shepherd TSV missing {psm_col}/{pct_col}")
        for row in r:
            try:
                mass = float(row["Mass Shift"])
                count = int(float(row[psm_col]))
                pct = float(row[pct_col])
            except (ValueError, KeyError):
                continue
            if count <= 0:
                continue
            rows.append({
                "mass": mass,
                "count": count,
                "pct": pct,
                "label": row["Modification"],
            })
    return rows


# --- Mascot error-tolerant adapter -----------------------------------------
# Mascot reports mods by Unimod NAME + SITE (not mass), and splits ONE mod mass
# across many site rows (e.g. Carbamidomethyl on C / N-term / Y / D / E / H —
# the C row is the intended fix-mod, the rest are OVER-ALKYLATION off-site).
# Our tool and PTM-Shepherd report +57 as a SINGLE un-localized mass peak (the
# no-per-residue-localization lock). To compare like-for-like the adapter MUST:
#   1. resolve (name) -> monoisotopic mass via Unimod, and
#   2. ROLL UP all site rows sharing a mass into one row (sum ET counts),
# else Mascot's C-only row understates the true +57 population and the
# comparison is apples-to-oranges. The multi-site spread IS the over-alkylation
# signal; it is preserved in the label (sites listed), not discarded.
# Invariant asserted: rolled-up count == sum of contributing site-row ET.

def load_unimod_title_mass(unimod_xml):
    """title -> monoisotopic delta mass, parsed from unimod.xml (namespace-agnostic)."""
    title_to_mass = {}
    for _ev, el in ET.iterparse(unimod_xml, events=("end",)):
        if el.tag.split("}")[-1] == "mod":
            title = el.attrib.get("title")
            mono = None
            for child in el:
                if child.tag.split("}")[-1] == "delta":
                    mono = child.attrib.get("mono_mass")
                    break
            if title and mono is not None:
                title_to_mass[title] = float(mono)
            el.clear()
    return title_to_mass


def load_mascot(txt_path, unimod_title_mass):
    """Adapter: Mascot error-tolerant mod summary -> generic mod-table (one file).

    Columns: Modification, Site, Above thr., ET, Total matches. We count on
    `Total matches` (== ET here, Above thr. is 0 in error-tolerant mode).
    pct = row count / total resolved ET for the file (Mascot's OWN denominator,
    NOT PSM count — stated so no one forces PSM-count equality across tools).
    Returns (rows, skipped) where skipped is a list of (name, site, count) that
    had no clean Unimod mass (e.g. 'Non-specific cleavage', unresolved
    substitutions) — logged, never silently dropped.
    """
    # Aggregate ET across all site rows of the same resolved mass.
    by_mass = {}        # rounded-mass key -> {"mass","count","sites":{name->et}}
    skipped = []
    source_total = 0
    skipped_total = 0
    with open(txt_path, encoding="utf-8", newline="") as fh:
        r = csv.DictReader(fh, delimiter="\t")
        for row in r:
            name = row["Modification"].strip()
            site = row["Site"].strip()
            try:
                count = int(float(row["Total matches"]))
            except (ValueError, KeyError):
                continue
            if count <= 0:
                continue
            # Independent tally of everything read, before any roll-up touches it.
            # The conservation check below compares against THIS, not against the
            # accumulator it is checking.
            source_total += count
            mass = unimod_title_mass.get(name)
            if mass is None:
                skipped.append((name, site, count))
                skipped_total += count
                continue
            key = round(mass, 4)
            slot = by_mass.setdefault(key, {"mass": mass, "count": 0, "sites": {}})
            slot["count"] += count
            slot["sites"][f"{name}@{site}"] = slot["sites"].get(f"{name}@{site}", 0) + count

    # Conservation invariant: every ET count read from the file is either rolled up
    # or explicitly skipped for having no Unimod mass. Nothing may vanish or double.
    #
    # REWRITTEN 2026-08-25. The previous form asserted
    #     slot["count"] == sum(slot["sites"].values())
    # but both sides were incremented by the same `count` in the same iteration, so
    # it could never fail for any input. It was labelled a conservation invariant and
    # checked nothing. Same could-not-fail shape as the old Tier 3 gate. Verified by
    # reduction: 200 randomised trials produced 0 slots where it could fire.
    rolled_total = sum(s["count"] for s in by_mass.values())
    assert rolled_total + skipped_total == source_total, (
        f"Mascot roll-up conservation broke: read {source_total} ET counts, "
        f"rolled up {rolled_total}, skipped {skipped_total} "
        f"(difference {source_total - rolled_total - skipped_total})"
    )

    total_et = sum(s["count"] for s in by_mass.values())
    rows = []
    for slot in by_mass.values():
        # Label = dominant mod name + the site list (over-alkylation stays visible).
        site_items = sorted(slot["sites"].items(), key=lambda kv: -kv[1])
        dom_name = site_items[0][0].split("@")[0]
        sites = sorted({s.split("@")[1] for s in slot["sites"]})
        label = f"{dom_name} [{','.join(sites)}]"
        rows.append({
            "mass": slot["mass"],
            "count": slot["count"],
            "pct": 100.0 * slot["count"] / total_et if total_et else 0.0,
            "label": label,
        })
    return rows, skipped


# --- Alignment + metrics ---------------------------------------------------

def in_window(mass):
    return WINDOW_LO <= mass <= WINDOW_HI


def align(reference, other, tol=MATCH_TOL_DA):
    """Align two mod-tables by mass within tol, restricted to the shared window.

    Returns (matched, ref_only, other_only). Greedy nearest-mass match; each
    row used once. Both inputs are pre-filtered to the window so 'other_only'
    is a genuine disagreement, not an out-of-range artifact.
    """
    ref = [r for r in reference if in_window(r["mass"])]
    oth = [o for o in other if in_window(o["mass"])]
    ref_sorted = sorted(range(len(ref)), key=lambda i: -ref[i]["count"])
    used_other = set()
    matched, ref_only = [], []
    for i in ref_sorted:
        rmass = ref[i]["mass"]
        best_j, best_d = None, tol + 1
        for j, o in enumerate(oth):
            if j in used_other:
                continue
            d = abs(o["mass"] - rmass)
            if d <= tol and d < best_d:
                best_j, best_d = j, d
        if best_j is not None:
            used_other.add(best_j)
            matched.append((ref[i], oth[best_j], best_d))
        else:
            ref_only.append(ref[i])
    other_only = [oth[j] for j in range(len(oth)) if j not in used_other]
    return matched, ref_only, other_only


def rank_map(rows):
    """mass -> rank by descending count (1-based), for matched rows."""
    order = sorted(range(len(rows)), key=lambda i: -rows[i]["count"])
    return {id(rows[i]): pos + 1 for pos, i in enumerate(order)}


def spearman_and_topn(matched, n=10):
    """Spearman rank correlation + top-N overlap on the matched subset.

    Returns (rho, pval, topn_overlap_count, topn_ref_masses, topn_other_masses).
    rho/pval: Spearman on percent_PSMs of all matched rows (None if < 3 pairs).
    top-N: the N highest-% rows in each tool's matched set; overlap = masses in both.
    """
    if len(matched) < 2:
        return None, None, 0, [], []
    ref_pcts = [m[0]["pct"] for m in matched]
    oth_pcts = [m[1]["pct"] for m in matched]
    if len(matched) >= 3:
        rho, pval = spearmanr(ref_pcts, oth_pcts)
    else:
        rho, pval = None, None

    ref_top = sorted(matched, key=lambda m: -m[0]["pct"])[:n]
    oth_top = sorted(matched, key=lambda m: -m[1]["pct"])[:n]
    ref_top_masses = {round(m[0]["mass"], 3) for m in ref_top}
    oth_top_masses = {round(m[1]["mass"], 3) for m in oth_top}
    overlap = ref_top_masses & oth_top_masses
    return rho, pval, len(overlap), sorted(ref_top_masses), sorted(oth_top_masses)


def window_spot_check(recon_rows, other_rows):
    """Assert the window genuinely covers >+100 Da and return a summary line.

    This is the exact class of silent-cap bug we just fixed: if both tools have
    peaks above +100 but the comparison reports zero matched above +100, the fix
    didn't land. Raises AssertionError if both inputs have peaks above +100 but
    none appear in the window — i.e. the window is still capped.
    """
    recon_above = [r for r in recon_rows if r["mass"] > 100.0]
    other_above = [o for o in other_rows if o["mass"] > 100.0]
    in_window_above = [r for r in recon_rows if 100.0 < r["mass"] <= WINDOW_HI]
    # Both tools have >+100 peaks but our window shows none → still capped.
    if recon_above and other_above:
        assert in_window_above, (
            f"WINDOW BUG: both tools have peaks >+100 Da but none appear in window "
            f"[{WINDOW_LO}, {WINDOW_HI}] — WINDOW_HI may still be capped at 100."
        )
    return (f"Window coverage check: Sage-Recon peaks >+100 Da in window: "
            f"{len(in_window_above)} "
            f"(Sage total >+100: {len(recon_above)}, "
            f"other tool total >+100: {len(other_above)})")


def compare_file(file_key, recon_rows, other_rows, other_name):
    # --- spot-check before alignment -----------------------------------------
    spot_line = window_spot_check(recon_rows, other_rows)

    matched, ref_only, other_only = align(recon_rows, other_rows)
    # Ranks within the matched set (each tool ranked among the mods both found).
    ref_matched = [m[0] for m in matched]
    oth_matched = [m[1] for m in matched]
    ref_rank = rank_map(ref_matched)
    oth_rank = rank_map(oth_matched)

    rho, pval, topn_overlap, ref_top_m, oth_top_m = spearman_and_topn(matched)

    lines = []
    lines.append(f"### {file_key}  (Sage-Recon vs {other_name})")
    lines.append("")
    lines.append(f"- Matched (both, within {MATCH_TOL_DA} Da): **{len(matched)}**")
    lines.append(f"- Sage-Recon only: **{len(ref_only)}**")
    lines.append(f"- {other_name} only: **{len(other_only)}**")
    if rho is not None:
        lines.append(f"- Spearman ρ (% matched): **{rho:.3f}** (p={pval:.3g}, n={len(matched)})")
    lines.append(f"- Top-10 overlap (by % in matched set): **{topn_overlap}/10**")
    lines.append(f"- _{spot_line}_")
    lines.append("")
    lines.append("| mass (Da) | Sage-Recon label | Recon % | Recon rank | "
                 f"{other_name} label | {other_name} % | {other_name} rank |")
    lines.append("|---|---|---|---|---|---|---|")
    for ref, oth, _d in sorted(matched, key=lambda m: -m[0]["count"]):
        lines.append(
            f"| {ref['mass']:+.4f} | {ref['label']} | {ref['pct']:.2f} | "
            f"{ref_rank[id(ref)]} | {oth['label']} | {oth['pct']:.2f} | "
            f"{oth_rank[id(oth)]} |"
        )
    lines.append("")
    if ref_only:
        lines.append("**Sage-Recon only (in window, no match):**")
        for r in sorted(ref_only, key=lambda r: -r["count"])[:20]:
            lines.append(f"- {r['mass']:+.4f}  {r['label']}  ({r['pct']:.2f}%, {r['count']} PSMs)")
        lines.append("")
    if other_only:
        lines.append(f"**{other_name} only (in window, no match):**")
        for o in sorted(other_only, key=lambda o: -o["count"])[:20]:
            lines.append(f"- {o['mass']:+.4f}  {o['label']}  ({o['pct']:.2f}%, {o['count']} PSMs)")
        lines.append("")
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--recon-json", nargs="+", required=True,
                    help="path:file_key pairs (file_key in {serum,bcell,b1906})")
    ap.add_argument("--ptmshepherd",
                    help="path to a PTM-Shepherd global.modsummary.tsv (open or reallyOpen)")
    ap.add_argument("--mascot-dir",
                    help="dir holding MascotErrorTol-{909c,b1906,Bcell}.txt")
    ap.add_argument("--unimod", default="testing/reference-data/unimod.xml",
                    help="unimod.xml, for the Mascot name->mass adapter")
    ap.add_argument("--out-dir", default="testing/recon-output/comparison")
    ap.add_argument("--out-name", default="recon_vs_ptmshepherd.md",
                    help="output markdown filename within out-dir")
    ap.add_argument("--tool-name", default="PTM-Shepherd",
                    help="display name for the compared tool (e.g. 'PTM-Shepherd (reallyOpen)')")
    args = ap.parse_args()

    if not args.ptmshepherd and not args.mascot_dir:
        raise SystemExit("give --ptmshepherd and/or --mascot-dir")

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    unimod_tm = load_unimod_title_mass(args.unimod) if args.mascot_dir else None

    md = [f"# Mod-discovery cross-comparison: Sage-Recon vs {args.tool_name}",
          "",
          "OBJECTIVE tool-vs-tool benchmark (NOT a gate; NOT a comparison to the "
          "tool's author). Sage-Recon is the reference column. Divergence is "
          "expected — see the methodology deltas in "
          "`testing/reference-data/ptm-shepherd/README.md` (recalibrated two-stage "
          "search, per-file instruments, different FDR, wider window, higher peak "
          "floor, speed). Percentages are the currency; PSM totals differ by FDR.",
          f"",
          f"Match tolerance {MATCH_TOL_DA} Da; shared window [{WINDOW_LO}, {WINDOW_HI}] Da "
          f"(Sage delta range −100..+500, PTM-Shepherd −150..+500; true overlap −100..+500).",
          ""]

    for pair in args.recon_json:
        path, _, file_key = pair.partition(":")
        if not file_key:
            raise SystemExit(f"--recon-json arg needs path:file_key, got {pair!r}")
        recon_rows = load_recon(path)
        if args.ptmshepherd:
            other_rows = load_ptmshepherd(args.ptmshepherd, file_key)
            md.append(compare_file(file_key, recon_rows, other_rows, args.tool_name))
            md.append("")
        if args.mascot_dir:
            txt = Path(args.mascot_dir) / MASCOT_FILE_STEM[file_key]
            other_rows, skipped = load_mascot(txt, unimod_tm)
            skip_et = sum(c for _n, _s, c in skipped)
            print(f"[{file_key}] Mascot: {len(other_rows)} mass rows, "
                  f"{len(skipped)} rows unresolved to a Unimod mass "
                  f"({skip_et} ET matches skipped — incl. Non-specific cleavage / "
                  f"unresolved substitutions)")
            md.append(compare_file(file_key, recon_rows, other_rows, args.tool_name))
            md.append(f"_Mascot rows with no Unimod mass (skipped, not dropped silently): "
                      f"{len(skipped)} ({skip_et} ET matches)._")
            md.append("")

    out_md = out_dir / args.out_name
    out_md.write_text("\n".join(md), encoding="utf-8")
    print(f"Wrote {out_md}")


if __name__ == "__main__":
    main()
