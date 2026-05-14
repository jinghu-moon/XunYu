# XunYu CLI 架构重构方案

> 版本: 2.0
> 日期: 2026-05-12
> 状态: 已完成（argh→clap 迁移、命名统一、shell completion 更新、性能基准、文档同步）
> 设计原则: 类型驱动、零成本抽象、编译期保证一致性

---

## 一、现状分析

### 1.1 项目规模

| 指标 | 数值 |
|------|------|
| 顶层命令 | 22 |
| Cmd struct | 370 |
| Feature gate | 12 |
| 集成测试 | 499 |
| CLI 库 | argh 0.1.x（**已确认为结构性瓶颈**） |

### 1.2 核心问题（根因链）

```
argh 不支持 flatten/global/env
    → 参数无法复用 → 370 个 struct 大量重复
    → 输出格式无法统一 → 三种写法并存
    → 无 shell completion 生成 → 自己维护

全局 OnceLock 状态
    → 测试间状态泄漏
    → 无法为不同命令设置不同行为
    → 配置加载时机不确定

缺乏类型级约束
    → 输出格式靠人工保证 → 不一致
    → 错误处理靠字符串匹配 → hack
    → 新增命令无编译期检查 → 容易遗漏
```

### 1.3 详细问题清单

参见 [CLI-Current-State.md](./CLI-Current-State.md) 第二至九节。

---

## 二、目标架构

### 2.1 架构愿景

从 **命令中心化** 转向 **Operation Runtime**:

```
当前:
  命令 → 直接操作 → 分散输出

目标（执行模型）:
  Adapter(CLI/Dashboard/AI)
    → parse intent
    → CommandSpec::run  (只读查询)
       或 Operation::preview → confirm → execute → audit  (危险操作)
    → StructuredValue / Table
    → Renderer(Terminal / WebSocket / JSON)
```

核心理念：
- **Capability > Command** — 业务逻辑在 Service 层，命令只是薄适配器
- **StructuredValue 是数据总线** — CLI、Dashboard、未来 AI 共享同一数据模型
- **Operation 是危险操作的统一协议** — preview/confirm/execute/rollback/audit
- **Renderer 是输出的唯一出口** — 命令不知道自己被谁消费
- **编译器保证一致性** — 关联类型、trait bound，而非人工约定

### 2.2 目录结构

```
src/
├── core/                    # 统一基础设施（P0）
│   ├── mod.rs
│   ├── value.rs            # StructuredValue — CLI/Dashboard/AI 共享数据模型
│   ├── renderer.rs         # Renderer trait（Terminal/Json/Dashboard）
│   ├── output.rs           # Renderable trait + OutputHandler
│   ├── table_row.rs        # TableRow trait（声明表格列）
│   ├── operation.rs        # Operation trait（preview/execute/rollback）
│   ├── args.rs             # 公共参数组（OutputArgs/ListArgs/FuzzyArgs/ScopeArgs）
│   ├── context.rs          # CmdContext（替代全局 OnceLock）
│   ├── command.rs          # CommandSpec trait + Pipeline
│   ├── error.rs            # XunError 分层错误
│   └── shell.rs            # ShellIntegration trait
│
├── commands/                # 命令实现层（薄适配器）
│   ├── mod.rs              # dispatch
│   ├── bookmark/
│   ├── proxy/
│   ├── env/
│   ├── port/              # 原 ports，重组
│   ├── proc/              # 原 ps/pkill，重组
│   └── ...
│
├── cli/                     # CLI 定义层（纯 clap derive struct）
│   ├── mod.rs              # 根命令 Xun
│   ├── global.rs           # 全局参数（global = true）
│   └── ...                 # 各子命令定义
│
├── services/                # 能力层（业务逻辑，不依赖 CLI）
│   ├── bookmark.rs         # BookmarkService
│   ├── proxy.rs            # ProxyService
│   ├── env.rs              # EnvService
│   └── ...                 # 可被 CLI / Dashboard / 测试 独立调用
│
└── ...                      # 现有业务模块（bookmark/、env_core/、acl/ 等渐进迁移）
```

### 2.3 核心组件

#### 2.3.1 统一输出层（Renderable trait）

```rust
// src/core/output.rs

use std::borrow::Cow;
use std::io::Write;

/// 输出格式
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum OutputFormat {
    Auto,    // TTY → Table, Pipe → Json
    Table,
    Json,
    Tsv,
    Csv,
}

/// 表格行声明 — 编译期定义列结构
pub trait TableRow {
    fn columns() -> &'static [&'static str];
    fn cells(&self) -> Vec<Cow<'_, str>>;
}

/// 核心 trait：任何命令输出必须实现
pub trait Renderable {
    fn render(&self, format: OutputFormat, w: &mut dyn Write) -> anyhow::Result<()>;
}

/// 为 Vec<T: Serialize + TableRow> 提供 blanket impl
impl<T: serde::Serialize + TableRow> Renderable for Vec<T> {
    fn render(&self, format: OutputFormat, w: &mut dyn Write) -> anyhow::Result<()> {
        match format {
            OutputFormat::Json => serde_json::to_writer_pretty(w, self)?,
            OutputFormat::Tsv => render_tsv(self, w)?,
            OutputFormat::Csv => render_csv(self, w)?,
            OutputFormat::Table | OutputFormat::Auto => render_table(self, w)?,
        }
        Ok(())
    }
}

/// 为单个记录提供 impl
impl<T: serde::Serialize + TableRow> Renderable for T {
    fn render(&self, format: OutputFormat, w: &mut dyn Write) -> anyhow::Result<()> {
        match format {
            OutputFormat::Json => serde_json::to_writer_pretty(w, self)?,
            _ => render_table(&[self], w)?,
        }
        Ok(())
    }
}

/// 空输出（命令无数据返回时使用）
pub struct Empty;
impl Renderable for Empty {
    fn render(&self, _: OutputFormat, _: &mut dyn Write) -> anyhow::Result<()> { Ok(()) }
}

/// 输出处理器
pub struct OutputHandler {
    pub format: OutputFormat,
    pub quiet: bool,
    pub writer: Box<dyn Write>,
}

impl OutputHandler {
    pub fn emit<R: Renderable>(&mut self, value: &R) -> anyhow::Result<()> {
        if self.quiet { return Ok(()); }
        value.render(self.format.clone(), &mut self.writer)
    }

    pub fn info(&self, msg: &str) {
        if !self.quiet { eprintln!("{msg}"); }
    }

    pub fn warning(&self, msg: &str) {
        eprintln!("Warning: {msg}");
    }
}
```

#### 2.3.2 统一参数层（global + flatten）

```rust
// src/core/args.rs
use clap::Args;
use std::path::PathBuf;

/// 输出格式参数 — 所有命令通过 global = true 自动继承
/// 不需要 flatten，直接在根命令定义
/// 各命令通过 CmdContext.output 访问

/// 列表参数（列表类命令 flatten）
#[derive(Args, Clone, Debug)]
pub struct ListArgs {
    #[arg(short, long)]
    pub tag: Option<String>,
    #[arg(short = 'n', long, default_value = "50")]
    pub limit: usize,
    #[arg(long, default_value = "0")]
    pub offset: usize,
    #[arg(short, long, value_enum, default_value = "name")]
    pub sort: SortField,
    #[arg(short, long)]
    pub reverse: bool,
}

/// 模糊查询参数（bookmark z/zi/o/oi 共享）
#[derive(Args, Clone, Debug)]
pub struct FuzzyArgs {
    /// 模糊匹配模式
    pub patterns: Vec<String>,
    #[arg(short, long)]
    pub list: bool,
    #[arg(long)]
    pub score: bool,
    #[arg(long)]
    pub why: bool,
}

/// 作用域参数（bookmark 系列共享）
#[derive(Args, Clone, Debug)]
pub struct ScopeArgs {
    #[arg(short, long)]
    pub global: bool,
    #[arg(short, long)]
    pub child: bool,
    #[arg(long)]
    pub base: Option<PathBuf>,
    #[arg(short, long)]
    pub workspace: Option<String>,
}

/// 确认参数（危险操作共享）
#[derive(Args, Clone, Debug)]
pub struct ConfirmArgs {
    #[arg(short, long)]
    pub yes: bool,
    #[arg(long)]
    pub dry_run: bool,
}
```

#### 2.3.3 统一执行上下文（替代全局 OnceLock）

```rust
// src/core/context.rs
use std::cell::OnceCell;
use std::path::PathBuf;

pub struct CmdContext {
    pub output: OutputHandler,
    pub cwd: PathBuf,
    pub interactive: bool,
    pub verbose: bool,
    config: OnceCell<Config>,  // 延迟加载
}

impl CmdContext {
    pub fn from_global(args: &Xun) -> anyhow::Result<Self> {
        let format = if args.json { OutputFormat::Json }
                     else { args.format.clone() };
        let is_tty = std::io::stdout().is_terminal();
        let resolved_format = match format {
            OutputFormat::Auto if is_tty => OutputFormat::Table,
            OutputFormat::Auto => OutputFormat::Json,
            other => other,
        };
        Ok(Self {
            output: OutputHandler {
                format: resolved_format,
                quiet: args.quiet,
                writer: Box::new(std::io::stdout()),
            },
            cwd: std::env::current_dir()?,
            interactive: !args.non_interactive && std::io::stdin().is_terminal(),
            verbose: args.verbose,
            config: OnceCell::new(),
        })
    }

    /// 延迟加载配置 — 只在需要时读取磁盘
    pub fn config(&self) -> &Config {
        self.config.get_or_init(|| Config::load().unwrap_or_default())
    }

    /// 交互确认
    pub fn confirm(&self, msg: &str) -> bool {
        if !self.interactive { return true; }
        dialoguer::Confirm::new().with_prompt(msg).default(false).interact().unwrap_or(false)
    }
}
```

#### 2.3.4 统一命令接口（CommandSpec + Pipeline）

```rust
// src/core/command.rs

/// 核心 trait — 每个命令实现此 trait
/// 关联类型 Output 强制声明输出结构，编译期保证可渲染
pub trait CommandSpec: clap::Args + Sized {
    type Output: Renderable;

    /// 参数验证（可选，默认通过）
    fn validate(&self, _ctx: &CmdContext) -> anyhow::Result<()> { Ok(()) }

    /// 执行命令，返回结构化输出
    fn run(self, ctx: &CmdContext) -> anyhow::Result<Self::Output>;
}

/// Middleware 类型
pub type Middleware = Box<dyn Fn(&mut CmdContext) -> anyhow::Result<()>>;

/// 执行管线
pub struct Pipeline {
    pub before: Vec<Middleware>,
    pub after: Vec<Middleware>,
}

impl Pipeline {
    pub fn new() -> Self { Self { before: vec![], after: vec![] } }

    pub fn before(mut self, m: impl Fn(&mut CmdContext) -> anyhow::Result<()> + 'static) -> Self {
        self.before.push(Box::new(m)); self
    }

    pub fn after(mut self, m: impl Fn(&mut CmdContext) -> anyhow::Result<()> + 'static) -> Self {
        self.after.push(Box::new(m)); self
    }
}

/// 统一执行入口
pub fn execute<C: CommandSpec>(cmd: C, ctx: &mut CmdContext, pipeline: &Pipeline) -> CliResult {
    for m in &pipeline.before { m(ctx)?; }
    cmd.validate(ctx)?;
    let output = cmd.run(ctx)?;
    ctx.output.emit(&output)?;
    for m in &pipeline.after { m(ctx)?; }
    Ok(())
}
```

#### 2.3.5 统一错误类型

```rust
// src/core/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum XunError {
    #[error("{message}")]
    User { message: String, hints: Vec<String>, code: i32 },

    #[error(transparent)]
    Internal(#[from] anyhow::Error),

    #[error("Operation cancelled")]
    Cancelled,

    #[error("Elevation required: {0}")]
    ElevationRequired(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

impl XunError {
    pub fn user(msg: impl Into<String>) -> Self {
        Self::User { message: msg.into(), hints: vec![], code: 1 }
    }

    pub fn user_with_hints(msg: impl Into<String>, hints: Vec<String>) -> Self {
        Self::User { message: msg.into(), hints, code: 1 }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::User { code, .. } => *code,
            Self::Internal(_) => 1,
            Self::Cancelled => 130,
            Self::ElevationRequired(_) => 77,
            Self::NotFound(_) => 2,
        }
    }
}

pub type CliResult<T = ()> = Result<T, XunError>;
```

#### 2.3.6 StructuredValue — 统一数据模型

> 这是 CLI ↔ Dashboard ↔ 未来 AI 的**共享数据层**。
> 所有命令产出 StructuredValue，各 Renderer 独立消费。

```rust
// src/core/value.rs
use std::collections::BTreeMap;

/// XunYu 的统一结构化值 — 类似 Nushell 的 Value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Duration(std::time::Duration),
    Filesize(u64),
    Date(chrono::DateTime<chrono::Utc>),
    Record(Record),
    List(Vec<Value>),
    Binary(Vec<u8>),
}

/// 有序键值对
pub type Record = BTreeMap<String, Value>;

/// 带 schema 的表格（列表类命令的标准输出）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub columns: Vec<ColumnDef>,
    pub rows: Vec<Record>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub kind: ValueKind,
    pub sortable: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ValueKind {
    String, Int, Float, Bool, Date, Duration, Filesize, Path,
}
```

**为什么不只用 `serde_json::Value`？**
- `Value` 携带语义类型（Duration、Filesize、Date）→ Renderer 可以智能格式化
- `Table` 携带 schema → Dashboard 可以自动生成列头、排序、过滤
- 与 Dashboard WebSocket API 直接对接，无需二次转换

#### 2.3.7 Renderer 分离 — 多端消费

```rust
// src/core/renderer.rs

/// Renderer trait — 将 StructuredValue 渲染到不同目标
pub trait Renderer {
    fn render_value(&mut self, value: &Value) -> anyhow::Result<()>;
    fn render_table(&mut self, table: &Table) -> anyhow::Result<()>;
    fn render_info(&mut self, msg: &str);
    fn render_warning(&mut self, msg: &str);
    fn render_error(&mut self, err: &XunError);
}

/// CLI 终端渲染器
pub struct TerminalRenderer { format: OutputFormat, writer: Box<dyn Write> }

/// JSON 渲染器（管道输出 / API 响应）
pub struct JsonRenderer { writer: Box<dyn Write>, pretty: bool }

/// Dashboard WebSocket 渲染器（推送到前端）
#[cfg(feature = "dashboard")]
pub struct DashboardRenderer { tx: tokio::sync::mpsc::Sender<WsMessage> }
```

**关键设计：命令不知道自己被谁消费。**

```rust
// 命令只产出 Value/Table，不关心渲染
impl CommandSpec for BookmarkListCmd {
    type Output = Table;
    fn run(self, ctx: &CmdContext) -> Result<Table> { ... }
}

// dispatch 层决定用哪个 Renderer
match ctx.renderer {
    Renderer::Terminal(r) => r.render_table(&output)?,
    Renderer::Dashboard(r) => r.render_table(&output)?,  // 推送到 WebSocket
}
```

#### 2.3.8 Operation 层 — 危险操作的统一抽象

> 适用于：backup、ACL、batch_rename、env apply、vault、delete、restore
> 不适用于：只读查询（list、show、search）

```rust
// src/core/operation.rs

/// 操作预览 — dry-run 的统一输出
#[derive(Debug, Clone, Serialize)]
pub struct Preview {
    pub summary: String,
    pub changes: Vec<Change>,
    pub risk_level: RiskLevel,
    pub reversible: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Change {
    pub target: String,
    pub action: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum RiskLevel { Low, Medium, High, Critical }

/// Operation trait — 危险操作的统一接口
pub trait Operation: Send {
    /// 生成预览（dry-run）
    fn preview(&self, ctx: &CmdContext) -> anyhow::Result<Preview>;

    /// 执行操作
    fn execute(&self, ctx: &CmdContext) -> anyhow::Result<OperationResult>;

    /// 回滚（可选，不是所有操作都可逆）
    fn rollback(&self, _ctx: &CmdContext) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("rollback not supported"))
    }
}

/// 操作结果
#[derive(Debug, Clone, Serialize)]
pub struct OperationResult {
    pub success: bool,
    pub summary: String,
    pub changes_applied: usize,
    pub duration: std::time::Duration,
}

/// 操作执行器 — 统一 preview → confirm → execute → audit 流程
pub fn run_operation(
    op: &dyn Operation,
    ctx: &CmdContext,
    confirm: bool,
) -> anyhow::Result<OperationResult> {
    // 1. Preview
    let preview = op.preview(ctx)?;
    ctx.renderer.render_value(&serde_json::to_value(&preview)?)?;

    // 2. Confirm（高风险操作强制确认）
    if confirm || preview.risk_level as u8 >= RiskLevel::High as u8 {
        if !ctx.confirm(&format!("Proceed? ({} changes)", preview.changes.len())) {
            return Err(XunError::Cancelled.into());
        }
    }

    // 3. Execute
    let result = op.execute(ctx)?;

    // 4. Audit log
    ctx.audit_log(&result);

    Ok(result)
}
```

**Operation 与 CommandSpec 的关系：**

```rust
/// 危险操作命令 — 同时实现 CommandSpec 和构建 Operation
impl CommandSpec for BackupCreateCmd {
    type Output = OperationResult;

    fn run(self, ctx: &CmdContext) -> Result<OperationResult> {
        let op = BackupCreateOperation::from_args(self, ctx)?;
        run_operation(&op, ctx, !self.confirm.yes)
    }
}
```

#### 2.3.9 Dashboard 打通 — CLI ↔ Web 统一执行模型

> Dashboard (`dashboard-ui/`) 当前通过 HTTP API 调用后端。
> 重构后，Dashboard 和 CLI 共享同一个执行模型。

```
┌─────────────────────────────────────────────────────┐
│                   Execution Model                    │
│                                                     │
│  ┌─────────┐    ┌───────────┐    ┌──────────────┐  │
│  │ CLI     │    │ Dashboard │    │ Future: RPC  │  │
│  │ Adapter │    │ WS/HTTP   │    │ / AI Agent   │  │
│  └────┬────┘    └─────┬─────┘    └──────┬───────┘  │
│       │               │                 │           │
│       ▼               ▼                 ▼           │
│  ┌─────────────────────────────────────────────┐    │
│  │            CommandSpec / Operation           │    │
│  │         (统一业务逻辑，不知道调用者)          │    │
│  └─────────────────────┬───────────────────────┘    │
│                        │                            │
│                        ▼                            │
│  ┌─────────────────────────────────────────────┐    │
│  │         StructuredValue / Table              │    │
│  │         (统一数据模型)                        │    │
│  └─────────────────────┬───────────────────────┘    │
│                        │                            │
│       ┌────────────────┼────────────────┐           │
│       ▼                ▼                ▼           │
│  ┌─────────┐    ┌───────────┐    ┌──────────┐      │
│  │Terminal │    │ WebSocket │    │  JSON    │      │
│  │Renderer │    │ Renderer  │    │ Renderer │      │
│  └─────────┘    └───────────┘    └──────────┘      │
└─────────────────────────────────────────────────────┘
```

**Dashboard API 变更：**

```rust
// src/commands/dashboard/api.rs — 重构后

/// Dashboard 调用命令的统一入口
async fn handle_command(ws_msg: WsCommand) -> WsResponse {
    let ctx = CmdContext::for_dashboard(&ws_msg.session);
    match ws_msg.command.as_str() {
        "bookmark.list" => {
            let cmd: BookmarkListCmd = serde_json::from_value(ws_msg.args)?;
            let output = cmd.run(&ctx)?;
            WsResponse::table(output)  // 直接推送 Table 到前端
        }
        "backup.create" => {
            let cmd: BackupCreateCmd = serde_json::from_value(ws_msg.args)?;
            let op = BackupCreateOperation::from_args(cmd, &ctx)?;
            let preview = op.preview(&ctx)?;
            WsResponse::preview(preview)  // 前端展示预览，等待确认
        }
        _ => WsResponse::error("unknown command"),
    }
}
```

**前端受益：**
- `Table` 带 schema → 前端自动生成列、排序、过滤（当前 `BookmarksPanel.vue` 手写列定义）
- `Preview` 统一结构 → `TaskConfirmDialog.vue` 可以通用化
- `OperationResult` → `RecentTasksPanel.vue` 自动记录
- `RiskLevel` → `UnifiedConfirmDialog.vue` 自动调整确认强度

---

## 三、分阶段实施计划（激进版）

> 项目处于快速开发期，允许破坏性改动。采用一次性迁移策略，避免两套系统长期共存。

### Phase 0: 依赖准备 (1 天)

**目标:** clap 与 argh 共存，验证编译。

**任务:**

- [ ] 添加 clap 4 依赖（精确 feature 配置）
- [ ] 创建 `src/core/` 目录，放入空 mod.rs
- [ ] 验证 `cargo check` 通过（两库共存）

**Cargo.toml 变更:**

```toml
[dependencies]
clap = { version = "4", features = ["derive", "env", "unicode", "wrap_help"], default-features = false }
# argh 暂时保留，Phase 4 删除
argh = "0.1"
```

### Phase 1: 基础设施 (3 天)

**目标:** 建立 core/ 全部组件，100% 单元测试覆盖。

**任务:**

- [ ] 实现 `core/error.rs` — XunError 分层错误
- [ ] 实现 `core/value.rs` — StructuredValue + Table + ColumnDef
- [ ] 实现 `core/renderer.rs` — Renderer trait + TerminalRenderer + JsonRenderer
- [ ] 实现 `core/output.rs` — Renderable trait（基于 Value 的 blanket impl）
- [ ] 实现 `core/table_row.rs` — TableRow trait（向 Table 转换的桥接）
- [ ] 实现 `core/operation.rs` — Operation trait + Preview + run_operation()
- [ ] 实现 `core/args.rs` — ListArgs / FuzzyArgs / ScopeArgs / ConfirmArgs
- [ ] 实现 `core/context.rs` — CmdContext（延迟配置加载 + Renderer 持有）
- [ ] 实现 `core/command.rs` — CommandSpec trait + Pipeline + execute()
- [ ] 实现 `core/shell.rs` — ShellIntegration trait
- [ ] 为每个组件编写单元测试

**验收标准:**

- `cargo test -p xun --lib core` 全部通过
- StructuredValue 可序列化为 JSON（Dashboard 兼容）
- Operation::preview 返回的 Preview 可直接推送到 Dashboard WebSocket
- 不影响现有功能（core/ 是独立模块，无人引用）

### Phase 2: 验证设计 (2 天)

**目标:** 用 proxy 模块端到端验证新架构。

**选择 proxy 的理由:** 最简单（5 子命令），有快捷命令（pon/poff），覆盖输出格式问题。

**任务:**

- [ ] 用 clap derive 重写 proxy CLI 定义
- [ ] 为 proxy 输出实现 TableRow + Renderable
- [ ] 实现 ProxySetCmd / ProxyDelCmd 等的 CommandSpec
- [ ] 在 dispatch 中用 feature flag 切换新旧实现
- [ ] 跑通 proxy 相关测试
- [ ] 验证 `--format json`、`--format table`、管道输出

**验收标准:**

- proxy 命令使用新架构，所有测试通过
- 输出格式统一，管道输出为 JSON
- 代码量减少 40%+

### Phase 3: 全量迁移 (1-2 周)

**目标:** 一次性迁移所有命令。

**迁移顺序（按复杂度递增）:**

| 批次 | 模块 | 子命令数 | 预计耗时 |
|------|------|----------|----------|
| 1 | config, ctx, tree, find | 10 | 1 天 |
| 2 | ports→port, ps/pkill→proc | 4 | 0.5 天 |
| 3 | backup, video, verify | 12 | 1 天 |
| 4 | bookmark (最大) | 26 | 2 天 |
| 5 | env (最复杂) | 27 | 2 天 |
| 6 | acl | 15 | 1 天 |
| 7 | alias, brn, vault, desktop | 大量 | 2 天 |
| 8 | 其他 feature-gated | — | 1 天 |

**每个模块迁移流程:**

```
1. 定义 CLI struct（clap derive + flatten 参数组）
2. 为输出类型实现 TableRow + Serialize
3. 实现 CommandSpec trait（run 返回 Output）
4. 更新 dispatch
5. 适配测试
6. 删除旧 argh struct
```

**验收标准:**

- 每批迁移后 `cargo test` 通过
- 参数定义零重复（通过 flatten 复用）
- 所有命令输出格式统一

### Phase 4: 清理与优化 (2 天)

**目标:** 删除 argh，统一命名，性能基准。

**任务:**

- [ ] 删除 argh 依赖
- [ ] 删除所有旧 Cmd struct 和 monolith backup 文件
- [ ] 统一命名规范（rm/list/add/show/set）
- [ ] 快捷命令降级为 `#[command(hide = true)]`
- [ ] 生成 shell completion（clap_complete）
- [ ] 运行 hyperfine 基准测试对比
- [ ] 更新文档

**验收标准:**

- argh 完全移除
- `cargo clippy` 无警告
- 编译时间增加 < 15%
- 二进制大小增加 < 500KB
- 所有 499+ 测试通过

### Phase 5: 扩展性（按需，后续迭代）

**目标:** 进一步提升架构扩展性。

**按需引入:**

- workspace 拆分（当编译时间成为瓶颈时）
- 命令自注册（inventory crate，当 feature-gated 命令超过 15 个时）
- Shell 集成扩展（fish/nushell 支持）
- Operation Layer（统一 undo/redo，当 bookmark 以外的模块也需要时）

---

## 四、具体实现示例

### 4.1 迁移前 (argh) — 当前代码

```rust
// src/cli/proxy.rs — 当前
use argh::FromArgs;

#[derive(FromArgs)]
#[argh(subcommand, name = "proxy")]
pub struct ProxyCmd {
    #[argh(subcommand)]
    pub cmd: ProxySubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
pub struct ProxySetCmd {
    #[argh(positional)]
    pub url: String,
    #[argh(option, default = "String::from(\"localhost,127.0.0.1\")", short = 'n')]
    pub noproxy: String,
    #[argh(option, short = 'm')]
    pub msys2: Option<String>,
    #[argh(option, short = 'o')]
    pub only: Option<String>,
}
```

### 4.2 迁移后 (clap + CommandSpec + Renderable)

```rust
// src/cli/proxy.rs — 新版
use clap::{Args, Subcommand};

/// 代理管理
#[derive(Args)]
pub struct ProxyCmd {
    #[command(subcommand)]
    pub command: ProxySubCommand,
}

#[derive(Subcommand)]
pub enum ProxySubCommand {
    /// 设置代理
    Set(ProxySetCmd),
    /// 删除代理
    Rm(ProxyRmCmd),
    /// 显示当前代理
    Show(ProxyShowCmd),
    /// 检测系统代理
    Detect(ProxyDetectCmd),
    /// 测试代理延迟
    Test(ProxyTestCmd),
}

#[derive(Args)]
pub struct ProxySetCmd {
    /// Proxy URL (e.g. http://127.0.0.1:7890)
    pub url: String,
    /// No-proxy list
    #[arg(short, long, default_value = "localhost,127.0.0.1")]
    pub noproxy: String,
    /// Only set for specific tools
    #[arg(short, long)]
    pub only: Option<String>,
}
```

```rust
// src/commands/proxy.rs — 新版实现
use crate::core::{CommandSpec, CmdContext, Renderable, TableRow, Empty};

/// 输出类型 — 编译期保证可渲染
#[derive(Serialize)]
pub struct ProxyInfo {
    pub url: String,
    pub noproxy: String,
    pub source: String,
}

impl TableRow for ProxyInfo {
    fn columns() -> &'static [&'static str] { &["URL", "No-Proxy", "Source"] }
    fn cells(&self) -> Vec<Cow<'_, str>> {
        vec![self.url.as_str().into(), self.noproxy.as_str().into(), self.source.as_str().into()]
    }
}

impl CommandSpec for ProxySetCmd {
    type Output = Empty;  // set 命令无数据输出

    fn run(self, ctx: &CmdContext) -> anyhow::Result<Empty> {
        let config = ctx.config();
        // ... 设置代理逻辑
        ctx.output.info(&format!("Proxy set to {}", self.url));
        Ok(Empty)
    }
}

impl CommandSpec for ProxyShowCmd {
    type Output = ProxyInfo;  // show 命令输出 ProxyInfo

    fn run(self, ctx: &CmdContext) -> anyhow::Result<ProxyInfo> {
        let config = ctx.config();
        Ok(ProxyInfo {
            url: config.proxy_url().to_string(),
            noproxy: config.noproxy().to_string(),
            source: "config".to_string(),
        })
    }
}
```

### 4.3 参数复用示例 — bookmark 系列

```rust
// src/cli/bookmark.rs
use crate::core::args::{FuzzyArgs, ScopeArgs, ListArgs};

/// bookmark z / zi / o / oi 共享的查询参数
#[derive(Args, Clone)]
pub struct BookmarkQueryArgs {
    #[command(flatten)]
    pub fuzzy: FuzzyArgs,
    #[command(flatten)]
    pub scope: ScopeArgs,
}

/// xun bookmark z <patterns...>
#[derive(Args)]
pub struct BookmarkZCmd {
    #[command(flatten)]
    pub query: BookmarkQueryArgs,
}

/// xun bookmark o <patterns...> — 与 z 共享参数，仅行为不同
#[derive(Args)]
pub struct BookmarkOpenCmd {
    #[command(flatten)]
    pub query: BookmarkQueryArgs,
    /// Open in file manager
    #[arg(long)]
    pub file_manager: bool,
}

/// xun bookmark list — 使用 ListArgs
#[derive(Args)]
pub struct BookmarkListCmd {
    #[command(flatten)]
    pub list: ListArgs,
}
```

### 4.4 全局参数 — 根命令定义

```rust
// src/cli/mod.rs
use clap::Parser;
use crate::core::output::OutputFormat;

#[derive(Parser)]
#[command(name = "xun", about = "Windows-first CLI for paths, proxies, envs, and file workflows")]
pub struct Xun {
    /// Output format (auto detects TTY)
    #[arg(long, global = true, value_enum, default_value = "auto", env = "XUN_FORMAT")]
    pub format: OutputFormat,

    /// JSON output shorthand
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress UI output
    #[arg(short, long, global = true, env = "XUN_QUIET")]
    pub quiet: bool,

    /// Verbose output
    #[arg(short, long, global = true, env = "XUN_VERBOSE")]
    pub verbose: bool,

    /// Disable ANSI colors
    #[arg(long, global = true, env = "NO_COLOR")]
    pub no_color: bool,

    /// Force non-interactive mode
    #[arg(long, global = true, env = "XUN_NON_INTERACTIVE")]
    pub non_interactive: bool,

    #[command(subcommand)]
    pub cmd: SubCommand,
}
```

### 4.5 快捷命令降级示例

```rust
#[derive(Subcommand)]
pub enum SubCommand {
    /// 代理管理
    Proxy(ProxyCmd),
    /// 书签管理
    Bookmark(BookmarkCmd),

    // --- 快捷命令：hidden，不出现在 help ---
    #[command(hide = true, name = "pon")]
    ProxyOn(ProxyOnCmd),
    #[command(hide = true, name = "poff")]
    ProxyOff(ProxyOffCmd),
    #[command(hide = true, name = "bak")]
    BackupAlias(BackupCmd),
}
```

---

## 五、命名规范统一

### 5.1 命令命名

| 操作 | 规范 | 允许的 hidden alias | 理由 |
|------|------|-------------------|------|
| 列出 | `list` | — | 唯一，无歧义 |
| 删除 | `rm` | `del` (Windows 习惯) | Unix 标准，更短 |
| 查看 | `show` | — | 唯一，`get` 保留给配置 |
| 设置 | `set` | — | 幂等语义，创建+更新合并 |
| 创建 | `add` | — | "往集合里加东西" |
| 重命名 | `rename` | `mv` | 允许 Unix 习惯 |
| 搜索 | `search` | `find` | 语义更精确 |

### 5.2 参数命名

| 参数 | 规范 | 来源 | 说明 |
|------|------|------|------|
| `--format` | 全局 | `global = true` + `env = "XUN_FORMAT"` | 所有命令自动继承 |
| `--json` | 全局 | `global = true` | `--format json` 的快捷方式 |
| `--tag` | ListArgs | `#[command(flatten)]` | 列表类命令共享 |
| `--limit` | ListArgs | `#[command(flatten)]` | 列表类命令共享 |
| `--sort` | ListArgs | `#[command(flatten)]` | 列表类命令共享 |
| `--quiet` | 全局 | `global = true` + `env = "XUN_QUIET"` | 所有命令自动继承 |
| `--verbose` | 全局 | `global = true` + `env = "XUN_VERBOSE"` | 所有命令自动继承 |
| `--yes` | ConfirmArgs | `#[command(flatten)]` | 危险操作共享 |
| `--dry-run` | ConfirmArgs | `#[command(flatten)]` | 危险操作共享 |

### 5.3 快捷命令策略

**保留为顶层命令（高频使用）:**

| 命令 | 等价于 | 说明 |
|------|--------|------|
| `z` | `bookmark z` | 最高频，shell function |
| `bak` | `backup` | 常用缩写 |

**降级为 hidden alias（兼容旧脚本，不出现在 help）:**

| 命令 | 等价于 | 处理方式 |
|------|--------|---------|
| `pon` | `proxy set` | `#[command(hide = true)]` |
| `poff` | `proxy rm` | `#[command(hide = true)]` |
| `pst` | `proxy show` | `#[command(hide = true)]` |
| `px` | `proxy exec` | `#[command(hide = true)]` |
| `del` | `delete` → 未来 `rm` | normalize 阶段处理 |
| `kill` | `port kill` | `#[command(hide = true)]` |
| `ps` | `proc list` | `#[command(hide = true)]` |
| `pkill` | `proc kill` | `#[command(hide = true)]` |

### 5.4 命令重组

```
当前:                          目标:
├── ports                      ├── port
├── kill                       │   ├── list    (原 ports)
├── ps                         │   └── kill    (原 kill)
├── pkill                      ├── proc
                               │   ├── list    (原 ps)
                               │   └── kill    (原 pkill)
```

---

## 六、风险与缓解

### 6.1 风险清单

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| clap 编译时间增加 | +5-8s | 确定 | feature 裁剪，不开 `color`/`suggestions` |
| 499 测试需适配 | 1-2 天 | 确定 | 大部分测试不涉及 CLI 解析层 |
| shell init 脚本兼容性 | 用户需重新 source | 中 | hidden alias 保留旧命令 |
| 二进制大小增加 | +300-500KB | 确定 | LTO + strip（当前已很大） |
| 迁移期间功能回退 | 高 | 低 | Phase 2 验证后再全量迁移 |

### 6.2 回滚策略

- Phase 0-1: 删除 `src/core/` 目录 + 移除 clap 依赖
- Phase 2: 恢复 proxy 旧代码（feature flag 切换）
- Phase 3: git revert 到 Phase 2 完成点
- Phase 4: 不可回滚（argh 已删除），但此时已全量验证

### 6.3 性能预算

| 指标 | 当前基线 | 允许上限 | 测量方式 |
|------|---------|---------|---------|
| `xun z foo` 冷启动 | ~15ms | <20ms | hyperfine |
| `xun --help` | ~5ms | <10ms | hyperfine |
| 增量编译 | ~3s | <5s | cargo build --timings |
| 全量编译 | ~45s | <55s | cargo build --release --timings |
| 二进制大小 | ~8MB | <9MB | ls -la |

---

## 七、验收标准

### 7.1 代码质量

- [ ] 所有测试通过 (499+)
- [ ] `cargo clippy` 无警告
- [ ] 参数定义零重复（grep 验证无 `pub tag: Option<String>` 重复出现）
- [ ] 所有命令输出类型实现 `Renderable`（编译期保证）
- [ ] 所有命令实现 `CommandSpec`（编译期保证）

### 7.2 用户体验

- [ ] `xun --help` 输出清晰分组
- [ ] 所有命令支持 `--format json/table/tsv`（通过 global 参数）
- [ ] 管道输出自动切换为 JSON
- [ ] Shell completion 正常（clap_complete 生成）
- [ ] 旧快捷命令仍可用（hidden alias）

### 7.3 性能

- [ ] `xun z foo` 冷启动 < 20ms
- [ ] 增量编译 < 5s
- [ ] 全量编译增加 < 15%
- [ ] 二进制大小增加 < 500KB

### 7.4 架构

- [ ] `src/core/` 模块 100% 单元测试覆盖
- [ ] 新增命令只需：定义 struct + impl CommandSpec + impl TableRow
- [ ] 无全局可变状态（OnceLock 移除）
- [ ] 错误类型分层，exit code 自动化

---

## 八、附录

### 8.1 依赖变更

**新增:**

```toml
[dependencies]
clap = { version = "4", features = ["derive", "env", "unicode", "wrap_help"], default-features = false }
clap_complete = "4"
thiserror = "2"  # 或沿用现有 thiserror = "1"
```

**移除:**

```toml
[dependencies]
argh = "0.1"
```

**注意:** 不开 clap 的 `color` feature（已有 `console` crate），不开 `suggestions`（已有 `src/suggest.rs`）。

### 8.2 参考资料

- [clap 官方文档](https://docs.rs/clap/latest/clap/)
- [clap derive API](https://docs.rs/clap/latest/clap/_derive/index.html)
- [clap global arguments](https://docs.rs/clap/latest/clap/_derive/_cookbook/typed_derive/index.html)
- [Rain's Rust CLI recommendations](https://rust-cli-recommendations.sunshowers.io/cli-parser.html)
- [kbknapp: CLI Structure in Rust (Cmd trait pattern)](https://dev.to/kbknapp/cli-structure-in-rust-03-2182)
- [argparse-benchmarks-rs](https://github.com/rust-cli/argparse-benchmarks-rs)

### 8.3 相关文档

- [CLI 现状说明](./CLI-Current-State.md)
- [模块文档规范](../module/00-writing-guide.md)
- [重构计划 (旧)](../refactor-split-plan.md)

### 8.4 设计决策记录 (ADR)

| 决策 | 选择 | 否决方案 | 理由 |
|------|------|---------|------|
| CLI 库 | clap 4 derive | argh, bpaf, pico-args | 生态最大、flatten/global/env/completion 全支持 |
| 执行模型 | Operation Runtime | Command-centric | 统一 preview/confirm/execute/audit，Dashboard 复用 |
| 数据模型 | StructuredValue (自定义) | serde_json::Value | 语义类型（Duration/Filesize）、Table schema → Dashboard 自动化 |
| 命令抽象 | CommandSpec trait (关联类型) | XunCommand (OOP 钩子) | 编译期保证输出类型、零成本 |
| 输出消费 | Renderer trait (多端) | 单一 OutputHandler | CLI/Dashboard/AI 共享，命令不知道消费者 |
| 危险操作 | Operation trait | 每命令自己 dry-run | 统一 audit/undo/preview，Dashboard 预览复用 |
| 全局参数 | `global = true` | 每子命令 flatten | 更少代码、clap 原生支持 |
| 错误类型 | XunError (thiserror) | CliError (手写) | 类型安全、exit code 自动化 |
| 业务逻辑 | services/ 层 | commands/ 内 | Dashboard 和 CLI 共享，命令只是薄适配器 |
| 实施节奏 | 一次性迁移 (2-3 周) | 逐模块 (6-8 周) | 快速开发期、避免双系统共存 |
| 删除命名 | `rm` | `delete` | Unix 标准、更短 |

---

## 九、审批记录

| 角色 | 姓名 | 日期 | 签名 |
|------|------|------|------|
| 提交人 | — | 2026-05-12 | — |
| 审核人 | Kiro (AI) | 2026-05-12 | 通过，已融入架构建议 |
| 批准人 | — | — | — |
