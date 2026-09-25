#!/usr/bin/env python3
"""Summarize the claim-test arms: IDs at 1 % FDR, runtime, memory, per-mod PSMs, overlap.
Usage: python3 summarize.py <work-dir>. Standard library only."""
import csv, os, re, sys
work = sys.argv[1]
ARMS = [a for a in ("vanilla", "recon_guided", "vanilla_repeat") if os.path.isdir(os.path.join(work, a))]

def load(arm):
    rows = list(csv.DictReader(open(os.path.join(work, arm, "results.sage.tsv")), delimiter="\t"))
    t = [r for r in rows if r["label"] == "1"]
    psms = [r for r in t if float(r["spectrum_q"]) <= 0.01]
    peps = {r["peptide"] for r in t if float(r["peptide_q"]) <= 0.01}
    prots = {r["proteins"] for r in t if float(r["protein_q"]) <= 0.01}
    mods = {}
    for r in psms:
        for m in set(re.findall(r"(\S)?\[([+-][0-9.]+)\]", r["peptide"])):
            mods[m] = mods.get(m, 0) + 1
    return psms, peps, prots, mods

def timelog(arm):
    p = os.path.join(work, arm + ".time.log")
    if not os.path.exists(p):
        return "?", "?"
    txt = open(p, errors="replace").read()
    wall = re.search(r"([0-9.]+) real", txt) or re.search(r"Elapsed.*?: ([0-9:.]+)", txt)
    mem = re.search(r"(\d+)\s+maximum resident set size", txt) or re.search(r"Maximum resident set size \(kbytes\): (\d+)", txt)
    gb = None
    if mem:
        v = int(mem.group(1)); gb = v / 1e9 if "maximum resident" in mem.group(0) else v / 1e6
    return (wall.group(1) if wall else "?"), (f"{gb:.1f}" if gb else "?")

strip = lambda p: re.sub(r"\[[^\]]*\]", "", p).replace("-", "")
data = {a: load(a) for a in ARMS}
print("| arm | PSMs (spectrum q<=0.01) | peptides (peptide q<=0.01) | stripped sequences | protein groups | wall | peak GB |")
print("|---|---|---|---|---|---|---|")
for a in ARMS:
    psms, peps, prots, _ = data[a]; w, g = timelog(a)
    print(f"| {a} | {len(psms)} | {len(peps)} | {len({strip(p) for p in peps})} | {len(prots)} | {w} | {g} |")
if "vanilla" in data and "recon_guided" in data:
    v = {strip(p) for p in data["vanilla"][1]}; r = {strip(p) for p in data["recon_guided"][1]}
    print(f"\nStripped sequences: shared {len(v & r)}, recon-guided only {len(r - v)}, vanilla only {len(v - r)}")
    if "vanilla_repeat" in data:
        v2 = {strip(p) for p in data["vanilla_repeat"][1]}
        print(f"Run-to-run noise (vanilla vs repeat): {len(v ^ v2)} sequences differ")
for a in ARMS:
    print(f"\nPSMs per modification, {a}:")
    for (res, mass), n in sorted(data[a][3].items(), key=lambda x: -x[1]):
        print(f"- {res or 'N-term'} {mass}: {n}")
