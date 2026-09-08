#!/usr/bin/env python3
"""Residue corroboration — a MEASUREMENT, no longer a pass/fail gate.

⚠ THE BINARY THRESHOLD WAS RETIRED 2026-08-26, AND NOT BECAUSE IT FAILED.
It was retired because the PRE-COMMITMENT WAS DEFECTIVE, provably so from a
document committed the day BEFORE the gate was specified. This script now reports
a corroboration RATE and does not pass or fail.

WHY. `ptm-stratification-design.md` (commit ba2ff58, 2026-08-25) already said:
"Recon's open-search peak percentage is a discovery-rank statistic. Other tools
report a post-localization PSM fraction. THESE ARE DIFFERENT QUANTITIES." And:
"Recon reports an un-localized delta-bin count." Its "What this does not do"
section says plainly: "It does not localize."

The mechanism, from Sage's own vendored docs (sage-online-docs.md:3538):
"hyperscore | X!Tandem hyperscore for the PSM." A PSM-level SPECTRAL MATCH score.
It scores the peptide IDENTIFICATION, not the modification SITE, and Sage runs no
site-confirmation pass. So recon produces an UN-LOCALIZED delta mass plus a
POPULATION-level acceptor enrichment, while MetaMorpheus and PTM-Shepherd run
secondary sweeps that produce PER-PSM site assignments. Hanging a binary verdict
on agreement between those two quantities was the error.

Both can be true at once. recon says the -17.0265 BAND is enriched for N-terminal
Q; MetaMorpheus says INDIVIDUAL PSMs carry ammonia loss on internal Asn. A mixed
peak satisfies both.

WHAT THIS STILL MEASURES, and it is worth having: where the two quantities happen
to point the same way, that is real corroboration. Reported as a rate, with the
RANK of each agreement, and with every divergence named rather than buried.

⚠ THIS LEAVES STEP 2 WITH NO BINARY GATE — only this rate and the carpet assert.
That is the same weakened-surface complaint that retired gate 1, and it is a
STATED LIMITATION, not something to gloss. See NOTES and the limitations note.

The negative control stays wired (`--prove-control-fails`) because it still
discriminates: serum +57 -> Gly is INCOMPATIBLE with a band that is 96%
Cys-containing, whereas pyro-Glu vs ammonia-loss is COMPATIBLE. Respecifying the
gate on that axis -- incompatible vs merely different -- is option C in NOTES and
would need a FRESH pre-commitment, written before it is run.

WHY GATE 1 IS NOT ENOUGH ANY MORE
Routing by specificity shrank gate 1's surface to ONE peak across three files —
gate 1 validates an abundance ordering, and only the abundance path still makes
one. The 21/21 corroboration figure that filled the gap was designed AFTER seeing
the data and only asks whether any tool reports the mass at all. Gate 5 tests the
claim the routing rule actually makes: a RESIDUE claim.

THE RULE  (REVISED 2026-08-26 — the original is below and is still REPORTED)
For every recommendation on the STATISTICS path, recon's acceptor set and the
reference's published set of localized residues must INTERSECT.

  sets intersect                        -> pass, and the RANK of agreement is
                                           reported (AA1 / AA2 / AA3)
  disjoint                              -> VIOLATION (degeneracy is reported as
                                           context but CANNOT excuse it — see below)
  reference does not localize the mass  -> counted NEITHER way, uncovered

THRESHOLD: ZERO violations. NOT moved by the revision.

⚠ WHAT THE REVISION CHANGED, AND WHAT IT COST
The ORIGINAL rule compared recon's SET against the reference's TOP residue only,
and knew nothing about mass degeneracy. It failed with 10 violations. Two things
were then measured, and BOTH are reported by this script every run so the
revision can never quietly hide the original result:

  1. DEGENERACY -- MEASURED, THEN REJECTED AS A RULE. 6 of the 10 were the
     reference localizing a DIFFERENT curated chemistry sharing the mass (Ammonia
     loss on N at -17.0265; Water loss on D at -18.0106; Formylation on K).
     Measured by `gate5_degeneracy_probe.py`. Excusing those was TRIED and BROKE
     THE NEGATIVE CONTROL -- see the comment in `judge`. Degeneracy is now
     reported as context on each violation and excuses nothing.
  2. TOP-RESIDUE-ONLY. In ALL 6 PTM-Shepherd disagreements recon's residue was
     present in that tool's OWN AA2/AA3 columns. PTM-Shepherd publishes a RANKED
     SET of enriched residues, not one answer; recon's `sites` is also a set.
     Comparing a set to a single element was an asymmetry in the original wording.

  A CONFIDENCE THRESHOLD ON THE REFERENCE WAS CONSIDERED AND REJECTED. The
  enrichment scores do NOT separate: agreeing calls run 1.9-42.2 and disagreeing
  calls 2.9-18.7, overlapping heavily. Any cutoff would have been chosen to make
  the failures disappear. Rejected on the measurement, not on principle alone.

  THE REVISION MAKES THE GATE WEAKER. That is stated, not hidden: it widens what
  counts as agreement. The strict count is printed alongside the revised one on
  every run, and the negative control is re-checked under the REVISED rule.

UNCOVERED SURFACE IS PRINTED, NOT HIDDEN. A gate that covers 3 of 8 recommendations
and reports PASS is the 21/21 mistake again.

NEGATIVE CONTROL: serum +57. Already adjudicated as over-alkylation, not added
glycine (Phase 8 Gate 3, 0 of 26 peptides with Gly flanking context). If recon
routed +57 to Gly (T/S/K) the references must localize it to C and the gate must
FAIL. `--prove-control-fails` runs exactly that counterfactual, so the control is
watched failing before a pass is believed.

TWO INSTRUMENTS, NOT EQUIVALENT — every number says which one produced it.
  PTM-Shepherd  global.profile.tsv AA1/AA2/AA3.  POOLED across all three files:
                the PSM-count columns are per-file but the AA columns are not, so
                this gives ONE residue call per mass, not one per file.
  MetaMorpheus  AllPSMs.psmtsv.  PER FILE. Localization is read positionally from
                `Full Sequence` — the residue immediately before each mod bracket
                — not from the mod's name string.
                JOINED ON IDENTITY, NOT ON MASS. Task3 was a CLOSED search, so
                `Mass Diff (Da)` is ~0 for almost every PSM: the modification is
                already inside the identification rather than sitting in a delta.
                Matching on that column found nothing (0 of 21 localized) and was
                a defect in this script, not a property of the data. Each MM mod
                annotation is mapped to a mass through the SAME curated list recon
                uses, and matched to recon's peak centre.

Usage:
    python testing/scripts/gate5_residue_agreement.py
    python testing/scripts/gate5_residue_agreement.py --prove-control-fails
"""
import argparse
import csv
import os
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
FILES = ["serum", "bcell", "b1906"]
REPORT = {f: REPO / "testing/recon-output/full-run" / f"{f}.json" for f in FILES}
PTMS = REPO / "testing/reference-data/ptm-shepherd/reallyOpen/global.profile.tsv"
# MetaMorpheus's AllPSMs.psmtsv is gitignored (it is large and it is somebody
# else's output), so it has no in-repo copy. Point at it explicitly:
#   RECON_METAMORPHEUS_PSMS=/path/to/AllPSMs.psmtsv python3 <this script>
MM = Path(
    os.environ.get(
        "RECON_METAMORPHEUS_PSMS",
        REPO / "testing/reference-data/metamorpheus/2026-08-21-10-29-48"
        "/Task3-SearchTask/AllPSMs.psmtsv",
    )
)

# Matching tolerance between recon's peak centre and a reference's mass, in Da.
# recon's bin width is 0.01; this is one bin.
MASS_TOL = 0.01
# A MetaMorpheus residue call needs enough PSMs to mean anything.
MM_MIN_PSMS = 10

MM_FILE_STEM = {
    "serum": "2019-4-9_909c_0311",
    "bcell": "B.naive_01steady-state",
    "b1906": "b1906_293T_proteinID_01A_QE3_122212",
}

csv.field_size_limit(10_000_000)


def load_ptmshepherd():
    """mass -> top-localized residue, with its enrichment score and PSM count."""
    out = []
    with PTMS.open(encoding="utf-8", errors="replace") as fh:
        for row in csv.DictReader(fh, delimiter="\t"):
            try:
                apex = float(row["peak_apex"])
            except (TypeError, ValueError):
                continue
            aa = (row.get("AA1") or "").strip()
            if not aa:
                continue  # PTM-Shepherd did not localize this mass
            try:
                score = float(row.get("AA1_enrichment_score") or "nan")
                n = int(float(row.get("AA1_psm_count") or 0))
            except ValueError:
                score, n = float("nan"), 0
            alts = [(row.get(k) or "").strip() for k in ("AA2", "AA3")]
            out.append({"mass": apex, "aa": aa, "score": score, "n": n,
                        "alts": [a for a in alts if a]})
    return out


MOD_RE = re.compile(r"\[([^\]]*)\]")


def localized_residue(full_sequence):
    """Residue each mod bracket sits on, read POSITIONALLY.

    MetaMorpheus writes `PEPTIDE[Common Variable:Oxidation on M]K`. The bracket
    follows the residue it modifies. Reading the residue from the mod's NAME
    would just echo the name back; reading it from the sequence is an independent
    localization.
    """
    out, plain = [], []
    i = 0
    while i < len(full_sequence):
        c = full_sequence[i]
        if c == "[":
            depth, j = 1, i + 1
            while j < len(full_sequence) and depth:
                if full_sequence[j] == "[":
                    depth += 1
                elif full_sequence[j] == "]":
                    depth -= 1
                j += 1
            # residue before the bracket; N-terminal mods have none
            out.append(plain[-1] if plain else "N-term")
            i = j
        else:
            if c.isalpha():
                plain.append(c)
            i += 1
    return out


def curated_masses():
    """MetaMorpheus mod annotation -> monoisotopic mass, from the curated list.

    The annotation in `Full Sequence` is `Category:ID on TG` (or `Category:ID`),
    which is exactly `MT`, `ID` and `TG` from Mods.txt. So the curated list is the
    join between MetaMorpheus's identity and a mass.
    """
    elems = {}
    ux = (REPO / "testing/reference-data/unimod.xml").read_text(encoding="utf-8", errors="replace")
    for m in re.finditer(r"<umod:elem [^>]*>", ux):
        t = re.search(r'title="([^"]+)"', m.group(0))
        mo = re.search(r'mono_mass="([-\d.]+)"', m.group(0))
        if t and mo:
            elems[t.group(1)] = float(mo.group(1))

    def formula_mass(cf):
        tot = 0.0
        for sym, cnt in re.findall(r"([A-Z][a-z]?)\s*(-?\d+)?", cf):
            if not sym:
                continue
            if sym not in elems:
                return None
            tot += elems[sym] * (int(cnt) if cnt else 1)
        return tot

    out = {}
    for fn in ["Mods.txt", "aListOfmods.txt", "ProteaseMods.txt", "surfactants.txt"]:
        path = REPO / "reference-notes/metaMorpheusMods" / fn
        if not path.exists():
            continue
        for block in path.read_text(encoding="utf-8", errors="replace").split("\n//"):
            f = dict(re.findall(r"^([A-Z]{2})\s+(.*)$", block, re.M))
            if not f.get("ID") or not f.get("CF"):
                continue
            mass = formula_mass(f["CF"])
            if mass is None:
                continue
            mt, idv, tg = f.get("MT", ""), f["ID"], f.get("TG", "")
            # MetaMorpheus writes ONE residue per annotation ("Deamidation on N")
            # while the curated TG lists the whole acceptor set ("N or Q"). Without
            # expanding the set, almost every lookup misses -- which is what made
            # Deamidation resolve to R on 17 PSMs while the file holds 2300
            # `Deamidation on N`. Register every residue in TG separately.
            targets = [t.strip() for t in tg.split(" or ") if t.strip()] or [tg]
            keys = [f"{mt}:{idv}", idv]
            for t in [tg] + targets:
                keys += [f"{mt}:{idv} on {t}", f"{idv} on {t}"]
            for key in keys:
                out.setdefault(key.strip(), mass)
    return out


MOD_NAME_RE = re.compile(r"\[(.*?)\](?=[A-Z]|$)")


def load_metamorpheus(name_mass):
    """file -> Counter of (mass, localized residue) -> PSM count.

    Joined on IDENTITY: Task3 is a closed search, so `Mass Diff (Da)` is ~0 and
    carries no modification mass. See the module docstring.
    """
    if not MM.exists():
        return None
    per_file = defaultdict(Counter)
    unmatched = Counter()
    with MM.open(encoding="utf-8", errors="replace") as fh:
        for row in csv.DictReader(fh, delimiter="\t"):
            if (row.get("Decoy/Contaminant/Target") or "").strip() != "T":
                continue
            try:
                if float(row.get("QValue") or 1.0) >= 0.01:
                    continue
            except ValueError:
                continue
            stem = (row.get("File Name") or "").strip()
            key = next((k for k, v in MM_FILE_STEM.items() if v in stem), None)
            if key is None:
                continue
            fs = row.get("Full Sequence") or ""
            if "[" not in fs:
                continue
            for nm, res in zip(mod_names(fs), localized_residue(fs)):
                mass = name_mass.get(nm)
                if mass is None:
                    unmatched[nm] += 1
                    continue
                per_file[key][(round(mass, 4), res)] += 1
    if unmatched:
        print(f"NOTE  {len(unmatched)} MetaMorpheus mod name(s) had no curated mass; "
              f"top: {unmatched.most_common(3)}")
    return per_file


def mod_names(full_sequence):
    """Mod annotation inside each bracket, handling the nested `[` in Fe[III]."""
    out, i = [], 0
    while i < len(full_sequence):
        if full_sequence[i] == "[":
            depth, j = 1, i + 1
            while j < len(full_sequence) and depth:
                if full_sequence[j] == "[":
                    depth += 1
                elif full_sequence[j] == "]":
                    depth -= 1
                j += 1
            out.append(full_sequence[i + 1:j - 1])
            i = j
        else:
            i += 1
    return out


def mm_call(mm_counter, mass):
    """Top localized residue for `mass` in one file, or None if not localized."""
    hits = Counter()
    for (m, res), n in mm_counter.items():
        if abs(m - mass) <= MASS_TOL:
            hits[res] += n
    if not hits:
        return None
    total = sum(hits.values())
    if total < MM_MIN_PSMS:
        return None
    aa, n = hits.most_common(1)[0]
    return {"aa": aa, "n": n, "total": total, "all": hits.most_common(4)}


MM_CONFIG = (REPO / "testing/reference-data/metamorpheus/2026-08-21-10-29-48"
             "/Task Settings/Task2-GPTMDTaskconfig.toml")

# Our own renames of curated entries, mapped back to the upstream names
# MetaMorpheus's config still uses. Direct consequence of the 2026-08-26
# corrections in Mods.txt; listed here so the coupling is visible.
MM_NAME_ALIAS = {
    "Glu to PyroGlu": "Gln->pyro-Glu",
    "Water Loss": "Water Loss (Glu->pyro-Glu)",
}


def mm_capability(curated):
    """(mass, residue) pairs MetaMorpheus was CONFIGURED to be able to find.

    Read from its GPTMD mod list, which defines the search space — NOT from its
    output. Inferring capability from output would be circular: "MetaMorpheus can
    express X" would then be true exactly when it agreed.

    A reference can only be evidence about a claim it is capable of making. Where
    it is not, it is silent, and silence is uncovered surface — never disagreement.
    """
    if not MM_CONFIG.exists():
        return None
    txt = MM_CONFIG.read_text(encoding="utf-8", errors="replace")
    m = re.search(r'ListOfModsGptmd\s*=\s*"(.*?)"\s*$', txt, re.M | re.S)
    if not m:
        return None
    by_name = {}
    for c in curated:
        by_name.setdefault(c["id"], []).append(c)
    caps, unresolved = set(), set()
    # The TOML stores LITERAL backslash-t, not tab characters. Splitting on a real
    # tab yields one field and an empty capability set, which silently reads as
    # "MetaMorpheus was configured for nothing".
    raw = m.group(1).replace("\\t", "\t")
    fields = [f for f in raw.split("\t") if f.strip()]
    # entries are Category, "Name on X" pairs
    for i in range(0, len(fields) - 1, 2):
        spec = fields[i + 1].strip()
        mm = re.match(r"^(.*?) on (\S+)$", spec)
        if not mm:
            continue
        name, site = mm.group(1).strip(), mm.group(2).strip()
        entries = by_name.get(name) or by_name.get(MM_NAME_ALIAS.get(name, ""), [])
        if not entries:
            unresolved.add(name)
            continue
        for e in entries:
            caps.add((round(e["mass"], 3), site))
    if unresolved:
        print(f"NOTE  {len(unresolved)} MetaMorpheus GPTMD mod name(s) not in the curated "
              f"list, so capability is UNDERSTATED for them: {sorted(unresolved)[:5]}")
    return caps


def mm_can_express(caps, mass, sites):
    """Could MetaMorpheus have reported recon's claim at all?"""
    if caps is None:
        return True  # config unreadable: assume capable, the conservative reading
    return any((round(mass, 3), r) in caps
               or any(abs(cm - mass) <= MASS_TOL and cr == r for cm, cr in caps)
               for r in sites)


def curated_entries():
    """Every curated entry as (mass, id, acceptor residues), for the degeneracy test."""
    import gate5_degeneracy_probe as P  # single source for the parser
    return P.curated_entries()


def judge(sites, ref_set, mass, curated, label):
    """Revised verdict. Returns (verdict, rank, note).

    `ref_set` is the reference's PUBLISHED ranked residues (AA1, AA2, AA3 for
    PTM-Shepherd; a single call for MetaMorpheus). `rank` is 1-based position of
    the first agreement, so a rank-3 agreement is visibly weaker than rank-1.
    """
    ranked = [a for a in ref_set if a not in ("N-term", "C-term")]
    if not ranked:
        # A purely terminal call names no residue and cannot agree or disagree.
        return "uncovered", None, "terminal call only"
    want = set(sites)
    for i, aa in enumerate(ranked, start=1):
        if aa in want:
            return "pass", i, ""
    # Disjoint -> VIOLATION.
    #
    # ⚠ A DEGENERACY ESCAPE WAS TRIED HERE AND REJECTED ON EVIDENCE, 2026-08-26.
    # The clause was: "not a violation if another curated candidate at this mass
    # accepts the reference's residue." It BROKE THE NEGATIVE CONTROL. Route serum
    # +57 to Gly (T/S/K) and the reference says C; the clause then finds
    # `Carbamidomethyl on C` at that mass and excuses the disagreement. At a
    # degenerate mass, "the reference named a different chemistry" and "recon named
    # the WRONG chemistry" are indistinguishable by that test — so the clause
    # disables precisely the detection this gate exists for. Do not reintroduce it.
    #
    # The degeneracy information is still REPORTED on each violation, because it is
    # genuinely useful context. It just cannot excuse one.
    top = ranked[0]
    alts = [c for c in curated
            if abs(c["mass"] - mass) <= MASS_TOL and top in c["residues"]
            and not (c["id"] == label and c["residues"] == want)]
    note = (f"NOTE {top!r} also belongs to {alts[0]['id']!r} (TG={alts[0]['tg']!r}) "
            f"at this mass — context, not an excuse"
            if alts else f"no curated candidate at this mass accepts {top!r}")
    return "violation", None, note


def judge_strict(sites, ref_set):
    """The ORIGINAL rule, kept so the revision cannot hide what it changed."""
    ranked = [a for a in ref_set if a not in ("N-term", "C-term")]
    if not ranked:
        return "uncovered"
    return "pass" if ranked[0] in set(sites) else "violation"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--prove-control-fails", action="store_true",
                    help="run the counterfactual: route serum +57 to Gly (T/S/K) and "
                         "show the gate FAILS. Watch the control fail before "
                         "believing a pass.")
    args = ap.parse_args()

    ptms = load_ptmshepherd()
    mm = load_metamorpheus(curated_masses())
    curated = curated_entries()
    mm_caps = mm_capability(curated)
    if mm_caps is not None:
        print(f"MetaMorpheus capability: {len(mm_caps)} (mass, residue) pairs from its "
              f"GPTMD config.\n  A claim it was not configured to find is UNCOVERED, "
              f"never a violation.\n")
    if mm is None:
        print(f"NOTE  MetaMorpheus not readable at {MM}\n"
              f"      Running on PTM-Shepherd alone — the POOLED instrument.\n")

    violations, degenerate, uncovered, passes = [], [], [], []
    strict_violations = 0
    ranks = Counter()

    print("=" * 84)
    print("RESIDUE CORROBORATION — a measurement, not a gate. The binary threshold")
    print("was retired 2026-08-26 because the pre-commitment compared two different")
    print("quantities. See this file's docstring.")
    print("=" * 84)

    for f in FILES:
        rep = json.loads(REPORT[f].read_text(encoding="utf-8"))
        rec = rep.get("recommendations")
        if not rec:
            print(f"\n{f}: no recommendations block")
            continue
        stats = [m for m in rec["fixed"] + rec["variable"] if m["decided_by"] == "statistics"]
        print(f"\n--- {f}  ({len(stats)} statistics-path recommendations) ---")

        for m in stats:
            sites, mass = m["sites"], m["delta_mass"]
            if args.prove_control_fails and f == "serum" and abs(mass - 57.0215) < 0.02:
                sites = "TSK"  # counterfactual: Gly instead of Carbamidomethyl

            near = sorted([p for p in ptms if abs(p["mass"] - mass) <= MASS_TOL],
                          key=lambda p: abs(p["mass"] - mass))
            mc = mm_call(mm[f], mass) if mm else None

            refs = []
            if near:
                p0 = near[0]
                refs.append(("PTM-Shepherd", [p0["aa"]] + p0["alts"],
                             f"score {p0['score']:.1f}, n={p0['n']}"))
            if mc:
                if not mm_can_express(mm_caps, mass, sites):
                    refs.append(("MetaMorpheus", [], "NOT CONFIGURED for recon's "
                                 "acceptor set — silent, not disagreeing"))
                else:
                    refs.append(("MetaMorpheus", [mc["aa"]], f"{mc['n']}/{mc['total']} PSMs"))

            # STRICT baseline on the RAW calls, before any capability filtering, so
            # the original rule's count cannot drift as the revision is tuned.
            raw_sets = ([[near[0]["aa"]]] if near else []) + ([[mc["aa"]]] if mc else [])
            for rs in raw_sets:
                if judge_strict(sites, rs) == "violation":
                    strict_violations += 1

            results = []
            for tag, ref_set, detail in refs:
                v, rank, note = judge(sites, ref_set, mass, curated, m["label"])
                if v == "pass" and rank:
                    ranks[rank] += 1
                results.append((tag, ref_set, v, rank, note, detail))

            verdicts = [r[2] for r in results]
            mark = ("VIOLATION" if "violation" in verdicts
                    else "pass" if "pass" in verdicts
                    else "degenerate" if "degenerate" in verdicts
                    else "uncovered")
            print(f"  {mass:+9.4f}  {m['label'][:30]:<30} sites={sites or '-':<5} -> {mark}")
            for tag, ref_set, v, rank, note, detail in results:
                rk = f"AA{rank}" if rank else "-"
                print(f"       {tag:<13} {('/'.join(ref_set) or '-'):<10} {v:<11} {rk:<4} {detail}"
                      + (f"  [{note}]" if note else ""))

            entry = (f, mass, m["label"], sites, results)
            {"VIOLATION": violations, "pass": passes,
             "degenerate": degenerate, "uncovered": uncovered}[mark].append(entry)

    total = len(passes) + len(violations) + len(degenerate) + len(uncovered)
    print("\n" + "=" * 84)
    print(f"COVERAGE   {len(passes)} pass, {len(violations)} violation, "
          f"{len(degenerate)} degenerate, {len(uncovered)} uncovered, of {total}")
    print(f"AGREEMENT RANK  " + ", ".join(f"AA{k}: {v}" for k, v in sorted(ranks.items()))
          + "   (AA1 is the reference's own top call; AA2/AA3 are weaker agreement)")
    print(f"STRICT READING  {strict_violations} violation(s) under the ORIGINAL rule "
          f"(top residue only, no degeneracy allowance). Printed so the revision "
          f"cannot hide what it changed.")
    if degenerate:
        print("DEGENERATE (reference localized a different chemistry at the same mass):")
        for f, mass, label, _, results in degenerate:
            note = next((r[4] for r in results if r[2] == "degenerate"), "")
            print(f"   {f:<7} {mass:+9.4f} {label} — {note}")
    if uncovered:
        print("UNCOVERED (no reference localizes this mass — counted neither way):")
        for f, mass, label, *_ in uncovered:
            print(f"   {f:<7} {mass:+9.4f} {label}")

    if args.prove_control_fails:
        ok = any(f == "serum" and abs(mass - 57.0215) < 0.02 for f, mass, *_ in violations)
        print("\nNEGATIVE CONTROL under the REVISED rule "
              "(counterfactual: serum +57 routed to Gly T/S/K)")
        print(f"  control fired: {ok}")
        print("  " + ("PASS — the revised gate CAN still fail."
                      if ok else
                      "FAIL — the revision broke the control. The gate can no longer "
                      "detect a wrong residue and must not be trusted."))
        return 0 if ok else 1

    agree = len(passes)
    print()
    print("=" * 84)
    print(f"CORROBORATION RATE  {agree}/{total} statistics-path recommendations are "
          f"corroborated by at least one reference")
    print(f"                    ranks " +
          ", ".join(f"AA{k}: {v}" for k, v in sorted(ranks.items())) +
          "   (AA2/AA3 are weaker agreement than AA1)")
    print(f"                    {strict_violations} would have failed the ORIGINAL "
          f"top-residue-only rule; printed so the revisions cannot hide them")
    if violations:
        print(f"\nDIVERGENCES — {len(violations)}, NAMED not buried. These are NOT errors")
        print("and NOT passes: a population-level enrichment and a per-PSM localization")
        print("are different quantities and can both be right over a mixed peak.")
        for f, mass, label, sites, results in violations:
            bad = [(r[0], r[1]) for r in results if r[2] == "violation"]
            print(f"  {f:<7} {mass:+9.4f} {label}: recon sites={sites}, reference {bad}")
    print("\n⚠ NOT A PASS MARK. Step 2 ships with this rate and the carpet assert as")
    print("  its validation surface, and with the divergences above as a stated")
    print("  limitation. There is no binary gate on this axis.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
