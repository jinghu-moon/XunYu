# XunYu CLI（xun）交互与视觉设计规范

> 依据：主流 CLI 设计实践（cli-guidelines、gh、cargo、gum、lazygit、pnpm）
> 核心理念：**"好看"是信息层级清晰，"好用"是行为可预测、可恢复、可组合**
> 补充（2026-02）：Dashboard Web UI 迭代（Toast/Skeleton/命令面板/密度/主题/导出）不改变本 CLI 规范，UI 细节见 `Dashboard-Design.md`。

---

## 1. 设计原则

1. **安全默认**：危险动作需显式参数触发，默认选项永远是安全的。
2. **非交互优先**：默认走非交互模式，仅 TTY 下启用交互提示；管道场景必须稳定可脚本化。
3. **通道分离**：结果数据 → stdout，提示/进度/错误 → stderr（gum 做法）。
4. **可预演**：所有危险命令支持 `--dry-run`，展示具体变化而非模糊描述。
5. **可组合**：`--format json` 输出字段稳定，退出码有文档，适配脚本与管道。
6. **可审计**：策略变更与强制动作写审计日志。
7. **渐进披露**：默认输出短，详情放 `--verbose`，避免信息噪音。
8. **颜色增强不承载语义**：状态必须可用纯文字读懂，颜色只做增强。
9. **可降级**：无 Nerd Font / 无颜色 / 非 TTY 时仍完全可用。

---

## 2. 命令语法规范

### 2.1 命名结构

统一采用 `xun <resource> <action>` 或 `xun <action> <target>` 结构，不混用：

```
xun lock who <path>              # resource + action
xun protect set <path>           # resource + action
xun acl view <path>              # resource + action
xun rm <path>                    # action + target（文件操作简写）
xun encrypt <path> --efs         # action + target + mode
```

### 2.2 全局标志

所有命令统一支持：

| 标志 | 作用 |
|------|------|
| `--help`, `-h` | 帮助信息 |
| `--format auto\|table\|tsv\|json` | 输出格式 |
| `--quiet`, `-q` | 抑制 UI 输出（错误仍显示） |
| `--verbose`, `-v` | 详细输出 |
| `--no-color` | 禁用 ANSI 颜色 |
| `--non-interactive` | 强制非交互模式 |
| `--icons` | 启用 Nerd Font 图标（opt-in） |

环境变量覆盖：`NO_COLOR`、`XUN_QUIET`、`XUN_VERBOSE`、`XUN_NON_INTERACTIVE`、`XUN_ICONS`。

### 2.3 参数约定

- `--yes`：跳过确认（非交互必需）。
- `--dry-run`：仅展示计划动作。
- `--force`：绕过保护规则（需配合 `--reason`）。
- `--unlock`：失败后自动进入解锁流程。
- `--force-kill`：允许终止占用进程（高风险）。
- `--on-reboot`：登记重启后执行。

---

## 3. 输出通道与格式

### 3.1 通道分离

```
stdout → 结构化数据（供管道/脚本消费）
stderr → UI 提示、进度条、错误信息、表格（供人阅读）
```

已有宏：
- `out_println!()` → stdout
- `ui_println!()` → stderr（受 `--quiet` 控制）

### 3.2 输出格式

| 格式 | 场景 | 说明 |
|------|------|------|
| `auto` | 默认 | TTY → table，非 TTY → tsv |
| `table` | 人阅读 | `comfy_table` 圆角边框，输出到 stderr |
| `tsv` | 管道/脚本 | Tab 分隔，stdout |
| `json` | 程序消费 | 字段稳定，stdout |

### 3.3 信息层级（三层）

```
标题行：操作摘要（始终显示，即使 --quiet）
状态行：关键状态变化
细节行：详细信息（仅 --verbose）
```

### 3.4 行宽与对齐

- 表格列对齐，动态适配终端宽度。
- 非表格输出：固定宽度列或动态计算对齐。
- 空行用于分组，不用于装饰。
- 相对时间（`20m ago`）优于绝对时间戳。

---

## 4. 色彩语义系统

### 4.1 语义色彩常量

```rust
// src/theme.rs
pub struct Theme {
    pub success: Style,    // 绿色 — 操作成功
    pub error: Style,      // 红色加粗 — 错误
    pub warning: Style,    // 黄色 — 警告/危险操作
    pub info: Style,       // 青色 — 信息性提示
    pub hint: Style,       // 灰色 — 次要提示、建议
    pub path: Style,       // 下划线 — 文件路径
    pub cmd: Style,        // 加粗 — 命令/参数引用
    pub dim: Style,        // 暗色 — 辅助信息
}
```

### 4.2 状态词绑定

| 状态 | 颜色 | 图标（Unicode） | 图标（Nerd Font） |
|------|------|-----------------|-------------------|
| SUCCESS | 绿色 | `✓` | `` |
| WARNING | 黄色 | `⚠` | `` |
| ERROR | 红色加粗 | `✗` | `` |
| INFO | 青色 | `●` | `` |

### 4.3 规则

- 一处定义全局复用，保持视觉一致性。
- `--no-color` 时 `console` 自动降级为无样式。
- 颜色只做增强，去掉颜色后信息仍完整可读。
- 参考标杆：`cargo`（绿=成功、红=错误、黄=警告、青=信息）。

---

## 5. 图标系统

### 5.1 三层降级

| 层级 | 触发条件 | 来源 | 兼容性 |
|------|----------|------|--------|
| Nerd Font | `--icons` 或 `XUN_ICONS=1` | PUA 码位 | 需用户安装 Nerd Font |
| Unicode | 默认（现代终端） | 标准 Unicode | Windows Terminal / PowerShell 5.1+ |
| ASCII | `--no-color` 或旧终端 | 纯 ASCII | 所有终端 |

### 5.2 符号表

```rust
// src/icons.rs — 全部 const，零运行时开销
pub struct IconSet {
    pub success: &'static str,
    pub failure: &'static str,
    pub warning: &'static str,
    pub lock: &'static str,
    pub unlock: &'static str,
    pub shield: &'static str,
    pub key: &'static str,
    pub folder: &'static str,
    pub file: &'static str,
    pub arrow: &'static str,
    pub skip: &'static str,
}
```

| 语义 | Nerd Font | Unicode | ASCII |
|------|-----------|---------|-------|
| 成功 | `` `\u{f00c}` | `✓` | `[ok]` |
| 失败 | `` `\u{f00d}` | `✗` | `[FAIL]` |
| 警告 | `` `\u{f071}` | `⚠` | `[WARN]` |
| 锁定 | `` `\u{f023}` | `⊘` | `[LOCKED]` |
| 解锁 | `` `\u{f09c}` | `◎` | `[UNLOCKED]` |
| 保护 | `` `\u{f132}` | `◆` | `[PROTECTED]` |
| 加密 | `` `\u{f084}` | `◇` | `[KEY]` |
| 箭头 | `` `\u{f061}` | `→` | `->` |

### 5.3 选择逻辑

```rust
pub fn icons() -> &'static IconSet {
    if runtime::use_nerd_icons() { &NERD }
    else if console::colors_enabled() { &UNICODE }
    else { &ASCII }
}
```

### 5.4 不用 Emoji 的原因

- 终端中宽度不确定（1 列或 2 列），表格对齐崩溃。
- `comfy_table` 计算列宽时 emoji 宽度不准。
- Windows 旧版 conhost 渲染异常。

---

## 6. 进度反馈

### 6.1 三级反馈模型

| 级别 | 场景 | 组件 | 示例 |
|------|------|------|------|
| Spinner | 未知耗时的单项操作 | `indicatif::ProgressBar::new_spinner()` | `⠋ 正在解锁文件...` |
| 计数器 | 已知项数的批量操作 | `{pos}/{len} {msg}` | `3/12 正在删除...` |
| 进度条 | 已知总量的大操作 | `[████░░░░] 45%` | 大文件移动、批量加密 |

### 6.2 触发规则

- 批量操作目标项 **≥ 10** 时启用进度条。
- 单项操作预计 **> 500ms** 时启用 Spinner。
- `--quiet` 抑制所有进度输出。
- 非 TTY 环境自动静默（与 `can_interact()` 兼容）。

### 6.3 样式

```
{spinner:.green} [{bar:30}] {pos}/{len} {msg}
```

进度输出到 stderr，不干扰 stdout 数据流。

---

## 7. 错误信息规范

### 7.1 三段式结构

每条错误信息包含三部分：

```
✗ <发生了什么>
  <为什么发生>
  <下一步建议（含可复制命令）>
```

### 7.2 示例

```
✗ 无法删除 C:\data\a.txt
  文件被 EXCEL.EXE (PID 1234) 占用
  尝试: xun rm "C:\data\a.txt" --unlock
```

```
✗ 权限不足：--on-reboot 需要管理员权限
  当前以普通用户运行
  尝试: 以管理员身份重新打开终端
```

### 7.3 规则

- 错误输出到 stderr，`--format json` 时同时输出结构化错误到 stdout。
- 建议命令用 `theme.cmd`（加粗）高亮，方便用户复制。
- 路径用 `theme.path`（下划线）标记。
- 不使用技术性堆栈信息，除非 `--verbose`。

---

## 8. 退出码

| 码 | 含义 | 场景 |
|----|------|------|
| `0` | 成功 | 操作正常完成 |
| `2` | 参数错误 | 路径不存在、无效参数、保护规则拦截 |
| `3` | 权限不足 | 非管理员执行 `--on-reboot` |
| `10` | 占用未授权 | 检测到占用但未提供 `--unlock` |
| `11` | 解锁失败 | 已尝试解锁仍失败 |
| `20` | 已登记重启 | `--on-reboot` 注册成功，当前未完成 |

规则：
- 退出码在 `--format json` 的 `code` 字段中同步输出。
- 脚本可通过退出码判断后续流程，无需解析文本。
- 常量统一定义在 `src/util.rs`。

---

## 9. 交互组件

### 9.1 当前选型

| 职责 | 库 | 说明 |
|------|-----|------|
| 表格渲染 | `comfy-table` | 圆角边框，动态列宽 |
| 进度反馈 | `indicatif` | Spinner / 进度条 |
| 交互提示 | `dialoguer` | Confirm / Select / MultiSelect / FuzzySelect |
| 样式控制 | `console` | 颜色、终端检测 |

### 9.2 演进策略

- **现阶段**：沿用 `comfy-table` + `indicatif` + `dialoguer`，功能够用，零额外依赖。
- **后续**：功能稳定后，根据实际渲染需求评估是否手写渲染层替换 `comfy-table`，以获得完全的输出控制（对齐、颜色、布局）。
- **不引入** Spectre.Console / PwshSpectreConsole（强制 PS7、300ms+ 启动延迟、分发复杂度高，投入产出比不合理）。
- **不引入** cliclack（当前 dialoguer 已满足需求，YAGNI）。

### 9.2 非交互降级

- 非 TTY 环境跳过所有交互提示。
- 缺少 `--yes` 时拒绝执行破坏性操作（退出码 2）。
- `--non-interactive` 强制禁用交互，等效非 TTY。

---

## 10. 危险操作确认

### 10.1 确认触发条件

| 操作 | 交互模式 | 非交互模式 |
|------|----------|------------|
| `rm`（单文件） | 直接执行 | 直接执行 |
| `rm`（目录递归） | 弹确认 | 需 `--yes` |
| `--force-kill` | 弹确认（列出进程） | 需 `--yes` |
| `--on-reboot` | 弹确认 | 需 `--yes` |
| `protect clear --system-acl` | 弹确认 | 需 `--yes` |

### 10.2 确认格式

```
⚠ 即将终止以下进程以解锁文件：
  PID 1234  EXCEL.EXE
  PID 5678  notepad.exe

  影响：未保存的数据将丢失
  确认终止？[y/N]
```

规则：
- 默认选项永远是安全的（`N`）。
- 确认前列出具体影响范围（文件数、进程列表）。
- `--dry-run` 展示完整计划但不弹确认。

---

## 11. `--dry-run` 预览

### 11.1 输出格式

展示具体变化，而非模糊描述：

```
[DRY-RUN] xun rm "C:\data\report" --unlock

  将删除 3 个文件：
    C:\data\report\a.txt        (12 KB)
    C:\data\report\b.docx       (1.2 MB, 被 WORD.EXE 占用 → 将解锁)
    C:\data\report\c.pdf        (340 KB)

  将删除 1 个空目录：
    C:\data\report\

  总计：3 文件, 1 目录, 1.55 MB
  实际未执行任何操作。
```

### 11.2 规则

- `--dry-run` 与所有危险命令兼容（`rm`/`mv`/`ren`/`encrypt`/`decrypt`/`protect`）。
- 输出到 stderr（人阅读），`--format json` 时输出结构化计划到 stdout。
- 标题行标注 `[DRY-RUN]`，结尾明确提示"实际未执行任何操作"。

---

## 12. Shell 补全

### 12.1 协议：隐藏子命令 `complete`

```
xun complete <cursor_pos> <words...>
```

xun 根据上下文返回候选列表（每行一个），Shell 脚本负责注册和转发。

### 12.2 补全逻辑

```
输入                          → 候选来源
xun <TAB>                     → 子命令列表（list, z, rm, protect...）
xun z <TAB>                   → 书签 key（来自 xun keys）
xun rm <TAB>                  → 文件系统路径
xun rm --<TAB>                → 可用 flag（--unlock, --force-kill, --dry-run...）
xun acl <TAB>                 → ACL 子命令（view/add/remove/purge/diff/batch/effective/copy/backup/restore/inherit/owner/orphans/repair/audit/config）
xun acl view <TAB>            → 文件系统路径
xun protect set <TAB>         → 文件系统路径
xun encrypt <TAB> --<TAB>     → --efs, --to, --out, --verify
```

### 12.3 Shell 集成

**PowerShell**（在 `xun init powershell` 输出中注册）：

```powershell
Register-ArgumentCompleter -CommandName 'xun' -Native -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    $words = $commandAst.ToString().Substring(0, $cursorPosition) -split '\s+'
    $results = & $xun complete $cursorPosition @words 2>$null
    $results | ForEach-Object {
        [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
    }
}
```

**Bash**（在 `xun init bash` 输出中注册）：

```bash
_xun_complete() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    COMPREPLY=($(compgen -W "$(xun complete $COMP_POINT ${COMP_WORDS[@]} 2>/dev/null)" -- "$cur"))
}
complete -F _xun_complete xun
```

---

## 13. 帮助系统

### 13.1 渐进披露

帮助信息分三层，避免一次性信息过载：

```
第一层：xun --help          → 子命令一行摘要列表
第二层：xun rm --help       → 常用示例 + 参数说明
第三层：xun rm --help -v    → 完整参数、退出码、环境变量
```

### 13.2 子命令帮助格式

```
xun rm — 删除文件（支持解锁被占用文件）

用法：xun rm <path> [选项]

示例：
  xun rm "C:\data\a.txt"                  # 直接删除
  xun rm "C:\data\a.txt" --unlock         # 解锁后删除
  xun rm "C:\data\busy" --dry-run         # 预览删除计划

选项：
  --unlock        失败后自动解锁占用进程
  --force-kill    允许终止占用进程（需确认）
  --on-reboot     登记重启后删除（需管理员）
  --dry-run       仅展示计划，不执行
  --yes           跳过确认（非交互必需）
  --format <fmt>  输出格式：auto|table|tsv|json
```

### 13.3 规则

- 每个子命令至少提供 2-3 个常用示例。
- 示例中的路径使用引号包裹，培养用户习惯。
- 危险参数在帮助中标注风险等级。

---

## 14. 实现优先级

### P0：基础设施（与 Phase 0 同步）

| 模块 | 文件 | 说明 |
|------|------|------|
| 色彩主题 | `src/theme.rs` | `Theme` 结构体，全局单例 |
| 图标系统 | `src/icons.rs` | 三层 `IconSet`，`icons()` 选择函数 |
| 进度封装 | `src/output.rs` | `ProgressReporter`（已有） |
| 错误输出 | 各命令文件 | 统一三段式错误格式 |

### P1：交互增强

| 模块 | 说明 |
|------|------|
| 确认模板 | 危险操作确认格式统一（基于 dialoguer） |
| dry-run 输出 | 结构化预览模板 |
| 三段式错误 | 统一错误输出格式 |

### P2：补全与帮助

| 模块 | 说明 |
|------|------|
| `complete` 子命令 | 隐藏子命令，上下文感知补全 |
| Shell 集成 | 更新 `xun init` 输出的补全注册脚本 |
| 帮助文本 | 渐进披露，示例优先 |
