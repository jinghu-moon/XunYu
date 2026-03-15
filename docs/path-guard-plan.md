# Windows 路径校验组件方案（Path Guard）

## 背景与目标

当前项目中多个命令需要解析并校验路径输入（`--path`/`--paths`/`--file`），但实现分散且缺少统一策略。为保证性能与一致性，需要一个**通用路径校验组件**，仅针对 Windows 平台优化（`x86_64-pc-windows-msvc`）。

目标：
- 统一解析与校验路径集合，输出结构化结果（可用路径 + 错误清单）。
- 以 WinAPI 为主，绕开 `std::fs` 的高开销，满足批量高性能场景。
- 支持长路径、重解析点、相对路径、路径穿越等关键策略控制。

非目标：
- 不做跨平台路径语义兼容（仅 Windows 目标）。
- 不覆盖业务层“如何处理错误”的策略，仅提供明确输入与结果。

## 参考与对比（主流库 + 官方说明）

本项目仅编译 Windows 目标，`std::path::Path` 即具备 Windows 语义，不引入跨平台路径语义库。只参考其设计思想。

参考来源：
- Windows-rs 官方文档：`windows` 更偏 idiomatic 与安全封装，`windows-sys` 提供 raw bindings 与更快编译。`https://microsoft.github.io/windows-rs/book/rust-getting-started/windows-or-windows-sys.html`
- Windows API：`GetFileAttributesW` 用于文件属性/存在性检查。`https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileattributesw`
- Windows API：`GetFileAttributesExW` 扩展属性查询。`https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileattributesexw`
- Windows API：`GetFullPathNameW` 规范化相对路径。`https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfullpathnamew`
- Windows API：`GetFinalPathNameByHandleW` 获取真实路径。`https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew`
- Windows API：`CreateFileW` 用于原子打开与策略控制。`https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew`
- Windows Native API：`NtQueryAttributesFile`（高级可选）。`https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntqueryattributesfile`
- 文件属性常量（含 `FILE_ATTRIBUTE_REPARSE_POINT`）：`https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants`
- Windows MAX_PATH 及 long path 机制：`https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation`
- Windows 路径与命名空间规则：`https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file`
- Windows API：`ExpandEnvironmentStringsW` 环境变量展开。`https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-expandenvironmentstringsw`
- `dunce::simplified`：将 `\\?\` 前缀尽可能转换为用户友好路径（显示层）。`https://docs.rs/dunce/latest/dunce/fn.simplified.html`

结论：采用“**纯字符串合法性检查 + WinAPI 存在性检查**”的自研方案，保持 `windows-sys` 性能优势，并引入一层安全薄包装（减少散落的 `unsafe`）。

## 设计原则

- KISS：只做路径解析与校验，不与业务耦合。
- DRY：所有命令统一入口，避免重复实现。
- YAGNI：不引入跨平台路径语义，仅保留 Windows 优化。

## 模块边界与职责

组件名称建议：`path_guard`（内部模块）。

职责：
- 解析路径输入（单路径、逗号列表、文件列表）。
- 路径去重（统一大小写与分隔符后去重，保证批量稳定性）。
- 合法性校验（纯字符串，不访问磁盘）。
- 存在性校验（WinAPI）。
- 输出 `ok_paths + issues`，不直接决定是否中断流程。

补充：
- 内置 WinAPI helper，统一错误码映射，避免业务层出现 `unsafe`。

## 目录结构与依赖建议

目录结构（落地到 `src/path_guard/`，与 `docs/path-guard-architecture.md` 对齐）：
```
src/
  path_guard/
    mod.rs
    policy.rs
    types.rs
    parser/
      mod.rs
      fsm.rs
      normalize.rs
      detect.rs
    validate/
      mod.rs
      string_rules.rs
      reserved.rs
      traversal.rs
    winapi/
      mod.rs
      attributes.rs
      open.rs
      canonicalize.rs
      utf16.rs
    parallel/
      mod.rs
      pipeline.rs
    util/
      mod.rs
      dedupe.rs
      long_path.rs
```

依赖与特性：
```
[features]
default = ["fast-bindings"]
fast-bindings = ["windows-sys"]
safe-bindings = ["windows"]
```

## 接口草案

```rust
pub struct PathPolicy {
    pub must_exist: bool,
    pub allow_relative: bool,
    pub expand_env: bool,
    pub allow_reparse: bool,
    pub allow_ads: bool,
    pub allow_device_namespace: bool,
    pub allow_long: bool,
    pub max_len: Option<usize>,
    pub base: Option<PathBuf>,
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
    IoError,
}

pub struct PathIssue {
    pub raw: String,
    pub kind: PathIssueKind,
    pub detail: String,
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

pub fn validate_paths(
    inputs: impl IntoIterator<Item = String>,
    policy: &PathPolicy,
) -> PathValidationResult;

pub fn validate_paths_with_info(
    inputs: impl IntoIterator<Item = String>,
    policy: &PathPolicy,
) -> (Vec<PathInfo>, Vec<PathIssue>);

pub fn open_path_with_policy(
    path: &Path,
    policy: &PathPolicy,
) -> Result<RawHandle, PathIssueKind>;
```

输入与分配优化建议：
- 优先接受 `AsRef<OsStr>`，避免强制 UTF-8 `String` 带来的转换开销与信息丢失。
- `PathIssue.raw` 可用 `Cow<'a, OsStr>` 借用原始输入，降低复制成本。
- `PathIssue.detail` 建议用 `&'static str` 或 `Box<str>`，减少堆分配。
- 支持传入可复用的 UTF-16 Scratch Buffer（`&mut Vec<u16>`），减少 TLS 与重复分配。

性能优先接口草案（可选）：
```rust
pub fn validate_paths<I, P>(
    inputs: I,
    policy: &PathPolicy,
) -> PathValidationResult
where
    I: IntoIterator<Item = P>,
    P: AsRef<OsStr>;
```

## Windows 路径类型与策略（基于 taxonomy）

路径类型与默认策略：
- Drive absolute：`C:\...`，默认允许。
- Relative：`.\` / `..\` / `dir\file`，受 `allow_relative` 控制。
- Drive relative：`C:dir\file`，默认拒绝（`DriveRelativeNotAllowed`）。
- UNC：`\\server\share\...`，默认允许但限并发。
- Extended length：`\\?\C:\...`、`\\?\UNC\...`，`allow_long=true` 时允许。
- Device namespace：`\\.\`，默认拒绝（`DeviceNamespaceNotAllowed`）。
- NT namespace：`\Device\...`、`\??\...`，默认拒绝（`NtNamespaceNotAllowed`）。
- Volume GUID：`\\?\Volume{GUID}\...`，默认拒绝（`VolumeGuidNotAllowed`）。
- ADS：`file:stream`，默认拒绝（`AdsNotAllowed`）。
- 环境变量路径：`%TEMP%\x.log`，可选扩展（`ExpandEnvironmentStringsW`，受 `expand_env` 控制）。

路径分布（工程统计）：
- 本地路径占比最高（70%+），相对路径次之，UNC/长路径极低频，设备与 ADS 基本极少。

## 路径解析 FSM（零 IO）

目标：零分配识别 `PathKind`，提升早期过滤速度。

检测规则（前缀 O(1) 扫描）：
- `\\?\UNC\` → `ExtendedUNC`
- `\\?\Volume{` → `VolumeGuid`
- `\\?\` → `ExtendedLength`
- `\\.\` → `DeviceNamespace`
- `\\` → `UNC`
- `\Device\` 或 `\??\` → `NTNamespace`
- `X:\` → `DriveAbsolute`
- `X:foo` → `DriveRelative`
- 其他 → `Relative`

ADS 检测：
- 查找 `:`，若不是盘符位置（index=1），则判定为 ADS。

## 项目功能与路径需求矩阵（定制化依据）

强路径依赖（必须校验）：
| 功能 | 典型命令 | 路径类型 | 必须存在 | 备注 |
| --- | --- | --- | --- | --- |
| 删除/清理 | `delete`、`rm` | 文件/目录 | 是 | 高风险，默认拒绝重解析点，强制 `open_path_with_policy`，并结合 `windows::safety::ensure_safe_target` 保护系统目录 |
| ACL 读写 | `acl view/add/remove/repair/owner` | 文件/目录 | 是 | 写操作高风险，默认拒绝重解析点，必要时增加系统目录防护 |
| 锁与移动 | `lock who`、`mv`、`ren` | src: 文件/目录，dst: 文件/目录 | src 必须，dst 可新建 | dst 仅做合法性校验 |
| Redirect | `redirect` | 目录/计划文件 | source 必须 | `--plan` 输出可新建，`--apply` 必须存在 |
| Vault/Crypt | `vault`、`encrypt/decrypt` | 文件 | input 必须 | output 可新建；keyfile/identity 必须存在 |
| Diff | `diff` | 文件 | 是 | 仅文件，非目录 |

中等路径依赖（读多写少）：
| 功能 | 典型命令 | 路径类型 | 必须存在 | 备注 |
| --- | --- | --- | --- | --- |
| 备份 | `bak` | 目录/相对文件 | dir 必须 | `--file` 为相对路径，需基于工作目录校验 |
| 图像处理 | `img` | 输入文件/目录、输出目录 | input 必须 | output 可创建 |
| 视频处理 | `video` | 输入/输出文件 | input 必须 | `ffmpeg/ffprobe` 路径如指定需存在 |
| 批量重命名 | `brn` | 目录 | 是 | `--apply` 时进入写路径策略 |

弱路径依赖（允许不存在或纯配置）：
| 功能 | 典型命令 | 路径类型 | 必须存在 | 备注 |
| --- | --- | --- | --- | --- |
| Find/Tree/Cstat | `find`、`tree`、`cstat` | 目录 | 通常是 | 读操作可允许重解析点 |
| Bookmarks/Ctx | `bookmark set`、`ctx set` | 目录 | 可选 | 允许保存未存在路径，但 `open` 应告警 |
| Env 导入导出 | `env import/export` | 文件 | input 必须 | output 可新建 |
| Alias | `alias import/export`、`alias app add` | 文件 | input 必须 | output 可新建 |

非路径功能（无需 path-guard）：
- `proxy`、`ports/kill/ps/pkill`、`config`、`completion`、`desktop`（除 run/exe 路径外）

路径来源与输入形态：
- 直接路径：CLI positional 或 option（多数命令）。
- 路径列表：`acl add/batch` 支持 `--file` 与 `--paths`（逗号列表）。
- 计划文件：`redirect --plan/--apply` 等 JSON 文件输入。

## 合法性校验规则（零 IO）

- 禁止字符：`< > " / \ | ? *` + 控制字符 0x00~0x1F。
- 保留设备名：`CON/PRN/AUX/NUL/COM1~9/LPT1~9`（含扩展名），并覆盖 superscript 版本（`COM¹/COM²/COM³`、`LPT¹/LPT²/LPT³`），与 `delete` 模块的 `reserved_names()` 保持一致。
- 文件名末尾禁止空格或句号。
- 分隔符统一为 `\`（除 `\\?\` 前缀外）。
- `:` 仅允许出现在盘符 `C:` 的第 2 位；其余 `:` 视为 ADS，除非 `allow_ads=true`。
- 设备命名空间（`\\.\`、`\\?\Volume{GUID}\`）默认拒绝，除非 `allow_device_namespace=true`。
- 驱动器相对路径（`C:dir\file`）默认拒绝，避免“当前盘符目录”语义不确定。
- 去重规则：先做“规范化字符串”（统一分隔符、去尾斜杠、大小写折叠），再进行集合去重。
- 路径穿越（有 `base` 时）：优先使用 `std::path` 语义拼接与 `strip_prefix` 校验，避免手写解析在 UNC/设备路径下出错。

示例（无 IO）：
```rust
if let Some(base) = &policy.base {
    let joined = base.join(&normalized_path);
    if joined.strip_prefix(base).is_err() {
        return issue(PathIssueKind::TraversalDetected);
    }
}
```

说明：该阶段只做字符串检查，速度最快，避免不必要的系统调用。

## 推荐检测顺序（结合 FSM）

1. trim + 分隔符统一（`/` → `\`）。
2. 非法字符扫描。
3. `PathKind` 检测（FSM）。
4. ADS 检测与保留设备名检测。
5. `allow_relative` 判断（含 `DriveRelative` 特判）。
6. `expand_env`（如启用）并再次校验输出。
7. `GetFullPathNameW` 规范化（仅相对路径）。
8. 长路径前缀补齐（`\\?\` / `\\?\UNC\`）。
9. `GetFileAttributesW/ExW` 存在性探测。
10. `FILE_ATTRIBUTE_REPARSE_POINT` 检测。
11. 必要时 `open_path_with_policy` 原子打开。

## 正规化与真实路径获取

- 当 `allow_relative=true` 时，使用 `GetFullPathNameW` 进行词法正规化，解析 `.`/`..` 并转为绝对路径。
- `GetFullPathNameW` 不保证路径存在，仅做字符串级规范化。
- 需要真实目标路径时，使用 `CreateFileW` 打开后调用 `GetFinalPathNameByHandleW` 获取最终路径。
- 当 `allow_reparse=false` 时，打开句柄需带 `FILE_FLAG_OPEN_REPARSE_POINT`，避免自动跟随重解析点。

长路径前缀注意事项：
- 必须在完成绝对化与正规化之后再添加 `\\?\` 前缀。
- 一旦使用 `\\?\`，Windows 不再解析 `.` 和 `..`，因此顺序不可颠倒。

## 长路径策略

默认路径长度限制 `MAX_PATH=260` 仍存在，但可通过前缀规避：

- `allow_long=false`：超限直接报错。
- `allow_long=true`：统一补 `\\?\` 前缀。
  - 盘符路径：`\\?\C:\...`
  - UNC 路径：`\\?\UNC\server\share\...`

可选增强：为可执行文件嵌入 `longPathAware=true` 的 manifest，结合系统 `LongPathsEnabled=1`。

显示层建议：借鉴 `dunce::simplified` 的逻辑，把 `\\?\` 前缀转回用户可读的 `C:\...`，仅用于输出与日志。

## 重解析点策略（Reparse Point）

使用 `GetFileAttributesW` 判断 `FILE_ATTRIBUTE_REPARSE_POINT`：

- `allow_reparse=false`：直接拒绝，避免路径跳转与安全风险。
- `allow_reparse=true`：记录提示但允许通过。

## 存在性检查（高性能）

使用 WinAPI：

- `GetFileAttributesW`：最小开销存在性检查。
- `GetFileAttributesExW`：需要区分文件/目录/重解析点时使用。
- 失败时 `GetLastError` 区分：不存在、拒绝访问、路径过长。

可选高级优化：
- 当业务需要文件大小/时间戳时，优先用 `GetFileAttributesExW` 一次获取，避免二次系统调用。
- 极致性能场景可评估 `NtQueryAttributesFile`（`ntdll.dll`），但要承担兼容性与维护成本。

注意：存在性检查永远是 racy，只做“快照判定”。

建议封装（避免到处 `unsafe`）：
```rust
pub fn exists_fast(path: &str) -> Result<bool, PathIssueKind> {
    let mut wide = path.encode_utf16().chain(std::iter::once(0)).collect::<Vec<_>>();
    ensure_long_path_prefix(&mut wide);
    let attr = unsafe { GetFileAttributesW(wide.as_ptr()) };
    if attr == INVALID_FILE_ATTRIBUTES {
        let code = unsafe { GetLastError() };
        return match code {
            2 | 3 => Err(PathIssueKind::NotFound),
            5 => Err(PathIssueKind::AccessDenied),
            206 => Err(PathIssueKind::TooLong),
            _ => Err(PathIssueKind::IoError),
        };
    }
    Ok(true)
}
```

## 并行与性能

- 批量路径校验采用 `rayon::par_iter()` 并行，限制线程数（建议 4~8）。
- UTF-16 缓冲区复用，减少每条路径的分配成本。
- 先做字符串合法性过滤，再进行 WinAPI 调用。

可选优化：
- `AHashSet` 作为去重容器，减少哈希开销。
- `thread_local` 复用 UTF-16 缓冲区，进一步减少分配。
- 两阶段并行：先高并发做字符串校验，再限并发做 WinAPI 调用。
- UNC 路径（`\\server\share\...`）单独降并发，避免远端 I/O 反噬吞吐。

两阶段并行建议：
- 阶段 A：字符串校验使用高并发（`num_cpus()`），仅 CPU 密集。
- 阶段 B：WinAPI 校验使用受控线程池（例如 `min(8, num_cpus()*2)`），IO 密集。
- UNC 分流：检测 `\\server\share` 前缀后降低阶段 B 并发。

线程池模型建议：
- 阶段 A 使用 Rayon 处理纯 CPU 任务。
- 阶段 B 使用独立阻塞线程池，避免 Rayon 线程被 I/O 阻塞拖慢。

## 错误模型与输出

组件只输出结构化错误，不直接打印或中止：

- `issues` 供命令层决定是否中断或跳过。
- 使用 `validate_paths_with_info` 时返回 `PathInfo`，包含规范化结果与探测信息。
- 对应 CLI 输出：按路径汇总错误原因，便于批量场景定位。

显示层建议：
- `ok` 路径输出前可做 `simplify_if_possible`，避免暴露 `\\?\` 前缀。

CLI 使用建议：
- 批量输入优先调用 `validate_paths`，集中输出问题清单。
- 涉及写入/删除等高风险操作时，优先使用 `open_path_with_policy` 获取句柄后再操作。

## TOCTTOU 与安全打开

存在性检查与后续操作存在竞态（TOCTTOU）。建议提供原子打开接口：

- `open_path_with_policy` 使用 `CreateFileW` 按策略打开。
- 对目录需追加 `FILE_FLAG_BACKUP_SEMANTICS`。
- 允许业务层基于句柄继续读写，避免二次路径解析。

实现要点：
- `allow_reparse=false` 时追加 `FILE_FLAG_OPEN_REPARSE_POINT`。
- 最小权限策略：按业务需求设置 `desired_access`，并保留 `FILE_SHARE_READ|WRITE|DELETE`。
- 错误映射统一走 `GetLastError`，避免 CLI 层理解 Win32 码。

## 接入计划（从底层到消费层）

阶段 1：
- 接入 `acl add` 与 `acl batch`。
- 统一处理 `--path/--paths/--file` 输入。

阶段 2：
- 接入 delete/redirect/copy 等依赖路径输入的命令。

阶段 3：
- 可选增加 manifest 支持，统一长路径策略。

## 定制化接入优先级（按功能风险）

P0（高风险写入/删除）：
- `delete`、`rm`、`acl` 写操作（add/remove/owner/repair/copy/restore）、`protect`、`mv/ren`、`redirect --apply/--undo`、`vault/crypt` 输出路径。

P1（中风险写入）：
- `bak restore`、`img` 输出目录、`video` 输出文件、`brn --apply`、`alias export`、`env export`。

P2（只读/分析）：
- `find`、`tree`、`cstat`、`diff`、`vault/crypt` 只读路径、`bak list`、`redirect --plan`。

P3（配置与弱约束）：
- `bookmarks set`、`ctx set`、`env path-dedup`、`desktop run` 等可允许不存在的路径。

## 落地计划（分阶段）

MVP（2~4 天）：
- 实现 `validate_paths` 与 `validate_paths_with_info`。
- 字符串校验 + `GetFullPathNameW` 规范化，不引入 `GetFinalPathNameByHandleW`。
- 单元测试覆盖非法字符、保留名、末尾空格/句号、ADS、命名空间。
- 小型基准：1k/5k 本地路径，记录 `p50/p95`。

增强（3~5 天）：
- 两阶段并行与受控 WinAPI 并发。
- 实现 `open_path_with_policy`。
- 引入 `AHashSet`、`thread_local` UTF-16 缓冲与 `SmallVec` 优化。

稳定与扩展（后续）：
- `GetFinalPathNameByHandleW` 可选接入。
- fuzz（`cargo-fuzz`）与 UNC/SMB 压测。
- CI Windows runner 与基线报告。

## 测试计划

单元测试：
- 合法性：非法字符、保留名、末尾空格/句号。
- 长路径：`allow_long` 开关行为。
- 重解析点：无权限创建时自动跳过。
- ADS：包含 `:` 的路径在 `allow_ads` 不同设置下的表现。
- 命名空间：`\\.\`、`\\?\Volume{}` 在 `allow_device_namespace` 下的行为。

集成测试：
- `--file/--paths` 混合输入的错误与成功路径输出一致。
- 批量大输入下的耗时与错误统计稳定。
- 权限：对受限目录返回 `AccessDenied` 的分类准确。
- UNC/网络路径：低并发策略下的稳定性验证。

性能测试：
- 1k/5k/10k 路径批量校验耗时基线，记录 `p50/p95`。
- Fuzz：针对合法性规则的随机输入覆盖边界组合。

基准建议与验收参考：
- 记录运行环境（CPU、磁盘类型、是否网络盘）。
- 目标参考：本地 SSD 下，1k 路径校验 `p50 < 100ms`，`p95 < 350ms`。

## 风险与约束

- Windows 环境差异：长路径支持与组策略可能导致行为差异。
- 重解析点权限：创建 symlink/junction 可能需要管理员或开发者模式。
- 误判风险：路径存在性与权限检测可能返回 AccessDenied，需要提示而非误报不存在。

## CI 与交付建议

CI：
- Windows runner（`windows-latest`）执行单元与集成测试。
- 可选 feature matrix：`fast-bindings` 与 `safe-bindings` 均需编译通过。

交付物：
- `path_guard` 模块对外 API 文档与最小使用示例。
