#!/usr/bin/env python3
"""PROTOTYPE of the step-2 recommendation block, for all three test files.

DECISION RULE -- route by specificity. Every peak goes to the ONE instrument that
can decide it, and nothing is judged twice:

  * residue-specific acceptor  -> STATISTICS. Fisher exact one-sided, odds ratio
    as effect size, Benjamini-Hochberg across the sweep. Accept on OR >= 2 AND
    q < 0.05. Both are needed: Gly reaches q = 1e-5 on bcell from sample size
    alone while its odds ratio is 1.37.
  * unspecific acceptor        -> ABUNDANCE. `TG=X` at a terminus, or an acceptor
    set so common the 2x2 saturates (Carbamyl's K/R/C/M sit in 99.6% of
    peptides). The test cannot reach these, so the count floor decides, and the
    report says which instrument was used.
  * isotope satellite          -> demoted before either test.
  * not in the curated list    -> unannotated tail, never tiered.

Why not both tests on every peak: they answer different questions and disagree.
Applying the floor as a gate on top of the statistics drops pyro-Glu, which
carries the strongest odds ratios we measure (27 / 330 / 286) while sitting below
the floor at every X on all three files. Applying statistics as a promotion path
on top of the floor breaks gate 1 with ~30 violations, because gate 1 validates an
abundance ordering and evidence promotion deliberately inverts abundance.

USE OF ODDS RATIO, NOT THE ENRICHMENT RATIO: the enrichment ratio is capped at
1/p_background, which wrongly makes common-residue acceptors look untestable.
Deamidation at 69-74% background caps near 1.4x by ratio and is OR 2.9-11.6.

Candidate names and acceptor sites come from MetaMorpheus's curated mod files
(reference-notes/metaMorpheusMods/), NOT from all of Unimod. Unimod supplies the
element table for computing masses from the CF formulas, and nothing else here.

This is a PROTOTYPE for reading and for pinning numbers. It is not the shipped
implementation.
"""
import re, sys, json
import xml.etree.ElementTree as ET
from pathlib import Path
from scipy.stats import fisher_exact, false_discovery_control as bh

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "testing/scripts"))
from residue_enrichment import band, C13_C12_DIFF
from protein_nterm_evidence import DEFAULT_FASTA, load_fasta, load_rows, protein_start

X_PCT = 20.0            # floor, used ONLY on the abundance path
MASS_TOL = 0.01
OR_MIN = 2.0
Q_MAX = 0.05
NEAR_ZERO = 0.1
# Satellite window. Measured parent+1xC13 offsets are 1.1-1.5 mDa. Must stay well
# below 14 mDa: serum Carboxymethyl at +58.0128 sits 14.3 mDa from the satellite,
# and a 20 mDa window swallowed it as an "isotope satellite".
SAT_TOL = 0.006
# Above this background the 2x2 has no room -- band and background are both ~100%,
# the odds ratio blows up or is undefined, and the test says nothing.
BG_SATURATED = 0.95

# MetaMorpheus ID strings we override. Theirs is loose; TG/DR are correct.
LABEL_OVERRIDE = {"Glu to PyroGlu": "Gln->pyro-Glu (Unimod 28)"}

root = ET.parse(REPO / "testing/reference-data/unimod.xml").getroot()
NS = f"{{{root.tag.split('}')[0].strip('{')}}}" if "}" in root.tag else ""
EL = {e.get("title"): float(e.get("mono_mass")) for e in root.iter(f"{NS}elem")}


def cf_mass(cf):
    tot = 0.0
    for sym, n in re.findall(r"([A-Z][a-z]?(?:\[\d+\])?)\s*(-?\d*)", cf):
        if not sym:
            continue
        if sym not in EL:
            return None
        tot += EL[sym] * (int(n) if n not in ("", "-") else 1)
    return tot


def parse_mm(path):
    out, cur = [], {}
    for line in open(path, encoding="utf-8-sig"):
        if line.strip() == "//":
            if cur:
                out.append(cur); cur = {}
            continue
        m = re.match(r"^([A-Z]{2})\s\s+(.*)$", line.rstrip("\n"))
        if m:
            cur.setdefault(m.group(1), m.group(2).strip())
    if cur:
        out.append(cur)
    return out


CURATED = []
for fn in ["Mods.txt", "aListOfmods.txt", "ProteaseMods.txt", "surfactants.txt"]:
    for e in parse_mm(REPO / "reference-notes/metaMorpheusMods" / fn):
        m = cf_mass(e.get("CF", ""))
        if m is not None:
            e["mass"] = m
            e["label"] = LABEL_OVERRIDE.get(e["ID"], e["ID"])
            CURATED.append(e)

FIXED_CATS = {"Common Fixed"}


def tg_residues(tg):
    """'K or D or E' -> 'KDE'. 'X' -> None (any residue; nothing to test)."""
    parts = [p.strip() for p in tg.split(" or ")]
    res = "".join(p for p in parts if len(p) == 1 and p.isalpha() and p != "X")
    return res or None


def test_candidate(rows, delta, entry, seqs=None):
    """Fisher exact + odds ratio for one curated candidate.

    Returns (odds_ratio, p) on the statistics path, or None when the candidate is
    unspecific -- no residue in TG, a saturated background, or (for a
    protein-terminal candidate) no FASTA to place the peptide in its protein.
    None means "route this to abundance", never "no support".

    PROTEIN-TERMINAL CANDIDATES TAKE THEIR OWN BRANCH -- step 2.5. Before that,
    this function read any PP containing "N-terminal" as a PEPTIDE N-terminus,
    which is a different and much weaker test: on bcell, peptides beginning with
    M are 3.17% of all PSMs against 1.09% at protein position 0, and the shortcut
    passes every internal tryptic peptide that happens to begin with M. That
    made this prototype disagree with the shipped Rust, invisibly, because the
    pinning test covers b1906 where no protein-terminal candidate lands on a
    peak. The rule is: the peptide must start at protein position 0, AND residue
    1 must be in TG. It is NOT a residue-2 test -- see NOTES "Met-loss encoding".
    """
    res = tg_residues(entry.get("TG", ""))
    b = band(rows, delta)
    if not b:
        return None
    protein_terminal = entry.get("PP", "").startswith(("Protein N-terminal",
                                                       "Protein C-terminal"))
    # An empty TG at a PROTEIN terminus is not unspecific -- the POSITION carries
    # the specificity, and protein position 0 holds ~1% of PSMs. Everywhere else
    # an empty TG means "any residue anywhere", which really is untestable.
    if res is None and not protein_terminal:
        return None
    if res is None:
        res = ""   # no residue requirement; position alone decides
    if protein_terminal:
        if seqs is None:
            return None
        met_loss = entry.get("PP", "").find("Met loss") >= 0
        accept = (lambda c: True) if res == "" else (lambda c: c in res)
        if met_loss:
            # TG names the acceptor of the mod that FOLLOWS the removal, at the
            # EXPOSED residue (protein residue 2). Residue 1 is the lost Met and
            # is required by PP itself. Myristoylation is why this matters: every
            # N-terminal myristoyl record in the pinned Unimod is site=G, and the
            # Gly is only an N-terminus after the Met goes.
            hit = lambda p, pr: (p[:1] == "M" and protein_start(seqs, p, pr) is not None
                                 and accept(p[1:2]))
        else:
            hit = lambda p, pr: accept(p[:1]) and protein_start(seqs, p, pr) is not None
    elif "N-terminal" in entry.get("PP", ""):
        hit = lambda p, pr: p.startswith(tuple(res))
    else:
        hit = lambda p, pr: any(a in p for a in res)
    a = sum(1 for p, pr in b if hit(p, pr)); c = len(b) - a
    H = sum(1 for _, p, pr in rows if hit(p, pr)); N = len(rows) - H
    H -= a; N -= c
    if a + c == 0 or H + N == 0:
        return None
    if (H + a) / len(rows) > BG_SATURATED:
        return None
    if 0 in (a, c, H, N):        # Haldane-Anscombe, keeps the odds ratio finite
        orr = ((a + .5) * (N + .5)) / ((c + .5) * (H + .5))
        _, p = fisher_exact([[a, c], [H, N]], alternative="greater")
    else:
        orr, p = fisher_exact([[a, c], [H, N]], alternative="greater")
    return (orr, p)


def render(fkey, seqs=None):
    # (delta, peptide, proteins) -- the third slot carries proteins, not charge,
    # because `test_candidate` needs it for the protein-terminal branch.
    rows, _ = load_rows(fkey)
    md = json.loads((REPO / f"testing/recon-output/full-run/{fkey}.json").read_text())["mod_discovery"]
    nz = [p for p in md["peaks"] if abs(p["delta_mass"]) >= NEAR_ZERO]
    parent = max(nz, key=lambda p: p["count"])["delta_mass"]
    floor = max(p["count"] for p in nz) * X_PCT / 100.0
    is_sat = lambda d: any(abs(d - (parent + n * C13_C12_DIFF)) <= SAT_TOL for n in (1, 2))

    jobs = []
    for p in nz:
        if is_sat(p["delta_mass"]):
            continue
        for e in [c for c in CURATED if abs(c["mass"] - p["delta_mass"]) <= MASS_TOL]:
            jobs.append((p, e, test_candidate(rows, p["delta_mass"], e, seqs)))
    pv = [j[2][1] for j in jobs if j[2] is not None]
    adj = iter(list(bh(pv)) if pv else [])
    bypeak = {}
    for p, e, r in jobs:
        q = next(adj) if r is not None else None
        bypeak.setdefault(id(p), (p, []))[1].append((e, r[0] if r else None, q))

    by_stats, by_abundance, tail = [], [], []
    for p in sorted(nz, key=lambda p: -p["count"]):
        if is_sat(p["delta_mass"]):
            tail.append((p, f"isotope satellite of {parent:+.4f}")); continue
        cands = bypeak.get(id(p), (p, []))[1]
        if not cands:
            tail.append((p, None)); continue
        testable = [c for c in cands if c[1] is not None]
        if testable:
            sup = [c for c in testable if c[1] >= OR_MIN and c[2] < Q_MAX]
            if sup:
                e, o, q = max(sup, key=lambda t: t[1])
                by_stats.append((p, e, f"OR {o:.1f}, q {q:.1e}"))
            else:
                o = max(t[1] for t in testable)
                tail.append((p, f"no residue support (OR {o:.2f})"))
        else:
            e = cands[0][0]
            if p["count"] >= floor:
                by_abundance.append((p, e, f"{100 * p['count'] / floor:.0f}% of floor"))
            else:
                tail.append((p, f"below floor ({p['count']} < {floor:.0f})"))

    print("=" * 80)
    print(f"  RECOMMENDED SEARCH MODIFICATIONS — {fkey}")
    print(f"  {md['total_psms']} PSMs")
    print("=" * 80)
    for title, group, extra in (
            ("Decided by STATISTICS — residue-specific acceptor", by_stats, ""),
            (f"Decided by ABUNDANCE — unspecific acceptor, the test cannot reach it "
             f"(floor {floor:.0f} PSMs)", by_abundance, "")):
        print(f"\n  {title}")
        if not group:
            print("    (none)")
        for p, e, note in group:
            role = "FIXED   " if e.get("MT") in FIXED_CATS else "variable"
            src = "  [role from MetaMorpheus 'Common Fixed']" if role.strip() == "FIXED" else ""
            print(f"    {role} {e['label'][:32]:32s} {p['delta_mass']:+9.4f}  "
                  f"{p['count']:6d} PSMs ({p['count_pct']:5.2f}%)")
            print(f"             {e.get('MT', '?'):20s} {note}{src}")
    print(f"\n  Unranked tail: {len(tail)} peaks, listed not recommended.")
    for p, why in [t for t in tail if t[1]][:5]:
        print(f"    {p['delta_mass']:+9.4f}  {p['count']:6d} PSMs   {why}")


if __name__ == "__main__":
    # The FASTA is gitignored. Without it, protein-terminal candidates are NOT
    # TESTABLE and go to abundance -- the same behaviour the Rust has without
    # `analyze --fasta`. Say which mode this run is in, rather than producing
    # numbers that quietly mean something different.
    seqs = None
    if DEFAULT_FASTA.exists():
        seqs = load_fasta(DEFAULT_FASTA)
        print(f"Protein context: {len(seqs)} sequences from {DEFAULT_FASTA.name}\n")
    else:
        print(f"NO protein context ({DEFAULT_FASTA.name} absent): protein-terminal "
              f"candidates route to abundance\n")
    for f in ["serum", "bcell", "b1906"]:
        render(f, seqs); print()
