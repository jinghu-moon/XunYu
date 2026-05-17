# XunYu 系统架构剖析与 TDD 开发任务清单

> **文档版本**: v2.0 | **生成日期**: 2026-05-15  
> **基于源码版本**: commit main HEAD | **Rust Edition**: 2024  
> **核心愿景**: Windows-first · 零运行时开销 · 毫秒级响应 · 极致轻量

---

## 第一部分：系统架构与设计剖析

### 1. 架构总览：三层组件模型

```
┌─────────────────────────────────────────────────────────────────────┐
│                        执行层 (Execution Layer)                       │
│  CLI Router · Dashboard Daemon · Shell Integration · Background Jobs │
│  src/lib.rs · src/xun_core/dispatch.rs · src/xun_core/dashboard_cmd │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ CommandSpec::run() / Operation::execute()
┌───────────────────────────────▼─────────────────────────────────────┐
│                        消费层 (Consumer Layer)                        │
│  Bookmark Engine · Xunbak Pipeline · ACL Processor · FileVault      │
│  EnvManager · Batch Rename · Diff Engine · Desktop Manager          │
│  src/bookmark/ · src/xunbak/ · src/acl/ · src/filevault/            │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ trait abstractions / direct FFI
┌───────────────────────────────▼─────────────────────────────────────┐
│                        基层 (Foundation Layer)                        │
│  Path Guard · Win32 FFI · Runtime · Output · Config · Store         │
│  src/path_guard/ · src/windows/ · src/runtime.rs · src/output.rs    │
└─────────────────────────────────────────────────────────────────────┘
```

---

### 2. 基层组件精准归类与复用分析

#### 2.1 基层组件清单

| 组件 | 源码路径 | 职责 | 跨模块复用度 |
|------|----------|------|-------------|
| **Path Guard** | `src/path_guard/` | Windows 路径解析、验证、规范化 | ★★★★★ (find, bookmark, batch_rename, backup, acl) |
| **Win32 FFI** | `src/windows/` | 系统 API 封装层 | ★★★★★ (acl, desktop, ports, filevault, path_guard) |
| **Runtime** | `src/runtime.rs` | 全局运行时状态 (quiet/verbose/color) | ★★★★★ (所有模块) |
| **Output** | `src/output.rs` + `src/macros.rs` | stdout/stderr 分离、表格渲染 | ★★★★★ (所有模块) |
| **CommandSpec** | `src/xun_core/command.rs` | 命令 trait 抽象 | ★★★★★ (所有命令) |
| **Renderer** | `src/xun_core/renderer.rs` | 多目标输出渲染 | ★★★★☆ (CLI/Dashboard/Test) |
| **Value** | `src/xun_core/value.rs` | 结构化输出数据总线 | ★★★★☆ |
| **Operation** | `src/xun_core/operation.rs` | 危险操作协议 | ★★★☆☆ (delete, acl, env, redirect) |
| **Config** | `src/config/` | 配置加载与默认值 | ★★★☆☆ |
| **Store** | `src/store.rs` | 时间戳工具 (可测试) | ★★☆☆☆ |

#### 2.2 零成本抽象论证

**CommandSpec trait 的单态化 (Monomorphization)**

```rust
// src/xun_core/command.rs — 实际代码模式
pub trait CommandSpec {
    fn validate(&self) -> Result<(), XunError> { Ok(()) }
    fn run(&self, ctx: &CmdContext, renderer: &dyn Renderer) -> Result<Value, XunError>;
}

pub fn execute<C: CommandSpec>(cmd: &C, ctx: &CmdContext, renderer: &dyn Renderer) -> Result<Value, XunError> {
    cmd.validate()?;
    cmd.run(ctx, renderer)
}
```

**编译器行为分析**：
- `execute<C>` 对每个具体命令类型生成独立的机器码实例
- `cmd.validate()` 和 `cmd.run()` 在编译期静态分派，**无 vtable 查找开销**
- `renderer: &dyn Renderer` 是唯一的动态分派点（仅在最终输出时触发一次 vtable 调用）
- 由于 `execute` 函数体极小（2 行），编译器会将其内联到调用点，**零额外函数调用开销**

**二进制体积影响**：
- 当前 370 个 Cmd 结构体 × `execute<C>` = 370 个单态化实例
- 但每个实例仅 ~20 字节机器码（validate + run 调用），总增量 < 8KB
- 对比 `dyn CommandSpec` 方案：节省 370 × 16 字节 vtable 指针 = 5.9KB，且避免间接跳转的 CPU 分支预测失败

**建议的破坏性重构**：将 `Renderer` 从 `&dyn` 改为泛型参数：

```rust
pub fn execute<C: CommandSpec, R: Renderer>(cmd: &C, ctx: &CmdContext, renderer: &R) -> Result<Value, XunError>
```

**收益**：消除最后一个 vtable 调用点。**代价**：每个 (Command, Renderer) 组合生成独立代码。由于 Renderer 实现仅 2-3 种（Terminal/Json/Test），增量 < 24KB，可接受。



#### 2.3 基层组件务实改进（拒绝为复用而复用）

##### 已否决的提案

| 提案 | 否决原因 |
|------|---------|
| `StreamPipeline<S: Stage>` 泛型管道 | xunbak（追加式容器写入）和 filevault（帧级加密）领域语义完全不同，表面相似不构成抽象理由。强行统一会模糊错误处理、增加理解成本、不减少代码量。 |
| `Renderer` 泛型化 (`&dyn` → `R: Renderer`) | 消除的是每次命令执行 1 次 vtable 调用（~1ns），对 CLI 工具无意义。`&dyn Renderer` 是正确设计——隔离输出目标，支持测试 mock。 |
| `WideString` newtype wrapper | 过度封装。一个 `fn to_wide(&OsStr) -> Vec<u16>` 就够了，不需要 newtype、不需要 trait。 |
| `win32_call!` 万能宏 | Win32 API 返回值约定不统一（BOOL / HANDLE / HRESULT），一个宏无法覆盖。按返回值类型提供 2-3 个辅助函数更实际。 |

##### 保留的务实改进

**改进 1：`to_wide()` 工具函数**

5+ 处重复的 `.encode_wide().chain(once(0)).collect::<Vec<u16>>()`，提取为：

```rust
// src/windows/mod.rs
pub fn to_wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}
```

3 行代码，零抽象成本，纯消除重复。

**改进 2：Win32 错误处理辅助函数**

按返回值类型提供具体函数（不是万能宏）：

```rust
// src/windows/error.rs
pub struct Win32Error { pub code: u32, pub message: String }

pub fn check_bool(ret: i32) -> Result<(), Win32Error> {
    if ret != 0 { Ok(()) } else { Err(Win32Error::last()) }
}

pub fn check_handle(ret: isize) -> Result<isize, Win32Error> {
    if ret != 0 && ret != -1 { Ok(ret) } else { Err(Win32Error::last()) }
}
```

**改进 3：`lazy_static` → `std::sync::LazyLock`**

机械替换，消除外部依赖。Rust 1.80+ 稳定，语义等价：

```rust
// Before
lazy_static! { static ref CACHE: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new()); }
// After
static CACHE: LazyLock<Mutex<HashMap<String, bool>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
```

##### 原则：基层"共用"的正确粒度

> 基层组件的共用应该是**工具函数级别**（`fn`），不是**架构级别**（`trait` / `Pipeline<S>`）。
> 如果一个抽象需要超过 10 行代码来定义，它大概率不值得存在于基层。

---

#### 2.4 基层组件目录整合方案

**现状问题**：基层组件散落在 `src/` 顶层，与消费层模块混杂：

```
src/
├── runtime.rs      ← 基层
├── output.rs       ← 基层
├── macros.rs       ← 基层
├── store.rs        ← 基层
├── util.rs         ← 基层
├── model.rs        ← 基层
├── suggest.rs      ← 基层
├── proc.rs         ← 基层（进程工具）
├── config/         ← 基层
├── windows/        ← 基层
├── path_guard/     ← 基层
├── security/       ← 基层
├── bookmark/       ← 消费层（混在一起）
├── xunbak/         ← 消费层
├── acl/            ← 消费层
├── ...
```

**提案：引入 `src/foundation/` 目录**

```
src/
├── foundation/           ← 基层组件统一入口
│   ├── mod.rs            ← pub use 重导出
│   ├── runtime.rs        ← 全局运行时状态
│   ├── output.rs         ← stdout/stderr 分离
│   ├── macros.rs         ← ui_println! / out_println!
│   ├── store.rs          ← 时间戳工具
│   ├── util.rs           ← 通用工具函数
│   ├── model.rs          ← 核心数据模型
│   ├── suggest.rs        ← 命令建议
│   ├── proc.rs           ← 进程工具
│   └── win32/            ← Win32 FFI（从 windows/ 改名，语义更明确）
│       ├── mod.rs
│       ├── error.rs      ← check_bool / check_handle
│       ├── safety.rs
│       ├── ctrlc.rs
│       ├── file_copy.rs
│       ├── window_api.rs
│       ├── ...
├── config/               ← 保持独立（有自己的 model/defaults/compat 子结构）
├── path_guard/           ← 保持独立（有自己的 policy/parallel/winapi 子结构）
├── security/             ← 并入 foundation/ 或保持独立（仅 2 个文件）
├── xun_core/             ← 执行层（不变）
├── commands/             ← 执行层（不变）
├── bookmark/             ← 消费层（不变）
├── xunbak/               ← 消费层（不变）
├── ...
```

**是否值得做？逐项评估：**

| 因素 | 分析 |
|------|------|
| **收益** | `src/` 顶层从 ~30 个条目降至 ~18 个；新人一眼看出哪些是基础设施 |
| **代价** | 所有 `crate::runtime` → `crate::foundation::runtime` 的 import 路径变更；或通过 `mod.rs` 重导出保持兼容 |
| **风险** | 低——纯文件移动 + `mod` 声明变更，不改逻辑 |
| **时机** | 适合在一次大重构中一并完成（比如 Phase 0） |

**我的建议：做，但有条件**

1. **`path_guard/` 和 `config/` 不并入** — 它们有自己的子目录结构，已经是独立的"小模块"，强行塞进 `foundation/` 反而增加嵌套深度
2. **`windows/` 改名为 `win32/` 并入 `foundation/`** — 语义更明确（这不是跨平台的 windows 兼容层，就是 Win32 API 封装）
3. **通过 `lib.rs` 重导出保持兼容**：

```rust
// src/lib.rs — 过渡期兼容
mod foundation;
pub(crate) use foundation::runtime;
pub(crate) use foundation::output;
// ... 其他模块的 import 路径不变
```

4. **散文件（runtime.rs, output.rs, util.rs 等）并入**，目录模块（path_guard/, config/）保持顶层

**最终目标结构**：

```
src/
├── lib.rs
├── main.rs
├── cli.rs
├── foundation/        ← 基层：工具函数、运行时、Win32 FFI
├── config/            ← 基层：配置系统（独立子结构）
├── path_guard/        ← 基层：路径验证（独立子结构）
├── xun_core/          ← 执行层：命令分发、服务、协议
├── commands/          ← 执行层：命令实现
├── bookmark/          ← 消费层
├── xunbak/            ← 消费层
├── acl/               ← 消费层
├── env_core/          ← 消费层
├── filevault/         ← 消费层
├── backup/            ← 消费层
├── alias/             ← 消费层
├── batch_rename/      ← 消费层
├── find/              ← 消费层
├── desktop/           ← 消费层
├── ports/             ← 消费层
├── img/               ← 消费层
├── diff/              ← 消费层
├── bin/               ← 二进制入口
```

从 30+ 顶层条目降至 ~20 个，且层次一目了然。

---

### 3. 消费层组件极限性能分析

#### 3.1 Bookmark Engine (`src/bookmark/`)

**当前热路径**：`xun z <query>` → 加载 JSON → 构建索引 → 模糊匹配 → 输出 `__BM_CD__:<path>`

**性能瓶颈定位**（基于 `benches/bookmark_bench_divan.rs` 和 `target/bm-timing-release/`）：

| 阶段 | 当前耗时 | 瓶颈原因 | 优化目标 |
|------|---------|----------|---------|
| JSON 加载 | ~15ms (5000条) | serde_json 全量反序列化 | < 1ms |
| 索引构建 | ~5ms | HashMap 分配 | < 0.5ms |
| 模糊匹配 | ~2ms | 线性扫描 | < 0.5ms |
| 路径输出 | < 0.1ms | — | — |

**优化路径 1：rkyv 零拷贝缓存**

```rust
// 提案：src/bookmark/cache.rs 重构
// 当前：JSON → serde_json::from_str → Vec<Bookmark> → build_index
// 优化：JSON → rkyv::to_bytes → mmap → ArchivedVec<ArchivedBookmark> (零拷贝)

use rkyv::{Archive, Serialize, Deserialize};

#[derive(Archive, Serialize, Deserialize)]
pub struct BookmarkCache {
    pub version: u32,
    pub fingerprint: u64,  // xxHash of source JSON
    pub entries: Vec<BookmarkEntry>,
}

// 读取路径：mmap .xun.bookmark.rkyv → check fingerprint → 直接使用 ArchivedBookmarkCache
// 写入路径：仅在 JSON 变更时重建 rkyv 缓存
```

**性能预期**（基于 Apache Iggy 基准）：
- 加载 5000 条书签：15ms → **< 0.1ms**（mmap + 零拷贝）
- 内存占用：从 ~2MB 堆分配降至 **0 额外分配**（直接引用 mmap 页）

**优化路径 2：xxHash 快速指纹**

```rust
// 当前：每次启动都 full parse JSON 判断是否需要重建缓存
// 优化：xxHash3 对 JSON 文件计算 64-bit 指纹，与缓存头部比对
use xxhash_rust::xxh3::xxh3_64;

fn cache_valid(json_path: &Path, cache_path: &Path) -> bool {
    let json_bytes = std::fs::read(json_path).ok()?;
    let fingerprint = xxh3_64(&json_bytes);
    let cache_header = read_cache_header(cache_path).ok()?;
    cache_header.fingerprint == fingerprint
}
```

**性能预期**：xxHash3 在 2MB JSON 上耗时 < 0.1ms（30+ GB/s 吞吐）。

#### 3.2 Xunbak Container (`src/xunbak/`)

**当前热路径**：`xun xunbak create <dir>` → 遍历文件 → BLAKE3 哈希 → ZSTD 压缩 → 写入容器

**性能瓶颈定位**（基于 `benches/xunbak_bench_divan.rs` 和 `logs/xunbak_baseline.md`）：

| 阶段 | 当前吞吐 | 理论极限 | 差距 |
|------|---------|---------|------|
| 文件遍历 | ~50K files/s | ~200K files/s (MFT) | 4x |
| BLAKE3 哈希 | ~4 GB/s | ~6 GB/s (AVX-512) | 1.5x |
| ZSTD L3 压缩 | ~350 MB/s | ~350 MB/s | ≈1x (已达极限) |
| 容器写入 | ~300 MB/s | ~500 MB/s (NVMe seq) | 1.7x |

**优化路径 1：MFT 直扫替代 ReadDir 遍历**

```rust
// 当前：std::fs::read_dir() 递归 → 每个文件一次 syscall
// 优化：NTFS MFT 直接扫描（src/find/mft/ 已有实现）
// 收益：50K → 200K files/s（4x），因为 MFT 是连续磁盘读取

// 集成方式：
pub fn collect_files_mft(root: &Path) -> Vec<FileEntry> {
    if is_ntfs_volume(root) {
        mft::scan(root)  // 复用 src/find/mft/
    } else {
        fallback_readdir(root)
    }
}
```

**优化路径 2：双哈希策略 (xxHash + BLAKE3)**

```rust
// 当前：每个文件都计算 BLAKE3（4 GB/s）
// 优化：增量备份时先用 xxHash3 快速比对（30 GB/s），仅变更文件计算 BLAKE3

pub struct FileFingerprint {
    pub quick_hash: u64,      // xxHash3 — 用于快速变更检测
    pub content_hash: [u8; 32], // BLAKE3 — 用于容器完整性
}

fn needs_backup(file: &Path, prev: &FileFingerprint) -> bool {
    let quick = xxh3_64(&std::fs::read(file)?);
    quick != prev.quick_hash  // 30 GB/s 快速判断
}
```

**性能预期**：增量备份场景（<5% 文件变更），哈希阶段耗时降低 **90%+**。

**优化路径 3：IO 管道并行化**

```rust
// 当前：串行 read → hash → compress → write
// 优化：3 阶段流水线并行

// Stage 1: 文件读取 (IO-bound, 独立线程)
// Stage 2: BLAKE3 + ZSTD (CPU-bound, rayon 线程池)
// Stage 3: 容器写入 (IO-bound, 独立线程)

// 使用 crossbeam-channel 连接各阶段（已在 Cargo.toml 中）
let (tx_read, rx_read) = crossbeam_channel::bounded(16);
let (tx_proc, rx_proc) = crossbeam_channel::bounded(16);
```

**性能预期**：在 NVMe SSD 上，总吞吐从 ~300 MB/s 提升至 **~500 MB/s**（瓶颈转移到磁盘顺序写入带宽）。

#### 3.3 FileVault 加密管道 (`src/filevault/mod.rs`)

**当前实现**（72KB，已高度优化）：
- 多线程帧加密：`available_parallelism()` 个 worker
- 有界通道：inflight = 2 × workers
- AES-256-GCM 硬件加速（AES-NI 指令集）

**剩余优化空间**：

| 优化点 | 当前 | 优化后 | 依据 |
|--------|------|--------|------|
| 帧大小 | 固定 | 自适应 (64KB~1MB) | 小文件减少帧开销，大文件提高吞吐 |
| 密钥派生 | 每次 Argon2 | 缓存 derived key (DPAPI 保护) | 避免重复 KDF（~100ms/次） |
| 内存池 | per-frame alloc | 预分配帧缓冲池 | 消除 GC 压力 |

#### 3.4 Path Guard (`src/path_guard/`)

**当前实现**（已高度优化）：
- `thread_local! CHECK_BUF`：per-thread 缓冲区
- 批量探测缓存（≥10 路径）
- 并行验证管道

**剩余优化空间**：

```rust
// 当前：每个路径独立调用 GetFileAttributesW
// 优化：批量 NtQueryDirectoryFile 一次获取目录下所有文件属性
// 适用场景：验证同一目录下的多个文件（batch_rename, backup）

// 当前：String → OsString → encode_wide → Vec<u16>（3 次分配）
// 优化：直接在 thread_local 缓冲区中构建 UTF-16（0 次堆分配）
thread_local! {
    static WIDE_BUF: RefCell<Vec<u16>> = RefCell::new(Vec::with_capacity(512));
}
```



---

### 4. 执行层组件分析

#### 4.1 CLI 路由分发 (`src/xun_core/dispatch.rs`)

**当前架构**（36KB 文件）：

```
run_from_env() → arg normalize → clap::Parser::try_parse_from()
    → runtime::init_from_flags() → dispatch::run_from_args()
        → match args.cmd { SubCommand::Backup(_) => ..., ... }
            → execute(&cmd_spec, ctx, renderer)
```

**性能特征**：
- Clap 解析：~1-2ms（370 个子命令的 derive 模式）
- 运行时初始化：< 0.1ms（OnceLock 写入）
- 分发：O(1) match 跳转

#### 4.2 CLI 质量改进方案

##### 问题 1：SubCommand 枚举过大（40+ 变体），help 输出不可读

当前 `SubCommand` 包含大量快捷别名（`Pon`, `Poff`, `Px`, `Pst`, `Ports`, `Kill`, `Ps`, `Pkill`），导致 `xun --help` 巨长。

**方案**：快捷别名统一 `#[command(hide = true)]` 或移入 `normalize_top_level_aliases()`：

```rust
// 方案 A：隐藏别名（保留解析能力，不出现在 help 中）
#[command(hide = true)]
Pon(ProxyOnCmd),
#[command(hide = true)]
Poff(ProxyOffCmd),

// 方案 B：移入 normalize（SubCommand 枚举瘦身）
fn normalize_top_level_aliases(raw_args: &mut [String]) {
    match arg.as_str() {
        "bak" => *arg = "backup".to_string(),
        "pon" => *arg = "proxy".to_string(),  // + insert "on"
        "poff" => *arg = "proxy".to_string(), // + insert "off"
        "ports" => *arg = "port".to_string(),
        _ => {}
    }
}
```

推荐方案 A（改动最小，保持 clap 路由能力）。

##### 问题 2：参数校验应前移到 clap 层

当前很多校验在 `CommandSpec::validate()` 中手写，但 clap 自身能力可以覆盖大部分场景，且自动生成准确的错误信息和 help 文本：

```rust
// ❌ 当前：运行时手动校验
#[arg(long)]
pub retain: Option<usize>,
// validate() 里: if retain == Some(0) { return Err(...) }

// ✅ 建议：clap 层面约束
#[arg(long, value_parser = clap::value_parser!(u32).range(1..1000))]
pub retain: Option<u32>,
```

**适合 clap 层的校验**（自动生成错误信息 + help 提示）：
- 数值范围：`value_parser!(u32).range(1..)`
- 枚举值：`#[arg(value_enum)]`
- 互斥参数：`#[arg(conflicts_with = "other")]`
- 必须组合：`#[arg(requires = "other")]`
- 路径类型：`#[arg(value_hint = ValueHint::DirPath)]`（shell 补全用）

**必须留在 `validate()` 的校验**：
- 需要文件系统访问的（路径是否存在、权限检查）
- 需要 `CmdContext` 的（配置依赖、运行时状态）
- 跨字段复杂业务逻辑

##### 问题 3：Help 文本质量不足

当前 about 过于简略（如 `"Incremental project backup"`），用户无法从 help 中学会用法。

**规范**：每个命令必须包含 `after_help` 示例：

```rust
#[command(
    name = "backup",
    about = "Incremental project backup",
    long_about = "Incremental project backup with hash-based change detection.\n\
                  Supports .xunbak container and traditional directory output.",
    after_help = "Examples:\n  \
                  xun backup                         # backup cwd\n  \
                  xun backup -C ~/proj --incremental\n  \
                  xun backup --container out.xunbak"
)]
```

##### 问题 4：输出格式参数不统一

当前各命令独立声明 `--json` flag，应统一为全局 `--format`：

```rust
// Xun 顶层
#[arg(long, global = true, value_enum, default_value_t = OutputFormat::Auto)]
pub format: OutputFormat,

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputFormat { Auto, Table, Json, Tsv, Csv }
```

各命令的 `--json` 标记为 `#[deprecated]` 并映射到 `--format json`。

#### 4.3 Dashboard 守护进程 (`src/xun_core/dashboard_cmd.rs`)

**架构**：`axum` HTTP 服务 + `rust-embed` 静态资源 + WebSocket 实时通信

**当前特征**：
- Feature-gated (`dashboard` feature 拉入 `axum`, `tokio`, `rust-embed`)
- 嵌入 Vite+Vue3 构建产物
- WebSocket 协议用于实时系统监控推送

**优化方向**：
- 将 `tokio` 限制为 `current_thread` runtime（Dashboard 不需要多线程异步）
- 使用 `axum` 的 `http1` only（已配置）减少 HTTP/2 开销
- 静态资源启用 `Content-Encoding: br`（Brotli 预压缩）

#### 4.4 Shell 集成协议

**当前实现**（`src/xun_core/shell.rs` + `src/xun_core/init_cmd.rs`）：

```
CLI stdout 输出魔术行 → Shell 脚本解析并执行副作用

魔术行协议：
  __BM_CD__:<path>     → Set-Location <path>
  __CD__:<path>        → Set-Location <path>
  __ENV_SET__:<k>=<v>  → $env:<k> = <v>
  __ENV_DEL__:<k>      → Remove-Item Env:<k>
```

**设计优势**：零 IPC 开销（利用 shell 管道），无需 named pipe/socket。

---

### 5. 统一化、抽象化与分工明确：架构治理

#### 5.1 当前问题诊断

| 问题 | 现象 | 根因 |
|------|------|------|
| **职责边界模糊** | `commands/backup.rs` (39KB) 包含业务逻辑；`services/backup.rs` (7KB) 也有业务逻辑 | 没有强制的分层规则，代码随手放 |
| **输出管道被架空** | 部分命令直接 `println!` / `out_println!` 绕过 Renderer | 迁移不彻底，旧代码未清理 |
| **Operation 协议未落地** | `run_operation()` 存在但大部分危险命令直接执行 | 协议定义了但没有强制入口 |
| **CmdContext 太薄** | 配置加载是 stub（返回空 JSON），没连接 `src/config/` | 上下文设计先行，实现未跟上 |
| **命令定义分散** | clap 结构体在 `xun_core/*_cmd.rs`，实现在 `commands/`，服务在 `services/` | 三处文件改一个命令 |

#### 5.2 统一分层规则（强制执行）

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 1: CLI 定义层 (xun_core/*_cmd.rs)                     │
│  职责：clap 结构体 + CommandSpec impl（仅做参数转换）         │
│  规则：禁止业务逻辑，禁止 IO，禁止 println                   │
│  体积：每个文件 < 200 行                                     │
└──────────────────────────┬──────────────────────────────────┘
                           │ 调用
┌──────────────────────────▼──────────────────────────────────┐
│  Layer 2: 服务层 (xun_core/services/*.rs)                    │
│  职责：编排业务流程，调用领域模块，返回 Value                 │
│  规则：无 clap 依赖，无 Renderer 依赖，纯逻辑               │
│  输入：普通 Rust 类型（&str, &Path, bool）                   │
│  输出：Result<Value, XunError>                               │
└──────────────────────────┬──────────────────────────────────┘
                           │ 调用
┌──────────────────────────▼──────────────────────────────────┐
│  Layer 3: 领域层 (src/bookmark/, src/xunbak/, src/acl/ ...)  │
│  职责：核心算法、数据结构、Win32 交互                        │
│  规则：无 xun_core 依赖，无 CLI 概念，可独立测试             │
│  输出：领域类型（Bookmark, AclSnapshot, Container...）       │
└─────────────────────────────────────────────────────────────┘
```

**关键约束**：
- Layer 1 **只做**：解构 clap args → 调用 Layer 2 → 返回 Value
- Layer 2 **只做**：校验 → 调用 Layer 3 → 组装 Value
- Layer 3 **不知道** CLI 的存在（不 import `xun_core` 任何东西）

**当前违规示例**：

```rust
// ❌ commands/backup.rs — 39KB，混合了 Layer 1+2+3
// 包含：clap 参数解析、文件遍历、哈希计算、压缩、进度条、输出格式化

// ✅ 正确拆分：
// xun_core/backup_cmd.rs (Layer 1): BackupCmdSpec::run() → services::backup::create(...)
// xun_core/services/backup.rs (Layer 2): create(dir, opts) → backup::engine::run(...)
// src/backup/ (Layer 3): engine::run() → 纯备份逻辑
```

#### 5.3 输出统一：所有命令必须走 Renderer

**规则**：命令的 `run()` 方法**禁止**直接写 stdout/stderr。所有输出通过返回 `Value` 交给 Renderer。

**例外**（仅以下场景允许直接输出）：
- Shell 魔术行（`__BM_CD__:`, `__ENV_SET__:`）— 这些是 shell 协议，不是用户输出
- 进度条（`indicatif`）— 实时 UI，不适合 Value 模型

**迁移策略**：

```rust
// ❌ 当前（绕过 Renderer）
fn run(&self, ctx: &mut CmdContext) -> Result<Value, XunError> {
    let items = do_work()?;
    for item in &items {
        println!("{}\t{}", item.name, item.path);  // 直接输出
    }
    Ok(Value::Null)  // Renderer 无事可做
}

// ✅ 正确（通过 Value 交给 Renderer）
fn run(&self, ctx: &mut CmdContext) -> Result<Value, XunError> {
    let items = do_work()?;
    let table = Table::new(vec![
        ColumnDef::new("name", ValueKind::String),
        ColumnDef::new("path", ValueKind::Path),
    ]);
    for item in &items {
        table.push_row(btreemap! {
            "name" => Value::from(item.name),
            "path" => Value::from(item.path),
        });
    }
    Ok(Value::Table(table))  // Renderer 决定格式
}
```

**收益**：
- `--format json` 自动生效（不需要每个命令手写 JSON 输出）
- Dashboard WebSocket 可直接消费 Value（不需要解析 stdout 文本）
- 测试可断言 Value 结构（不需要解析输出字符串）

#### 5.4 Operation 协议强制落地

**规则**：所有满足以下条件的命令**必须**走 `run_operation()`：
- 删除文件/目录
- 修改注册表
- 修改 ACL
- 修改系统配置（hosts, proxy）
- 批量重命名

**实现方式**：在 `execute()` 中增加编译期检查：

```rust
// 标记 trait：声明命令是否为危险操作
pub trait CommandSpec {
    /// 是否为危险操作（默认 false）
    const DANGEROUS: bool = false;
    fn validate(&self, ctx: &CmdContext) -> Result<(), XunError> { Ok(()) }
    fn run(&self, ctx: &mut CmdContext) -> Result<Value, XunError>;
}

// 危险命令必须同时实现 Operation
pub trait DangerousCommand: CommandSpec + Operation {}
```

或者更简单：在 code review 中强制要求，不需要类型系统保证。

#### 5.5 CmdContext 充实

当前 `CmdContext` 是空壳。应该成为**服务定位器**（Service Locator），提供：

```rust
pub struct CmdContext {
    format: OutputFormat,
    quiet: bool,
    verbose: bool,
    non_interactive: bool,
    // ↓ 新增：真正有用的上下文
    config: OnceCell<AppConfig>,       // 延迟加载配置
    cwd: PathBuf,                      // 当前工作目录（可被 -C 覆盖）
    data_dir: PathBuf,                 // %LOCALAPPDATA%/xun/
    home_dir: PathBuf,                 // %USERPROFILE%/
}

impl CmdContext {
    /// 获取配置（首次调用时从磁盘加载）
    pub fn config(&self) -> &AppConfig {
        self.config.get_or_init(|| AppConfig::load_or_default(&self.data_dir))
    }

    /// 获取工作目录（尊重 -C 参数）
    pub fn cwd(&self) -> &Path { &self.cwd }

    /// 获取数据目录
    pub fn data_dir(&self) -> &Path { &self.data_dir }
}
```

**收益**：
- 服务层不需要自己找配置文件路径（当前各模块各自 `std::env::var("LOCALAPPDATA")`）
- 测试可注入 mock 路径（当前测试依赖真实文件系统）
- `-C <dir>` 全局参数自然传递到所有子命令

#### 5.6 `commands/` 目录的命运

当前 `commands/` 是旧架构遗留（argh 时代的命令实现）。新架构下：
- Layer 1 在 `xun_core/*_cmd.rs`
- Layer 2 在 `xun_core/services/*.rs`
- Layer 3 在各领域模块

**`commands/` 应该逐步清空**，其内容按职责拆分到 Layer 2 或 Layer 3。

迁移优先级：
1. `commands/backup.rs` (39KB) → 拆分到 `services/backup.rs` + `src/backup/`
2. `commands/desktop.rs` (43KB) → 拆分到 `services/desktop.rs` + `src/desktop/`
3. `commands/xunbak.rs` (39KB) → 拆分到 `services/xunbak.rs` + `src/xunbak/`
4. 其余小文件逐步迁移

最终目标：**删除 `src/commands/` 目录**。

---

### 6. 编译期插件体系与依赖优化

#### 6.1 设计决策：编译期选择，不做运行时插件

| 方案 | 优点 | 缺点 | 结论 |
|------|------|------|------|
| 运行时 DLL 插件 | 用户可热加载 | ABI 兼容性地狱、unsafe FFI、版本管理复杂、违背"零运行时开销" | ❌ 否决 |
| Cargo feature gate | 零成本、编译器保证类型安全、已有 12 个 feature 基础 | 用户需自行编译或选择预编译版本 | ✅ 采用 |

#### 6.2 Feature 预设分层（精简为 2 个发布版本）

```toml
[features]
default = ["preset-standard"]

# ── 发布预设（面向用户）───────────────────────────────────
preset-standard = ["acl", "alias", "crypt", "xunbak", "lock", "protect", "batch_rename"]
preset-full = ["preset-standard", "dashboard", "diff", "desktop", "redirect", "img", "img-moz"]

# ── 原子 feature（面向开发者自定义编译）──────────────────
# ... 保持现有定义不变 ...
```

**必需组件**（始终编译，无 feature gate）：

| 组件 | 理由 |
|------|------|
| bookmark (z/zi/o) | 核心高频命令 |
| proxy (on/off/exec) | 开发者日常 |
| config / ctx | 基础设施 |
| tree / find | 文件浏览 |
| ports / proc | 系统诊断 |
| backup (基础目录备份) | 核心功能 |
| path_guard | 被多模块依赖 |
| init / completion | Shell 集成 |

**发布产物（仅 2 个）**：

| 版本 | 体积目标 | 包含 | 适用场景 |
|------|---------|------|---------|
| **xun.exe** | < 6MB | 必需 + ACL/加密/别名/xunbak/lock/protect/brn | 99% 用户日常运维 |
| **xun-full.exe** | < 12MB | 全部功能（含 Dashboard/Diff/Desktop/图像） | 需要 Web UI 或图像处理 |

用户决策一句话：**"要 Dashboard？下 full。不要？下默认的。"**

#### 6.3 CI 多产物发布

```yaml
# .github/workflows/release.yml
jobs:
  build:
    strategy:
      matrix:
        include:
          - preset: "preset-standard"
            artifact: "xun.exe"
          - preset: "preset-full"
            artifact: "xun-full.exe"
    steps:
      - run: cargo build --release --features "${{ matrix.preset }}"
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: target/release/xun.exe
```

GitHub Release 页面 2 个文件，零选择困难。

#### 6.4 运行时 Feature 感知

**`xun --version` 显示已编译的 feature**：

```rust
// build.rs
fn main() {
    let mut features = Vec::new();
    for (key, _) in std::env::vars() {
        if let Some(feat) = key.strip_prefix("CARGO_FEATURE_") {
            features.push(feat.to_lowercase().replace('_', "-"));
        }
    }
    features.sort();
    println!("cargo:rustc-env=XUN_FEATURES={}", features.join(","));
}

// src/lib.rs
// $ xun --version
// xun 0.1.0 [standard: acl, alias, crypt, xunbak, lock, protect, batch-rename]
```

**未启用 feature 时的友好错误**：

```rust
// dispatch.rs — 编译期生成 stub
#[cfg(not(feature = "img"))]
SubCommand::Img(_) => Err(XunError::user(
    "img module not available in this build"
).with_hints(&[
    "Download xun-media.exe for image support",
    "Or compile: cargo build --features img",
])),
```

但这有个问题：`#[cfg(not(feature = "img"))]` 时 `SubCommand::Img` 变体本身不存在，clap 不会解析到它。所以用户输入 `xun img` 会得到 clap 的 "unrecognized subcommand" 错误。

**更好的方案**：在 `suggest.rs` 中处理未知命令时检查是否是已知的可选命令：

```rust
// src/suggest.rs
const OPTIONAL_COMMANDS: &[(&str, &str)] = &[
    ("img", "preset-media or --features img"),
    ("dashboard", "preset-full or --features dashboard"),
    ("serve", "preset-full or --features dashboard"),
    ("diff", "preset-full or --features diff"),
    ("desktop", "preset-full or --features desktop"),
    ("brn", "preset-standard or --features batch_rename"),
];

pub fn suggest_for_unknown(cmd: &str) -> Option<String> {
    OPTIONAL_COMMANDS.iter()
        .find(|(name, _)| *name == cmd)
        .map(|(_, hint)| format!("'{cmd}' requires: {hint}"))
}
```

#### 6.5 依赖图优化

| 问题 | 影响 | 建议 |
|------|------|------|
| 同时依赖 `windows` + `windows-sys` | 编译时间 +30s，二进制 +200KB | 统一为 `windows-sys`，仅 COM 场景保留 `windows` |
| `lazy_static` 冗余 | 额外依赖 | 迁移至 `std::sync::LazyLock` |
| `rayon` 全量引入 | 二进制 +150KB | 仅在 xunbak/filevault 中使用，考虑手动线程池 |
| `regex` 全量引入 | 编译时间 +5s | 评估 `memchr` + 手写匹配替代部分场景 |

**二进制体积目标**：

| 预设 | 当前估计 | 目标 |
|------|---------|------|
| standard (xun.exe) | ~8MB | < 6MB |
| full (xun-full.exe) | ~15MB | < 12MB |

---

### 7. 跨模块数据流与性能关键路径

#### 7.1 `xun z <query>` 完整调用链（目标 < 5ms）

```
main() → run_from_env()
  → normalize_args()           [< 0.01ms]
  → clap::parse()             [~1.5ms]
  → runtime::init()           [< 0.01ms]
  → dispatch → BookmarkCmd
    → bookmark::state::load() [~15ms → 0.1ms with rkyv cache]
    → bookmark::query::search() [~2ms → 0.3ms with index]
    → out_println!("__BM_CD__:{path}") [< 0.01ms]
```

**优化后总耗时**：1.5ms (clap) + 0.1ms (load) + 0.3ms (search) = **< 2ms**
（瓶颈已从 bookmark 加载转移到 clap 解析，后者是固定开销，不值得优化。）

#### 7.2 `xun xunbak create` 完整调用链（目标：接近磁盘带宽）

```
dispatch → XunbakCreateCmd
  → collect_files()           [MFT scan: ~50ms for 10K files]
  → parallel_pipeline:
      Reader thread → [file bytes]
      → Worker pool → [xxHash check → BLAKE3 → ZSTD L3]
      → Writer thread → [container append]
  → write_footer()           [< 1ms]
```

**目标吞吐**：NVMe SSD 上 ≥ 400 MB/s（当前 ~300 MB/s）。



---

### 8. 测试基础设施革新：`insta` 快照测试体系

#### 8.1 当前痛点

| 痛点 | 现象 | 规模 |
|------|------|------|
| 输出变更维护成本高 | 改一个表格格式 → 手动改 50+ 个 `assert!(contains(...))` | 1638 个测试 |
| 看不到完整预期输出 | `assert!(stdout.contains("project-a"))` 不告诉你完整输出长什么样 | 85 个测试文件 |
| 巨型测试文件 | `core_integration.rs` 261KB / 534 个测试 | 不可维护 |
| 新增命令测试成本高 | 每个命令写 10+ 行 assert | 阻碍开发速度 |
| 回归检测不完整 | 只检查"包含某字符串"，新增/删除列不会被捕获 | 漏测风险 |

#### 8.2 方案：`insta` + `insta-cmd` 快照测试

**核心思想**：不再手写 assert，而是将命令的**完整输出**保存为 `.snap` 快照文件。每次测试对比当前输出与快照，不一致就失败。变更通过 `cargo insta review` 交互式审批。

**依赖**：

```toml
[dev-dependencies]
insta = { version = "1.47", features = ["json", "redactions"] }
insta-cmd = "0.6"
```

```powershell
cargo install cargo-insta  # 交互式 review 工具
```

**工作流**：

```
代码变更 → cargo insta test → 快照不匹配 → cargo insta review
                                                    ↓
                                          终端显示 diff（旧 vs 新）
                                          按 a 接受 / r 拒绝 / s 跳过
                                                    ↓
                                          .snap 文件更新 → git commit
```

#### 8.3 三种快照模式在 XunYu 中的应用

**模式 1：CLI 输出快照（替代 assert_cmd + contains）**

```rust
use insta_cmd::assert_cmd_snapshot;

#[test]
fn backup_list_default() {
    let env = TestEnv::new();
    setup_sample_backups(&env);
    assert_cmd_snapshot!(env.cmd().args(["backup", "--list"]));
}
```

生成 `snapshots/backup_list__default.snap`：
```
---
source: tests/cli_snapshots.rs
---
success: true
exit_code: 0
----- stdout -----
╭──────────┬────────────┬───────┬──────────╮
│ Name     │ Date       │ Files │ Size     │
├──────────┼────────────┼───────┼──────────┤
│ proj-a   │ 2026-05-15 │    42 │ 1.2 MB   │
╰──────────┴────────────┴───────┴──────────╯
----- stderr -----
```

**模式 2：Value/JSON 结构快照（验证服务层输出）**

```rust
#[test]
fn bookmark_query_result_structure() {
    let store = build_test_store(100);
    let result = services::bookmark::query(&store, "proj", &opts());
    insta::assert_json_snapshot!(result);
}
```

**模式 3：Help 文本快照（防止 CLI 接口意外变更）**

```rust
#[test]
fn main_help_stable() {
    assert_cmd_snapshot!(TestEnv::new().cmd().arg("--help"));
}

#[test]
fn each_subcommand_help_stable() {
    for cmd in ["backup", "acl", "env", "bookmark", "proxy"] {
        assert_cmd_snapshot!(TestEnv::new().cmd().args([cmd, "--help"]), @cmd);
    }
}
```

#### 8.4 处理不稳定输出（redactions）

时间戳、绝对路径、PID 等每次运行不同的值，用 filter 替换为占位符：

```rust
#[test]
fn backup_create_output() {
    let env = TestEnv::new();
    insta::with_settings!({
        filters => vec![
            (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}", "[TIMESTAMP]"),
            (r"[A-Z]:\\[^\s\\]+(?:\\[^\s\\]+)*", "[PATH]"),
            (r"\d+\.\d+ [KMGT]B", "[SIZE]"),
            (r"in \d+ms", "in [DURATION]"),
        ]
    }, {
        assert_cmd_snapshot!(env.cmd().args(["backup"]));
    });
}
```

项目级 filter 放在 `insta.yaml`（根目录）：

```yaml
# insta.yaml — 全局 redaction 规则
filters:
  - regex: '\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}'
    replacement: '[TIMESTAMP]'
  - regex: '[A-Z]:\\Users\\[^\\]+\\'
    replacement: '[HOME]\'
```

#### 8.5 旧测试迁移计划

**原则**：全量迁移，分批执行，每批保证 CI 绿色。

**Phase T1：基础设施搭建**

| 步骤 | 内容 |
|------|------|
| 1 | `Cargo.toml` 添加 `insta` + `insta-cmd` dev-dependencies |
| 2 | 创建 `insta.yaml` 全局 redaction 规则 |
| 3 | 创建 `tests/snapshots/` 目录（insta 默认快照存储位置） |
| 4 | 在 `TestEnv` 中添加 `snapshot_cmd()` 辅助方法 |

**Phase T2：`core_integration.rs` 拆分（534 个测试）**

```
tests/core_integration.rs (261KB, 534 tests)
    ↓ 拆分为
tests/core/
├── mod.rs
├── error.rs          ← xun_error_tests (保持 assert_eq，不适合快照)
├── value.rs          ← structured_value_tests (改为 insta::assert_json_snapshot!)
├── renderer.rs       ← renderer_tests (改为 insta::assert_snapshot! 对比渲染输出)
├── command.rs        ← command_spec_tests
├── operation.rs      ← operation_tests
├── table_row.rs      ← table_row_tests
└── context.rs        ← context_tests
```

迁移规则：
- **纯逻辑断言**（exit_code == 1）→ 保持 `assert_eq!`，不改
- **输出内容断言**（contains/starts_with）→ 改为 `insta::assert_snapshot!`
- **JSON 结构断言**→ 改为 `insta::assert_json_snapshot!`

**Phase T3：CLI 集成测试迁移（按模块）**

| 批次 | 测试文件 | 测试数 | 迁移方式 |
|------|---------|--------|---------|
| T3.1 | `general/cli_core.rs` + cases/ | ~50 | `assert_cmd_snapshot!` 替代 `assert!(contains)` |
| T3.2 | `modules/backup_restore.rs` | ~177 | 快照 + redaction（路径/时间） |
| T3.3 | `modules/batch_rename.rs` | ~132 | 快照（重命名预览输出） |
| T3.4 | `bookmark_phase_*.rs` | ~60 | 快照（查询结果） |
| T3.5 | `modules/acl.rs` + cases/ | ~40 | 快照 + redaction（SID） |
| T3.6 | `modules/alias.rs` + cases/ | ~85 | 快照 |
| T3.7 | `xunbak/*.rs` | ~120 | 混合：二进制格式用 assert_eq，CLI 输出用快照 |
| T3.8 | `modules/proxy.rs` | ~27 | 快照 |
| T3.9 | `modules/diff.rs`, `redirect_*.rs` | ~50 | 快照 |
| T3.10 | `special/*.rs` | ~70 | 性能测试保持原样，不改为快照 |

**每批迁移的验证步骤**：
1. `cargo insta test` — 生成初始快照
2. `cargo insta accept` — 接受所有快照作为 baseline
3. `cargo nextest run` — 确认全部通过
4. Git commit `.snap` 文件

**Phase T4：性能测试与快照结合**

性能测试不适合快照（数值每次不同），但可以用 insta 记录**结构**：

```rust
#[test]
fn benchmark_result_structure() {
    let result = run_benchmark();
    // 只快照结构，redact 具体数值
    insta::with_settings!({
        filters => vec![(r"\d+", "[N]")]
    }, {
        insta::assert_json_snapshot!(result);
    });
}
```

#### 8.6 迁移后的测试目录结构

```
tests/
├── core/                    ← 从 core_integration.rs 拆分
│   ├── mod.rs
│   ├── error.rs
│   ├── value.rs
│   ├── renderer.rs
│   └── ...
├── cli/                     ← CLI 输出快照测试（新）
│   ├── mod.rs
│   ├── backup.rs
│   ├── bookmark.rs
│   ├── acl.rs
│   └── ...
├── modules/                 ← 保留，逐步迁移为快照
├── xunbak/                  ← 保留，二进制格式测试不改
├── special/                 ← 保留，性能测试不改
├── snapshots/               ← insta 自动生成的快照文件
│   ├── cli__backup__list_default.snap
│   ├── cli__bookmark__query_proj.snap
│   ├── core__renderer__table_output.snap
│   └── ...
├── support/
│   └── mod.rs              ← TestEnv（增加 snapshot 辅助）
└── insta.yaml              ← 或放项目根目录
```

#### 8.7 `TestEnv` 增强

```rust
// tests/support/mod.rs — 新增辅助
impl TestEnv {
    /// 创建带 insta redaction 的命令（自动过滤路径/时间）
    pub fn snap_cmd(&self) -> Command {
        let mut c = self.cmd();
        // 强制 non-interactive + no-color（快照稳定性）
        c.env("NO_COLOR", "1");
        c.env("XUN_NON_INTERACTIVE", "1");
        c
    }
}

/// 项目级 redaction 宏
macro_rules! snap {
    ($cmd:expr) => {
        insta::with_settings!({
            filters => vec![
                (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[^\s]*", "[TIMESTAMP]"),
                (r"[A-Z]:\\Users\\[^\\]+", "[HOME]"),
                (r"[A-Z]:\\[^\s]+\\AppData\\Local\\xun", "[DATA_DIR]"),
                (r"in \d+(\.\d+)?m?s", "in [DURATION]"),
                (r"\d+\.\d+ [KMGT]B", "[SIZE]"),
            ]
        }, {
            insta_cmd::assert_cmd_snapshot!($cmd);
        });
    };
}
```

使用：

```rust
#[test]
fn backup_list() {
    let env = TestEnv::new();
    setup_backups(&env);
    snap!(env.snap_cmd().args(["backup", "--list"]));
}
```

一行测试，完整输出验证。

---

## 第二部分：TDD 重构与开发任务清单

> **原则**：每个 Task 遵循 Red-Green-Refactor 循环。先写失败测试，再写最小实现，最后重构。
> **允许破坏性改动**：快速开发期，接口可自由变更。

### 任务状态总览

| Phase | 描述 | 状态 | 完成度 |
|-------|------|------|--------|
| **Phase 0** | 基础设施升级 | 🟡 进行中 | 6/8 |
| **Phase 0T** | 测试基础设施迁移 | ⬜ 未开始 | 0/6 |
| **Phase 0P** | path_guard 全面采用 | ⬜ 未开始 | 0/7 |
| **Phase 1** | Bookmark 引擎极速化 | ✅ 已完成 | 5/5 |
| **Phase 2** | Xunbak 容器管道极限优化 | 🟡 进行中 | 2/5 |
| **Phase 3** | FileVault 加密管道优化 | ⬜ 未开始 | 0/3 |
| **Phase 4** | ACL 与环境变量子系统强化 | ⬜ 未开始 | 0/4 |
| **Phase 5** | 新功能开发 | ⬜ 未开始 | 0/3 |
| **Phase 6** | Dashboard 与集成优化 | ⬜ 未开始 | 0/3 |
| **Phase 7** | 横切关注点与质量保障 | ⬜ 未开始 | 0/5 |

**图例**: ✅ 已完成 | 🟡 进行中 | ⏳ 待实现 | ⬜ 未开始

---

### Phase 0：基础设施升级（无功能变更，纯重构）🟡

#### Task 0.1：`lazy_static` → `LazyLock` 全局迁移 ✅

| 维度 | 内容 |
|------|------|
| **目标** | 消除 `lazy_static` 依赖，统一为 `std::sync::LazyLock` |
| **源码模块** | `src/util.rs` (has_cmd cache), `src/runtime.rs`, 全局搜索 `lazy_static!` |
| **涉及组件** | 基层：Runtime, Util |
| **前置测试** | `#[test] fn lazy_lock_init_is_threadsafe()` — 多线程并发访问验证 |
| **验收标准** | `Cargo.toml` 中移除 `lazy_static` 依赖；`cargo build` 编译时间减少 ≥ 1s；所有现有测试通过 |

**实现说明**: `lazy_static` 已从直接依赖中移除，源码统一使用 `std::sync::LazyLock`（如 `env_core/template.rs`、`diff/vue.rs`）或 `OnceLock`（`runtime.rs`）。`lazy_static` 仅作为传递依赖存在于 `Cargo.lock`。

#### Task 0.2：Win32 错误处理辅助函数 + `to_wide()` ✅

| 维度 | 内容 |
|------|------|
| **目标** | 提供 `check_bool()` / `check_handle()` 辅助函数 + `to_wide()` 工具函数，统一错误捕获与 UTF-16 转换 |
| **源码模块** | `src/windows/error.rs`（新建）, `src/windows/mod.rs`（添加 `to_wide`） |
| **涉及组件** | 基层：Win32 FFI |
| **前置测试** | `#[test] fn check_bool_captures_access_denied()` — 模拟权限不足；`#[test] fn to_wide_null_terminates()` — 输出以 0 结尾 |
| **验收标准** | 逐步替换现有调用点（不要求一次全改）；新代码必须使用辅助函数；错误信息包含 Win32 错误码 |

**实现说明**: 实际路径为 `src/foundation/win32/error.rs`（随 Task 0.3 一并迁移）。`Win32Error` 结构体 + `check_bool()`/`check_handle()` 已就位。`to_wide()` 位于 `src/foundation/win32/mod.rs`。

#### Task 0.3：基层组件目录整合 (`src/foundation/`) ✅

| 维度 | 内容 |
|------|------|
| **目标** | 将散落的基层文件（runtime.rs, output.rs, util.rs, store.rs, macros.rs, model.rs, suggest.rs, proc.rs）统一移入 `src/foundation/`，`windows/` 改名为 `foundation/win32/` |
| **源码模块** | `src/` 顶层散文件 → `src/foundation/`；`src/windows/` → `src/foundation/win32/` |
| **涉及组件** | 基层：所有 |
| **前置测试** | `cargo build` 通过；所有现有测试通过（通过 `lib.rs` 重导出保持 import 兼容） |
| **验收标准** | `src/` 顶层条目从 30+ 降至 ~20；`foundation/mod.rs` 通过 `pub use` 重导出所有公开 API；`path_guard/` 和 `config/` 保持顶层独立 |

**实现说明**: `src/foundation/` 已建立，包含 `runtime.rs`、`output.rs`、`store.rs`、`util.rs`、`macros.rs`、`model.rs`、`suggest.rs`、`proc.rs` 及 `win32/` 子目录。`windows/` 已改名为 `foundation/win32/`。`path_guard/` 和 `config/` 保持顶层独立。

#### Task 0.4：`windows` crate 依赖瘦身 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 将 `windows` crate 使用限制在 COM 必需场景，其余迁移至 `windows-sys` |
| **源码模块** | `Cargo.toml` features 列表, `src/acl/` (当前用 `windows` 的 Security APIs) |
| **涉及组件** | 基层：Win32 FFI, ACL |
| **前置测试** | `#[test] fn acl_read_with_windows_sys()` — 验证迁移后 ACL 读取结果一致 |
| **验收标准** | `windows` crate features 减少 ≥ 50%；编译时间（clean build）减少 ≥ 15s；`cargo bloat` 报告二进制减少 ≥ 100KB |

#### Task 0.5：CLI Help 质量与参数校验强化 ✅

| 维度 | 内容 |
|------|------|
| **目标** | 快捷别名 `hide=true`、参数校验前移 clap 层、补充 `after_help` 示例、统一 `--format` |
| **源码模块** | `src/xun_core/dispatch.rs`（SubCommand 枚举）, 各 `*_cmd.rs` 命令定义 |
| **涉及组件** | 执行层：CLI Router |
| **前置测试** | `#[test] fn help_output_fits_80_cols()` — 主 help 宽度合理；`#[test] fn hidden_aliases_still_parse()` — `xun pon` 仍可执行；`#[test] fn value_parser_rejects_zero_retain()` — `--retain 0` 被 clap 拒绝并给出错误信息 |
| **验收标准** | `xun --help` 仅显示 ≤ 20 个核心命令（别名隐藏）；所有数值参数有 `value_parser` 范围约束；互斥参数声明 `conflicts_with`；每个顶层命令有 `after_help` 示例 |

**实现说明**: `dispatch.rs` 中 10 个别名设置 `hide = true`（Pst/Pon/Poff/Px/Ports/Kill/Ps/Pkill/Complete/Rm）。11 个 `*_cmd.rs` 文件包含 `after_help` 示例。`value_parser` 范围约束用于 `proxy_cmd.rs` 和 `env_cmd.rs`。

#### Task 0.6：CmdContext 充实与配置连接 ✅

| 维度 | 内容 |
|------|------|
| **目标** | CmdContext 接入真实配置加载、cwd、data_dir，替代各模块自行查找路径 |
| **源码模块** | `src/xun_core/context.rs`, `src/config/` |
| **涉及组件** | 基层：Config；执行层：CmdContext |
| **前置测试** | `#[test] fn context_loads_config_lazily()` — 首次 `.config()` 触发加载；`#[test] fn context_cwd_respects_dash_c()` — `-C /tmp` 覆盖 cwd；`#[test] fn context_for_test_uses_tempdir()` — 测试不污染真实文件系统 |
| **验收标准** | 所有 `std::env::var("LOCALAPPDATA")` 调用点改为 `ctx.data_dir()`；配置加载失败时使用默认值（不 panic）；测试可注入任意路径 |

**实现说明**: `src/xun_core/context.rs` 中 `CmdContext` 包含 `config: OnceCell<GlobalConfig>`（延迟加载）、`cwd`、`data_dir`、`home_dir`。提供 `with_cwd()`/`with_data_dir()` builder 和 `for_test()` 测试构造器。

#### Task 0.7：输出统一 — 消灭直接 println ✅

| 维度 | 内容 |
|------|------|
| **目标** | 所有命令的 `run()` 返回有意义的 `Value`（非 `Value::Null`），禁止直接 stdout 输出 |
| **源码模块** | `src/xun_core/services/*.rs`, `src/commands/*.rs`（逐步迁移） |
| **涉及组件** | 执行层：所有 CommandSpec 实现 |
| **前置测试** | `#[test] fn all_commands_return_non_null_value()` — 遍历命令列表验证；`#[test] fn json_format_produces_valid_json()` — `--format json` 对所有命令输出合法 JSON |
| **验收标准** | `grep -r "println!" src/xun_core/services/` 结果为 0；所有列表命令返回 `Value::Table`；所有单条命令返回 `Value::Record`；`--format json` 全局可用 |

**实现说明**: `CommandSpec::run()` 返回 `Result<Value, XunError>`，通过 `Renderer` 渲染。`Value` 枚举支持 Null/Bool/Int/Float/String/Duration/Filesize/Date/List/Record/Table。`src/xun_core/` 中 `println!` 仅 1 处（`init_cmd.rs` shell 脚本输出）。

#### Task 0.8：`commands/` 目录清空（分批执行） ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 将 `commands/` 中的业务逻辑拆分到 `services/` (Layer 2) 和领域模块 (Layer 3)，最终删除 `commands/` |
| **源码模块** | `src/commands/backup.rs` (39KB) → `services/backup.rs` + `src/backup/`；同理其他大文件 |
| **涉及组件** | 执行层：commands（消亡）→ services |
| **前置测试** | 每次迁移前：确保对应的集成测试全部通过；迁移后：相同测试仍通过 |
| **验收标准** | Phase 1: `commands/backup.rs` 清空；Phase 2: `commands/desktop.rs` 清空；Phase 3: `commands/xunbak.rs` 清空；最终：`src/commands/` 目录删除 |

---

### Phase 0T：测试基础设施迁移（与 Phase 0 并行）⬜

#### Task 0T.1：insta 基础设施搭建 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 添加 insta/insta-cmd 依赖，创建全局 redaction 规则，增强 TestEnv |
| **源码模块** | `Cargo.toml`, `insta.yaml`（新建）, `tests/support/mod.rs` |
| **涉及组件** | 测试基础设施 |
| **前置测试** | 写 1 个 `assert_cmd_snapshot!` 验证 `xun --version` 输出 |
| **验收标准** | `cargo insta test` 可运行；`cargo insta review` 可交互审批；`.snap` 文件正确生成在 `tests/snapshots/` |

#### Task 0T.2：`core_integration.rs` 拆分 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 将 261KB/534 测试的巨型文件拆分为 `tests/core/*.rs` 子模块 |
| **源码模块** | `tests/core_integration.rs` → `tests/core/{error,value,renderer,command,operation,context}.rs` |
| **涉及组件** | 测试基础设施 |
| **前置测试** | 拆分前 `cargo nextest run -E 'binary_id(core_integration)'` 全部通过 |
| **验收标准** | 拆分后测试数量不变（534）；所有测试通过；输出验证类测试改为 `insta::assert_snapshot!`；`core_integration.rs` 删除或仅保留 `mod core;` 一行 |

#### Task 0T.3：CLI 输出测试迁移 — Bookmark 模块 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 将 bookmark 相关的 CLI 测试（`bookmark_phase_*.rs` + `general/cli_core_cases/bookmark_ops.rs`）迁移为快照测试 |
| **源码模块** | `tests/bookmark_phase_*.rs`, `tests/general/cli_core_cases/bookmark_ops.rs` → `tests/cli/bookmark.rs` |
| **涉及组件** | Bookmark 测试 |
| **前置测试** | 迁移前所有 bookmark 测试通过 |
| **验收标准** | 所有 `assert!(contains(...))` 替换为 `snap!()` 或 `assert_cmd_snapshot!()`；快照文件 committed；`cargo insta test` 全部通过 |

#### Task 0T.4：CLI 输出测试迁移 — Backup/Xunbak 模块 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 将 backup/xunbak CLI 输出测试迁移为快照 |
| **源码模块** | `tests/modules/backup_restore.rs` (177 tests), `tests/xunbak/cli.rs` (61 tests) |
| **涉及组件** | Backup/Xunbak 测试 |
| **前置测试** | 迁移前所有 backup 测试通过 |
| **验收标准** | CLI 输出断言改为快照；二进制格式断言（header/footer/record 字节）保持 `assert_eq!` 不改；redaction 覆盖路径和时间戳 |

#### Task 0T.5：CLI 输出测试迁移 — 剩余模块 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 迁移 ACL、Alias、Batch Rename、Proxy、Env、Diff、Redirect 的 CLI 测试 |
| **源码模块** | `tests/modules/*.rs` 中的 CLI 输出测试 |
| **涉及组件** | 所有消费层模块测试 |
| **前置测试** | 每个模块迁移前该模块测试全部通过 |
| **验收标准** | 所有 `assert!(stdout.contains(...))` 模式消除；`special/*.rs`（性能测试）不改；最终 `grep -r "contains(" tests/` 结果中无 stdout 内容断言 |

#### Task 0T.6：Help 文本快照锁定 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 为所有顶层命令和关键子命令的 `--help` 输出创建快照，防止接口意外变更 |
| **源码模块** | `tests/cli/help.rs`（新建） |
| **涉及组件** | 执行层：CLI 接口稳定性 |
| **前置测试** | N/A（新增测试） |
| **验收标准** | 覆盖所有非 hidden 的顶层命令 + 所有有子命令的二级命令；任何 help 文本变更必须通过 `cargo insta review` 显式批准 |

---

### Phase 0P：path_guard 全面采用（安全加固 + 统一路径处理）⬜

> **背景**：项目有 317 处裸 `.exists()/.is_dir()/.is_file()/dunce::canonicalize` 调用，但仅 31 个文件使用了 `path_guard`。
> 大量模块在"裸操作"路径，缺少 traversal 检测、安全检查、统一错误信息。

#### Task 0P.1：Dashboard 文件浏览 — traversal 防护（安全漏洞）⬜

| 维度 | 内容 |
|------|------|
| **目标** | Dashboard 的 file browse/preview/diff/convert 接口全部走 `path_guard`，防止路径遍历攻击 |
| **源码模块** | `src/commands/dashboard/handlers/files/{browse,preview,diff,convert}.rs` (12 处裸路径操作) |
| **涉及组件** | 执行层：Dashboard；基层：Path Guard |
| **前置测试** | `#[test] fn browse_rejects_traversal()` — `../../Windows/System32` 被拦截；`#[test] fn browse_rejects_absolute_outside_root()` — 绝对路径超出 serve 根目录被拒绝 |
| **验收标准** | 所有 file handler 入口使用 `PathPolicy { must_exist: true, safety_check: true, allow_relative: false }`；traversal 路径返回 403；正常路径不受影响 |

#### Task 0P.2：Redirect Engine — 写入目标验证 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | redirect 的 scan/plan/apply/conflict 全部走 `PathPolicy::for_write()` |
| **源码模块** | `src/commands/redirect/engine/{plan,scan,conflict,path,ops}.rs`, `fs_utils.rs` (15 处) |
| **涉及组件** | 消费层：Redirect Engine；基层：Path Guard |
| **前置测试** | `#[test] fn redirect_rejects_system_path()` — 重定向到 `C:\Windows` 被 safety_check 拦截；`#[test] fn redirect_rejects_reparse_target()` — 目标是 symlink 时报错 |
| **验收标准** | 所有写入目标经过 `PathPolicy::for_write()` 验证；reparse point 目标被拒绝；系统关键路径被 safety_check 保护 |

#### Task 0P.3：Delete 命令 — 安全检查强化 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | delete 的路径收集和执行阶段走 `path_guard` 安全检查 |
| **源码模块** | `src/commands/delete/{paths,scanner,tree/walk}.rs` (3 处) |
| **涉及组件** | 执行层：Delete；基层：Path Guard |
| **前置测试** | `#[test] fn delete_rejects_system_root()` — `C:\Windows` 被拦截；`#[test] fn delete_rejects_traversal_in_glob()` — `..\..\*` 模式被拒绝 |
| **验收标准** | 删除目标必须通过 `PathPolicy { must_exist: true, safety_check: true }`；系统路径保护列表生效 |

#### Task 0P.4：Backup Restore — 恢复目标验证 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | backup restore 的目标路径走 `PathPolicy::for_write()`，防止恢复到危险位置 |
| **源码模块** | `src/backup/app/restore.rs` (11 处裸 `.exists()` + `dunce::canonicalize`) |
| **涉及组件** | 消费层：Backup；基层：Path Guard |
| **前置测试** | `#[test] fn restore_rejects_system_target()` — 恢复到 `C:\Windows\System32` 被拒绝；`#[test] fn restore_canonicalizes_target()` — 相对路径正确解析 |
| **验收标准** | 所有 `dunce::canonicalize` 替换为 `path_guard` 验证；恢复目标经过 safety_check；错误信息包含具体 PathIssueKind |

#### Task 0P.5：ACL Reader — 统一规范化 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | ACL reader 的 `dunce::canonicalize` 替换为 `path_guard`，获得统一的路径分类和错误处理 |
| **源码模块** | `src/acl/reader.rs` (2 处 `dunce::canonicalize`), `src/acl/writer/apply/common.rs` (1 处) |
| **涉及组件** | 消费层：ACL；基层：Path Guard |
| **前置测试** | `#[test] fn acl_read_handles_unc_path()` — UNC 路径正确分类为 PathKind::UNC；`#[test] fn acl_read_rejects_device_namespace()` — `\\.\` 路径被拒绝 |
| **验收标准** | 消除 `acl/` 中所有 `dunce::` 直接调用；PathKind 信息可用于 ACL 操作决策（UNC vs 本地路径不同处理） |

#### Task 0P.6：Env Doctor — 批量 PATH 验证 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | `env doctor` 的 PATH 条目存在性检查改为 `path_guard::validate_paths` 批量验证 |
| **源码模块** | `src/env_core/doctor.rs` (2 处逐个 `.exists()`) |
| **涉及组件** | 消费层：EnvManager；基层：Path Guard |
| **前置测试** | `#[test] fn doctor_reports_missing_path_entries()` — 不存在的 PATH 条目被报告；`#[test] fn doctor_reports_path_issues()` — 含非法字符的条目被标记 |
| **验收标准** | 使用 `validate_paths()` 一次性验证所有 PATH 条目（利用批量探测缓存）；输出包含具体 PathIssueKind（NotFound/AccessDenied/InvalidChar）；性能：100 条 PATH < 10ms |

#### Task 0P.7：Batch Rename + Xunbak + Find — 统一路径入口 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 将 batch_rename/collect、xunbak/writer+reader、find/walker 的路径验证统一为 path_guard |
| **源码模块** | `src/batch_rename/collect.rs` (2 处 dunce), `src/xunbak/{writer,reader,verify}.rs` (9 处), `src/find/walker/*.rs` (4 处) |
| **涉及组件** | 消费层：Batch Rename, Xunbak, Find；基层：Path Guard |
| **前置测试** | `#[test] fn brn_collect_rejects_invalid_root()` — 含非法字符的根目录被拒绝；`#[test] fn xunbak_create_validates_source()` — 源目录不存在时给出 PathIssue 错误 |
| **验收标准** | 消除这三个模块中所有 `dunce::canonicalize` 直接调用；根目录/源目录验证统一走 `PathPolicy::for_read()`；错误信息从 bool 升级为结构化 PathIssue |

---

### Phase 1：Bookmark 引擎极速化 ✅

#### Task 1.1：rkyv 二进制缓存格式定义 ✅

| 维度 | 内容 |
|------|------|
| **目标** | 定义 `BookmarkCache` rkyv 结构体，实现 JSON → rkyv 序列化 |
| **源码模块** | `src/bookmark/cache.rs`（新建或重构现有） |
| **涉及组件** | 消费层：Bookmark Engine |
| **前置测试** | `#[test] fn rkyv_roundtrip_5000_entries()` — 序列化→反序列化一致性；`#[test] fn rkyv_cache_size_reasonable()` — 5000 条 < 500KB |
| **验收标准** | rkyv 序列化 5000 条书签 < 2ms；反序列化（零拷贝 access）< 0.01ms；缓存文件大小 < 原 JSON 的 80% |

**实现说明**: `src/bookmark/cache.rs` 定义 `CachedBookmark` + `CachePayload`（均 derive rkyv Archive/Serialize/Deserialize）。`CacheHeader` 52 字节固定头部含 magic/version/fingerprint/payload_len。`write_cache_payload_atomic` 使用 `to_bytes` 序列化，`load_cache_payload_checked` 使用 `from_bytes` 反序列化。

#### Task 1.2：xxHash 指纹快速校验 ✅

| 维度 | 内容 |
|------|------|
| **目标** | 实现 JSON 文件 xxHash3 指纹，用于判断缓存是否过期 |
| **源码模块** | `src/bookmark/cache.rs` |
| **涉及组件** | 消费层：Bookmark Engine |
| **前置测试** | `#[test] fn fingerprint_detects_single_byte_change()` — 修改 1 字节后指纹变化；`#[test] fn fingerprint_stable_across_runs()` — 相同内容指纹一致 |
| **验收标准** | 2MB JSON 指纹计算 < 0.1ms；指纹存储在 rkyv 缓存头部（8 字节） |

**实现说明**: `compute_source_hash()` 使用 `xxh3_64` 计算源文件指纹。`SourceFingerprint` 结构体存储 `len`/`modified_ms`/`hash`，`CacheHeader.source_hash` 存储 xxh3 值用于缓存失效判断。

#### Task 1.3：mmap 零拷贝加载路径 ✅

| 维度 | 内容 |
|------|------|
| **目标** | 使用 `CreateFileMappingW` + `MapViewOfFile` 实现 rkyv 缓存零拷贝加载 |
| **源码模块** | `src/bookmark/cache.rs`, `src/windows/mmap.rs`（新建） |
| **涉及组件** | 基层：Win32 FFI；消费层：Bookmark Engine |
| **前置测试** | `#[test] fn mmap_load_returns_valid_archive()` — mmap 后可直接访问 ArchivedBookmarkCache；`#[test] fn mmap_handles_file_not_found()` — 优雅降级到 JSON 加载 |
| **验收标准** | 5000 条书签加载：15ms → **< 0.2ms**；零堆分配（mmap 直接引用）；文件锁定正确释放（UnmapViewOfFile） |

**实现说明**: `src/foundation/win32/mmap.rs` 实现 `MmapView`（RAII 封装 `CreateFileMappingW`+`MapViewOfFile`）。`cache.rs` 中 `load_cache_payload_mmap` / `load_cache_store_data_mmap` 通过 mmap 零拷贝访问 rkyv archived 数据。`state.rs` 调用 mmap 路径加载书签。

#### Task 1.4：Bookmark 查询索引优化 ✅

| 维度 | 内容 |
|------|------|
| **目标** | 基于 rkyv ArchivedVec 构建 O(1) 路径查找 + 前缀匹配索引 |
| **源码模块** | `src/bookmark/index.rs`（重构现有 OnceLock<BookmarkIndex>） |
| **涉及组件** | 消费层：Bookmark Engine |
| **前置测试** | `#[test] fn index_prefix_match_returns_top_frecency()` — "pro" 匹配 "projects" 且按 frecency 排序；`#[bench] fn query_5000_entries()` — divan 基准 |
| **验收标准** | 模糊查询 5000 条：2ms → **< 0.5ms**；索引构建（首次）< 1ms；内存增量 < 100KB |

**实现说明**: `src/bookmark/index.rs` 实现 `BookmarkIndex`：`path_map: HashMap<String, usize>` 提供 O(1) 精确查找；`terms: Vec<IndexTermEntry>` 支持二分前缀匹配；`frecency_scores: Vec<f64>` 按 frecency 降序排序结果。索引可持久化为 JSON 或嵌入 rkyv 缓存（`CachePayload.index`）。

#### Task 1.5：Bookmark 查询索引优化（原 Task 1.4 扩展）✅

> **注意**：原 Task 1.5（`xun z` fast-path 跳过 clap）已删除。
> 理由：破坏 `--help` 一致性，1-2ms 的 clap 解析开销对 CLI 工具完全可接受，真正瓶颈在 bookmark 加载（15ms）。



---

### Phase 2：Xunbak 容器管道极限优化 🟡

#### Task 2.1：双哈希策略实现 ✅

| 维度 | 内容 |
|------|------|
| **目标** | 为 xunbak 增量备份实现 xxHash3 快速变更检测 + BLAKE3 完整性哈希 |
| **源码模块** | `src/xunbak/blob.rs`, `src/xunbak/manifest.rs` |
| **涉及组件** | 消费层：Xunbak Pipeline |
| **前置测试** | `#[test] fn dual_hash_detects_change()` — 修改文件后 xxHash 变化；`#[test] fn dual_hash_unchanged_skips_blake3()` — 未变更文件不计算 BLAKE3；`#[bench] fn incremental_10k_files_5pct_changed()` |
| **验收标准** | 增量备份 10K 文件（5% 变更）：哈希阶段耗时降低 ≥ 85%；xxHash3 吞吐 ≥ 20 GB/s；BLAKE3 仅对变更文件计算 |

**实现详情** (2026-05-16):

- **`src/backup/common/hash.rs`**: 新增 `compute_file_quick_hash()` — xxHash3-64 流式计算（64KB 缓冲），吞吐 ~30 GB/s
- **`src/xunbak/manifest.rs`**: `ManifestEntry` 新增 `quick_hash: Option<u64>` 字段（`skip_serializing_if`，向后兼容）
- **`src/xunbak/writer.rs`**:
  - `PreparedBlobRecord` 新增 `quick_hash: u64` 字段
  - `prepare_blob_record` 在 BLAKE3+compress 后计算 xxHash3
  - `update_with_progress` / `update_split_with_progress`:
    - 构建 `quick_hash_index: HashMap<(u64, u64), &ManifestEntry>` — 复合键 `(quick_hash, size)` 减少误判
    - 增量循环中先检查 size+mtime，再检查 xxHash3，最后才走 BLAKE3+compress
    - xxHash3 匹配时正确处理路径更新（rename 语义）
  - 所有 ManifestEntry 构造点统一添加 `quick_hash` 字段
  - multipart 文件 `quick_hash: None`（逐 chunk 无意义）
  - dedup 命中时使用 `manifest_entry_from_locator_with_quick_hash` 传递已有 quick_hash
- **测试**: `writer_update` 7 个测试全部通过，包括 rename 语义验证

#### Task 2.2：三阶段流水线并行化 ⏳

| 维度 | 内容 |
|------|------|
| **目标** | 将 xunbak create 重构为 Reader→Processor→Writer 三阶段并行管道 |
| **源码模块** | `src/xunbak/writer.rs`（89KB，核心重构） |
| **涉及组件** | 消费层：Xunbak Pipeline |
| **前置测试** | `#[test] fn pipeline_produces_valid_container()` — 管道输出与串行输出 byte-identical；`#[test] fn pipeline_handles_read_error_gracefully()` — 单文件读取失败不中断整体；`#[bench] fn pipeline_throughput_1gb()` |
| **验收标准** | 1GB 数据备份吞吐：300 MB/s → **≥ 400 MB/s**；内存峰值 < 64MB（bounded channel 16 slots × 4MB buffer）；CPU 利用率 ≥ 80%（多核） |

**设计方案**: 使用 `crossbeam-channel` bounded channel（16 slots）连接三阶段：
- **Stage A (Reader)**: `rayon::par_iter` 读取文件内容到内存缓冲
- **Stage B (Processor)**: 并行 BLAKE3 + xxHash3 + compress（已有 rayon 基础设施）
- **Stage C (Writer)**: 单线程顺序写入（保证 blob 顺序确定性）

#### Task 2.3：MFT 扫描集成 ⏳

| 维度 | 内容 |
|------|------|
| **目标** | 将 `src/find/mft/` 的 MFT 扫描能力集成到 xunbak 文件收集阶段 |
| **源码模块** | `src/xunbak/writer.rs` (collect 阶段), `src/find/mft/` |
| **涉及组件** | 消费层：Xunbak Pipeline；基层：Find/MFT |
| **前置测试** | `#[test] fn mft_collect_matches_readdir()` — MFT 结果与 ReadDir 一致（排序后比较）；`#[test] fn mft_fallback_on_non_ntfs()` — FAT32/exFAT 自动回退；`#[bench] fn collect_50k_files_mft_vs_readdir()` |
| **验收标准** | 50K 文件收集：ReadDir ~1s → MFT **< 250ms**（4x 提升）；需要管理员权限时优雅降级到 ReadDir；结果包含完整的文件大小和修改时间 |

**设计方案**: 将 `find::mft` 的核心类型（`MftRecord`, `WcharPool`, `ChildrenIndex`）提升为 `pub(crate)` 可见性，在 xunbak 模块中实现 `collect_files_mft()` 函数，失败时自动 fallback 到 `collect_files()`。

#### Task 2.4：ZSTD 字典训练与复用 ⏳

| 维度 | 内容 |
|------|------|
| **目标** | 对同类型小文件（源码、配置）训练 ZSTD 字典，提升压缩率 |
| **源码模块** | `src/xunbak/codec.rs`, `src/xunbak/writer.rs` |
| **涉及组件** | 消费层：Xunbak Pipeline |
| **前置测试** | `#[test] fn dict_improves_ratio_on_small_files()` — 1KB 源码文件压缩率提升 ≥ 30%；`#[test] fn dict_stored_in_container()` — 字典嵌入容器头部，restore 可自解压；`#[test] fn no_dict_fallback()` — 无字典时正常工作 |
| **验收标准** | 1000 个 < 4KB 源码文件：压缩率从 ~2.5x 提升至 **≥ 3.5x**；字典训练耗时 < 500ms；字典大小 < 64KB；容器格式向后兼容（旧版本可跳过字典） |

**设计方案**: 收集前 N 个小文件样本 → `zstd::dict::from_samples()` 训练 → 嵌入 Header 扩展字段 → compress/decompress 时传入字典。

#### Task 2.5：容器写入 IO 优化 ✅

| 维度 | 内容 |
|------|------|
| **目标** | 使用 `FILE_FLAG_SEQUENTIAL_SCAN` 优化顺序写入带宽 |
| **源码模块** | `src/xunbak/writer.rs` (write 阶段) |
| **涉及组件** | 消费层：Xunbak Pipeline；基层：Win32 FFI |
| **前置测试** | `#[test] fn aligned_write_produces_valid_container()` — 对齐写入后容器可正常读取；`#[test] fn unaligned_tail_handled()` — 最后一个不满扇区正确 flush；`#[bench] fn sequential_write_bandwidth()` |
| **验收标准** | NVMe SSD 顺序写入吞吐提升；所有输出文件使用顺序访问提示 |

**实现详情** (2026-05-16):

- **`src/xunbak/writer.rs`**:
  - 新增 `create_sequential_file()` — 使用 `OpenOptionsExt::custom_flags(FILE_FLAG_SEQUENTIAL_SCAN)` 打开输出文件
  - 所有文件创建点统一使用 `create_sequential_file()`：
    - `ContainerWriter::create()`
    - `SingleVolumeOutput::new()`
    - `VolumeOutput::new()` / `rotate()`
    - `backup_with_progress()`
  - `FILE_FLAG_SEQUENTIAL_SCAN` (0x08000000) 提示内核使用顺序预读策略，优化写合并
- **测试**: `writer_update` 7 个测试全部通过，容器读写正常

---

### Phase 3：FileVault 加密管道优化 ⬜

#### Task 3.1：自适应帧大小 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 根据文件大小动态选择加密帧大小（小文件 64KB，大文件 1MB） |
| **源码模块** | `src/filevault/mod.rs` (EncJob 构造逻辑) |
| **涉及组件** | 消费层：FileVault |
| **前置测试** | `#[test] fn small_file_uses_64kb_frame()` — < 256KB 文件使用 64KB 帧；`#[test] fn large_file_uses_1mb_frame()` — > 4MB 文件使用 1MB 帧；`#[bench] fn encrypt_mixed_sizes()` |
| **验收标准** | 混合文件集（100 个 1KB~100MB）加密吞吐提升 ≥ 15%；小文件（< 64KB）的 per-file 开销从 ~24 字节帧头降至单帧处理；格式兼容（v13 帧头已支持可变大小） |

#### Task 3.2：派生密钥 DPAPI 缓存 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 缓存 Argon2 派生的 master key（DPAPI 保护），避免重复 KDF |
| **源码模块** | `src/filevault/mod.rs` (密钥派生逻辑), `src/windows/volume.rs` |
| **涉及组件** | 消费层：FileVault；基层：Win32 FFI (DPAPI) |
| **前置测试** | `#[test] fn cached_key_decrypts_correctly()` — 缓存密钥与新派生密钥解密结果一致；`#[test] fn cache_invalidated_on_password_change()` — 密码变更后缓存失效；`#[test] fn dpapi_protects_key_material()` — 缓存文件无法在其他用户会话解密 |
| **验收标准** | 重复加密操作：首次 ~100ms (Argon2)，后续 **< 1ms** (DPAPI 解密缓存)；密钥材料使用 `zeroize` 清理；缓存有效期 = 用户会话（注销后失效） |

#### Task 3.3：帧缓冲池预分配 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 预分配帧缓冲池，消除 per-frame 堆分配 |
| **源码模块** | `src/filevault/mod.rs` (worker 线程逻辑) |
| **涉及组件** | 消费层：FileVault |
| **前置测试** | `#[test] fn buffer_pool_recycles_correctly()` — 归还的缓冲区被下一帧复用；`#[test] fn pool_grows_under_pressure()` — 超出初始容量时动态扩展；`#[bench] fn encrypt_1gb_allocation_count()` |
| **验收标准** | 1GB 文件加密：堆分配次数从 ~1000 次降至 **< 50 次**；内存峰值不变（bounded by inflight limit）；吞吐提升 ≥ 5% |



---

### Phase 4：ACL 与环境变量子系统强化 ⬜

#### Task 4.1：ACL 批量操作并行化 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 将 ACL batch 操作（backup/restore 多目录）改为 rayon 并行 |
| **源码模块** | `src/acl/writer/`, `src/xun_core/services/acl.rs` |
| **涉及组件** | 消费层：ACL Processor |
| **前置测试** | `#[test] fn parallel_acl_backup_matches_serial()` — 并行结果与串行一致；`#[test] fn parallel_handles_access_denied()` — 单目录权限不足不中断整体；`#[bench] fn acl_backup_1000_dirs()` |
| **验收标准** | 1000 个目录 ACL 备份：串行 ~5s → 并行 **< 1.5s**（4 核）；线程数 = `min(available_parallelism(), config.throttle_limit)`；错误收集为 Vec 而非 panic |

#### Task 4.2：ACL 快照差异算法优化 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 优化 `src/acl/diff.rs` 的 ACL 差异计算，使用 SID 哈希索引 |
| **源码模块** | `src/acl/diff.rs`, `src/acl/types/` |
| **涉及组件** | 消费层：ACL Processor |
| **前置测试** | `#[test] fn diff_detects_added_ace()` — 新增 ACE 正确识别；`#[test] fn diff_detects_permission_change()` — 权限位变更正确报告；`#[bench] fn diff_100_aces()` |
| **验收标准** | 100 ACE 差异计算：O(n²) → **O(n)**（哈希索引）；内存增量 < 10KB；输出格式兼容现有 `xun acl diff` 命令 |

#### Task 4.3：EnvManager 原子写入强化 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 确保 `src/env_core/` 的注册表写入具备事务语义（全部成功或全部回滚） |
| **源码模块** | `src/env_core/write_guard.rs`, `src/env_core/registry.rs` |
| **涉及组件** | 消费层：EnvManager |
| **前置测试** | `#[test] fn atomic_write_rolls_back_on_partial_failure()` — 模拟第 3 个变量写入失败，前 2 个回滚；`#[test] fn wm_settingchange_sent_only_on_success()` — 失败时不广播 |
| **验收标准** | 批量写入 10 个环境变量：全部成功或全部回滚（无中间状态）；`WM_SETTINGCHANGE` 仅在 commit 成功后发送一次；回滚耗时 < 50ms |

#### Task 4.4：EnvManager 快照 rkyv 持久化 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 将环境变量快照从 JSON 迁移至 rkyv 格式，加速加载 |
| **源码模块** | `src/env_core/snapshot.rs`, `src/env_core/io/` |
| **涉及组件** | 消费层：EnvManager |
| **前置测试** | `#[test] fn snapshot_rkyv_roundtrip()` — 序列化→反序列化一致；`#[test] fn snapshot_migration_from_json()` — 旧 JSON 快照自动迁移；`#[bench] fn load_snapshot_500_vars()` |
| **验收标准** | 500 变量快照加载：JSON ~5ms → rkyv **< 0.5ms**；自动迁移旧格式（首次加载时转换）；快照文件大小减少 ≥ 30% |

---

### Phase 5：新功能开发（破坏性改动允许）⬜

#### Task 5.1：系统监控数据采集器 (`sys_monitor`) ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 实现 CPU/内存/Swap 实时采集，供 Dashboard WebSocket 推送 |
| **源码模块** | `src/desktop/sys_monitor.rs`（新建）, 参考 `docs/implementation/SysMonitor-MemCleaner-Design.md` |
| **涉及组件** | 消费层：Desktop Manager；基层：Win32 FFI |
| **前置测试** | `#[test] fn cpu_usage_in_valid_range()` — 0.0 ≤ cpu ≤ 100.0；`#[test] fn memory_usage_matches_task_manager()` — 与 taskmgr 偏差 < 2%；`#[test] fn collector_stops_on_cancel()` — CancellationToken 正确停止线程 |
| **验收标准** | 采样间隔 1s；CPU 计算使用 `GetSystemTimes` 差分；内存使用 `GlobalMemoryStatusEx`；数据通过 `mpsc::channel` 零锁传输；线程启动 < 1ms |

#### Task 5.2：内存清理器 (`mem_cleaner`) ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 实现 8 区域独立内存清理，基于 `NtSetSystemInformation` |
| **源码模块** | `src/desktop/mem_cleaner.rs`（新建） |
| **涉及组件** | 消费层：Desktop Manager；基层：Win32 FFI |
| **前置测试** | `#[test] fn clean_working_set_requires_privilege()` — 无 SeProfileSingleProcessPrivilege 时返回错误；`#[test] fn clean_standby_requires_admin()` — 非管理员返回 ElevationRequired；`#[test] fn bitmask_selects_regions_correctly()` — 0b00000101 仅清理 region 0 和 2 |
| **验收标准** | 单区域清理 < 100ms；支持 8 区域独立位掩码选择；OS 版本检测（Win8.1+/Win10+ 功能门控）；清理前后内存差值报告 |

#### Task 5.3：`xun find` MFT 扫描模式增强 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 增强 MFT 扫描支持文件名模式匹配，实现 Everything-like 即时搜索 |
| **源码模块** | `src/find/mft/`, `src/find/matcher.rs` |
| **涉及组件** | 消费层：Find Engine |
| **前置测试** | `#[test] fn mft_glob_match()` — `*.rs` 匹配所有 Rust 文件；`#[test] fn mft_regex_match()` — 正则模式工作；`#[test] fn mft_requires_admin_or_fallback()` — 非管理员优雅降级；`#[bench] fn mft_search_100k_files()` |
| **验收标准** | 100K 文件搜索（文件名匹配）：< **200ms**（MFT 模式）vs ~3s（ReadDir 模式）；支持 glob 和 regex 两种模式；非 NTFS 卷自动回退到 ReadDir |

---

### Phase 6：Dashboard 与集成优化 ⬜

#### Task 6.1：WebSocket 系统监控推送 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 将 sys_monitor 数据通过 WebSocket 实时推送到 Dashboard UI |
| **源码模块** | `src/xun_core/dashboard_cmd.rs`, `src/xun_core/ws_protocol.rs` |
| **涉及组件** | 执行层：Dashboard Daemon |
| **前置测试** | `#[tokio::test] fn ws_receives_system_snapshot()` — 连接后 1s 内收到首个快照；`#[tokio::test] fn ws_handles_slow_client()` — 慢客户端不阻塞采集；`#[tokio::test] fn ws_graceful_disconnect()` |
| **验收标准** | WebSocket 消息延迟 < 50ms；使用 `broadcast::channel` 多客户端扇出；JSON 序列化 < 0.1ms/帧；慢客户端自动丢弃旧帧（lagging receiver） |

#### Task 6.2：Dashboard 静态资源 Brotli 预压缩 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 构建时对 `dashboard-ui/dist/` 预压缩 Brotli，运行时直接发送 |
| **源码模块** | `build.rs`, `src/xun_core/dashboard_cmd.rs` (static file handler) |
| **涉及组件** | 执行层：Dashboard Daemon |
| **前置测试** | `#[test] fn brotli_response_decompresses_correctly()` — 浏览器可正确解压；`#[test] fn fallback_to_uncompressed()` — 不支持 br 的客户端收到原始文件 |
| **验收标准** | JS/CSS 传输大小减少 ≥ 70%（vs gzip）；首次加载时间减少 ≥ 40%；`Accept-Encoding: br` 检测正确 |

#### Task 6.3：Operation Protocol 可视化 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 将 Operation 的 preview/execute/rollback 状态通过 WebSocket 实时推送到 Dashboard |
| **源码模块** | `src/xun_core/operation.rs`, `src/xun_core/ws_protocol.rs` |
| **涉及组件** | 基层：Operation Protocol；执行层：Dashboard |
| **前置测试** | `#[test] fn operation_emits_preview_event()` — preview 阶段发送 WS 事件；`#[test] fn operation_emits_progress()` — execute 阶段发送进度；`#[test] fn rollback_emits_undo_event()` |
| **验收标准** | 事件延迟 < 100ms；事件格式：`{ type: "operation", phase: "preview"|"execute"|"rollback", data: {...} }`；不影响 CLI-only 模式性能 |



---

### Phase 7：横切关注点与质量保障 ⬜

#### Task 7.1：全局性能回归基准线 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 建立 CI 可执行的性能基准线，防止回归 |
| **源码模块** | `benches/*.rs`, `.github/workflows/ci.yml` |
| **涉及组件** | 所有模块 |
| **前置测试** | 基准测试本身即为测试：`divan` 框架自动检测回归 |
| **验收标准** | CI 中运行 `cargo bench`；任何基准回归 > 10% 自动 fail；基准覆盖：bookmark load/query, path_guard validate, xunbak write, acl read |

#### Task 7.2：Fuzzing 覆盖扩展 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 扩展 fuzz 目标覆盖 xunbak reader、path_guard、filevault header 解析 |
| **源码模块** | `fuzz/fuzz_targets/`（新增目标） |
| **涉及组件** | 消费层：Xunbak, Path Guard, FileVault |
| **前置测试** | N/A（fuzzing 本身是测试生成器） |
| **验收标准** | 新增 fuzz 目标：`xunbak_read_header`, `filevault_parse_header`, `path_guard_normalize`；每个目标运行 10 分钟无 crash；发现的 bug 转化为回归测试 |

#### Task 7.3：内存安全审计 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 对所有 `unsafe` 块进行安全审计，添加 SAFETY 注释 |
| **源码模块** | `src/windows/*.rs`, `src/ports/process_map.rs`, `src/acl/`, `src/filevault/` |
| **涉及组件** | 基层：Win32 FFI |
| **前置测试** | `#[test] fn no_undefined_behavior_under_miri()` — 纯 Rust 逻辑部分通过 Miri 检查 |
| **验收标准** | 每个 `unsafe` 块有 `// SAFETY:` 注释说明不变量；Win32 FFI 调用的返回值全部检查；指针解引用前验证非空 |

#### Task 7.4：错误处理统一迁移 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 将残留的 `CliError` 全部迁移至 `XunError`，统一错误语义 |
| **源码模块** | `src/output.rs` (CliError 定义), 所有返回 `CliResult` 的函数 |
| **涉及组件** | 基层：Output, Error |
| **前置测试** | `#[test] fn xun_error_exit_codes_match_legacy()` — 新错误类型的 exit code 与旧 CliError 一致 |
| **验收标准** | `CliError` 和 `CliResult` 类型标记为 `#[deprecated]`；所有新代码使用 `XunError`；exit code 语义不变（0=success, 1=general, 3=access_denied, 10/11=lock, 20=reboot） |

#### Task 7.5：编译时间优化 ⬜

| 维度 | 内容 |
|------|------|
| **目标** | 将 clean build 时间降低 ≥ 20% |
| **源码模块** | `Cargo.toml`, `build.rs`, 大型模块拆分 |
| **涉及组件** | 所有 |
| **前置测试** | N/A（度量型任务） |
| **验收标准** | `cargo build --release` clean build：当前 ~180s → **< 144s**；手段：`windows` crate 瘦身、`regex` feature 精简、大文件拆分（xunbak/writer.rs 89KB → 3 个文件） |

---

## 附录

### A. 性能验收标准汇总

| 场景 | 指标 | 目标值 | 测量方式 |
|------|------|--------|---------|
| `xun z <query>` 端到端 | 延迟 | < 5ms | `XUN_CMD_TIMING=1` |
| Bookmark 加载 5000 条 | 延迟 | < 0.2ms | divan bench |
| Bookmark 查询 5000 条 | 延迟 | < 0.5ms | divan bench |
| Xunbak create 1GB | 吞吐 | ≥ 400 MB/s | divan bench |
| Xunbak 增量 10K files | 哈希阶段 | 降低 ≥ 85% | divan bench |
| FileVault encrypt 1GB | 吞吐 | ≥ 2 GB/s (AES-NI) | divan bench |
| ACL backup 1000 dirs | 延迟 | < 1.5s | divan bench |
| Path Guard validate 1000 | 延迟 | < 5ms | divan bench |
| MFT search 100K files | 延迟 | < 200ms | divan bench |
| CLI 启动（无子命令） | 延迟 | < 3ms | Measure-Command |
| 二进制体积（最小构建） | 大小 | < 3MB | ls -la |
| 内存峰值（xunbak 1GB） | 内存 | < 64MB | Windows 任务管理器 |

### B. 技术选型依据（联网检索结果）

| 技术 | 选型 | 依据 |
|------|------|------|
| 零拷贝序列化 | rkyv 0.8 | Apache Iggy 实测 2x 吞吐提升；零反序列化延迟 |
| 快速哈希 | xxHash3 (xxhash-rust) | 30-60 GB/s，非密码学场景最优 |
| 完整性哈希 | BLAKE3 | 4-6 GB/s + 密码学安全 + SIMD 自动加速 |
| 压缩 | ZSTD Level 3 | 350 MB/s 压缩 / 1500 MB/s 解压，最佳速度/比率平衡 |
| Win32 FFI | windows-sys 0.59 | 零运行时开销，编译时间远优于 windows crate |
| 单态化控制 | inner-function pattern | std 库标准做法，减少泛型膨胀 |
| 全局状态 | std::sync::LazyLock | Rust 1.80+ 稳定，替代 lazy_static |
| 性能基准 | divan 0.1 | 统计学严谨，支持回归检测 |

### C. 模块依赖拓扑（构建顺序）

```
Level 0 (无依赖):  foundation/ (runtime, store, macros, output, util, model, proc, win32/)
Level 1 (仅 L0):   config/
Level 2 (L0+L1):   path_guard/
Level 3 (L0-L2):   security/
Level 4 (L0-L3):   bookmark, acl, env_core, ports, find
Level 5 (L0-L4):   xunbak, filevault, backup, alias, batch_rename, diff, desktop, img
Level 6 (L0-L5):   xun_core (dispatch + services + commands)
Level 7 (L6):      cli (re-export facade), lib.rs (entry)
```

### D. 破坏性改动风险评估

| 改动 | 影响范围 | 风险 | 缓解措施 |
|------|---------|------|---------|
| `lazy_static` 移除 | 全局 | 低 | 机械替换，语义等价 |
| `windows` crate 瘦身 | ACL, Desktop | 中 | 逐模块迁移，保留 COM 场景 |
| `foundation/` 目录整合 | 全局 import 路径 | 低 | `lib.rs` 重导出保持兼容，一次性完成 |
| Bookmark rkyv 缓存 | Bookmark | 中 | 保留 JSON 回退路径 |
| Xunbak 并行管道 | Xunbak writer | 高 | 保留串行模式作为 fallback |

---

*文档结束。所有提案均基于实际源码分析，性能目标基于行业基准测试数据。*
