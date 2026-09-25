# Claim test on Windows: Ben's vanilla search vs the recon-guided search, on the liver file.
# Same as run.sh. Runs vanilla, recon_guided, then vanilla again, one at a time,
# recording wall time and peak memory for each, then writes summary.md.
# Usage (PowerShell):
#   powershell -ExecutionPolicy Bypass -File run.ps1 -Sage <sage.exe> -Mzml <10mg_1_A_1.mzML.gz> -Fasta <fasta> -Work <work-dir>
param(
  [Parameter(Mandatory)][string]$Sage,
  [Parameter(Mandatory)][string]$Mzml,
  [Parameter(Mandatory)][string]$Fasta,
  [Parameter(Mandatory)][string]$Work
)
$ErrorActionPreference = 'Stop'
$Here = Split-Path -Parent $MyInvocation.MyCommand.Path
New-Item -ItemType Directory -Force -Path $Work | Out-Null
& $Sage --version | Tee-Object -FilePath (Join-Path $Work 'sage_version.txt')
Get-FileHash -Algorithm SHA256 $Mzml, $Fasta | Format-Table -AutoSize | Out-String |
  Tee-Object -FilePath (Join-Path $Work 'inputs.sha256')
foreach ($arm in 'vanilla', 'recon_guided', 'vanilla_repeat') {
  $cfg = Join-Path $Here ('configs\' + ($arm -replace '_repeat', '') + '.json')
  Write-Host "== $arm $(Get-Date)"
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $p = Start-Process -FilePath $Sage -NoNewWindow -PassThru `
    -ArgumentList @($cfg, '--disable-telemetry-i-dont-want-to-improve-sage', '-f', $Fasta, '-o', (Join-Path $Work $arm), $Mzml) `
    -RedirectStandardError (Join-Path $Work "$arm.stderr.log")
  $peak = 0
  while (-not $p.HasExited) {
    try { $p.Refresh(); if ($p.PeakWorkingSet64 -gt $peak) { $peak = $p.PeakWorkingSet64 } } catch {}
    Start-Sleep -Seconds 2
  }
  $sw.Stop()
  if ($p.ExitCode -ne 0) { throw "Sage failed on $arm (exit $($p.ExitCode)); see $arm.stderr.log" }
  # Same wording run.sh gets from /usr/bin/time -l, so summarize.py reads both.
  "{0:F2} real`n{1} maximum resident set size" -f $sw.Elapsed.TotalSeconds, $peak |
    Set-Content -Path (Join-Path $Work "$arm.time.log")
}
python (Join-Path $Here 'summarize.py') $Work | Tee-Object -FilePath (Join-Path $Work 'summary.md')
