param(
    [string]$m = ""
)

$ErrorActionPreference = "Stop"

# ==========================================
# 1. C# 全能内核 (Ultimate Engine)
# ==========================================
$SourceCode = @"
using System;
using System.IO;
using System.IO.Compression; 
using System.Collections; 

public class SmartBackup {
    public static int CountNew = 0;
    public static int CountMod = 0;
    public static int CountDel = 0;
    public static int CountSame = 0;

    struct Meta {
        public long Size;
        public DateTime Time;
        public Meta(long s, DateTime t) { Size = s; Time = t; }
    }

    private static string Norm(string path) {
        return path?.Trim().Replace("/", "\\").Trim('\\');
    }

    private static string FmtSize(long bytes) {
        if (bytes == 0) return "    +0.0 B";
        string[] suf = { "B", "KB", "MB" };
        int place = Convert.ToInt32(Math.Floor(Math.Log(Math.Abs(bytes), 1024)));
        double num = Math.Round(Math.Abs(bytes) / Math.Pow(1024, place), 1);
        double val = Math.Sign(bytes) * num;
        return string.Format("{0,8:+0.0;-0.0;+0.0} {1}", val, suf[place]);
    }

    // 新增：分离路径和文件名的着色输出方法
    private static void PrintPathWithColoredFilename(string fullPath) {
        int lastSlash = fullPath.LastIndexOf('\\');
        if (lastSlash >= 0) {
            // 路径部分 (浅色 DarkGray)
            string pathPart = fullPath.Substring(0, lastSlash + 1);
            System.Console.ForegroundColor = ConsoleColor.DarkGray;
            System.Console.Write(pathPart);
            
            // 文件名部分 (黄色)
            string fileName = fullPath.Substring(lastSlash + 1);
            System.Console.ForegroundColor = ConsoleColor.Yellow;
            System.Console.Write(fileName);
        } else {
            // 没有路径分隔符，整个就是文件名
            System.Console.ForegroundColor = ConsoleColor.Yellow;
            System.Console.Write(fullPath);
        }
    }

    public static void Run(string rootPath, string[] includes, string[] excludes, string destBaseDir, string prevPath) {
        CountNew = 0; CountMod = 0; CountDel = 0; CountSame = 0;

        // --- 1. 处理排除项 ---
        var excludeSet = new ArrayList();
        if (excludes != null) {
            foreach (var ex in excludes) {
                string val = Norm(ex);
                if (!string.IsNullOrEmpty(val)) excludeSet.Add(val);
            }
        }
        
        // --- 2. 扫描当前项目文件 (Source) ---
        var currentFiles = new Hashtable(StringComparer.OrdinalIgnoreCase);

        void ScanDir(string dir) {
            try {
                var entries = Directory.GetFileSystemEntries(dir);
                foreach (var entry in entries) {
                    string relPath = entry.Substring(rootPath.Length);
                    string normRel = Norm(relPath);
                    string name = Path.GetFileName(entry);

                    bool isExcluded = false;
                    foreach (var obj in excludeSet) {
                        string ex = (string)obj;
                        if (normRel.IndexOf(ex, StringComparison.OrdinalIgnoreCase) >= 0 || 
                            name.Equals(ex, StringComparison.OrdinalIgnoreCase)) {
                            isExcluded = true; break;
                        }
                    }
                    if (isExcluded) continue;

                    if (Directory.Exists(entry)) ScanDir(entry);
                    else currentFiles[normRel] = new FileInfo(entry);
                }
            } catch {}
        }

        if (includes != null) {
            foreach (var inc in includes) {
                string full = Path.Combine(rootPath, inc);
                if (File.Exists(full)) currentFiles[Norm(inc)] = new FileInfo(full);
                else if (Directory.Exists(full)) ScanDir(full);
            }
        }

        // --- 3. 读取旧版本基准 (Baseline) ---
        var oldFiles = new Hashtable(StringComparer.OrdinalIgnoreCase);
        
        if (!string.IsNullOrEmpty(prevPath)) {
            try {
                if (File.Exists(prevPath)) {
                    using (FileStream fs = File.OpenRead(prevPath))
                    using (ZipArchive archive = new ZipArchive(fs, ZipArchiveMode.Read)) {
                        foreach (var entry in archive.Entries) {
                            string normKey = Norm(entry.FullName);
                            if (!string.IsNullOrEmpty(normKey) && !entry.FullName.EndsWith("/")) {
                                oldFiles[normKey] = new Meta(entry.Length, entry.LastWriteTime.DateTime);
                            }
                        }
                    }
                }
                else if (Directory.Exists(prevPath)) {
                    string cleanPrevPath = Path.GetFullPath(prevPath).TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
                    
                    void ScanOldDir(string dir) {
                        var files = Directory.GetFiles(dir);
                        foreach (var f in files) {
                            string rel = f.Substring(cleanPrevPath.Length);
                            oldFiles[Norm(rel)] = new Meta(new FileInfo(f).Length, File.GetLastWriteTime(f));
                        }
                        var subDirs = Directory.GetDirectories(dir);
                        foreach (var d in subDirs) ScanOldDir(d);
                    }
                    ScanOldDir(cleanPrevPath);
                }
            } catch {
                try { System.Console.WriteLine("[Warn] Failed to read previous backup baseline."); } catch {}
            }
        }

        // --- 4. 第一遍扫描：收集所有变化的文件并计算最大路径长度 ---
        var changedFiles = new ArrayList();
        int maxPathLength = 0;

        foreach (DictionaryEntry kvp in currentFiles) {
            string relPath = (string)kvp.Key;
            FileInfo currInfo = (FileInfo)kvp.Value;
            
            string statusSymbol = "";
            ConsoleColor symbolColor = ConsoleColor.Gray;
            string extraInfo = "";

            if (!oldFiles.ContainsKey(relPath)) {
                statusSymbol = "+";
                symbolColor = ConsoleColor.Green;
                CountNew++;
            }
            else {
                Meta oldMeta = (Meta)oldFiles[relPath];
                oldFiles.Remove(relPath);

                bool sizeChanged = (currInfo.Length != oldMeta.Size);
                bool timeChanged = (currInfo.LastWriteTime > oldMeta.Time.AddSeconds(2)); 

                if (sizeChanged || timeChanged) {
                    long sizeDiff = currInfo.Length - oldMeta.Size;

                    if (sizeDiff > 0) { 
                        statusSymbol = "↑"; 
                        symbolColor = ConsoleColor.Yellow;
                        extraInfo = FmtSize(sizeDiff); 
                    }
                    else if (sizeDiff < 0) { 
                        statusSymbol = "↓"; 
                        symbolColor = ConsoleColor.Blue;
                        extraInfo = FmtSize(sizeDiff); 
                    }
                    else { 
                        statusSymbol = "•"; 
                        symbolColor = ConsoleColor.DarkGray;
                    } 
                    CountMod++;
                } else {
                    statusSymbol = "";
                    CountSame++;
                }
            }

            if (!string.IsNullOrEmpty(statusSymbol)) {
                changedFiles.Add(new object[] { relPath, statusSymbol, symbolColor, extraInfo, currInfo });
                if (relPath.Length > maxPathLength) {
                    maxPathLength = relPath.Length;
                }
            }

            // 复制文件
            string destPath = Path.Combine(destBaseDir, relPath);
            Directory.CreateDirectory(Path.GetDirectoryName(destPath));
            currInfo.CopyTo(destPath, true);
        }

        // --- 5. 第二遍扫描：对齐输出 ---
        foreach (object[] item in changedFiles) {
            string relPath = (string)item[0];
            string statusSymbol = (string)item[1];
            ConsoleColor symbolColor = (ConsoleColor)item[2];
            string extraInfo = (string)item[3];

            // 输出状态符号
            System.Console.ForegroundColor = symbolColor;
            System.Console.Write($"{statusSymbol} ");
            
            // 输出路径和文件名
            PrintPathWithColoredFilename(relPath);
            
            // 输出对齐的大小变化信息
            if (!string.IsNullOrEmpty(extraInfo)) {
                int padding = maxPathLength - relPath.Length + 4;
                System.Console.Write(new string(' ', padding));
                System.Console.ForegroundColor = symbolColor;
                System.Console.Write(extraInfo);
            }
            
            System.Console.WriteLine();
            System.Console.ResetColor();
        }

        // --- 6. 报告被删除的文件 ---
        foreach (DictionaryEntry kvp in oldFiles) {
            System.Console.ForegroundColor = ConsoleColor.Red;
            System.Console.Write("- ");
            
            PrintPathWithColoredFilename((string)kvp.Key);
            
            System.Console.WriteLine();
            System.Console.ResetColor();
            CountDel++;
        }
    }
}
"@

$Assemblies = @("System.IO.Compression", "System.IO.Compression.FileSystem")
if ($PSVersionTable.PSVersion.Major -ge 6) {
    $Assemblies += "System.Console"; $Assemblies += "System.Collections.NonGeneric"; $Assemblies += "System.Runtime"
}
try { 
    Add-Type -TypeDefinition $SourceCode -Language CSharp -ReferencedAssemblies $Assemblies -ErrorAction Stop
} catch { 
    Write-Host "❌ C# Compilation Error:" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Yellow
    Write-Host "`nDetailed Errors:" -ForegroundColor Red
    $_.Exception.Errors | ForEach-Object { Write-Host $_ -ForegroundColor Yellow }
    exit 
}
# ==========================================
# 2. 智能配置加载 (Smart Config)
# ==========================================

$DefaultConfigJson = @'
{
  "storage": {
    "backupsDir": "A_backups",
    "compress": true
  },
  "naming": {
    "prefix": "v",
    "dateFormat": "yyyy-MM-dd_HHmm",
    "defaultDesc": "backup"
  },
  "retention": {
    "maxBackups": 50,
    "deleteCount": 10
  },
  "include": [
    "src",
    "public",
    "package.json",
    "package-lock.json",
    "tsconfig.json",
    "tsconfig.node.json",
    "vite.config.ts",
    "index.html",
    ".env",
    ".env.local"
  ],
  "exclude": [
    "node_modules",
    "dist",
    "build",
    ".git",
    ".DS_Store",
    ".log",
    "src/assets/font"
  ]
}
'@

$Config = @{ Storage=@{}; Naming=@{}; Retention=@{}; Include=@(); Exclude=@() }
$ConfigFile = Join-Path $PSScriptRoot ".svconfig.json"

if (-not (Test-Path $ConfigFile)) {
    try {
        Set-Content -Path $ConfigFile -Value $DefaultConfigJson -Encoding UTF8
        Write-Host "ℹ Auto-created default config: .svconfig.json" -ForegroundColor DarkGray
    } catch {
        Write-Host "⚠ Failed to create default config" -ForegroundColor Red
    }
}

if (Test-Path $ConfigFile) {
    try {
        $JsonContent = Get-Content $ConfigFile -Raw | ConvertFrom-Json
        
        # 加载 Storage
        if ($JsonContent.storage) { 
            $Config.Storage = $JsonContent.storage 
        }
        
        # 加载 Naming (兼容旧版 backupsDir 写法)
        if ($JsonContent.naming) { 
            $Config.Naming = $JsonContent.naming 
            if ($JsonContent.naming.backupsDir -and -not $Config.Storage.backupsDir) { 
                $Config.Storage.backupsDir = $JsonContent.naming.backupsDir 
            }
        }

        # 加载 Retention
        if ($JsonContent.retention) { $Config.Retention = $JsonContent.retention }
        
        # 加载 Include/Exclude
        if ($JsonContent.include) { $Config.Include = $JsonContent.include }
        if ($JsonContent.exclude) { $Config.Exclude = $JsonContent.exclude }

    } catch { 
        Write-Host "⚠ Config error: Invalid JSON format" -ForegroundColor Red 
    }
}

# 设置默认值 (Fallbacks)
if (!$Config.Storage.backupsDir) { $Config.Storage.backupsDir = "A_backups" }
if ($null -eq $Config.Storage.compress) { $Config.Storage.compress = $true }

if (!$Config.Naming.prefix) { $Config.Naming.prefix = "v" }
if (!$Config.Naming.dateFormat) { $Config.Naming.dateFormat = "yyyy-MM-dd_HHmm" }
if (!$Config.Naming.defaultDesc) { $Config.Naming.defaultDesc = "backup" }

if (!$Config.Retention.maxBackups) { $Config.Retention.maxBackups = 0 }
if (!$Config.Retention.deleteCount) { $Config.Retention.deleteCount = 10 }

$NamingSchema = $Config.Naming
$IncludeList = $Config.Include
$ExcludePatterns = $Config.Exclude
$Storage = $Config.Storage

# ==========================================
# 3. 版本与基准 (Version & Baseline)
# ==========================================
$backupsRoot = Join-Path -Path $PSScriptRoot -ChildPath $Storage.backupsDir
if (-not (Test-Path $backupsRoot)) { New-Item $backupsRoot -ItemType Directory | Out-Null }

$allBackups = Get-ChildItem $backupsRoot | Where-Object { 
    ($_.Name -match "^$($NamingSchema.Prefix)(\d+)-") 
} | Sort-Object @{Expression={ if ($_.Name -match "^$($NamingSchema.Prefix)(\d+)-") { [int]$Matches[1] } }}

# 修改：不再强制过滤 .zip，而是取最后一个备份项（可能是 Zip 也可能是 Folder）
$lastBackupItem = $allBackups | Select-Object -Last 1
$prevBackupPath = if ($lastBackupItem) { $lastBackupItem.FullName } else { "" }

$nextVersion = 1
if ($allBackups.Count -gt 0) {
    $lastItem = $allBackups[$allBackups.Count - 1]
    if ($lastItem.Name -match "^$($NamingSchema.Prefix)(\d+)-") {
        $nextVersion = [int]$Matches[1] + 1
    }
}

Write-Host "Next Version: " -NoNewline -ForegroundColor Gray
Write-Host "$($NamingSchema.Prefix)$nextVersion" -ForegroundColor Yellow
if ($lastBackupItem) { Write-Host "Compare With: $($lastBackupItem.Name)" -ForegroundColor DarkGray }

if ([string]::IsNullOrWhiteSpace($m)) {
    $inputDesc = Read-Host "Description [$($NamingSchema.defaultDesc)] "
    if ([string]::IsNullOrWhiteSpace($inputDesc)) {
        $userDesc = $NamingSchema.defaultDesc
    } else {
        $userDesc = $inputDesc
    }
} else {
    $userDesc = $m
}

$folderName = "$($NamingSchema.Prefix)${nextVersion}-${userDesc}_$(Get-Date -Format $NamingSchema.DateFormat)"
$destinationDir = Join-Path -Path $backupsRoot -ChildPath $folderName

# ==========================================
# 4. 执行 C# 内核 (Execution)
# ==========================================
Write-Host "`nAnalysis & Backup..." -ForegroundColor DarkGray
Write-Host "--------------------" -ForegroundColor DarkGray

New-Item -Path $destinationDir -ItemType Directory -Force | Out-Null
$rootWithSlash = $PSScriptRoot.TrimEnd('\') + "\"

[SmartBackup]::Run(
    $rootWithSlash,
    [string[]]$IncludeList,
    [string[]]$ExcludePatterns,
    $destinationDir,
    $prevBackupPath
)

# ==========================================
# 5. 压缩与存储 (Archiving)
# ==========================================
Write-Host "--------------------" -ForegroundColor DarkGray

$finalPath = $destinationDir
$isZip = $false

if ($Storage.compress) {
    Write-Host "Archiving to .zip..." -ForegroundColor DarkGray
    $zipFileName = "$folderName.zip"
    $zipPath = Join-Path $backupsRoot $zipFileName

    if ($PSVersionTable.PSVersion.Major -ge 6) { Add-Type -AssemblyName System.IO.Compression.FileSystem }
    [System.IO.Compression.ZipFile]::CreateFromDirectory($destinationDir, $zipPath)
    Remove-Item -Path $destinationDir -Recurse -Force
    
    $finalPath = $zipPath
    $isZip = $true
} else {
    Write-Host "Keeping as folder (No Zip)..." -ForegroundColor DarkGray
}

# ==========================================
# 6. 保留策略执行 (Safe Retention)
# ==========================================
$cleanedCount = 0
$retentionTip = ""
$limit = $Config.Retention.maxBackups
$batch = $Config.Retention.deleteCount

$currentBackups = Get-ChildItem $backupsRoot | Where-Object { 
    ($_.Name -match "^$($NamingSchema.Prefix)(\d+)-") 
} | Sort-Object CreationTime

$totalCount = $currentBackups.Count

if ($limit -gt 0) {
    if ($totalCount -gt $limit) {
        $overflow = $totalCount - $limit
        $toDeleteNum = [Math]::Max($overflow, $batch)
        if ($toDeleteNum -ge $totalCount) { $toDeleteNum = $totalCount - 1 }
        
        if ($toDeleteNum -gt 0) {
            $filesToDelete = $currentBackups | Select-Object -First $toDeleteNum
            foreach ($f in $filesToDelete) {
                try { 
                    Remove-Item $f.FullName -Recurse -Force; 
                    $cleanedCount++ 
                } catch {}
            }
        }
    }
} else {
    if ($totalCount -gt 50) {
        $retentionTip = "⚠ Tip: You have $totalCount backups. Set 'maxBackups' in .svconfig.json to auto-clean."
    }
}

# ==========================================
# 7. 报告 (Report)
# ==========================================
$cntNew = [SmartBackup]::CountNew
$cntMod = [SmartBackup]::CountMod
$cntDel = [SmartBackup]::CountDel

$finalItem = Get-Item $finalPath
$sizeBytes = 0
if ($isZip) {
    $sizeBytes = $finalItem.Length
} else {
    try {
        $sizeBytes = (Get-ChildItem $finalPath -Recurse | Measure-Object -Property Length -Sum).Sum
    } catch { $sizeBytes = 0 }
}

$sizeDisplay = if ($sizeBytes -gt 1MB) { "{0:N2} MB" -f ($sizeBytes / 1MB) } else { "{0:N2} KB" -f ($sizeBytes / 1KB) }

function Report([string]$Label, [string]$Value, [ConsoleColor]$Color = "Gray") {
    Write-Host ("{0}: " -f $Label).PadRight(10) -NoNewline -ForegroundColor DarkGray
    Write-Host $Value -ForegroundColor $Color
}

Write-Host "`n✔ Backup Success" -ForegroundColor Green
Write-Host ""
Report "Version"  "$($NamingSchema.Prefix)$nextVersion" Yellow
Report "Result"   $finalItem.Name Cyan
Report "Size"     $sizeDisplay Cyan
Report "Stats"    "+$cntNew  ~$cntMod  -$cntDel" White

if ($cleanedCount -gt 0) {
    Report "Cleaned"  "$cleanedCount old backups" Yellow
}
if ($retentionTip) {
    Write-Host "`n$retentionTip" -ForegroundColor Yellow
}

Write-Host ""