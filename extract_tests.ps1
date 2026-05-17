$content = Get-Content tests\core_integration.rs
if ($content.Length -lt 100) { throw "File too short, restoration failed" }

function Save-Range($start, $end, $name) {
    $content[($start-1)..($end-1)] | Set-Content "tests\core\$name.rs" -Encoding UTF8
}

Save-Range 8 913 "foundation"
Save-Range 1226 1364 "config"
Save-Range 1365 1557 "tree_find"
Save-Range 1558 2048 "ctx"
Save-Range 2049 2566 "port_proc"
Save-Range 2567 3012 "backup_video"
Save-Range 3013 4682 "app_part1"
Save-Range 4683 5782 "fs_part1"
Save-Range 5783 6100 "vault"
Save-Range 6101 7201 "misc"
Save-Range 7251 $content.Length "operations"

# Special for proxy
($content[913..1224] + $content[7201..7249]) | Set-Content tests\core\proxy.rs -Encoding UTF8
