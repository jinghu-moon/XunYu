# xun alias Phase 5 acceptance helper (Windows)
[CmdletBinding()]
param(
    [string]$RepoRoot = "",
    [string]$ShimPath = "",
    [switch]$BuildShim,
    [switch]$SkipManual
)

$ErrorActionPreference = "Stop"

function Get-UacEnabled {
    try {
        $v = (Get-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System" -Name "EnableLUA" -ErrorAction Stop).EnableLUA
        return ([int]$v -ne 0)
    } catch {
        return $true
    }
}

if (-not $RepoRoot) {
    $RepoRoot = Split-Path -Parent $PSScriptRoot
}
$RepoRoot = [System.IO.Path]::GetFullPath($RepoRoot)

Push-Location $RepoRoot
try {
    $isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).
        IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    $uacEnabled = Get-UacEnabled

    Write-Host "xun alias Phase 5 acceptance runner"
    Write-Host "Repo: $RepoRoot"
    Write-Host "Admin shell: $isAdmin"
    Write-Host "UAC enabled: $uacEnabled"
    Write-Host "This script validates runtime behavior (#25-#30); it does not modify system-wide settings."
    if ($isAdmin) {
        Write-Host "Note: #30 UAC(740) fallback is hard to trigger in Administrator shell; use non-admin shell for strict validation."
    }
    Write-Host ""

    if ($BuildShim) {
        & cargo build -p alias-shim --profile release-shim
    }

    if (-not $ShimPath) {
        $ShimPath = Join-Path $RepoRoot "target/release-shim/alias-shim.exe"
    }
    $ShimPath = [System.IO.Path]::GetFullPath($ShimPath)
    if (-not (Test-Path $ShimPath)) {
        throw "alias-shim.exe not found: $ShimPath"
    }

    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $work = Join-Path $RepoRoot "target/alias-phase5/$stamp"
    New-Item -ItemType Directory -Path $work -Force | Out-Null

    function Write-Result {
        param(
            [string]$Id,
            [string]$Status,
            [string]$Detail
        )
        [pscustomobject]@{
            id     = $Id
            status = $Status
            detail = $Detail
        }
    }

    function Set-GuiSubsystem {
        param([string]$ExePath)
        $bytes = [System.IO.File]::ReadAllBytes($ExePath)
        if ($bytes.Length -lt 0x40) { throw "PE too small: $ExePath" }
        $e = [BitConverter]::ToUInt32($bytes, 0x3c)
        if ($bytes.Length -lt ($e + 4 + 20 + 70)) { throw "PE header out of range: $ExePath" }
        $opt = $e + 4 + 20
        $sub = $opt + 68
        $bytes[$sub] = 2
        $bytes[$sub + 1] = 0
        [System.IO.File]::WriteAllBytes($ExePath, $bytes)
    }

    function New-CaseBinary {
        param(
            [string]$Name,
            [switch]$Gui
        )
        $exe = Join-Path $work "$Name.exe"
        Copy-Item $ShimPath $exe -Force
        if ($Gui) {
            Set-GuiSubsystem -ExePath $exe
        }
        return $exe
    }

    function Write-Shim {
        param(
            [string]$CaseName,
            [string]$Content
        )
        Set-Content -Path (Join-Path $work "$CaseName.shim") -Value $Content -Encoding ASCII
    }

    $results = New-Object System.Collections.Generic.List[object]

    # #25 exe shim: args + exit code pass-through
    try {
        $case = "case25_exe"
        $exe = New-CaseBinary $case
        Write-Shim $case @"
type = exe
path = C:\Windows\System32\cmd.exe
args = /c exit
wait = true
"@
        & $exe 37
        $code = $LASTEXITCODE
        if ($code -eq 37) {
            $results.Add((Write-Result "#25" "pass" "exe shim exit-code pass-through = 37"))
        } else {
            $results.Add((Write-Result "#25" "fail" "expected 37, got $code"))
        }
    } catch {
        $results.Add((Write-Result "#25" "fail" $_.Exception.Message))
    }

    # #26 cmd shim: pipe / redirection / && behavior
    try {
        $case = "case26_cmd"
        $exe = New-CaseBinary $case
        $outFile = Join-Path $work "case26.out.txt"
        Write-Shim $case @"
type = cmd
cmd = echo ok|findstr ok > "$outFile" && exit /b 5
wait = true
"@
        & $exe
        $code = $LASTEXITCODE
        $txt = if (Test-Path $outFile) { (Get-Content $outFile -Raw).Trim() } else { "" }
        if ($code -eq 5 -and $txt -eq "ok") {
            $results.Add((Write-Result "#26" "pass" "cmd shim operators passed (exit=5, output=ok)"))
        } else {
            $results.Add((Write-Result "#26" "fail" "expected exit=5/output=ok, got exit=$code output='$txt'"))
        }
    } catch {
        $results.Add((Write-Result "#26" "fail" $_.Exception.Message))
    }

    # #27 process-tree cleanup when shim is force-killed
    # Uses ping-process count delta as a no-admin proxy (avoids WMI/CIM permissions).
    try {
        $case = "case27_tree"
        $exe = New-CaseBinary $case
        Write-Shim $case @"
type = cmd
cmd = ping -t 127.0.0.1 > nul
wait = true
debug = true
"@
        $basePing = @(Get-Process -Name "ping" -ErrorAction SilentlyContinue).Count
        $proc = Start-Process -FilePath $exe -PassThru -WindowStyle Hidden
        Start-Sleep -Milliseconds 1200

        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 1200

        $afterPing = @(Get-Process -Name "ping" -ErrorAction SilentlyContinue).Count
        if ($afterPing -le $basePing) {
            $results.Add((Write-Result "#27" "pass" "no extra ping process remained after shim kill"))
        } else {
            $results.Add((Write-Result "#27" "fail" "ping process count leak: baseline=$basePing after=$afterPing"))
        }
    } catch {
        $results.Add((Write-Result "#27" "fail" $_.Exception.Message))
    }

    if (-not $SkipManual) {
        # #28 GUI behavior check (wait=false should return quickly while GUI target launches)
        try {
            $case = "case28_gui"
            $exe = New-CaseBinary $case -Gui
            $procName = "winver"
            Write-Shim $case @"
type = exe
path = C:\Windows\System32\winver.exe
wait = false
"@
            $before = @(Get-Process -Name $procName -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            $shimProc = Start-Process -FilePath $exe -PassThru
            $shimFastExit = $true
            $shimExitCode = $null
            try {
                Wait-Process -Id $shimProc.Id -Timeout 2 -ErrorAction Stop
                $shimProc.Refresh()
                $shimExitCode = $shimProc.ExitCode
            } catch {
                $shimFastExit = $false
            }
            $sw.Stop()
            $new = @()
            for ($i = 0; $i -lt 30; $i++) {
                $after = @(Get-Process -Name $procName -ErrorAction SilentlyContinue)
                $new = @($after | Where-Object { $before -notcontains $_.Id })
                if ($new.Count -gt 0) {
                    break
                }
                Start-Sleep -Milliseconds 100
            }
            foreach ($p in $new) {
                Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
            }
            $elapsed = [math]::Round($sw.Elapsed.TotalMilliseconds, 2)
            $afterCount = @(Get-Process -Name $procName -ErrorAction SilentlyContinue).Count
            $launched = ($new.Count -ge 1) -or (($before.Count -gt 0) -and ($afterCount -ge $before.Count))
            if ($shimFastExit -and $shimExitCode -eq 0 -and $launched -and $elapsed -lt 800) {
                $results.Add((Write-Result "#28" "pass" "GUI wait=false behavior ok (launch + fast return ${elapsed}ms)"))
            } else {
                $results.Add((Write-Result "#28" "fail" "unexpected GUI/wait behavior (shim_fast_exit=$shimFastExit, shim_exit=$shimExitCode, new_proc=$($new.Count), before=$($before.Count), after=$afterCount, elapsed_ms=$elapsed)"))
            }
        } catch {
            $results.Add((Write-Result "#28" "fail" $_.Exception.Message))
        }

        # #29 AssignProcessToJobObject failure best-effort fallback
        try {
            $case = "case29_job_fallback"
            $exe = New-CaseBinary $case
            $log = Join-Path $work "case29.stderr.log"
            Write-Shim $case @"
type = cmd
cmd = exit /b 0
wait = true
debug = true
"@
            & $exe 2> $log
            $stderr = if (Test-Path $log) { Get-Content $log -Raw } else { "" }
            if ($stderr -match "Job Object bind skipped") {
                $results.Add((Write-Result "#29" "pass" "observed best-effort fallback log"))
            } else {
                $results.Add((Write-Result "#29" "warn" "fallback not reproduced in current host (no debug log)"))
            }
        } catch {
            $results.Add((Write-Result "#29" "fail" $_.Exception.Message))
        }

        # #30 UAC 740 fallback path (manual due consent dialog)
        try {
            $case = "case30_uac"
            $exe = New-CaseBinary $case -Gui
            Write-Shim $case @"
type = exe
path = C:\Windows\System32\regedit.exe
wait = false
"@
            Write-Host ""
            Write-Host "[#30] UAC check: shim will run regedit.exe (requires elevation)."
            Write-Host "     If UAC prompt appears, try both 'No' and 'Yes' in separate runs."
            if (-not $uacEnabled) {
                Write-Host "     UAC is disabled (EnableLUA=0). Prompt path cannot be validated on this host."
                $results.Add((Write-Result "#30" "warn" "UAC disabled by system policy (EnableLUA=0); skipped"))
                throw [System.OperationCanceledException]::new("skip-#30")
            }
            if ($isAdmin) {
                Write-Host "     Current shell is Administrator; UAC prompt may NOT appear in this run."
            }
            $ans = Read-Host "Run this check now? (y/n)"
            if ($ans -match '^(y|yes)$') {
                & $exe
                $code = $LASTEXITCODE
                $uacShown = Read-Host "Did UAC prompt appear? (y/n)"
                if ($uacShown -match '^(y|yes)$') {
                    $results.Add((Write-Result "#30" "pass" "UAC prompt observed via shim (exit=$code)"))
                } else {
                    if ($isAdmin) {
                        $results.Add((Write-Result "#30" "warn" "UAC prompt not observed in Administrator shell (exit=$code)"))
                    } else {
                        $results.Add((Write-Result "#30" "fail" "UAC prompt not observed in non-admin shell (exit=$code)"))
                    }
                }
            } else {
                $results.Add((Write-Result "#30" "manual" "skipped by user; run later from $exe"))
            }
        } catch {
            if ($_.Exception -is [System.OperationCanceledException] -and $_.Exception.Message -eq "skip-#30") {
                # already recorded as warn
            } else {
            $results.Add((Write-Result "#30" "fail" $_.Exception.Message))
            }
        }
    } else {
        $results.Add((Write-Result "#28" "manual" "skipped (SkipManual)"))
        $results.Add((Write-Result "#29" "manual" "skipped (SkipManual)"))
        $results.Add((Write-Result "#30" "manual" "skipped (SkipManual)"))
    }

    $jsonPath = Join-Path $work "phase5-results.json"
    $results | ConvertTo-Json -Depth 5 | Set-Content -Path $jsonPath -Encoding UTF8

    Write-Host ""
    Write-Host "Phase 5 acceptance results:"
    $results | Format-Table -AutoSize | Out-String | Write-Host
    Write-Host "Artifacts: $work"
    Write-Host "Result JSON: $jsonPath"
}
finally {
    Pop-Location
}
