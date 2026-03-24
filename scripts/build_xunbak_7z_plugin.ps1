param(
    [ValidateSet('Debug', 'Release')]
    [string]$Config = 'Debug'
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$buildDir = Join-Path $repoRoot 'build/xunbak-7z-plugin'

Push-Location $repoRoot
try {
    & (Join-Path $repoRoot 'scripts/generate_xunbak_ffi_header.ps1')
    if ($LASTEXITCODE -ne 0) {
        throw 'generate_xunbak_ffi_header.ps1 failed'
    }

    cargo build -p xunbak-7z-core
    if ($LASTEXITCODE -ne 0) {
        throw 'cargo build -p xunbak-7z-core failed'
    }

    cmake -S 'cpp/xunbak-7z-plugin' -B $buildDir
    if ($LASTEXITCODE -ne 0) {
        throw 'cmake configure failed'
    }

    cmake --build $buildDir --config $Config
    if ($LASTEXITCODE -ne 0) {
        throw 'cmake build failed'
    }
}
finally {
    Pop-Location
}

Write-Host "Built xunbak plugin: $buildDir/$Config/xunbak.dll"
