# Path Guard 任务清单

> 基于 `docs/path-guard.md`（精简版方案）编写。
> 状态标记：`[ ]` 待开始 · `[~]` 进行中 · `[x]` 完成

---

## 阶段 0：准备工作

- [x] **T0-1** 删除 `src/windows/path_validator.rs`（早期草稿，与本方案不符）
- [x] **T0-2** 在 `src/lib.rs` 中注册 `mod path_guard;`
- [x] **T0-3** 创建目录 `src/path_guard/` 及以下占位文件：
  - `src/path_guard/mod.rs`
  - `src/path_guard/policy.rs`
  - `src/path_guard/string_check.rs`
  - `src/path_guard/winapi.rs`
  - `src/path_guard/parallel.rs`（增强阶段填充）
- [x] **T0-4** 确认 `Cargo.toml` 依赖（无需新增 feature 开关）：
  - `windows-sys`（`Win32_Storage_FileSystem`、`Win32_Foundation` 已开启）
  - `rayon`（已有）
  - `memchr`（已有，`cstat` feature 下）
  - `indexmap`：若未存在则新增 `indexmap = { version = "2", features = ["std"] }`

---

## 阶段 1：类型定义（`policy.rs`）

> 零 unsafe，零 IO，纯类型定义。

- [x] **T1-1** 定义 `PathKind` 枚举：
  `DriveAbsolute | DriveRelative | Relative | UNC | ExtendedLength | ExtendedUNC | DeviceNamespace | NTNamespace | VolumeGuid | ADS`

- [x] **T1-2** 定义完整 `PathIssueKind` 枚举（含 Win32 错误码注释，不省略）：
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

- [x] **T1-3** 定义 `PathIssue` 结构体：
  - `raw: String`（原始输入）
  - `kind: PathIssueKind`
  - `detail: &'static str`（静态字符串，避免堆分配）

- [x] **T1-4** 定义 `PathValidationResult` 结构体：
  - `ok: Vec<PathBuf>`（去重后、通过所有策略的路径，保持输入顺序）
  - `issues: Vec<PathIssue>`
  - `deduped: usize`（去重丢弃的条数）

- [x] **T1-5** 定义 `PathPolicy` 结构体：
  ```rust
  pub struct PathPolicy {
      pub must_exist: bool,
      pub allow_relative: bool,
      pub expand_env: bool,
      pub allow_reparse: bool,
      pub allow_ads: bool,
      pub base: Option<PathBuf>,
      pub safety_check: bool,       // 仅 P0 写路径启用
      pub cwd_snapshot: Option<PathBuf>, // 并行时冻结 CWD
  }
  // allow_long 固定 true（内部处理，不暴露）
  // allow_device_namespace 固定 false（内部处理，不暴露）
  ```

- [x] **T1-6** 为 `PathPolicy` 实现 3 个预设 builder：
  - `for_write()`：`must_exist=true, allow_reparse=false, safety_check=true, allow_relative=false`
  - `for_read()`：`must_exist=true, allow_reparse=true, safety_check=false, allow_relative=true`
  - `for_output()`：`must_exist=false, safety_check=false, allow_relative=false`

- [x] **T1-7** 为 `PathPolicy` 实现 `Default`，等同于 `for_read()`

- [x] **T1-8** 单元测试：builder 各字段值正确；`Default` 与 `for_read()` 等价

---

## 阶段 2：字符串校验（`string_check.rs`）

> 零 IO，纯字符串操作。所有对外函数为 `pub(crate)`。

- [x] **T2-1** 实现 `pub(crate) fn reserved_names() -> &'static [&'static str]`：
  - 静态切片，全小写，包含：`con prn aux nul com1~9 lpt1~9` 及 superscript 变体
    `com\u{b9} com\u{b2} com\u{b3} lpt\u{b9} lpt\u{b2} lpt\u{b3}`
  - 此函数将在 T5-2 中供 `delete` 模块复用（DRY）

- [x] **T2-2** 实现 `pub(crate) fn detect_kind(path: &str) -> PathKind`（FSM，字节级匹配）：
  - 使用 `path.as_bytes()` 字节模式，优先级从高到低：
    1. `\\?\UNC\` → `ExtendedUNC`
    2. `\\?\Volume{` → `VolumeGuid`
    3. `\\?\` → `ExtendedLength`
    4. `\\.\` → `DeviceNamespace`
    5. `\\` → `UNC`
    6. `\Device\` 或 `\??\` → `NTNamespace`
    7. `[a-zA-Z]:\\` → `DriveAbsolute`
    8. `[a-zA-Z]:[^\\]` → `DriveRelative`
    9. 其他 → `Relative`

- [x] **T2-3** 实现 `pub(crate) fn is_ads(path: &str, kind: PathKind) -> bool`：
  - 查找 index != 1 处的 `:`；UNC/ExtendedLength 路径跳过盘符位检测

- [x] **T2-4** 实现 `pub(crate) fn check_chars(path: &str) -> Option<PathIssueKind>`：
  - 空字符串 → `Empty`
  - 控制字符 0x00~0x1F（字节扫描，编译器自动向量化）→ `InvalidChar`
  - 非法字符 `< > " | ? *`（字节扫描）→ `InvalidChar`

- [x] **T2-5** 实现 `pub(crate) fn check_component(component: &str) -> Option<PathIssueKind>`：
  - 末尾为空格或 `.` → `TrailingDotSpace`
  - stem（第一个 `.` 前的部分）lowercase 后在 `reserved_names()` 中 → `ReservedName`

- [x] **T2-6** 实现 `pub(crate) fn check_traversal(base: &Path, joined: &Path) -> Option<PathIssueKind>`：
  - `joined.strip_prefix(base).is_err()` → `TraversalDetected`（纯字符串语义，不访问磁盘）

- [x] **T2-7** 实现主入口 `pub(crate) fn check_string(raw: &str, policy: &PathPolicy) -> Option<PathIssueKind>`，严格按以下顺序：
  1. `check_chars`
  2. `detect_kind` 得到 `PathKind`
  3. `is_ads` 检测，若 `!allow_ads` → `AdsNotAllowed`
  4. `PathKind` 策略检查（DeviceNamespace / NTNamespace / VolumeGuid / DriveRelative 固定拒绝；Relative 受 `allow_relative` 控制）
  5. 分隔符统一 `/` → `\`，逐组件调用 `check_component`
  6. 若有 `base`，调用 `check_traversal`

- [x] **T2-8** 单元测试，覆盖：
  - `detect_kind`：每种 `PathKind` 至少一个正例
  - `check_chars`：控制字符、非法字符、空字符串
  - `check_component`：末尾空格/句号、各保留名、带扩展名保留名（`NUL.txt`、`COM1.log`）、superscript 变体
  - `check_traversal`：`..` 穿越、正常子路径
  - `check_string`：`PathPolicy` 各字段开关的组合场景

---

## 阶段 3：WinAPI 封装（`winapi.rs`）

> 所有 unsafe 集中于此文件，对外接口安全。

- [x] **T3-1** 实现 `fn to_wide_with_prefix(path: &Path) -> Vec<u16>`：
  - `OsStrExt::encode_wide()` 转 UTF-16
  - 若路径不以 `\?\` 开头，补前缀：UNC 路径 → `\?\UNC\server...`；其他 → `\?\C:\...`
  - 末尾追加 `\0`
  - 注意：必须在绝对化完成后才加前缀（`\?\` 下 Windows 不解析 `.`/`..`）

- [x] **T3-2** 实现 TLS 缓冲区：
  `thread_local! { static WIDE_BUF: RefCell<Vec<u16>> = RefCell::new(Vec::with_capacity(512)); }`

- [x] **T3-3** 实现内部函数 `fn probe_path(wide: &[u16]) -> Result<u32, PathIssueKind>`：
  - 调用 `GetFileAttributesW(wide.as_ptr())`
  - 返回 `INVALID_FILE_ATTRIBUTES` 时调用 `GetLastError()` 映射：
    2/3→NotFound · 5→AccessDenied · 32→SharingViolation · 53→NetworkPathNotFound · 206→TooLong · 1921→SymlinkLoop · 其他→IoError
  - 成功返回 `Ok(attr)`（完整 u32 属性位，一次调用同时判断存在/目录/重解析点）

- [x] **T3-4** 实现 `pub(crate) fn probe(path: &Path) -> Result<u32, PathIssueKind>`：
  - 使用 TLS 缓冲区，调用 `probe_path`

- [x] **T3-5** 实现 `pub(crate) fn is_reparse_point(attr: u32) -> bool`
- [x] **T3-6** 实现 `pub(crate) fn is_directory(attr: u32) -> bool`

- [x] **T3-7**（增强阶段）`get_full_path`：调用 `GetFullPathNameW`，MVP 用 `cwd.join(rel)` 替代

- [x] **T3-8** 单元测试：`to_wide_with_prefix` 前缀正确、不重复；`probe` 存在/不存在路径；属性位辅助函数

---

## 阶段 4：主入口（`mod.rs`）

- [x] **T4-1** 去重逻辑（`IndexSet`）：
  - normalize key：统一分隔符 `\`、`to_lowercase()`、去尾 `\`
  - 保持输入顺序；统计 `deduped` 计数

- [x] **T4-2** 相对路径绝对化（MVP，不用 `GetFullPathNameW`）：
  - 优先用 `policy.cwd_snapshot`，否则 `std::env::current_dir()`
  - 拼接后重新检测 `PathKind` 应为 `DriveAbsolute`

- [x] **T4-3** `expand_env` 展开（启用时）：
  - `winapi.rs` 中封装 `ExpandEnvironmentStringsW`
  - 展开后重新走字符 + FSM 检测

- [x] **T4-4** 实现 `pub fn validate_paths`，校验顺序：
  1. 去重
  2. `check_string`（字符 → FSM → ADS → 策略 → 组件 → 穿越）
  3. `expand_env` 展开并二次校验
  4. 相对路径绝对化
  5. `must_exist` 时调用 `probe` 获取属性
  6. `allow_reparse` 判断
  7. `safety_check` 时调用 `ensure_safe_target`
  8. 通过 → `ok`；失败 → `issues`

- [x] **T4-5** `mod.rs` re-export 所有公共类型

- [x] **T4-6** 单元测试（`must_exist=false`，无需真实文件系统）：
  - 去重顺序稳定；合法/非法混合分类正确；`allow_relative=false` 时相对路径进 issues

- [x] **T4-7** 集成测试（`tests/` 目录，需真实文件系统）：
  - 存在路径通过；不存在路径返回 `NotFound`；受限目录 `safety_check=true` 返回错误

---

## 阶段 5：DRY 整合（复用 `reserved_names`）

- [x] **T5-1** 在 `src/lib.rs` 中将 `mod path_guard` 注册为无条件模块（与 `mod windows` 同级）
- [x] **T5-2** 修改 `src/commands/delete/filters.rs`：
  - 删除本地 `reserved_names()` 函数（当前 filters.rs:12~46）
  - 改为 `use crate::path_guard::string_check::reserved_names;`
  - 验证编译通过，行为与删除前完全一致

---

## 阶段 6：接入 `acl add`（MVP 首个消费方）

- [x] **T6-1** 修改 `src/commands/acl_cmd/edit.rs` 的 `cmd_add`：
  - 替换路径收集段（当前 edit.rs:43~63）为 `path_guard::validate_paths` 调用
  - 使用 `PathPolicy::for_write()` 作为策略
  - 将 `validate_paths` 返回的 `issues` 通过现有 `CliError` 体系输出
  - 删除 `paths.sort(); paths.dedup();`（edit.rs:64~65，改由 `validate_paths` 内部去重保序完成）
- [x] **T6-2** 验证 `acl add -p <valid_path>` 正常工作
- [x] **T6-3** 验证 `acl add -p <reserved_name>` 返回明确错误
- [x] **T6-4** 验证 `acl add --paths a,b,a` 去重后只处理 a、b 两条
- [x] **T6-5** 验证 `acl add -p "C:\Windows\System32"` 因 `safety_check` 被拒绝

---

## 阶段 7：性能优化（MVP 阶段可顺手完成）

- [x] **T7-1** `winapi.rs`：确认 TLS 缓冲区 `RefCell<Vec<u16>>` 与 rayon worker 线程兼容（无 `Send` 问题）
- [x] **T7-2** `string_check.rs`：`check_chars` 使用字节级扫描（`path.as_bytes().iter()`），确认编译器自动向量化（release 模式检查汇编或 benchmark）
- [x] **T7-3** `string_check.rs`：FSM `detect_kind` 使用字节 slice 模式匹配，不用 `starts_with(&str)`
- [x] **T7-4** `mod.rs`：去重时只对 normalize key 分配，原始路径直接 move 进 `ok`，避免克隆
- [x] **T7-5** `mod.rs`：`PathIssue.detail` 字段使用 `&'static str`，不分配堆内存

---

## 阶段 8：并行管线（增强阶段）

- [x] **T8-1** `parallel.rs`：实现并行阈值分支逻辑：
  - `< 64` 条 → 直接调用串行 `validate_paths`
  - `64~500` 条 → `rayon::par_iter()` 字符串校验 + WinAPI 探测
  - `> 500` 且含 UNC → UNC 路径分流，降并发至 `min(4, num_cpus::get())`
- [x] **T8-2** `parallel.rs`：WinAPI 阶段使用 `crossbeam-channel`（项目已有依赖）构建固定线程数 worker 池，避免 IO 阻塞 Rayon
- [x] **T8-3** `parallel.rs`：`cwd_snapshot` 在进入并行前快照一次，所有 worker 共享同一个 `Arc<PathBuf>`
- [x] **T8-4** 更新 `mod.rs` 入口，按阈值选择串行或并行路径
- [x] **T8-5** 性能基准（`benches/` 或 `tests/` 下手动 benchmark）：
  - 记录运行环境（CPU、磁盘类型）
  - 目标：本地 SSD，1k 路径串行 `p50 < 50ms`，并行 `p50 < 15ms`

---

## 阶段 9：增强校验（增强阶段）

- [x] **T9-1** `winapi.rs`：实现 `get_full_path(path: &Path) -> Result<PathBuf, PathIssueKind>`
  - 调用 `GetFullPathNameW`，替换 T4-2 的 `cwd.join()` 方案
- [x] **T9-2** `winapi.rs`：实现 `open_path_with_policy`（`CreateFileW`）：
  - `allow_reparse=false` 时追加 `FILE_FLAG_OPEN_REPARSE_POINT`
  - 目录需追加 `FILE_FLAG_BACKUP_SEMANTICS`
  - 最小权限：按业务需求设置 `desired_access`
  - 返回 `OwnedHandle`（RAII 自动关闭）
- [x] **T9-3** `winapi.rs`：实现 `ExpandEnvironmentStringsW` 封装（T4-3 的完整版）
- [x] **T9-4** P0 命令接入 `open_path_with_policy`（`delete`、`acl remove/repair`）

---

## 阶段 10：扩展接入（后续命令）

按优先级逐步替换各命令的手动路径处理：

- [x] **T10-1** P0：`delete` 命令接入 `PathPolicy::for_write()`
- [x] **T10-2** P0：`acl remove/owner/repair/copy/restore` 接入
- [x] **T10-3** P0：`redirect --apply/--undo` 接入
- [x] **T10-4** P0：`vault/crypt` 输出路径接入 `PathPolicy::for_output()`
- [x] **T10-5** P1：`bak restore` 接入
- [x] **T10-6** P1：`img` 输出目录、`brn --apply` 接入
- [x] **T10-7** P2：`find`/`cstat`/`diff` 接入 `PathPolicy::for_read()`
- [x] **T10-8** P3：`bookmarks set`/`ctx set` 接入（`must_exist=false`）

---

## 阶段 11：测试与 CI

- [x] **T11-1** 单元测试覆盖率：所有 `PathIssueKind` 变体至少有一个触发用例
- [x] **T11-2** 集成测试（`tests/path_guard_integration.rs`）：
  - `--file` 输入（多行路径文件）
  - `--paths` 逗号列表
  - 权限受限目录的 `AccessDenied` 分类
  - UNC 路径（若测试环境有网络共享）
- [x] **T11-3** Fuzz 测试（`cargo-fuzz`，可选）：针对 `check_string` 的随机输入边界
- [x] **T11-4** CI：Windows runner（`windows-latest`）执行全部单元 + 集成测试
- [x] **T11-5** CI：release 模式编译验证（`opt-level=z` + `strip=symbols`）

---

## 依赖关系图

```
T0（准备）
  └─ T1（类型定义）
       └─ T2（字符串校验）
            └─ T3（WinAPI 封装）
                 └─ T4（主入口）
                      ├─ T5（DRY 整合）
                      ├─ T6（接入 acl add）← MVP 完成标志
                      ├─ T7（性能优化）
                      ├─ T8（并行管线）
                      ├─ T9（增强校验）
                      └─ T10（扩展接入）
                           └─ T11（测试与 CI）
```

---

## MVP 完成标志

以下任务全部完成即视为 MVP 交付：

- [x] T0-1 ~ T0-4
- [x] T1-1 ~ T1-8
- [x] T2-1 ~ T2-8
- [x] T3-1 ~ T3-6, T3-8
- [x] T4-1 ~ T4-7
- [x] T5-1 ~ T5-2
- [x] T6-1 ~ T6-5
- [x] T7-1 ~ T7-5
