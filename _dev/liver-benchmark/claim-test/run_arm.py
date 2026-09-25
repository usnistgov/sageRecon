#!/usr/bin/env python3
"""Run one claim-test arm with stock Sage, under a watchdog.

Usage (from a work directory that holds the inputs, so that Sage records
relative paths in results.json):

    python3 run_arm.py SAGE CONFIG.json OUTDIR MZML FASTA

It wraps Sage in `/usr/bin/time -l` (macOS) and writes OUTDIR/run_meta.json
with wall time, peak RSS, and the load average before the run. A watchdog
kills Sage if its memory footprint (top MEM,
which counts compressed memory) passes RSS_LIMIT_GB or the wall time passes
WALL_LIMIT_S. Python 3 standard library only.
"""
import json
import os
import re
import subprocess
import sys
import time

RSS_LIMIT_GB = float(os.environ.get("CLAIM_MEM_LIMIT_GB", "30"))  # footprint, GB
WALL_LIMIT_S = float(os.environ.get("CLAIM_WALL_LIMIT_S", "14400"))


def footprint_kb(pid):
    """Memory footprint of Sage (the child of /usr/bin/time), from `top`.

    RSS alone misses macOS compressed memory: a probe run showed 0.5 GB RSS
    while `top` showed a 41 GB footprint, 40 GB of it compressed.
    """
    kids = subprocess.run(["pgrep", "-P", str(pid)], capture_output=True, text=True).stdout.split()
    total = 0
    for p in kids:
        out = subprocess.run(["top", "-l", "1", "-pid", p, "-stats", "mem"],
                             capture_output=True, text=True).stdout.strip().splitlines()
        val = out[-1].strip().rstrip("+-") if out else ""
        m = re.match(r"([\d.]+)([BKMG])", val)
        if m:
            total += float(m.group(1)) * {"B": 1 / 1024, "K": 1, "M": 1024, "G": 1024 ** 2}[m.group(2)]
    return int(total)


def main():
    sage, config, outdir, mzml, fasta = sys.argv[1:6]
    os.makedirs(outdir, exist_ok=True)
    load = os.getloadavg()
    cmd = ["/usr/bin/time", "-l", sage, config, "--disable-telemetry-i-dont-want-to-improve-sage",
           "-f", fasta, "-o", outdir, mzml]
    t0 = time.time()
    logpath = os.path.join(outdir, "sage.log")
    with open(logpath, "w") as log:
        # Sage logs to stderr; /usr/bin/time -l also writes to stderr.
        proc = subprocess.Popen(cmd, stdout=log, stderr=log)
        killed = None
        peak_poll = 0
        while proc.poll() is None:
            fp = footprint_kb(proc.pid)
            peak_poll = max(peak_poll, fp)
            if fp > RSS_LIMIT_GB * 1024 * 1024:
                killed = f"footprint {fp} kB > {RSS_LIMIT_GB} GB"
            elif time.time() - t0 > WALL_LIMIT_S:
                killed = f"wall > {WALL_LIMIT_S} s"
            if killed:
                subprocess.run(["pkill", "-9", "-P", str(proc.pid)])
                proc.kill()
                break
            time.sleep(2)
        proc.wait()
        wall = time.time() - t0
    err = open(logpath).read()
    m = re.search(r"(\d+)\s+maximum resident set size", err)
    pf = re.search(r"(\d+)\s+peak memory footprint", err)
    meta = {
        "config": os.path.basename(config),
        "returncode": proc.returncode,
        "killed_by_watchdog": killed,
        "wall_seconds": round(wall, 1),
        "peak_rss_bytes_time_l": int(m.group(1)) if m else None,
        "peak_footprint_bytes_time_l": int(pf.group(1)) if pf else None,
        "peak_footprint_kb_polled": peak_poll,
        "loadavg_before": [round(x, 2) for x in load],
        "loadavg_after": [round(x, 2) for x in os.getloadavg()],
        "started": time.strftime("%Y-%m-%dT%H:%M:%S", time.localtime(t0)),
    }
    with open(os.path.join(outdir, "run_meta.json"), "w") as fh:
        json.dump(meta, fh, indent=2)
    print(json.dumps(meta))


if __name__ == "__main__":
    main()
