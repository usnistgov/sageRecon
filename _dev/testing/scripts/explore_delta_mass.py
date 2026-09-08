"""
Quick exploratory analysis of delta masses from Sage open search results.
Used to inform Phase 1 JSON schema design.
"""
import csv
from collections import Counter

# Read the TSV
# Path is relative to testing/ (run from there, or pass an absolute path)
results_file = "search-output/open-search/results.sage.tsv"

delta_masses = []
with open(results_file, 'r') as f:
    reader = csv.DictReader(f, delimiter='\t')
    for row in reader:
        # Filter: peptide_q < 0.01 and not decoy
        if float(row['peptide_q']) < 0.01 and not row['proteins'].startswith('rev_'):
            expmass = float(row['expmass'])
            calcmass = float(row['calcmass'])
            delta = expmass - calcmass
            # Round to 0.01 Da bins for histogram
            delta_bin = round(delta, 2)
            delta_masses.append(delta_bin)

# Count occurrences
counts = Counter(delta_masses)

# Top 30 delta mass peaks
print("=" * 60)
print("TOP 30 DELTA MASS PEAKS (0.01 Da bins, peptide_q < 0.01)")
print("=" * 60)
print(f"{'Delta (Da)':<15} {'Count':<10} {'Likely Annotation'}")
print("-" * 60)

# Known modifications for quick annotation
known_mods = {
    0.0: "Unmodified",
    0.98: "Deamidation (NQ)",
    0.99: "Deamidation (NQ)",
    1.0: "Deamidation (NQ) / 13C",
    15.99: "Oxidation (M)",
    16.0: "Oxidation (M)",
    28.03: "Dimethyl (KR)",
    31.99: "Dioxidation (M)",
    32.0: "Dioxidation (M)",
    42.01: "Acetyl (K/N-term)",
    57.02: "Carbamidomethyl (C) - should be fixed!",
    79.97: "Phospho (STY)",
    114.04: "GG/Ubiquitin (K)",
}

for delta, count in counts.most_common(30):
    annotation = ""
    for known_delta, name in known_mods.items():
        if abs(delta - known_delta) < 0.02:
            annotation = name
            break
    print(f"{delta:<15.2f} {count:<10} {annotation}")

print("\n" + "=" * 60)
print("SUMMARY STATISTICS")
print("=" * 60)
print(f"Total PSMs (filtered): {len(delta_masses)}")
print(f"Unique delta bins: {len(counts)}")

# Count PSMs near zero (unmodified)
near_zero = sum(c for d, c in counts.items() if abs(d) < 0.5)
print(f"PSMs with delta near 0 (±0.5 Da): {near_zero} ({100*near_zero/len(delta_masses):.1f}%)")

# Count PSMs with significant modifications
significant = sum(c for d, c in counts.items() if abs(d) >= 0.5)
print(f"PSMs with significant delta (≥0.5 Da): {significant} ({100*significant/len(delta_masses):.1f}%)")
