#!/usr/bin/env python3
"""Write the Sage configs of the liver claim test.

Reads recon's liver recommendations from
_dev/testing/recon-output/full-run/liver.json and writes one Sage config
per arm (and per stage) into configs/.

Every search mass is the Unimod monoisotopic mass, read from
recon-tool/resources/unimod.xml by Unimod title. The script stops if a
recommended label has no mapping, if the report's sites or position do not
match the mapping, or if the measured delta differs from the Unimod mass by
more than 0.01 Da.

Python 3 standard library only. Run from the repository root.
"""
import copy
import hashlib
import json
import os
import sys
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
LIVER = os.path.join(ROOT, "_dev/testing/recon-output/full-run/liver.json")
UNIMOD = os.path.join(ROOT, "recon-tool/resources/unimod.xml")
OUT = os.path.join(HERE, "configs")
TRIPWIRE_DA = 0.01

# recon label -> (Sage keys, Unimod title, report sites, report position).
# Sage keys: a residue letter, "^X" = peptide N-term on X,
# "[" = protein N-term, "[M" = protein N-term residue M.
MAP = {
    "Carbamidomethyl on C": (["C"], "Carbamidomethyl", "C", "Anywhere."),
    "Oxidation on M": (["M"], "Oxidation", "M", "Anywhere."),
    "Deamidation": (["N", "Q"], "Deamidated", "NQ", "Anywhere."),
    "Gln->pyro-Glu": (["^Q"], "Gln->pyro-Glu", "Q", "Peptide N-terminal."),
    "Fe[III]": (["D", "E"], "Cation:Fe[III]", "DE", "Anywhere."),
    "Trioxidation": (["C"], "Trioxidation", "C", "Anywhere."),
    # Sage df92199 never removes the initiator Met (see README). "[M" puts
    # -89.029920 on a protein-N-terminal Met. The M residue then weighs
    # 42.010565, the mass of an N-terminal acetyl, so precursor and b/y
    # masses equal those of the Met-clipped, acetylated peptide.
    "Met-loss+Acetylation": (["[M"], "Met-loss+Acetyl", "", "Protein N-terminal, Met loss."),
    "Dehydroalanine": (["C"], "Cys->Dha", "C", "Anywhere."),
    "Formylation": (["K"], "Formyl", "K", "Anywhere."),
    "Water Loss (Glu->pyro-Glu)": (["^E"], "Glu->pyro-Glu", "E", "Peptide N-terminal."),
    "Acetylation": (["["], "Acetyl", "", "Protein N-terminal."),
    "Oxidation to Kynurenine": (["W"], "Trp->Kynurenin", "W", "Anywhere."),
    "Oxidation and then loss of oxidized M side chain": (["M"], "Met->AspSA", "M", "Anywhere."),
}

# Ben's vanilla search, as specified for the claim test.
VANILLA_STATIC = {"C": 57.021464}
VANILLA_VARIABLE = {
    "M": [15.994915],
    "^Q": [-17.026549],
    "N": [0.984016],
    "Q": [0.984016],
    "[": [42.010565],
}

VANILLA_TOL = 20.0
# recon's user recommendation for this file. liver.html renders
# "Recommended MS1 / MS2 10 / 10 ppm"; liver.json has
# ms1_calibration.user_recommendation_tolerance_ppm = 10.0. The MS2 half is
# the same ladder rung (report.rs, ms2_user_recommendation). The
# ms2_tolerance_*_ppm fields (+/-1.14) are recon's internal pass-2 window,
# not the recommendation, and are not used.
RECON_TOL = 10.0

# Staging order, fixed before any run: cheap rare-site mods first, the C
# mods next, Fe[III] on D and E (the largest combinatorial load) last.
STAGES = [
    ("stage1_rare", ["Water Loss (Glu->pyro-Glu)", "Met-loss+Acetylation",
                     "Oxidation to Kynurenine",
                     "Oxidation and then loss of oxidized M side chain", "Formylation"]),
    ("stage2_cys", ["Trioxidation", "Dehydroalanine"]),
    ("stage3_fe", ["Fe[III]"]),
]


def unimod_masses():
    ns = "{http://www.unimod.org/xmlns/schema/unimod_2}"
    out = {}
    for m in ET.parse(UNIMOD).getroot().iter(ns + "mod"):
        out[m.get("title")] = float(m.find(ns + "delta").get("mono_mass"))
    return out


def base(tol):
    return {
        "database": {
            "enzyme": {
                "missed_cleavages": 2,
                "min_len": 7,
                "max_len": 50,
                "cleave_at": "KR",
                "restrict": "P",
                "c_terminal": True,
                "semi_enzymatic": False,
            },
            "static_mods": {},
            "variable_mods": {},
            "max_variable_mods": 2,
            "generate_decoys": True,
        },
        "precursor_tol": {"ppm": [-tol, tol]},
        "fragment_tol": {"ppm": [-tol, tol]},
        "precursor_charge": [2, 4],
    }


def add_var(vm, key, mass):
    lst = vm.setdefault(key, [])
    if mass not in lst:
        lst.append(mass)


def main():
    uni = unimod_masses()
    with open(LIVER, "rb") as fh:
        raw = fh.read()
    rep = json.loads(raw)
    rec = rep["recommendations"]

    rows = []  # (role, label, sage keys, unimod title, unimod mass, measured)
    for role in ("fixed", "variable"):
        for r in rec[role]:
            lab = r["label"]
            if lab not in MAP:
                sys.exit(f"STOP: no Sage mapping for recommended label {lab!r}")
            keys, title, sites, pos = MAP[lab]
            if r["sites"] != sites or r["position"] != pos:
                sys.exit(f"STOP: {lab}: report says sites={r['sites']!r} "
                         f"position={r['position']!r}; mapping expects {sites!r} {pos!r}")
            mass = uni[title]
            diff = r["delta_mass"] - mass
            if abs(diff) > TRIPWIRE_DA:
                sys.exit(f"STOP: {lab}: measured {r['delta_mass']:.6f} vs Unimod "
                         f"{title} {mass:.6f} differ by {diff:+.6f} Da")
            rows.append((role, lab, keys, title, mass, r["delta_mass"], r["count"]))

    fixed = [r for r in rows if r[0] == "fixed"]
    if [r[1] for r in fixed] != ["Carbamidomethyl on C"]:
        sys.exit(f"STOP: expected one fixed mod (Carbamidomethyl on C), got {fixed}")

    def recon_mods(labels=None):
        static = {"C": uni["Carbamidomethyl"]}
        vm = {}
        for role, lab, keys, title, mass, _, _ in rows:
            if role != "variable" or (labels is not None and lab not in labels):
                continue
            for k in keys:
                add_var(vm, k, mass)
        return static, vm

    os.makedirs(OUT, exist_ok=True)
    arms = {}

    def put(name, static, vm, tol):
        c = base(tol)
        c["database"]["static_mods"] = copy.deepcopy(static)
        c["database"]["variable_mods"] = copy.deepcopy(vm)
        arms[name] = c

    rs, rv = recon_mods()
    put("arm1_vanilla", VANILLA_STATIC, VANILLA_VARIABLE, VANILLA_TOL)
    put("arm2_vanilla_mods_recon_tol", VANILLA_STATIC, VANILLA_VARIABLE, RECON_TOL)
    put("arm3_recon_mods_vanilla_tol", rs, rv, VANILLA_TOL)
    put("arm4_recon_full", rs, rv, RECON_TOL)

    # Stages: the vanilla-equivalent recon mods, plus each group in turn,
    # at recon's tolerances (the arm 4 setting).
    core = ["Oxidation on M", "Deamidation", "Gln->pyro-Glu", "Acetylation"]
    have = list(core)
    for name, labels in STAGES:
        have += labels
        s, v = recon_mods(set(have))
        put(f"arm4_{name}", s, v, RECON_TOL)
    all_var = {r[1] for r in rows if r[0] == "variable"}
    if set(have) != all_var:
        sys.exit(f"STOP: stages do not cover every recommended mod: {all_var ^ set(have)}")
    if arms["arm4_stage3_fe"] != arms["arm4_recon_full"]:
        sys.exit("STOP: the last stage is not identical to arm 4")
    del arms["arm4_stage3_fe"]  # identical to arm4_recon_full

    for name, c in arms.items():
        with open(os.path.join(OUT, name + ".json"), "w") as fh:
            json.dump(c, fh, indent=2)
            fh.write("\n")

    with open(os.path.join(OUT, "mod_mapping.tsv"), "w") as fh:
        fh.write("role\trecon_label\tsage_keys\tunimod_title\tunimod_mass\t"
                 "recon_measured_delta\tdiff_Da\trecon_psm_count\n")
        for role, lab, keys, title, mass, meas, n in rows:
            fh.write(f"{role}\t{lab}\t{' '.join(keys)}\t{title}\t{mass:.6f}\t"
                     f"{meas:.6f}\t{meas - mass:+.6f}\t{n}\n")

    print(f"liver.json sha1 {hashlib.sha1(raw).hexdigest()} "
          f"generated_at {rep['generated_at']} tool_version {rep['tool_version']} "
          f"git_commit {rep['git_commit']}")
    print(f"MS1 user recommendation: +/-{rep['ms1_calibration']['user_recommendation_tolerance_ppm']} ppm")
    for role, lab, keys, title, mass, meas, n in rows:
        print(f"  {role:8s} {lab:50s} {' '.join(keys):6s} {mass:+.6f} (measured {meas:+.6f})")
    print(f"wrote {len(arms)} configs to {os.path.relpath(OUT, ROOT)}")


if __name__ == "__main__":
    main()
