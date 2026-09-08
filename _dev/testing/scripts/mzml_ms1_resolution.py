#!/usr/bin/env python3
"""Check 5 — record the MS1 resolution of each test file from its mzML metadata.

WHY THIS MATTERS. `reference-notes/deamidation-wide-search-notes.md` states that the
19 mDa gap between deamidation (+0.984) and the M1 isotope (+1.003) needs roughly
120,000 MS1 resolution to baseline-resolve, that 60,000 blurs it, and that the
blurring worsens with charge state. Our files' resolution is not recorded anywhere
in the repo. Without it we cannot say whether the +/-1 Da structure recon reports is
resolvable signal or smear.

WHAT THIS SCRIPT DOES NOT ASSUME. Resolution is recorded inconsistently across
vendors and converters. Rather than look for one term and report "not found", this
dumps every plausible carrier and lets the reader decide:
  - scan-level cvParam MS:1000011 (mass resolution)
  - the Thermo filter string, cvParam MS:1000512, which usually contains FTMS/ITMS
    and the scan ranges
  - preset scan configuration, MS:1000616
  - instrument configuration components and the analyzer terms in the header
  - the distinct values seen across MS1 scans, with counts, since a run can mix

Reads .mzML or .mzML.gz by streaming, so it does not load a multi-GB file into RAM.

Usage (from repo root):
    python testing/scripts/mzml_ms1_resolution.py --mzml testing/inputs/2019-4-9_909c_0311.mzML.gz --name serum
"""

import argparse
import gzip
import io
import re
import xml.etree.ElementTree as ET
from collections import Counter

# cvParam accessions worth reporting if present.
ACCESSIONS = {
    "MS:1000011": "mass resolution",
    "MS:1000512": "filter string",
    "MS:1000616": "preset scan configuration",
    "MS:1000927": "ion injection time",
    "MS:1000505": "base peak intensity",
}
ANALYZER_TERMS = ["orbitrap", "fourier transform", "ion trap", "time-of-flight",
                  "quadrupole", "FTMS", "ITMS"]


def opener(path):
    return gzip.open(path, "rt", encoding="utf-8", errors="replace") if path.endswith(".gz") \
        else open(path, "r", encoding="utf-8", errors="replace")


def strip_ns(tag):
    return tag.rsplit("}", 1)[-1]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--mzml", required=True)
    ap.add_argument("--name", required=True)
    ap.add_argument("--max-scans", type=int, default=4000)
    args = ap.parse_args()

    # Streaming XML parse. The first version scanned line by line, which returned
    # 0 MS1 scans on real files because converters commonly write mzML with very
    # few newlines -- the whole spectrumList can be one line. Do not go back to
    # line matching.
    header_terms = Counter()
    ms1_res = Counter()
    ms1_filter = Counter()
    ms1_seen = 0
    other_res = Counter()

    fh = opener(args.mzml)
    try:
        for event, elem in ET.iterparse(fh, events=("end",)):
            tag = strip_ns(elem.tag)

            if tag == "cvParam":
                nm = (elem.get("name") or "").lower()
                for t in ANALYZER_TERMS:
                    if t.lower() in nm:
                        header_terms[elem.get("name")] += 1

            if tag != "spectrum":
                continue

            level = None
            res = None
            filt = None
            for cv in elem.iter():
                if strip_ns(cv.tag) != "cvParam":
                    continue
                acc = cv.get("accession")
                if acc == "MS:1000511":
                    try:
                        level = int(cv.get("value"))
                    except (TypeError, ValueError):
                        pass
                elif acc == "MS:1000011":
                    res = cv.get("value")
                elif acc == "MS:1000512":
                    filt = cv.get("value")

            if level == 1:
                ms1_seen += 1
                if res:
                    ms1_res[res] += 1
                if filt:
                    ms1_filter[filt] += 1
            elif res:
                other_res[res] += 1

            elem.clear()
            if ms1_seen >= args.max_scans:
                break
    finally:
        fh.close()

    print("=" * 84)
    print(f"{args.name}  ({args.mzml})")
    print(f"MS1 scans inspected: {ms1_seen}")
    print("=" * 84)

    print("\nAnalyzer terms seen in cvParam names:")
    if header_terms:
        for t, n in header_terms.most_common(8):
            print(f"   {t}  (x{n})")
    else:
        print("   none matched")

    print("\nscan-level 'mass resolution' (MS:1000011) on MS1 scans:")
    if ms1_res:
        for v, n in ms1_res.most_common(10):
            print(f"   {v:>14}   {n} scans")
    else:
        print("   NOT RECORDED on MS1 scans.")
        if other_res:
            print("   (it IS present on non-MS1 scans: "
                  + ", ".join(f"{v} x{n}" for v, n in other_res.most_common(3)) + ")")

    print("\nDistinct MS1 filter strings (MS:1000512):")
    if ms1_filter:
        for v, n in ms1_filter.most_common(8):
            print(f"   [{n:5d} scans] {v}")
        print("\n   Thermo filter strings name the analyzer (FTMS = Orbitrap) but usually")
        print("   NOT the resolution setting. If resolution is absent everywhere, record")
        print("   that absence and take the value from the instrument method or the")
        print("   publication's methods section rather than inferring it.")
    else:
        print("   none found")

    print("\nWHAT TO DO WITH THIS:")
    print("   Wilmarth digest: ~120,000 baseline-resolves the 19 mDa deamidation/M1")
    print("   doublet; 60,000 blurs it, worse at higher charge. Record the value found,")
    print("   or its absence, in NOTES.")


if __name__ == "__main__":
    main()
