# Claim test: does recon's guidance beat a vanilla search?

The question an expert user asks. Take an unknown liver file. Either search it
with Ben's vanilla settings, or run recon first and search with what it
recommends, filtered by expert judgement. Which identifies more?

## The two searches

Both use stock Sage v0.15.0-beta.2 at the pinned rev
`df9219951cc9a54cf4cd55d76541af24b687bd3d`, fully tryptic (KR, not before P),
missed cleavages 1, length 8-50 (recon's own settings), charge 2-4, at most
2 variable mods per peptide, decoys generated. Only the mods and tolerances
differ.

| | vanilla (`configs/vanilla.json`) | recon-guided (`configs/recon_guided.json`) |
|---|---|---|
| fixed | Carbamidomethyl C | Carbamidomethyl C |
| variable | Oxidation M; pyro-Glu (peptide N-term Q); Deamidation N, Q; Acetyl (protein N-term) | the same, plus pyro-Glu (peptide N-term E), Fe[III] on D and E, and Met-loss+Acetylation (protein N-term) |
| MS1 / MS2 | 20 / 20 ppm | 10 / 10 ppm (recon's recommendation) |

**How the recon-guided list was chosen.** recon's liver report (v0.2.0,
`_dev/testing/recon-output/full-run/liver.json`) recommends 12 variable mods.
recon is written for an expert user, who applies judgement to that list. Ben's
rule of thumb: keep a modification if its PSM count is at least about 10 % of
the alkylation count (+57 on C: 1233 PSMs). The line falls at Trioxidation
(131 PSMs), which is also impossible alongside a fixed +57 on C, so it is out.
Oxidation M (1691), Deamidation (544), Gln->pyro-Glu (169) and Fe[III] (155)
are in. pyro-Glu from peptide N-term E (22) and Met-loss+Acetylation (73) are
kept because they are common and cheap to include. The rest (Dehydroalanine,
Formylation, Acetylation, Kynurenine, the -32 Da Met loss) are below the line.

**Met-loss+Acetylation in Sage.** Sage never removes the initiator Met, so it is
written as `[M: -89.02992` (protein N-terminal Met with the Met-loss plus
acetyl mass). Precursor and fragment masses match the real modified peptide.
Side effects: the Met still counts toward peptide length, and Sage can combine
it with the `[` +42.010565 acetyl entry on the same peptide.

## Run it

Needs `git`, Rust (https://rustup.rs) and Python 3. On the machine that runs
the searches:

```bash
git clone https://github.com/lazear/sage.git sage-df92199
cd sage-df92199 && git checkout df9219951cc9a54cf4cd55d76541af24b687bd3d && cargo build --release && cd ..
bash sageRecon/_dev/liver-benchmark/claim-test/run.sh \
  sage-df92199/target/release/sage \
  /path/to/10mg_1_A_1.mzML.gz \
  sageRecon/examples/uniprot_sprot_iso_human-2018_06.fasta \
  claim-work
```

On Windows (no bash), use PowerShell instead of the last command:

```powershell
powershell -ExecutionPolicy Bypass -File sageRecon\_dev\liver-benchmark\claim-test\run.ps1 -Sage sage-df92199\target\release\sage.exe -Mzml C:\path\to\10mg_1_A_1.mzML.gz -Fasta sageRecon\examples\uniprot_sprot_iso_human-2018_06.fasta -Work claim-work
```

- mzML: `10mg_1_A_1.mzML.gz` (NIST RM 8461 liver, PRIDE PXD013608).
- FASTA: `examples/uniprot_sprot_iso_human-2018_06.fasta` in this repo.
- `run.sh` records the Sage version and the input sha256, runs vanilla,
  recon-guided, then vanilla again (to measure run-to-run noise), strictly one
  at a time, with wall time and peak memory for each, and ends by writing
  `claim-work/summary.md` with `summarize.py`.

## Success criterion (set before the results)

The recon-guided search identifies more stripped peptide sequences at
peptide q <= 0.01 than vanilla, by more than the vanilla-vs-repeat noise, at a
runtime and memory cost an expert would accept. A null or negative result is
reported as found.

## Results (2026-09-25)

Run by Ben with `run.ps1` on a Windows laptop with 32 GB RAM; Sage
0.15.0-beta.2 built at the pinned rev. Input sha256: mzML
`460cd316cb95f0db468dfdbcdaeabcb195ba51eff585a1af2ae861f37575512e`, FASTA
`75cc5a96a489a04b385e07a3d4caa223cf361a3727b267c8961b86211c14c922`. Full output of
`summarize.py` is in `results.md`.

| | vanilla | recon-guided | change |
|---|---|---|---|
| PSMs (spectrum q <= 0.01) | 24,005 | 24,160 | +155 (+0.6 %) |
| peptides (peptide q <= 0.01) | 18,320 | 18,574 | +254 (+1.4 %) |
| stripped sequences | 15,185 | 15,310 | +125 (+0.8 %) |
| protein groups | 1,396 | 1,432 | +36 (+2.6 %) |
| wall time | 63 s | 355 s | 5.7x |
| peak memory | 10.0 GB | 22.3 GB | 2.2x |

- Run-to-run noise is zero: the vanilla repeat gave identical results (0
  sequences differ). So the gain is attributable to recon's guidance.
- Sequences: 14,796 shared, 514 found only with guidance, 389 only with vanilla.
- The added modifications carry real signal that vanilla cannot see: Fe[III] on
  D and E (407 PSMs), Met-loss+Acetylation (273), pyro-Glu from peptide N-term E
  (36).

**Verdict.** The criterion's first half is met: the guided search identifies
more sequences than vanilla, beyond run-to-run noise. The cost half is not
recon's call, and the note should say so (Ben, 2026-09-25). recon reports that
a modification is present in the sample at a level worth searching for,
sometimes one the user did not expect. Whether to pay the compute cost is the
user's decision, weighed against their search engine and resources. Here the
guidance gave a modest identification gain (about 1-3 %) and showed about 700
PSMs of chemistry the vanilla search missed, for 5.7x the time and 2.2x the
memory, most of it from Fe[III] on the common residues D and E.
