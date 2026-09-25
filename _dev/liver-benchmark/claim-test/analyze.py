#!/usr/bin/env python3
"""Count identifications per claim-test arm and write results.md.

Usage, from the repository root:

    python3 _dev/liver-benchmark/claim-test/analyze.py WORKDIR

WORKDIR holds one Sage output folder per arm, named out_<arm>, each with
results.sage.tsv, results.json and run_meta.json (written by run_arm.py).

Definitions (fixed before the runs, see README):
- PSMs: target rows (label == 1) with spectrum_q <= 0.01.
- Peptides: distinct stripped sequences of target rows with peptide_q <= 0.01.
  Modified forms (distinct `peptide` strings) are reported as well.
- Protein groups: distinct `protein_groups` strings of target rows with
  protein_group_q <= 0.01.
Verdict (criterion fixed in README before any run): arm 4 must identify
more peptides than arm 1, by more than the arm 1 vs arm 1 repeat difference,
with arm 4 wall time at most 3x arm 1 and a completed run.

    python3 analyze.py WORKDIR [--write-reference DIR]

--write-reference saves arm 1's counts and sorted peptide list to DIR (used
once, for the laptop cross-check in laptop-arm1/). If laptop-arm1/ exists,
arm 1 is compared with it.
Python 3 standard library only.
"""
import gzip
import csv
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
Q = 0.01
RUNTIME_RATIO_MAX = 3.0
REF = os.path.join(HERE, "laptop-arm1")

ARMS = [
    ("arm1_vanilla", "1 vanilla (vanilla mods, 20/20 ppm)"),
    ("arm1_vanilla_rep", "1 vanilla, repeat run"),
    ("arm2_vanilla_mods_recon_tol", "2 vanilla mods, recon tol 10/10 ppm"),
    ("arm3_recon_mods_vanilla_tol", "3 recon mods, 20/20 ppm"),
    ("arm4_recon_full", "4 recon mods, recon tol 10/10 ppm"),
    ("arm4_stage1_rare", "4, stage 1: core + rare-site mods"),
    ("arm4_stage2_cys", "4, stage 2: + Cys mods"),
]


def strip(pep):
    return re.sub(r"\[[^\]]*\]-?", "", pep)


def mod_sites(pep):
    """Return (site, mass) pairs. site = 'Nterm' or a residue letter."""
    out = []
    pos = 0
    for m in re.finditer(r"\[([+-]\d+\.\d+)\]-|([A-Z])(?:\[([+-]\d+\.\d+)\])?", pep):
        if m.group(1):
            out.append(("Nterm", float(m.group(1))))
        elif m.group(3):
            out.append((m.group(2) + ("@1" if pos == 0 else ""), float(m.group(3))))
        if m.group(2):
            pos += 1
    return out


def load(outdir):
    psms, peps, forms, groups = [], set(), set(), set()
    with open(os.path.join(outdir, "results.sage.tsv"), newline="") as fh:
        for r in csv.DictReader(fh, delimiter="\t"):
            if r["label"] != "1":
                continue
            if float(r["spectrum_q"]) <= Q:
                psms.append(r["peptide"])
            if float(r["peptide_q"]) <= Q:
                peps.add(strip(r["peptide"]))
                forms.add(r["peptide"])
            if float(r["protein_group_q"]) <= Q:
                groups.add(r["protein_groups"])
    return psms, peps, forms, groups


def mod_counts(psms):
    counts = {}
    for p in psms:
        seen = set()
        for site, mass in mod_sites(p):
            key = (site.split("@")[0] if site != "Nterm" else "Nterm", round(mass, 2))
            if key not in seen:
                counts[key] = counts.get(key, 0) + 1
                seen.add(key)
    return counts


def third_party(targets):
    """Liver counts from PTM-Shepherd, MetaMorpheus, Mascot, nearest within 0.01 Da."""
    sys.path.insert(0, os.path.join(ROOT, "_dev/testing/scripts"))
    import compare_4way as C4  # noqa: E402
    bench = os.path.join(ROOT, "_dev/liver-benchmark")
    C4.PTMS_COLUMN_STEM["liver"] = "10mg_1_A_1_1"
    C4.METAMORPHEUS_FILE_STEM["liver"] = "10mg_1_A_1-calib"
    shep = C4.load_ptmshepherd(f"{bench}/ptm-shepherd/liverShepherd/global.modsummary.tsv", "liver")
    mm = C4.load_metamorpheus(
        f"{bench}/metamorpheus/liverMetaMorpheus/Task3-SearchTask/AllPeptides.psmtsv", "liver")
    titles = C4.load_unimod_title_mass(os.path.join(ROOT, "recon-tool/resources/unimod.xml"))
    mascot = C4.load_mascot(f"{bench}/mascot/error-tolerant/MascotErrorTol-liver.txt", titles)
    if isinstance(mascot, tuple):
        mascot = mascot[0]
    out = {}
    for label, mass in targets:
        row = []
        for rows in (shep, mm, mascot):
            best, bd = None, 0.01
            for r in rows:
                d = abs(r["mass"] - mass)
                if d <= bd:
                    best, bd = r, d
            row.append(best["count"] if best else None)
        out[label] = row
    return out


def verdict(res):
    L = ["", "## Verdict against the pre-set criterion", ""]
    ok = lambda a: a in res and res[a][2] is not None
    peps = {a: len(res[a][2][1]) for a in res if res[a][2] is not None}
    wall = {a: res[a][1]["wall_seconds"] for a in res}
    if not ok("arm1_vanilla"):
        return L + ["Arm 1 is missing or failed. No verdict."]
    p1 = peps["arm1_vanilla"]
    if ok("arm1_vanilla_rep"):
        noise = abs(p1 - peps["arm1_vanilla_rep"])
        L.append(f"- Noise band: arm 1 {p1} vs repeat {peps['arm1_vanilla_rep']} peptides, "
                 f"difference {noise}.")
    else:
        noise = None
        L.append("- Noise band: no arm 1 repeat. The gain cannot be tested against noise.")
    for a, what in (("arm2_vanilla_mods_recon_tol", "tolerance effect (arm 2 - arm 1)"),
                    ("arm3_recon_mods_vanilla_tol", "mod effect (arm 3 - arm 1)")):
        if ok(a):
            L.append(f"- {what}: {peps[a] - p1:+d} peptides.")
        else:
            L.append(f"- {what}: arm missing or failed.")
    if not ok("arm4_recon_full"):
        L.append("- Arm 4 is missing or failed: the claim is not shown on this machine. "
                 "If stages ran, their rows above show how far recon's mods went.")
        return L + ["", "**Verdict: NOT MET (arm 4 did not complete).**"]
    gain = peps["arm4_recon_full"] - p1
    ratio = wall["arm4_recon_full"] / wall["arm1_vanilla"] if wall["arm1_vanilla"] else float("inf")
    L.append(f"- Arm 4 - arm 1: {gain:+d} peptides. Wall time ratio arm 4 / arm 1: {ratio:.2f} "
             f"(limit {RUNTIME_RATIO_MAX}).")
    if ok("arm2_vanilla_mods_recon_tol") and ok("arm3_recon_mods_vanilla_tol"):
        inter = gain - (peps["arm2_vanilla_mods_recon_tol"] - p1) - (peps["arm3_recon_mods_vanilla_tol"] - p1)
        L.append(f"- Interaction (arm 4 gain minus the two single effects): {inter:+d} peptides.")
    more = gain > (noise if noise is not None else 0)
    cheap = ratio <= RUNTIME_RATIO_MAX
    met = more and cheap and noise is not None
    why = []
    if not more:
        why.append("no peptide gain beyond the noise band")
    if not cheap:
        why.append(f"runtime {ratio:.2f}x > {RUNTIME_RATIO_MAX}x")
    if noise is None:
        why.append("no arm 1 repeat")
    L.append("")
    L.append("**Verdict: " + ("MET." if met else "NOT MET (" + "; ".join(why) + ").") + "**")
    return L


def reference(res, write_dir=None):
    if "arm1_vanilla" not in res or res["arm1_vanilla"][2] is None:
        return []
    psms, peps, forms, groups = res["arm1_vanilla"][2]
    counts = {"psms": len(psms), "peptides": len(peps), "modified_forms": len(forms),
              "protein_groups": len(groups)}
    if write_dir:
        os.makedirs(write_dir, exist_ok=True)
        with open(os.path.join(write_dir, "counts.json"), "w") as fh:
            json.dump(counts, fh, indent=2)
        with gzip.open(os.path.join(write_dir, "peptides.txt.gz"), "wt") as fh:
            fh.write("\n".join(sorted(peps)) + "\n")
        return []
    if not os.path.exists(os.path.join(REF, "counts.json")):
        return []
    ref = json.load(open(os.path.join(REF, "counts.json")))
    with gzip.open(os.path.join(REF, "peptides.txt.gz"), "rt") as fh:
        rp = set(fh.read().split())
    L = ["", "## Arm 1 against the laptop run (laptop-arm1/)", "",
         "| count | laptop | this run |", "|---|---|---|"]
    for k, v in counts.items():
        L.append(f"| {k} | {ref[k]} | {v} |")
    L.append(f"\nPeptides only in the laptop run: {len(rp - peps)}; only in this run: "
             f"{len(peps - rp)}. Small differences are q-value jitter; a large one means "
             f"a different Sage build or input.")
    return L


def main():
    work = sys.argv[1]
    write_ref = sys.argv[3] if len(sys.argv) > 3 and sys.argv[2] == "--write-reference" else None
    res = {}
    for arm, desc in ARMS:
        d = os.path.join(work, "out_" + arm)
        if not os.path.exists(os.path.join(d, "run_meta.json")):
            continue
        meta = json.load(open(os.path.join(d, "run_meta.json")))
        if meta["returncode"] != 0 or not os.path.exists(os.path.join(d, "results.sage.tsv")):
            res[arm] = (desc, meta, None)
            continue
        res[arm] = (desc, meta, load(d))

    base = res["arm1_vanilla"][2]
    L = []
    L.append("| arm | PSMs | peptides (stripped) | modified forms | protein groups | "
             "peptides gained vs arm 1 | lost vs arm 1 | wall (s) | peak RSS / footprint (GB) | load before |")
    L.append("|---|---|---|---|---|---|---|---|---|---|")
    for arm, (desc, meta, data) in res.items():
        rss = meta["peak_rss_bytes_time_l"]
        fp = meta.get("peak_footprint_bytes_time_l")
        rss_s = (f"{rss / 1e9:.2f}" if rss else "n/a") + " / " + (f"{fp / 1e9:.2f}" if fp else "n/a")
        if data is None:
            L.append(f"| {desc} | failed: {meta['killed_by_watchdog'] or meta['returncode']} "
                     f"| | | | | | {meta['wall_seconds']} | {rss_s} | {meta['loadavg_before'][0]} |")
            continue
        psms, peps, forms, groups = data
        gained = len(peps - base[1])
        lost = len(base[1] - peps)
        L.append(f"| {desc} | {len(psms)} | {len(peps)} | {len(forms)} | {len(groups)} | "
                 f"{gained} | {lost} | {meta['wall_seconds']} | {rss_s} | "
                 f"{meta['loadavg_before'][0]} |")

    # PSMs carrying each mod
    keys = set()
    mc = {}
    for arm, (desc, meta, data) in res.items():
        if data:
            mc[arm] = mod_counts(data[0])
            keys |= set(mc[arm])
    L.append("")
    L.append("PSMs at 1 % FDR carrying each mass shift (site, rounded mass):")
    L.append("")
    L.append("| site | mass | " + " | ".join(a for a in mc) + " |")
    L.append("|---|---|" + "---|" * len(mc))
    for k in sorted(keys, key=lambda k: (k[1], k[0])):
        L.append(f"| {k[0]} | {k[1]:+.2f} | " + " | ".join(str(mc[a].get(k, 0)) for a in mc) + " |")

    # Third-party counts for recon-only mods
    mapping = []
    with open(os.path.join(HERE, "configs/mod_mapping.tsv")) as fh:
        mapping = list(csv.DictReader(fh, delimiter="\t"))
    vanilla_titles = {"Carbamidomethyl", "Oxidation", "Deamidated", "Gln->pyro-Glu", "Acetyl"}
    extra = [(r["recon_label"], float(r["unimod_mass"]), r["recon_psm_count"])
             for r in mapping if r["unimod_title"] not in vanilla_titles]
    tp = third_party([(lab, m) for lab, m, _ in extra])
    L.append("")
    L.append("recon-only mods, liver counts from the other tools (nearest mass within "
             "0.01 Da of the Unimod mass; PSMs):")
    L.append("")
    L.append("| recon label | Unimod mass | recon pass-1 count | PTM-Shepherd | MetaMorpheus | Mascot |")
    L.append("|---|---|---|---|---|---|")
    for lab, m, n in extra:
        s, mm, ma = tp[lab]
        f = lambda v: "none" if v is None else str(v)
        L.append(f"| {lab} | {m:+.6f} | {n} | {f(s)} | {f(mm)} | {f(ma)} |")

    if write_ref:
        reference(res, write_ref)
        print(f"wrote reference to {write_ref}")
        return
    L += reference(res)
    L += verdict(res)
    text = "\n".join(L) + "\n"
    with open(os.path.join(HERE, "results.md"), "w") as fh:
        fh.write("# Claim test results (generated by analyze.py)\n\n" + text)
    print(text)


if __name__ == "__main__":
    main()
