#!/usr/bin/env python3
"""How many curated modifications share a mass, and which of them a residue
CONTAINMENT test can never separate.

WHY THIS EXISTS. `tier_assignment.rs` gives one recommendation per mass peak.
Candidates within `CURATED_TOL_DA` (0.010 Da) are each tested with Fisher exact
on their acceptor residues, then `max_by` odds ratio keeps exactly ONE. The test
is `peptide_hits`, which asks "does this peptide CONTAIN an acceptor residue" —
it is NOT a localization. So a candidate whose acceptor is a common residue is
compared against a background in which most peptides already contain it, and
cannot reach a large odds ratio however real it is.

⚠ The reachability number here is an UPPER BOUND ON DIFFICULTY, not a prediction.
It says what band containment a candidate would need; whether it gets there
depends on the actual band. Confirmed both ways on liver 10mg_1_A_1:
  * +15.9949 Hydroxylation on P  needs 71.8%, band has 62.7%  -> OR ~1.34, FAILS
  * -17.0265 Gln->pyro-Glu on Q  needs 63.3%, band is nearly all Q -> OR 650, PASSES
The test works when one modification dominates its mass band. It cannot surface a
SECOND, co-occurring modification at the same mass — which is the proline case.

Reads only the four curated files actually COMPILED INTO the binary. That set is
not a guess: it is what the build dependency graph lists
(`recon-tool/target/*/deps/recon_tool-*.d`). `ptmlist.txt`, `glyco.txt`,
`substitutions.txt` and `tmt.txt` are present in the directory but NOT loaded.

Usage:
    python curated_mass_collisions.py [--sage-tsv PATH]

With --sage-tsv the background residue frequencies come from that run's own
peptides. Without it, only the mass-collision census is printed.
"""
import argparse
import csv
import os
import re
import sys

# Mirrors tier_assignment.rs
CURATED_TOL_DA = 0.010
OR_MIN = 2.0

LOADED_FILES = ["Mods.txt", "aListOfmods.txt", "ProteaseMods.txt", "surfactants.txt"]

# Monoisotopic element masses, same table as compare_4way.py plus the metals the
# curated list actually uses. A symbol missing here makes the entry unparseable
# and it is COUNTED AND REPORTED, never silently dropped.
EL = {
    'H': 1.0078250319, 'C': 12.0, 'N': 14.0030740052, 'O': 15.9949146221,
    'S': 31.97207069, 'P': 30.97376151, 'Na': 22.98976928, 'K': 38.96370649,
    'Ca': 39.9625912, 'Fe': 55.9349421, 'Mg': 23.9850417, 'Zn': 63.9291466,
    'Cu': 62.9295975, 'Cl': 34.96885271, 'Se': 79.9165196, 'Br': 78.9183376,
    'I': 126.904473, 'Ni': 57.9353429, 'Mo': 97.9054073, 'Co': 58.9331950,
    'Al': 26.98153863, 'As': 74.9215942, 'B': 11.0093055, 'F': 18.99840322,
    'Li': 7.0160045, 'Hg': 201.970617, 'Ag': 106.905092, 'Au': 196.966543,
    'Cd': 113.903357, 'Cr': 51.9405098, 'Mn': 54.9380471, 'Pd': 105.903478,
    'Pt': 194.964766, 'Ru': 101.9043485, 'W': 183.9509222,
}


def formula_mass(cf):
    total = 0.0
    for sym, cnt in re.findall(r'([A-Z][a-z]?)(-?\d*)', cf):
        if not sym:
            continue
        if sym not in EL:
            return None
        total += EL[sym] * (int(cnt) if cnt not in ('', '-') else 1)
    return total


def parse_targets(tg):
    """TG is 'K or N' / 'Y or W or F' / 'X' / 'Proline.'. Single letters only:
    a word like 'Proline.' is a UniProt-style spelling that the loaded files do
    not use, so it correctly yields an empty acceptor set here."""
    out = set()
    for tok in re.split(r'\s+or\s+|,', tg or ''):
        tok = tok.strip().rstrip('.')
        if len(tok) == 1 and tok.isalpha() and tok.upper() != 'X':
            out.add(tok.upper())
    return out


def load_curated(moddir):
    entries, unparsed = [], []
    for fname in LOADED_FILES:
        path = os.path.join(moddir, fname)
        if not os.path.exists(path):
            raise SystemExit(f"MISSING INPUT: {path}\n  needed for: curated mod census")
        text = open(path, encoding='utf-8', errors='replace').read()
        for blk in text.split('\n//'):
            ident = re.search(r'^ID\s+(.+)$', blk, re.M)
            cf = re.search(r'^CF\s+(.+)$', blk, re.M)
            tg = re.search(r'^TG\s+(.+)$', blk, re.M)
            mt = re.search(r'^MT\s+(.+)$', blk, re.M)
            if not ident or not cf:
                continue
            mass = formula_mass(cf.group(1).strip())
            if mass is None:
                unparsed.append((fname, ident.group(1).strip(), cf.group(1).strip()))
                continue
            entries.append({
                'file': fname,
                'id': ident.group(1).strip(),
                'mass': mass,
                'tg': parse_targets(tg.group(1) if tg else ''),
                'raw_tg': (tg.group(1).strip() if tg else ''),
                'category': (mt.group(1).strip() if mt else ''),
            })
    entries.sort(key=lambda e: e['mass'])
    return entries, unparsed


def cluster(entries, tol=CURATED_TOL_DA):
    out = []
    for e in entries:
        if out and abs(e['mass'] - out[-1][0]['mass']) <= tol:
            out[-1].append(e)
        else:
            out.append([e])
    return out


def load_peptides(tsv):
    """recon's own PSM set: target rows at peptide_q <= 0.01.

    ⚠ peptide_q, NOT spectrum_q. Verified against liver: peptide_q reproduces
    mod_discovery.total_psms = 32497 exactly; spectrum_q gives 29929.
    """
    peps = []
    with open(tsv, newline='') as fh:
        for r in csv.DictReader(fh, delimiter='\t'):
            if r.get('label') == '-1':
                continue
            try:
                if float(r['peptide_q']) > 0.01:
                    continue
            except (ValueError, KeyError):
                continue
            peps.append(re.sub(r'[^A-Z]', '', r['peptide'].upper()))
    return peps


def required_band_fraction(f, or_min=OR_MIN):
    """Band containment x needed for OR >= or_min against background f.

        OR = [x/(1-x)] / [f/(1-f)] >= or_min   =>   x >= k*f / (1 + (k-1)*f)
    """
    k = or_min
    return k * f / (1 + (k - 1) * f)


BAND_TOL_DA = 0.010  # mirrors tier_assignment.rs


def load_psms_with_delta(tsv):
    out = []
    with open(tsv, newline='') as fh:
        for r in csv.DictReader(fh, delimiter='\t'):
            if r.get('label') == '-1':
                continue
            try:
                if float(r['peptide_q']) > 0.01:
                    continue
                d = float(r['expmass']) - float(r['calcmass'])
            except (ValueError, KeyError):
                continue
            out.append((d, re.sub(r'[^A-Z]', '', r['peptide'].upper())))
    return out


def odds_ratio(a, b, c, d):
    """Haldane-Anscombe on an empty cell, as tier_assignment.rs does."""
    if b == 0 or c == 0:
        return ((a + 0.5) * (d + 0.5)) / ((b + 0.5) * (c + 0.5))
    return (a * d) / (b * c)


def reproduce_peak(entries, psms, peak, bg_saturated=0.95):
    """Mirror of tier_assignment.rs::test_candidate for one peak.

    ⚠ APPROXIMATE ON THE BAND. This takes a flat +/- BAND_TOL_DA window around
    `peak`; recon bins with its own merged-peak detector. On liver +15.9945 this
    gives 1717 PSMs against recon's reported 1688, and an odds ratio of 59.4 for
    Oxidation on M against recon's 63.53. Same conclusion, not the same number —
    do not quote these as recon's own values.
    """
    band = [p for p in psms if abs(p[0] - peak) <= BAND_TOL_DA]
    cands = [e for e in entries if abs(e['mass'] - peak) <= CURATED_TOL_DA]
    rows = []
    for e in cands:
        if not e['tg']:
            rows.append((e, None))
            continue
        a = sum(1 for _, s in band if any(ch in e['tg'] for ch in s))
        b = len(band) - a
        hits_all = sum(1 for _, s in psms if any(ch in e['tg'] for ch in s))
        bgf = hits_all / len(psms)
        if bgf > bg_saturated:
            rows.append((e, ('saturated', a, b, bgf, None)))
            continue
        c = hits_all - a
        d = (len(psms) - len(band)) - c
        rows.append((e, ('tested', a, b, bgf, odds_ratio(a, b, c, d))))
    return band, rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--moddir", default=None)
    ap.add_argument("--sage-tsv", default=None)
    ap.add_argument("--peak", type=float, default=None,
                    help="Reproduce test_candidate for every curated candidate at this "
                         "delta mass, using the --sage-tsv run's own PSMs.")
    args = ap.parse_args()

    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    moddir = args.moddir or os.path.join(os.path.dirname(root), "reference-notes", "metaMorpheusMods")

    entries, unparsed = load_curated(moddir)
    clusters = cluster(entries)
    multi = [c for c in clusters if len(c) > 1]

    print("# Curated mass collisions\n")
    print(f"Files compiled into the binary: {', '.join(LOADED_FILES)}")
    print(f"(NOT loaded, though present: ptmlist.txt, glyco.txt, substitutions.txt, tmt.txt)\n")
    print(f"* curated entries parsed: **{len(entries)}**")
    if unparsed:
        print(f"* entries with an unparseable formula, SKIPPED and counted: {len(unparsed)}")
        for f, i, cf in unparsed:
            print(f"    * [{f}] {i} — CF {cf}")
    print(f"* distinct mass clusters at {CURATED_TOL_DA} Da: **{len(clusters)}**")
    print(f"* clusters holding more than one candidate: **{len(multi)}**")
    n_in_multi = sum(len(c) for c in multi)
    print(f"* entries sitting in a contested cluster: **{n_in_multi}** "
          f"({100.0 * n_in_multi / len(entries):.1f}% of all entries)")
    print(f"* largest cluster: **{max(len(c) for c in clusters)}** candidates\n")

    if not args.sage_tsv:
        print("_No --sage-tsv given, so no background frequencies or reachability._")
        return

    if not os.path.exists(args.sage_tsv):
        raise SystemExit(f"MISSING INPUT: {args.sage_tsv}\n  needed for: background residue frequencies")
    peps = load_peptides(args.sage_tsv)
    n = len(peps)
    print(f"Background from {n} PSMs (target, peptide_q <= 0.01) in `{os.path.basename(args.sage_tsv)}`.\n")

    cache = {}

    def bg(tg):
        key = frozenset(tg)
        if not tg:
            return None
        if key not in cache:
            cache[key] = sum(1 for p in peps if any(ch in tg for ch in p)) / n
        return cache[key]

    print("## Single-residue background containment\n")
    print("| residue | peptides containing it | band % needed for OR >= 2 |")
    print("|---|---|---|")
    for aa in sorted("ACDEFGHIKLMNPQRSTVWY", key=lambda a: -bg({a})):
        f = bg({aa})
        print(f"| {aa} | {100 * f:.1f} % | {100 * required_band_fraction(f):.1f} % |")

    print("\n## Contested masses\n")
    print("Only clusters where at least two candidates are residue-testable "
          "(an `X` acceptor set carries no residue claim and is decided elsewhere).\n")
    print("| mass | candidate | acceptors | background | band % needed |")
    print("|---|---|---|---|---|")
    shown = 0
    for c in clusters:
        testable = [e for e in c if e['tg']]
        if len(c) < 2 or len(testable) < 2:
            continue
        shown += 1
        for j, e in enumerate(testable):
            f = bg(e['tg'])
            need = required_band_fraction(f)
            mass = f"**{e['mass']:+.4f}**" if j == 0 else ""
            acc = ''.join(sorted(e['tg']))
            print(f"| {mass} | {e['id']} | {acc} | {100 * f:.1f} % | {100 * need:.1f} % |")
    print(f"\n_{shown} contested masses with two or more residue-testable candidates._")

    if args.peak is None:
        return
    psms = load_psms_with_delta(args.sage_tsv)
    band, rows = reproduce_peak(entries, psms, args.peak)
    print(f"\n## test_candidate reproduced at {args.peak:+.4f}\n")
    print(f"Band = |delta - peak| <= {BAND_TOL_DA} Da: **{len(band)}** of {len(psms)} PSMs. "
          f"APPROXIMATE — recon uses its own merged-peak binning, not a flat window.\n")
    print("| candidate | acceptors | band containment | background | odds ratio | verdict |")
    print("|---|---|---|---|---|---|")
    for e, res in rows:
        if res is None:
            print(f"| {e['id']} | _(no residue claim)_ | — | — | — | not residue-testable |")
            continue
        kind, a, b, bgf, orv = res
        acc = ''.join(sorted(e['tg']))
        if kind == 'saturated':
            print(f"| {e['id']} | {acc} | {100*a/len(band):.1f} % | {100*bgf:.1f} % | — | "
                  f"background saturated, untestable |")
            continue
        verdict = f"**passes** OR >= {OR_MIN}" if orv >= OR_MIN else f"**fails** OR >= {OR_MIN}"
        print(f"| {e['id']} | {acc} | {100*a/len(band):.1f} % | {100*bgf:.1f} % | {orv:.2f} | {verdict} |")


if __name__ == "__main__":
    main()
