$ErrorActionPreference = 'Stop'

$sevenZip = 'C:\A_Softwares\7-Zip\7z.exe'
$fixtureDir = Join-Path $PSScriptRoot '..\tests\fixtures\xunbak_sample'
$debugExe = Join-Path $PSScriptRoot '..\target\debug\xun.exe'
$releaseExe = Join-Path $PSScriptRoot '..\target\release\xun.exe'
$cargoToml = Join-Path $PSScriptRoot '..\Cargo.toml'
$xun = $null

$logDir = Join-Path $PSScriptRoot '..\logs'
$logPath = Join-Path $logDir 'compat_7zip_24_20260324.md'
if (!(Test-Path $logDir)) {
    New-Item -ItemType Directory -Path $logDir | Out-Null
}

$work = Join-Path $env:TEMP ('xun-7zip-compat-' + [guid]::NewGuid().ToString())
$src = Join-Path $work 'src'
$includeRoots = @('README.md', 'empty.txt', '中文目录', 'path with spaces', 'deep', 'config', 'docs', 'src', 'assets')
New-Item -ItemType Directory -Path $work | Out-Null
$null = robocopy $fixtureDir $src /E /COPY:DAT /R:0 /W:0 /NFL /NDL /NJH /NJS /NP
if ($LASTEXITCODE -gt 7) {
    throw "robocopy failed: $LASTEXITCODE"
}

$cfg = @"
{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "fixture" },
  "retention": { "maxBackups": 20, "deleteCount": 1 },
  "include": [ "README.md", "empty.txt", "中文目录", "path with spaces", "deep", "config", "docs", "src", "assets" ],
  "exclude": []
}
"@
Set-Content -Path (Join-Path $src '.xun-bak.json') -Value $cfg -NoNewline

function Run-Xun([string[]]$argv) {
    $env:XUN_DB = Join-Path $work '.xun.json'
    $env:USERPROFILE = $work
    $env:HOME = $work
    $env:XUN_NON_INTERACTIVE = '1'
    $output = & $xun @argv 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "xun failed: $($argv -join ' ')`n$output"
    }
}

function Resolve-XunBinary() {
    if ($env:XUN_BIN) {
        return $env:XUN_BIN
    }
    if (!(Test-Path $debugExe) -and !(Test-Path $releaseExe)) {
        throw 'xun binary not found; set XUN_BIN or build target/debug/xun.exe'
    }
    if ((Test-Path $cargoToml) -and (!(Test-Path $debugExe) -or ((Get-Item $cargoToml).LastWriteTimeUtc -gt (Get-Item $debugExe).LastWriteTimeUtc))) {
        Push-Location (Join-Path $PSScriptRoot '..')
        try {
            & cargo build --bin xun | Out-Null
            if ($LASTEXITCODE -ne 0) {
                throw 'cargo build --bin xun failed'
            }
        } finally {
            Pop-Location
        }
    }
    if (Test-Path $debugExe) {
        return $debugExe
    }
    return $releaseExe
}

function Get-FileIndex([string]$root, [string[]]$allowedPaths = @()) {
    $index = @{}
    if (!(Test-Path $root)) {
        return $index
    }
    $allowed = @{}
    foreach ($path in $allowedPaths) {
        $allowed[$path] = $true
    }
    $resolvedRoot = (Resolve-Path -LiteralPath $root).Path
    Get-ChildItem -LiteralPath $root -Recurse -Force -File | ForEach-Object {
        $full = (Resolve-Path -LiteralPath $_.FullName).Path
        $rel = $full.Substring($resolvedRoot.Length).TrimStart('\').Replace('\', '/')
        if ($rel.StartsWith('__xunyu__/')) { return }
        if ($allowed.Count -gt 0 -and -not $allowed.ContainsKey($rel)) { return }
        $index[$rel] = [PSCustomObject]@{
            Rel = $rel
            Hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
            Length = $_.Length
        }
    }
    return $index
}

function Get-ExpectedIndexFromManifest([string]$extractRoot, [string]$sourceRoot) {
    $manifestPath = Join-Path $extractRoot '__xunyu__/export_manifest.json'
    if (!(Test-Path $manifestPath)) {
        $manifestCandidate = Get-ChildItem -LiteralPath $extractRoot -Recurse -Force -File -Filter 'export_manifest.json' -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName.Replace('\', '/').EndsWith('__xunyu__/export_manifest.json') } |
            Select-Object -First 1
        if ($null -eq $manifestCandidate) {
            return $null
        }
        $manifestPath = $manifestCandidate.FullName
    }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    $paths = @($manifest.entries | ForEach-Object { $_.path })
    return @{
        Paths = $paths
        Expected = (Get-FileIndex $sourceRoot $paths)
    }
}

function Compare-Indexes($expected, $actual) {
    $missing = New-Object System.Collections.Generic.List[string]
    $unexpected = New-Object System.Collections.Generic.List[string]
    $changed = New-Object System.Collections.Generic.List[string]

    foreach ($path in $expected.Keys) {
        if (-not $actual.ContainsKey($path)) {
            $missing.Add($path)
            continue
        }
        if ($expected[$path].Hash -ne $actual[$path].Hash -or $expected[$path].Length -ne $actual[$path].Length) {
            $changed.Add($path)
        }
    }
    foreach ($path in $actual.Keys) {
        if (-not $expected.ContainsKey($path)) {
            $unexpected.Add($path)
        }
    }
    return [PSCustomObject]@{
        Passed = ($missing.Count -eq 0 -and $unexpected.Count -eq 0 -and $changed.Count -eq 0)
        Missing = @($missing)
        Unexpected = @($unexpected)
        Changed = @($changed)
    }
}

function Get-SelectedSourceIndex([string]$root, [string[]]$includes) {
    $index = @{}
    foreach ($include in $includes) {
        $full = Join-Path $root $include
        if (Test-Path -LiteralPath $full -PathType Leaf) {
            $fileIndex = Get-FileIndex $root @($include.Replace('\', '/'))
            foreach ($key in $fileIndex.Keys) {
                $index[$key] = $fileIndex[$key]
            }
            continue
        }
        if (Test-Path -LiteralPath $full -PathType Container) {
            $dirIndex = Get-FileIndex $full
            foreach ($key in $dirIndex.Keys) {
                $relative = ($include.TrimEnd('/', '\') + '/' + $key).Replace('\', '/')
                $index[$relative] = [PSCustomObject]@{
                    Rel = $relative
                    Hash = $dirIndex[$key].Hash
                    Length = $dirIndex[$key].Length
                }
            }
        }
    }
    return $index
}

function Invoke-7Zip([string[]]$args) {
    $output = & $sevenZip @args 2>&1
    [PSCustomObject]@{
        ExitCode = $LASTEXITCODE
        Output = ($output | Out-String).Trim()
    }
}

function Test-ExtractRoundtrip([string]$archivePath) {
    $temp = Join-Path $env:TEMP ('xun-7zip-extract-' + [guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $temp | Out-Null
    $extract = Join-Path $temp 'out'
    $result = Invoke-7Zip @('x', $archivePath, "-o$extract", '-y')
    $passed = $false
    $diffSummary = ''
    if ($result.ExitCode -eq 0) {
        $expectedInfo = Get-ExpectedIndexFromManifest $extract $src
        if ($null -ne $expectedInfo) {
            $actualIndex = Get-FileIndex $extract $expectedInfo.Paths
            $comparison = Compare-Indexes $expectedInfo.Expected $actualIndex
        } else {
            $expectedIndex = Get-SelectedSourceIndex $src $includeRoots
            $actualIndex = Get-FileIndex $extract
            $comparison = Compare-Indexes $expectedIndex $actualIndex
            $diffSummary = 'manifest missing'
        }
        $passed = $comparison.Passed
        if (-not $passed) {
            $parts = @()
            if ($diffSummary) {
                $parts += $diffSummary
            }
            if ($comparison.Missing.Count -gt 0) {
                $parts += ('missing=' + ($comparison.Missing -join ', '))
            }
            if ($comparison.Unexpected.Count -gt 0) {
                $parts += ('unexpected=' + ($comparison.Unexpected -join ', '))
            }
            if ($comparison.Changed.Count -gt 0) {
                $parts += ('changed=' + ($comparison.Changed -join ', '))
            }
            $diffSummary = $parts -join ' | '
        } elseif ($diffSummary) {
            $diffSummary = $diffSummary + ' | fallback compare passed'
        }
    }
    Remove-Item -Recurse -Force $temp
    [PSCustomObject]@{
        ExitCode = $result.ExitCode
        Passed = $passed
        Diff = $diffSummary
        Output = $result.Output
    }
}

$xun = Resolve-XunBinary

$zipPath = Join-Path $work 'fixture.zip'
$sevenPath = Join-Path $work 'fixture.7z'
$splitSevenPath = Join-Path $work 'fixture_split.7z'
Run-Xun @('backup', 'create', '-C', $src, '--format', 'zip', '-o', $zipPath)
Run-Xun @('backup', 'create', '-C', $src, '--format', '7z', '-o', $sevenPath)
Run-Xun @('backup', 'create', '-C', $src, '--format', '7z', '-o', $splitSevenPath, '--split-size', '40000')

$listZip = Invoke-7Zip @('l', $zipPath)
$testZip = Invoke-7Zip @('t', $zipPath)
$extractZip = Test-ExtractRoundtrip $zipPath

$list7z = Invoke-7Zip @('l', $sevenPath)
$test7z = Invoke-7Zip @('t', $sevenPath)
$extract7z = Test-ExtractRoundtrip $sevenPath

$splitFirst = "$splitSevenPath.001"
$listSplit7z = Invoke-7Zip @('l', $splitFirst)
$testSplit7z = Invoke-7Zip @('t', $splitFirst)
$extractSplit7z = Test-ExtractRoundtrip $splitFirst

$lines = @(
    '# 7-Zip 24.x Compatibility',
    '',
    '- Date: 2026-03-24',
    ('- Binary: ' + $sevenZip),
    ('- Source fixture: ' + $fixtureDir),
    ('- Workdir: ' + $work),
    '',
    '| Scenario | ExitCode | Passed |',
    '|---|---:|---|',
    ('| 7-Zip 24.x list ZIP | ' + $listZip.ExitCode + ' | ' + ($(if ($listZip.ExitCode -eq 0) { 'yes' } else { 'no' })) + ' |'),
    ('| 7-Zip 24.x test ZIP | ' + $testZip.ExitCode + ' | ' + ($(if ($testZip.ExitCode -eq 0) { 'yes' } else { 'no' })) + ' |'),
    ('| 7-Zip 24.x extract ZIP | ' + $extractZip.ExitCode + ' | ' + ($(if ($extractZip.Passed) { 'yes' } else { 'no' })) + ' |'),
    ('| 7-Zip 24.x list 7z | ' + $list7z.ExitCode + ' | ' + ($(if ($list7z.ExitCode -eq 0) { 'yes' } else { 'no' })) + ' |'),
    ('| 7-Zip 24.x test 7z | ' + $test7z.ExitCode + ' | ' + ($(if ($test7z.ExitCode -eq 0) { 'yes' } else { 'no' })) + ' |'),
    ('| 7-Zip 24.x extract 7z | ' + $extract7z.ExitCode + ' | ' + ($(if ($extract7z.Passed) { 'yes' } else { 'no' })) + ' |'),
    ('| 7-Zip 24.x list split 7z | ' + $listSplit7z.ExitCode + ' | ' + ($(if ($listSplit7z.ExitCode -eq 0) { 'yes' } else { 'no' })) + ' |'),
    ('| 7-Zip 24.x test split 7z | ' + $testSplit7z.ExitCode + ' | ' + ($(if ($testSplit7z.ExitCode -eq 0) { 'yes' } else { 'no' })) + ' |'),
    ('| 7-Zip 24.x extract split 7z | ' + $extractSplit7z.ExitCode + ' | ' + ($(if ($extractSplit7z.Passed) { 'yes' } else { 'no' })) + ' |'),
    '',
    '## Extract Diffs',
    '',
    ('- ZIP: ' + $(if ($extractZip.Diff) { $extractZip.Diff } else { 'none' })),
    ('- 7z: ' + $(if ($extract7z.Diff) { $extract7z.Diff } else { 'none' })),
    ('- split 7z: ' + $(if ($extractSplit7z.Diff) { $extractSplit7z.Diff } else { 'none' }))
)
$lines | Set-Content -Path $logPath
Write-Output "Wrote $logPath"
