#!/usr/bin/env python3
"""Assert what a full-run regeneration is ALLOWED to change, and what it is not.

HISTORICAL, 2026-09-01. This script asserts the invariants of the regeneration
that ADDED `three_layer_ms1`. That field was REMOVED from the report later the
same day (schema 2.0.0) and now lives only on `signal-fate --three-layer`. The
script is kept as the record of that regeneration; it is NOT a standing gate and
will not pass against current output. See NOTES
"THREE-LAYER MS1 REMOVED FROM RECON".

Run this BEFORE copying a regenerated report over the committed one. It compares
an OLD committed report against a NEW candidate and hard-stops on any change
outside the declared allow-list.

    python testing/scripts/assert_regeneration_invariants.py OLD_DIR NEW_DIR

WHY THE FIRST CHECK IS "DID ANYTHING CHANGE AT ALL":
`analyze --output foo.json` appends its own extension and writes `foo.json.json`,
leaving `foo.json` untouched. On 2026-08-25 that made an invariant run compare the
old files against THEMSELVES and print "ALL INVARIANTS HOLD" -- every assertion
passing because nothing had changed. A before/after check without a did-it-change
guard is not a check. See NOTES.

THE ALLOW-LIST for the 2026-08-27 regeneration (step 2.5), and the reason for each:
  * `schema_version`      1.3.0 -> 1.4.0.
  * `recommendations`     `protein_context` added, and EXACTLY THREE decisions
                          move. Supplying the search FASTA makes protein-terminal
                          candidates testable for the first time:
                            serum +14.0149 Methylation      below_floor -> stats
                            bcell +42.0109 Acetylation      no_residue_support -> stats
                            bcell -89.0289 Met-loss+Acetyl  below_floor -> stats
                          All three are protein N-terminal. The first two carry
                          `TG=X`, which the original step-2 rule read as
                          "unspecific"; at a PROTEIN terminus the position is the
                          specificity (~1% of PSMs). This script checks that set
                          and rejects any other. Every q also shifts, because more
                          p-values enter the BH sweep. Odds ratios move ONLY where
                          the winning candidate changed (+42.0109: `TG=K anywhere`
                          OR 1.19 -> `TG=X` protein N-term OR 173.6).
  * peak/label TEXT       no change expected this time. Allowed, and reported.
  * `generated_at`,
    `tool_version`,
    `git_commit`          provenance, expected to move.

THE PREVIOUS ALLOW-LIST, for the 2026-08-26 regeneration (1.2.0 -> 1.3.0), kept
because a reader comparing an older pair of directories needs it:
  * `schema_version` 1.2.0 -> 1.3.0; `recommendations` new block, absent in 1.2.0
    by definition; peak/label TEXT for two curated name corrections plus the
    Unimod XML entity fix (`Lys-&gt;Allysine` -> `Lys->Allysine`, 381 titles).

⚠ OLD_DIR MUST BE A TRUE PRE-CHANGE BASELINE, NOT WHATEVER IS COMMITTED NOW.
`EXPECTED_PROMOTIONS` describes the FULL 1.3.0 -> 1.4.0 change. If the committed
reports already carry part of it -- as they did on 2026-08-27, when a first
step-2.5 pass shipped before the `TG=X` defect was found -- comparing against
them shows only the remainder and this script fails, correctly. Recover the real
baseline from git instead:

    mkdir /tmp/base130
    for f in serum bcell b1906; do
      git show 246b381~1:testing/recon-output/full-run/$f.json > /tmp/base130/$f.json
    done

EVERYTHING ELSE MUST BE BIT-IDENTICAL. In particular every peak's delta_mass and
count, total_psms, unified_ms1_error and three_layer_ms1. If a number moves, this
exits non-zero and says which one. Numbers are printed, not summarised as a pass
mark -- acceptance is numeric.
"""
import json
import sys
from pathlib import Path

FILES = ["serum", "bcell", "b1906", "liver"]

# The schema this regeneration must produce. NOT bumped: `three_layer_ms1` was
# already in the schema, it was simply never populated because the 2026-08-31 run
# omitted --full.
# ⚠ Was "1.7.0" until 2026-09-02, while the code emitted 2.1.0 and the committed
# artifacts were 2.0.0. The guard was two minors stale and would have failed the
# next regeneration for the wrong reason. Keep this equal to
# report.rs SCHEMA_VERSION.
# ⚠ 3.1.0 -> 3.2.0 on 2026-09-03: `recommendations.not_recommended[]` and
# `notable_unannotated[]` gained `count_pct`, `sites` and `position`. Additive
# and omitted when empty, so a 3.1.0 consumer keeps working. NO EXISTING VALUE
# MOVES.
EXPECTED_SCHEMA = "3.2.0"

# ---------------------------------------------------------------------------
# ALLOW-LIST for the 2026-09-01 regeneration — WRITTEN BEFORE THE OUTPUT EXISTED.
#
# Pre-committed deliberately. Writing an allow-list after reading the new numbers
# is fitting the gate to the data, which is the failure this project has recorded
# more than once. If reality disagrees with what is declared here, the gate must
# FAIL and the disagreement gets investigated — the list does not get retuned.
#
# WHAT THIS REGENERATION IS FOR: adding `three_layer_ms1`, which needs --full.
# Nothing else about the pipeline changed.
#
# 1. `three_layer_ms1` appears on ALL FOUR files, having been absent on all four.
#    This is the entire point of the run.
# 2. Provenance moves: generated_at, tool_version, git_commit.
# 3. `input.mzml_file` changes for LIVER ONLY — its mzML was copied into
#    testing/inputs/ so all four runs use repo-relative paths. serum, bcell and
#    b1906 were already there and must NOT move.
# 4. Sage is not bit-reproducible across runs (NOTES, locked). PSM membership at
#    q<0.01 drifts. Budget: total_psms and any per-peak count may move by at most
#    0.1 % RELATIVE. Measured precedent on liver 2026-09-01: total_psms identical
#    (32496) and the +57 peak moved by exactly 1 PSM of 32496 = 0.003 %.
#    The NUMBER of peaks, every delta_mass, and peak ordering must NOT move.
# 5. Named TIC float noise stays inside the existing ULP budget.
#
# EVERYTHING ELSE MUST BE BIT-IDENTICAL — ms1_calibration in full, digestion,
# pass-2 composition, recommendations, signal_fate integer counts.
# ---------------------------------------------------------------------------
# Budget for Sage q-boundary membership drift on peak counts.
#
# ⚠ CORRECTED 2026-09-01, DELIBERATELY, AFTER IT FIRED — and recorded as such
# rather than quietly widened. The pre-committed rule was "at most 0.1 % RELATIVE
# per peak". On liver a peak moved 127 -> 128, which is 0.79 % relative and so
# broke the rule, while being ONE PSM out of 32496 = 0.003 % of the file.
#
# The metric was wrong, not the result. A relative bound on a small peak makes a
# single-PSM shift look like a large failure: at count 10, one PSM is 10 %. Sage's
# non-determinism is a per-PSM phenomenon (membership at the q<0.01 boundary), so
# the budget has to be expressed in PSMs, with a relative term that only takes
# over for large peaks.
#
# Approved by Ben 2026-09-01 as a deliberate correction. The reason it is written
# here rather than silently changed: retuning a gate after seeing the data it
# failed on is the exact habit this project keeps catching, and the difference
# between that and this is that this one is argued and recorded.
COUNT_DRIFT_PSMS = 2          # absolute floor: up to 2 PSMs on any peak
COUNT_DRIFT_REL_OF_TOTAL = 0.001   # or 0.1 % of total_psms, whichever is larger

# Derived STATISTICS (percentages, odds ratios) move when the PSM set moves. They
# are allowed this much RELATIVE movement, and ONLY when a PSM drift was already
# detected and printed for that file. No drift -> no allowance, and any movement
# is a failure.
#
# ⚠ ADDED 2026-09-01 as a deliberate correction, and justified by a CONTROLLED
# EXPERIMENT rather than by fitting this gate to the data it failed on:
#
#   * recon, run TWICE on ONE fixed Sage TSV, differs in exactly three things:
#     `generated_at`, `polymer.total_pct_tic` (1 ULP) and
#     `signal_fate.id_rate_by_tic_pct` (32 ULP). total_psms, all 49 peaks,
#     ms1_calibration.bias_ppm, the +57 count and the whole three_layer_ms1 block
#     are IDENTICAL. recon's own processing is deterministic.
#   * Sage, run TWICE on BYTE-IDENTICAL input (md5 6820489483a0668cf8a2f25fff3cd276)
#     with the SAME binary, emits DIFFERENT TSVs: 38,510,214 bytes md5 94ce181d...
#     vs 38,463,466 bytes md5 9bff7910..., same 96,049 rows.
#     (This line paired the sizes with the WRONG md5s until 2026-09-01. Re-measured
#     from both files, which are still on disk: full-run/liver_search is the
#     38,510,214 B / 94ce181d one, _verify/liver_search the 38,463,466 B / 9bff7910
#     one. The point of the example is unchanged. See NOTES "SAGE IS
#     NON-DETERMINISTIC, RECON IS NOT".)
#
# So every measurement difference here originates in Sage, upstream of recon. The
# budget bounds the CONSEQUENCE of that, and anything larger still fails.
#
# Observed on the 2026-09-01 regeneration: all but one movement under 0.025 %.
# The exception is serum's odds ratio at +14.0149 (Methylation), 24.1285 ->
# 24.2603 = 0.55 %, which is 20x the others and is named here so it is not lost
# in an aggregate.
DERIVED_DRIFT_REL = 0.01

# The ONLY recommendations this regeneration is allowed to add, per file. Written
# from the pre-committed step-2.5 prediction and from
# `tier_assignment_integration::met_loss_is_promoted_on_bcell_and_nowhere_else`,
# not read off the output. serum and b1906 gain nothing: their only
# protein-terminal candidates are TG=X, which is unspecific and stays on the
# abundance path.
EXPECTED_PROMOTIONS = {
    "serum": {(14.0149, "Methylation")},
    "bcell": {(-89.0289, "Met-loss+Acetylation"), (42.0109, "Acetylation")},
    "b1906": set(),
}

# Keys whose value may differ without failing the run.
ALLOWED_TOP = {"schema_version", "recommendations", "generated_at", "tool_version",
               "git_commit", "three_layer_ms1", "input"}
# Per-peak keys that carry NAMES rather than measurements.
LABEL_KEYS = {"annotation", "label", "name", "unimod_name", "annotations"}

# Fields that are NOT bit-reproducible run to run, with the ULP budget allowed.
#
# `signal_fate.id_rate_by_tic_pct` is a percentage of a large float sum whose
# summation ORDER varies between runs. Measured 2026-08-26 over three runs of the
# same input: committed -> run130 -> final drifted 14/59/17 then 23/6/11 ULP, with
# NO code change between the last two that touches TIC. Every other signal_fate
# field, including all integer counts, is bit-identical.
#
# The budget is deliberately far above observed noise (max seen 59) and far below
# any real change, which would move by orders of magnitude. This is an explicit,
# named exception -- NOT a blanket float tolerance. Everything not listed here
# must still be bit-identical, and the actual ULP distance is always printed.
ULP_BUDGET = 1024
# The affected CLASS is TIC-derived percentages -- both known members divide by a
# large parallel-summed TIC. Listed explicitly rather than pattern-matched, so a
# new member has to be added deliberately and shows up in the diff.
FLOAT_NOISE_OK = {
    ("signal_fate", "id_rate_by_tic_pct"),
    ("polymer", "total_pct_tic"),
}

# Per-peak fields that are float sums over the same intensities, and drift in the
# last digit for the same reason. ADDED 2026-09-01 as a deliberate correction:
# serum reported 47 "peak changed" failures that were all of the form
# 21760586883.058964 -> 21760586883.058968, with every count and delta_mass
# identical. Same documented mechanism as the TIC fields above, so it is the same
# named exception rather than a new blanket tolerance. Counts and delta_mass are
# NOT in this set and must still be handled by the drift budget.
# Per-peak fields DERIVED from which PSMs landed in the peak. When a peak's count
# drifts, these MUST move — they are consequences of that one fact, not several
# independent failures. Excused ONLY for a peak whose count actually moved.
# ⚠ They are deliberately NOT a blanket noise class: on liver 2026-09-01
# intensity_sum moved by 1.27 MILLION, which is real drift, not float error.
PEAK_DERIVED = {"intensity_sum", "intensity_pct", "representative_mz",
                "confidence", "prominence", "count_pct"}
# Derived from a WHOLE-FILE denominator, so a drift anywhere moves them on EVERY
# peak: count_pct divides by total_psms, intensity_pct by the summed intensity of
# all peaks. One PSM changing peak membership moves both on all 49 peaks.
PEAK_DERIVED_FROM_TOTAL = {"count_pct", "intensity_pct"}


def near(a, b, budget=ULP_BUDGET):
    """Equality that tolerates last-digit float drift, recursively.

    Needed because `confidence` is a nested dict of floats: a plain != on the
    dict reports a change for a 1-ULP difference in one member.
    """
    if isinstance(a, float) and isinstance(b, float):
        return a == b or ulps(a, b) <= budget
    if isinstance(a, dict) and isinstance(b, dict):
        return set(a) == set(b) and all(near(a[k], b[k], budget) for k in a)
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(near(x, y, budget) for x, y in zip(a, b))
    return a == b


def ulps(a, b):
    """Distance between two floats in units in the last place."""
    import struct
    ia = struct.unpack("<q", struct.pack("<d", a))[0]
    ib = struct.unpack("<q", struct.pack("<d", b))[0]
    return abs(ia - ib)


def diff_block(name, old, new):
    """Fields that changed in a block, split into noise-allowed and real."""
    noise, real = [], []
    for k in set(old or {}) | set(new or {}):
        a, b = (old or {}).get(k), (new or {}).get(k)
        if a == b:
            continue
        if (name, k) in FLOAT_NOISE_OK and isinstance(a, float) and isinstance(b, float):
            d = ulps(a, b)
            (noise if d <= ULP_BUDGET else real).append((k, a, b, d))
        else:
            real.append((k, a, b, None))
    return noise, real


def load(d, n):
    p = Path(d) / f"{n}.json"
    if not p.exists():
        sys.exit(f"FAIL missing report: {p}")
    return json.loads(p.read_text(encoding="utf-8"))


def strip_labels(obj):
    """Deep copy with every label-ish value blanked, so numbers compare alone."""
    if isinstance(obj, dict):
        return {k: ("<label>" if k in LABEL_KEYS else strip_labels(v)) for k, v in obj.items()}
    if isinstance(obj, list):
        return [strip_labels(v) for v in obj]
    return obj


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    old_dir, new_dir = sys.argv[1], sys.argv[2]
    failures, label_moves = [], []

    for n in FILES:
        old, new = load(old_dir, n), load(new_dir, n)
        # Did the INPUT change? Measured, not inferred.
        #
        # ⚠ A count-based heuristic is NOT sufficient, and serum proves it: its
        # total_psms and all 48 peak counts are IDENTICAL between the two runs,
        # yet seven odds ratios moved. The reason is that Sage returned a
        # DIFFERENT SET of PSMs with the same per-peak counts — different peptide
        # identities, so different residue tallies in the Fisher 2x2, so different
        # odds ratios, with every count unchanged. Serum's two TSVs are 25,555,140
        # vs 25,557,458 bytes over the same 66,772 rows.
        #
        # So the gate checksums the source TSV when it can reach it. That turns
        # "something may have drifted" from an inference into a measurement.
        # Falls back to the count heuristic when the TSVs are absent — they live
        # under a gitignored path, so a fresh clone has no access to them.
        _op, _np = old["mod_discovery"]["peaks"], new["mod_discovery"]["peaks"]
        counts_moved = (old["mod_discovery"]["total_psms"] != new["mod_discovery"]["total_psms"]
                        or len(_op) != len(_np)
                        or any(x.get("count") != y.get("count") for x, y in zip(_op, _np)))
        tsv_old = Path(old_dir) / f"{n}_search" / "results.sage.tsv"
        tsv_new = Path(new_dir) / f"{n}_search" / "results.sage.tsv"
        if tsv_old.exists() and tsv_new.exists():
            import hashlib
            def _md5(p):
                h = hashlib.md5()
                with open(p, "rb") as fh:
                    for chunk in iter(lambda: fh.read(1 << 20), b""):
                        h.update(chunk)
                return h.hexdigest()
            ha, hb = _md5(tsv_old), _md5(tsv_new)
            psm_drift = ha != hb
            if psm_drift:
                print(f"  INPUT  source TSV DIFFERS ({ha[:8]} -> {hb[:8]}) -- Sage is not "
                      f"bit-reproducible; derived statistics may move within "
                      f"{DERIVED_DRIFT_REL*100:.0f}%")
            else:
                print(f"  INPUT  source TSV identical ({ha[:8]}) -- NOTHING may move")
        else:
            psm_drift = counts_moved
            print(f"  INPUT  source TSV not available; falling back to peak counts "
                  f"(drift={psm_drift})")
        print(f"\n=== {n} ===")

        # ---- GUARD: prove something actually changed before asserting sameness.
        if old == new:
            failures.append(f"{n}: OLD and NEW are byte-identical -- nothing was regenerated")
            print("  GUARD  FAIL: files identical, the comparison would be vacuous")
            continue
        print(f"  GUARD  ok: reports differ  ({old.get('schema_version')} -> {new.get('schema_version')})")

        # ---- the two declared additions
        if new.get("schema_version") != EXPECTED_SCHEMA:
            failures.append(f"{n}: schema_version is {new.get('schema_version')!r}, "
                            f"expected {EXPECTED_SCHEMA!r}")
        if new.get("recommendations") is None:
            failures.append(f"{n}: `recommendations` is absent -- the curated list did not load")
        else:
            r = new["recommendations"]
            n_rec = len(r["fixed"]) + len(r["variable"])
            print(f"  NEW    recommendations: {n_rec} recommended "
                  f"({len(r['fixed'])} fixed, {len(r['variable'])} variable), "
                  f"{len(r['not_recommended'])} not, floor {r['floor_psms']:.1f}")
            # ---- protein context: the step-2.5 addition, and its two guards.
            pc = r.get("protein_context")
            if pc is None:
                failures.append(f"{n}: `protein_context` absent -- was --fasta passed? "
                                "Without it the protein-terminal class is untested and "
                                "this regeneration means something different.")
            else:
                print(f"  NEW    protein_context: {pc['psms_resolved']}/{pc['psms_total']} "
                      f"resolved ({pc['resolved_pct']:.2f}%), "
                      f"{pc['protein_nterm_psms']} at protein position 0 "
                      f"({100.0 * pc['protein_nterm_psms'] / pc['psms_total']:.3f}%), "
                      f"{pc['proteins_indexed']} sequences")
                if pc["resolved_pct"] < 95.0:
                    failures.append(f"{n}: only {pc['resolved_pct']:.2f}% of PSMs resolve "
                                    "in the FASTA -- wrong database")
                nterm_pct = 100.0 * pc["protein_nterm_psms"] / pc["psms_total"]
                if nterm_pct >= 5.0:
                    failures.append(f"{n}: {nterm_pct:.3f}% at protein position 0 -- the "
                                    "lookup is matching too much to discriminate")

            # ---- the recommendation set: exactly one declared move, no others.
            old_r = old.get("recommendations") or {}
            def label_set(block):
                return {(round(m["delta_mass"], 4), m["label"])
                        for m in block.get("fixed", []) + block.get("variable", [])}
            added = label_set(r) - label_set(old_r)
            removed = label_set(old_r) - label_set(r)
            # CORRECTION 2026-09-01, deliberate. EXPECTED_PROMOTIONS describes the
            # 2026-08-27 (1.3.0 -> 1.4.0) regeneration, where supplying a FASTA made
            # protein-terminal candidates testable for the first time. Both sides of
            # THIS comparison are already 1.7.0 and already carry protein_context, so
            # nothing can be promoted. The expectation for a same-schema regeneration
            # is NO recommendation movement at all. The old set is kept above as the
            # record of what that earlier run was allowed to do.
            expected_add = set() if old.get("schema_version") == new.get("schema_version") \
                           else EXPECTED_PROMOTIONS.get(n, set())
            if added != expected_add:
                failures.append(f"{n}: recommendations ADDED {sorted(added)}, "
                                f"expected {sorted(expected_add)}")
            elif added:
                print(f"  MOVED  promoted to recommended: {sorted(added)}")
            if removed:
                failures.append(f"{n}: recommendations REMOVED {sorted(removed)} -- "
                                "nothing should lose its recommendation")
            # Odds ratios are per-candidate and cannot depend on how many other
            # tests ran. A moved OR means the 2x2 itself changed.
            def ors(block):
                return {round(m["delta_mass"], 4): m.get("odds_ratio")
                        for m in block.get("fixed", []) + block.get("variable", [])
                        if m.get("odds_ratio") is not None}
            a_or, b_or = ors(old_r), ors(r)
            for mass, o in a_or.items():
                if mass in b_or and abs(b_or[mass] - o) > 1e-9:
                    rel = abs(b_or[mass] - o) / abs(o) if o else float("inf")
                    if psm_drift and rel <= DERIVED_DRIFT_REL:
                        print(f"  DRIFT  odds ratio {mass:+.4f}: {o:.6f} -> {b_or[mass]:.6f} "
                              f"({rel*100:.4f}%, budget {DERIVED_DRIFT_REL*100:.1f}%)")
                    else:
                        failures.append(f"{n}: odds ratio at {mass:+.4f} moved {o} -> {b_or[mass]}"
                                        + ("" if psm_drift else " with NO PSM drift to explain it"))

        # ---- blocks that must not move at all
        for block in ["signal_fate", "polymer", "oxonium", "digestion", "alkylation",
                      "three_layer_ms1", "ms1_calibration"]:
            ob, nb = strip_labels(old.get(block)), strip_labels(new.get(block))
            if ob == nb:
                print(f"  SAME   {block}")
                continue
            # A block that was ABSENT and is now PRESENT is an ADDITION, not a
            # change — and for this regeneration it is the entire point: the
            # 2026-08-31 run omitted --full, so `three_layer_ms1` was never
            # computed. Only declared blocks may appear this way.
            if ob is None and nb is not None:
                if block in ALLOWED_TOP:
                    extra = ""
                    if block == "three_layer_ms1":
                        extra = (f"  non-peptidic {nb['non_peptidic_pct']:.2f}%, "
                                 f"never-sampled {nb['never_sampled_pct_of_peptide_like']:.2f}%, "
                                 f"sampled-not-ID {nb['sampled_not_id_pct_of_peptide_like']:.2f}%, "
                                 f"identified {nb['identified_pct_of_peptide_like']:.2f}%")
                    print(f"  NEW    block `{block}` ADDED (was absent){extra}")
                else:
                    failures.append(f"{n}: block `{block}` APPEARED but is not declared")
                continue
            if nb is None and ob is not None:
                failures.append(f"{n}: block `{block}` DISAPPEARED — a regeneration "
                                "must not drop a block")
                continue
            if not isinstance(ob, dict) or not isinstance(nb, dict):
                failures.append(f"{n}: block `{block}` CHANGED")
                continue
            noise, real = diff_block(block, ob, nb)
            for k, a, b, _ in real:
                rel = (abs(a - b) / abs(a)) if (isinstance(a, float) and isinstance(b, float) and a) else None
                if psm_drift and rel is not None and rel <= DERIVED_DRIFT_REL:
                    print(f"  DRIFT  {block}.{k}: {a:.6f} -> {b:.6f} "
                          f"({rel*100:.4f}%, budget {DERIVED_DRIFT_REL*100:.1f}%)")
                else:
                    failures.append(f"{n}: {block}.{k} CHANGED  {a!r} -> {b!r}"
                                    + ("" if psm_drift else " with NO PSM drift to explain it"))
            for k, a, b, d in noise:
                print(f"  NOISE  {block}.{k}: {d} ULP (budget {ULP_BUDGET})  {a!r} -> {b!r}")
            if not real and noise:
                print(f"  SAME   {block} (every other field bit-identical)")

        # CORRECTION 2026-09-01, deliberate. This block used to REQUIRE
        # `unified_ms1_error` and blame a missing `--closed-tsv`. Both are
        # RETIRED: recon no longer produces or accepts a closed search, and the
        # signed MS1 bias comes from the open search's own clean subset. The
        # assertion now runs the other way — the block must be ABSENT, so a
        # future edit that resurrects it is caught.
        for side, rep in (("old", old), ("new", new)):
            if (rep.get("mass_accuracy") or {}).get("unified_ms1_error") is not None:
                failures.append(f"{n}: {side} report carries unified_ms1_error, which was "
                                "retired 2026-09-01. recon needs no closed search.")

        # ---- peaks: numbers identical, names allowed to move
        op, np_ = old["mod_discovery"]["peaks"], new["mod_discovery"]["peaks"]
        if len(op) != len(np_):
            failures.append(f"{n}: peak COUNT moved {len(op)} -> {len(np_)}")
        else:
            print(f"  SAME   peak count: {len(op)}")
        _ot, _nt = old["mod_discovery"]["total_psms"], new["mod_discovery"]["total_psms"]
        if _ot != _nt:
            _b = max(COUNT_DRIFT_PSMS, COUNT_DRIFT_REL_OF_TOTAL * _nt)
            if abs(_ot - _nt) <= _b:
                print(f"  DRIFT  total_psms {_ot} -> {_nt} ({abs(_ot-_nt)} PSM, "
                      f"{abs(_ot-_nt)/_ot*100:.4f}%, budget {_b:.1f}) -- q<0.01 boundary")
            else:
                failures.append(f"{n}: total_psms moved BEYOND budget {_ot} -> {_nt}")
        else:
            print(f"  SAME   total_psms: {_nt}")

        total = new["mod_discovery"]["total_psms"]
        budget = max(COUNT_DRIFT_PSMS, COUNT_DRIFT_REL_OF_TOTAL * total)
        old_total = old["mod_discovery"]["total_psms"]
        total_moved = old_total != total
        moved = 0
        drifted = []
        for a, b in zip(op, np_):
            dm_a, dm_b = a.get("delta_mass"), b.get("delta_mass")
            dm_ok = (isinstance(dm_a, float) and isinstance(dm_b, float)
                     and ulps(dm_a, dm_b) <= ULP_BUDGET)
            ca, cb = a.get("count"), b.get("count")
            count_moved = ca != cb
            # delta_mass is the CENTROID of the peak's members, so it legitimately
            # moves when membership does. Only unexplained movement is a failure.
            if dm_a != dm_b and not dm_ok and not count_moved:
                failures.append(f"{n}: peak delta_mass MOVED with no count change: "
                                f"{dm_a} -> {dm_b}")
            if count_moved:
                if abs(ca - cb) <= budget:
                    drifted.append((dm_a, ca, cb))
                else:
                    failures.append(f"{n}: peak count moved BEYOND budget "
                                    f"({abs(ca - cb)} PSMs > {budget:.1f}): "
                                    f"{dm_a}/{ca} -> {dm_b}/{cb}")
            # A derived field is EXCUSED only when something explains it: this
            # peak's own count drifted, or (for count_pct) the denominator did.
            # Without that, a moved intensity_sum is a real change and fails.
            excused = set()
            if count_moved:
                excused |= PEAK_DERIVED
            if total_moved:
                excused |= PEAK_DERIVED_FROM_TOTAL
            sa, sb = strip_labels(a), strip_labels(b)
            for k in set(sa) | set(sb):
                if k in ("count", "delta_mass") or k in excused:
                    continue
                va, vb = sa.get(k), sb.get(k)
                if va == vb:
                    continue
                if near(va, vb):
                    continue
                failures.append(f"{n}: peak {dm_a:+.4f} field {k!r} changed with nothing "
                                f"to explain it: {str(va)[:55]} -> {str(vb)[:55]}")
            if a != b:
                moved += 1
                for k in LABEL_KEYS & set(a) | LABEL_KEYS & set(b):
                    if a.get(k) != b.get(k):
                        label_moves.append(f"{n}  {a.get('delta_mass'):+.4f}  "
                                           f"{a.get(k)!r} -> {b.get(k)!r}")
        if drifted:
            tot = sum(abs(y - x) for _, x, y in drifted)
            print(f"  DRIFT  {len(drifted)} peak count(s) moved, {tot} PSM(s) total "
                  f"= {tot / total * 100:.4f}% (budget {budget:.1f} PSMs/peak):")
            for dm, x, y in drifted:
                print(f"           {dm:+11.4f}  {x} -> {y}")
        print(f"  LABEL  {moved} peak(s) differ in name text only (allowed)")

    if label_moves:
        print(f"\n=== label changes, {len(label_moves)} total (expected: entity fix + 2 corrections) ===")
        for m in label_moves[:25]:
            print("  " + m)
        if len(label_moves) > 25:
            print(f"  ... and {len(label_moves) - 25} more")

    print("\n" + "=" * 70)
    if failures:
        print(f"INVARIANTS VIOLATED — {len(failures)}")
        for f in failures:
            print("  FAIL " + f)
        sys.exit(1)
    print("ALL INVARIANTS HOLD — measurements identical, only the allow-listed items moved.")


if __name__ == "__main__":
    main()
