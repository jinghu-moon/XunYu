# XunYu 智能补全实施检查清单（含演示）

本清单用于落地与验收 `Completion-Design.md`。每条项后给出最小可复现演示；运行前请完成 Demo Setup。

补充（2026-02）：Dashboard Web UI 迭代不影响补全清单与验收步骤。

---

## Demo Setup

Build：
```
cargo build --features redirect
```

PowerShell（默认演示环境）：
```
$env:XUN_DB = "D:\100_Projects\110_Daily\Xun\docs\fixtures\completion-demo\xun.json"
$env:XUN_CONFIG = "D:\100_Projects\110_Daily\Xun\docs\fixtures\completion-demo\xun.config.json"
$env:XUN_COMPLETE_CWD = "D:\Demo\Proj"
$env:XUN_COMPLETE_SHELL = "pwsh"
```

备用数据集：
- `xun.flat.json`：零历史/平坦排序
- `xun.edge.json`：空格/中文/emoji
- `xun.bad.json`：损坏 JSON（安全降级）

二进制：
```
target\debug\xun.exe
```

---

## A. 协议与路由（核心）

- [ ] `xun __complete <args...>` 采用 shell 预分词（不解析原始行）  
  Demo：A1
- [ ] 处理 `--flag=value` 形式（拆分为 flag + partial）  
  Demo：A2
- [ ] 处理独立 `--`（其后全部视为位置参数）  
  Demo：A3
- [ ] 输出候选行 `value<TAB>desc`，禁止 `\t`/`\n`  
  Demo：A1（desc 为空时仅输出 `value`）
- [ ] Sentinel 行包含 `directive` 与 `v=1`  
  Demo：A1
- [ ] 零候选仍输出 `ok + directive`，避免回退文件补全  
  Demo：A6
- [ ] 异常输出 `__XUN_COMPLETE__=fallback` 且退出码非 0  
  Demo：A7
- [ ] 正常路径仅输出协议内容（stdout 无额外调试文本）  
  Demo：A1

### A Demo

A1：基础补全（协议输出）
```
target\debug\xun.exe __complete z ""
proj
proj-sub
docs
tmp
__XUN_COMPLETE__=ok	directive=1	v=1
```

A2：`--flag=value`
```
target\debug\xun.exe __complete redirect --profile=de
--profile=default
__XUN_COMPLETE__=ok	directive=1	v=1
```

A3：独立 `--`
```
target\debug\xun.exe __complete z -- ""
proj
proj-sub
docs
tmp
__XUN_COMPLETE__=ok	directive=1	v=1
```

A6：零候选（仍输出 ok + directive）
```
target\debug\xun.exe __complete z zzz
__XUN_COMPLETE__=ok	directive=1	v=1
```

A7：强制 fallback（退出码非 0）
```
$env:XUN_DISABLE_DYNAMIC_COMPLETE = "1"
target\debug\xun.exe __complete z ""
__XUN_COMPLETE__=fallback
EXIT=1
```

---

## B. Directive 位与行为

- [ ] 定义并使用 bitmask 常量（NoFileComp/NoSpace/FilterDirs/FilterExt）  
  Demo：B0
- [ ] `FilterExt` 使用 `ext=json|tsv`（`|` 分隔）  
  Demo：B2
- [ ] `NoSpace` 仅用于 flag 名（支持 `--flag=value` / `--flag value`）  
  Demo：B1
- [ ] `FilterDirs` 用于位置参数为目录的场景（如 `set` 第二参数）  
  Demo：B4

### B Demo

B0：位常量定义与使用
```
rg -n "NO_FILE_COMP|NO_SPACE|FILTER_DIRS|FILTER_EXT" src\commands\completion.rs
18:const NO_FILE_COMP: u32 = 1;
19:const NO_SPACE: u32 = 2;
20:const FILTER_DIRS: u32 = 4;
21:const FILTER_EXT: u32 = 8;
...
```

B1：flag 名（NoFileComp+NoSpace → directive=3）
```
target\debug\xun.exe __complete redirect --
--apply
--confirm
--copy
--dry-run
--explain
--format
--last
--log
--plan
--profile
--review
--simulate
--stats
--status
--tx
--undo
--validate
--watch
--yes
__XUN_COMPLETE__=ok	directive=3	v=1
```

B2：`FilterExt`（ext 使用 `|` 分隔）
```
target\debug\xun.exe __complete import --input ""
__XUN_COMPLETE__=ok	directive=8	ext=json|tsv	v=1
```

B4：`FilterDirs`
```
target\debug\xun.exe __complete set demo ""
__XUN_COMPLETE__=ok	directive=4	v=1
```

---

## C. 路由覆盖面

- [ ] 子命令补全（静态列表）  
  Demo：C1
- [ ] 全局 flag 补全  
  Demo：C2
- [ ] 子命令 flag 补全  
  Demo：B1
- [ ] `redirect --profile` 值补全  
  Demo：C4
- [ ] `redirect --undo/--tx` 值补全（最近 N 条 tx）  
  Demo：C5
- [ ] `config get/set` 第一个位置参数（dot-path key 列表）  
  Demo：C6
- [ ] `z/del/touch` 位置参数（书签名）  
  Demo：A1
- [ ] `redirect` 位置参数（source dir，FilterDirs）  
  Demo：C8

### C Demo

C1：子命令补全
```
target\debug\xun.exe __complete ""
all
bak
check
completion
config
decrypt
dedup
del
delete
encrypt
export
fuzzy
gc
import
init
keys
kill
list
lock
mv
open
poff
pon
ports
protect
proxy
pst
px
recent
redirect
rename
renfile
rm
save
serve
set
stats
tag
touch
tree
workspace
z
__XUN_COMPLETE__=ok	directive=1	v=1
```

C2：全局 flag（注意：需要传入 `--` 作为参数）
```
target\debug\xun.exe __complete -- --
--help
--no-color
--non-interactive
--quiet
--verbose
--version
__XUN_COMPLETE__=ok	directive=3	v=1
```

C4：`redirect --profile`
```
target\debug\xun.exe __complete redirect --profile ""
default
docs
media
__XUN_COMPLETE__=ok	directive=1	v=1
```

C5：`redirect --undo`
```
target\debug\xun.exe __complete redirect --undo ""
redirect_20260203_0003
redirect_20260202_0002
redirect_20260201_0001
__XUN_COMPLETE__=ok	directive=1	v=1
```

C6：`config get/set` key 列表
```
target\debug\xun.exe __complete config get ""
export
export.defaultFormat
proxy
proxy.defaultUrl
redirect
redirect.profiles
redirect.profiles.default
redirect.profiles.default.on_conflict
redirect.profiles.docs
redirect.profiles.docs.on_conflict
redirect.profiles.media
redirect.profiles.media.on_conflict
tree
tree.defaultDepth
__XUN_COMPLETE__=ok	directive=1	v=1
```

C8：`redirect` 位置参数（FilterDirs）
```
target\debug\xun.exe __complete redirect ""
__XUN_COMPLETE__=ok	directive=4	v=1
```

---

## D. 排序与数据

- [ ] 前缀过滤后再排序  
  Demo：D1
- [ ] frecency 排序复用现有 `visit_count + last_visited`  
  Demo：D2
- [ ] 新用户/零历史按字典序稳定排序  
  Demo：D3
- [ ] CWD 加分（导航类命令）  
  Demo：D4（对比 D2）
- [ ] 稳定排序（tie-break）  
  Demo：D3
- [ ] 匹配规则大小写一致（与现有命令行为保持一致）  
  Demo：D6

### D Demo

D1：前缀过滤
```
target\debug\xun.exe __complete z pr
proj
proj-sub
__XUN_COMPLETE__=ok	directive=1	v=1
```

D2：frecency 排序（未设置 XUN_COMPLETE_CWD）
```
$env:XUN_COMPLETE_CWD = $null
target\debug\xun.exe __complete z ""
proj-sub
proj
docs
tmp
__XUN_COMPLETE__=ok	directive=1	v=1
```

D3：零历史/平坦排序（切换到 xun.flat.json）
```
$env:XUN_DB = "D:\100_Projects\110_Daily\Xun\docs\fixtures\completion-demo\xun.flat.json"
target\debug\xun.exe __complete z ""
alpha
beta
gamma
__XUN_COMPLETE__=ok	directive=1	v=1
```

D4：CWD 加分（对比 D2）
```
$env:XUN_DB = "D:\100_Projects\110_Daily\Xun\docs\fixtures\completion-demo\xun.json"
$env:XUN_COMPLETE_CWD = "D:\Demo\Proj"
target\debug\xun.exe __complete z pr
proj
proj-sub
__XUN_COMPLETE__=ok	directive=1	v=1
```

D6：大小写一致（大写前缀仍匹配）
```
target\debug\xun.exe __complete z PRO
proj
proj-sub
__XUN_COMPLETE__=ok	directive=1	v=1
```

---

## E. 缓存与并发

- [ ] 进程内缓存 + mtime 校验  
  Demo：E1
- [ ] 无新增持久化 cache 文件  
  Demo：E2
- [ ] 并发安全（OnceLock/RwLock）  
  Demo：E1
- [ ] 监测 `.xun.json`、`.xun.visits.jsonl`、配置文件 mtime  
  Demo：E1

### E Demo

E1：缓存与 mtime 证据
```
rg -n "CompletionCache|db_mtime|visits_mtime|config_mtime|OnceLock" src\commands\completion.rs
88:struct CompletionCache {
90:    db_mtime: Option<SystemTime>,
91:    visits_mtime: Option<SystemTime>,
94:    config_mtime: Option<SystemTime>,
102:static CACHE: OnceLock<Mutex<CompletionCache>> = OnceLock::new();
338:    let db_mtime = file_mtime(&path);
339:    let visits_mtime = file_mtime(&visits);
343:    let mtime_changed = cache.db_mtime != db_mtime || cache.visits_mtime != visits_mtime;
364:    let mtime_changed = cache.config_mtime != mtime;
```

E2：无新增持久化 cache 文件（仅 fixtures）
```
Get-ChildItem docs\fixtures\completion-demo | Select-Object -ExpandProperty Name
audit.jsonl
xun.config.json
xun.edge.json
xun.flat.json
xun.json
xun.visits.jsonl
```

---

## F. 候选数量与性能

- [ ] `XUN_COMPLETE_LIMIT=0` 默认不限制  
  Demo：A1
- [ ] 硬上限 200 条  
  Demo：F1
- [ ] 10k 数据下单次补全 < 50ms  
  Demo：F2（基线脚本）
- [ ] 1 万条 + 连续 100 次总耗时 < 500ms  
  Demo：F2（基线脚本）

### F Demo

F1：硬上限与 limit 逻辑
```
rg -n "HARD_LIMIT|XUN_COMPLETE_LIMIT" src\commands\completion.rs
14:const HARD_LIMIT: usize = 200;
320:    let limit = env::var("XUN_COMPLETE_LIMIT")
326:        HARD_LIMIT
328:        limit.min(HARD_LIMIT)
```

F1b：限制输出（示例：limit=2）
```
$env:XUN_COMPLETE_LIMIT = "2"
target\debug\xun.exe __complete z ""
proj-sub
proj
__XUN_COMPLETE__=ok	directive=1	v=1
```

F2：基线性能（样例数据；10k 需替换 XUN_DB）
```
tools\bench-complete.ps1 -Runs 100 -Warmup 5 -Env "XUN_DB=D:\100_Projects\110_Daily\Xun\docs\fixtures\completion-demo\xun.json","XUN_CONFIG=D:\100_Projects\110_Daily\Xun\docs\fixtures\completion-demo\xun.config.json"
xun __complete bench
  bin: D:\100_Projects\110_Daily\Xun\target\debug\xun.exe
  args: z 
  runs: 100 (warmup: 5)
  avg_ms: 41.74 | p50: 41.54 | p95: 43.28 | p99: 45.69 | min: 40.1 | max: 46.74
  candidates: min=4 p50=4 max=4
  exit_nonzero: 0 | fallback_runs: 0
```

---

## G. Shell 集成

- [ ] PowerShell `Register-ArgumentCompleter` 调用 `__complete`  
  Demo：G1
- [ ] Bash/Zsh `complete` / compsys 调用 `__complete`  
  Demo：G2
- [ ] Fish `complete -c xun -a "(xun __complete ...)"` 可用  
  Demo：G3
- [ ] `xun init` 优先使用 `xun completion` 脚本  
  Demo：G4

### G Demo

G1：
```
target\debug\xun.exe completion powershell | Select-String "__complete" | ForEach-Object { $_.Line }
    $job = Start-Job -ScriptBlock { param($exe, $argv) & $exe __complete @argv 2>$null } -ArgumentList $xun, $args
    $out = & $xun __complete @args 2>$null
```

G2：
```
target\debug\xun.exe completion bash | Select-String "__complete" | ForEach-Object { $_.Line }
    out=$(timeout "${timeout_sec}s" xun __complete "${args[@]}" 2>/dev/null)
    out=$(xun __complete "${args[@]}" 2>/dev/null)
```

G3：
```
target\debug\xun.exe completion fish | Select-String "__complete" | ForEach-Object { $_.Line }
    set -l out (xun __complete $args)
```

G4：
```
target\debug\xun.exe init powershell | Select-String "completion powershell" | ForEach-Object { $_.Line }
    $comp = & $xun completion powershell 2>$null
```

---

## H. 回退与调试

- [ ] `XUN_DISABLE_DYNAMIC_COMPLETE=1` 强制静态补全  
  Demo：A7
- [ ] `XUN_COMP_DEBUG_FILE` 可记录路由与耗时  
  Demo：H2
- [ ] 超时由 shell 脚本控制（Rust 端不强制计时）  
  Demo：H3
- [ ] 协议版本不匹配（`v!=1`）时脚本回退静态补全  
  Demo：H4
- [ ] 数据文件损坏/权限不足时安全降级为 fallback  
  Demo：H5

### H Demo

H2：调试日志
```
$env:XUN_COMP_DEBUG_FILE = "D:\100_Projects\110_Daily\Xun\docs\fixtures\completion-demo\comp.log"
target\debug\xun.exe __complete z "" | Out-Null
Get-Content $env:XUN_COMP_DEBUG_FILE
elapsed_ms=0	start tokens=["z", ""] current=
elapsed_ms=0	route=positional subcmd=z subsub= index=0
elapsed_ms=1	status=ok items=4 directive=1 ext=
```

H3：脚本超时控制（示意）
```
rg -n "XUN_COMPLETE_TIMEOUT_MS" src\commands\completion.rs
1001:    $timeoutMs = $env:XUN_COMPLETE_TIMEOUT_MS
1145:    if [[ -n "$XUN_COMPLETE_TIMEOUT_MS" && "$XUN_COMPLETE_TIMEOUT_MS" =~ ^[0-9]+$ ]] && command -v timeout &>/dev/null; then
1147:        timeout_sec=$(awk -v ms="$XUN_COMPLETE_TIMEOUT_MS" 'BEGIN { printf "%.3f", ms/1000 }')
```

H4：版本检查（脚本侧）
```
target\debug\xun.exe completion powershell | Select-String "v=([0-9]+)" | ForEach-Object { $_.Line }
    if ($parsed.Sentinel -match 'v=([0-9]+)') { $ver = [int]$matches[1] }
target\debug\xun.exe completion bash | Select-String -SimpleMatch 'sentinel" =~ v=' | ForEach-Object { $_.Line }
    if [[ "$sentinel" =~ v=([0-9]+) ]]; then
target\debug\xun.exe completion fish | Select-String -SimpleMatch "v=([0-9]+)" | ForEach-Object { $_.Line }
    if test (string match -rq "v=([0-9]+)" $sentinel)
```

H5：损坏数据安全降级（当前为 “空结果 + ok”）
```
$env:XUN_DB = "D:\100_Projects\110_Daily\Xun\docs\fixtures\completion-demo\xun.bad.json"
target\debug\xun.exe __complete z ""
__XUN_COMPLETE__=ok	directive=1	v=1
```

---

## I. 边缘场景

- [ ] 路径含空格/引号/中文/emoji（PowerShell 重点）  
  Demo：I1
- [ ] 超长候选被跳过或替换  
  Demo：I2
- [ ] 大量候选时不阻塞 shell  
  Demo：F2
- [ ] 环境变量缺失（`XUN_COMPLETE_CWD`/`XUN_COMPLETE_SHELL`）时有合理退化路径  
  Demo：D2

### I Demo

I1：空格/中文/emoji
```
$env:XUN_DB = "D:\100_Projects\110_Daily\Xun\docs\fixtures\completion-demo\xun.edge.json"
target\debug\xun.exe __complete z ""
emoji🚀
my project
中文路径
__XUN_COMPLETE__=ok	directive=1	v=1
```

I2：超长/控制字符过滤（证据）
```
rg -n "MAX_VALUE_LEN|contains_control|sanitize_control" src\commands\completion.rs
15:const MAX_VALUE_LEN: usize = 500;
280:        if value.len() > MAX_VALUE_LEN || contains_control(&value) {
287:        if contains_control(&desc) {
288:            desc = sanitize_control(&desc);
435:fn contains_control(s: &str) -> bool {
439:fn sanitize_control(s: &str) -> String {
```
