#!/usr/bin/env python3
"""Mod-discovery agreement on liver: recon vs PTM-Shepherd vs MetaMorpheus vs Mascot.

Matched by DELTA MASS, not by name, at 0.01 Da — names differ between tools for
the same chemistry ("Carbamidomethyl" vs "Iodoacetamide derivative"), so a
name join would under-count agreement and a rank correlation built on it would
be meaningless.

⚠ RANK, NOT MAGNITUDE. NOTES "Prevalence currency" records that the four tools
report four different quantities; the counts are not commensurable. Spearman on
the shared masses is the claim this project makes and the only one it defends.

⚠ All three ran ALKYLATION-AGNOSTIC on liver (no fixed Cys):
  recon        - both passes strip and assert empty mods
  PTM-Shepherd - FragPipe Open workflow, all fixed/variable mods removed
  MetaMorpheus - Common Fixed/Variable removed as mod OPTIONS and added to the
                 G-PTM-D discovery list instead, so +57 is DISCOVERED not assumed
  Mascot       - error-tolerant with MODS= and IT_MODS= both EMPTY, so its
                 Carbamidomethyl is a discovery too (verified in Human_ertol-2018.par)

⚠ MASCOT REPORTS BY NAME + SITE, NOT MASS, and splits ONE mass across many site
rows. The site rows MUST be rolled up before any mass-axis comparison, or the
C-only row understates the true +57 population. That roll-up already exists and
is reused here verbatim — `compare_mod_discovery.load_mascot`. See NOTES
"Mascot error-tolerant adapter — MULTI-SITE ROLL-UP is load-bearing".

⚠ Mascot's counts are its OWN resolved-ET fraction, not PSM counts, and its
`PFA=1` allows one missed cleavage against recon's two. Neither is a reason to
compare counts — the claim here is RANK only.
"""
import csv, json, re, os, collections

TOL = 0.01


def recon_peaks(path):
    d = json.load(open(path))
    out = []
    for p in d["mod_discovery"]["peaks"]:
        if abs(p["delta_mass"]) < 0.05:      # the unmodified population
            continue
        out.append((p["delta_mass"], p["count"]))
    return out


def shepherd_peaks(path):
    out = []
    for r in csv.DictReader(open(path), delimiter="\t"):
        if not r.get("PSMs"):
            continue
        m = float(r["peak_apex"])
        if abs(m) < 0.05:
            continue
        out.append((m, float(r["PSMs"])))
    return out


def element_masses(unimod_path):
    """Monoisotopic element masses, read from the PINNED unimod.xml.

    Not from memory and not hardcoded: the same file the tool already treats as
    its annotation authority carries an <umod:elem> table.
    """
    text = open(unimod_path, encoding="utf-8", errors="replace").read()
    out = {}
    for m in re.finditer(r'<umod:elem title="([^"]+)"[^>]*mono_mass="([0-9.eE+-]+)"', text):
        out[m.group(1)] = float(m.group(2))
    return out


def formula_mass(formula, elems):
    """Mass of a MetaMorpheus chemical formula such as 'C2H3NO' or 'H-1O'."""
    total = 0.0
    for sym, cnt in re.findall(r"([A-Z][a-z]?)(-?\d*)", formula or ""):
        if not sym:
            continue
        if sym not in elems:
            return None
        total += elems[sym] * (int(cnt) if cnt not in ("", "-") else 1)
    return total


def metamorpheus_peaks(path, elems, q=0.01):
    """Discovered-mod counts, keyed by a mass computed from the run's OWN
    reported chemical formula.

    ⚠ The 'Mass Diff (Da)' column is the PRECURSOR mass error, not the mod
    delta. Using it produced ZERO matches against recon on 2026-09-01; the
    formula column is the correct source.
    """
    per_mod = collections.Counter()
    formula_of = {}
    for r in csv.DictReader(open(path), delimiter="\t"):
        if (r.get("Decoy/Contaminant/Target") or "").strip() != "T":
            continue
        try:
            if float(r.get("QValue") or 1) > q:
                continue
        except ValueError:
            continue
        mods = re.findall(r"\[([^\]]+)\]", r.get("Full Sequence") or "")
        if len(mods) != 1:
            continue
        name = mods[0]
        per_mod[name] += 1
        f = (r.get("Mods Combined Chemical Formula") or "").strip()
        if f:
            formula_of.setdefault(name, f)
    out = []
    for name, n in per_mod.items():
        m = formula_mass(formula_of.get(name), elems)
        if m is None:
            continue
        out.append((m, n, name))
    return out


def spearman(xs, ys):
    def ranks(v):
        order = sorted(range(len(v)), key=lambda i: v[i])
        r = [0.0] * len(v)
        i = 0
        while i < len(order):
            j = i
            while j + 1 < len(order) and v[order[j + 1]] == v[order[i]]:
                j += 1
            avg = (i + j) / 2.0 + 1
            for k in range(i, j + 1):
                r[order[k]] = avg
            i = j + 1
        return r
    rx, ry = ranks(xs), ranks(ys)
    n = len(xs)
    mx, my = sum(rx) / n, sum(ry) / n
    num = sum((a - mx) * (b - my) for a, b in zip(rx, ry))
    dx = sum((a - mx) ** 2 for a in rx) ** 0.5
    dy = sum((b - my) ** 2 for b in ry) ** 0.5
    return num / (dx * dy) if dx and dy else float("nan")


def perm_p(xs, ys, seed=42, iters=200000):
    """Two-sided permutation p-value for Spearman rho.

    EXACT when n <= 8 (all n! permutations); Monte Carlo with a FIXED seed
    otherwise, so the number is reproducible. Reported instead of a table
    lookup because the n here is small and asymptotic p-values misbehave there.
    """
    import itertools, random
    obs = abs(spearman(xs, ys))
    n = len(xs)
    if n <= 8:
        perms = list(itertools.permutations(ys))
        hits = sum(1 for q in perms if abs(spearman(xs, list(q))) >= obs - 1e-12)
        return hits / len(perms), "exact"
    rnd = random.Random(seed)
    y = list(ys)
    hits = 0
    for _ in range(iters):
        rnd.shuffle(y)
        if abs(spearman(xs, y)) >= obs - 1e-12:
            hits += 1
    return (hits + 1) / (iters + 1), f"permutation seed={seed} iters={iters}"


def merge_by_mass(pairs, tol=TOL):
    """Sum counts of entries that sit within `tol` of each other.

    ⚠ REQUIRED before any cross-tool match. MetaMorpheus reports per-NAME, and
    several names share one delta ("Oxidation on M" and "Hydroxylation on P" are
    both formula O = +15.9949). Matching name-wise picks ONE of them and silently
    discards the rest: on 2026-09-01 that reported Oxidation as 14 peptides
    against a true 2127. A mass-basis comparison must aggregate on mass first.
    """
    items = sorted(pairs, key=lambda x: x[0])
    out = []
    for m, c in items:
        if out and abs(m - out[-1][0]) <= tol:
            n = out[-1][1] + c
            out[-1] = (out[-1][0], n)
        else:
            out.append((m, c))
    return out


def match(a, b, tol=TOL):
    pairs = []
    for ma, ca in a:
        best, bd = None, tol
        for mb, cb in b:
            d = abs(ma - mb)
            if d <= bd:
                best, bd = cb, d
        if best is not None:
            pairs.append((ma, ca, best))
    return pairs


def require(path, why):
    """Hard-stop on a missing input, with the reason.

    ⚠ DELIBERATELY NOT A SILENT SKIP. Some inputs here are GITIGNORED — notably
    MetaMorpheus's `AllPeptides.psmtsv` (`.gitignore:62`) — so on a fresh clone
    they are simply absent. A script that quietly drops an arm in that case
    reports a smaller comparison as if it were the whole one. The MS1
    calibration integration test has exactly that weakness (see NOTES
    2026-09-01); do not copy it here.
    """
    import os
    if not os.path.exists(path):
        raise SystemExit(
            f"MISSING INPUT: {path}\n  needed for: {why}\n"
            f"  If this is a fresh clone, this file is gitignored and must be "
            f"restored from the run that produced it. Refusing to report a "
            f"partial comparison as a whole one."
        )
    return path


def mascot_peaks(txt_path, unimod_xml):
    """Reuse the shipped Mascot adapter rather than re-implementing the roll-up."""
    import importlib.util
    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location(
        "cmd_mod", os.path.join(here, "compare_mod_discovery.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    titles = mod.load_unimod_title_mass(unimod_xml)
    rows, skipped = mod.load_mascot(txt_path, titles)
    out = []
    for r in rows:
        m = r.get("mass") if isinstance(r, dict) else r[0]
        c = r.get("count") if isinstance(r, dict) else r[1]
        if m is None or abs(float(m)) < 0.05:
            continue
        out.append((float(m), float(c)))
    return out, skipped


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    rec = recon_peaks(require(f"{root}/recon-output/full-run/liver.json", "recon arm"))
    sh = shepherd_peaks(require(f"{root}/reference-data/ptm-shepherd/liverShepherd/global.profile.tsv", "PTM-Shepherd arm"))
    elems = element_masses(f"{root}/reference-data/unimod.xml")
    mm3 = metamorpheus_peaks(require(
        f"{root}/reference-data/metamorpheus/liverMetaMorpheus/Task3-SearchTask/AllPeptides.psmtsv",
        "MetaMorpheus arm — GITIGNORED, see .gitignore:62"),
        elems)
    msc, msc_skipped = mascot_peaks(
        require(f"{root}/reference-data/mascot/error-tolerant/MascotErrorTol-liver.txt",
                "Mascot arm"),
        f"{root}/reference-data/unimod.xml")
    msc = merge_by_mass(msc)
    mm = merge_by_mass([(m, n) for m, n, _ in mm3])
    sh = merge_by_mass(sh)
    rec = merge_by_mass(rec)

    print(f"peaks/mods after removing the delta~0 population:")
    print(f"  recon        {len(rec)}")
    print(f"  PTM-Shepherd {len(sh)}")
    print(f"  MetaMorpheus {len(mm)}")
    print(f"  Mascot       {len(msc)}   ({len(msc_skipped)} rows had no clean Unimod "
          f"mass and were SKIPPED, not dropped silently)\n")

    for label, other in (("PTM-Shepherd", sh), ("MetaMorpheus", mm), ("Mascot", msc)):
        p = match(rec, other)
        if len(p) < 3:
            print(f"recon vs {label}: only {len(p)} shared masses — no correlation reported")
            continue
        xs = [x[1] for x in p]
        ys = [x[2] for x in p]
        rho = spearman(xs, ys)
        pv, how = perm_p(xs, ys)
        print(f"recon vs {label:<13} n={len(p):<3} Spearman rho = {rho:+.3f}   p = {pv:.4g}  ({how})")
    print()

    print("Top shared masses, recon count vs the other tool's count:")
    hdr = f"{'delta':>10} {'recon':>8} {'Shepherd':>9} {'MetaM':>8} {'Mascot':>8}"
    print(hdr); print("-" * len(hdr))
    shd = dict((round(m, 2), c) for m, c in sh)
    mmd = dict((round(m, 2), c) for m, c in mm)
    mscd = dict((round(m, 2), c) for m, c in msc)
    for m, c in sorted(rec, key=lambda x: -x[1])[:12]:
        k = round(m, 2)
        s = shd.get(k)
        v = mmd.get(k)
        # allow +/- one hundredth for rounding at the bin edge
        if s is None:
            s = next((shd[kk] for kk in (round(k + .01, 2), round(k - .01, 2)) if kk in shd), None)
        if v is None:
            v = next((mmd[kk] for kk in (round(k + .01, 2), round(k - .01, 2)) if kk in mmd), None)
        w = mscd.get(k)
        if w is None:
            w = next((mscd[kk] for kk in (round(k + .01, 2), round(k - .01, 2)) if kk in mscd), None)
        print(f"{m:>10.4f} {c:>8} {(s if s is not None else '-'):>9} "
              f"{(int(v) if v is not None else '-'):>8} {(int(w) if w is not None else '-'):>8}")


if __name__ == "__main__":
    main()
