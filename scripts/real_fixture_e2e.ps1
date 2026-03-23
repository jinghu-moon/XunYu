$ErrorActionPreference = 'Stop'

$fixture = Join-Path $PSScriptRoot '..\tests\fixtures\xunbak_sample'
$debugExe = Join-Path $PSScriptRoot '..\target\debug\xun.exe'
$releaseExe = Join-Path $PSScriptRoot '..\target\release\xun.exe'
$exe = if ($env:XUN_BIN) {
    $env:XUN_BIN
} elseif (Test-Path $debugExe) {
    $debugExe
} else {
    $releaseExe
}

$logDir = Join-Path $PSScriptRoot '..\logs'
$logPath = Join-Path $logDir 'real_fixture_e2e_20260323.md'
if (!(Test-Path $logDir)) {
    New-Item -ItemType Directory -Path $logDir | Out-Null
}

$work = Join-Path $env:TEMP ('xun-real-fixture-' + [guid]::NewGuid().ToString())
$src = Join-Path $work 'src'
New-Item -ItemType Directory -Path $work | Out-Null
$null = robocopy $fixture $src /E /COPY:DAT /R:0 /W:0 /NFL /NDL /NJH /NJS /NP
if ($LASTEXITCODE -gt 7) {
    throw "robocopy failed: $LASTEXITCODE"
}

$config = @"
{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "fixture" },
  "retention": { "maxBackups": 20, "deleteCount": 1 },
  "include": [ "Cargo.toml", "README.md", "empty.txt", "FIXTURE_README.md", "src", "config", "docs", "assets", "中文目录", "path with spaces", "deep", "empty_dir" ],
  "exclude": []
}
"@
Set-Content -Path (Join-Path $src '.xun-bak.json') -Value $config -NoNewline

function Run-Xun([string[]]$argv) {
    $env:XUN_DB = Join-Path $work '.xun.json'
    $env:USERPROFILE = $work
    $env:HOME = $work
    $env:XUN_NON_INTERACTIVE = '1'
    $output = & $exe @argv 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "xun failed: $($argv -join ' ')`n$output"
    }
}

function Get-TreeState([string]$root) {
    $items = Get-ChildItem -LiteralPath $root -Recurse -Force -File |
        Where-Object {
            $_.Name -ne '.xun-bak.json' -and
            $_.FullName.Replace('\', '/').EndsWith('__xunyu__/export_manifest.json') -eq $false
        }
    $rows = foreach ($item in $items) {
        $rel = $item.FullName.Substring($root.Length).TrimStart('\').Replace('\', '/')
        $hash = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash
        [PSCustomObject]@{
            Rel = $rel
            Hash = $hash
            Length = $item.Length
        }
    }
    $rows | Sort-Object Rel
}

function TreesEqual([string]$expectedRoot, [string]$actualRoot) {
    (Get-TreeState $expectedRoot | ConvertTo-Json -Depth 3) -eq
        (Get-TreeState $actualRoot | ConvertTo-Json -Depth 3)
}

function ReadonlyMatch([string]$expectedRoot, [string]$actualRoot, [string]$rel) {
    $expected = (Get-Item -LiteralPath (Join-Path $expectedRoot $rel) -Force).Attributes.ToString().Contains('ReadOnly')
    $actual = (Get-Item -LiteralPath (Join-Path $actualRoot $rel) -Force).Attributes.ToString().Contains('ReadOnly')
    $expected -eq $actual
}

$results = [System.Collections.Generic.List[object]]::new()

$zipOut = Join-Path $work 'fixture.zip'
Run-Xun @('backup', 'create', '-C', $src, '--format', 'zip', '-o', $zipOut)
$zipRestore = Join-Path $work 'zip_restore'
Run-Xun @('backup', 'restore', $zipOut, '--to', $zipRestore, '-C', $src, '-y')
$results.Add([pscustomobject]@{
    Scenario = 'create zip -> restore'
    Passed = TreesEqual $src $zipRestore
    Note = 'content/tree'
})

$sevenOut = Join-Path $work 'fixture.7z'
Run-Xun @('backup', 'create', '-C', $src, '--format', '7z', '-o', $sevenOut)
$sevenRestore = Join-Path $work 'seven_restore'
Run-Xun @('backup', 'restore', $sevenOut, '--to', $sevenRestore, '-C', $src, '-y')
$results.Add([pscustomobject]@{
    Scenario = 'create 7z -> restore'
    Passed = (TreesEqual $src $sevenRestore) -and (ReadonlyMatch $src $sevenRestore 'config/readonly_file.txt')
    Note = 'content/tree + readonly'
})

$split7zOut = Join-Path $work 'fixture_split.7z'
Run-Xun @('backup', 'create', '-C', $src, '--format', '7z', '-o', $split7zOut, '--split-size', '40000')
$split7zRestore = Join-Path $work 'split7z_restore'
Run-Xun @('backup', 'restore', $split7zOut, '--to', $split7zRestore, '-C', $src, '-y')
$results.Add([pscustomobject]@{
    Scenario = 'create split 7z -> restore'
    Passed = TreesEqual $src $split7zRestore
    Note = 'base-path restore'
})

$xunbakOut = Join-Path $work 'fixture.xunbak'
Run-Xun @('backup', '-C', $src, '--container', $xunbakOut, '--compression', 'none', '-m', 'fixture')
$xunRestore = Join-Path $work 'xun_restore'
Run-Xun @('backup', 'restore', $xunbakOut, '--to', $xunRestore, '-C', $src, '-y')
$results.Add([pscustomobject]@{
    Scenario = 'create xunbak -> restore'
    Passed = (TreesEqual $src $xunRestore) -and (ReadonlyMatch $src $xunRestore 'config/readonly_file.txt')
    Note = 'content/tree + readonly'
})

$splitXunOut = Join-Path $work 'fixture_split.xunbak'
Run-Xun @('backup', '-C', $src, '--container', $splitXunOut, '--compression', 'none', '--split-size', '180000', '-m', 'fixture')
$splitXunRestore = Join-Path $work 'splitxun_restore'
Run-Xun @('backup', 'restore', $splitXunOut, '--to', $splitXunRestore, '-C', $src, '-y')
$results.Add([pscustomobject]@{
    Scenario = 'create split xunbak -> restore'
    Passed = TreesEqual $src $splitXunRestore
    Note = 'base-path restore'
})

Set-Content -Path (Join-Path $src 'docs/sample.log') -Value ((Get-Content -Path (Join-Path $src 'docs/sample.log') -Raw) + "`nUPDATED-FIXTURE-LINE") -NoNewline
Set-Content -Path (Join-Path $src 'src/main.rs') -Value ((Get-Content -Path (Join-Path $src 'src/main.rs') -Raw) + "`n// incremental update") -NoNewline
Set-Content -Path (Join-Path $src 'new_added.txt') -Value 'new file for incremental' -NoNewline
Remove-Item -LiteralPath (Join-Path $src 'docs/duplicate_b.txt') -Force

$xunUpdateRestore = Join-Path $work 'xun_update_restore'
Run-Xun @('backup', '-C', $src, '--container', $xunbakOut, '--compression', 'none', '-m', 'fixture-update')
Run-Xun @('backup', 'restore', $xunbakOut, '--to', $xunUpdateRestore, '-C', $src, '-y')
$results.Add([pscustomobject]@{
    Scenario = 'incremental xunbak update -> restore'
    Passed = TreesEqual $src $xunUpdateRestore
    Note = 'modify/add/delete'
})

$splitXunUpdateRestore = Join-Path $work 'splitxun_update_restore'
Run-Xun @('backup', '-C', $src, '--container', $splitXunOut, '--compression', 'none', '--split-size', '180000', '-m', 'fixture-update-split')
Run-Xun @('backup', 'restore', $splitXunOut, '--to', $splitXunUpdateRestore, '-C', $src, '-y')
$results.Add([pscustomobject]@{
    Scenario = 'incremental split xunbak update -> restore'
    Passed = TreesEqual $src $splitXunUpdateRestore
    Note = 'modify/add/delete'
})

$convZip = Join-Path $work 'converted_from_xunbak.zip'
Run-Xun @('backup', 'convert', $xunbakOut, '--format', 'zip', '-o', $convZip)
$convZipRestore = Join-Path $work 'conv_zip_restore'
Run-Xun @('backup', 'restore', $convZip, '--to', $convZipRestore, '-C', $src, '-y')
$results.Add([pscustomobject]@{
    Scenario = 'convert xunbak -> zip -> restore'
    Passed = TreesEqual $src $convZipRestore
    Note = 'latest snapshot'
})

$conv7z = Join-Path $work 'converted_from_xunbak.7z'
Run-Xun @('backup', 'convert', $xunbakOut, '--format', '7z', '-o', $conv7z)
$conv7zRestore = Join-Path $work 'conv_7z_restore'
Run-Xun @('backup', 'restore', $conv7z, '--to', $conv7zRestore, '-C', $src, '-y')
$results.Add([pscustomobject]@{
    Scenario = 'convert xunbak -> 7z -> restore'
    Passed = TreesEqual $src $conv7zRestore
    Note = 'latest snapshot'
})

$lines = @(
    '# Real Fixture E2E Test',
    '',
    '- Date: 2026-03-23',
    ('- Fixture: ' + $fixture),
    ('- Workdir: ' + $work),
    '',
    '| Scenario | Passed | Note |',
    '|---|---|---|'
)
foreach ($r in $results) {
    $lines += ('| ' + $r.Scenario + ' | ' + ($(if ($r.Passed) { 'yes' } else { 'no' })) + ' | ' + $r.Note + ' |')
}
$lines | Set-Content -Path $logPath
$results | Format-Table -AutoSize | Out-String
