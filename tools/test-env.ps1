# Xun CLI test environment bootstrap (isolated sandbox)
[CmdletBinding()]
param(
    [string]$Root = "",
    [string]$Bin = "",
    [string]$Features = "dashboard,redirect,protect,lock,crypt",
    [switch]$Build,
    [switch]$Release,
    [switch]$Reset,
    [switch]$NonInteractive
)

$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
if (-not $Root) {
    $Root = Join-Path $repo "target\xun-cli-env"
}

if ($Reset -and (Test-Path $Root)) {
    Remove-Item -Path $Root -Recurse -Force
}

if ($Build) {
    $args = @("build")
    if ($Release) { $args += "--release" }
    if ($Features) { $args += @("--features", $Features) }
    Push-Location $repo
    try { & cargo @args } finally { Pop-Location }
}

if (-not $Bin) {
    $candidate = if ($Release) {
        Join-Path $repo "target\release\xun.exe"
    } else {
        Join-Path $repo "target\debug\xun.exe"
    }
    if (Test-Path $candidate) {
        $Bin = $candidate
    } else {
        Write-Error "xun binary not found. Use -Build or -Bin to specify one."
        exit 1
    }
}

New-Item -ItemType Directory -Force -Path $Root | Out-Null

$paths = @{
    Root        = $Root
    Data        = Join-Path $Root "data"
    Project     = Join-Path $Root "project"
    RedirectSrc = Join-Path $Root "redirect-src"
    RedirectOut = Join-Path $Root "redirect-out"
    LockDir     = Join-Path $Root "lock"
    ProtectDir  = Join-Path $Root "protect"
    CryptDir    = Join-Path $Root "crypt"
    BakDir      = Join-Path $Root "bak-src"
    TreeDir     = Join-Path $Root "tree"
    ImportDir   = Join-Path $Root "import"
    OutDir      = Join-Path $Root "out"
}

$paths.Values | ForEach-Object { New-Item -ItemType Directory -Force -Path $_ | Out-Null }

New-Item -ItemType Directory -Force -Path (Join-Path $paths.Data "docs") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $paths.Project "src") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $paths.Project "assets") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $paths.TreeDir "a\b\c") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $paths.RedirectSrc "sub") | Out-Null

Set-Content -Path (Join-Path $paths.RedirectSrc "a.jpg") -Value "jpg"
Set-Content -Path (Join-Path $paths.RedirectSrc "b.png") -Value "png"
Set-Content -Path (Join-Path $paths.RedirectSrc "c.pdf") -Value "pdf"
Set-Content -Path (Join-Path $paths.RedirectSrc "d.docx") -Value "docx"
Set-Content -Path (Join-Path $paths.RedirectSrc "e.zip") -Value "zip"
Set-Content -Path (Join-Path $paths.RedirectSrc "sub\notes.txt") -Value "notes"

Set-Content -Path (Join-Path $paths.Project "README.md") -Value "# Demo Project"
Set-Content -Path (Join-Path $paths.Project "src\main.rs") -Value "fn main() {}"
Set-Content -Path (Join-Path $paths.Project "assets\logo.txt") -Value "logo"

Set-Content -Path (Join-Path $paths.Data "docs\guide.txt") -Value "guide"
Set-Content -Path (Join-Path $paths.TreeDir "a\b\c\deep.txt") -Value "deep"

Set-Content -Path (Join-Path $paths.LockDir "locked.txt") -Value "lock me"
Set-Content -Path (Join-Path $paths.ProtectDir "protected.txt") -Value "protected"
Set-Content -Path (Join-Path $paths.CryptDir "secret.txt") -Value "secret"

Set-Content -Path (Join-Path $paths.BakDir "app.conf") -Value "config"
Set-Content -Path (Join-Path $paths.BakDir "data.bin") -Value "data"

Set-Content -Path (Join-Path $paths.ImportDir "bookmarks.json") -Value '[{"name":"demo_import","path":"D:\\Demo\\Import","tags":["import"],"visits":1,"last_visited":0}]'

$now = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
$db = @{
    env      = @{ path = $paths.Root;        tags = @("env");      visit_count = 1;  last_visited = $now }
    data     = @{ path = $paths.Data;        tags = @("data");     visit_count = 5;  last_visited = $now }
    project  = @{ path = $paths.Project;     tags = @("work");     visit_count = 7;  last_visited = $now }
    redirect = @{ path = $paths.RedirectSrc; tags = @("redirect"); visit_count = 3;  last_visited = $now }
    crypt    = @{ path = $paths.CryptDir;    tags = @("secure");   visit_count = 1;  last_visited = $now }
    tree     = @{ path = $paths.TreeDir;     tags = @("tree");     visit_count = 2;  last_visited = $now }
}

$dbPath = Join-Path $Root ".xun.json"
$db | ConvertTo-Json -Depth 6 | Set-Content -Path $dbPath -Encoding UTF8

$cfg = @{
    tree = @{
        defaultDepth = 3
        excludeNames = @("node_modules", ".git")
    }
    proxy = @{
        defaultUrl = "http://127.0.0.1:7890"
        noproxy = "localhost,127.0.0.1,::1,.local"
    }
    redirect = @{
        profiles = @{
            default = @{
                rules = @(
                    @{ name = "Images"; match = @{ ext = @("jpg","png") }; dest = "Images" },
                    @{ name = "Docs"; match = @{ ext = @("pdf","docx","txt") }; dest = "Docs" },
                    @{ name = "Archives"; match = @{ glob = "*.zip" }; dest = "Archives" }
                )
                unmatched = "skip"
                on_conflict = "rename_new"
                recursive = $false
                max_depth = 2
            }
            media = @{
                rules = @(
                    @{ name = "Media"; match = @{ ext = @("jpg","png","gif") }; dest = "Media" }
                )
                unmatched = "skip"
                on_conflict = "rename_new"
                recursive = $false
                max_depth = 2
            }
            docs = @{
                rules = @(
                    @{ name = "DocsOnly"; match = @{ ext = @("pdf","docx") }; dest = "Docs" }
                )
                unmatched = "skip"
                on_conflict = "rename_new"
                recursive = $false
                max_depth = 2
            }
        }
    }
}

$cfgPath = Join-Path $Root ".xun.config.json"
$cfg | ConvertTo-Json -Depth 8 | Set-Content -Path $cfgPath -Encoding UTF8

$enterPath = Join-Path $Root "enter.ps1"
$enterContent = @"
`$env:XUN_EXE = "$Bin"
`$env:XUN_DB = "$dbPath"
`$env:XUN_CONFIG = "$cfgPath"
`$env:XUN_CTX_FILE = "$(Join-Path $Root ".xun.ctx.json")"
`$env:USERPROFILE = "$Root"
`$env:HOME = "$Root"
`$env:XUN_COMPLETE_CWD = "$($paths.Project)"
`$env:XUN_COMPLETE_SHELL = "pwsh"
if (-not `$env:XUN_CTX_STATE) { `$env:XUN_CTX_STATE = Join-Path `$env:TEMP ("xun-ctx-{0}.json" -f `$PID) }
$(if ($NonInteractive) { '`$env:XUN_NON_INTERACTIVE = "1"' } else { '' })
Set-Location "$Root"
Write-Host "Xun test env loaded."
Write-Host "Root: $Root"
"@

Set-Content -Path $enterPath -Value $enterContent -Encoding UTF8

Write-Host "Test env prepared:"
Write-Host "  Root: $Root"
Write-Host "  DB:   $dbPath"
Write-Host "  CFG:  $cfgPath"
Write-Host "  BIN:  $Bin"
Write-Host ""
Write-Host "Enter shell:"
Write-Host "  pwsh -NoExit -File $enterPath"
Write-Host ""
Write-Host "Hint: dot-source to keep env in current shell:"
Write-Host "  . .\\tools\\test-env.ps1 -Root `"$Root`" -Bin `"$Bin`""
