#!/usr/bin/env python3
"""
Phase 6C: Digestion Efficiency Analysis Tool

Two-pass workflow for assessing digestion efficiency:
1. subset: Extract proteins from Pass 1 results, create subset FASTA
2. annotate: Classify peptide termini from Pass 2 results

Usage (paths relative to testing/):
    # Step 1: Subset FASTA to identified proteins
    python scripts/digestion_efficiency.py subset \
        --tsv search-output/digestion-pass1/results.sage.tsv \
        --fasta inputs/UniProt-Human-UP000005640_canonical-2023_05.fasta \
        --output inputs/subset_identified_proteins.fasta

    # Step 2: Annotate termini and compute digestion score
    python scripts/digestion_efficiency.py annotate \
        --tsv search-output/digestion-pass2/results.sage.tsv \
        --fasta inputs/subset_identified_proteins.fasta \
        --output search-output/annotated_termini.tsv \
        --output-json search-output/digestion_efficiency_result.json

Note: Sage handles decoy generation internally, so the subset FASTA only
needs target sequences. Decoys are generated on-the-fly during the search.

Requires: pyteomics (pip install pyteomics)
"""

import argparse
import csv
import json
import re
from collections import defaultdict
from pathlib import Path

try:
    from pyteomics import fasta
    HAS_PYTEOMICS = True
except ImportError:
    HAS_PYTEOMICS = False


# =============================================================================
# Shared utilities
# =============================================================================

def extract_accession(header: str) -> str:
    """
    Extract accession from FASTA header.
    Returns the full UniProt format: sp|P12345|GENE_HUMAN
    This matches what Sage outputs in the proteins column.
    """
    header = header.lstrip('>')
    return header.split()[0]


def strip_modifications(peptide: str) -> str:
    """
    Remove modification annotations from peptide sequence.
    Handles formats like: [+42.011]-PEPTIDE or PEPT[+15.995]IDE
    """
    stripped = re.sub(r'\[.*?\]', '', peptide)
    stripped = re.sub(r'[^A-Z]', '', stripped.upper())
    return stripped


# =============================================================================
# Subset command
# =============================================================================

def extract_accessions_from_tsv(tsv_path: str, q_threshold: float) -> set:
    """
    Extract unique protein accessions from Sage TSV.
    Filters to target PSMs (label=1) at q-value threshold.
    """
    accessions = set()
    
    with open(tsv_path, 'r', newline='', encoding='utf-8') as f:
        reader = csv.DictReader(f, delimiter='\t')
        
        for row in reader:
            if row.get('label') != '1':
                continue
            
            try:
                peptide_q = float(row.get('peptide_q', 1.0))
            except ValueError:
                continue
                
            if peptide_q > q_threshold:
                continue
            
            proteins = row.get('proteins', '')
            for protein in proteins.split(';'):
                protein = protein.strip()
                if protein:
                    accessions.add(protein)
    
    return accessions


def parse_fasta_entries(fasta_path: str):
    """
    Generator that yields (header, sequence) tuples from FASTA file.
    """
    header = None
    sequence_lines = []
    
    with open(fasta_path, 'r', encoding='utf-8') as f:
        for line in f:
            line = line.rstrip('\n\r')
            if line.startswith('>'):
                if header is not None:
                    yield header, ''.join(sequence_lines)
                header = line
                sequence_lines = []
            else:
                sequence_lines.append(line)
        
        if header is not None:
            yield header, ''.join(sequence_lines)


def subset_fasta(fasta_path: str, accessions: set, output_path: str) -> int:
    """
    Write subset FASTA containing only specified accessions.
    
    Note: Sage handles decoy generation internally, so we only need
    target sequences. Decoys are generated on-the-fly during the search.
    """
    entries_written = 0
    
    with open(output_path, 'w', encoding='utf-8') as out:
        for header, sequence in parse_fasta_entries(fasta_path):
            acc = extract_accession(header)
            
            if acc in accessions:
                out.write(f"{header}\n")
                for i in range(0, len(sequence), 60):
                    out.write(f"{sequence[i:i+60]}\n")
                entries_written += 1
    
    return entries_written


def cmd_subset(args):
    """Execute the subset command."""
    print(f"Reading PSMs from: {args.tsv}")
    accessions = extract_accessions_from_tsv(args.tsv, args.q_value)
    print(f"Found {len(accessions)} unique target protein accessions at q<={args.q_value}")
    
    print(f"Subsetting FASTA: {args.fasta}")
    entries_written = subset_fasta(args.fasta, accessions, args.output)
    
    print(f"Wrote {entries_written} entries to: {args.output}")
    print(f"  (Sage will generate decoys internally during Pass 2)")


# =============================================================================
# Annotate command
# =============================================================================

def load_fasta_sequences(fasta_path: str) -> dict:
    """
    Load FASTA file into dict: accession -> sequence.
    Uses pyteomics for robust parsing.
    """
    if not HAS_PYTEOMICS:
        print("Error: pyteomics required. Install with: pip install pyteomics")
        exit(1)
    
    sequences = {}
    
    for record in fasta.read(fasta_path):
        header = record.description if hasattr(record, 'description') else record[0]
        sequence = record.sequence if hasattr(record, 'sequence') else record[1]
        acc = extract_accession(header)
        sequences[acc] = sequence
    
    return sequences


def is_tryptic_nterm(peptide: str, protein_seq: str, start_pos: int) -> bool:
    """
    Check if peptide N-terminus is tryptic.
    
    Tryptic N-terminus means:
    - Peptide starts at protein N-terminus (position 0), OR
    - Residue before peptide is K or R (and not followed by P)
    """
    if start_pos == 0:
        return True
    
    if start_pos > 0:
        prev_residue = protein_seq[start_pos - 1]
        if prev_residue in ('K', 'R'):
            if peptide[0] == 'P':
                return False
            return True
    
    return False


def is_tryptic_cterm(peptide: str, protein_seq: str, end_pos: int) -> bool:
    """
    Check if peptide C-terminus is tryptic.
    
    Tryptic C-terminus means:
    - Peptide ends at protein C-terminus, OR
    - Peptide ends with K or R (and next residue is not P)
    """
    if end_pos >= len(protein_seq):
        return True
    
    last_residue = peptide[-1]
    if last_residue in ('K', 'R'):
        if end_pos < len(protein_seq) and protein_seq[end_pos] == 'P':
            return False
        return True
    
    return False


def find_peptide_position(peptide: str, protein_seq: str) -> tuple:
    """
    Find peptide position in protein sequence.
    Returns (start_pos, end_pos) or (None, None) if not found.
    """
    start = protein_seq.find(peptide)
    if start == -1:
        return None, None
    return start, start + len(peptide)


def classify_terminus(peptide: str, protein_seq: str) -> str:
    """
    Classify peptide terminus specificity.
    
    Returns one of:
    - 'fully_tryptic'
    - 'semi_n_ragged' (C-term tryptic, N-term not)
    - 'semi_c_ragged' (N-term tryptic, C-term not)
    - 'non_tryptic'
    - 'not_found' (peptide not in protein)
    """
    start_pos, end_pos = find_peptide_position(peptide, protein_seq)
    
    if start_pos is None:
        return 'not_found'
    
    n_tryptic = is_tryptic_nterm(peptide, protein_seq, start_pos)
    c_tryptic = is_tryptic_cterm(peptide, protein_seq, end_pos)
    
    if n_tryptic and c_tryptic:
        return 'fully_tryptic'
    elif c_tryptic and not n_tryptic:
        return 'semi_n_ragged'
    elif n_tryptic and not c_tryptic:
        return 'semi_c_ragged'
    else:
        return 'non_tryptic'


def cmd_annotate(args):
    """Execute the annotate command."""
    print(f"Loading FASTA: {args.fasta}")
    sequences = load_fasta_sequences(args.fasta)
    print(f"Loaded {len(sequences)} protein sequences")
    
    print(f"Processing PSMs from: {args.tsv}")
    
    # Counters
    counts = defaultdict(int)
    mc_counts = defaultdict(int)
    total_psms = 0
    filtered_psms = 0
    
    with open(args.tsv, 'r', newline='', encoding='utf-8') as infile, \
         open(args.output, 'w', newline='', encoding='utf-8') as outfile:
        
        reader = csv.DictReader(infile, delimiter='\t')
        
        fieldnames = reader.fieldnames + ['terminus_class']
        writer = csv.DictWriter(outfile, fieldnames=fieldnames, delimiter='\t')
        writer.writeheader()
        
        for row in reader:
            total_psms += 1
            
            if row.get('label') != '1':
                continue
            
            try:
                peptide_q = float(row.get('peptide_q', 1.0))
            except ValueError:
                continue
            
            if peptide_q > args.q_value:
                continue
            
            filtered_psms += 1
            
            # Track missed cleavages
            try:
                mc = int(row.get('missed_cleavages', 0))
                mc_counts[mc] += 1
            except ValueError:
                pass
            
            peptide = row.get('peptide', '')
            stripped_peptide = strip_modifications(peptide)
            proteins = row.get('proteins', '').split(';')
            
            protein_acc = proteins[0].strip() if proteins else ''
            protein_seq = sequences.get(protein_acc, '')
            
            if protein_seq:
                terminus_class = classify_terminus(stripped_peptide, protein_seq)
            else:
                terminus_class = 'protein_not_found'
            
            counts[terminus_class] += 1
            
            row['terminus_class'] = terminus_class
            writer.writerow(row)
    
    # Compute statistics
    semi_total = counts['semi_n_ragged'] + counts['semi_c_ragged']
    semi_pct = semi_total / filtered_psms * 100 if filtered_psms > 0 else 0
    n_ragged_pct = counts['semi_n_ragged'] / filtered_psms * 100 if filtered_psms > 0 else 0
    c_ragged_pct = counts['semi_c_ragged'] / filtered_psms * 100 if filtered_psms > 0 else 0
    
    # MC distribution as percentages
    mc_distribution = {}
    for mc, count in mc_counts.items():
        mc_distribution[mc] = count / filtered_psms * 100 if filtered_psms > 0 else 0

    # NOTE: no composite "digestion score" is computed. A single judgmental
    # score assumes a cell-culture tryptic digest and misreads biofluids —
    # e.g. serum's ~32% semi-tryptic is endogenous proteolysis, not a bad
    # digest. The tool can't know sample type, so it presents the raw
    # breakdown and lets the user judge. (See NOTES "Known permanent
    # limitations".)
    
    # Print summary
    print(f"\n{'='*60}")
    print(f"DIGESTION EFFICIENCY REPORT")
    print(f"{'='*60}")
    print(f"\nTotal PSMs in file: {total_psms}")
    print(f"PSMs after filtering (q<={args.q_value}, target): {filtered_psms}")
    
    print(f"\n--- Missed Cleavage Distribution ---")
    for mc in sorted(mc_counts.keys()):
        pct = mc_distribution.get(mc, 0)
        print(f"  MC={mc}: {mc_counts[mc]:,} ({pct:.1f}%)")
    
    print(f"\n--- Terminus Classification ---")
    for cls in ['fully_tryptic', 'semi_n_ragged', 'semi_c_ragged', 'non_tryptic', 'not_found', 'protein_not_found']:
        if counts[cls] > 0:
            pct = counts[cls] / filtered_psms * 100 if filtered_psms > 0 else 0
            print(f"  {cls}: {counts[cls]:,} ({pct:.1f}%)")
    
    print(f"\n  Total semi-tryptic: {semi_total:,} ({semi_pct:.1f}%)")
    if counts['semi_c_ragged'] > 0:
        nc_ratio = counts['semi_n_ragged'] / counts['semi_c_ragged']
        print(f"  N-ragged:C-ragged ratio: {nc_ratio:.1f}:1")

    print(f"\n  (No composite 'digestion score' — the raw breakdown above is")
    print(f"   the report. Whether a given semi-tryptic rate is expected is a")
    print(f"   sample-type call for the user, not the tool.)")

    print(f"\nOutput written to: {args.output}")
    
    # JSON output if requested
    if args.output_json:
        result = {
            'summary': {
                'total_psms': total_psms,
                'filtered_psms': filtered_psms,
                'q_value_threshold': args.q_value
            },
            'missed_cleavages': {
                'distribution': {str(k): v for k, v in mc_counts.items()},
                'distribution_pct': {str(k): round(v, 2) for k, v in mc_distribution.items()}
            },
            'terminus_classification': {
                'counts': dict(counts),
                'fully_tryptic_pct': round(counts['fully_tryptic'] / filtered_psms * 100, 2) if filtered_psms > 0 else 0,
                'semi_tryptic_total': semi_total,
                'semi_tryptic_pct': round(semi_pct, 2),
                'semi_n_ragged_pct': round(n_ragged_pct, 2),
                'semi_c_ragged_pct': round(c_ragged_pct, 2),
                'n_c_ratio': round(counts['semi_n_ragged'] / counts['semi_c_ragged'], 2) if counts['semi_c_ragged'] > 0 else None
            }
        }
        
        with open(args.output_json, 'w', encoding='utf-8') as f:
            json.dump(result, f, indent=2)
        
        print(f"JSON output written to: {args.output_json}")


# =============================================================================
# Main entry point
# =============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="Digestion Efficiency Analysis Tool",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )
    
    subparsers = parser.add_subparsers(dest='command', help='Available commands')
    
    # Subset command
    subset_parser = subparsers.add_parser(
        'subset',
        help='Subset FASTA to proteins identified in Pass 1'
    )
    subset_parser.add_argument(
        '--tsv', required=True, help='Path to results.sage.tsv from Pass 1'
    )
    subset_parser.add_argument(
        '--fasta', required=True, help='Path to original FASTA file'
    )
    subset_parser.add_argument(
        '--output', required=True, help='Path for output subset FASTA'
    )
    subset_parser.add_argument(
        '--q-value', type=float, default=0.01,
        help='Peptide q-value threshold (default: 0.01)'
    )
    
    # Annotate command
    annotate_parser = subparsers.add_parser(
        'annotate',
        help='Annotate termini and report digestion breakdown (MC dist, semi-tryptic split, N:C ratio)'
    )
    annotate_parser.add_argument(
        '--tsv', required=True, help='Path to results.sage.tsv from Pass 2'
    )
    annotate_parser.add_argument(
        '--fasta', required=True, help='Path to FASTA file (same as used in search)'
    )
    annotate_parser.add_argument(
        '--output', required=True, help='Path for output annotated TSV'
    )
    annotate_parser.add_argument(
        '--output-json', help='Path for JSON summary output'
    )
    annotate_parser.add_argument(
        '--q-value', type=float, default=0.01,
        help='Peptide q-value threshold (default: 0.01)'
    )
    
    args = parser.parse_args()
    
    if args.command == 'subset':
        cmd_subset(args)
    elif args.command == 'annotate':
        cmd_annotate(args)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
