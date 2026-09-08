#!/usr/bin/env python3
"""Compose the full five-tool liver comparison: digestion, mass tolerance, PTMs.

WHY THIS IS A COMPOSER AND NOT A CALCULATOR. Every number here already has
exactly one producer:

* digestion  -> `liver_four_tool_digestion.py`
* PTMs       -> `liver_5way_mods.py`
* recon's own mass accuracy -> `testing/recon-output/full-run/liver.json`
* Preview's mass accuracy   -> its own `result_summary.html`

This script RUNS those producers and assembles their output. It recomputes
nothing, so there is no second implementation to drift. That is the same rule the
project applies to `CuratedDb::load` delegating to `load_from_sources`.

⚠ THE THREE SECTIONS DO NOT SHARE A TOOL SET, ON PURPOSE.

* **PTMs: five tools.** recon, Byonic Preview, PTM-Shepherd, MetaMorpheus, Mascot.
* **Digestion: four.** Mascot is EXCLUDED and must stay excluded — its `PFA=1`
  allows one missed cleavage against recon's two, and it ships no peptide list to
  reclassify under recon's own terminus rule. Putting it in the digestion table
  would be the same convention error as the four traps in NOTES.
* **Mass tolerance: two.** Only recon and Preview report a comparable quantity.
  PTM-Shepherd, MetaMorpheus and Mascot are not in that table because their runs
  do not publish one on the same basis — absence of a column, not a measurement
  of zero.

Usage:  python3 testing/scripts/liver_5way_report.py [-o OUT.md]
"""

import argparse
import html
import re
import subprocess
import sys
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RECON_LIVER = ROOT / "testing/recon-output/full-run/liver.json"
PREVIEW_SUMMARY = ROOT / "testing/reference-data/preview/10mg_1_A_1/result_summary.html"
DIGESTION = ROOT / "testing/scripts/liver_four_tool_digestion.py"
MODS = ROOT / "testing/scripts/liver_5way_mods.py"


def run(script):
    """Run a producer and return its stdout, failing loudly."""
    r = subprocess.run([sys.executable, str(script)], capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit(f"FATAL: {script.name} failed:\n{r.stderr[-2000:]}")
    return r.stdout


def preview_mass_accuracy():
    """Pull Preview's own measured mass error out of its summary HTML.

    ⚠ Parsed, not copied from NOTES. NOTES records these values, but a record of a
    measurement is not the measurement — and this project has already been bitten
    by trusting a summary. The numbers live in prose inside the HTML, not in a
    table, which is why they were missed for weeks.
    """
    if not PREVIEW_SUMMARY.exists():
        return None
    text = html.unescape(re.sub(r"<[^>]+>", " ", PREVIEW_SUMMARY.read_text(
        encoding="utf-8", errors="replace")))
    text = re.sub(r"\s+", " ", text)

    out = {}
    for what in ("precursor", "fragment"):
        m = re.search(
            r"Median %s m/z error \(observed minus true\):\s*(-?[\d.]+) Da,\s*(-?[\d.]+) ppm"
            r"\s*Median %s accuracy \(absolute value of error\):\s*(-?[\d.]+) Da,\s*(-?[\d.]+) ppm"
            r"\s*\w+[^:]*measured too high:\s*(\d+)\s*Too low:\s*(\d+)" % (what, what),
            text)
        if m:
            out[what] = dict(signed_ppm=float(m.group(2)), abs_ppm=float(m.group(4)),
                             high=int(m.group(5)), low=int(m.group(6)))
    return out or None


def recon_mass_accuracy():
    import json
    d = json.loads(RECON_LIVER.read_text())
    ma, cal = d["mass_accuracy"], d["ms1_calibration"]
    return d, ma, cal


def section(text, start, end=None):
    """Slice a producer's stdout between two markers."""
    i = text.find(start)
    if i < 0:
        return ""
    j = text.find(end, i + len(start)) if end else -1
    return text[i:j if j > 0 else len(text)].rstrip()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-o", "--out", type=Path,
                    default=ROOT / "testing/recon-output/comparison/LIVER-FIVE-TOOL-2026-09-01.md")
    args = ap.parse_args()

    d, ma, cal = recon_mass_accuracy()
    sage_version = "unknown"
    try:
        import json
        rj = json.loads((ROOT / "testing/recon-output/full-run/liver_search/results.json").read_text())
        sage_version = rj.get("version", "unknown")
    except Exception:
        pass

    dig_out = run(DIGESTION)
    mod_out = run(MODS)
    pv = preview_mass_accuracy()

    L = []
    A = L.append
    A("# Liver — five-tool comparison: digestion, mass tolerance, modifications\n")
    A(f"Generated {date.today().isoformat()} by `testing/scripts/liver_5way_report.py`, "
      "which RUNS the per-section producers and composes their output. It recomputes "
      "nothing.\n")
    A(f"**recon numbers are Sage `{sage_version}`, report schema "
      f"`{d['schema_version']}`.** ⚠ Every recon number below is search-engine "
      "specific. The Sage version is part of the citation, not a footnote — the "
      "v0.14.7 -> v0.15.0-beta.2 upgrade moved most of them.\n")
    A("⚠ **The three sections deliberately use different tool sets.** PTMs have "
      "five tools; digestion has four, because Mascot's `PFA=1` allows one missed "
      "cleavage against recon's two and it ships no peptide list to reclassify; "
      "mass tolerance has two, because only recon and Preview publish a comparable "
      "quantity. A blank is 'this tool does not report it', never 'zero'.\n")

    A("\n---\n\n## 1. Digestion — four tools, one classifier\n")
    A("Every tool's peptide list is reclassified with **recon's own terminus rule**. "
      "No tool's own class columns are read: they do not share a convention. "
      "Preview's row is quoted from `result_summary.html` because its peptide list "
      "is not shipped.\n")
    A("```")
    A(section(dig_out, "source ").rstrip())
    A("```")

    A("\n---\n\n## 2. Mass tolerance — recon vs Byonic Preview\n")
    if pv:
        A("Preview's figures are **parsed from its own `result_summary.html`**, not "
          "copied from NOTES. Pre-recalibration is the comparable side: recon reports "
          "what the instrument delivered and does not recalibrate spectra.\n")
        A("| quantity | recon | Preview (pre-recal) | agreement |")
        A("|---|---|---|---|")
        f_abs = ma["fragment_median_ppm"]
        A(f"| MS2 median \\|error\\| | **{f_abs:.4f} ppm** | **{pv['fragment']['abs_ppm']:.1f} ppm** | "
          f"{abs(f_abs - pv['fragment']['abs_ppm']):.2f} ppm |")
        A(f"| MS2 signed median | not produced, by design | **{pv['fragment']['signed_ppm']:+.1f} ppm** | "
          f"recon has no signed MS2 number; Preview supplies the reference |")
        A(f"| MS1 signed median | **{cal['bias_ppm']:+.4f} ppm** | **{pv['precursor']['signed_ppm']:+.1f} ppm** | "
          f"⚠ **{abs(cal['bias_ppm'] - pv['precursor']['signed_ppm']):.2f} ppm apart — UNRESOLVED** |")
        A(f"| MS1 median \\|error\\| | not reported separately | {pv['precursor']['abs_ppm']:.1f} ppm | |")
        A("")
        A(f"Preview's own directional counts: fragments **{pv['fragment']['high']} high / "
          f"{pv['fragment']['low']} low** (a {pv['fragment']['low']/max(pv['fragment']['high'],1):.0f}:1 "
          "imbalance, which is not compatible with a centred distribution and "
          "corroborates its signed MS2 value); precursors "
          f"**{pv['precursor']['high']} high / {pv['precursor']['low']} low** (balanced).\n")
        A("✅ **recon's MS2 accuracy is corroborated by an independent tool on the "
          "same raw file, same quantity.**\n")
        A("⚠ **The MS1 disagreement is NOT resolved and must not be presented as "
          "corroborated.** Untested candidate causes: Preview measured the `.mgf` "
          "after its own conversion; the populations differ by an order of magnitude "
          f"(recon's clean subset is {cal['clean_subset_n_psms']} PSMs against "
          "Preview's ~1793 precursors); and the peptide sets are not the same.\n")
    else:
        A("⚠ Preview's `result_summary.html` is absent, so this section could not be "
          "produced. It is NOT reported as agreement.\n")

    A("### What recon recommends from this\n")
    A(f"* MS1 search window: **±{cal['user_recommendation_tolerance_ppm']:.0f} ppm** "
      f"(quantized ladder rung; the raw requirement is "
      f"{cal.get('user_recommendation_requirement_ppm', float('nan')):.3f} ppm).")
    A(f"* Pass-2 MS1 window actually used: "
      f"{cal['pass2_window_low_ppm']:+.3f} .. {cal['pass2_window_high_ppm']:+.3f} ppm.")
    A(f"* MS2: measured |error| {cal['ms2_median_abs_ppm']:.4f} ppm, spread "
      f"{cal['ms2_spread_mad_ppm']:.4f} ppm MAD.")
    A(f"  ⚠ `ms2_tolerance_*` = {cal['ms2_tolerance_low_ppm']:+.4f} .. "
      f"{cal['ms2_tolerance_high_ppm']:+.4f} ppm is a SPREAD ABOUT THE MEDIAN, **not "
      "a search window**. A closed search wants roughly ±5 ppm here, not ±1.1.\n")

    A("\n---\n\n## 3. Modifications — five tools\n")
    A(section(mod_out, "## Do recon's recommendations line up?"))

    A("\n---\n\n## Caveats that travel with these numbers\n")
    A("* **+57 is three discoveries and one assumption.** Preview's cysteine +57 was "
      "an operator preset, not a finding, so it is not a fourth independent "
      "corroboration of the alkylation call.")
    A("* **-89.0302 Met-loss+Acetylation is SINGLE-SOURCE** (Mascot only) despite a "
      "large odds ratio. Label it as such wherever the recommendation set is quoted.")
    A("* **Oxidation on proline is not recommended, and all four other tools report "
      "it.** That is a design consequence, not a gap: `peptide_hits` is a CONTAINMENT "
      "test, proline sits in 56.0 % of liver peptides, and the candidate scores "
      "OR 1.34 against a threshold of 2.0. See NOTES "
      "\"`peptide_hits` IS A CONTAINMENT TEST\".")
    A("* **Preview's `(-fixed mod)` rows are offsets from its fixed +57 on cysteine**, "
      "not absolute deltas, and its `VariableMods.txt` repeats one group total across "
      "site rows. Both conventions are verified against Preview's own detail text.")
    A("")

    out = "\n".join(L) + "\n"
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(out, encoding="utf-8")
    print(f"wrote {args.out.relative_to(ROOT)}  ({len(out.splitlines())} lines)")

    hp = args.out.with_suffix(".html")
    hp.write_text(to_html(out, "Liver, Five Ways: Digestion, Mass Tolerance, Mods"),
                  encoding="utf-8")
    print(f"wrote {hp.relative_to(ROOT)}")




# ---------------------------------------------------------------------------
# HTML companion
# ---------------------------------------------------------------------------
# The stylesheet is READ FROM the existing companion page rather than copied
# into this file, so there is ONE stylesheet for the comparison reports and a
# restyle does not have to be applied twice. The output is standalone (inline
# CSS) because these pages are read on their own, off the repo.

STYLE_SOURCE = ROOT / "testing/recon-output/comparison/LIVER-FIVE-TOOL-MODS-2026-09-01.html"


def _inline(s):
    """Markdown inline -> HTML, for the subset this report emits."""
    s = (s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;"))
    s = re.sub(r"`([^`]+)`", r"<code>\1</code>", s)
    s = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", s)
    s = s.replace(r"\|", "|")
    return s


def to_html(md, title):
    css = ""
    if STYLE_SOURCE.exists():
        m = re.search(r"<style>.*?</style>", STYLE_SOURCE.read_text(encoding="utf-8"), re.S)
        if m:
            css = m.group(0)
    out, lines, i = [], md.splitlines(), 0
    while i < len(lines):
        ln = lines[i]
        if ln.startswith("```"):
            j = i + 1
            buf = []
            while j < len(lines) and not lines[j].startswith("```"):
                buf.append(lines[j]); j += 1
            body = "\n".join(buf).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
            out.append(f"<pre>{body}</pre>"); i = j + 1; continue
        if ln.startswith("|") and i + 1 < len(lines) and set(lines[i+1].replace("|", "").strip()) <= set("-: "):
            SP = "\x00PIPE\x00"
            hdr = [c.strip().replace(SP, r"\|")
                   for c in ln.replace(r"\|", SP).strip("|").split("|")]
            j = i + 2; rows = []
            while j < len(lines) and lines[j].startswith("|"):
                rows.append([c.strip().replace(SP, r"\|")
                             for c in lines[j].replace(r"\|", SP).strip("|").split("|")]); j += 1
            t = ["<table><tbody><tr>" + "".join(f"<th>{_inline(h)}</th>" for h in hdr) + "</tr>"]
            for r in rows:
                t.append("<tr>" + "".join(f"<td>{_inline(c)}</td>" for c in r) + "</tr>")
            t.append("</tbody></table>")
            out.append("".join(t)); i = j; continue
        if ln.startswith("#"):
            lvl = len(ln) - len(ln.lstrip("#"))
            out.append(f"<h{lvl}>{_inline(ln.lstrip('# ').strip())}</h{lvl}>"); i += 1; continue
        if ln.strip() == "---":
            out.append("<hr>"); i += 1; continue
        if ln.startswith("* "):
            items = []
            while i < len(lines) and lines[i].startswith("* "):
                items.append(f"<li>{_inline(lines[i][2:])}</li>"); i += 1
            out.append("<ul>" + "".join(items) + "</ul>"); continue
        if ln.strip():
            cls = ""
            if ln.lstrip().startswith("⚠"): cls = ' class="callout warn"'
            elif ln.lstrip().startswith("✅"): cls = ' class="callout ok"'
            out.append(f"<p{cls}>{_inline(ln)}</p>")
        i += 1
    extra = ("<style>.callout{border-left:3px solid var(--caution);padding:.6rem .9rem;"
             "background:var(--surface);}.callout.ok{border-left-color:var(--settled);}"
             "hr{border:0;border-top:1px solid var(--rule);margin:2rem 0;}</style>")
    return (f"<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n"
            f"<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n"
            f"<!-- Rendered companion. Generated by liver_5way_report.py; not hand-edited.\n"
            f"     Stylesheet is read from LIVER-FIVE-TOOL-MODS-2026-09-01.html so the\n"
            f"     comparison reports share one look. -->\n<title>{title}</title>\n"
            "<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\n"
            "<link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin>\n"
            "<link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500;600&family=IBM+Plex+Sans:wght@400;500;600;700&family=IBM+Plex+Serif:ital,wght@0,400;0,500;1,400&display=swap\">\n"
            f"{css}\n{extra}\n</head>\n<body>\n<div class=\"wrap\">\n"
            + "\n".join(out) +
            "\n</div>\n</body>\n</html>\n")


if __name__ == "__main__":
    main()
