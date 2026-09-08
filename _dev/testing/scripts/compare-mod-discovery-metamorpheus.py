#!/usr/bin/env python3
"""
Mod-discovery cross-comparison: Sage-Recon vs other tools.

OBJECTIVE benchmark (tool-vs-tool), NOT a gate and NOT a comparison to the
tool's author. Sage-Recon is the fixed reference column; each other tool is
adapted into a generic mod-table and compared against it. See NOTES "Software
comparison is tool-vs-tool" (locked). Divergence is expected (different engines,
FDR, windows) — the six methodology deltas are in
testing/reference-data/ptm-shepherd/README.md.

N-tool shape: add a new adapter function to compare another tool (Mascot,
Preview, MetaMorpheus). The alignment + metric core does not change.

Metric family (per file, in the shared mass window, percentages as currency):
  - presence: is a peak/mod found by both tools at the same mass (+/- tol)?
  - rank:     ordering by PSM count among matched mods.
  - prevalence: per-file percent-of-PSMs for each matched mod, side by side.

Usage (paths relative to repo root):
    # vs PTM-Shepherd (open OR reallyOpen — just point at the modsummary)
    python testing/scripts/compare_mod_discovery.py \
        --recon-json testing/recon-output/full-run/bcell.json:bcell \
                     testing/recon-output/full-run/serum.json:serum \
                     testing/recon-output/full-run/b1906.json:b1906 \
        --ptmshepherd testing/reference-data/ptm-shepherd/reallyOpen/global.modsummary.tsv \
        --tool-name "PTM-Shepherd (reallyOpen)" \
        --out-name recon_vs_ptmshepherd_reallyOpen.md

    # vs Mascot error-tolerant (name+site output -> resolved to mass, sites rolled up)
    python testing/scripts/compare_mod_discovery.py \
        --recon-json .../bcell.json:bcell .../serum.json:serum .../b1906.json:b1906 \
        --mascot-dir testing/reference-data/mascot/error-tolerant \
        --tool-name "Mascot (error-tolerant)" --out-name recon_vs_mascot.md

    # vs MetaMorpheus (Task3-SearchTask ALLPSMs.psmtsv, GPTMD-confirmed, reallyOpen tier)
    python testing/scripts/compare_mod_discovery.py \
        --recon-json testing/recon-output/nofixedmods/bcell.json:bcell \
                     testing/recon-output/nofixedmods/serum.json:serum \
                     testing/recon-output/nofixedmods/b1906.json:b1906 \
        --metamorpheus testing/reference-data/metamorpheus/2026-08-21-10-29-48/Task3-SearchTask/AllPSMs.psmtsv \
        --tool-name "MetaMorpheus (reallyOpen, GPTMD-confirmed)" \
        --out-name recon_nofixedmods_vs_metamorpheus_reallyOpen.md

The --recon-json args are path:file_key pairs; file_key must match the per-file
column/filename mapping below.
"""

import argparse
import csv
import json
import os
import re
import statistics
import xml.etree.ElementTree as ET
from pathlib import Path
from scipy.stats import spearmanr

# --- Configuration ---------------------------------------------------------

# Match tolerance for calling two masses "the same mod".
# 0.015 Da (vs prior 0.01) adds ~5 mDa headroom for residual m/z-dependent drift
# after scalar apex_offset correction (serum pre-cal +2.5 ppm ≈ 2.5 mDa at 1000 Da).
MATCH_TOL_DA = 0.015

# Shared comparison window: true delta axis overlap of both tools' searches.
# Our Sage open config da:[-500,100] → delta window is −100..+500 (Lazear confirmed,
# 2026-08-17). PTM-Shepherd window is −150/+500. True overlap = −100..+500.
# Prior script had WINDOW_LO=-150, WINDOW_HI=100 — both wrong; the bug that silently
# dropped +100..+500 from the comparison. Verified fix: output tables MUST show peaks
# above +100 Da when both tools have them (the new spot-check below asserts this).
WINDOW_LO, WINDOW_HI = -100.0, 500.0

# Map our file_key -> PTM-Shepherd modsummary column stem (their shortened names).
# One report per file; never blended (NOTES locked).
PTMS_COLUMN_STEM = {
    "serum": "2019_4_9_909c_0311_1",
    "bcell": "Bnaive_01steady_state_1",
    "b1906": "b1906_1",
}

# Map our file_key -> Mascot error-tolerant summary filename stem (Ben's hand-copied
# per-file exports). One file per key; never blended (NOTES locked).
MASCOT_FILE_STEM = {
    "serum": "MascotErrorTol-909c.txt",
    "bcell": "MascotErrorTol-Bcell.txt",
    "b1906": "MascotErrorTol-b1906.txt",
}

# Map our file_key -> MetaMorpheus "File Name" column value (ALLPSMs.psmtsv pools
# all three files with a File Name column — confirmed 2026-08-19 via `tree /F` +
# ALLPSMs.psmtsv inspection; no per-file split needed, filter by this column instead).
METAMORPHEUS_FILE_STEM = {
    "serum": "2019-4-9_909c_0311-calib",
    "bcell": "B.naive_01steady-state-calib",
    "b1906": "b1906_293T_proteinID_01A_QE3_122212-calib",
}

# Matches one bracket tag in Full Sequence, e.g.:
#   [Common Fixed:Carbamidomethyl on C]
#   [Metal:Calcium on E]
#   [Common Artifact:Water Loss on E]
# Group 1 = category (before colon), Group 2 = mod name, Group 3 = site.
# Site matched up to the closing bracket via \w+/hyphen (covers residue letters
# and non-residue codes like "Nxs" sequon shorthand, confirmed 2026-08-19).
MOD_TAG_RE = re.compile(r'\[([^:\]]+):(.+?) on ([\w\-]+)\]')

# Rounding precision (decimal places) for binning the PER-PSM SUMMED mass
# of all Full Sequence tags into discrete mod-mass bins. 3 decimals distinguishes
# all real PTMs seen so far (e.g. 0.984 deamidation vs 0.992 isotope residue)
# while collapsing negligible floating-point summation noise.
METAMORPHEUS_MASS_BIN_DECIMALS = 3

# Monoisotopic atomic masses for the elements observed in MetaMorpheus's
# "Mods Combined Chemical Formula" column (2026-08-21 investigation). This
# REPLACES a Unimod-name-lookup approach that was tried and failed: MetaMorpheus's
# internal mod names ("Deamidation", "Acetylation", "Fe[III]") do not match
# Unimod's `title` strings ("Deamidated", "Acetyl") exactly, so name-based lookup
# silently failed to resolve the majority of real mods (e.g. bcell: 3753 of the
# file's mod-tag occurrences unresolved, falsely inflating the "Unmodified" bin
# to 82.8% when the true value is ~58-64%). Chemical-formula summation avoids
# name-matching entirely — it reads MetaMorpheus's own already-computed atomic
# composition per PSM, which is unambiguous and requires no external reference
# file. Table covers every element seen in this project's MetaMorpheus output;
# extend if a new element symbol appears (KeyError will name it explicitly).
ATOMIC_MONOISOTOPIC_MASS = {
    "H": 1.0078250319,
    "C": 12.0000000,
    "N": 14.0030740052,
    "O": 15.9949146221,
    "S": 31.97207069,
    "P": 30.97376151,
    "Na": 22.98976928,
    "K": 38.96370649,
    "Ca": 39.9625912,
    "Fe": 55.9349421,
    "Mg": 23.9850417,
    "Zn": 63.9291466,
    "Cu": 62.9295975,
    "Cl": 34.96885271,
    "Se": 79.9165196,
}

# Parses one element+optional-signed-integer token, e.g. "C2", "H-2", "Fe", "O-1".
# MetaMorpheus's combined-formula strings (e.g. "C2H3NO", "H-2Ca", "H-2O-1") have
# no delimiters between tokens, so this must match greedily left-to-right.
_FORMULA_TOKEN_RE = re.compile(r'([A-Z][a-z]?)(-?\d+)?')


def _formula_to_mass(formula):
    """Sum monoisotopic mass from a MetaMorpheus combined chemical formula string.

    Raises KeyError (with the unknown symbol named) if an element outside
    ATOMIC_MONOISOTOPIC_MASS is encountered — surfaced, not silently skipped,
    since a silently-dropped element would understate a PSM's true mod mass
    the same way the old Unimod-name approach silently understated it.
    """
    if not formula:
        return 0.0
    total = 0.0
    for symbol, count_str in _FORMULA_TOKEN_RE.findall(formula):
        if not symbol:
            continue
        count = int(count_str) if count_str else 1
        if symbol not in ATOMIC_MONOISOTOPIC_MASS:
            raise KeyError(
                f"Unknown element symbol '{symbol}' in formula {formula!r} — "
                f"add its monoisotopic mass to ATOMIC_MONOISOTOPIC_MASS."
            )
        total += ATOMIC_MONOISOTOPIC_MASS[symbol] * count
    return total


# --- Generic mod-table -----------------------------------------------------
# A tool's output adapts into: list of {mass, count, pct, label} per file.

def load_recon(json_path):
    """Adapter: Sage-Recon report JSON -> generic mod-table (single file).

    Accepts BOTH output shapes: the `analyze` unified report
    (peaks under report["mod_discovery"]["peaks"]) and the `discover`
    standalone JSON (peaks at the top level report["peaks"]). Same peak
    fields either way.
    """
    with open(json_path, encoding="utf-8") as fh:
        report = json.load(fh)
    if "mod_discovery" in report:
        peaks = report["mod_discovery"]["peaks"]
    else:
        peaks = report["peaks"]
    rows = []
    for peak in peaks:
        # Prefer the first annotation name; fall back to UNANNOTATED.
        anns = peak.get("annotations", [])
        label = anns[0]["name"] if anns else "UNANNOTATED"
        if peak.get("unannotated"):
            label = "UNANNOTATED"
        rows.append({
            "mass": peak["delta_mass"],
            "count": peak["count"],
            "pct": peak["count_pct"],
            "label": label,
        })
    return rows


def load_ptmshepherd(tsv_path, file_key):
    """Adapter: PTM-Shepherd global.modsummary.tsv -> generic mod-table (one file)."""
    stem = PTMS_COLUMN_STEM[file_key]
    psm_col = f"{stem}_PSMs"
    pct_col = f"{stem}_percent_PSMs"
    rows = []
    with open(tsv_path, encoding="utf-8", newline="") as fh:
        r = csv.DictReader(fh, delimiter="\t")
        if psm_col not in r.fieldnames or pct_col not in r.fieldnames:
            raise SystemExit(f"PTM-Shepherd TSV missing {psm_col}/{pct_col}")
        for row in r:
            try:
                mass = float(row["Mass Shift"])
                count = int(float(row[psm_col]))
                pct = float(row[pct_col])
            except (ValueError, KeyError):
                continue
            if count <= 0:
                continue
            rows.append({
                "mass": mass,
                "count": count,
                "pct": pct,
                "label": row["Modification"],
            })
    return rows


# --- Unimod name -> mass lookup (shared by Mascot and MetaMorpheus adapters) ---

def load_unimod_title_mass(unimod_xml):
    """title -> monoisotopic delta mass, parsed from unimod.xml (namespace-agnostic)."""
    title_to_mass = {}
    for _ev, el in ET.iterparse(unimod_xml, events=("end",)):
        if el.tag.split("}")[-1] == "mod":
            title = el.attrib.get("title")
            mono = None
            for child in el:
                if child.tag.split("}")[-1] == "delta":
                    mono = child.attrib.get("mono_mass")
                    break
            if title and mono is not None:
                title_to_mass[title] = float(mono)
            el.clear()
    return title_to_mass


# --- Mascot error-tolerant adapter -----------------------------------------
# Mascot reports mods by Unimod NAME + SITE (not mass), and splits ONE mod mass
# across many site rows (e.g. Carbamidomethyl on C / N-term / Y / D / E / H —
# the C row is the intended fix-mod, the rest are OVER-ALKYLATION off-site).
# Our tool and PTM-Shepherd report +57 as a SINGLE un-localized mass peak (the
# no-per-residue-localization lock). To compare like-for-like the adapter MUST:
#   1. resolve (name) -> monoisotopic mass via Unimod, and
#   2. ROLL UP all site rows sharing a mass into one row (sum ET counts),
# else Mascot's C-only row understates the true +57 population and the
# comparison is apples-to-oranges. The multi-site spread IS the over-alkylation
# signal; it is preserved in the label (sites listed), not discarded.
# Invariant asserted: rolled-up count == sum of contributing site-row ET.

def load_mascot(txt_path, unimod_title_mass):
    """Adapter: Mascot error-tolerant mod summary -> generic mod-table (one file).

    Columns: Modification, Site, Above thr., ET, Total matches. We count on
    `Total matches` (== ET here, Above thr. is 0 in error-tolerant mode).
    pct = row count / total resolved ET for the file (Mascot's OWN denominator,
    NOT PSM count — stated so no one forces PSM-count equality across tools).
    Returns (rows, skipped) where skipped is a list of (name, site, count) that
    had no clean Unimod mass (e.g. 'Non-specific cleavage', unresolved
    substitutions) — logged, never silently dropped.
    """
    # Aggregate ET across all site rows of the same resolved mass.
    by_mass = {}        # rounded-mass key -> {"mass","count","sites":{name->et}}
    skipped = []
    source_total = 0
    skipped_total = 0
    with open(txt_path, encoding="utf-8", newline="") as fh:
        r = csv.DictReader(fh, delimiter="\t")
        for row in r:
            name = row["Modification"].strip()
            site = row["Site"].strip()
            try:
                count = int(float(row["Total matches"]))
            except (ValueError, KeyError):
                continue
            if count <= 0:
                continue
            # Independent tally of everything read, before any roll-up touches it.
            # The conservation check below compares against THIS, not against the
            # accumulator it is checking.
            source_total += count
            mass = unimod_title_mass.get(name)
            if mass is None:
                skipped.append((name, site, count))
                skipped_total += count
                continue
            key = round(mass, 4)
            slot = by_mass.setdefault(key, {"mass": mass, "count": 0, "sites": {}})
            slot["count"] += count
            slot["sites"][f"{name}@{site}"] = slot["sites"].get(f"{name}@{site}", 0) + count

    # Conservation invariant: every ET count read from the file is either rolled up
    # or explicitly skipped for having no Unimod mass. Nothing may vanish or double.
    #
    # REWRITTEN 2026-08-25. The previous form asserted
    #     slot["count"] == sum(slot["sites"].values())
    # but both sides were incremented by the same `count` in the same iteration, so
    # it could never fail for any input. It was labelled a conservation invariant and
    # checked nothing. Same could-not-fail shape as the old Tier 3 gate. Verified by
    # reduction: 200 randomised trials produced 0 slots where it could fire.
    rolled_total = sum(s["count"] for s in by_mass.values())
    assert rolled_total + skipped_total == source_total, (
        f"Mascot roll-up conservation broke: read {source_total} ET counts, "
        f"rolled up {rolled_total}, skipped {skipped_total} "
        f"(difference {source_total - rolled_total - skipped_total})"
    )

    total_et = sum(s["count"] for s in by_mass.values())
    rows = []
    for slot in by_mass.values():
        # Label = dominant mod name + the site list (over-alkylation stays visible).
        site_items = sorted(slot["sites"].items(), key=lambda kv: -kv[1])
        dom_name = site_items[0][0].split("@")[0]
        sites = sorted({s.split("@")[1] for s in slot["sites"]})
        label = f"{dom_name} [{','.join(sites)}]"
        rows.append({
            "mass": slot["mass"],
            "count": slot["count"],
            "pct": 100.0 * slot["count"] / total_et if total_et else 0.0,
            "label": label,
        })
    return rows, skipped


# --- MetaMorpheus adapter ---------------------------------------------------
# Source: Task3-SearchTask/AllPSMs.psmtsv — the GPTMD-CONFIRMED search (not the
# GPTMD candidate-database task), i.e. PSM-backed matches at q<0.01, same tier
# as Mascot/PTM-Shepherd's confirmed output. All three files are pooled in one
# TSV with a "File Name" column — confirmed 2026-08-19, no per-file split needed.
#
# Population filter (evidence-based, 2026-08-19 investigation — see JOURNAL):
#   - Decoy/Contaminant/Target == "T"
#   - QValue < 0.01
#   - Full Sequence contains NO '|' pipe character.
# The pipe check replaces filtering on "Ambiguity Level" directly. Verified by
# direct measurement: Ambiguity Level 1 and 2D are 0% pipe-bearing (2D is
# protein-mapping-only — same peptide maps to multiple paralog genes, e.g.
# TUBA1A/TUBA1C — NOT mass/mod ambiguity); levels 2A/2B/2C/3/4/5 are 100%
# pipe-bearing (genuine multi-candidate mass/mod ambiguity, e.g. Calcium vs
# Potassium adduct on the same spectrum). Filtering on the pipe directly (rather
# than trusting the Ambiguity Level label's meaning) keeps ~97-98% of confident
# PSMs per file, vs ~90-92% if Level != "1" had been excluded wholesale.
#
# MASS SOURCE — v2 fix (2026-08-21): does NOT use "Mass Diff (Da)".
# That column measures leftover PSM-vs-theoretical-database mass error, which is
# NOT the same thing as "how much does this PSM's modification weigh" once GPTMD
# has baked candidate mods directly into the theoretical database. A PSM matching
# a GPTMD-expanded (e.g. Carbamidomethyl-modified) database entry shows a near-zero
# "Mass Diff (Da)" (ordinary calibration noise), NOT +57 — confirmed empirically
# 2026-08-21: v1 of this adapter (binning on Mass Diff (Da)) reported MetaMorpheus
# as having ~no Carbamidomethyl population, contradicting MetaMorpheus's own
# results.txt ("Localized mods seen below q-value 0.01: Carbamidomethyl on C 5448").
# Fix: reconstruct each PSM's true modification mass by summing the Unimod
# monoisotopic mass of every bracket tag found in Full Sequence (same
# name->mass lookup as the Mascot adapter, `load_unimod_title_mass`). This
# measures the same physical quantity recon/PTM-Shepherd/Mascot all measure —
# mass added by modification — regardless of whether the mod was baked into a
# search database (MetaMorpheus/GPTMD) or found as leftover precursor tolerance
# (Sage/PTM-Shepherd/Mascot error-tolerant).
#
# Label convention: PTM-Shepherd-style — one row per resolved mass bin, label is
# a plain descriptive string. For multi-mod PSMs (e.g. 2x Carbamidomethyl +
# Oxidation in one peptide) the label is the comma-joined list of all tag
# "Name on Site" strings from Full Sequence, sorted for stable output — the most
# information-preserving choice given recon/PTM-Shepherd's convention of one
# label per bin (not per mod), and Mascot's format doesn't cover the
# multi-simultaneous-mod case (its roll-up only merges same-mod site variants).
#
# Unresolved tags (a bracket name with no exact Unimod title match) are logged
# via `skipped_tags`, never silently dropped or zeroed — same discipline as the
# Mascot adapter's `skipped` list.
#
# Mass-error population: reported TWO ways (Phase 8.6 population-definition
# question) — "all confident targets" (the filtered population above) and
# "unmodified-only" (Full Sequence == Base Sequence, i.e. no tags at all) as a
# stricter apples-to-apples with recon's near-zero-delta clean-subset filter.
# NOTE: mass-error still legitimately uses "Mass Diff (ppm)" as-is — that IS the
# right column for mass accuracy (precursor mass error), a different question
# from "what does this PSM's modification weigh." Only the mod-identity/mass
# axis needed the Unimod-reconstruction fix.

def _median_mad(values):
    if not values:
        return None, None
    med = statistics.median(values)
    mad = statistics.median([abs(v - med) for v in values])
    return med, mad


def _parse_mod_tags(full_sequence):
    """Return list of (category, name, site) tuples from Full Sequence bracket tags.

    Used for LABELS ONLY (descriptive text) — mass comes from the chemical
    formula columns instead, via _formula_to_mass(). Kept as a separate
    concern deliberately, after the Unimod-name-lookup approach (v2) was
    found to silently under-resolve real mods due to naming mismatches.
    """
    return MOD_TAG_RE.findall(full_sequence)


# MASS SOURCE — v3 fix (2026-08-21): chemical-formula summation, not Unimod lookup.
# v1 used "Mass Diff (Da)" directly — WRONG, because GPTMD bakes candidate mods
# into the theoretical database itself, so a PSM matching a GPTMD-modified entry
# shows near-zero "Mass Diff (Da)" (ordinary calibration noise), not the mod's
# mass. v2 tried summing each Full Sequence tag's Unimod monoisotopic mass by
# NAME lookup — also WRONG: MetaMorpheus's internal mod names ("Deamidation",
# "Acetylation", "Fe[III]") do not match Unimod's `title` strings ("Deamidated",
# "Acetyl") exactly, so most real mods silently failed to resolve (bcell: 3753
# tag occurrences unresolved, falsely inflating "Unmodified" to 82.8% vs the
# true ~58-64%). v3 fix: read "Mods Combined Chemical Formula" directly (e.g.
# "C2H3NO" for Carbamidomethyl, "H-2Ca" for a Calcium adduct) and sum monoisotopic
# atomic masses via `_formula_to_mass()`. This is MetaMorpheus's OWN computed
# composition — no external name-matching, no ambiguity. Falls back to
# "Mods Chemical Formulas" if the combined column is empty (observed identical
# in every non-ambiguous row seen so far, per-mod formulas concatenated).
#
# Label convention: PTM-Shepherd-style — one row per resolved mass bin, label is
# a plain descriptive string, still built from Full Sequence's bracket tags
# (category:name on site) since those are just descriptive text, not a mass
# source — no change needed there. Comma-joined, sorted for multi-mod PSMs.

def load_metamorpheus(tsv_path, file_key):
    """Adapter: MetaMorpheus Task3 AllPSMs.psmtsv -> generic mod-table (one file).

    Returns (rows, mass_error_stats, n_excluded_ambiguous, n_total_candidate)
    where mass_error_stats is a dict:
        {
          "all_targets":   {"n": int, "median_ppm": float, "mad_ppm": float},
          "unmodified_only": {"n": int, "median_ppm": float, "mad_ppm": float},
        }
    Both computed on the SAME filtered population (Target, QValue<0.01,
    non-ambiguous) — "all_targets" is that whole population, "unmodified_only"
    is the subset of it with no mods at all (Full Sequence == Base Sequence).
    """
    file_name_wanted = METAMORPHEUS_FILE_STEM[file_key]

    n_total_candidate = 0     # rows matching this file, Target, QValue<0.01 (pre-pipe-filter)
    n_excluded_ambiguous = 0  # of those, how many had a pipe (excluded)

    # bin_key (rounded formula-summed mass) -> {"count": int, "mass_sum": float,
    #                                             "labels": {label_str: count}}
    bins = {}

    all_target_ppm = []
    unmodified_ppm = []

    with open(tsv_path, encoding="utf-8", newline="") as fh:
        r = csv.DictReader(fh, delimiter="\t")
        required = ["File Name", "Full Sequence", "Base Sequence",
                    "Mods Combined Chemical Formula", "Mods Chemical Formulas",
                    "Mass Diff (ppm)", "Decoy/Contaminant/Target", "QValue"]
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

            n_total_candidate += 1

            full_seq = row["Full Sequence"]
            if "|" in full_seq:
                n_excluded_ambiguous += 1
                continue

            try:
                mass_diff_ppm = float(row["Mass Diff (ppm)"])
            except ValueError:
                continue
            all_target_ppm.append(mass_diff_ppm)

            is_unmodified = (full_seq == row["Base Sequence"])
            if is_unmodified:
                unmodified_ppm.append(mass_diff_ppm)

            formula = row["Mods Combined Chemical Formula"].strip() or \
                      row["Mods Chemical Formulas"].strip()
            try:
                total_mass = _formula_to_mass(formula)
            except KeyError as e:
                raise SystemExit(str(e))

            tags = _parse_mod_tags(full_seq)
            if not tags:
                label = "Unmodified"
            else:
                label = ", ".join(sorted(f"{name} on {site}" for _cat, name, site in tags))

            key = round(total_mass, METAMORPHEUS_MASS_BIN_DECIMALS)
            slot = bins.setdefault(key, {"count": 0, "mass_sum": 0.0, "labels": {}})
            slot["count"] += 1
            slot["mass_sum"] += total_mass
            slot["labels"][label] = slot["labels"].get(label, 0) + 1

    n_kept = n_total_candidate - n_excluded_ambiguous
    rows = []
    for slot in bins.values():
        dom_label = max(slot["labels"].items(), key=lambda kv: kv[1])[0]
        rows.append({
            "mass": slot["mass_sum"] / slot["count"],
            "count": slot["count"],
            "pct": 100.0 * slot["count"] / n_kept if n_kept else 0.0,
            "label": dom_label,
        })

    all_med, all_mad = _median_mad(all_target_ppm)
    unmod_med, unmod_mad = _median_mad(unmodified_ppm)
    mass_error_stats = {
        "all_targets": {"n": len(all_target_ppm), "median_ppm": all_med, "mad_ppm": all_mad},
        "unmodified_only": {"n": len(unmodified_ppm), "median_ppm": unmod_med, "mad_ppm": unmod_mad},
    }

    return rows, mass_error_stats, n_excluded_ambiguous, n_total_candidate


# --- Alignment + metrics ---------------------------------------------------

def in_window(mass):
    return WINDOW_LO <= mass <= WINDOW_HI


def align(reference, other, tol=MATCH_TOL_DA):
    """Align two mod-tables by mass within tol, restricted to the shared window.

    Returns (matched, ref_only, other_only). Greedy nearest-mass match; each
    row used once. Both inputs are pre-filtered to the window so 'other_only'
    is a genuine disagreement, not an out-of-range artifact.
    """
    ref = [r for r in reference if in_window(r["mass"])]
    oth = [o for o in other if in_window(o["mass"])]
    ref_sorted = sorted(range(len(ref)), key=lambda i: -ref[i]["count"])
    used_other = set()
    matched, ref_only = [], []
    for i in ref_sorted:
        rmass = ref[i]["mass"]
        best_j, best_d = None, tol + 1
        for j, o in enumerate(oth):
            if j in used_other:
                continue
            d = abs(o["mass"] - rmass)
            if d <= tol and d < best_d:
                best_j, best_d = j, d
        if best_j is not None:
            used_other.add(best_j)
            matched.append((ref[i], oth[best_j], best_d))
        else:
            ref_only.append(ref[i])
    other_only = [oth[j] for j in range(len(oth)) if j not in used_other]
    return matched, ref_only, other_only


def rank_map(rows):
    """mass -> rank by descending count (1-based), for matched rows."""
    order = sorted(range(len(rows)), key=lambda i: -rows[i]["count"])
    return {id(rows[i]): pos + 1 for pos, i in enumerate(order)}


def spearman_and_topn(matched, n=10):
    """Spearman rank correlation + top-N overlap on the matched subset.

    Returns (rho, pval, topn_overlap_count, topn_ref_masses, topn_other_masses).
    rho/pval: Spearman on percent_PSMs of all matched rows (None if < 3 pairs).
    top-N: the N highest-% rows in each tool's matched set; overlap = masses in both.
    """
    if len(matched) < 2:
        return None, None, 0, [], []
    ref_pcts = [m[0]["pct"] for m in matched]
    oth_pcts = [m[1]["pct"] for m in matched]
    if len(matched) >= 3:
        rho, pval = spearmanr(ref_pcts, oth_pcts)
    else:
        rho, pval = None, None

    ref_top = sorted(matched, key=lambda m: -m[0]["pct"])[:n]
    oth_top = sorted(matched, key=lambda m: -m[1]["pct"])[:n]
    ref_top_masses = {round(m[0]["mass"], 3) for m in ref_top}
    oth_top_masses = {round(m[1]["mass"], 3) for m in oth_top}
    overlap = ref_top_masses & oth_top_masses
    return rho, pval, len(overlap), sorted(ref_top_masses), sorted(oth_top_masses)


def window_spot_check(recon_rows, other_rows):
    """Assert the window genuinely covers >+100 Da and return a summary line.

    This is the exact class of silent-cap bug we just fixed: if both tools have
    peaks above +100 but the comparison reports zero matched above +100, the fix
    didn't land. Raises AssertionError if both inputs have peaks above +100 but
    none appear in the window — i.e. the window is still capped.
    """
    recon_above = [r for r in recon_rows if r["mass"] > 100.0]
    other_above = [o for o in other_rows if o["mass"] > 100.0]
    in_window_above = [r for r in recon_rows if 100.0 < r["mass"] <= WINDOW_HI]
    # Both tools have >+100 peaks but our window shows none → still capped.
    if recon_above and other_above:
        assert in_window_above, (
            f"WINDOW BUG: both tools have peaks >+100 Da but none appear in window "
            f"[{WINDOW_LO}, {WINDOW_HI}] — WINDOW_HI may still be capped at 100."
        )
    return (f"Window coverage check: Sage-Recon peaks >+100 Da in window: "
            f"{len(in_window_above)} "
            f"(Sage total >+100: {len(recon_above)}, "
            f"other tool total >+100: {len(other_above)})")


def compare_file(file_key, recon_rows, other_rows, other_name):
    # --- spot-check before alignment -----------------------------------------
    spot_line = window_spot_check(recon_rows, other_rows)

    matched, ref_only, other_only = align(recon_rows, other_rows)
    # Ranks within the matched set (each tool ranked among the mods both found).
    ref_matched = [m[0] for m in matched]
    oth_matched = [m[1] for m in matched]
    ref_rank = rank_map(ref_matched)
    oth_rank = rank_map(oth_matched)

    rho, pval, topn_overlap, ref_top_m, oth_top_m = spearman_and_topn(matched)

    lines = []
    lines.append(f"### {file_key}  (Sage-Recon vs {other_name})")
    lines.append("")
    lines.append(f"- Matched (both, within {MATCH_TOL_DA} Da): **{len(matched)}**")
    lines.append(f"- Sage-Recon only: **{len(ref_only)}**")
    lines.append(f"- {other_name} only: **{len(other_only)}**")
    if rho is not None:
        lines.append(f"- Spearman ρ (% matched): **{rho:.3f}** (p={pval:.3g}, n={len(matched)})")
    lines.append(f"- Top-10 overlap (by % in matched set): **{topn_overlap}/10**")
    lines.append(f"- _{spot_line}_")
    lines.append("")
    lines.append("| mass (Da) | Sage-Recon label | Recon % | Recon rank | "
                 f"{other_name} label | {other_name} % | {other_name} rank |")
    lines.append("|---|---|---|---|---|---|---|")
    for ref, oth, _d in sorted(matched, key=lambda m: -m[0]["count"]):
        lines.append(
            f"| {ref['mass']:+.4f} | {ref['label']} | {ref['pct']:.2f} | "
            f"{ref_rank[id(ref)]} | {oth['label']} | {oth['pct']:.2f} | "
            f"{oth_rank[id(oth)]} |"
        )
    lines.append("")
    if ref_only:
        lines.append("**Sage-Recon only (in window, no match):**")
        for r in sorted(ref_only, key=lambda r: -r["count"])[:20]:
            lines.append(f"- {r['mass']:+.4f}  {r['label']}  ({r['pct']:.2f}%, {r['count']} PSMs)")
        lines.append("")
    if other_only:
        lines.append(f"**{other_name} only (in window, no match):**")
        for o in sorted(other_only, key=lambda o: -o["count"])[:20]:
            lines.append(f"- {o['mass']:+.4f}  {o['label']}  ({o['pct']:.2f}%, {o['count']} PSMs)")
        lines.append("")
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--recon-json", nargs="+", required=True,
                    help="path:file_key pairs (file_key in {serum,bcell,b1906})")
    ap.add_argument("--ptmshepherd",
                    help="path to a PTM-Shepherd global.modsummary.tsv (open or reallyOpen)")
    ap.add_argument("--mascot-dir",
                    help="dir holding MascotErrorTol-{909c,b1906,Bcell}.txt")
    ap.add_argument("--metamorpheus",
                    help="path to MetaMorpheus Task3-SearchTask/AllPSMs.psmtsv")
    ap.add_argument("--unimod", default="testing/reference-data/unimod.xml",
                    help="unimod.xml, for the Mascot/MetaMorpheus name->mass adapters")
    ap.add_argument("--out-dir", default="testing/recon-output/comparison")
    ap.add_argument("--out-name", default="recon_vs_ptmshepherd.md",
                    help="output markdown filename within out-dir")
    ap.add_argument("--tool-name", default="PTM-Shepherd",
                    help="display name for the compared tool (e.g. 'PTM-Shepherd (reallyOpen)')")
    args = ap.parse_args()

    if not args.ptmshepherd and not args.mascot_dir and not args.metamorpheus:
        raise SystemExit("give --ptmshepherd and/or --mascot-dir and/or --metamorpheus")

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    unimod_tm = None
    if args.mascot_dir:
        unimod_tm = load_unimod_title_mass(args.unimod)

    md = [f"# Mod-discovery cross-comparison: Sage-Recon vs {args.tool_name}",
          "",
          "OBJECTIVE tool-vs-tool benchmark (NOT a gate; NOT a comparison to the "
          "tool's author). Sage-Recon is the reference column. Divergence is "
          "expected — see the methodology deltas in "
          "`testing/reference-data/ptm-shepherd/README.md` (recalibrated two-stage "
          "search, per-file instruments, different FDR, wider window, higher peak "
          "floor, speed). Percentages are the currency; PSM totals differ by FDR.",
          f"",
          f"Match tolerance {MATCH_TOL_DA} Da; shared window [{WINDOW_LO}, {WINDOW_HI}] Da "
          f"(Sage delta range −100..+500, PTM-Shepherd −150..+500; true overlap −100..+500).",
          ""]

    for pair in args.recon_json:
        path, _, file_key = pair.partition(":")
        if not file_key:
            raise SystemExit(f"--recon-json arg needs path:file_key, got {pair!r}")
        recon_rows = load_recon(path)
        if args.ptmshepherd:
            other_rows = load_ptmshepherd(args.ptmshepherd, file_key)
            md.append(compare_file(file_key, recon_rows, other_rows, args.tool_name))
            md.append("")
        if args.mascot_dir:
            txt = Path(args.mascot_dir) / MASCOT_FILE_STEM[file_key]
            other_rows, skipped = load_mascot(txt, unimod_tm)
            skip_et = sum(c for _n, _s, c in skipped)
            print(f"[{file_key}] Mascot: {len(other_rows)} mass rows, "
                  f"{len(skipped)} rows unresolved to a Unimod mass "
                  f"({skip_et} ET matches skipped — incl. Non-specific cleavage / "
                  f"unresolved substitutions)")
            md.append(compare_file(file_key, recon_rows, other_rows, args.tool_name))
            md.append(f"_Mascot rows with no Unimod mass (skipped, not dropped silently): "
                      f"{len(skipped)} ({skip_et} ET matches)._")
            md.append("")
        if args.metamorpheus:
            other_rows, mass_err, n_excl, n_total = load_metamorpheus(args.metamorpheus, file_key)
            pct_excl = 100.0 * n_excl / n_total if n_total else 0.0
            print(f"[{file_key}] MetaMorpheus: {len(other_rows)} mass bins, "
                  f"{n_excl}/{n_total} PSMs excluded as ambiguous ({pct_excl:.2f}%, "
                  f"Full Sequence contained '|')")
            all_t = mass_err["all_targets"]
            unmod = mass_err["unmodified_only"]
            print(f"[{file_key}] MetaMorpheus mass error — all confident targets: "
                  f"n={all_t['n']}, median={all_t['median_ppm']:.3f} ppm, "
                  f"MAD={all_t['mad_ppm']:.3f}  |  unmodified-only: n={unmod['n']}, "
                  f"median={unmod['median_ppm']:.3f} ppm, MAD={unmod['mad_ppm']:.3f}")
            md.append(compare_file(file_key, recon_rows, other_rows, args.tool_name))
            md.append(f"_MetaMorpheus PSMs excluded as ambiguous (Full Sequence had '|', "
                      f"not dropped silently): {n_excl}/{n_total} ({pct_excl:.2f}%)._")
            md.append("")
            md.append(f"_MetaMorpheus mass error (Mass Diff ppm), Phase 8.6 population check — "
                      f"all confident targets: n={all_t['n']}, median={all_t['median_ppm']:.3f} ppm, "
                      f"MAD={all_t['mad_ppm']:.3f}; unmodified-only: n={unmod['n']}, "
                      f"median={unmod['median_ppm']:.3f} ppm, MAD={unmod['mad_ppm']:.3f}._")
            md.append("")

    out_md = out_dir / args.out_name
    out_md.write_text("\n".join(md), encoding="utf-8")
    print(f"Wrote {out_md}")


if __name__ == "__main__":
    main()
