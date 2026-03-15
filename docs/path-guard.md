# Path Guard 方案（精简版，面向项目集成）

本方案基于项目现有命令与 Windows 路径体系，目标是提供高性能、低分配、Windows-only 的路径校验模块，并与现有 `windows-sys` 依赖与 `windows::safety::ensure_safe_target` 形成清晰分工。

## 目标与边界

目标：
- 统一 CLI 路径合法性与存在性校验，避免各命令重复实现。
- 强化高风险写路径的安全性与一致性。
- 保持低分配与高吞吐，满足批量校验场景。

非目标：
- 不做跨平台语义兼容。
- 不替代业务层安全策略（例如系统目录黑名单）。

## 目录结构（精简后）

建议结构：
```
src/path_guard/
  mod.rs          # 公共接口与 re-export
  policy.rs       # PathPolicy / PathKind / PathIssue
  string_check.rs # FSM + 保留名 + ADS + 规则校验
  winapi.rs       # GetFileAttributesW/ExW + GetFullPathNameW + CreateFileW + GetFinalPathNameByHandleW
  parallel.rs     # 批量入口与并行策略（可选）
```

说明：
- 不新增 feature 开关，直接复用项目已有的 `windows-sys` 依赖。
- FSM、保留名、去重、长路径前缀合并到 `string_check.rs` 与 `winapi.rs` 内部函数，减少模块粒度。

## 数据结构（精简版）

PathPolicy 建议：
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
```

设计说明：
- `allow_device_namespace` 固定为 false，无需暴露。
- `allow_long` 固定为 true，内部统一补 `\\?\` 前缀。
- `max_len` 删除，和 `allow_long=true` 冲突。
- `expand_env` 仅在 env 类命令需要时开启。
- `cwd_snapshot` 用于并行环境下冻结 CWD，避免竞态。
- `safety_check` 只用于 P0 写路径，避免批量只读场景额外 IO。

建议提供预设 Builder：
```rust
impl PathPolicy {
    pub fn for_write() -> Self { /* must_exist=true, allow_reparse=false, safety_check=true */ }
    pub fn for_read() -> Self { /* must_exist=true, allow_reparse=true, safety_check=false */ }
    pub fn for_output() -> Self { /* must_exist=false, safety_check=false */ }
}
```

PathKind 建议：
```rust
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
```

PathIssueKind 建议补充：
```rust
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
    NetworkPathNotFound,  // ERROR_BAD_NETPATH (53)
    SharingViolation,     // ERROR_SHARING_VIOLATION (32)
    SymlinkLoop,          // ERROR_CANT_RESOLVE_FILENAME (1921)
    IoError,
}
```

PathIssue / PathInfo：
```rust
pub struct PathIssue {
    pub raw: String,
    pub kind: PathIssueKind,
    pub detail: &'static str,
}

pub struct PathInfo {
    pub path: PathBuf,
    pub kind: PathKind,
    pub canonical: Option<PathBuf>,
    pub is_reparse_point: bool,
    pub is_directory: Option<bool>,
    pub existence_probe: Option<PathIssueKind>,
}
```

## Windows 路径类型策略

默认策略：
- Drive absolute 允许。
- Relative 受 `allow_relative` 控制。
- Drive relative 默认拒绝。
- UNC 允许但限并发。
- Extended length 允许（内部补前缀）。
- Device namespace、NT namespace、Volume GUID 默认拒绝。
- ADS 默认拒绝。

保留设备名：
- 使用与 `delete` 模块一致的列表，包含 superscript 版本（`COM¹/COM²/COM³`、`LPT¹/LPT²/LPT³`）。
- 建议将 `reserved_names()` 放入 `path_guard/string_check.rs` 并设为 `pub(crate)`，`delete` 模块复用以保持 DRY。

## 校验顺序（核心流程）

1. UTF-16 字符检查（控制字符/非法字符）。
2. FSM 检测 `PathKind`。
3. ADS 检测与保留设备名检测。
4. 处理 `allow_relative` 与 `DriveRelative` 特判。
5. `expand_env` 展开（启用时）并再次校验输出。
6. 若为相对路径，使用 `cwd_snapshot`（若有）或 `std::env::current_dir()` 拼接并 `GetFullPathNameW` 规范化。
7. `GetFileAttributesW/ExW` 探测存在性与属性。
8. `FILE_ATTRIBUTE_REPARSE_POINT` 判断（受 `allow_reparse` 控制）。
9. `safety_check` 为 true 时调用 `windows::safety::ensure_safe_target`（仅 P0 写路径启用）。
10. `validate_paths_with_info` 场景可选使用 `open_path_with_policy` + `GetFinalPathNameByHandleW` 获取 canonical。

## WinAPI 封装（精简版）

推荐接口：
- 对外：`probe(path: &Path) -> Result<u32, PathIssueKind>`。
- 对外：`probe_ex(path: &Path) -> Result<WIN32_FILE_ATTRIBUTE_DATA, PathIssueKind>`。
- 对外：`get_full_path(path: &Path) -> Result<PathBuf, PathIssueKind>`。
- 对外：`open_path_with_policy(path: &Path, policy: &PathPolicy) -> Result<OwnedHandle, PathIssueKind>`。
- 对外：`get_final_path(handle: &OwnedHandle) -> Result<PathBuf, PathIssueKind>`。
- 对内：`probe_path(wide: &[u16]) -> Result<u32, PathIssueKind>`（并行阶段复用缓冲区）。

错误映射建议：
- 2/3 → NotFound
- 5 → AccessDenied
- 32 → SharingViolation
- 53 → NetworkPathNotFound
- 206 → TooLong
- 1921 → SymlinkLoop
- 其他 → IoError

## 并行策略（阈值）

建议阈值：
- `< 64` 条：单线程执行。
- `64~500` 条：Rayon 默认线程池。
- `> 500` 且含 UNC：UNC 降并发至 `min(4, num_cpus)`。

并行注意事项：
- 阶段 A 纯字符串校验可使用 Rayon。
- WinAPI 阶段建议独立阻塞线程池，避免 I/O 阻塞 Rayon。
- 增强阶段可用 `crossbeam-channel` 连接 CPU 阶段与 I/O worker，避免新增依赖。
- `mod.rs` 作为唯一公共入口，按阈值与 UNC 判定选择串行或调用 `parallel.rs` 管线，调用方无需感知并行分支。

## 与系统安全防护的协作

保留 `windows::safety::ensure_safe_target`：
- Path Guard 负责“路径格式合法性”。
- safety 负责“系统目录黑名单保护”。
- `PathPolicy.safety_check=true` 时在返回 ok 结果前调用 safety。

## 当前落地范围

- `string_check.rs`：UTF-16 规则校验 + FSM + 保留名 + ADS。
- `winapi.rs`：`GetFileAttributesW/ExW`、`GetFullPathNameW`、`CreateFileW`、`GetFinalPathNameByHandleW` 封装。
- `validate_paths` / `validate_paths_with_info` / `validate_single` 入口。
- 并行管线按阈值自动启用。
- P0 写路径使用 `open_path_with_policy` 降低 TOCTTOU 风险。

可选增强：
- 更细粒度的错误统计与性能埋点。
- UNC/SMB 压测与动态并发调节。

## 去重与顺序保证

- `validate_paths` 内部负责去重（UTF-16 层大小写与分隔符归一化）。  
- 返回顺序保持输入顺序并稳定去重；如需排序，交由调用方显式处理。  
- 接入 `acl add` 后，删除其内部 `sort/dedup`，避免双重处理与顺序混乱。

## 性能优化建议（MVP 优先）

1. UTF-16 缓冲复用：使用 `thread_local` 复用 `Vec<u16>`，避免批量场景频繁分配。
2. 非法字符扫描：优先用字节级扫描，结合 `memchr` 或 SIMD 友好循环替代 `chars()`。
3. FSM 前缀匹配：用字节数组匹配前缀，避免 `starts_with(&str)` 的 UTF-8 解码开销。
4. 去重容器：若需要保序，可选 `IndexSet`；不新增依赖时用 `AHashSet + Vec` 手动保序。
5. 避免中间 `String` 克隆：去重只对 key 分配，`PathBuf` 直接 move 到结果。
6. `NtQueryAttributesFile`：不纳入 MVP 与增强阶段，除非出现 UNC/SMB 极端瓶颈才评估。

## 参考文档

- Windows 路径命名规范：`https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file`
- GetFileAttributesW：`https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileattributesw`
- GetFileAttributesExW：`https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileattributesexw`
- GetFullPathNameW：`https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfullpathnamew`
- ExpandEnvironmentStringsW：`https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-expandenvironmentstringsw`
