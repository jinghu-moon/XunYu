param(
  [string]$Port = "7071",
  [switch]$SkipTests = $false,
  [switch]$SkipServe = $false,
  [switch]$VerifyWsChanged = $false
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

function Invoke-Step {
  param(
    [string]$Name,
    [scriptblock]$Body
  )
  Write-Host "==> $Name"
  & $Body
}

function Assert-StatusCode {
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
  return $resp
}

Invoke-Step "cargo check --all-features" {
  cargo check --all-features
}

if (-not $SkipTests) {
  Invoke-Step "cargo test --all-features" {
    cargo test --all-features
  }
}

Invoke-Step "pnpm -C dashboard-ui build" {
  pnpm -C "dashboard-ui" build
}

Invoke-Step "cargo build --features dashboard" {
  cargo build --features dashboard | Out-Null
}

$BinPath = Join-Path (Get-Location) "target/debug/xun.exe"
if (-not (Test-Path $BinPath)) {
  throw "missing binary: $BinPath"
}

Invoke-Step "xun env --help" {
  & $BinPath env --help | Out-Null
}

Invoke-Step "xun env snapshot create --desc smoke" {
  & $BinPath env snapshot create --desc smoke | Out-Null
}

Invoke-Step "xun env diff-live --scope user --format json" {
  & $BinPath env diff-live --scope user --format json | Out-Null
}

Invoke-Step "xun env list --scope user -f json" {
  & $BinPath env list --scope user -f json | Out-Null
}

if (-not $SkipServe) {
  Invoke-Step "serve + /api/env/* + ws smoke" {
    $process = Start-Process -FilePath $BinPath -ArgumentList @("serve", "--port", $Port) -PassThru
    try {
      $ok = $false
      for ($i = 0; $i -lt 60; $i++) {
        Start-Sleep -Milliseconds 500
        try {
          $resp = Invoke-WebRequest -UseBasicParsing -Uri ("http://127.0.0.1:{0}/api/env/ping" -f $Port) -TimeoutSec 2
          if ($resp.StatusCode -eq 200) {
            $ok = $true
            break
          }
        } catch {
        }
      }
      if (-not $ok) {
        throw "serve smoke failed: /api/env/ping not ready"
      }

      $base = "http://127.0.0.1:$Port"
      Assert-StatusCode -Uri "$base/api/env/ping" | Out-Null
      Assert-StatusCode -Uri "$base/api/env/vars?scope=user" | Out-Null
      Assert-StatusCode -Uri "$base/api/env/schema" | Out-Null
      Assert-StatusCode -Uri "$base/api/env/annotations" | Out-Null
      Assert-StatusCode -Uri "$base/api/env/export-live?scope=user&format=dotenv" | Out-Null
      Assert-StatusCode -Uri "$base/api/env/template/expand" -Method "POST" -Body '{"scope":"user","template":"Path=%PATH%"}' | Out-Null

      $ws = [System.Net.WebSockets.ClientWebSocket]::new()
      $ws.ConnectAsync([Uri]("ws://127.0.0.1:{0}/api/env/ws" -f $Port), [Threading.CancellationToken]::None).GetAwaiter().GetResult()
      $buffer = New-Object byte[] 4096
      $task = $ws.ReceiveAsync([ArraySegment[byte]]::new($buffer), [Threading.CancellationToken]::None)
      try {
        if (-not $task.Wait(5000)) {
          throw "ws smoke failed: no first frame in 5s"
        }
      } catch {
        throw "ws smoke failed: receive first frame error: $($_.Exception.Message)"
      }
      try {
        $res = $task.Result
      } catch {
        throw "ws smoke failed: first frame task faulted: $($_.Exception.Message)"
      }
      if ($res.Count -le 0) {
        throw "ws smoke failed: empty first frame"
      }
      $first = [Text.Encoding]::UTF8.GetString($buffer, 0, $res.Count)
      if (-not $first.Contains('"channel":"env"')) {
        throw "ws smoke failed: unexpected first frame: $first"
      }

      if ($VerifyWsChanged) {
        $tmpName = "XUN_ENVMGR_WS_SMOKE_$PID"
        try {
          Assert-StatusCode -Uri "$base/api/env/vars/${tmpName}?scope=user" -Method "POST" -Body '{"value":"1","no_snapshot":true}' | Out-Null
          $changedOk = $false
          for ($idx = 0; $idx -lt 6; $idx++) {
            $evtTask = $ws.ReceiveAsync([ArraySegment[byte]]::new($buffer), [Threading.CancellationToken]::None)
            $evtReady = $false
            try {
              $evtReady = $evtTask.Wait(5000)
            } catch {
              continue
            }
            if (-not $evtReady) {
              continue
            }
            try {
              $evt = $evtTask.Result
            } catch {
              continue
            }
            if ($evt.Count -le 0) {
              continue
            }
            $frame = [Text.Encoding]::UTF8.GetString($buffer, 0, $evt.Count)
            if ($frame.Contains('"type":"changed"') -and $frame.Contains($tmpName)) {
              $changedOk = $true
              break
            }
          }
          if (-not $changedOk) {
            throw "ws smoke failed: no changed event frame for $tmpName"
          }
        } finally {
          try {
            Assert-StatusCode -Uri "$base/api/env/vars/${tmpName}?scope=user" -Method "DELETE" | Out-Null
          } catch {
          }
        }
      }

      try {
        if ($ws.State -eq [System.Net.WebSockets.WebSocketState]::Open) {
          $null = $ws.CloseAsync([System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure, "done", [Threading.CancellationToken]::None).GetAwaiter().GetResult()
        }
      } catch {
      }

      Write-Host "serve smoke passed"
    } finally {
      if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
      }
    }
  }
}

Write-Host "EnvMgr smoke completed."
