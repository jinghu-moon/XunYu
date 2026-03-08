# ============================================
# 文件树生成器 - PowerShell + C# 混合版本
# 利用 Add-Type 内嵌 C# 代码获得性能提升
# ============================================

<#
.SYNOPSIS
    生成目录树结构文件

.DESCRIPTION
    递归扫描指定路径并生成格式化的文件树，支持自定义排除规则和深度限制

.PARAMETER Path
    要扫描的目录路径（默认为当前目录）

.PARAMETER OutputFile
    输出文件名（默认为 fileTree.txt）

.PARAMETER MaxDepth
    最大递归深度，0 表示无限制（默认为 0）

.PARAMETER IncludeHidden
    是否包含隐藏文件和文件夹

.PARAMETER NoClipboard
    不复制到剪贴板

.EXAMPLE
    .\tree-hybrid.ps1
    在当前目录生成文件树

.EXAMPLE
    .\tree-hybrid.ps1 -Path "C:\Projects\MyApp"
    扫描指定路径

.EXAMPLE
    .\tree-hybrid.ps1 -Path "D:\Code" -MaxDepth 3 -OutputFile "structure.txt"
    扫描指定路径，限制深度为 3 层

.EXAMPLE
    .\tree-hybrid.ps1 -MaxDepth 5 -IncludeHidden
    包含隐藏文件，限制深度为 5 层
#>

param(
    [Parameter(Position = 0)]
    [ValidateScript({
        if (Test-Path $_ -PathType Container) {
            $true
        } else {
            throw "路径 '$_' 不存在或不是一个有效的目录"
        }
    })]
    [string]$Path = (Get-Location).Path,
    
    [Parameter()]
    [string]$OutputFile = "fileTree.txt",
    
    [Parameter()]
    [ValidateRange(0, [int]::MaxValue)]
    [int]$MaxDepth = 0,
    
    [Parameter()]
    [switch]$IncludeHidden,
    
    [Parameter()]
    [switch]$NoClipboard
)

# --- C# 代码定义 ---
$csharpCode = @"
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;

namespace FileTreeGenerator
{
    public class TreeGenerator
    {
        private static readonly HashSet<string> ExcludeNames = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            ".vscode", "dist", "node_modules", "public",
            "icons", "debug", "release", "bin", "obj",
            "sv.ps1", "fileTree.txt", "tree.ps1",
            "test.svg", "gen", "workers",
            "README.md", ".gitignore", "target",
            ".spec-workflow", ".trae", "script", ".agent",
            ".git", ".vs"
        };

        private static readonly string[] ExcludePaths = 
        {
            "src/assets"
        };

        private static readonly HashSet<string> ExcludeExtensions = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            ".dll", ".exe", ".obj", ".pdb", ".ilk"
        };

        private static readonly Regex VersionPattern = new Regex(@"^v\d+", RegexOptions.Compiled);

        private string rootPath;
        private int maxDepth;
        private bool includeHidden;

        public TreeGenerator(string path, int depth, bool hidden)
        {
            rootPath = path;
            maxDepth = depth;
            includeHidden = hidden;
        }

        private bool ShouldExclude(FileSystemInfo item)
        {
            var name = item.Name;

            // 隐藏文件处理
            if (!includeHidden && name.StartsWith("."))
            {
                return true;
            }

            // 名称排除
            if (ExcludeNames.Contains(name))
            {
                return true;
            }

            // 路径排除
            foreach (var excludePath in ExcludePaths)
            {
                var fullExcludePath = Path.Combine(rootPath, excludePath);
                if (item.FullName.StartsWith(fullExcludePath, StringComparison.OrdinalIgnoreCase))
                {
                    return true;
                }
            }

            // 版本目录排除
            if (item is DirectoryInfo && VersionPattern.IsMatch(name))
            {
                return true;
            }

            // 扩展名排除
            if (item is FileInfo fileInfo && ExcludeExtensions.Contains(fileInfo.Extension))
            {
                return true;
            }

            return false;
        }

        private void GenerateTreeInternal(string path, string prefix, int currentDepth, List<string> lines)
        {
            // 深度限制检查
            if (maxDepth > 0 && currentDepth > maxDepth)
            {
                return;
            }

            try
            {
                var dirInfo = new DirectoryInfo(path);
                var entries = dirInfo.GetFileSystemInfos()
                    .Where(item => !ShouldExclude(item))
                    .OrderByDescending(item => item is DirectoryInfo)
                    .ThenBy(item => item.Name)
                    .ToList();

                int count = entries.Count;

                for (int i = 0; i < count; i++)
                {
                    var item = entries[i];
                    bool isLast = (i == count - 1);

                    string branch = isLast ? "└──" : "├──";
                    string childPrefix = isLast ? "    " : "│   ";

                    if (item is DirectoryInfo)
                    {
                        lines.Add(prefix + branch + " " + item.Name + "/");
                        GenerateTreeInternal(item.FullName, prefix + childPrefix, currentDepth + 1, lines);
                    }
                    else
                    {
                        lines.Add(prefix + branch + " " + item.Name);
                    }
                }
            }
            catch (UnauthorizedAccessException)
            {
                // 静默处理权限错误
            }
            catch (Exception)
            {
                // 静默处理其他错误
            }
        }

        public List<string> Generate()
        {
            var lines = new List<string>();
            var rootDir = new DirectoryInfo(rootPath);
            lines.Add(rootDir.Name + "/");
            GenerateTreeInternal(rootPath, "", 1, lines);
            return lines;
        }
    }
}
"@

# --- 编译 C# 代码 ---
Write-Host "正在初始化 C# 引擎..." -ForegroundColor Cyan

try {
    Add-Type -TypeDefinition $csharpCode -Language CSharp -ErrorAction Stop
    Write-Host "✓ C# 引擎初始化成功" -ForegroundColor Green
} catch {
    Write-Host "✗ C# 编译失败，回退到纯 PowerShell 模式" -ForegroundColor Yellow
    Write-Host "错误: $($_.Exception.Message)" -ForegroundColor Red
    
    # 这里可以添加纯 PowerShell 的备用实现
    # 为了简洁，此示例不包含备用实现
    exit 1
}

Write-Host ""

# --- 使用 C# 生成文件树 ---
# 规范化路径
$targetPath = Resolve-Path $Path | Select-Object -ExpandProperty Path

Write-Host "正在生成文件树..." -ForegroundColor Cyan
Write-Host "扫描路径: $targetPath" -ForegroundColor Gray
if ($MaxDepth -gt 0) {
    Write-Host "最大深度: $MaxDepth" -ForegroundColor Gray
}
Write-Host ""

$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

# 创建 C# 对象并调用
$generator = New-Object FileTreeGenerator.TreeGenerator($targetPath, $MaxDepth, $IncludeHidden.IsPresent)
$treeLines = $generator.Generate()

$stopwatch.Stop()

# --- 输出结果 ---
# 输出到控制台
$treeLines | ForEach-Object { Write-Host $_ }

# 保存到文件
try {
    $treeLines | Out-File -FilePath $OutputFile -Encoding UTF8
    Write-Host ""
    Write-Host "✓ 成功保存到 `"$OutputFile`"" -ForegroundColor Green
} catch {
    Write-Host ""
    Write-Host "✗ 保存文件失败: $($_.Exception.Message)" -ForegroundColor Red
}

# 复制到剪贴板
if (-not $NoClipboard) {
    try {
        $treeLines | Set-Clipboard
        Write-Host "✓ 已复制到剪贴板" -ForegroundColor Green
    } catch {
        Write-Host "⚠ 无法复制到剪贴板" -ForegroundColor Yellow
    }
}

# 显示统计信息
Write-Host "  - 扫描路径: $targetPath" -ForegroundColor Cyan
Write-Host "  - 总计: $($treeLines.Count) 行" -ForegroundColor Cyan
Write-Host "  - 耗时: $($stopwatch.ElapsedMilliseconds) ms" -ForegroundColor Cyan

if ($MaxDepth -gt 0) {
    Write-Host "  - 最大深度: $MaxDepth" -ForegroundColor Cyan
}

Write-Host ""