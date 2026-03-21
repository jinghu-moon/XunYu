# XunYu（xun）性能优化与自定义功能规划

> 综合主流 CLI 设计指南（clig.dev、fuchsia.dev、Shopify CLI Kit）与同类工具调研（zoxide、fd、restic、maram）。
> 补充（2026-02）：Dashboard Web UI 迭代不影响本规划的 CLI 性能/自定义项。

## 目录

- [一、性能优化](#一性能优化)
  - [P-1 tree syscall 精简](#p-1-tree-syscall-精简)
  - [P-2 z 命令 frecency 权重调优](#p-2-z-命令-frecency-权重调优)
  - [P-3 bak 跳过未变化文件](#p-3-bak-跳过未变化文件)
  - [P-4 统一输出格式自动降级](#p-4-统一输出格式自动降级)
  - [P-5 NO_COLOR 与 --no-color](#p-5-no_color-与---no-color)
  - [P-6 --quiet / --verbose](#p-6---quiet----verbose)
  - [P-7 显式非交互开关](#p-7-显式非交互开关)
  - [P-8 错误码语义一致性](#p-8-错误码语义一致性)
- [二、自定义功能](#二自定义功能)
  - [C-1 tree 排除规则可配置](#c-1-tree-排除规则可配置)
  - [C-2 bak CLI 覆盖与 dry-run](#c-2-bak-cli-覆盖与-dry-run)
  - [C-3 bak 尊重 .gitignore](#c-3-bak-尊重-gitignore)
  - [C-4 list 排序与分页](#c-4-list-排序与分页)
  - [C-5 ports 过滤增强](#c-5-ports-过滤增强)
  - [C-6 proxy 配置持久化](#c-6-proxy-配置持久化)
  - [C-7 全局配置与环境变量](#c-7-全局配置与环境变量)
  - [C-8 命名与 flag 风格规范化](#c-8-命名与-flag-风格规范化)
- [三、优先级总览](#三优先级总览)
- [四、现状已符合的最佳实践](#四现状已符合的最佳实践)

---

## 一、性能优化

### P-1 tree syscall 精简

**现状**：`build_tree` 对每个 entry 调用 `path.is_dir()`（触发 `GetFileAttributesW`），再进入 `should_exclude`。

**参考**：fd 使用 `DirEntry::file_type()` 零额外 syscall；maram 采用提前剪枝。

**方案**：

1. `e.file_type().unwrap().is_dir()` 替代 `e.path().is_dir()`——Windows 上 `FindNextFileW` 已返回类型信息。
2. 构造 `PathBuf` 前先用文件名快速排除，命中则 `continue`，省去堆分配。

```rust
// 优化后
let ft = e.file_type().unwrap();
let name = e.file_name();
let name_str = name.to_string_lossy();
if !hidden && name_str.starts_with('.') { continue; }
if EXCLUDE_NAMES.iter().any(|n| n.eq_ignore_ascii_case(&name_str)) { continue; }
let is_dir = ft.is_dir();  // 零 syscall
let path = e.path();        // 仅对未排除项分配
```

**收益**：大目录（>1000 entries）减少 30-50% syscall。

### P-2 z 命令 frecency 权重调优

**现状**：`z` 已实现 frecency 加权排序。`fuzzy.rs` 中 `frecency()` 函数按时间衰减（4.0 / 2.0 / 0.5 / 0.3 / 0.1）× 访问次数计算分数，`navigation.rs` 中以 `fuzzy_score * (1 + frecency * 0.05)` 合并为最终排序依据。

**参考**：zoxide 的 frecency 算法采用更细粒度的时间衰减区间，且 frecency 权重占比更高。

**优化方向**：

1. **权重系数调优**：当前 `0.05` 系数使 frecency 对排序影响较小，高频目录优势不明显。可考虑提升至 `0.1`–`0.2`，或改为 `fuzzy_score + frecency * weight` 加法模型，使频繁访问的目录更容易胜出。
2. **衰减区间细化**：当前 5 档衰减（1h/1d/7d/30d/∞），可参考 zoxide 增加中间档位（如 2h、12h），使近期访问的区分度更高。

```rust
// 当前实现（已有）
let combined = fs as f64 * (1.0 + frecency(e) * 0.05);
// 可选优化：提升 frecency 权重
let combined = fs as f64 * (1.0 + frecency(e) * 0.15);
```

**收益**：进一步对齐 zoxide 体验，高频目录跳转更精准。成本极低——仅调整常量。

### P-3 bak 跳过未变化文件

**现状**：`diff_copy_and_print` 对所有 include 文件执行 `fs::copy`，即使文件未变化。

**参考**：restic 仅存储变更 chunk；rustic 支持 content-defined chunking 去重。

**方案**：仅在目录模式（`compress: false`）下生效，zip 模式不适用。

1. **保守方案（推荐）**：未变化文件跳过 `fs::copy`，在报告中标注 `=`（unchanged）。备份目录仅包含变更文件，体积更小但不再是完整快照。
2. **完整快照方案**：未变化文件用 `fs::hard_link` 替代 `fs::copy`，零 I/O 开销且保持完整目录结构。

```rust
if let Some(old_meta) = old.remove(rel) {
    if meta.len() == old_meta.size && !time_changed {
        // 方案 1：跳过
        continue;
        // 方案 2：硬链接（仅目录模式）
        // let _ = fs::hard_link(prev_dir.join(rel), &dst);
    }
}
```

**注意**：硬链接方案下删除旧版本会影响新版本中的链接文件，需在文档中说明此语义差异。

**收益**：大项目（>500 文件、<5% 变更率）目录模式备份时间减少 80%+。

### P-4 统一输出格式自动降级

**现状**：`list` 已支持 `--format tsv|json|table`，非 TTY 自动降级 TSV。但 `stats`、`recent`、`ports`、`proxy status`、`dedup`、`gc` 等命令的格式支持不统一。

**参考**：clig.dev 要求"stdout 面向机器、stderr 面向人类"，所有表格命令应保证脚本模式输出稳定。beaker-project CLI 指南要求 `--json` 在所有命令间保持一致的 schema。

**方案**：

1. 为所有表格输出命令补齐 `--format tsv|json|table`（或至少 `--json`）。
2. 默认行为：TTY → table（stderr），非 TTY → TSV（stdout）。`--json` 始终输出到 stdout。
3. `XUN_UI=1` 保持现有语义（强制 table）。

**涉及命令**：`stats`、`recent`、`ports`、`proxy status`、`proxy detect`、`dedup`、`gc`。

### P-5 NO_COLOR 与 --no-color

**现状**：颜色输出依赖 `console` crate 自动检测 TTY，未显式尊重 `NO_COLOR` 环境变量。

**参考**：no-color.org 标准——当 `NO_COLOR` 存在时禁用所有 ANSI 颜色。sophieau.com CLI 指南将此列为基本要求。

**方案**：

1. 启动时检测 `NO_COLOR` 环境变量，若存在则调用 `console::set_colors_enabled(false)`。
2. 添加全局 `--no-color` flag 作为显式开关。
3. 减少非 TTY 场景下的格式化字符串构造开销。

### P-6 --quiet / --verbose

**现状**：所有命令输出固定详细度，无法调节。

**参考**：clig.dev 建议 `--quiet`（仅错误）和 `--verbose`（调试信息）作为标准 flag。sophieau.com 指出性能敏感场景下减少输出可降低 I/O 成本。

**方案**：

- `--quiet`（`-q`）：抑制 stderr 上的 UI 输出，仅保留 stdout 机器输出和错误信息。适用于脚本场景。
- `--verbose`（`-v`）：输出额外调试信息（如 bak 的逐文件对比详情、proxy test 的连接握手耗时）。
- 通过全局 flag 或 `XUN_QUIET=1` / `XUN_VERBOSE=1` 环境变量控制。

### P-7 显式非交互开关

**现状**：`dedup`、`gc`、`kill`、`bak`（无 `-m`）、`import --mode overwrite` 可能弹出交互提示。当前通过 TTY 检测自动判断，但无显式开关。

**参考**：fuchsia.dev CLI 指南要求工具既能自动识别交互环境，又能通过 flag 显式选择模式，避免脚本调用意外阻塞。

**方案**：

- 已有 `--yes`/`-y` 的命令（`import`、`gc --purge`）保持不变。
- 为 `dedup`、`kill`、`bak` 补齐 `--yes` flag，跳过确认直接执行。
- 可选：全局 `--non-interactive` flag 或 `XUN_NON_INTERACTIVE=1` 环境变量，一次性覆盖所有命令。

### P-8 错误码语义一致性

**现状**：大部分失败命令使用 `process::exit(1)` 或 `exit(2)`，但未形成统一规范。

**参考**：clig.dev 要求非零退出码表示失败，并建议区分不同错误类型。

**方案**：统一退出码语义：

| 退出码 | 含义 | 示例 |
|---|---|---|
| 0 | 成功 | 所有正常完成的命令 |
| 1 | 一般错误 | 路径不存在、书签未找到 |
| 2 | 用户输入错误 | 无效端口、无效 format、无效 --only |
| 3 | 操作被取消 | 交互确认拒绝、import 冲突未 --yes |

当前 exit(1) 和 exit(2) 的使用基本符合此规范，需审计少数不一致的调用点。

---

## 二、自定义功能

### C-1 tree 排除规则可配置

**现状**：排除规则硬编码在 `EXCLUDE_NAMES`、`EXCLUDE_PATHS`、`EXCLUDE_EXTS` 中，用户无法自定义。

**参考**：

| 工具 | 排除机制 |
|---|---|
| fd | `.gitignore` + `.fdignore` + `~/.config/fd/ignore` |
| ripgrep | `.gitignore` + `.rgignore` + 全局 ignore |
| maram | 内置 + `.gitignore` + CLI `--ignore` |

**方案**：三层排除 + CLI 覆盖：

1. **内置默认**：当前 `EXCLUDE_NAMES` 等保持不变。
2. **项目级 `.xunignore`**：放在目标目录下，每行一个 glob pattern（简化版 gitignore 语法）。
3. **CLI 参数**：`--exclude <glob>` 追加排除、`--include <glob>` 强制包含（覆盖排除）。
4. **附加选项**：`--sort name|mtime|size`、`--max-items N`、`--plain`（关闭 `├──` 装饰，利于 diff）。

```
# .xunignore 示例
__pycache__/
*.pyc
.mypy_cache/
coverage/
```

### C-2 bak CLI 覆盖与 dry-run

**现状**：备份行为完全由 `.svconfig.json` 控制，无法通过 CLI 临时覆盖。

**参考**：restic 支持 `--dry-run`（`-n`）预览变更；borgmatic 支持 CLI 覆盖配置项。

**方案**：

| 新增 flag | 说明 |
|---|---|
| `--dry-run` | 仅输出 diff 报告，不执行复制/压缩/清理 |
| `--include <path>` | 临时追加 include 路径 |
| `--exclude <path>` | 临时追加 exclude 路径 |
| `--no-compress` | 本次跳过 zip 压缩 |
| `--retain <N>` | 临时覆盖 maxBackups |

`--dry-run` 实现成本极低——在 `cmd_backup` 中加 flag 跳过写操作即可。对大项目用户价值最高。

### C-3 bak 尊重 .gitignore

**现状**：`bak` 的 include/exclude 完全依赖 `.svconfig.json`，用户需手动维护排除列表。

**参考**：restic 社区最高票 feature request 之一（#1514，59👍）；rustic（restic 的 Rust 重写）已内置 `.gitignore` 支持。

**方案**：`.svconfig.json` 新增 `"useGitignore": true`。启用时 `scan_files` 读取 `.gitignore` 并将 pattern 追加到 exclude 列表。

```json
{
  "useGitignore": true,
  "exclude": ["src/assets/font"]
}
```

开发者项目中 `.gitignore` 已覆盖 `node_modules`、`dist`、`target` 等，无需在 `.svconfig.json` 中重复维护。

### C-4 list 排序与分页

**现状**：`list` 输出按内部存储顺序，无排序和分页选项。

**参考**：beaker-project CLI 指南要求表格命令支持排序和分页；zoxide 按 frecency 排序。

**方案**：

- `--sort name|last|visits`：按名称、最近访问时间、访问次数排序。默认 `name`。
- `--limit N`：限制输出条数，配合 `recent` 使用或独立使用。
- `--reverse`：反转排序方向。

### C-5 ports 过滤增强

**现状**：`ports` 仅支持 `--all` 和 `--udp`，`kill` 仅按端口号过滤。

**参考**：sophieau.com CLI 指南建议过滤选项应覆盖常见使用场景，减少用户二次处理。

**方案**：

- `ports --range 3000-3999`：仅显示指定范围内的端口。
- `ports --pid <pid>`：按进程 ID 过滤。
- `ports --name <substr>`：按进程名模糊过滤。
- `kill --tcp` / `kill --udp`：仅终止指定协议的占用进程，避免不必要枚举。

### C-6 proxy 配置持久化

**现状**：`pon`/`poff` 通过 shell 环境变量控制，重启终端后丢失。每次 `pon` 无参数时需检测系统代理。

**参考**：主流 CLI 工具（git、npm、cargo）均将代理配置持久化到配置文件。

**方案**：`%USERPROFILE%/.xun.proxy.json` 持久化上次代理配置：

```json
{ "url": "http://127.0.0.1:7890", "noproxy": "localhost,127.0.0.1" }
```

- `proxy set` 写入时同步更新此文件。
- `pon` 无参数时优先读取此文件，而非每次检测系统代理。
- `poff` 不删除此文件（保留配置供下次 `pon` 使用）。

### C-7 全局配置与环境变量

**现状**：xun 无全局配置文件，所有行为由 CLI 参数或命令级配置（`.svconfig.json`）控制。

**参考**：fd 使用 `~/.config/fd/ignore`；ripgrep 使用 `~/.config/.ripgreprc`；zoxide 支持 `_ZO_*` 环境变量族。Shopify CLI Kit 建议环境变量用于覆盖默认路径。

**方案**：`%USERPROFILE%/.xun.config.json` 全局配置 + 环境变量覆盖：

```json
{
  "tree": {
    "defaultDepth": 3,
    "excludeNames": ["__pycache__", ".mypy_cache"]
  },
  "proxy": {
    "defaultUrl": "http://127.0.0.1:7890",
    "noproxy": "localhost,127.0.0.1,*.local"
  }
}
```

环境变量：

| 变量 | 说明 |
|---|---|
| `XUN_DB` | 覆盖书签数据文件路径（默认 `~/.xun.json`） |
| `XUN_CONFIG` | 覆盖全局配置文件路径 |
| `XUN_UI` | 已有，强制表格输出 |
| `XUN_QUIET` | 抑制 UI 输出 |
| `XUN_NON_INTERACTIVE` | 禁用交互提示 |

优先级：CLI 参数 > 环境变量 > 项目配置 > 全局配置 > 内置默认值。

### C-8 命名与 flag 风格规范化

**现状**：已基本合规——所有多词 flag 均使用 kebab-case（`--no-clip`、`--format`、`--no-color`），短选项仅用于高频参数。后续仅需统一新增 flag 的命名风格。

**参考**：Shopify CLI Kit 要求长选项统一 kebab-case，短选项仅用于高频参数；布尔参数建议 `--opt` / `--no-opt` 成对。

**方案**：

1. **长选项 kebab-case 审计**：确认所有多词 flag 使用 `-` 分隔（如 `--no-clip`、`--no-color`、`--dry-run`）。
2. **布尔成对**：新增 flag 遵循 `--compress` / `--no-compress` 模式。
3. **短选项保留给高频参数**：`-m`（message）、`-t`（tag）、`-d`（depth）、`-o`（output）、`-f`（format）、`-n`（limit）、`-q`（quiet）、`-v`（verbose）。
4. **子命令命名一致性**：动词优先（`set`、`del`、`get`、`add`、`remove`、`rename`），避免混用名词和动词。

---

## 三、优先级总览

| 优先级 | 编号 | 建议 | 成本 | 收益 | 来源 |
|---|---|---|---|---|---|
| P0 | P-2 | z frecency 权重调优 | 极低 | 高频目录跳转更精准 | zoxide |
| P0 | C-2 | bak --dry-run | 极低 | 安全性 + 实用性 | restic / borgmatic |
| P0 | P-5 | NO_COLOR 支持 | 极低 | 标准合规 | no-color.org / sophieau.com |
| P1 | P-1 | tree syscall 精简 | 低 | 大目录性能 | fd / maram |
| P1 | C-1 | tree 排除可配置 | 中 | 可定制性 | fd / ripgrep |
| P1 | C-6 | proxy 持久化 | 低 | 减少重复操作 | git / npm 模式 |
| P1 | P-7 | 显式非交互开关 | 低 | 脚本安全 | fuchsia.dev |
| P1 | P-4 | 统一 --format/--json | 中 | 脚本一致性 | clig.dev / beaker |
| P2 | C-3 | bak .gitignore | 中 | 开发者友好 | restic #1514 / rustic |
| P2 | C-7 | 全局配置 + 环境变量 | 中 | 统一自定义入口 | fd / ripgrep / zoxide |
| P2 | C-4 | list 排序分页 | 低 | 大量书签可用性 | beaker-project |
| P2 | C-5 | ports 过滤增强 | 低 | 精准过滤 | sophieau.com |
| P2 | P-6 | --quiet / --verbose | 低 | 灵活输出控制 | clig.dev / sophieau.com |
| P2 | P-3 | bak 跳过未变化文件 | 中 | 大项目（>500 文件）备份加速 80%+ | restic dedup 思路 |
| P3 | C-8 | flag 风格规范化 | 低 | 一致性 | Shopify CLI Kit |
| P3 | P-8 | 错误码统一 | 低 | 脚本判断 | clig.dev |

---

## 四、现状已符合的最佳实践

以下设计已对齐主流 CLI 规范，无需改动：

| 实践 | xun 现状 | 对应规范 |
|---|---|---|
| stdout/stderr 分流 | `out_println!` → stdout，`ui_println!` → stderr | clig.dev："stdout 面向机器，stderr 面向人类" |
| XUN_UI 强制模式 | `XUN_UI=1` 强制表格输出 | 管道场景下的可控性 |
| xtree 避免冲突 | shell wrapper 用 `xtree`，子命令仍为 `tree` | 不覆盖系统内置命令 |
| 非 TTY 自动降级 | `list` 非 TTY 时自动输出 TSV | clig.dev："默认面向自动化" |
| 交互环境检测 | `can_interact()` 判断 TTY | fuchsia.dev："自动识别交互环境" |
| 非零退出码 | 失败命令 `exit(1)` / `exit(2)` | clig.dev："非零表示失败" |
| --help 全覆盖 | argh 自动生成 | 所有指南的基本要求 |

---

## 参考来源

- [Command Line Interface Guidelines](https://clig.dev/) — 通用 CLI 设计原则
- [Fuchsia CLI Guidelines](https://fuchsia.dev/fuchsia-src/development/api/cli) — 非交互模式规范
- [Shopify CLI Kit Guidelines](https://shopify.github.io/cli/cli-kit/command-guidelines.html) — flag 命名规范
- [sophieau.com CLI Guidelines](https://sophieau.com/article/cli-guidelines/) — NO_COLOR / quiet / verbose
- [beaker-project CLI Guidelines](https://beaker-project.org/dev/guide/cli-guidelines.html) — 输出格式一致性
- [zoxide](https://github.com/ajeetdsouza/zoxide) — frecency 算法、shell 集成
- [fd](https://github.com/sharkdp/fd) — .gitignore / .fdignore 排除机制
- [restic #1514](https://github.com/restic/restic/issues/1514) — .gitignore 作为备份排除源
- [maram](https://github.com/mufeedvh/maram) — 并行目录遍历、提前剪枝
