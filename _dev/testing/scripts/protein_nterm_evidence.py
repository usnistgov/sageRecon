#!/usr/bin/env python3
"""The evidence base for step 2.5 — Met-loss protein N-term. Re-derived, not quoted.

Every number NOTES states about the Met-loss population is produced here, from
the committed `step1-open-*/results.sage.tsv` plus the search FASTA. NOTES held
these as prose only; no script produced them. A summary is not a source, so this
script IS the source, and it hard-stops if it cannot reproduce the pinned counts.

WHAT IT MEASURES, per file:
  * confident target PSMs (rank 1, spectrum_q < 0.01, label 1)
  * PSMs whose peptide starts at PROTEIN position 0, with the Met RETAINED
  * PSMs whose peptide is the Met-EXCISED form (protein position 1, after an
    initiator Met). Must be 0: the pass-1 search is fully tryptic with no
    Met-clipping option, so a peptide starting at position 1 has a non-tryptic
    N-terminus and is not in the search space. This is WHY the delta-mass route
    is the only route.
  * the -89.0289 band, its protein-position-0 members, and the residue-2
    composition of those members
  * the NME-permissive fraction in the band, and the background TWO WAYS

TWO BACKGROUNDS, BOTH PRINTED, LABELLED. The band-EXCLUSIVE figure is the one
that matches how recon tests a candidate: `tier_assignment::test_candidate`
builds a 2x2 of band vs NOT-band, because counting the band inside its own
background understates the odds ratio. The band-INCLUSIVE figure is printed only
because NOTES quoted it as though it were the background. It is not.

THE RESIDUE-2 COMPOSITION IS A FINDING, NOT A RULE. No NME acceptor set is
derived from it. The tool makes no judgement about whether a given residue 2
makes Met excision plausible -- that stays the reader's call.

WHAT TG DOES CARRY on a Met-loss entry (since 2026-08-27): the acceptor of the
modification that FOLLOWS the removal, at the exposed residue. Read off the
pinned unimod.xml, not from enzymology -- Acetyl/Methyl/Succinyl at Protein
N-term are `site=N-term` (no residue, TG=X), while every N-terminal Myristoyl
record is `site=G` (TG=G). That is each modification's own chemistry, NOT the
NME rule. The initiator-Met requirement is carried by PP and enforced in code.
See NOTES "Met-loss encoding".

Usage:  python3 testing/scripts/protein_nterm_evidence.py [--fasta PATH]
"""
import argparse
import csv
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "testing/scripts"))
from residue_enrichment import C13_C12_DIFF, BAND_TOL_DA, Q_MAX, FILES

DEFAULT_FASTA = REPO / "testing/inputs/UniProt-Human-UP000005640_canonical-2023_05.fasta"
MET_LOSS_ACETYL = -89.0289          # bcell's measured peak centre
UNIMOD_766 = -89.02992              # Met-loss+Acetyl, for the mDa distance
NME_PERMISSIVE = set("ACGSTV")      # a FINDING to report against, never a filter

# TRIPWIRES. Recompute against a known count before trusting the derived set.
EXPECTED_CONFIDENT = {"serum": 12438, "bcell": 55419, "b1906": 22298}
EXPECTED_PROTEIN_NTERM = {"serum": 196, "bcell": 606, "b1906": 97}


def load_fasta(path):
    """accession -> sequence. Key is the header's first whitespace token, which
    is exactly what Sage writes into the `proteins` column."""
    seqs, acc, buf = {}, None, []
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            if line.startswith(">"):
                if acc:
                    seqs[acc] = "".join(buf)
                acc, buf = line[1:].strip().split()[0], []
            else:
                buf.append(line.strip())
    if acc:
        seqs[acc] = "".join(buf)
    return seqs


def load_rows(file_key):
    """Confident TARGET PSMs as (delta, peptide, proteins). Decoys excluded by
    `label`, the column Sage writes it in, not by an accession-prefix guess."""
    tsv = REPO / "testing/search-output" / FILES[file_key] / "results.sage.tsv"
    rows, decoys = [], 0
    with open(tsv, newline="", encoding="utf-8") as fh:
        for r in csv.DictReader(fh, delimiter="\t"):
            if r["rank"] != "1" or float(r["spectrum_q"]) >= Q_MAX:
                continue
            if r["label"] != "1":
                decoys += 1
                continue
            delta = (float(r["expmass"]) - float(r["calcmass"])
                     - float(r["isotope_error"]) * C13_C12_DIFF)
            rows.append((delta, r["peptide"], r["proteins"]))
    return rows, decoys


def protein_start(seqs, peptide, proteins):
    """The protein sequence this peptide starts at position 0 of, or None."""
    for acc in proteins.split(";"):
        s = seqs.get(acc)
        if s and s.startswith(peptide):
            return s
    return None


def met_excised(seqs, peptide, proteins):
    """True when the peptide is the Met-CLIPPED form: it sits at protein
    position 1 and position 0 is an initiator Met."""
    for acc in proteins.split(";"):
        s = seqs.get(acc)
        if s and s.startswith("M") and s[1:].startswith(peptide):
            return True
    return False


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--fasta", default=str(DEFAULT_FASTA))
    args = ap.parse_args()

    fasta = Path(args.fasta)
    if not fasta.exists():
        sys.exit(f"FAIL FASTA not found: {fasta}\n"
                 "     It is gitignored. Copy it into testing/inputs/ first.")
    seqs = load_fasta(fasta)
    print(f"FASTA {fasta.name}: {len(seqs)} sequences")
    print(f"Met-loss+Acetyl band centre {MET_LOSS_ACETYL:+.4f}, "
          f"Unimod 766 {UNIMOD_766:+.5f}, "
          f"{abs(MET_LOSS_ACETYL - UNIMOD_766) * 1000:.1f} mDa apart\n")

    failures = []
    for key in ("serum", "bcell", "b1906"):
        rows, decoys = load_rows(key)
        n = len(rows)

        # --- accession resolution. A wrong FASTA would read as "no protein
        # N-terminal modifications present", which is a confident wrong answer.
        resolved = sum(1 for _, _, pr in rows
                       if any(a in seqs for a in pr.split(";")))
        res_pct = 100.0 * resolved / n if n else 0.0

        nterm = excised = 0
        band_n = band_pos0 = 0
        in_band, out_band = Counter(), Counter()
        for delta, pep, pr in rows:
            is_band = abs(delta - MET_LOSS_ACETYL) <= BAND_TOL_DA
            if is_band:
                band_n += 1
            s = protein_start(seqs, pep, pr)
            if s is None:
                if met_excised(seqs, pep, pr):
                    excised += 1
                continue
            nterm += 1
            r2 = s[1] if len(s) > 1 else "?"
            if is_band:
                band_pos0 += 1
                in_band[r2] += 1
            else:
                out_band[r2] += 1

        ip = sum(v for k, v in in_band.items() if k in NME_PERMISSIVE)
        bp = sum(v for k, v in out_band.items() if k in NME_PERMISSIVE)
        nb, no = sum(in_band.values()), sum(out_band.values())

        print(f"=== {key}")
        print(f"  confident target PSMs        {n}   (decoys excluded: {decoys})")
        print(f"  accessions resolved in FASTA {resolved}/{n} = {res_pct:.2f}%")
        print(f"  protein position 0, Met RETAINED {nterm}  "
              f"({100.0 * nterm / n:.3f}% of confident)")
        print(f"  Met EXCISED (protein position 1) {excised}")
        print(f"  -89.03 band                  {band_n} PSMs, "
              f"{band_pos0} at protein position 0")
        if nb:
            print(f"  band residue 2               "
                  f"{', '.join(f'{k} {v}' for k, v in in_band.most_common())}")
            print(f"  NME-permissive IN BAND       {ip}/{nb} = {100.0 * ip / nb:.1f}%")
        print(f"  background, band-EXCLUSIVE   {bp}/{no} = "
              f"{100.0 * bp / no:.1f}%   <- the figure recon's 2x2 uses")
        print(f"  background, band-INCLUSIVE   {ip + bp}/{nb + no} = "
              f"{100.0 * (ip + bp) / (nb + no):.1f}%   <- NOT a background")

        # --- tripwires
        if n != EXPECTED_CONFIDENT[key]:
            failures.append(f"{key}: confident PSMs {n}, expected {EXPECTED_CONFIDENT[key]}")
        if nterm != EXPECTED_PROTEIN_NTERM[key]:
            failures.append(f"{key}: protein N-term {nterm}, "
                            f"expected {EXPECTED_PROTEIN_NTERM[key]}")
        if excised != 0:
            failures.append(f"{key}: {excised} Met-EXCISED PSMs, expected 0 — "
                            "the search space should not contain them")
        print()

    print("=" * 70)
    if failures:
        print(f"TRIPWIRE VIOLATED — {len(failures)}")
        for f in failures:
            print("  FAIL " + f)
        sys.exit(1)
    print("Counts reproduce the pinned values on all three files.")


if __name__ == "__main__":
    main()
