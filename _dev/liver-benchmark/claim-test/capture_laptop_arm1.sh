#!/usr/bin/env bash
# Save the laptop arm 1 run as the cross-check reference (laptop-arm1/).
# Usage, from the repository root: bash _dev/liver-benchmark/claim-test/capture_laptop_arm1.sh WORKDIR
set -euo pipefail
WORK="$1"
D=_dev/liver-benchmark/claim-test
OUT="$WORK/out_arm1_vanilla"
[ -f "$OUT/results.sage.tsv" ] || { echo "STOP: $OUT/results.sage.tsv missing" >&2; exit 1; }
python3 "$D/analyze.py" "$WORK" --write-reference "$D/laptop-arm1"
python3 "$D/redact_copy.py" "$D/laptop-arm1" "$OUT/results.json" "$OUT/sage.log" "$OUT/run_meta.json"
if grep -rn -e '/Users/' -e '/private/tmp' -e 'claude-501' "$D/laptop-arm1" --include='*.json' --include='*.log'; then
  echo "STOP: a personal or temporary path is left in laptop-arm1/" >&2; exit 1
fi
echo "laptop-arm1/ written"
