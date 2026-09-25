#!/usr/bin/env bash
# Run the liver claim test: arms 1 to 4 with stock Sage, one at a time,
# then a repeat of arm 1 (the run-to-run noise band).
#
# Usage:
#   bash run_claim_test.sh --sage PATH/TO/sage --mzml PATH/TO/10mg_1_A_1.mzML.gz \
#        --fasta PATH/TO/uniprot_sprot_iso_human-2018_06.fasta --work WORKDIR \
#        [--sage-src PATH/TO/sage-git-clone] [--stages] [--no-repeat] [--check-only]
#
# macOS or Linux. On Windows, run it in WSL (Linux) with a Linux Sage build.
# It stops on a wrong Sage version, a wrong Sage source rev (if --sage-src is
# given), a wrong input checksum, or another Sage process already running.
# Each arm writes WORKDIR/out_<config>/ with results.sage.tsv, results.json,
# sage.log (Sage's log plus the time report) and run_meta.json.
set -euo pipefail

SAGE_VERSION="0.15.0-beta.2"
SAGE_REV="df9219951cc9a54cf4cd55d76541af24b687bd3d"
MZML_SHA256="460cd316cb95f0db468dfdbcdaeabcb195ba51eff585a1af2ae861f37575512e"
FASTA_SHA256="75cc5a96a489a04b385e07a3d4caa223cf361a3727b267c8961b86211c14c922"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

SAGE="" MZML="" FASTA="" WORK="" SAGE_SRC="" STAGES=0 REPEAT=1 CHECK_ONLY=0
while [ $# -gt 0 ]; do
  case "$1" in
    --sage) SAGE="$2"; shift 2 ;;
    --mzml) MZML="$2"; shift 2 ;;
    --fasta) FASTA="$2"; shift 2 ;;
    --work) WORK="$2"; shift 2 ;;
    --sage-src) SAGE_SRC="$2"; shift 2 ;;
    --stages) STAGES=1; shift ;;
    --no-repeat) REPEAT=0; shift ;;
    --check-only) CHECK_ONLY=1; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
for v in SAGE MZML FASTA WORK; do
  [ -n "${!v}" ] || { echo "missing --$(echo "$v" | tr 'A-Z' 'a-z')" >&2; exit 2; }
done

abspath() { (cd "$(dirname "$1")" && echo "$(pwd)/$(basename "$1")"); }
SAGE="$(abspath "$SAGE")"; MZML="$(abspath "$MZML")"; FASTA="$(abspath "$FASTA")"

sha256() {
  if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

# ---- checks
got="$("$SAGE" --version 2>&1 | head -1)"
case "$got" in
  *"$SAGE_VERSION"*) echo "Sage version OK: $got" ;;
  *) echo "STOP: Sage says '$got', expected $SAGE_VERSION" >&2; exit 1 ;;
esac
if [ -n "$SAGE_SRC" ]; then
  rev="$(git -C "$SAGE_SRC" rev-parse HEAD)"
  [ "$rev" = "$SAGE_REV" ] || { echo "STOP: Sage source is at $rev, expected $SAGE_REV" >&2; exit 1; }
  echo "Sage source rev OK: $rev"
else
  echo "NOTE: no --sage-src given; the version string is checked, the source rev is not."
fi
for pair in "MZML:$MZML_SHA256" "FASTA:$FASTA_SHA256"; do
  var="${pair%%:*}"; want="${pair#*:}"; file="${!var}"
  have="$(sha256 "$file")"
  [ "$have" = "$want" ] || { echo "STOP: $file sha256 $have, expected $want" >&2; exit 1; }
  echo "sha256 OK: $(basename "$file")"
done

case "$(uname -s)" in
  Darwin) TIMEFLAG="-l" ;;
  Linux) TIMEFLAG="-v" ;;
  *) echo "STOP: unsupported OS $(uname -s); use macOS, Linux or WSL" >&2; exit 1 ;;
esac
[ -x /usr/bin/time ] || { echo "STOP: /usr/bin/time not found (Linux: install the 'time' package)" >&2; exit 1; }

# ---- work folder: inputs linked in, so Sage records relative paths
mkdir -p "$WORK"; cd "$WORK"
ln -sf "$MZML" 10mg_1_A_1.mzML.gz
ln -sf "$FASTA" uniprot_sprot_iso_human-2018_06.fasta
ln -sf "$SAGE" sage
cp "$HERE"/configs/arm*.json .
{
  echo "date	$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "host_os	$(uname -srm)"
  echo "cpus	$(getconf _NPROCESSORS_ONLN)"
  if [ "$(uname -s)" = Darwin ]; then echo "mem_bytes	$(sysctl -n hw.memsize)"
  else echo "mem_kB	$(awk '/MemTotal/ {print $2}' /proc/meminfo)"; fi
  echo "sage_version	$got"
  echo "sage_src_rev	${rev:-not checked}"
  echo "mzml_sha256	$MZML_SHA256"
  echo "fasta_sha256	$FASTA_SHA256"
} > machine.tsv

if [ "$CHECK_ONLY" = 1 ]; then echo "checks passed; --check-only, no search run"; cat machine.tsv; exit 0; fi

ARMS="arm1_vanilla arm2_vanilla_mods_recon_tol arm3_recon_mods_vanilla_tol arm4_recon_full"
[ "$STAGES" = 1 ] && ARMS="$ARMS arm4_stage1_rare arm4_stage2_cys"
[ "$REPEAT" = 1 ] && ARMS="$ARMS arm1_vanilla_rep"
[ -f arm1_vanilla_rep.json ] || cp arm1_vanilla.json arm1_vanilla_rep.json

for arm in $ARMS; do
  if pgrep -f '(^|/)sage [^ ]*\.json' >/dev/null; then
    echo "STOP: another Sage search is running; runs must not overlap" >&2; exit 1
  fi
  out="out_$arm"
  if [ -f "$out/run_meta.json" ] && [ -f "$out/results.sage.tsv" ]; then echo "skip $arm (done)"; continue; fi
  rm -rf "$out"; mkdir -p "$out"
  load="$(uptime | sed 's/.*load average[s]*: //')"
  echo "run $arm  (load: $load)"
  t0=$(date +%s)
  set +e
  /usr/bin/time $TIMEFLAG ./sage "$arm.json" --disable-telemetry-i-dont-want-to-improve-sage \
    -f uniprot_sprot_iso_human-2018_06.fasta -o "$out" 10mg_1_A_1.mzML.gz > "$out/sage.log" 2>&1
  rc=$?
  set -e
  t1=$(date +%s)
  if [ "$(uname -s)" = Darwin ]; then
    rss=$(awk '/maximum resident set size/ {print $1}' "$out/sage.log" | tail -1)
    fp=$(awk '/peak memory footprint/ {print $1}' "$out/sage.log" | tail -1)
  else
    kb=$(awk -F': ' '/Maximum resident set size/ {print $2}' "$out/sage.log" | tail -1)
    rss=$(( ${kb:-0} * 1024 )); fp=""
  fi
  cat > "$out/run_meta.json" <<EOF
{
  "config": "$arm.json",
  "returncode": $rc,
  "killed_by_watchdog": null,
  "wall_seconds": $((t1 - t0)),
  "peak_rss_bytes_time_l": ${rss:-null},
  "peak_footprint_bytes_time_l": ${fp:-null},
  "loadavg_before": ["$load"],
  "started": "$(date -u -r "$t0" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -d "@$t0" +%Y-%m-%dT%H:%M:%SZ)"
}
EOF
  printf '%s\t%s\t%s\t%s\n' "$arm" "$rc" "$((t1 - t0))" "${rss:-}" >> runs.tsv
  # A failed arm (for example out of memory) is recorded, and the next arm runs.
  [ "$rc" = 0 ] || echo "WARNING: $arm failed (exit $rc); see $WORK/$out/sage.log" >&2
done
echo "done. Next: python3 _dev/liver-benchmark/claim-test/analyze.py $WORK"
