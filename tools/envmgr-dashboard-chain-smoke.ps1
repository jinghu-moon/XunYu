param(
  [string]$Port = "7073"
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

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

function Invoke-ApiJson {
  param(
    [string]$Uri,
    [string]$Method = "GET",
    [string]$Body = "",
    [string]$ContentType = "application/json"
  )

  if ($Method -eq "GET") {
    $resp = Invoke-WebRequest -UseBasicParsing -Uri $Uri -TimeoutSec 5
  } else {
    $resp = Invoke-WebRequest -UseBasicParsing -Method $Method -ContentType $ContentType -Body $Body -Uri $Uri -TimeoutSec 5
  }
  if ($resp.StatusCode -ne 200) {
    throw "unexpected status=$($resp.StatusCode) uri=$Uri"
  }
  if ([string]::IsNullOrWhiteSpace($resp.Content)) {
    return $null
  }
  return ($resp.Content | ConvertFrom-Json)
}

$binPath = Ensure-Bin
$process = Start-Process -FilePath $binPath -ArgumentList @("serve", "--port", $Port) -PassThru

try {
  $base = "http://127.0.0.1:$Port"
  Write-Host "==> waiting for serve ready"
  $ready = $false
  for ($i = 0; $i -lt 60; $i++) {
    Start-Sleep -Milliseconds 500
    try {
      $ping = Invoke-WebRequest -UseBasicParsing -Uri "$base/api/env/ping" -TimeoutSec 2
      if ($ping.StatusCode -eq 200) {
        $ready = $true
        break
      }
    } catch {
    }
  }
  if (-not $ready) {
    throw "serve not ready: /api/env/ping"
  }

  $tmpName = "XUN_ENVMGR_PANEL_SMOKE_$PID"
  $tmpValue = "panel_smoke_$(Get-Date -Format 'yyyyMMddHHmmss')"

  Write-Host "==> env vars list"
  $varsResp = Invoke-ApiJson -Uri "$base/api/env/vars?scope=user"
  if (-not $varsResp.ok) {
    throw "vars response not ok"
  }

  Write-Host "==> set temp var"
  $setBody = "{`"value`":`"$tmpValue`",`"no_snapshot`":true}"
  $setResp = Invoke-ApiJson -Uri "$base/api/env/vars/${tmpName}?scope=user" -Method "POST" -Body $setBody
  if (-not $setResp.ok) {
    throw "set response not ok"
  }

  Write-Host "==> get temp var"
  $getResp = Invoke-ApiJson -Uri "$base/api/env/vars/${tmpName}?scope=user"
  if (-not $getResp.ok) {
    throw "get response not ok"
  }
  if ($getResp.data.name -ne $tmpName) {
    throw "unexpected get var name: $($getResp.data.name)"
  }

  Write-Host "==> snapshot create/list"
  $snapListBefore = Invoke-ApiJson -Uri "$base/api/env/snapshots"
  if (-not $snapListBefore.ok) {
    throw "snapshot list response not ok (before)"
  }
  $createBody = "{`"desc`":`"panel-chain-smoke-$PID`"}"
  $snapCreate = Invoke-ApiJson -Uri "$base/api/env/snapshots" -Method "POST" -Body $createBody
  if (-not $snapCreate.ok) {
    throw "snapshot create response not ok"
  }
  $snapListAfter = Invoke-ApiJson -Uri "$base/api/env/snapshots"
  if (-not $snapListAfter.ok) {
    throw "snapshot list response not ok (after)"
  }

  Write-Host "==> doctor run"
  $doctorResp = Invoke-ApiJson -Uri "$base/api/env/doctor/run" -Method "POST" -Body '{"scope":"user"}'
  if (-not $doctorResp.ok) {
    throw "doctor response not ok"
  }

  Write-Host "==> export/import(diff-live)"
  $exportResp = Invoke-WebRequest -UseBasicParsing -Uri "$base/api/env/export?scope=user&format=json" -TimeoutSec 5
  if ($exportResp.StatusCode -ne 200) {
    throw "export response not ok"
  }
  $importResp = Invoke-ApiJson -Uri "$base/api/env/import" -Method "POST" -Body '{"scope":"user","content":"XUN_ENVMGR_IMPORT_SMOKE=1","mode":"merge","dry_run":true}'
  if (-not $importResp.ok) {
    throw "import response not ok"
  }
  $diffResp = Invoke-ApiJson -Uri "$base/api/env/diff-live?scope=user"
  if (-not $diffResp.ok) {
    throw "diff-live response not ok"
  }

  Write-Host "==> delete temp var"
  $delResp = Invoke-ApiJson -Uri "$base/api/env/vars/${tmpName}?scope=user" -Method "DELETE"
  if (-not $delResp.ok) {
    throw "delete response not ok"
  }

  Write-Host "dashboard chain smoke passed"
} finally {
  if ($process -and -not $process.HasExited) {
    Stop-Process -Id $process.Id -Force
  }
}
