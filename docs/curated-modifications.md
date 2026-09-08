# Curated modifications

This is the modification list `recon` uses to name a delta mass and to say
which residues can carry it. It is compiled into the binary, so a release
needs no extra file to annotate a peak.

**99 entries.** Generated from the four files the binary embeds.
Do not edit this document by hand: it is produced by
`_dev/testing/scripts/gen_curated_mods_doc.py`, which reads those same files.

## Where the list comes from

The list is a dated snapshot of the curated modification files from
[MetaMorpheus](https://github.com/smith-chem-wisc/MetaMorpheus) (MIT licence),
taken at commit `7e453540`. We adopted it rather than using all of Unimod
because Unimod is a catalogue of everything ever reported, while this list is
a working set with acceptor residues already curated. Attribution and the one
modified file are recorded in `THIRD_PARTY_LICENSES.md`.

Masses are not stored in the source files. We compute each one from the
entry's chemical formula using the element table in the bundled `unimod.xml`,
which is the same rule the tool applies at run time.

| Source file | Entries |
|---|---:|
| `Mods.txt` | 93 |
| `aListOfmods.txt` | 3 |
| `ProteaseMods.txt` | 1 |
| `surfactants.txt` | 2 |

## Categories

The category is MetaMorpheus's own `MT` field, carried through unchanged.
It is an inherited label, not a measurement `recon` makes.

| Category | Entries |
|---|---:|
| Less Common | 43 |
| Common Biological | 29 |
| Metal | 8 |
| Common Artifact | 6 |
| Speculative | 3 |
| Common Fixed | 2 |
| Trypsin Digested | 2 |
| Surfactant | 2 |
| AspN Digested | 2 |
| Protease | 1 |
| Common Variable | 1 |

## The full list

Sorted by monoisotopic mass, the order the tool holds them in. `Sites` is the
set of residues the entry accepts; `X` means any residue, and the position
column then carries the restriction (for example a protein N-terminus).

| Δ mass (Da) | Name | Sites | Position | Category | Formula |
|---:|---|---|---|---|---|
| -131.0405 | Met-loss | X | Protein N-terminal, Met loss. | Common Biological | `C-5 H-9 N-1 O-1 S-1` |
| -117.0248 | Met-loss+Methylation | X | Protein N-terminal, Met loss. | Less Common | `C-4 H-7 N-1 O-1 S-1` |
| -89.0299 | Met-loss+Acetylation | X | Protein N-terminal, Met loss. | Common Biological | `C-3 H-7 N-1 S-1` |
| -48.0034 | Homoserine lactone | M | Peptide C-terminal. | Protease | `C-1 H-4 S-1` |
| -33.9877 | Dehydroalanine | C | Anywhere. | Less Common | `H-2 S-1` |
| -32.0085 | Oxidation and then loss of oxidized M side chain | M | Anywhere. | Less Common | `H-4 C-1 S-1 O1` |
| -31.0244 | Met-loss+Succinylation | X | Protein N-terminal, Met loss. | Less Common | `C-1 H-5 N-1 O2 S-1` |
| -30.0106 | Pyrrolidinone | P | Anywhere. | Less Common | `H-2 C-1 O-1` |
| -30.0106 | Decarboxylation | D or E | Anywhere. | Less Common | `H-2 C-1 O-1` |
| -18.0106 | Water Loss (Glu->pyro-Glu) | E | Peptide N-terminal. | Common Artifact | `H-2 O-1` |
| -18.0106 | Water loss | D | Anywhere. | Less Common | `H-2 O-1` |
| -18.0106 | Dehydroalanine | S | Anywhere. | Less Common | `H-2 O-1` |
| -18.0106 | Dehydrobutyrine | T | Anywhere. | Less Common | `H-2 O-1` |
| -17.0265 | Gln->pyro-Glu | Q | Peptide N-terminal. | Common Biological | `H-3 N-1` |
| -17.0265 | Ammonia loss | C | Peptide N-terminal. | Common Artifact | `H-3 N-1` |
| -17.0265 | Ammonia loss | N | Anywhere. | Common Artifact | `H-3 N-1` |
| -15.9949 | Reduction | D or S or T | Anywhere. | Less Common | `O-1` |
| -2.0157 | Didehydro | Y | Anywhere. | Less Common | `H-2` |
| -2.0157 | H-2 | M | Peptide N-terminal. | Speculative | `H-2` |
| -0.9840 | Amidation | X | Peptide C-terminal. | Less Common | `H1 N1 O-1` |
| 0.9840 | Citrullination | R | Anywhere. | Common Biological | `H-1 N-1 O1` |
| 0.9840 | Deamidation | N or Q | Anywhere. | Common Artifact | `H-1 N-1 O1` |
| 3.9949 | Oxidation to Kynurenine | W | Anywhere. | Less Common | `C-1 O1` |
| 12.0000 | Proline pyrrole to pyrrolidine six member ring | P | Anywhere. | Less Common | `C1` |
| 12.0000 | Carbon Adduct | X | Peptide N-terminal. | Speculative | `C1` |
| 13.9793 | H-2 O1 | L or I or P or W or S | Anywhere. | Speculative | `H-2 O1` |
| 14.0157 | Methylation | K or R | Anywhere. | Common Biological | `H2 C` |
| 14.0157 | Methylation | C or H or N or Q or I or L or D or E or S or T | Anywhere. | Less Common | `H2 C` |
| 14.0157 | Methylation | X | Protein N-terminal. | Less Common | `H2 C` |
| 14.0157 | Methylation | X | C-terminal. | Less Common | `H2 C` |
| 15.9949 | Hydroxylation | K or N | Anywhere. | Common Biological | `O1` |
| 15.9949 | Hydroxylation | P | Anywhere. | Common Biological | `O1` |
| 15.9949 | Oxidation | Y or W or F or D or E or V or H or R or C or U or I or L or Q or S or T | Anywhere. | Less Common | `O1` |
| 15.9949 | Oxidation on M | M | Anywhere. | Common Variable | `O1` |
| 21.9694 | Magnesium | D or E | Anywhere. | Metal | `H-2 Mg1` |
| 21.9819 | Sodium | D or E | Anywhere. | Metal | `H-1 Na1` |
| 27.9949 | Formylation | K | Anywhere. | Common Biological | `C1 O1` |
| 27.9949 | Formylation | X | Peptide N-terminal. | Less Common | `C1 O1` |
| 27.9949 | Formylation | S or T | Anywhere. | Less Common | `C1 O1` |
| 28.0313 | Dimethylation | K | Anywhere. | Common Biological | `H4 C2` |
| 28.0313 | Dimethylation | R | Anywhere. | Common Biological | `H4 C2` |
| 28.0313 | Ethylation | K or E or D | Anywhere. | Less Common | `H4 C2` |
| 28.0313 | Ethylation | X | Peptide N-terminal. | Less Common | `H4 C2` |
| 28.0313 | Dimethylation | N | Anywhere. | Less Common | `H4 C2` |
| 28.9902 | Nitrosylation | C | Anywhere. | Common Biological | `H-1 N1 O1` |
| 29.9742 | Quinone | Y or W | Anywhere. | Less Common | `H-2 O2` |
| 31.9898 | Dioxidation | W or P or R or K or M or F or Y or C | Anywhere. | Less Common | `O2` |
| 37.9469 | Calcium | D or E | Anywhere. | Metal | `H-2 Ca1` |
| 37.9559 | Potassium | D or E | Anywhere. | Metal | `H-1 K1` |
| 42.0106 | Acetylation | X | Protein N-terminal. | Common Biological | `H2 C2 O1` |
| 42.0106 | Acetylation | K | Anywhere. | Common Biological | `H2 C2 O1` |
| 42.0106 | Acetylation | S or T | Anywhere. | Less Common | `H2 C2 O1` |
| 42.0470 | Trimethylation | K | Anywhere. | Common Biological | `H6 C3` |
| 43.0058 | Carbamyl | X | Peptide N-terminal. | Common Artifact | `H1 C1 N1 O1` |
| 43.0058 | Carbamyl | K or R or C or M | Anywhere. | Common Artifact | `H1 C1 N1 O1` |
| 43.9898 | Carboxylation | K or D or E | Anywhere. | Common Biological | `C1 O2` |
| 44.9851 | Nitrosylation | Y | Anywhere. | Common Biological | `H-1 N1 O2` |
| 47.9847 | Trioxidation | C | Anywhere. | Less Common | `O3` |
| 52.9115 | Fe[III] | D or E | Anywhere. | Metal | `H-3 Fe1` |
| 53.9193 | Fe[II] | D or E | Anywhere. | Metal | `H-2 Fe1` |
| 56.0262 | Propionylation | K | Anywhere. | Less Common | `C3 H4 O1` |
| 57.0215 | Carbamidomethyl | K or H or D or E or S or T or Y | Anywhere. | Less Common | `H3 C2 N1 O1` |
| 57.0215 | Carbamidomethyl | X | Peptide N-terminal. | Less Common | `H3 C2 N1 O1` |
| 57.0215 | Carbamidomethyl on C | C | Anywhere. | Common Fixed | `H3 C2 N O` |
| 57.0215 | Carbamidomethyl on U | U | Anywhere. | Common Fixed | `H3 C2 N O` |
| 58.0055 | Carboxymethylation | X | Peptide N-terminal. | Less Common | `H2 C2 O2` |
| 58.0055 | Carboxymethylation | C or K or W | Anywhere. | Less Common | `H2 C2 O2` |
| 61.9135 | Zinc | D or E | Anywhere. | Metal | `H-2 Zn1` |
| 61.9218 | Cu[I] | D or E | Anywhere. | Metal | `H-1 Cu1` |
| 68.0262 | Crotonylation | K | Anywhere. | Common Biological | `C4 H4 O1` |
| 70.0419 | Butyrylation | K | Anywhere. | Common Biological | `C4 H6 O1` |
| 71.0133 | Lactylation | K | Anywhere. | Less Common | `H3 C3 O2` |
| 71.0371 | Propionamidation | C or K | Anywhere. | Less Common | `H5 C3 N1 O1` |
| 71.0371 | Propionamidation | X | Peptide N-terminal. | Less Common | `H5 C3 N1 O1` |
| 79.1579 | Met-loss+Myristoylation | G | Protein N-terminal, Met loss. | Less Common | `C9 H17 N-1 S-1` |
| 79.9568 | Sulfonation | Y | Anywhere. | Common Biological | `O3 S1` |
| 79.9568 | Sulfonation | S or T | Anywhere. | Less Common | `O3 S1` |
| 79.9663 | Phosphorylation | S or T | Anywhere. | Common Biological | `H1 O3 P1` |
| 79.9663 | Phosphorylation | Y | Anywhere. | Common Biological | `H1 O3 P1` |
| 86.0004 | Malonylation | K | Anywhere. | Common Biological | `C3 H2 O3` |
| 86.0368 | Hydroxybutyrylation | K | Anywhere. | Common Biological | `C4 H6 O2` |
| 100.0160 | Succinylation | K | Anywhere. | Common Biological | `C4 H4 O3` |
| 100.0160 | Succinylation | X | Protein N-terminal. | Less Common | `C4 H4 O3` |
| 114.0317 | Glutarylation | K | Anywhere. | Common Biological | `C5 H6 O3` |
| 114.0429 | GG (Ubiquitination Site) | K | Anywhere. | Trypsin Digested | `H6 C4 N2 O2` |
| 203.0794 | HexNAc | Nxs or Nxt | Anywhere. | Common Biological | `C8H13NO5` |
| 203.0794 | HexNAc | S or T | Anywhere. | Common Biological | `C8H13NO5` |
| 204.1878 | Farnesylation | C | Anywhere. | Less Common | `H24 C15` |
| 210.1984 | Myristoylation | G | Protein N-terminal. | Less Common | `H26 C14 O1` |
| 210.1984 | Myristoylation | C or K | Anywhere. | Less Common | `H26 C14 O1` |
| 226.0776 | Biotinylation | K | Anywhere. | Less Common | `C10 H14 N2 O2 S1` |
| 229.0140 | Pyridoxal phosphate | K | Anywhere. | Common Biological | `C8 H8 N1 O5 P1` |
| 238.2297 | Palmitoylation | K or S or T or C | Anywhere. | Less Common | `H30 C16 O1` |
| 266.1518 | Hydrogen Dodecyl Sulfate | D | Anywhere. | Surfactant | `H22 C15 O4` |
| 294.1831 | Hydrogen Tetradecyl Sulfate | D | Anywhere. | Surfactant | `H26 C17 O4` |
| 484.2282 | EQIGG (sumoylation (SMT-3) Site yeast) | K | Anywhere. | Trypsin Digested | `C20 H32 N6 O8` |
| 541.0611 | ADP-ribosylation | S | Anywhere. | Common Biological | `C15 H21 N5 O13 P2` |
| 960.4301 | DVFQQQTGG (SUMO-2/3 Site human) | D | Anywhere. | AspN Digested | `C41 H60 N12 O15` |
| 1318.6041 | DVIEVYQEQTGG (SUMO-1 Site human) | D | Anywhere. | AspN Digested | `C57 H86 N14 O22` |
