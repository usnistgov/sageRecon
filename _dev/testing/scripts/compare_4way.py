#!/usr/bin/env python3
"""
4-way mod-discovery comparison: Sage-Recon + PTM-Shepherd + Mascot + MetaMorpheus,
ONE table per file, side by side — instead of three separate pairwise-vs-Recon docs.

This does NOT replace compare_mod_discovery.py / compare-mod-discovery-metamorpheus.py
(those stay the source of truth for the pairwise Spearman/top-N/window-check metrics
already in BENCHMARK-SUMMARY.md). This is a different question: "for a given mass, what
does EVERY tool say," so patterns like the +57 magnitude gap are visible in one glance
across all comparators instead of hunted across three markdown files.

Clustering: greedy, largest-count-first, single global pass across all four tools'
rows together (not pairwise-vs-recon). A tool's own two rows can never land in the
same cluster. This means a mass can get "discovered" by a cluster anchored by ANY
tool, not just Recon — so tool-only findings (e.g. MetaMorpheus's Phosphorylation)
show up as a row with blanks in the other three columns, not as a hidden "other only"
footnote.

Usage:
    python compare_4way.py \
        --recon-json testing/recon-output/nofixedmods/bcell.json:bcell \
                     testing/recon-output/nofixedmods/serum.json:serum \
                     testing/recon-output/nofixedmods/b1906.json:b1906 \
        --ptmshepherd testing/reference-data/ptm-shepherd/reallyOpen/global.modsummary.tsv \
        --mascot-dir testing/reference-data/mascot/error-tolerant \
        --metamorpheus testing/reference-data/metamorpheus/2026-08-21-10-29-48/Task3-SearchTask/AllPSMs.psmtsv \
        --unimod testing/reference-data/unimod.xml \
        --out-name fourway_comparison.md \
        --top 30
"""

import argparse
import csv
import json
import re
import xml.etree.ElementTree as ET
from pathlib import Path

# --- Configuration (same values as the pairwise scripts, kept consistent) ---

MATCH_TOL_DA = 0.015
WINDOW_LO, WINDOW_HI = -100.0, 500.0

PTMS_COLUMN_STEM = {
    "serum": "2019_4_9_909c_0311_1",
    "bcell": "Bnaive_01steady_state_1",
    "b1906": "b1906_1",
}
MASCOT_FILE_STEM = {
    "serum": "MascotErrorTol-909c.txt",
    "bcell": "MascotErrorTol-Bcell.txt",
    "b1906": "MascotErrorTol-b1906.txt",
}
METAMORPHEUS_FILE_STEM = {
    "serum": "2019-4-9_909c_0311-calib",
    "bcell": "B.naive_01steady-state-calib",
    "b1906": "b1906_293T_proteinID_01A_QE3_122212-calib",
}

ATOMIC_MONOISOTOPIC_MASS = {
    "H": 1.0078250319, "C": 12.0000000, "N": 14.0030740052, "O": 15.9949146221,
    "S": 31.97207069, "P": 30.97376151, "Na": 22.98976928, "K": 38.96370649,
    "Ca": 39.9625912, "Fe": 55.9349421, "Mg": 23.9850417, "Zn": 63.9291466,
    "Cu": 62.9295975, "Cl": 34.96885271, "Se": 79.9165196,
}
_FORMULA_TOKEN_RE = re.compile(r'([A-Z][a-z]?)(-?\d+)?')
MOD_TAG_RE = re.compile(r'\[([^:\]]+):(.+?) on ([\w\-]+)\]')


def _formula_to_mass(formula):
    if not formula:
        return 0.0
    total = 0.0
    for symbol, count_str in _FORMULA_TOKEN_RE.findall(formula):
        if not symbol:
            continue
        count = int(count_str) if count_str else 1
        if symbol not in ATOMIC_MONOISOTOPIC_MASS:
            raise KeyError(f"Unknown element symbol '{symbol}' in formula {formula!r}")
        total += ATOMIC_MONOISOTOPIC_MASS[symbol] * count
    return total


# --- Adapters (unchanged logic from the pairwise scripts) ------------------

def load_recon(json_path):
    with open(json_path, encoding="utf-8") as fh:
        report = json.load(fh)
    peaks = report["mod_discovery"]["peaks"] if "mod_discovery" in report else report["peaks"]
    rows = []
    for peak in peaks:
        anns = peak.get("annotations", [])
        label = anns[0]["name"] if anns else "UNANNOTATED"
        if peak.get("unannotated"):
            label = "UNANNOTATED"
        rows.append({"mass": peak["delta_mass"], "count": peak["count"],
                     "pct": peak["count_pct"], "label": label})
    return rows


def load_ptmshepherd(tsv_path, file_key):
    stem = PTMS_COLUMN_STEM[file_key]
    psm_col, pct_col = f"{stem}_PSMs", f"{stem}_percent_PSMs"
    rows = []
    with open(tsv_path, encoding="utf-8", newline="") as fh:
        r = csv.DictReader(fh, delimiter="\t")
        if psm_col not in r.fieldnames or pct_col not in r.fieldnames:
            raise SystemExit(f"PTM-Shepherd TSV missing {psm_col}/{pct_col}")
        for row in r:
            try:
                mass = float(row["Mass Shift"]); count = int(float(row[psm_col])); pct = float(row[pct_col])
            except (ValueError, KeyError):
                continue
            if count <= 0:
                continue
            rows.append({"mass": mass, "count": count, "pct": pct, "label": row["Modification"]})
    return rows


def load_unimod_title_mass(unimod_xml):
    title_to_mass = {}
    for _ev, el in ET.iterparse(unimod_xml, events=("end",)):
        if el.tag.split("}")[-1] == "mod":
            title = el.attrib.get("title")
            mono = None
            for child in el:
                if child.tag.split("}")[-1] == "delta":
                    mono = child.attrib.get("mono_mass"); break
            if title and mono is not None:
                title_to_mass[title] = float(mono)
            el.clear()
    return title_to_mass


def load_mascot(txt_path, unimod_title_mass):
    by_mass, skipped = {}, []
    with open(txt_path, encoding="utf-8", newline="") as fh:
        r = csv.DictReader(fh, delimiter="\t")
        for row in r:
            name, site = row["Modification"].strip(), row["Site"].strip()
            try:
                count = int(float(row["Total matches"]))
            except (ValueError, KeyError):
                continue
            if count <= 0:
                continue
            mass = unimod_title_mass.get(name)
            if mass is None:
                skipped.append((name, site, count)); continue
            key = round(mass, 4)
            slot = by_mass.setdefault(key, {"mass": mass, "count": 0, "sites": {}})
            slot["count"] += count
            slot["sites"][f"{name}@{site}"] = slot["sites"].get(f"{name}@{site}", 0) + count
    total_et = sum(s["count"] for s in by_mass.values())
    rows = []
    for slot in by_mass.values():
        site_items = sorted(slot["sites"].items(), key=lambda kv: -kv[1])
        dom_name = site_items[0][0].split("@")[0]
        sites = sorted({s.split("@")[1] for s in slot["sites"]})
        rows.append({"mass": slot["mass"], "count": slot["count"],
                     "pct": 100.0 * slot["count"] / total_et if total_et else 0.0,
                     "label": f"{dom_name} [{','.join(sites)}]"})
    return rows, skipped


def load_metamorpheus(tsv_path, file_key):
    file_name_wanted = METAMORPHEUS_FILE_STEM[file_key]
    n_total = n_excl = 0
    bins = {}
    with open(tsv_path, encoding="utf-8", newline="") as fh:
        r = csv.DictReader(fh, delimiter="\t")
        required = ["File Name", "Full Sequence", "Base Sequence",
                    "Mods Combined Chemical Formula", "Mods Chemical Formulas",
                    "Decoy/Contaminant/Target", "QValue"]
        missing = [c for c in required if c not in r.fieldnames]
        if missing:
            raise SystemExit(f"MetaMorpheus TSV missing columns: {missing}")
        for row in r:
            if row["File Name"] != file_name_wanted:
                continue
            if row["Decoy/Contaminant/Target"] != "T":
                continue
            try:
                qval = float(row["QValue"])
            except ValueError:
                continue
            if qval >= 0.01:
                continue
            n_total += 1
            full_seq = row["Full Sequence"]
            if "|" in full_seq:
                n_excl += 1; continue
            formula = row["Mods Combined Chemical Formula"].strip() or row["Mods Chemical Formulas"].strip()
            try:
                total_mass = _formula_to_mass(formula)
            except KeyError as e:
                raise SystemExit(str(e))
            tags = MOD_TAG_RE.findall(full_seq)
            label = "Unmodified" if not tags else ", ".join(sorted(f"{name} on {site}" for _c, name, site in tags))
            key = round(total_mass, 3)
            slot = bins.setdefault(key, {"count": 0, "mass_sum": 0.0, "labels": {}})
            slot["count"] += 1
            slot["mass_sum"] += total_mass
            slot["labels"][label] = slot["labels"].get(label, 0) + 1
    n_kept = n_total - n_excl
    rows = []
    for slot in bins.values():
        dom_label = max(slot["labels"].items(), key=lambda kv: kv[1])[0]
        rows.append({"mass": slot["mass_sum"] / slot["count"], "count": slot["count"],
                     "pct": 100.0 * slot["count"] / n_kept if n_kept else 0.0, "label": dom_label})
    return rows


# --- N-way clustering --------------------------------------------------
# Unlike the pairwise scripts' recon-anchored align(), this pools ALL FOUR tools'
# rows into one global greedy clustering pass, largest-count-first, so a mass can
# be "discovered" by whichever tool has the biggest peak there — a tool-only find
# (e.g. MetaMorpheus's Phosphorylation, absent from Recon's peak list entirely)
# still gets its own row instead of disappearing into an "other only" footnote.

def in_window(mass):
    return WINDOW_LO <= mass <= WINDOW_HI


def cluster_all(tool_rows, tol=MATCH_TOL_DA):
    """tool_rows: {tool_name: [rows]} -> list of clusters.

    Each cluster: {"mass": anchor_mass, "members": {tool_name: row}}.
    A tool can contribute at most one row per cluster (first-come, largest-count-first).
    """
    items = []
    for tool, rows in tool_rows.items():
        for r in rows:
            if in_window(r["mass"]):
                items.append((tool, r))
    items.sort(key=lambda tr: -tr[1]["count"])

    clusters = []
    for tool, r in items:
        best = None
        for c in clusters:
            if tool in c["members"]:
                continue
            if abs(c["mass"] - r["mass"]) <= tol:
                if best is None or abs(c["mass"] - r["mass"]) < abs(best["mass"] - r["mass"]):
                    best = c
        if best is not None:
            best["members"][tool] = r
        else:
            clusters.append({"mass": r["mass"], "members": {tool: r}})
    return clusters


def rank_within(tool_rows_in_window):
    order = sorted(range(len(tool_rows_in_window)), key=lambda i: -tool_rows_in_window[i]["count"])
    return {id(tool_rows_in_window[i]): pos + 1 for pos, i in enumerate(order)}


def build_table(file_key, tool_rows, tool_order, top_n):
    clusters = cluster_all(tool_rows)
    ranks = {tool: rank_within([r for r in rows if in_window(r["mass"])])
             for tool, rows in tool_rows.items()}

    # Sort clusters by the max % any tool reports for that mass — surfaces
    # "everyone agrees this is big" and "one tool alone thinks this is big" alike.
    def cluster_max_pct(c):
        return max((m["pct"] for m in c["members"].values()), default=0.0)

    clusters.sort(key=cluster_max_pct, reverse=True)

    header = "| mass (Da) | " + " | ".join(f"{t} %" for t in tool_order) + \
             " | " + " | ".join(f"{t} label" for t in tool_order) + " |"
    sep = "|" + "---|" * (1 + 2 * len(tool_order))
    lines = [f"### {file_key}", "", header, sep]

    for c in clusters[:top_n]:
        pct_cells, label_cells = [], []
        for t in tool_order:
            m = c["members"].get(t)
            if m is None:
                pct_cells.append("—")
                label_cells.append("—")
            else:
                r = ranks[t].get(id(m), "?")
                pct_cells.append(f"{m['pct']:.2f} (#{r})")
                label_cells.append(m["label"])
        lines.append(f"| {c['mass']:+.4f} | " + " | ".join(pct_cells) + " | " + " | ".join(label_cells) + " |")

    n_tools_hit = lambda c: sum(1 for t in tool_order if t in c["members"])
    only_one = [c for c in clusters if n_tools_hit(c) == 1]
    lines.append("")
    lines.append(f"_{len(clusters)} total clusters in window; {len(only_one)} found by exactly one tool "
                 f"(shown above only if in the top {top_n} by max %; full singleton list omitted here)._")
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--recon-json", nargs="+", required=True, help="path:file_key pairs")
    ap.add_argument("--ptmshepherd", required=True)
    ap.add_argument("--mascot-dir", required=True)
    ap.add_argument("--metamorpheus", required=True)
    ap.add_argument("--unimod", default="testing/reference-data/unimod.xml")
    ap.add_argument("--out-dir", default="testing/recon-output/comparison")
    ap.add_argument("--out-name", default="fourway_comparison.md")
    ap.add_argument("--top", type=int, default=30, help="max rows per file, sorted by max %% across tools")
    args = ap.parse_args()

    unimod_tm = load_unimod_title_mass(args.unimod)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    tool_order = ["Recon", "PTM-Shepherd", "Mascot", "MetaMorpheus"]
    md = ["# Four-tool mod-discovery comparison (Recon / PTM-Shepherd reallyOpen / "
          "Mascot ET / MetaMorpheus reallyOpen)", "",
          "Single global clustering across all four tools (not recon-anchored pairwise) — "
          "a mass can be \"discovered\" by whichever tool has the biggest peak there. "
          f"Match tolerance {MATCH_TOL_DA} Da, window [{WINDOW_LO}, {WINDOW_HI}] Da. "
          "Rank (#N) is each tool's own rank among ITS rows in-window, not the cluster rank.",
          ""]

    for pair in args.recon_json:
        path, _, file_key = pair.partition(":")
        if not file_key:
            raise SystemExit(f"--recon-json needs path:file_key, got {pair!r}")

        recon_rows = load_recon(path)
        ptms_rows = load_ptmshepherd(args.ptmshepherd, file_key)
        mascot_rows, mascot_skipped = load_mascot(Path(args.mascot_dir) / MASCOT_FILE_STEM[file_key], unimod_tm)
        mm_rows = load_metamorpheus(args.metamorpheus, file_key)

        tool_rows = {
            "Recon": recon_rows,
            "PTM-Shepherd": ptms_rows,
            "Mascot": mascot_rows,
            "MetaMorpheus": mm_rows,
        }
        md.append(build_table(file_key, tool_rows, tool_order, args.top))
        md.append("")

    out_md = out_dir / args.out_name
    out_md.write_text("\n".join(md), encoding="utf-8")
    print(f"Wrote {out_md}")


if __name__ == "__main__":
    main()
