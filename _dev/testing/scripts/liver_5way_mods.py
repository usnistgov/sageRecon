#!/usr/bin/env python3
"""Liver-only mod-discovery comparison, FIVE sources — the fourway table plus Byonic Preview.

Reuses testing/scripts/compare_4way.py verbatim for recon / PTM-Shepherd / Mascot /
MetaMorpheus loading and for the greedy global clustering. Adds ONE new arm: Preview.

Prints markdown to stdout; writes nothing itself.

Usage:
    python liver_5way_mods.py [TOP_N]        # default 30

Two Preview conventions are handled explicitly. Both were VERIFIED against Preview's
own output, not assumed:

1. "(-fixed mod)" rows are deltas RELATIVE TO THE FIXED +57.021464 ON C.
   Verified three ways against result_detail.html's own prose:
     Trioxidation  -9.036720 + 57.021464 =  47.984744  <- detail "C[+48]", = 3 x oxidation
     Propionamide  14.015650 + 57.021464 =  71.037114  <- detail "C[+71]"
     Dehydro      -58.029289 + 57.021464 =  -1.007825  <- detail "unmodified_C[-2]" (-H per C)
   Without this correction a mass join silently MISSES Preview's cysteine chemistry.

2. VariableMods.txt repeats ONE GROUP TOTAL across several site rows, exactly like the
   Mascot multi-site trap already recorded in NOTES. Summing the rows double-counts.
   Deduped on (ModName, DeltaMass, nmods, n16); where result_detail.html gives a finer
   per-mass split, the detail value overrides and is marked.

Preview's % column is its OWN `prop2` = nmods / base_num_ids(2667) * 100, which
reproduces the summary page exactly ("Oxidized methionine M[+16]: 19.1% (509 additional
identifications over 2667 baseline)" -> prop2 19.085114). It is Preview's own currency,
NOT a PSM percentage. See NOTES "Prevalence currency": the columns are not commensurable
and the claim here is RANK.
"""
import importlib.util
import os
import sys
import csv
import json

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PREVIEW_DIR = f"{ROOT}/reference-data/preview/10mg_1_A_1"
FIXED_C = 57.021464


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    m = importlib.util.module_from_spec(spec)
    sys.modules[name] = m
    spec.loader.exec_module(m)
    return m


C4 = load_module(f"{ROOT}/scripts/compare_4way.py", "compare_4way")


def require(path, why):
    if not os.path.exists(path):
        raise SystemExit(f"MISSING INPUT: {path}\n  needed for: {why}\n"
                         f"  Refusing to report a partial comparison as a whole one.")
    return path


# ---------------------------------------------------------------- Preview arm

# Per-mass splits that VariableMods.txt reports only as a GROUP TOTAL.
# Values quoted from result_detail.html; the quote is printed in the provenance block.
# Per-mass splits that VariableMods.txt reports only as a GROUP TOTAL.
# Keyed on (ModName, Target) — NOT on mass. Two different mods can share a mass
# (three separate populations sit at +14.0157), so a mass-keyed override corrupts
# the neighbours and double-counts a group listed under two names.
# Values quoted from result_detail.html:
#   "Pyro-glu N-terminus (Q[-17], E[-18], C[+57][-17]): 17.5% (66/378) (61 -17, 5 -18)"
#   "N-terminal methylation/dimethylation (N-terminus[+14/+28]): 0.5% (14/2672) (9 +14, 5 +28)"
DETAIL_SPLIT = {
    ("Gln->pyro-Glu", "N-Term Q"): 61,
    # camC[-17] is the SAME -17 population already counted as 61 above. Setting it
    # to 0 prevents the mass bin at -17.0265 from counting that group twice.
    ("Ammonia-loss", "N-Term C"): 0,
    ("Glu->pyro-Glu", "N-Term E"): 5,
    ("Methyl", "N-term"): 9,
    ("Dimethyl", "N-term"): 5,
}

# Group totals the detail page does NOT split: one count spread over two masses.
GROUP_TOTAL_UNSPLIT = {("Carbamidomethyl", "N-term, H, K"), ("Dicarbamidomethyl", "N-term")}


def load_preview(path):
    """Preview VariableMods.txt -> rows on the ABSOLUTE delta-mass axis."""
    raw = []
    with open(path, encoding="utf-8", newline="") as fh:
        for row in csv.DictReader(fh, delimiter="\t"):
            try:
                mass = float(row["DeltaMass"])
                n = int(float(row["nmods"]))
                prop2 = float(row["prop2"])
            except (ValueError, KeyError, TypeError):
                continue
            raw.append({
                "name": row["ModName"].strip(),
                "target": (row.get("Target") or "").strip(),
                "mass": mass, "n": n, "prop2": prop2,
                "n16": (row.get("n16") or "").strip(),
            })

    # 1. dedupe repeated group totals: same mod, same mass, same population
    seen, dedup = set(), []
    dup_dropped = 0
    for r in raw:
        key = (r["name"], round(r["mass"], 6), r["n"], r["n16"])
        if key in seen:
            dup_dropped += 1
            continue
        seen.add(key)
        dedup.append(r)

    # 2. fixed-mod offset back onto the absolute axis
    offset_applied = []
    for r in dedup:
        if "(-fixed mod)" in r["name"]:
            r["mass_abs"] = r["mass"] + FIXED_C
            r["offset"] = True
            offset_applied.append((r["name"], r["mass"], r["mass_abs"]))
        else:
            r["mass_abs"] = r["mass"]
            r["offset"] = False

    # 3. detail-HTML overrides, keyed on (ModName, Target)
    overridden = []
    for r in dedup:
        k = (r["name"], r["target"])
        if k in DETAIL_SPLIT and r["n"] != DETAIL_SPLIT[k]:
            overridden.append((r["name"], r["target"], r["mass_abs"], r["n"], DETAIL_SPLIT[k]))
            r["n"] = DETAIL_SPLIT[k]
            r["prop2"] = 100.0 * r["n"] / 2667.0
            r["from_detail"] = True
        r["unsplit"] = k in GROUP_TOTAL_UNSPLIT

    # 4. collapse to one row per mass
    bins = {}
    for r in dedup:
        if r["n"] <= 0:
            continue
        k = round(r["mass_abs"], 4)
        slot = bins.setdefault(k, {"mass": r["mass_abs"], "count": 0, "pct": 0.0,
                                   "labels": [], "offset": False, "detail": False, "unsplit": False})
        slot["count"] += r["n"]
        slot["pct"] += r["prop2"]
        lab = r["name"] + (f" @{r['target']}" if r["target"] else "")
        slot["labels"].append(lab)
        slot["offset"] = slot["offset"] or r["offset"]
        slot["detail"] = slot["detail"] or r.get("from_detail", False)
        slot["unsplit"] = slot["unsplit"] or r.get("unsplit", False)

    rows = []
    for slot in bins.values():
        rows.append({"mass": slot["mass"], "count": slot["count"], "pct": slot["pct"],
                     "label": "/".join(dict.fromkeys(slot["labels"])),
                     "offset": slot["offset"], "detail": slot["detail"],
                     "unsplit": slot["unsplit"]})
    return rows, dup_dropped, offset_applied, overridden


# ---------------------------------------------------------------- recon recs

def load_recommendations(json_path):
    d = json.load(open(json_path, encoding="utf-8"))
    recs = d["recommendations"]
    out = {}
    for r in recs.get("fixed", []):
        out[round(r["delta_mass"], 3)] = ("FIXED", r["label"], r.get("category"), r)
    for r in recs.get("variable", []):
        out[round(r["delta_mass"], 3)] = ("variable", r["label"], r.get("category"), r)
    tail = {}
    for r in recs.get("not_recommended", []):
        tail[round(r["delta_mass"], 3)] = (r.get("reason"), r.get("label"))
    return out, tail, recs


def rec_for(mass, recs, tail, tol=0.015):
    for k, v in recs.items():
        if abs(k - mass) <= tol:
            return "rec", v
    for k, v in tail.items():
        if abs(k - mass) <= tol:
            return "tail", v
    return None, None


# ---------------------------------------------------------------- main

def main():
    top_n = int(sys.argv[1]) if len(sys.argv) > 1 else 30

    recon = C4.load_recon(require(f"{ROOT}/recon-output/full-run/liver.json", "recon arm"))

    C4.PTMS_COLUMN_STEM["liver"] = "10mg_1_A_1_1"
    shep = C4.load_ptmshepherd(
        require(f"{ROOT}/reference-data/ptm-shepherd/liverShepherd/global.modsummary.tsv",
                "PTM-Shepherd arm"), "liver")

    titles = C4.load_unimod_title_mass(require(f"{ROOT}/reference-data/unimod.xml", "Unimod masses"))
    mascot = C4.load_mascot(
        require(f"{ROOT}/reference-data/mascot/error-tolerant/MascotErrorTol-liver.txt",
                "Mascot arm"), titles)
    if isinstance(mascot, tuple):
        mascot, mascot_skipped = mascot
    else:
        mascot_skipped = []

    C4.METAMORPHEUS_FILE_STEM["liver"] = "10mg_1_A_1-calib"
    mm = C4.load_metamorpheus(
        require(f"{ROOT}/reference-data/metamorpheus/liverMetaMorpheus/Task3-SearchTask/AllPeptides.psmtsv",
                "MetaMorpheus arm — GITIGNORED, see .gitignore:62"), "liver")

    prev, dup_dropped, offsets, overridden = load_preview(
        require(f"{PREVIEW_DIR}/objs/VariableMods.txt", "Byonic Preview arm"))

    recs, tail, rec_block = load_recommendations(f"{ROOT}/recon-output/full-run/liver.json")

    order = ["Recon", "Preview", "PTM-Shepherd", "Mascot", "MetaMorpheus"]
    tool_rows = {"Recon": recon, "Preview": prev, "PTM-Shepherd": shep,
                 "Mascot": mascot, "MetaMorpheus": mm}

    # ---- provenance block, printed so nothing is taken on trust
    print("## Preview arm — how it was put on the mass axis\n")
    print(f"* rows read from `objs/VariableMods.txt`, **{dup_dropped} repeated group-total "
          f"rows dropped** (same mod, same mass, same population listed once per site).")
    print(f"* **{len(offsets)} `(-fixed mod)` rows shifted back onto the absolute axis** "
          f"by +{FIXED_C}:")
    for name, listed, absm in offsets:
        print(f"  * `{name}` {listed:+.6f} -> **{absm:+.6f}**")
    print(f"* **{len(overridden)} group totals replaced by the finer split in "
          f"`result_detail.html`**:")
    for name, tgt, m, was, now in overridden:
        print(f"  * `{name}` @{tgt} at {m:+.4f}: {was} (group total) -> **{now}** (per-mass)")
    print(f"* Preview % is its own `prop2` = nmods / 2667 baseline, the same number its "
          f"summary page prints.\n")

    print("## Counts entering the comparison\n")
    for t in order:
        print(f"* {t}: {len(tool_rows[t])} rows")
    if mascot_skipped:
        print(f"* Mascot: {len(mascot_skipped)} rows had no clean Unimod mass — skipped and counted")
    print()

    # ---- the table
    clusters = C4.cluster_all(tool_rows)
    ranks = {t: C4.rank_within([r for r in rows if C4.in_window(r["mass"])])
             for t, rows in tool_rows.items()}
    clusters.sort(key=lambda c: max((m["pct"] for m in c["members"].values()), default=0.0),
                  reverse=True)

    print("## liver — 10mg_1_A_1\n")
    hdr = ("| mass (Da) | recon rec | " + " | ".join(f"{t} %" for t in order) +
           " | " + " | ".join(f"{t} label" for t in order) + " |")
    print(hdr)
    print("|" + "---|" * (2 + 2 * len(order)))

    for c in clusters[:top_n]:
        kind, v = rec_for(c["mass"], recs, tail)
        if kind == "rec":
            role, label, cat, _ = v
            recmark = f"**{role}** — {label}" if role == "FIXED" else f"{role} — {label}"
        elif kind == "tail":
            reason, label = v
            recmark = f"_no_ ({reason})"
        else:
            recmark = "—"
        pcts, labs = [], []
        for t in order:
            m = c["members"].get(t)
            if m is None:
                pcts.append("—"); labs.append("—")
            else:
                r = ranks[t].get(id(m), "?")
                mark = ""
                if t == "Preview" and m.get("offset"):
                    mark = " ⁺"
                if t == "Preview" and m.get("detail"):
                    mark += " ᵈ"
                if t == "Preview" and m.get("unsplit"):
                    mark += " ᵍ"
                pcts.append(f"{m['pct']:.2f} (#{r}){mark}")
                labs.append(m["label"])
        print(f"| {c['mass']:+.4f} | {recmark} | " + " | ".join(pcts) + " | " + " | ".join(labs) + " |")

    hit = lambda c: sum(1 for t in order if t in c["members"])
    only1 = [c for c in clusters if hit(c) == 1]
    print(f"\n_{len(clusters)} total clusters in window; {len(only1)} found by exactly one tool "
          f"(shown above only if in the top {top_n} by max %)._")
    print("_⁺ = Preview mass shifted off its fixed +57 C. ᵈ = count taken from "
          "`result_detail.html`, not the group total._\n")

    # ---- the recommendation crosswalk: the question actually asked
    print("## Do recon's recommendations line up?\n")
    print("| # | delta | recon calls it | " + " | ".join(order[1:]) + " | corroborated by |")
    print("|" + "---|" * (4 + len(order[1:])))

    allrecs = ([("FIXED", r) for r in rec_block.get("fixed", [])] +
               [("variable", r) for r in rec_block.get("variable", [])])
    allrecs.sort(key=lambda x: -x[1]["count"])

    for i, (role, r) in enumerate(allrecs, 1):
        m = r["delta_mass"]
        cells, n_hit = [], 0
        for t in order[1:]:
            best, bd = None, 0.015
            for row in tool_rows[t]:
                d = abs(row["mass"] - m)
                if d <= bd:
                    best, bd = row, d
            if best is None:
                cells.append("—")
            else:
                cells.append(f"{best['pct']:.2f}")
                n_hit += 1
        rolemark = f"**{role}**" if role == "FIXED" else role
        print(f"| {i} | {m:+.4f} | {rolemark} — {r['label']} ({r.get('category')}) | "
              + " | ".join(cells) + f" | **{n_hit}/4** |")

    print("\n### recon's tail — the biggest peaks it declined to recommend\n")
    print("| delta | recon PSMs | reason | " + " | ".join(order[1:]) + " |")
    print("|" + "---|" * (3 + len(order[1:])))
    for r in rec_block.get("not_recommended", [])[:10]:
        m = r["delta_mass"]
        cells = []
        for t in order[1:]:
            best, bd = None, 0.015
            for row in tool_rows[t]:
                d = abs(row["mass"] - m)
                if d <= bd:
                    best, bd = row, d
            cells.append(f"{best['pct']:.2f}" if best else "—")
        lab = f" — {r['label']}" if r.get("label") else ""
        print(f"| {m:+.4f} | {r['count']} | {r['reason']}{lab} | " + " | ".join(cells) + " |")


if __name__ == "__main__":
    main()
