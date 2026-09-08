#!/usr/bin/env python3
"""Test ONE hypothesis about why Gate 5 failed, before deciding what to do about it.

HYPOTHESIS (recorded in NOTES before this ran)
Gate 5 assumes the reference tool is localizing the SAME chemistry recon named. At
a mass-degenerate position it may instead be localizing a DIFFERENT curated
candidate that happens to share the mass. If so, "reference says a different
residue" does not mean "recon picked the wrong residue", and the gate's premise is
wrong rather than recon's answer.

THE TEST, stated before looking
For every Gate 5 violation, take the reference's residue and ask: is there ANOTHER
curated candidate within the matching tolerance of that same mass whose acceptor
set CONTAINS that residue?

  YES for every violation  -> hypothesis SUPPORTED. Every disagreement is explained
                             by mass degeneracy; the gate is comparing two different
                             chemistries and its premise needs revising.
  NO for some violation    -> hypothesis FAILS THERE. The reference put the SAME
                             chemistry on a residue recon did not test, which is a
                             real disagreement about recon's answer.

A partial result is the interesting one and must not be rounded up: the fraction
explained is reported, and any unexplained violation is named.

This script does NOT change Gate 5. It only measures.

Usage:
    python testing/scripts/gate5_degeneracy_probe.py
"""
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import gate5_residue_agreement as G5

REPO = G5.REPO
MASS_TOL = G5.MASS_TOL


def curated_entries():
    """Every curated entry as (mass, ID, acceptor residues, position)."""
    elems = {}
    ux = (REPO / "testing/reference-data/unimod.xml").read_text(encoding="utf-8", errors="replace")
    for m in re.finditer(r"<umod:elem [^>]*>", ux):
        t = re.search(r'title="([^"]+)"', m.group(0))
        mo = re.search(r'mono_mass="([-\d.]+)"', m.group(0))
        if t and mo:
            elems[t.group(1)] = float(mo.group(1))

    def fmass(cf):
        tot = 0.0
        for sym, cnt in re.findall(r"([A-Z][a-z]?)\s*(-?\d+)?", cf):
            if not sym:
                continue
            if sym not in elems:
                return None
            tot += elems[sym] * (int(cnt) if cnt else 1)
        return tot

    out = []
    for fn in ["Mods.txt", "aListOfmods.txt", "ProteaseMods.txt", "surfactants.txt"]:
        path = REPO / "reference-notes/metaMorpheusMods" / fn
        if not path.exists():
            continue
        for block in path.read_text(encoding="utf-8", errors="replace").split("\n//"):
            f = dict(re.findall(r"^([A-Z]{2})\s+(.*)$", block, re.M))
            if not f.get("ID") or not f.get("CF"):
                continue
            mass = fmass(f["CF"])
            if mass is None:
                continue
            tg = f.get("TG", "")
            residues = {t.strip() for t in tg.split(" or ") if len(t.strip()) == 1}
            out.append({"mass": mass, "id": f["ID"], "residues": residues,
                        "tg": tg, "pp": f.get("PP", "")})
    return out


def main():
    curated = curated_entries()
    ptms = G5.load_ptmshepherd()
    mm = G5.load_metamorpheus(G5.curated_masses())

    violations = []
    for f in G5.FILES:
        rep = json.loads(G5.REPORT[f].read_text(encoding="utf-8"))
        rec = rep.get("recommendations") or {}
        for m in rec.get("fixed", []) + rec.get("variable", []):
            if m["decided_by"] != "statistics":
                continue
            mass, sites = m["delta_mass"], m["sites"]
            near = sorted([p for p in ptms if abs(p["mass"] - mass) <= MASS_TOL],
                          key=lambda p: abs(p["mass"] - mass))
            calls = []
            if near:
                calls.append(("PTM-Shepherd", near[0]["aa"]))
            mc = G5.mm_call(mm[f], mass) if mm else None
            if mc:
                calls.append(("MetaMorpheus", mc["aa"]))
            for tag, aa in calls:
                if aa in ("N-term", "C-term"):
                    continue
                if aa not in set(sites):
                    violations.append((f, mass, m["label"], sites, tag, aa))

    print("=" * 88)
    print("GATE 5 DEGENERACY PROBE — is each violation explained by another curated")
    print("candidate at the same mass carrying the reference's residue?")
    print("=" * 88)
    print(f"\n{len(violations)} violation(s) to explain\n")

    explained, unexplained = [], []
    for f, mass, label, sites, tag, aa in violations:
        # A DIFFERENT curated ENTRY, not merely a different ID. `Formylation`
        # exists three times at +27.9949 (TG=K, TG=X, TG=S or T); the S/T entry
        # and the K entry are different candidates that share a name. Excluding by
        # ID alone wrongly called that one unexplained.
        alts = [c for c in curated
                if abs(c["mass"] - mass) <= MASS_TOL and aa in c["residues"]
                and not (c["id"] == label and c["residues"] == set(sites))]
        same_mod_other_site = [c for c in curated
                               if abs(c["mass"] - mass) <= MASS_TOL and c["id"] == label]
        print(f"{f:<7} {mass:+9.4f}  {label[:32]:<32}")
        print(f"        recon sites={sites!r}   {tag} says {aa!r}")
        if alts:
            explained.append((f, mass, label, tag, aa, alts))
            for c in alts:
                print(f"        EXPLAINED by another curated candidate at this mass: "
                      f"{c['id']!r} TG={c['tg']!r} PP={c['pp']!r} (mass {c['mass']:+.4f})")
        else:
            unexplained.append((f, mass, label, tag, aa, same_mod_other_site))
            allc = [c for c in curated if abs(c["mass"] - mass) <= MASS_TOL]
            print(f"        *** NOT EXPLAINED *** no curated candidate at this mass "
                  f"accepts {aa!r}")
            print(f"        candidates at this mass: "
                  f"{[(c['id'], c['tg']) for c in allc]}")
        print()

    n = len(violations)
    print("=" * 88)
    print(f"RESULT   {len(explained)} of {n} violations explained by mass degeneracy; "
          f"{len(unexplained)} NOT explained")
    if not violations:
        print("no violations to probe")
    elif not unexplained:
        print("\nHYPOTHESIS SUPPORTED for every violation. In each case the reference")
        print("localized a DIFFERENT curated chemistry that shares the mass, so the")
        print("disagreement is about which chemistry is present, not about which")
        print("residue recon tested. Gate 5's premise -- that both tools are talking")
        print("about the same modification -- does not hold at a degenerate mass.")
    else:
        print("\nHYPOTHESIS FAILS on these, which are REAL disagreements about recon's")
        print("answer and must not be dismissed as degeneracy:")
        for f, mass, label, tag, aa, same in unexplained:
            print(f"  {f:<7} {mass:+9.4f} {label} — {tag} says {aa!r}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
