## Pass-1 MS2 tolerance — the full table

**Five constants** (`recon-tool/src/mzml.rs`), all curated, none derived:

| class | constant | value | doc comment says |
|---|---|---|---|
| Orbitrap / FT-ICR | `ORBITRAP_MS2_HALF_WIDTH_PPM` | **±20 ppm** | "typical 1–5 ppm, up to ~20 poorly calibrated" |
| Astral | `ASTRAL_MS2_HALF_WIDTH_PPM` | **±20 ppm** | "typical <5 ppm RMS external, ~3 ppm internal" |
| Legacy TOF / QTOF | `LEGACY_TOF_MS2_HALF_WIDTH_PPM` | **±100 ppm** | "typical ~10–30 ppm, and 30 ppm is already a common default" |
| Ion trap / quadrupole | `ION_TRAP_MS2_HALF_WIDTH_DA` | **±1.0 Da** | "typical 0.3–0.8 Da at unit resolution" |
| unknown | `UNKNOWN_MS2_FALLBACK_PPM` | **±20 ppm** | — |

**CV term → bucket** (children of MS:1000443):

| accession | CV name | tolerance | source |
|---|---|---|---|
| MS:1000484 | orbitrap | ±20 ppm | curated |
| MS:1000079 | FT-ICR | ±20 ppm | extended |
| MS:1003379 | asymmetric track lossless TOF (Astral) | ±20 ppm | curated |
| MS:1000084 | time-of-flight | ±100 ppm | curated |
| MS:1000082 | quadrupole ion trap | ±1 Da | curated |
| MS:1000078 | axial ejection linear ion trap | ±1 Da | curated |
| MS:1000083 | radial ejection linear ion trap | ±1 Da | curated |
| MS:1000264 | ion trap | ±1 Da | extended |
| MS:1000291 | linear ion trap | ±1 Da | extended |
| MS:1000081 | quadrupole | ±1 Da | extended |
| MS:1000080 | magnetic sector | **fallback ±20 ppm** | extended |
| MS:1000254 | electrostatic energy analyzer | **fallback ±20 ppm** | extended |

**Thermo filter-string tokens** (MS:1000512) — the *preferred* signal:

`FTMS`→±20 ppm · `ITMS`→±1 Da · `TOFMS`→±100 ppm · `ASTMS`→±20 ppm · `TQMS`→±1 Da · `SQMS`→±1 Da · `SECTOR`→fallback ±20 ppm

## Detection and fallback logic

**Precedence:** per-scan filter string (MS:1000512) → `instrumentConfiguration` analyzer term → nothing.

**Four fallback paths**, all landing on ±20 ppm with `assumed: true`:

1. **MS2 detector changes mid-run** (`is_mixed`, >1 class present) → `DetectorSwitched`. One search runs at one tolerance, so no value is correct.
2. **Class detected but has no bucket** (magnetic sector, electrostatic) → `Unknown`
3. **No MS2 analyzer readable at all** → `Unknown`
4. Otherwise `dominant()` — the most common class — wins, `basis: Detected`, `assumed: false`

**Recon never refuses to search on analyzer grounds.**

## Four things I'd check if I were you

**1. `LEGACY_TOF = 100 ppm` has exactly the bug I just fixed on Orbitrap.** Its own comment says "typical ~10–30 ppm, and 30 ppm is already a common default" — and it's set to **100**, 3.3× that. Same pattern: constant contradicting its documented bound. Orbitrap at 50→20 bought +12–17 % PSMs and 35–41 % less wall clock; there's no reason TOF wouldn't behave the same. **We have no TOF file to measure it on**, so I didn't touch it.

**2. The fallback now equals the Orbitrap bucket.** It used to be tighter (20 vs 50). An unreadable file is now searched *as if it were an Orbitrap* — right for Thermo data, wrong for an unreadable ion-trap file that needs ~1 Da. I just corrected the stale comment claiming it was deliberately tighter.

**3. Detection reads only the first 100 MS2 scans** (`DEFAULT_ANALYZER_SAMPLE`). Sound because acquisition methods don't switch detector mid-run — but a file that *does* switch late won't be caught. Recorded as a known limitation.

**4. Two "extended" assignments are judgement calls worth a second opinion:** MS:1000081 `quadrupole` → ±1 Da (right for a triple quad, arguable elsewhere), and `magnetic sector` / `electrostatic energy analyzer` → no bucket at all, despite magnetic sector being a high-resolution analyzer.

