# XunYu 系统监控与内存清理子系统设计方案（修订版）

> 基于 WinMemoryCleaner (C#)、memreduct (C)、memory-monitor (Rust)、bottom (Rust) 四个参考项目的源码交叉验证。
> 所有技术参数均标注来源文件与行号。

---

## 1. 综述

在 XunYu 中实现一个**高性能、Windows 深度优化**的性能治理模块，包含两个子系统：

- **sys_monitor**：CPU / 内存实时采集，Dashboard WebSocket 推送
- **mem_cleaner**：基于 `NtSetSystemInformation` 的内核级内存清理

### 1.1 核心参考贡献

| 参考项目 | 核心贡献 | 具体应用 |
|:---|:---|:---|
| WinMemoryCleaner | 8 区域独立 bitmask + OS 版本门控 + 执行顺序 | 清理区域定义与权限模型 |
| memreduct | `NtSetSystemInformation` 四个信息类 + 自动清理触发 | 底层 API 调用参数与触发策略 |
| memory-monitor | Rust `windows-sys` FFI + 三阶段清理流水线 | `cleaner.rs` 模块架构与 FFI 绑定策略 |
| bottom | `mpsc::channel` + `Box<Data>` 所有权转移 + PDH swap 采集 | 采集线程数据流架构 |

---

## 2. 系统监控设计

### 2.1 指标定义

| 指标 | 采集 API | 输出字段 |
|:---|:---|:---|
| CPU 使用率 | `GetSystemTimes` 差分计算 | `usage_percent: f64` |
| 物理内存 | `GlobalMemoryStatusEx` | `total / used / free / percent` |
| 系统缓存 | `GetSystemFileCacheSize` 或 PDH | `cache_size: u64` |
| 分页文件 | PDH `\\Paging File(_Total)\\% Usage` | `swap_total / swap_used / swap_percent` |

### 2.2 CPU 采集算法

基于 Win32 `GetSystemTimes` API 的差分计算（非 memreduct，因其不含 CPU 监控）：

```
T1: 记录 Idle1, Kernel1, User1
T2: 记录 Idle2, Kernel2, User2
Total = (Kernel2 - Kernel1) + (User2 - User1)
Usage = (Total - (Idle2 - Idle1)) / Total * 100.0
```

注意：`GetSystemTimes` 返回的 Kernel 时间包含 Idle 时间，计算时需减去。

### 2.3 内存采集

物理内存直接调用 `GlobalMemoryStatusEx`（`WinMemoryCleaner/src/Interop/NativeMethods.cs:63`）。

分页文件（Swap）使用 PDH API，参照 bottom 的 Windows 实现（`bottom-main/src/collection/memory/windows.rs`）：

1. `PdhOpenQueryW` 打开查询句柄
2. `PdhAddEnglishCounterW` 添加 `\\Paging File(_Total)\\% Usage` 计数器
3. `PdhCollectQueryData` + `PdhGetFormattedCounterValue` 采集
4. 用百分比乘以 `total_swap` 得到已用字节

```rust
// PDH 绑定（windows-sys 已有 Win32_System_Performance，若无则手动绑定）
use windows_sys::Win32::System::Performance::{
    PdhOpenQueryW, PdhAddEnglishCounterW, PdhCollectQueryData,
    PdhGetFormattedCounterValue, PDH_FMT_DOUBLE, PDH_FMT_COUNTERVALUE,
};

struct SwapCollector {
    query: isize,           // PDH_HQUERY
    counter: isize,         // PDH_HCOUNTER
    total_swap: u64,        // GlobalMemoryStatusEx.ullTotalPageFile - ullAvailPhys
}

impl SwapCollector {
    fn new() -> Result<Self, XunError> {
        let mut query = 0isize;
        unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut query)?; }

        let counter_path: Vec<u16> = "\\Paging File(_Total)\\% Usage\0".encode_utf16().collect();
        let mut counter = 0isize;
        unsafe { PdhAddEnglishCounterW(query, counter_path.as_ptr(), 0, &mut counter)?; }

        // total_swap = total_pagefile - available_physical
        let mem = get_memory_status_ex();
        let total_swap = mem.ull_total_page_file.saturating_sub(mem.ull_avail_phys);

        Ok(Self { query, counter, total_swap })
    }

    fn sample(&self) -> Result<(u64, u64), XunError> {
        unsafe { PdhCollectQueryData(self.query)?; }
        let mut value = PDH_FMT_COUNTERVALUE { Anonymous: Default::default() };
        unsafe { PdhGetFormattedCounterValue(self.counter, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut value)?; }
        let percent = unsafe { value.Anonymous.double_value };
        let used = (self.total_swap as f64 * percent / 100.0) as u64;
        Ok((self.total_swap, used))
    }
}
```

总 swap 量从 `GlobalMemoryStatusEx.ullTotalPageFile - ullAvailPhys` 获取（bottom 同法），PDH 仅采集百分比。

### 2.4 数据流架构

参照 bottom 的 `mpsc::channel` + `Box<Data>` 所有权转移模式（`lib.rs:282-285`），**不使用** ArcSwap：

```
┌─────────────────┐     mpsc::channel      ┌──────────────────┐
│  采集线程        │ ── Box<SystemSnapshot) ──→│  Dashboard WS    │
│  (std::thread)   │                          │  handler         │
│  每 1s 采集一次   │                          │                  │
│  CancellationToken 用于优雅退出             │  broadcast::tx   │
└─────────────────┘                          └────────┬─────────┘
                                                      │
                                            broadcast::channel
                                                      │
                                              ┌───────┴───────┐
                                              │  所有 WS 客户端  │
                                              └───────────────┘
```

- 采集线程为纯 `std::thread::spawn`，不依赖 tokio
- 数据通过 `Box` move 语义传递，零锁争用
- Dashboard WS handler 接收后通过 `broadcast::channel` 推送至所有前端客户端
- 关闭信号使用 `CancellationToken`（`Mutex<bool>` + `Condvar`），支持中断式 sleep

### 2.5 不做数据平滑

bottom 不对 CPU 做 EMA 或平滑处理——sysinfo crate 内部处理 delta，bottom 直接读取百分比。XunYu 同理，`GetSystemTimes` 差分结果直接输出，前端可自行实现平滑。

---

## 3. 内存清理设计

### 3.1 清理区域定义

采用 8 区域独立 bitmask 模型，与 WinMemoryCleaner（`Enums.cs:47-59`）和 memreduct（`main.h:47-55`）一致：

```rust
bitflags::bitflags! {
    pub struct CleanAreas: u32 {
        const WORKING_SET             = 0x01;  // 进程工作集
        const SYSTEM_FILE_CACHE       = 0x02;  // 系统文件缓存
        const STANDBY_LOW_PRIORITY    = 0x04;  // Standby List（低优先级）
        const STANDBY_LIST            = 0x08;  // Standby List
        const MODIFIED_LIST           = 0x10;  // 修改后页面列表
        const COMBINE_MEMORY          = 0x20;  // 内存合并（Win10+）
        const REGISTRY_CACHE          = 0x40;  // 注册表缓存（Win8.1+）
        const MODIFIED_FILE_CACHE     = 0x80;  // 修改后文件缓存（卷刷新）
    }
}
```

预设组合：

```rust
// 安全清理：低优先级 Standby + 文件缓存 + 注册表
const LEVEL_SAFE: CleanAreas = CleanAreas::STANDBY_LOW_PRIORITY
    .union(CleanAreas::SYSTEM_FILE_CACHE)
    .union(CleanAreas::REGISTRY_CACHE)
    .union(CleanAreas::MODIFIED_FILE_CACHE);

// 标准清理：在安全基础上加入 WorkingSet 和 ModifiedList
const LEVEL_STANDARD: CleanAreas = LEVEL_SAFE
    .union(CleanAreas::WORKING_SET)
    .union(CleanAreas::MODIFIED_LIST);

// 深度清理：全部区域
const LEVEL_DEEP: CleanAreas = LEVEL_STANDARD
    .union(CleanAreas::STANDBY_LIST)
    .union(CleanAreas::COMBINE_MEMORY);
```

注意：`STANDBY_LIST` 和 `STANDBY_LOW_PRIORITY` 互斥——同时启用时优先使用 `STANDBY_LIST`。这与 WinMemoryCleaner 的 UI 互斥逻辑一致（`MainViewModel.cs:654-665`）。

### 3.2 每个区域的 Windows API 调用

以下所有常量均经过 WinMemoryCleaner 和 memreduct 交叉验证：

| 区域 | NtSetSystemInformation 信息类 | 参数 | 所需权限 | OS 要求 |
|:---|:---|:---|:---|:---|
| WorkingSet | `SystemMemoryListInformation` (80) | `MemoryEmptyWorkingSets` (2) | `SE_PROF_SINGLE_PROCESS` | XP+ |
| SystemFileCache | `SystemFileCacheInformation` (21) | `SYSTEM_FILECACHE_INFORMATION { min=-1, max=-1 }` + `SetSystemFileCacheSize(-1,-1,0)` | `SE_INCREASE_QUOTA` | XP+ |
| ModifiedList | `SystemMemoryListInformation` (80) | `MemoryFlushModifiedList` (3) | `SE_PROF_SINGLE_PROCESS` | Vista+ |
| StandbyList | `SystemMemoryListInformation` (80) | `MemoryPurgeStandbyList` (4) | `SE_PROF_SINGLE_PROCESS` | Vista+ |
| StandbyLowPri | `SystemMemoryListInformation` (80) | `MemoryPurgeLowPriorityStandbyList` (5) | `SE_PROF_SINGLE_PROCESS` | Vista+ |
| CombineMemory | `SystemCombinePhysicalMemoryInformation` (130) | 零初始化 `MEMORY_COMBINE_INFORMATION_EX` | `SE_PROF_SINGLE_PROCESS` | Win10+ |
| RegistryCache | `SystemRegistryReconciliationInformation` (155) | NULL, 0 | 无需额外权限 | Win8.1+ |
| ModifiedFileCache | N/A（卷 I/O 操作） | `FlushFileBuffers` + `DeviceIoControl` | 无需额外权限 | XP+ |

来源：
- WinMemoryCleaner `ComputerService.cs:459-789`
- memreduct `main.c:379-460`
- WinMemoryCleaner `Constants.cs:163-176`

### 3.3 NtSetSystemInformation FFI 绑定

参照 memory-monitor（`cleaner.rs:61-73`），`windows-sys` 不暴露 `NtSetSystemInformation` 和 `EmptyWorkingSet`，需手动绑定：

```rust
use windows_sys::Win32::Foundation::HANDLE;

#[link(name = "ntdll")]
unsafe extern "system" {
    /// 返回 NTSTATUS (i32)：>= 0 表示成功
    fn NtSetSystemInformation(
        system_information_class: u32,
        system_information: *mut core::ffi::c_void,
        system_information_length: u32,
    ) -> i32;
}

#[link(name = "psapi")]
unsafe extern "system" {
    fn EmptyWorkingSet(hProcess: HANDLE) -> i32;
}
```

结构体定义（参照 WinMemoryCleaner `Structs.cs:18-91`）：

```rust
#[repr(C)]
#[derive(Default)]
struct SystemFileCacheInformation {
    current_size: usize,
    peak_size: usize,
    page_fault_count: u32,
    minimum_working_set: usize,  // 设为 usize::MAX
    maximum_working_set: usize,  // 设为 usize::MAX
    // 后续字段省略，仅前两个 usize + u32 + 两个 usize 用于 flush
}

#[repr(C)]
#[derive(Default)]
struct MemoryCombineInformationEx {
    handle: usize,
    pages_combined: usize,
    flags: i64,
}
```

`EnumProcesses`、`OpenProcess`、`CloseHandle` 等可直接使用 `windows-sys`（Cargo.toml:155,159 已包含 `Win32_System_ProcessStatus`、`Win32_System_Threading`、`Win32_System_Memory`）。

**SystemFileCache 的双调用模式**：WinMemoryCleaner 对文件缓存区域执行两步操作（`ComputerService.cs:663-669`）——先调用 `NtSetSystemInformation(21, ...)` 刷新缓存内容，再调用 `SetSystemFileCacheSize(usize::MAX, usize::MAX, 0)` 重置缓存大小限制。两步缺一不可：

```rust
fn flush_system_file_cache() -> Result<(), XunError> {
    // Step 1: NtSetSystemInformation 刷新
    let mut sfci = SystemFileCacheInformation {
        minimum_working_set: usize::MAX,
        maximum_working_set: usize::MAX,
        ..Default::default()
    };
    let status = unsafe {
        NtSetSystemInformation(21, &mut sfci as *mut _ as *mut _, size_of_val(&sfci) as u32)
    };
    if status < 0 {
        return Err(XunError::internal(format!("NtSetSystemInformation(21) failed: 0x{status:08X}")));
    }

    // Step 2: SetSystemFileCacheSize 重置限制
    let ok = unsafe { SetSystemFileCacheSize(usize::MAX, usize::MAX, 0) };
    if ok == 0 {
        return Err(XunError::internal("SetSystemFileCacheSize failed"));
    }
    Ok(())
}
```

`SetSystemFileCacheSize` 也通过 `#[link(name = "kernel32")]` 绑定，`windows-sys` 已有 `Win32_System_Memory` feature。

### 3.4 权限提升

参照 memreduct（`main.c:1826-1846`），在初始化时一次性提升两个权限：

```rust
fn elevate_privileges() -> Result<(), XunError> {
    // 1. SE_PROF_SINGLE_PROCESS_PRIVILEGE
    //    用于: WorkingSet, ModifiedList, StandbyList, CombineMemory
    set_privilege(SE_PROF_SINGLE_PROCESS_PRIVILEGE)?;

    // 2. SE_INCREASE_QUOTA_PRIVILEGE
    //    用于: SystemFileCache
    set_privilege(SE_INCREASE_QUOTA_PRIVILEGE)?;

    Ok(())
}
```

实现方式：`OpenProcessToken` → `LookupPrivilegeValueW` → `AdjustTokenPrivileges`。

参照 memory-monitor 的容错策略：权限提升失败不 panic，而是降级到可执行的清理区域并提示用户以管理员身份运行。

### 3.5 清理执行流水线

参照 WinMemoryCleaner 的固定执行顺序（`ComputerService.cs:147-454`）和 memory-monitor 的三阶段流水线（`cleaner.rs:145-161`）：

```
1. 采集 before 快照（memreduct:376）
2. 按以下顺序执行用户选定的区域：
   a. WorkingSet         → NtSetSystemInformation(80, 2)
   b. SystemFileCache    → NtSetSystemInformation(21, ...) + SetSystemFileCacheSize
   c. ModifiedList       → NtSetSystemInformation(80, 3)
   d. StandbyList        → NtSetSystemInformation(80, 4)
   e. StandbyLowPri      → NtSetSystemInformation(80, 5)
   f. CombineMemory      → NtSetSystemInformation(130, ...)
   g. RegistryCache      → NtSetSystemInformation(155, NULL, 0)
   h. ModifiedFileCache  → 枚举卷 → FlushFileBuffers + DeviceIoControl
3. 采集 after 快照
4. 计算释放量: freed = before - after（memreduct:465-474）
5. 释放自身工作集: EmptyWorkingSet(self)（WinMemoryCleaner:614-620）
```

每个区域操作独立 try/catch，单个区域失败不阻塞后续区域（WinMemoryCleaner:161-414）。

### 3.6 Volume Cache Flush 实现

ModifiedFileCache 需要枚举卷并刷新。两种实现路径（可共存）：

**路径 A：Mount Manager 枚举（memreduct 方式，`main.c:205-284`）**

1. `NtCreateFile` 打开 `\\.\MountPointManager` 设备
2. `DeviceIoControl(IOCTL_MOUNTMGR_QUERY_POINTS)` 枚举卷挂载点
3. 对每个 `\\?\Volume{GUID}\` 格式的卷名：
   - `CreateFileW` 打开卷（`FILE_WRITE_DATA | SYNCHRONIZE`）
   - `FlushFileBuffers` 刷新脏页到磁盘
   - `CloseHandle`
4. Win8+ 额外调用 `DeviceIoControl(FSCTL_DISCARD_VOLUME_CACHE, 0x00090054)`

**路径 B：DriveInfo 枚举（WinMemoryCleaner 方式，`ComputerService.cs:497-574`）**

1. `GetLogicalDrives` 枚举盘符
2. 对每个盘符 `CreateFileW` 打开 `\\.\X:` 卷句柄
3. 调用 `FlushFileBuffers`
4. Win7+ 调用 `DeviceIoControl(FSCTL_RESET_WRITE_ORDER, 0x000900F8)`
5. Win8+ 调用 `DeviceIoControl(FSCTL_DISCARD_VOLUME_CACHE, 0x00090054)`

**推荐路径 B**——更简单，盘符枚举比 Mount Manager 枚举更直观，且覆盖相同卷集合。

```rust
fn flush_volume_cache() -> Result<u32, XunError> {
    let drives = get_logical_drives(); // GetLogicalDrives()
    let mut flushed = 0u32;
    for letter in drives {
        let path = format!(r"\\.\\{}:", letter);
        let handle = open_volume(&path)?;  // CreateFileW
        FlushFileBuffers(handle);          // windows-sys 直接可用
        if os_version() >= Win8 {
            device_ioctl(handle, FSCTL_DISCARD_VOLUME_CACHE, &[])?;
        }
        CloseHandle(handle);
        flushed += 1;
    }
    Ok(flushed)
}
```

### 3.7 自动清理触发策略

参照 memreduct 的双触发模型（`main.c:703-721`）：

| 触发类型 | 条件 | 默认值 | 来源 |
|:---|:---|:---|:---|
| 阈值触发 | 物理内存使用率 >= 阈值 | 90% | memreduct:705-709 |
| 定时触发 | 距上次清理 >= 间隔 | 30 分钟 | memreduct:711-717 |

安全策略：自动触发时**排除** `STANDBY_LIST` 和 `MODIFIED_LIST`，除非用户显式配置允许（memreduct:337-339）：

```rust
fn auto_clean_mask(user_mask: CleanAreas, allow_standby: bool) -> CleanAreas {
    if allow_standby {
        user_mask
    } else {
        user_mask - CleanAreas::STANDBY_LIST - CleanAreas::MODIFIED_LIST
    }
}
```

自动清理配置结构体：

```rust
pub struct AutoCleanConfig {
    pub enabled: bool,
    pub threshold_percent: u8,       // 默认 90
    pub interval_minutes: u32,       // 默认 30
    pub allow_standby_cleanup: bool, // 默认 false
    pub areas: CleanAreas,           // 默认 LEVEL_STANDARD
}
```

采集线程中的自动清理检查（参照 memreduct `main.c:703-721`）：

```rust
fn check_auto_clean(stats: &MemorySnapshot, config: &AutoCleanConfig, last_clean: Instant) -> Option<CleanAreas> {
    if !config.enabled {
        return None;
    }
    // 阈值触发
    if stats.percent >= config.threshold_percent as f64 {
        return Some(auto_clean_mask(config.areas, config.allow_standby_cleanup));
    }
    // 定时触发（仅在阈值未触发时检查）
    if last_clean.elapsed() >= Duration::from_secs(config.interval_minutes as u64 * 60) {
        return Some(auto_clean_mask(config.areas, config.allow_standby_cleanup));
    }
    None
}
```

### 3.8 OS 版本门控

每个区域需要检查 OS 版本，不支持时跳过并记录警告：

```rust
fn available_areas(os_version: OsVersion) -> CleanAreas {
    let mut areas = CleanAreas::WORKING_SET
        | CleanAreas::SYSTEM_FILE_CACHE
        | CleanAreas::MODIFIED_FILE_CACHE;

    if os_version >= Vista {
        areas |= CleanAreas::MODIFIED_LIST
            | CleanAreas::STANDBY_LIST
            | CleanAreas::STANDBY_LOW_PRIORITY;
    }
    if os_version >= Win8 {
        areas |= CleanAreas::COMBINE_MEMORY;
    }
    if os_version >= Win8_1 {
        areas |= CleanAreas::REGISTRY_CACHE;
    }
    areas
}
```

参照 WinMemoryCleaner `Model/OperatingSystem.cs` 和 memreduct `_r_sys_isosversiongreaterorequal` 的门控逻辑。

### 3.9 进程保护

参照 WinMemoryCleaner 的进程排除机制（`ComputerService.cs:724`）：

- 可配置的进程排除列表（名称匹配，大小写不敏感）
- 自身进程自动排除
- 对于系统级 `MemoryEmptyWorkingSets=2` 命令，内核自动跳受保护进程
- 逐进程 `EmptyWorkingSet` 路径中，`ErrorAccessDenied` 和 `InvalidOperationException` 静默忽略（`ComputerService.cs:736,741`）

---

## 4. 与 XunYu 架构的集成

### 4.1 模块结构

```
src/sys_monitor/
├── mod.rs              // 模块入口，CleanAreas bitflags 定义
├── collector.rs        // CPU/内存采集（GetSystemTimes, GlobalMemoryStatusEx, PDH）
├── types.rs            // SystemSnapshot, MemoryInfo, CpuInfo 数据结构
└── cleaner.rs          // 内存清理（NtSetSystemInformation, EmptyWorkingSet, 卷刷新）

src/xun_core/
├── sys_monitor_cmd.rs  // SysMonitorCmd + CommandSpec 实现
├── services/
│   ├── sys_monitor.rs  // 采集服务：snapshot_cpu(), snapshot_memory()
│   └── mem_cleaner.rs  // 清理服务：MemCleanOp (Operation trait)
```

### 4.2 Feature Gate 与依赖

```toml
# Cargo.toml
[dependencies]
bitflags = "2"  # CleanAreas bitmask 定义

[features]
sys_monitor = []
```

`windows-sys` 的 `Win32_System_Memory`、`Win32_System_SystemInformation`、`Win32_System_ProcessStatus`、`Win32_System_Threading` 已在 Cargo.toml:144-174 中声明，无需额外添加。`NtSetSystemInformation`、`EmptyWorkingSet`、`SetSystemFileCacheSize` 通过 `#[link]` 手动绑定。

### 4.3 数据类型

参照 `TableRow` trait（`table_row.rs`）实现：

```rust
/// 内存快照 — 实现 TableRow 以支持 CLI 表格输出
pub struct MemorySnapshot {
    pub total_mb: u64,
    pub used_mb: u64,
    pub free_mb: u64,
    pub percent: f64,
    pub swap_total_mb: u64,
    pub swap_used_mb: u64,
    pub cache_mb: u64,
}

/// CPU 快照
pub struct CpuSnapshot {
    pub usage_percent: f64,
    pub cores: Vec<f64>,  // 每核使用率
}

/// 清理执行报告 — 每个区域的独立结果
pub struct CleanReport {
    pub freed_mb: u64,
    pub area_results: Vec<AreaResult>,
}

pub struct AreaResult {
    pub area: CleanAreas,
    pub success: bool,
    pub error: Option<String>,  // 失败时的错误信息
    pub freed_mb: u64,
}
```

### 4.4 CLI 命令

```rust
// sys_monitor_cmd.rs
#[derive(Parser, Args)]
pub struct SysMonitorCmd {
    #[command(subcommand)]
    pub sub: SysMonitorSubCommand,
}

#[derive(Subcommand)]
pub enum SysMonitorSubCommand {
    /// 显示当前系统状态
    Status,
    /// 持续监控（指定间隔秒数）
    Watch { #[arg(short, long, default_value_t = 1)] interval: u64 },
    /// 执行内存清理
    Clean(CleanCmd),
}

#[derive(Parser, Args)]
pub struct CleanCmd {
    /// 清理级别: safe, standard, deep
    #[arg(short, long, default_value = "standard")]
    level: String,
    /// 精确指定清理区域（覆盖 --level）
    #[arg(short, long)]
    areas: Option<String>,
    /// 自动触发阈值（百分比）
    #[arg(long)]
    auto_threshold: Option<u8>,
    /// 自动触发间隔（分钟）
    #[arg(long)]
    auto_interval: Option<u32>,
}
```

接入 `SubCommand` enum（`dispatch.rs`），feature-gated：

```rust
#[cfg(feature = "sys_monitor")]
#[command(subcommand)]
SysMonitor(SysMonitorCmd),
```

### 4.5 Operation trait 集成

清理操作实现 `Operation` trait（`operation.rs:141`），对接 `run_operation()`（`operation.rs:155`）的 preview → confirm → execute 流程。

**CLI 路径**：`CommandSpec::run()` → `run_operation(&mem_clean_op, ctx)` → preview → confirm → execute。

**Dashboard WS 路径**：前端发送 `PreviewOp { operation: "mem_cleaner.clean", args }` → `dispatch_preview()` 返回 `Preview` → 用户确认 → 发送 `ConfirmOp` → `dispatch_confirm()` → 执行 → 返回 `OperationResult`。

```rust
// src/xun_core/services/mem_cleaner.rs

pub struct MemCleanOp {
    areas: CleanAreas,
    preview: Preview,
}

impl MemCleanOp {
    pub fn new(areas: CleanAreas) -> Self {
        let risk = if areas.contains(CleanAreas::STANDBY_LIST) {
            RiskLevel::High
        } else {
            RiskLevel::Medium
        };
        let preview = Preview::new(format!("Memory clean: {:?}", areas))
            .add_change(Change::new("clean", format!("{:?}", areas)))
            .with_risk_level(risk);
        Self { areas, preview }
    }
}

impl Operation for MemCleanOp {
    fn preview(&self) -> &Preview { &self.preview }

    fn execute(&self, _ctx: &mut CmdContext) -> Result<OperationResult, XunError> {
        // 权限不足时降级，不 panic（memory-monitor:424-425 容错策略）
        if let Err(e) = elevate_privileges() {
            log::warn!("Privilege elevation failed: {e}, falling back to user-level clean");
        }

        let before = snapshot_memory()?;
        let report = execute_clean_pipeline(self.areas)?;
        let after = snapshot_memory()?;

        let freed_mb = before.used_mb.saturating_sub(after.used_mb);
        release_self_working_set();

        Ok(OperationResult::new()
            .with_changes_applied(freed_mb as u32))
    }
}

/// CLI CommandSpec 调用入口
pub fn clean_memory(areas: CleanAreas, ctx: &mut CmdContext) -> Result<OperationResult, XunError> {
    let op = MemCleanOp::new(areas);
    run_operation(&op, ctx)
}
```

Dashboard WS 端 `dispatch_preview` / `dispatch_confirm` 激活方式（`handlers/ws.rs:132-154`）：

```rust
async fn dispatch_preview(operation: &str, args: &[String], state: &DashboardState) -> WsResponse {
    match operation {
        "mem_cleaner.clean" => {
            let areas = parse_areas_from_args(args);
            let op = MemCleanOp::new(areas);
            WsResponse::preview_result(op.preview().clone())
        }
        _ => WsResponse::error(format!("unknown operation: {operation}"), WsErrorCode::NotFound),
    }
}

async fn dispatch_confirm(operation: &str, args: &[String], state: &DashboardState) -> WsResponse {
    match operation {
        "mem_cleaner.clean" => {
            let areas = parse_areas_from_args(args);
            let op = MemCleanOp::new(areas);
            let mut ctx = CmdContext::builder().non_interactive(true).build();
            match op.execute(&mut ctx) {
                Ok(result) => WsResponse::op_result(result),
                Err(e) => WsResponse::error(e.to_string(), WsErrorCode::ExecutionFailed),
            }
        }
        _ => WsResponse::error(format!("unknown operation: {operation}"), WsErrorCode::NotFound),
    }
}
```

### 4.6 辅助函数

**parse_areas_from_args**：从 WS args 或 CLI `--areas` 参数解析 bitmask：

```rust
fn parse_areas_from_args(args: &[String]) -> CleanAreas {
    let mut areas = CleanAreas::empty();
    for arg in args {
        match arg.as_str() {
            "working_set"     => areas |= CleanAreas::WORKING_SET,
            "file_cache"      => areas |= CleanAreas::SYSTEM_FILE_CACHE,
            "standby_low"     => areas |= CleanAreas::STANDBY_LOW_PRIORITY,
            "standby"         => areas |= CleanAreas::STANDBY_LIST,
            "modified"        => areas |= CleanAreas::MODIFIED_LIST,
            "combine"         => areas |= CleanAreas::COMBINE_MEMORY,
            "registry"        => areas |= CleanAreas::REGISTRY_CACHE,
            "volume_cache"    => areas |= CleanAreas::MODIFIED_FILE_CACHE,
            "safe"            => areas |= LEVEL_SAFE,
            "standard"        => areas |= LEVEL_STANDARD,
            "deep"            => areas |= LEVEL_DEEP,
            _ => {}
        }
    }
    if areas.is_empty() { LEVEL_STANDARD } else { areas }
}
```

**release_self_working_set**：清理完成后释放自身进程工作集（参照 WinMemoryCleaner `ComputerService.cs:614-620`）：

```rust
fn release_self_working_set() {
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    unsafe { EmptyWorkingSet(GetCurrentProcess()); }
    // 返回值忽略：best-effort，失败不影响主流程
}
```

### 4.7 Dashboard WebSocket 集成

在 `handlers/ws.rs` 的 `dispatch_query()` 中添加：

```rust
"system.status" => {
    let snapshot = sys_monitor::snapshot_all()?;
    Ok(WsResponse::QueryResult { table: snapshot.to_table() })
}
```

清理操作对接现有的 stub `dispatch_preview()` / `dispatch_confirm()`（`handlers/ws.rs:132-154`），激活 PreviewOp → ConfirmOp 流程。

### 4.8 Dashboard 前端

在 `dashboard-ui/src/types.ts` 中添加 TypeScript 接口，在 `dashboard-ui/src/api.ts` 中添加 API 函数，新建 Vue 组件展示实时监控面板。

---

## 5. 安全与防御性设计

| 防御措施 | 实现方式 | 来源 |
|:---|:---|:---|
| 权限不足优雅降级 | 失败区域跳过，继续执行其他区域 | memory-monitor:424-425 |
| 单区域失败隔离 | 每个区域独立 try/catch | WinMemoryCleaner:161-414 |
| 自动清理排除高风险区域 | 默认排除 STANDBY_LIST + MODIFIED_LIST | memreduct:337-339 |
| 进程排除列表 | 可配置名称列表 + 自身自动排除 | WinMemoryCleaner:724 |
| 系统进程保护 | 内核自动跳过受保护进程 + AccessDenied 静默忽略 | WinMemoryCleaner:736,741 |
| OS 版本门控 | 不支持的区域自动禁用 | WinMemoryCleaner:14,38,46,54 |
| Handle 泄漏防护 | CloseHandle 在 EmptyWorkingSet 后无条件调用 | memory-monitor:109 |
| 无 unwrap 在内核路径 | 所有 FFI 调用使用 Result 或返回值检查 | memory-monitor 设计原则 |

---

## 6. 实现阶段规划

### Phase 1: 监控基座

- [ ] `Cargo.toml` — 添加 `bitflags` 依赖 + `sys_monitor = []` feature gate
- [ ] `src/sys_monitor/mod.rs` — 模块入口 + CleanAreas bitflags 定义
- [ ] `src/sys_monitor/types.rs` — MemorySnapshot, CpuSnapshot, CleanReport, AreaResult + TableRow 实现
- [ ] `src/sys_monitor/collector.rs` — GetSystemTimes CPU 差分 + GlobalMemoryStatusEx + PDH SwapCollector
- [ ] `src/xun_core/services/sys_monitor.rs` — snapshot_cpu(), snapshot_memory(), snapshot_all()
- [ ] `src/xun_core/sys_monitor_cmd.rs` — SysMonitorCmd + status 子命令 + CommandSpec
- [ ] dispatch.rs 接入 `SubCommand::SysMonitor`（feature-gated）
- [ ] 单元测试：CPU 差分计算、内存快照字段范围、SwapCollector 初始化

### Phase 2: 安全清理

- [ ] `src/sys_monitor/cleaner.rs` — FFI 绑定（NtSetSystemInformation, EmptyWorkingSet, SetSystemFileCacheSize）
- [ ] `src/sys_monitor/cleaner.rs` — 权限提升（elevate_privileges + set_privilege）
- [ ] `src/sys_monitor/cleaner.rs` — OS 版本门控（available_areas）
- [ ] `src/sys_monitor/cleaner.rs` — Level Safe 清理流水线（WorkingSet + FileCache + StandbyLowPri）
- [ ] `src/sys_monitor/cleaner.rs` — flush_system_file_cache（双调用模式）
- [ ] `src/sys_monitor/cleaner.rs` — release_self_working_set
- [ ] `src/xun_core/services/mem_cleaner.rs` — MemCleanOp (Operation trait) + clean_memory()
- [ ] CLI `sys-monitor clean --level safe` 命令接入 dispatch.rs
- [ ] 单元测试：CleanAreas 组合/互斥逻辑、OS 门控分支、权限降级路径

### Phase 3: 完整清理 + 自动触发

- [ ] `src/sys_monitor/cleaner.rs` — flush_volume_cache（DriveInfo 枚举 + FlushFileBuffers + DeviceIoControl）
- [ ] `src/sys_monitor/cleaner.rs` — CombineMemory（NtSetSystemInformation(130)）+ RegistryCache（155）
- [ ] `src/sys_monitor/cleaner.rs` — execute_clean_pipeline 完整 8 区域流水线
- [ ] `src/sys_monitor/auto_clean.rs` — AutoCleanConfig + check_auto_clean + 采集线程集成
- [ ] CLI `sys-monitor clean --areas working_set,file_cache` 精细控制
- [ ] CLI `sys-monitor clean --auto-threshold 85 --auto-interval 20` 自动触发配置
- [ ] 集成测试：清理前后内存差值验证、auto-clean 阈值/定时触发

### Phase 4: Dashboard 集成

- [ ] `handlers/ws.rs` — dispatch_query 添加 `"system.status"` 路由
- [ ] `handlers/ws.rs` — 激活 dispatch_preview / dispatch_confirm（`"mem_cleaner.clean"`）
- [ ] Dashboard 前端：types.ts 添加 SystemSnapshot 接口
- [ ] Dashboard 前端：api.ts 添加 system status API 函数
- [ ] Dashboard 前端：SystemMonitorPanel.vue 实时监控面板
- [ ] Dashboard 前端：CleanTriggerPanel.vue 清理触发与预览确认

---

## 7. 关键常量速查表

```rust
// NtSetSystemInformation 信息类
const SYSTEM_FILE_CACHE_INFO: u32 = 21;          // 0x15
const SYSTEM_MEMORY_LIST_INFO: u32 = 80;          // 0x50
const SYSTEM_COMBINE_MEMORY_INFO: u32 = 130;      // 0x82
const SYSTEM_REGISTRY_RECONCILE_INFO: u32 = 155;  // 0x9B

// SYSTEM_MEMORY_LIST_COMMAND 子命令
const MEMORY_EMPTY_WORKING_SETS: u32 = 2;          // 注意：不是 1
const MEMORY_FLUSH_MODIFIED_LIST: u32 = 3;
const MEMORY_PURGE_STANDBY_LIST: u32 = 4;
const MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST: u32 = 5;

// 权限常量
const SE_PROF_SINGLE_PROCESS_PRIVILEGE: &str = "SeProfileSingleProcessPrivilege";
const SE_INCREASE_QUOTA_PRIVILEGE: &str = "SeIncreaseQuotaPrivilege";

// Volume Cache Flush 控制码
const FSCTL_DISCARD_VOLUME_CACHE: u32 = 0x00090054;    // Win8+
const FSCTL_RESET_WRITE_ORDER: u32 = 0x000900F8;       // Win7+

// 自动清理默认值
const DEFAULT_AUTO_THRESHOLD_PERCENT: u8 = 90;     // memreduct main.h:32
const DEFAULT_AUTO_INTERVAL_MINUTES: u32 = 30;     // memreduct main.h:33

// 采集间隔
const DEFAULT_COLLECT_INTERVAL_MS: u64 = 1000;    // 1 秒

// PDH 计数器路径
const PDH_PAGING_FILE_PERCENT: &str = "\\Paging File(_Total)\\% Usage";
```
