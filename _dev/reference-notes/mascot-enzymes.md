# Mascot enzyme rules (cited source)

**Source:** <https://www.matrixscience.com/help/enzyme_help.html>
**Supplied by:** Ben, 2026-09-01, pasted into the session as the authority for
recon's `--enzyme` presets.
**Status:** a dated snapshot, like every other reference input here. A reader
re-reading the page today may see drift; that is expected.

This is the table recon's preset list in `recon-tool/src/enzyme.rs` is built
from. `Cleave` is the residue set, `Don't cleave` is the restriction applied to
the residue at the cleavage boundary, and `N or C term` is which side of the
matched residue the cut falls on.

| Name | Cleave | Don't cleave | N or C term |
|---|---|---|---|
| Trypsin | KR | P | C |
| Trypsin/P | KR | | C |
| Arg-C | R | P | C |
| Asp-N | BD | | N |
| Asp-N_ambic | DE | | N |
| Chymotrypsin | FYWL | P | C |
| CNBr | M | | C |
| CNBr+Trypsin | M | | C |
| | KR | P | C |
| Formic_acid | D | | C |
| | D | | N |
| Lys-C | K | P | C |
| Lys-C/P | K | | C |
| LysC+AspN | K | P | C |
| | DB | | N |
| Lys-N | K | | N |
| NoCleave | see notes | | |
| PepsinA | FL | | C |
| semiTrypsin | see notes | | |
| TrypChymo | FYWLKR | P | C |
| TrypsinMSIPI | KR | P | C |
| | J | | C |
| | J | | N |
| TrypsinMSIPI/P | KR | | C |
| | J | | C |
| | J | | N |
| V8-DE | BDEZ | P | C |
| V8-E | EZ | P | C |
| None | see notes | | |

## What recon takes from this, and what it cannot

**Scope (Ben, 2026-09-01): the common proteases only.** Anything else is reachable
with an explicit rule on the command line, so the preset table does not mirror
the whole page.

**Ambiguity codes are not representable.** Sage's `VALID_AA`
(`crates/sage/src/mass.rs`, pinned commit) is the 20 standard residues plus `U`
and `O`. It excludes `B` (Asx), `Z` (Glx), `J` and `X`. Sage validates
`cleave_at` with `assert!`, not a `Result`, so since Sage became a linked library
an ambiguity code would ABORT the process. recon rejects them first, with a
message. The affected rows are dropped to their representable core:

| Mascot | Mascot cleave | recon preset | recon cleave |
|---|---|---|---|
| Asp-N | `BD` | `asp-n` | `D` |
| V8-E | `EZ` | `glu-c` | `E` |
| V8-DE | `BDEZ` | `glu-c/de` | `DE` |
| Asp-N_ambic | `DE` | `asp-n/ambic` | `DE` (exact, no deviation) |

**Buffer-dependent proteases ship as PAIRS, following Mascot's own split.**
Glu-C/V8 cleaves after E in phosphate buffer and after both D and E in ammonium
bicarbonate; Asp-N cleaves before D, or before D and E in ambic. Mascot encodes
each as two rows and recon keeps that split, because collapsing them to one name
would silently choose a buffer condition for the user. Verified against the
source URL and the literature by Ben, 2026-09-01.

B and Z appear in essentially no modern FASTA, so the search result is unchanged.

**Multi-rule enzymes cannot be expressed at all.** `CNBr+Trypsin`, `LysC+AspN`,
`Formic_acid`, `TrypsinMSIPI` and `TrypsinMSIPI/P` each occupy two or three rows,
mixing N- and C-terminal cleavage. Sage's config carries ONE
(`cleave_at`, `restrict`, `c_terminal`) triple, so this is a Sage limit, not a
recon one.

**`NoCleave`, `semiTrypsin` and `None`** are Mascot modes rather than rules.
Sage's equivalents are `cleave_at ""` (non-specific) and `"$"` (no digestion),
both of which recon refuses for the digestion report because neither has a
cleavage rule to classify termini with.
