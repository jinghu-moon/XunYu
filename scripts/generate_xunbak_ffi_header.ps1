param(
    [string]$Output = (Join-Path (Resolve-Path (Join-Path $PSScriptRoot '..')) 'cpp/xunbak-7z-plugin/xunbak_ffi_generated.h')
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$crateRoot = Join-Path $repoRoot 'crates/xunbak-7z-core'
$localCbindgen = Join-Path $repoRoot '.tools/cbindgen/bin/cbindgen.exe'
$cbindgen = $null

if (Test-Path $localCbindgen) {
    $cbindgen = $localCbindgen
} else {
    $cmd = Get-Command cbindgen -ErrorAction SilentlyContinue
    if ($cmd) {
        $cbindgen = $cmd.Source
    }
}

if (-not $cbindgen) {
    throw 'cbindgen not found. Install local tool with: cargo install cbindgen --root .tools/cbindgen'
}

Push-Location $crateRoot
try {
    & $cbindgen --config 'cbindgen.toml' --output $Output
    if ($LASTEXITCODE -ne 0) {
        throw 'cbindgen generation failed'
    }
}
finally {
    Pop-Location
}

Write-Host "Generated header: $Output"
