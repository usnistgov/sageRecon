#!/usr/bin/env python3
"""Liver digestion numbers across four sources, under ONE classification rule.

THE POINT OF THIS SCRIPT IS THAT NOBODY'S CLASS COLUMN IS READ. MSFragger,
PTM-Shepherd and MetaMorpheus all ship a terminus/class call of their own, and
they do not mean the same thing. Every side here is reclassified from raw
sequence context against the SAME FASTA with the SAME rule.

The rule is a mirror of the SHIPPED Rust, `digestion.rs::is_tryptic_nterm` /
`is_tryptic_cterm` as of 2026-09-01, INCLUDING the initiator-Met excision branch
added 2026-08-31. `preview_compare.py` predates that fix and is 21 peptides
adrift on this file; do not use it for cross-tool work.

Basis and denominators follow Byonic Preview, because Preview is the anchor:
  * DISTINCT PEPTIDES, not PSMs
  * missed cleavage / ragged-N / ragged-C -> tryptic + semitryptic (nontryptic
    EXCLUDED from the denominator)
  * nontryptic -> all peptides
  * missed cleavage = an internal K/R not followed by P

⚠ The five sides are NOT the same population and this script does not pretend
they are. Preview kept ProtScore>=20 representative proteins; recon Pass 2
searches its own >=2-peptide subset FASTA; the FragPipe runs searched the whole
database. Read DIRECTION and SCALE, not equality. NOTES "ACCEPTANCE CRITERION"
sets the bar: between-method spread must stay small against between-sample
spread (24.7 pp ragged-N, 15.9 pp missed cleavage).

⚠ The FOUR sources are recon, PTM-Shepherd, MetaMorpheus and Byonic Preview.
An MSFragger closed semi-tryptic run (`liverFragger`) was an INTERMEDIATE CHECK
from a previous session and is NOT part of this comparison — archived to
`_archive/liverFragger-2026-08-31/` on 2026-09-01. Do not re-add it.

⚠ liverShepherd is `num_enzyme_termini = 2` (FULLY tryptic) and MetaMorpheus is
fully specific too. A fully tryptic search CANNOT produce a ragged peptide, so
their ragged rates are a control near zero, not a measurement. Only recon and
Preview are comparable on ragged termini. Stated, not hidden.
"""
import csv, re, sys, collections, os

csv.field_size_limit(10 ** 9)
STRIP = re.compile(r"\[[^\]]*\]|\(.*?\)")


def read_fasta(path):
    seqs, acc, buf = {}, None, []
    for line in open(path):
        if line.startswith(">"):
            if acc:
                seqs[acc] = "".join(buf)
            h = line[1:].split()[0]
            parts = h.split("|")
            acc = parts[1] if len(parts) >= 2 else h
            buf = []
        else:
            buf.append(line.strip())
    if acc:
        seqs[acc] = "".join(buf)
    return seqs


def is_tryptic_nterm(pep, prot, start):
    """Mirror of digestion.rs::is_tryptic_nterm (current)."""
    if start == 0:
        return True
    if start == 1 and prot[:1] == "M":     # initiator-Met excision, NOT ragged-N
        return True
    if prot[start - 1] in "KR":
        return pep[:1] != "P"
    return False


def is_tryptic_cterm(pep, prot, end):
    """Mirror of digestion.rs::is_tryptic_cterm (current)."""
    if end >= len(prot):
        return True
    if pep[-1:] in ("K", "R"):
        return prot[end] != "P"
    return False


def classify(pep, prot):
    start = prot.find(pep)          # FIRST occurrence only, as the Rust does
    if start < 0:
        return "NOT_FOUND"
    end = start + len(pep)
    n = is_tryptic_nterm(pep, prot, start)
    c = is_tryptic_cterm(pep, prot, end)
    return {(True, True): "FULLY", (False, True): "RAGGED_N",
            (True, False): "RAGGED_C", (False, False): "NONTRYPTIC"}[(n, c)]


def internal_missed(pep):
    return sum(1 for k in range(len(pep) - 1) if pep[k] in "KR" and pep[k + 1] != "P")


def tally(pairs, seqs, label):
    """pairs: iterable of (peptide, accession). Returns a result dict."""
    peps = {}
    for pep, acc in pairs:
        peps.setdefault(pep, acc)
    cls = collections.Counter()
    mc = 0
    unresolved = 0
    for pep, acc in peps.items():
        prot = seqs.get(acc)
        if prot is None:
            unresolved += 1
            cls["UNRESOLVED"] += 1
            continue
        c = classify(pep, prot)
        if c == "NOT_FOUND":
            unresolved += 1
        cls[c] += 1
        if c in ("FULLY", "RAGGED_N", "RAGGED_C") and internal_missed(pep) >= 1:
            mc += 1
    den = cls["FULLY"] + cls["RAGGED_N"] + cls["RAGGED_C"]
    allp = den + cls["NONTRYPTIC"]
    return dict(label=label, n_pep=len(peps), den=den, allp=allp, mc=mc,
                rn=cls["RAGGED_N"], rc=cls["RAGGED_C"], nt=cls["NONTRYPTIC"],
                unresolved=unresolved, cls=cls)


def from_fragpipe(path):
    with open(path) as fh:
        for r in csv.DictReader(fh, delimiter="\t"):
            if (r.get("Is Decoy") or "").strip().lower() == "true":
                continue
            q = r.get("Qvalue")
            if q not in (None, "") and float(q) > 0.01:
                continue
            pep = STRIP.sub("", r["Peptide"]).strip().upper()
            acc = (r.get("Protein ID") or "").strip()
            if pep and acc:
                yield pep, acc


def from_metamorpheus(path, q=0.01):
    with open(path) as fh:
        for r in csv.DictReader(fh, delimiter="\t"):
            if (r.get("Decoy/Contaminant/Target") or "").strip() != "T":
                continue
            qv = r.get("QValue")
            try:
                if qv in (None, "") or float(qv) > q:
                    continue
            except ValueError:
                continue
            pep = STRIP.sub("", r.get("Base Sequence") or "").strip().upper()
            acc = (r.get("Accession") or "").split("|")[0].strip()
            if "|" in (r.get("Accession") or ""):
                acc = (r.get("Accession") or "").split("|")[0].strip()
            if pep and acc:
                yield pep, acc


def from_sage(path, q=0.01):
    with open(path) as fh:
        for r in csv.DictReader(fh, delimiter="\t"):
            if r.get("label", "1") == "-1":
                continue
            if float(r["peptide_q"]) >= q:
                continue
            pep = STRIP.sub("", r["peptide"]).strip().upper()
            acc = r["proteins"].split(";")[0].strip()
            for tok in (acc.split("|")[1:2] or [acc]):
                acc = tok
            if pep and acc:
                yield pep, acc


def pct(n, d):
    return (100.0 * n / d) if d else float("nan")


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


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    fasta = os.path.expanduser("~/Documents/proteomicsTesting/uniprot_sprot_iso_human-2018_06.fasta")
    require(fasta, "the shared 2018 database every side is classified against")
    seqs = read_fasta(fasta)
    print(f"FASTA: {fasta}\n  {len(seqs)} sequences\n")

    rows = []
    rec_tsv = f"{root}/recon-output/full-run/liver_search/pass2/results.sage.tsv"
    require(rec_tsv, "recon Pass 2 arm")
    rows.append(tally(from_sage(rec_tsv), seqs, "recon Pass 2 (semi)"))
    rows.append(tally(from_fragpipe(require(
        f"{root}/reference-data/ptm-shepherd/liverShepherd/peptide.tsv", "PTM-Shepherd arm")), seqs,
        "PTM-Shepherd open (FULLY)"))
    rows.append(tally(from_metamorpheus(require(
        f"{root}/reference-data/metamorpheus/liverMetaMorpheus/Task3-SearchTask/AllPeptides.psmtsv",
        "MetaMorpheus arm — GITIGNORED, see .gitignore:62")),
        seqs, "MetaMorpheus (FULLY)"))

    hdr = f"{'source':<27}{'peptides':>9}{'den':>8}{'missed cl':>12}{'ragged-N':>11}{'ragged-C':>11}{'nontryp':>10}{'N:C':>7}"
    print(hdr)
    print("-" * len(hdr))
    for r in rows:
        nc = (r["rn"] / r["rc"]) if r["rc"] else float("nan")
        print(f"{r['label']:<27}{r['n_pep']:>9}{r['den']:>8}"
              f"{pct(r['mc'],r['den']):>11.2f}%{pct(r['rn'],r['den']):>10.2f}%"
              f"{pct(r['rc'],r['den']):>10.2f}%{pct(r['nt'],r['allp']):>9.2f}%{nc:>7.2f}")
    # Preview, quoted verbatim from result_summary.html via the vendored README.
    print(f"{'Byonic Preview v3.2.0':<27}{2009:>9}{2008:>8}"
          f"{15.9:>11.2f}%{8.6:>10.2f}%{1.3:>10.2f}%{0.1:>9.2f}%{172/26:>7.2f}")
    print()
    for r in rows:
        if r["unresolved"]:
            print(f"⚠ {r['label']}: {r['unresolved']} peptides unresolved "
                  f"(accession absent from this FASTA, or peptide not in that protein) — EXCLUDED")
    print("\nPreview row is quoted from result_summary.html (see the vendored README),")
    print("not recomputed: its peptide list is not shipped.")


if __name__ == "__main__":
    main()
