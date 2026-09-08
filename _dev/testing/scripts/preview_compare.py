#!/usr/bin/env python3
"""Compute Preview's four digestion numbers from a recon Pass 2 TSV.

Reproduces Preview's OWN denominators, read off the 10mg_1_A_1 report:
  missed cleavage : (peptides with internal K/R not followed by P) / (tryptic + semitryptic)
  ragged-N        : ragged-N / (tryptic + semitryptic)
  ragged-C        : ragged-C / (tryptic + semitryptic)
  nontryptic      : nontryptic / (all peptides)

Basis is DISTINCT PEPTIDES, not PSMs. Prints fractions beside percentages,
because Preview's own report shows the same numerator under two different
denominators on two pages and only the fraction disambiguates it.
"""
import csv, re, sys, collections

STRIP = re.compile(r"\[[^\]]*\]")


def read_fasta(path):
    seqs, acc, buf = {}, None, []
    for line in open(path):
        if line.startswith(">"):
            if acc:
                seqs[acc] = "".join(buf)
            acc = line[1:].split()[0]
            buf = []
        else:
            buf.append(line.strip())
    if acc:
        seqs[acc] = "".join(buf)
    return seqs


def classify(pep, seq):
    """recon's rule, mirrored: see digestion.rs classify_terminus."""
    i = seq.find(pep)
    if i < 0:
        return "NOT_FOUND"
    end = i + len(pep)
    nt = (i == 0) or (seq[i - 1] in "KR" and not pep.startswith("P"))
    ct = (end >= len(seq)) or (pep[-1] in "KR" and seq[end] != "P")
    return {(True, True): "FULLY", (False, True): "RAGGED_N",
            (True, False): "RAGGED_C", (False, False): "NONTRYPTIC"}[(nt, ct)]


def internal_missed(pep):
    return sum(1 for k in range(len(pep) - 1) if pep[k] in "KR" and pep[k + 1] != "P")


def main(tsv, fasta, q_thresh=0.01):
    seqs = read_fasta(fasta)
    # distinct peptide -> first protein seen (Preview counts peptides, not PSMs)
    peps, psm_rows = {}, 0
    with open(tsv) as fh:
        for r in csv.DictReader(fh, delimiter="\t"):
            if r.get("label", "1") == "-1":
                continue
            if float(r["peptide_q"]) >= q_thresh:
                continue
            psm_rows += 1
            pep = STRIP.sub("", r["peptide"]).strip()
            peps.setdefault(pep, r["proteins"].split(";")[0].strip())

    cls = collections.Counter()
    mc_hits = 0
    unresolved = []
    for pep, prot in peps.items():
        seq = seqs.get(prot)
        c = classify(pep, seq) if seq else "PROTEIN_NOT_FOUND"
        cls[c] += 1
        if c in ("FULLY", "RAGGED_N", "RAGGED_C") and internal_missed(pep) >= 1:
            mc_hits += 1
        if c in ("NOT_FOUND", "PROTEIN_NOT_FOUND"):
            unresolved.append((pep, prot))

    tryp_semi = cls["FULLY"] + cls["RAGGED_N"] + cls["RAGGED_C"]
    allp = tryp_semi + cls["NONTRYPTIC"]
    pct = lambda n, d: (100.0 * n / d) if d else float("nan")

    print(f"confident target PSMs      : {psm_rows}")
    print(f"distinct peptides          : {len(peps)}")
    print(f"tryptic + semitryptic (den): {tryp_semi}")
    print(f"all peptides (den)         : {allp}")
    if unresolved:
        print(f"⚠ unresolved (excluded)    : {len(unresolved)}  e.g. {unresolved[:3]}")
    print()
    print("                     recon                         Preview 10mg_1_A_1")
    print(f"missed cleavage : {pct(mc_hits,tryp_semi):5.1f}% ({mc_hits}/{tryp_semi})"
          f"{'':>12}15.9% (319/2008)")
    print(f"ragged-N        : {pct(cls['RAGGED_N'],tryp_semi):5.1f}% ({cls['RAGGED_N']}/{tryp_semi})"
          f"{'':>12} 8.6% (172/2008)")
    print(f"ragged-C        : {pct(cls['RAGGED_C'],tryp_semi):5.1f}% ({cls['RAGGED_C']}/{tryp_semi})"
          f"{'':>12} 1.3% (26/2008)")
    print(f"nontryptic      : {pct(cls['NONTRYPTIC'],allp):5.1f}% ({cls['NONTRYPTIC']}/{allp})"
          f"{'':>12} 0.1% (2/2009)")
    print()
    nc = (cls["RAGGED_N"] / cls["RAGGED_C"]) if cls["RAGGED_C"] else float("inf")
    print(f"N:C ratio       : {nc:.2f}   (Preview: {172/26:.2f})")
    print()
    print("⚠ Populations differ: Preview restricted to representative proteins with its")
    print("  own score thresholds; recon's Pass 2 searches the >=2-peptide subset FASTA.")
    print("  Gate is DIRECTION and rough scale, not equality.")


if __name__ == "__main__":
    if len(sys.argv) < 3:
        sys.exit("usage: preview_compare.py <pass2_results.sage.tsv> <subset.fasta> [q]")
    main(sys.argv[1], sys.argv[2], float(sys.argv[3]) if len(sys.argv) > 3 else 0.01)
