function Get-XunbakPluginTreeHashMap {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root
    )

    $map = @{}
    if (!(Test-Path $Root)) {
        return $map
    }

    $resolved = (Resolve-Path -LiteralPath $Root).Path
    Get-ChildItem -LiteralPath $Root -Recurse -Force -File | ForEach-Object {
        $full = (Resolve-Path -LiteralPath $_.FullName).Path
        $rel = $full.Substring($resolved.Length).TrimStart('\').Replace('\', '/')
        $map[$rel] = [PSCustomObject]@{
            Hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
            Length = $_.Length
        }
    }
    return $map
}

function Initialize-XunbakPluginFixture {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourceRoot
    )

    $srcDir = Join-Path $SourceRoot 'src'
    New-Item -ItemType Directory -Path (Join-Path $srcDir 'src') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $srcDir 'nested') -Force | Out-Null
    Set-Content -Path (Join-Path $srcDir 'README.md') -Value "plugin smoke`n" -NoNewline
    Set-Content -Path (Join-Path $srcDir 'src/a.txt') -Value ('alpha-' * 40) -NoNewline
    Set-Content -Path (Join-Path $srcDir 'nested/深层.txt') -Value ('beta-' * 55) -NoNewline
    Set-Content -Path (Join-Path $srcDir '.xun-bak.json') -Value '{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"plugin"},"retention":{"maxBackups":5,"deleteCount":1},"include":["README.md","src","nested"],"exclude":[]}' -NoNewline
    return $srcDir
}
