#!/usr/bin/env python3
"""Generate the PROPOSED HTML report layout from a committed recon artifact.

WHY THIS IS IN THE REPO. The report redesign (Ben, 2026-09-01) was settled over
four mockup revisions rather than in prose, and the agreement is the layout
itself. This script IS that agreement. Port it into `report.rs` when the HTML
landings happen; do not re-invent the arrangement from the notes.

    python3 testing/scripts/report_layout_mockup.py

Reads `testing/recon-output/full-run/serum{,_pass2}.json` and writes an HTML file
beside itself. Values that do not exist yet are rendered in orange and marked.

WHAT IT ENCODES, all of it decided at the mockup:
  * section order: header, detectors, mass accuracy, contamination, glyco,
    digestion, recommended modifications, footer;
  * Signal Fate, the Modification Landscape table and the Alkylation section are
    GONE -- see NOTES;
  * one tolerance line, both rungs off the ladder;
  * delta mass first; residue and position share ONE column;
  * "Detected but did not make the cut", never "not recommended";
  * `Decided by` is the ROUTE (statistics or floor), and the explainer opens with
    how many delta masses were assessed;
  * no Pass-1/Pass-2 vocabulary anywhere -- that is ours, not the user's;
  * CSV button over the three tables.

⚠ It reads the CURRENT schema. If a field is renamed, this breaks loudly, which
is the intent.
"""
import json, html, os
OUT=os.path.join(os.path.dirname(os.path.abspath(__file__)), "report_layout_mockup.html")
d=json.load(open('testing/recon-output/full-run/serum.json'))
p=json.load(open('testing/recon-output/full-run/serum_pass2.json'))
e=lambda s: html.escape(str(s))
inp=d['input']; an=d['analyzers']; cal=d['ms1_calibration']; pol=d['polymer']; ox=d['oxonium']
rec=d['recommendations']; comp=p['composition']; term=p['terminus']

# `ms2_bias_ppm` was renamed `ms2_median_abs_ppm` at schema 2.1.0. Committed
# artifacts predating that still carry the old key, so read either rather than
# forcing a regeneration just to render a mockup.
MS2_ABS = cal.get('ms2_median_abs_ppm', cal.get('ms2_bias_ppm'))

LADDER=[10,20,50,100]
def rung(v):
    for r in LADDER:
        if v<=r: return r
    return LADDER[-1]
ms2_rec=rung(cal['ms2_tolerance_high_ppm'])
ms1_rec=cal['user_recommendation_tolerance_ppm']

def site_cell(sites, position):
    s=(sites or '').strip()
    pos=(position or '').strip()
    main=f"<b>{e(s)}</b>" if s else "<span class=mut>any residue</span>"
    if pos and pos.lower() not in ('anywhere.','anywhere',''):
        main+=f" <span class=mut>{e(pos.rstrip('.'))}</span>"
    return main

def rec_rows(items):
    out=[]
    for m in items:
        ev=('OR %.1f, q %.2g'%(m['odds_ratio'],m['q_value'])) if m.get('odds_ratio') is not None \
           else ('%.0f%% of floor'%m['pct_of_floor'] if m.get('pct_of_floor') is not None else '—')
        out.append(f"<tr><td class=num>{m['delta_mass']:+.4f}</td><td>{e(m['label'])}</td>"
                   f"<td>{site_cell(m.get('sites'),m.get('position'))}</td>"
                   f"<td class=num>{m['count']:,}</td><td class=num>{m['count_pct']:.2f}%</td>"
                   f"<td>{e(m['decided_by'])}</td><td class=why>{ev}</td></tr>")
    return "\n".join(out)

# CORRECTED 2026-09-01. The floor is NOT gate 1: a curated mod with testable
# residues goes to statistics and the floor never applies to it (6 of serum's 9
# recommended mods are BELOW the floor). So this table holds anything that was
# actually TESTED and failed, plus un-curated/satellite deltas that cleared the
# floor. Untested things below the floor are simply not shown.
floor=rec['floor_psms']
cut=[m for m in rec['not_recommended']
     if m.get('reason')=='no_residue_support' or m['count']>=floor]
def cut_rows(items):
    out=[]
    for m in items:
        reason=m.get('reason','')
        if reason.startswith('satellite'):
            decided, why = 'floor', f"isotope satellite — {e(reason)}"
        elif reason=='no_residue_support':
            orr=m.get('odds_ratio')
            decided='statistics'
            why=f"failed the residue test — OR {orr:.2f} (needs ≥{rec['odds_ratio_min']}), q <span class=ph>NOT SERIALISED</span> (needs ≤{rec['q_max']})"
        elif reason=='not_curated':
            decided, why = 'floor', "no curated entry — un-curated delta mass"
        else:
            decided, why = '—', e(reason)
        lbl=m.get('label') or '<span class=mut>un-curated</span>'
        src='<span class=mut>named from Unimod</span>' if m.get('name_source')=='unimod' else ''
        out.append(f"<tr><td class=num>{m['delta_mass']:+.4f}</td><td>{lbl} {src}</td>"
                   f"<td><span class=mut>—</span></td><td class=num>{m['count']:,}</td><td class=num>—</td>"
                   f"<td>{decided}</td><td class=why>{why}</td></tr>")
    return "\n".join(out)

H="<tr><th class=num>Delta mass</th><th>Modification</th><th>Residue</th><th class=num>Count</th><th class=num>%</th><th>Decided by</th><th>Evidence</th></tr>"
HW=H.replace("<th>Evidence</th>","<th>Why not</th>")

doc=f"""<!doctype html><meta charset=utf-8><title>{e(inp['mzml_file'].split('/')[-1])} — recon</title>
<style>
:root{{--fg:#1a1a1a;--mut:#767676;--line:#e3e3e3;--card:#fafafa;--acc:#0b5fa5;--warn:#a35b00;--ph:#b34700}}
body{{margin:0;font:14px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;color:var(--fg);background:#fff}}
.wrap{{max-width:1000px;margin:0 auto;padding:24px}}
h1{{font-size:20px;margin:0 0 4px}} h2{{font-size:15px;margin:0}}
.sub{{color:var(--mut);font-size:13px;margin-bottom:18px}}
.meta{{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:10px 20px;background:var(--card);border:1px solid var(--line);border-radius:6px;padding:14px 16px;margin-bottom:22px}}
.meta div span{{display:block;color:var(--mut);font-size:11px;text-transform:uppercase;letter-spacing:.05em}}
.meta div b{{font-weight:600;font-size:13px;word-break:break-all}}
section{{border:1px solid var(--line);border-radius:6px;margin-bottom:16px;overflow:hidden}}
.hd{{background:var(--card);padding:10px 14px;border-bottom:1px solid var(--line);display:flex;justify-content:space-between;align-items:center}}
.bd{{padding:14px}}
.stats{{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:14px}}
.stat b{{display:block;font-size:19px}} .stat span{{color:var(--mut);font-size:11px;text-transform:uppercase;letter-spacing:.04em}}
table{{width:100%;border-collapse:collapse;font-size:13px}}
th{{text-align:left;font-size:11px;text-transform:uppercase;letter-spacing:.04em;color:var(--mut);border-bottom:1px solid var(--line);padding:6px 8px}}
td{{padding:6px 8px;border-bottom:1px solid #f0f0f0;vertical-align:top}}
.num{{text-align:right;font-variant-numeric:tabular-nums}}
.mut{{color:var(--mut);font-weight:400}}
.tbl-title{{font-size:12px;font-weight:600;text-transform:uppercase;letter-spacing:.05em;color:var(--acc);margin:16px 0 4px}}
.tbl-title:first-child{{margin-top:0}}
.why{{color:var(--mut)}}
.ph{{color:var(--ph);font-weight:600}}
.note{{background:#fbfbfb;border-left:3px solid var(--line);padding:10px 12px;font-size:12.5px;color:#555;margin-top:14px}}
details{{border-top:1px solid var(--line)}} details summary{{cursor:pointer;padding:9px 14px;font-size:13px;background:var(--card)}}
footer{{margin-top:26px;border-top:2px solid var(--fg);padding-top:14px;font-size:12.5px;color:var(--mut)}}
footer a{{color:var(--acc)}}
button{{font:inherit;font-size:12px;border:1px solid var(--line);background:#fff;border-radius:4px;padding:4px 10px;cursor:pointer}}
.banner{{background:#eef4fb;border:1px solid #c3d9ef;border-radius:6px;padding:10px 14px;margin-bottom:18px;font-size:12.5px}}
</style>
<div class=wrap>
<div class=banner><b>LAYOUT MOCKUP rev 3</b> — real serum data. <span class=ph>Orange</span> does not exist yet.</div>

<h1>{e(inp['mzml_file'].split('/')[-1])}</h1>
<div class=sub>Proteomics reconnaissance report</div>

<div class=meta>
  <div><span>FASTA</span><b>UniProt-Human-UP000005640_canonical-2023_05.fasta</b></div>
  <div><span>Enzyme</span><b class=ph>trypsin — KR, not before P, C-term</b></div>
  <div><span>Generated</span><b>{e(d['generated_at'][:19].replace('T',' '))} UTC</b></div>
  <div><span>recon</span><b>v{e(d['tool_version'])}</b></div>
  <div><span>Total runtime</span><b class=ph>65.1 s</b></div>
</div>

<section><div class=hd><h2>Detectors</h2></div><div class=bd><div class=stats>
  <div class=stat><span>Instrument</span><b style="font-size:14px">{e(an['instrument_model'])}</b></div>
  <div class=stat><span>MS1 analyzer</span><b style="font-size:14px">{e(an['ms1_analyzers'][0])}</b></div>
  <div class=stat><span>MS2 analyzer</span><b style="font-size:14px">{e(an['ms2_analyzers'][0])}</b></div>
  <div class=stat><span>MS2 scans</span><b>{inp['ms2_spectra']:,}</b></div>
</div></div></section>

<section><div class=hd><h2>Mass accuracy</h2></div><div class=bd><div class=stats>
  <div class=stat><span>Precursor / MS1 — signed median</span><b>{cal['bias_ppm']:+.2f} ppm</b></div>
  <div class=stat><span>Fragment / MS2 — |median|</span><b>{MS2_ABS:.2f} ppm</b></div>
  <div class=stat><span>Recommended MS1 / MS2</span><b>{ms1_rec:.0f} / {ms2_rec:.0f} ppm</b></div>
</div></div></section>

<section><div class=hd><h2>Contamination</h2></div><div class=bd>
<div class=stats>
  <div class=stat><span>Polymer % of TIC</span><b>{pol['total_pct_tic']:.2f}%</b></div>
  <div class=stat><span>Level</span><b style="font-size:14px">{e(pol['contamination_level'])}</b></div>
</div>
<table style="margin-top:12px"><tr><th>Polymer</th><th class=num>% TIC</th></tr>
{''.join(f"<tr><td>{e(x['name'])}</td><td class=num>{x['pct_tic']:.3f}%</td></tr>" for x in pol['top_polymers'][:5])}
</table></div></section>

<section><div class=hd><h2>Glycopeptides</h2></div><div class=bd><div class=stats>
  <div class=stat><span>Candidate spectra</span><b>{ox['glycopeptide_candidates']:,}</b></div>
  <div class=stat><span>% of MS2</span><b>{ox['glycopeptide_pct']:.2f}%</b></div>
</div></div></section>

<section><div class=hd><h2>Digestion</h2></div><div class=bd><div class=stats>
  <div class=stat><span>Missed cleavage</span><b>{comp['missed_cleavage']['pct']:.2f}%</b></div>
  <div class=stat><span>Ragged N</span><b>{comp['ragged_n']['pct']:.2f}%</b></div>
  <div class=stat><span>Ragged C</span><b>{comp['ragged_c']['pct']:.2f}%</b></div>
  <div class=stat><span>N : C ratio</span><b>{term['n_c_ratio']:.2f}</b></div>
</div></div></section>

<section><div class=hd><h2>Recommended search modifications</h2><button onclick="csv()">Copy as CSV</button></div><div class=bd>
<div class=tbl-title>Fixed</div>
<table id=t1>{H}{rec_rows(rec['fixed'])}</table>
<div class=tbl-title>Variable</div>
<table id=t2>{H}{rec_rows(rec['variable'])}</table>
<div class=tbl-title>Detected but did not make the cut</div>
<table id=t3>{HW}{cut_rows(cut)}</table>

<div class=note><b>{len(d['mod_discovery']['peaks'])} distinct delta masses were detected and assessed in this file.</b>
Each took one of two routes:<br><br>
<b>Statistics</b> — a curated modification naming specific acceptor residues is tested against them, and is
recommended only if <b>OR ≥ {rec['odds_ratio_min']}</b> and <b>q ≤ {rec['q_max']}</b>. Abundance is not
consulted: a modest peak with strong residue evidence is still recommended, and a tall one without it is not.<br><br>
<b>Floor</b> — everything statistics cannot test: modifications with no specific residue (anything at a
terminus, say) and delta masses with no curated entry. These are judged on abundance alone, against a floor of
<b>{rec['floor_psms']:.0f} PSMs</b> ({rec['floor_pct_of_top']:.0f}% of the tallest peak). Below it, they are not shown.<br><br>
<b>Decided by</b> tells you which route each row took.</div>

<div class=note><b>These counts are lower than your next search will report, and that is expected.</b>
This is one open search: each spectrum is assigned a single delta mass, so occupancy is split across every
form a peptide takes. <b>Use the ranking, not the magnitude.</b> A targeted search with these
modifications set will report higher numbers for the same chemistry.</div>
</div></section>

<footer>
<b>sageRecon</b> v{e(d['tool_version'])} · using Sage v0.15.0-beta.2 · maintained by Benjamin Neely (NIST),
<a href="mailto:benjamin.neely@nist.gov">benjamin.neely@nist.gov</a><br>
<span class=ph>[REPO URL PLACEHOLDER — likely "sageRecon"; NIST hosting not settled]</span>
<details><summary>Acknowledgements</summary><div class=bd>
<b>Sage</b> — <a href="https://github.com/lazear/sage">github.com/lazear/sage</a> — Lazear, M.R.
<i>J. Proteome Res.</i> 2023, 22(11), 3652–3659. doi:10.1021/acs.jproteome.3c00486<br>
<b>MetaMorpheus</b> (curated modification list) — <a href="https://github.com/smith-chem-wisc/metamorpheus">github.com/smith-chem-wisc/metamorpheus</a> —
Solntsev, Shortreed, Frey, Smith. <i>J. Proteome Res.</i> 2018, 17(5), 1844–1851. doi:10.1021/acs.jproteome.7b00873<br>
<b>mzSniffer</b> (polymer detection) — <a href="https://github.com/wfondrie/mzsniffer">github.com/wfondrie/mzsniffer</a> — no publication.<br>
<b>Pyteomics</b> (code ideas) — <a href="https://github.com/levitsky/pyteomics">github.com/levitsky/pyteomics</a> —
Levitsky, Klein, Ivanov, Gorshkov. <i>J. Proteome Res.</i> 2019, 18(2), 709–714. doi:10.1021/acs.jproteome.8b00717<br>
<b>Unimod</b> — <a href="https://www.unimod.org/">unimod.org</a> — Design Science License.
</div></details>
<details><summary>Licence — NIST</summary><div class=bd>NIST Software Licensing Statement (LICENSE.md) rendered here.</div></details>
<details><summary>Licence — third party</summary><div class=bd>THIRD_PARTY_LICENSES.md: Sage (MIT), mzSniffer (Apache 2.0), MetaMorpheus (MIT), Unimod (Design Science License, full text).</div></details>
</footer>
</div>
<script>
function csv(){{let o=[];for(const id of ['t1','t2','t3']){{const t=document.getElementById(id);
for(const r of t.rows){{o.push([...r.cells].map(c=>'"'+c.innerText.replace(/"/g,'""')+'"').join(','))}}o.push('')}}
navigator.clipboard.writeText(o.join('\\n'));alert('Copied as CSV')}}
</script>"""
open(OUT,'w',encoding='utf-8').write(doc)
print("wrote",len(doc),"bytes; cut-table rows:",len(cut))
