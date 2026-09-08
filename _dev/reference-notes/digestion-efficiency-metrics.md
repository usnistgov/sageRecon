# Digestion Efficiency Metrics in Proteomics

**Source:** Internal reference document for Sage + DDA data analysis  
**Created:** 2026-07-08

---

## The Three Metrics Defined

Before getting into the mechanics, it helps to be precise about what each metric means:

- **Digestion efficiency** — the fraction of the total precursor signal (or PSM count) accounted for by fully enzymatic (no missed cleavage, no non-specific cleavage) peptides. If porcine trypsin leaves ~10% of sites uncut, you'd expect roughly 85–90% of signal to be from clean tryptic peptides. [promega](https://www.promega.com/-/media/files/promega-worldwide/north-america/promega-us/webinars-and-events/trypsin_lys-c-webinar-apr2013.pdf?la=en)
- **Missed cleavages** — a peptide that spans an internal K or R (not followed by P) that trypsin *should* have cut but didn't. The `missed_cleavages` integer in Sage counts internal cleavage sites not cut per peptide. Trypsin generally yields 10–30% missed cleavage sites, predominantly at lysine residues. [info.gbiosciences](https://info.gbiosciences.com/blog/the-advantages-of-using-trypsin-for-mass-spectrometry)
- **Semi-tryptic peptides** — peptides with only *one* tryptic terminus. The other end is a "ragged" cut, i.e., cleavage at a position that doesn't follow trypsin's rules. In biofluids (plasma, CSF, urine), this is biologically expected because endogenous proteases (plasmin, kallikreins, matrix metalloproteases, etc.) circulate and produce ragged N- or C-termini *in vivo*, not as artifacts. A semi-tryptic peptide has exactly one enzymatic terminus and one non-specific terminus. [pubmed.ncbi.nlm.nih](https://pubmed.ncbi.nlm.nih.gov/35156369/)

---

## How Sage Handles This in Configuration

This is a **single search with the right parameters** — not multiple searches. Sage's `database` block in your JSON config controls everything: [sage-docs.vercel](https://sage-docs.vercel.app/docs/configuration/database)

```json
"database": {
    "enzyme": {
        "cleave_at": "KR",
        "restrict": "P",
        "c_terminal": true,
        "missed_cleavages": 2,
        "semi_enzymatic": true,
        "min_len": 7,
        "max_len": 50
    }
}
```

Key parameters for biofluid use case:

| Parameter | What it does | Recommended value (biofluids) |
|---|---|---|
| `cleave_at` | Residues where enzyme cuts | `"KR"` for trypsin |
| `restrict` | No cleavage if followed by this AA | `"P"` (trypsin rule) |
| `c_terminal` | Cut C-terminal of matching AAs | `true` for trypsin |
| `missed_cleavages` | Max internal uncut K/R per peptide | `2` (some push to 3 for biofluids) |
| `semi_enzymatic` | Allow one non-enzymatic terminus | `true` — **this is the key toggle** |

Setting `semi_enzymatic: true` tells Sage to generate candidates where *one* terminus obeys the trypsin rule and the *other* can be anywhere. This is what captures ragged ends. Without it, every PSM is forced to be fully tryptic and your semi-tryptic peptides will simply not be found. The tradeoff is a significantly larger search space, which increases search time and can lower scores for borderline PSMs. [reddit](https://www.reddit.com/r/massspectrometry/comments/sui5i6/how_exactly_does_the_missed_cleavages_parameter/)

---

## Reading the Metrics from `results.sage.tsv`

Sage natively outputs a `missed_cleavages` column per PSM in `results.sage.tsv` and the Parquet schema. After filtering to confident PSMs (e.g., `peptide_q ≤ 0.01`, `label == 1`), you can compute all three metrics in Pandas or Polars: [sage-docs.vercel](https://sage-docs.vercel.app/docs/results)

```python
import polars as pl

df = (
    pl.read_csv("results.sage.tsv", separator="\t")
    .filter(
        (pl.col("peptide_q") <= 0.01) &
        (pl.col("label") == 1)   # target, not decoy
    )
)

total = len(df)

# 1. Missed cleavage distribution
mc_dist = df.group_by("missed_cleavages").len().sort("missed_cleavages")

# 2. Digestion efficiency (% PSMs with zero missed cleavages AND tryptic both ends)
pct_fully_enzymatic = df.filter(pl.col("missed_cleavages") == 0).select(
    (pl.len() / total * 100).alias("pct_enzymatic")
)

# 3. Semi-tryptic fraction
# Sage labels semi-enzymatic peptides -- inspect the peptide termini
# against trypsin rules on the stripped_peptide column
```

For the semi-tryptic classification, Sage itself doesn't output an explicit `is_semi_tryptic` boolean column — you need to annotate it yourself from the `peptide` (or `stripped_peptide`) sequence by checking both termini against the trypsin rule (does the residue before the N-terminus = K or R? does the C-terminal residue = K or R and not followed by P?). This is a straightforward string operation on the sequence column. [sage-docs.vercel](https://sage-docs.vercel.app/docs/results)

---

## Signal-Weighted vs. Count-Based Metrics

PSM counting gives you a first-pass estimate, but **signal-weighted (precursor intensity) metrics are more accurate** because a single abundant semi-tryptic peptide can dominate sample content even if it's one of few PSMs. This is precisely the approach SPACEPro takes: it extracts ion chromatograms for each unique peptide precursor and computes the fraction of total precursor signal contributed by MC, non-specific (NS), and fully enzymatic peptides. [pubs.acs](https://pubs.acs.org/doi/10.1021/acs.jproteome.0c00928)

> **Digestion efficiency (signal-weighted)** = (sum of precursor areas for fully enzymatic peptides) / (total precursor area for all identified peptides)

SPACEPro showed that PSM-level MC rates are consistently *higher* than signal-weighted rates — for instance, reporting 19% MC at PSM level vs. 15% by signal in the same dataset. For biofluid work where you expect a lot of semi-tryptic signal, using intensity-weighted fractions will give a more honest picture. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC8026743/)

---

## Practical Strategy: One Search or Multiple?

**One search is the right answer** for discovery. Set `semi_enzymatic: true` and `missed_cleavages: 2` (or 3 for plasma). The single search gives you:

1. Fully tryptic peptides with 0 MC → your "clean digest" fraction
2. Peptides with ≥1 MC → missed cleavage fraction
3. Peptides flagged semi-enzymatic → ragged/semi-tryptic fraction

The reason to *not* do a separate fully-tryptic + semi-tryptic search is that you'd need to reconcile FDR across searches, deal with redundant PSMs, and there's no gain in sensitivity — Sage handles the semi-enzymatic generation internally before building the fragment index. [sage-docs.vercel](https://sage-docs.vercel.app/docs/configuration/database)

The **two-search workflow** is only useful if you want to benchmark sensitivity impact: run once fully-tryptic to establish your baseline ID rate, then run semi-enzymatic and compare what's newly identified. This also helps you quantify how much of your biofluid proteome is genuinely semi-tryptic vs. artifact. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC8162549/)

---

## Biofluid-Specific Considerations

In plasma, serum, urine, or CSF, semi-tryptic peptides have a dual origin: [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC3753091/)

1. **Biological**: circulating proteases (e.g., plasmin, thrombin, complement factors, kallikreins) cleave proteins *in vivo* before your sample ever sees trypsin. These represent true biology — protease activity, protein processing, shedding.
2. **Technical**: in-solution over-digestion, chymotryptic activity in porcine trypsin preparations, and sample handling artifacts.

Porcine trypsin specifically is known to produce more semi-tryptic peptides than bovine trypsin (which produces more missed cleavages instead). At your 10% missed-cleavage prior for porcine trypsin, you'd expect your digestion efficiency (signal-weighted, fully enzymatic) to sit around 85–90% for a clean cell lysate, but this can drop meaningfully in complex biofluids where the protein matrix is harder to denature/digest and endogenous proteolysis is already ongoing. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC4076643/)

---

## Recommended Post-Processing Workflow

1. **Run Sage** with `semi_enzymatic: true`, `missed_cleavages: 2`, `cleave_at: "KR"`, `restrict: "P"`
2. **Filter `results.sage.tsv`** to `peptide_q ≤ 0.01`, `label == 1`
3. **Annotate termini** of `stripped_peptide` against trypsin rules → classify each PSM as fully tryptic, semi-tryptic (N-ragged or C-ragged), or non-tryptic
4. **Compute PSM-level fractions**: `%MC`, `%semi-tryptic`, `%fully enzymatic`
5. **Optionally use SPACEPro** (part of the Trans-Proteomic Pipeline) for signal-weighted metrics — it reads pepXML, extracts precursor XICs, and outputs MC%, NS%, and enzymatic% by signal at PSM, peptide, and protein levels. It requires converting your Sage output to pepXML format (e.g., via Sage's `.pin` file + Percolator, or direct pepXML export if using a wrapper). [pubs.acs](https://pubs.acs.org/doi/10.1021/acs.jproteome.0c00928)

---

## Key Insight for Recon Tool

**Without `semi_enzymatic: true` in the search config, Sage will not find semi-tryptic peptides at all.** The `semi_enzymatic` column in the output will always be 0 because the search space didn't include semi-enzymatic candidates.

This is a search-time decision, not a post-processing one. If you want to measure digestion efficiency including semi-tryptic rate, you must enable `semi_enzymatic: true` when running Sage.

The tradeoff is increased search time (larger candidate space) and potentially lower scores for borderline PSMs (more competition). For a reconnaissance tool, this tradeoff may be acceptable since we're scouting one file, not running production-scale searches.

---

## Why Local Semi-Tryptic Search Is So Painful

The problem is combinatorial. For a fully tryptic search, the database is digested *in silico* at defined K/R sites, yielding a fixed, manageable peptide candidate list. When you switch to semi-tryptic, for every tryptic peptide of length *n*, you're now generating all *n-1* N-terminal truncations **plus** all *n-1* C-terminal truncations as additional candidates. For a typical human proteome, this inflates the candidate space by **~10–20×**. Then layer on missed cleavages and variable mods and you're in trouble fast. [sciencedirect](https://www.sciencedirect.com/science/article/pii/S153594762030092X)

Traditional search engines (the kind that iterate spectra → candidates sequentially, like early Mascot or Comet) do this naively: for each spectrum, score all candidates within the precursor mass window, which is now a massive list. Even with tight MS1/MS2 tolerances, the sheer number of semi-tryptic candidates dominates runtime and memory.

---

## What Fragment Ion Indexing Actually Does

This is the key architectural shift that makes tools like Sage, MSFragger, and now Comet-FI fast for semi-tryptic work. Instead of the loop being **spectrum → candidate list → score all**, it's **inverted**:

1. **Build time (once per database):** every candidate peptide (tryptic, semi-tryptic, MC variants) is *in silico* fragmented into b/y ions. Those fragment masses are inserted into a sorted index — Sage uses a **B+tree** (fragment mass as outer key, precursor mass as inner key). MSFragger uses a similar hash-based fragment ion index. [youtube](https://www.youtube.com/watch?v=1rG-ZE3Z0Xc)
2. **Search time (per spectrum):** instead of "score peptide A against this spectrum, then B, then C…", you take each observed fragment peak from the spectrum and do a single **index lookup** to find all candidate peptides that could produce that fragment mass within your MS2 tolerance. You accumulate hits per candidate and produce a score. The index lookup is O(log N), not O(N).

The result: the cost of expanding the semi-tryptic search space is paid once at index build time (which is parallelized across all CPU cores), and spectrum matching itself is nearly independent of how large the candidate space is. Sage's fragment index gives ~100× speedup over sequential matching; the Rust implementation adds another ~25× on top of that. [pubs.acs](https://pubs.acs.org/doi/10.1021/acs.jproteome.4c01094)

---

## Sage Specifically

Sage's fragment index is built in RAM before any spectra are processed, and the entire search is parallelized via Rust's `rayon` work-stealing thread pool — it uses every core you have automatically. For semi-tryptic, the index just gets larger (more entries), but the *per-spectrum matching cost* barely increases because you're still doing the same index lookup pattern. The bottleneck shifts from search time to **index build time and RAM**, which is why very large FASTAs + semi-enzymatic + MCs + variable mods can still blow your memory budget locally. [github](https://github.com/lazear/sage)

The RAM issue is real and acknowledged — there's even a community Sage branch that chunks the FASTA for unspecific/non-enzymatic searches to work around this. [reddit](https://www.reddit.com/r/proteomics/comments/1j96gqf/peptidomics_data_processing_and_analysis/)

---

## How Commercial/Cloud Tools Handle It

**Byonic** does something architecturally different: it uses a "candidate peptide pruning" approach where it constrains candidates using the precursor mass *first* (a mass-bucket filter), then scores only survivors. It also parallelizes across cores and benefits from highly optimized C++ scoring. It doesn't publish its exact index implementation, but the principle is the same: avoid the naive one-by-one candidate scoring loop. [proteinmetrics](https://www.proteinmetrics.com/products/byonic)

**Chaparral's SageDDA** (the cloud wrapper you mentioned) simply runs Sage on cloud hardware (many more cores, much more RAM) — it's the same algorithm, just with 64+ cores and 256+ GB RAM so the index build and search are trivially fast. [chaparral](https://www.chaparral.ai/products/sagedda.html)

**MSFragger** (FragPipe) is the academic tool most comparable in semi-tryptic performance to Sage, using the same fragment-ion indexing concept first published in 2017. On a quad-core workstation it handles open searches (500 Da window!) in under 10 minutes for a single run  — semi-tryptic with narrow tolerances is much lighter than that. [nesvilab](https://www.nesvilab.org/software.html)

---

## Practical Tips for Your Local Sage Setup

The semi-tryptic RAM/time crunch on local hardware is manageable:

- **Pre-filter your FASTA** — run a fully tryptic search first, collect identified proteins, then run semi-tryptic against only that subset protein list. This is the standard "two-pass" approach and dramatically shrinks the index. [reddit](https://www.reddit.com/r/proteomics/comments/1j96gqf/peptidomics_data_processing_and_analysis/)
- **Reduce `max_variable_mods`** — variable mods multiply candidates just as badly as semi-enzymatic does; cut them to the minimum needed.
- **Increase `min_len`** — semi-tryptic searches generate huge numbers of very short peptides. Setting `min_len: 8` or `9` prunes most of the junk and keeps memory in check.
- **Cap `missed_cleavages` at 1** for the semi-tryptic run — the MC × semi-enzymatic combination is the real killer. For biofluids you'll catch the bulk of biological semi-tryptic signal with 1 MC.
- **Consider running semi-tryptic on the targeted protein list only** (e.g., known plasma proteins, high-abundance proteins) and letting the tryptic search cover the long tail.

---

## Why Tools Make It Look Easy

Tools like Proteome Discoverer, Progenesis, and FragPipe report digestion efficiency and % semi-tryptic because they are doing the two-pass logic **internally and silently**. They run a permissive search under the hood, then post-process the PSM table against trypsin rules before surfacing a clean QC metric in a dashboard. You never see the intermediate steps. When you try to replicate it manually with Sage + a single config, you're doing what those tools hide from you — and hitting the resource wall directly. [nonlinear](https://www.nonlinear.com/progenesis/qi-for-proteomics/v4.2/faq/qc-metrics/missed-cleavages.aspx)

---

## The Two-Pass Approach Does Give You the Metrics

Here's the concrete flow:

**Pass 1 — Fully tryptic, permissive MC:**
```json
"semi_enzymatic": false,
"missed_cleavages": 2,
"cleave_at": "KR",
"restrict": "P"
```
Filter to `peptide_q ≤ 0.01`, `label == 1`. From this output alone you get:

- **Missed cleavage distribution** — directly from the `missed_cleavages` column in `results.sage.tsv` [sage-docs.vercel](https://sage-docs.vercel.app/docs/results)
- **Digestion efficiency (count-based)** — `% PSMs with missed_cleavages == 0`
- **Digestion efficiency (signal-weighted)** — sum of precursor intensities for `missed_cleavages == 0` / total precursor intensity [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC8026743/)

**Pass 2 — Semi-enzymatic, against the Pass 1 protein list only:**
```json
"semi_enzymatic": true,
"missed_cleavages": 1,
"cleave_at": "KR",
"restrict": "P"
```
Restrict your FASTA to only proteins identified in Pass 1 (a few hundred proteins rather than the full proteome). Then annotate each PSM's `stripped_peptide` for terminus specificity. From this you get:

- **% semi-tryptic** — PSMs (or intensity) where one terminus is non-enzymatic
- **N-ragged vs. C-ragged breakdown** — tells you whether endogenous N-terminal or C-terminal proteases dominate [freidok.uni-freiburg](https://freidok.uni-freiburg.de/files/256539/lXwBVsxJjyAD6qOG/Proteomics_2024_Cosenza-Contreras_TermineR.pdf)

The metrics you're after are entirely derivable from these two TSV files with ~30 lines of Pandas/Polars. The terminus annotation is the only "manual" piece — checking if the residue preceding the peptide N-terminus in the protein sequence is K or R, and whether the C-terminal residue is K or R not followed by P.

---

## The Real Reason Replication Is a Headache

There are a few specific friction points that nobody documents well:

1. **Terminus annotation requires the protein sequence context**, not just the peptide sequence. You need the FASTA to look up what sits before and after each peptide hit to classify it as tryptic/semi/non-tryptic. Sage doesn't output this directly. You have to join on the protein accession + peptide sequence. [sage-docs.vercel](https://sage-docs.vercel.app/docs/results)

2. **FDR is not stratified by specificity class.** A single 1% FDR cut on a mixed tryptic+semi-tryptic result set is not the same as 1% FDR within each class. Semi-tryptic PSMs have worse score distributions and their contamination at a given q-value cutoff is higher. Tools like FragPipe handle this with class-specific FDR. If you don't do this, your semi-tryptic fraction is inflated by false positives — which is why replication numbers often look different from published values. [pmc.ncbi.nlm.nih](https://pmc.ncbi.nlm.nih.gov/articles/PMC3753091/)

3. **Signal-weighted metrics need precursor XICs**, which Sage does output (`fragment_pct`, `intensity` in the results) but wiring it to per-PSM precursor area requires either Sage's quantification output or an external tool like FlashLFQ. [sage-docs.vercel](https://sage-docs.vercel.app/docs/results)

4. **The denominator matters enormously.** Is digestion efficiency calculated over all PSMs, all unique peptides, all unique precursors, or by signal? Each gives a different number and tools rarely say which they use. [mcponline](https://www.mcponline.org/article/S1535-9476(20)31519-X/fulltext)

---

## Simplest Path to Get the Numbers

If you just want the three metrics reliably without the full replication pain, the most practical route right now is:

- **Pass 1 Sage (tryptic, 2MC)** → compute MC distribution and signal-weighted digestion efficiency directly from `results.sage.tsv` + precursor intensities
- **Pass 2 Sage (semi-enzymatic, 1MC, subset FASTA)** → annotate termini with a small Python helper that uses `pyteomics.fasta` or Biopython to look up flanking residues in the FASTA, classify each PSM, and compute % semi-tryptic by count and by intensity

This is reproducible, transparent, and you own the denominator definition — which is ultimately the right way to report it for a biofluid study where the biology of ragged ends is part of the story. [pubmed.ncbi.nlm.nih](https://pubmed.ncbi.nlm.nih.gov/35156369/)
