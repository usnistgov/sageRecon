#!/usr/bin/env bash
# Claim test: Ben's vanilla search vs the recon-guided search, on the liver file.
# Usage: bash run.sh <sage-binary> <10mg_1_A_1.mzML.gz> <fasta> <work-dir>
# Runs the two arms one at a time, then vanilla again (run-to-run noise).
set -euo pipefail
SAGE=$1; MZML=$2; FASTA=$3; WORK=$4
HERE=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$WORK"
"$SAGE" --version | tee "$WORK/sage_version.txt"
shasum -a 256 "$MZML" "$FASTA" | tee "$WORK/inputs.sha256"
if [ "$(uname)" = Darwin ]; then TIME="/usr/bin/time -l"; else TIME="/usr/bin/time -v"; fi
for ARM in vanilla recon_guided vanilla_repeat; do
  CFG="$HERE/configs/${ARM%_repeat}.json"
  echo "== $ARM $(date)"
  $TIME "$SAGE" "$CFG" --disable-telemetry-i-dont-want-to-improve-sage \
    -f "$FASTA" -o "$WORK/$ARM" "$MZML" 2> "$WORK/$ARM.time.log"
done
python3 "$HERE/summarize.py" "$WORK" | tee "$WORK/summary.md"
