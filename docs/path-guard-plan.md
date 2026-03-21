# Windows 路径校验组件方案（Path Guard）

## 背景与目标

项目内多条命令需要解析与校验路径（`--path`/`--paths`/`--file`），当前实现分散。Path Guard 作为 Windows-only 校验组件，统一输入规范、错误分类与性能策略。

目标：
- 统一路径合法性与存在性校验，输出结构化结果。
- 使用 `windows-sys` 与 WinAPI，避免 `std::fs` 的额外开销。
- 面向批量场景，保持低分配与高吞吐。

非目标：
- 不做跨平台语义兼容。
- 不替代业务层安全策略（例如系统目录黑名单）。

## 设计原则

- KISS：仅做路径校验与探测，不耦合业务逻辑。
- DRY：所有命令统一入口与错误模型。
- YAGNI：不引入跨平台路径语义与多余抽象。

## 目录结构（精简 5 文件）

```
src/path_guard/
  mod.rs          # 公共入口与 re-export
  policy.rs       # PathPolicy / PathKind / PathIssue / PathInfo
  string_check.rs # UTF-16 规则校验 + FSM
  winapi.rs       # GetFileAttributesW/ExW + GetFullPathNameW + CreateFileW
  parallel.rs     # 并行入口（按阈值启用）
```

说明：
- 不引入 feature 开关，直接使用项目现有 `windows-sys` 依赖。
- 规则校验仅在 `string_check.rs`，避免层级过深。

## 公开接口与数据结构

```rust
pub struct PathPolicy {
    pub must_exist: bool,
    pub allow_relative: bool,
    pub expand_env: bool,
    pub allow_reparse: bool,
    pub allow_ads: bool,
    pub base: Option<PathBuf>,
    pub safety_check: bool,
    pub cwd_snapshot: Option<PathBuf>,
}

pub enum PathKind {
    DriveAbsolute,
    DriveRelative,
    Relative,
    UNC,
    ExtendedLength,
    ExtendedUNC,
    DeviceNamespace,
    NTNamespace,
    VolumeGuid,
    ADS,
}

pub enum PathIssueKind {
    Empty,
    InvalidChar,
    ReservedName,
    TrailingDotSpace,
    TooLong,
    RelativeNotAllowed,
    DriveRelativeNotAllowed,
    TraversalDetected,
    NotFound,
    AccessDenied,
    ReparsePoint,
    AdsNotAllowed,
    DeviceNamespaceNotAllowed,
    NtNamespaceNotAllowed,
    VolumeGuidNotAllowed,
    EnvVarNotAllowed,
    NetworkPathNotFound,
    SharingViolation,
    SymlinkLoop,
    IoError,
}

pub struct PathIssue {
    pub raw: String,
    pub kind: PathIssueKind,
    pub detail: &'static str,
}

pub struct PathValidationResult {
    pub ok: Vec<PathBuf>,
    pub issues: Vec<PathIssue>,
    pub deduped: usize,
}

pub struct PathInfo {
    pub path: PathBuf,
    pub kind: PathKind,
    pub canonical: Option<PathBuf>,
    pub is_reparse_point: bool,
    pub is_directory: Option<bool>,
    pub existence_probe: Option<PathIssueKind>,
}

pub fn validate_paths<I, P>(inputs: I, policy: &PathPolicy) -> PathValidationResult
where
    I: IntoIterator<Item = P>,
    P: AsRef<OsStr>;

pub fn validate_paths_with_info<I, P>(
    inputs: I,
    policy: &PathPolicy,
) -> (Vec<PathInfo>, Vec<PathIssue>)
where
    I: IntoIterator<Item = P>,
    P: AsRef<OsStr>;

pub fn validate_single(
    raw: &OsStr,
    policy: &PathPolicy,
    scratch: &mut Vec<u16>,
) -> Result<PathInfo, PathIssue>;
```

WinAPI 辅助接口位于 `path_guard::winapi`，包含：
- `probe(&Path) -> Result<u32, PathIssueKind>`
- `probe_ex(&Path) -> Result<WIN32_FILE_ATTRIBUTE_DATA, PathIssueKind>`
- `get_full_path(&Path) -> Result<PathBuf, PathIssueKind>`
- `open_path_with_policy(&Path, &PathPolicy) -> Result<OwnedHandle, PathIssueKind>`
- `get_final_path(&OwnedHandle) -> Result<PathBuf, PathIssueKind>`

## 输入与分配策略

- 输入接受 `AsRef<OsStr>`，避免强制 UTF-8。
- 规则校验使用 UTF-16（`encode_wide`）并复用 `Vec<u16>`，避免堆分配。
- 去重使用 `IndexSet<Vec<u16>>`，在 UTF-16 层做大小写与分隔符归一化，保序去重。

## Windows 路径类型策略

默认策略：
- Drive absolute 允许。
- Relative 受 `allow_relative` 控制。
- Drive relative 默认拒绝。
- UNC 允许，但并行阶段降并发。
- Extended length 允许（WinAPI 内部补 `\\?\` 前缀）。
- Device/NT/Volume GUID 默认拒绝。
- ADS 默认拒绝。

保留设备名：
- `CON/PRN/AUX/NUL/COM1~9/LPT1~9`，含 superscript 版本（`COM¹/COM²/COM³`、`LPT¹/²/³`）。
- `string_check::reserved_names()` 作为唯一来源，`delete` 模块复用，保持 DRY。

## 校验顺序（核心流程）

1. UTF-16 字符检查（控制字符/非法字符）。
2. FSM 检测 `PathKind`。
3. ADS 与保留名校验。
4. `allow_relative` 与 `DriveRelative` 判定。
5. `expand_env` 开启时先展开，再对展开结果重新校验。
6. 相对路径拼接：
   - 使用 `cwd_snapshot` 或当前目录。
   - 调用 `GetFullPathNameW` 规范化。
7. `GetFileAttributesW` 或 `GetFileAttributesExW` 探测存在性与属性。
8. `FILE_ATTRIBUTE_REPARSE_POINT` 判定（受 `allow_reparse` 控制）。
9. 高风险写路径可进一步使用 `open_path_with_policy` 进行原子打开。

## WinAPI 封装与错误映射

错误映射：
- 2/3 → `NotFound`
- 5 → `AccessDenied`
- 32 → `SharingViolation`
- 53 → `NetworkPathNotFound`
- 206 → `TooLong`
- 1921 → `SymlinkLoop`
- 其他 → `IoError`

说明：
- 所有 `unsafe` 限制在 `winapi.rs`。
- `GetFileAttributesExW` 用于 `PathInfo` 的 `is_directory/is_reparse_point` 与 `existence_probe`。
- `GetFinalPathNameByHandleW` 仅在 `validate_paths_with_info` 的允许场景中使用。

## 并行策略

- `<64`：串行。
- `64~500`：Rayon 并行。
- `>500` 且含 UNC：I/O 阶段线程数降为 `min(4, num_cpus)`。

并行模型：
- 阶段 A：字符串校验（Rayon）。
- 阶段 B：WinAPI I/O（独立线程池 + crossbeam channel）。

## 去重与顺序

- 输入统一去重，去重 key 为 UTF-16 归一化结果。
- 返回结果保持输入顺序。
- 调用方不再重复 `sort/dedup`。

## 接入策略（按风险）

P0（高风险写入/删除）：
- `delete`、`acl` 写操作、`redirect --apply`、`vault/crypt` 输出路径。

P1（中风险写入）：
- `restore`、`img` 输出目录、`brn --apply`。

P2（只读/分析）：
- `find`、`cstat`、`diff`、`redirect --plan`。

P3（配置/弱约束）：
- `bookmarks set`、`ctx set`。

## 测试与基准

单元测试：
- 非法字符、保留名、末尾空格/句号、ADS、命名空间。
- `expand_env` 开关与相对路径行为。

集成测试：
- `--file/--paths` 组合输入。
- UNC 路径（可用环境变量控制）。

性能测试：
- 1k/5k/10k 路径批量基线（记录 p50/p95）。
- fuzz（`cargo-fuzz`）覆盖边界组合。

## 风险与约束

- long path 支持依赖系统策略与 manifest。
- UNC/网络盘需降并发，避免 I/O 反噬。
- 存在性检查仅为快照，写操作需用原子打开规避 TOCTTOU。

## 参考文档

- Windows 路径规则：`https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file`
- GetFileAttributesW：`https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileattributesw`
- GetFileAttributesExW：`https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileattributesexw`
- GetFullPathNameW：`https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfullpathnamew`
- GetFinalPathNameByHandleW：`https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew`
- CreateFileW：`https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew`
- ExpandEnvironmentStringsW：`https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-expandenvironmentstringsw`
- 文件属性常量：`https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants`
- long path 机制：`https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation`
- windows-sys vs windows：`https://microsoft.github.io/windows-rs/book/rust-getting-started/windows-or-windows-sys.html`
