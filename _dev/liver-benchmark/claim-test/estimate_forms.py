#!/usr/bin/env python3
"""Count target peptide forms per Sage config (a memory ratio, not a Sage run).

Usage: python3 estimate_forms.py FASTA CONFIG.json [CONFIG.json ...]
Fully tryptic (KR, not before P), 2 missed cleavages, length 7 to 50, at most
2 variable mods with one mod per site, as Sage df92199 Peptide::apply. Ignores
the 500 to 5000 Da mass filter and decoys. Python 3 standard library only.
"""
import json,re,sys,itertools
FASTA=sys.argv[1]
def digest(seq):
    sites=[0]+[m.end() for m in re.finditer(r"[KR](?!P)",seq)]
    if sites[-1]!=len(seq): sites.append(len(seq))
    out=[]
    for i in range(len(sites)-1):
        for mc in range(3):
            j=i+1+mc
            if j>=len(sites): break
            s,e=sites[i],sites[j]; out.append((seq[s:e], s==0))
    return out
peps={}
seq=[];
for line in open(FASTA):
    if line.startswith(">"):
        if seq:
            for p,nt in digest("".join(seq)):
                if 7<=len(p)<=50: peps[(p,nt)]=1
        seq=[]
    else: seq.append(line.strip())
for p,nt in digest("".join(seq)):
    if 7<=len(p)<=50: peps[(p,nt)]=1
uniq={}
for (p,nt) in peps: uniq[(p,nt)]=1
print("base peptide/position entries",len(uniq))
for cfg in sys.argv[2:]:
    vm=json.load(open(cfg))["database"]["variable_mods"]
    tot=0; frag=0
    for (p,nt) in uniq:
        sites=[]
        for k,ms in vm.items():
            for m in ms:
                if k=="[":
                    if nt: sites.append(("N",m))
                elif k=="[M":
                    if nt and p[0]=="M": sites.append((0,m))
                elif k.startswith("^"):
                    if p[0]==k[1]: sites.append((0,m))
                else:
                    sites+= [(i,m) for i,a in enumerate(p) if a==k]
        n=1
        for r in (1,2):
            for c in itertools.combinations(sites,r):
                if len({s for s,_ in c})==r: n+=1
        tot+=n; frag+=n*(len(p)-1)
    print(cfg, "peptide forms", tot, "rel frag", frag)
