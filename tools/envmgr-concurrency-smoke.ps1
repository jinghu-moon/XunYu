param(
  [int]$Workers = 4,
  [int]$Iterations = 10,
  [string]$Scope = "user",
  [string]$Prefix = "XUN_ENVMGR_CONC_SMOKE",
  [switch]$NoCleanup = $false
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

if ($Workers -lt 1) {
  throw "Workers must be >= 1"
}
if ($Iterations -lt 1) {
  throw "Iterations must be >= 1"
}

function Ensure-Bin {
  $bin = Join-Path (Get-Location) "target/debug/xun.exe"
  if (-not (Test-Path $bin)) {
    Write-Host "==> building xun binary"
    cargo build --features dashboard | Out-Null
  }
  if (-not (Test-Path $bin)) {
    throw "missing binary: $bin"
  }
  return $bin
}

function Invoke-EnvCmd {
  param(
    [string]$Bin,
    [string[]]$CmdArgs
  )
  & $Bin @CmdArgs | Out-Null
}

function Remove-SmokeVars {
  param(
    [string]$Bin,
    [string]$Scope,
    [string]$Prefix,
    [int]$Workers,
    [int]$Iterations
  )

  for ($w = 1; $w -le $Workers; $w++) {
    for ($i = 1; $i -le $Iterations; $i++) {
      $name = "${Prefix}_${w}_${i}"
      try {
        Invoke-EnvCmd -Bin $Bin -CmdArgs @("env", "del", $name, "--scope", $Scope, "-y")
      } catch {
      }
    }
  }
}

$binPath = Ensure-Bin

Write-Host "==> collecting baseline snapshot count"
$baselineJson = & $binPath env snapshot list --format json
$baseline = $baselineJson | ConvertFrom-Json
$baselineCount = @($baseline).Count

Write-Host "==> running concurrent env write workload"
$jobs = @()
for ($worker = 1; $worker -le $Workers; $worker++) {
  $jobs += Start-Job -ScriptBlock {
    param($BinPath, $Worker, $Iterations, $Scope, $Prefix)
    $ErrorActionPreference = "Stop"
    $failed = @()
    for ($i = 1; $i -le $Iterations; $i++) {
      $name = "${Prefix}_${Worker}_${i}"
      $value = "v_${Worker}_${i}"
      try {
        & $BinPath env set $name $value --scope $Scope | Out-Null
        & $BinPath env get $name --scope $Scope | Out-Null
        & $BinPath env del $name --scope $Scope -y | Out-Null
      } catch {
        $failed += "$name :: $($_.Exception.Message)"
      }
    }
    [pscustomobject]@{
      worker = $Worker
      failed = $failed
      fail_count = $failed.Count
    }
  } -ArgumentList $binPath, $worker, $Iterations, $Scope, $Prefix
}

$results = Receive-Job -Wait -AutoRemoveJob $jobs
$totalFailures = ($results | Measure-Object -Property fail_count -Sum).Sum

if (-not $NoCleanup) {
  Write-Host "==> cleanup smoke vars"
  Remove-SmokeVars -Bin $binPath -Scope $Scope -Prefix $Prefix -Workers $Workers -Iterations $Iterations
}

Write-Host "==> collecting final snapshot count"
$finalJson = & $binPath env snapshot list --format json
$final = $finalJson | ConvertFrom-Json
$finalCount = @($final).Count

Write-Host ("baseline snapshots: {0}" -f $baselineCount)
Write-Host ("final snapshots: {0}" -f $finalCount)

if ($totalFailures -gt 0) {
  $details = $results | ForEach-Object { $_.failed } | Where-Object { $_ } | Select-Object -First 10
  throw ("concurrency smoke failed: {0} errors; sample={1}" -f $totalFailures, ($details -join " | "))
}

Write-Host ("concurrency smoke passed: workers={0}, iterations={1}, operations={2}" -f $Workers, $Iterations, ($Workers * $Iterations * 3))
