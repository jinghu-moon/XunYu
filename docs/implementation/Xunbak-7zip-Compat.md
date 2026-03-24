# .xunbak 与 7-Zip 兼容方案

> 生成时间：2026-03-23（第三轮审核修订）
> 依据：7-Zip 26.00 源码分析 + 7z-assembly 参考实现 + Rust COM DLL 可行性调研

---

## 1. 结论先行

**推荐方案：backup 直接支持多格式输出；方案 A 的 PoC 已实现，正式化与发布化后置**

| 方案 | 定位 | 实现语言 | 优先级 |
|------|------|---------|--------|
| **B1. `xun backup --format zip`** | 直接生成标准 `.zip` 备份产物 | Rust（内置） | Phase 2（优先） |
| **B2. `xun backup --format 7z`** | 直接生成标准 `.7z` 备份产物 | Rust（参考 `sevenz-rust2`，仓库内自实现） | Phase 3（正式路线） |
| **A. 7-Zip 格式插件 DLL（只读）** | 用户用 7-Zip 直接浏览/解压 `.xunbak` | Rust PoC / C++ 薄壳 + Rust core 正式版 | PoC 已完成，正式化在 Phase 4 |

不推荐修改 .xunbak 本体格式，也不推荐维护 7-Zip fork。

**术语定义**：

- `backup / restore / export` 是**动作**
- `dir / xunbak / zip / 7z` 是**备份产物或输出格式**

动作与产物的关系：

```text
backup create  -> dir | xunbak | zip | 7z
backup restore (dir | xunbak | zip | 7z) -> target dir
backup convert (dir | xunbak | zip | 7z) -> dir | xunbak | zip | 7z
```

约束：

1. `backup create` 负责生成**新的备份产物**
2. `backup restore` 的目标总是 `target dir`
3. `backup convert` 负责**已有备份产物之间的格式转换**
4. `convert --format dir` 的输出格式是 `dir`
5. `convert --format dir` 可复用 restore 内核，但语义上仍属于 `convert`

**方案 A 前置条件**（PoC 前必须完成）：
1. 确认底层解析函数（`Header::from_bytes`、`read_manifest_record`、`read_blob_record` 等）可脱离 `ContainerReader` 独立调用——当前已满足
2. 选定一个实际要测试的 7-Zip 版本（如 26.00），验证 DLL 加载可用

**方案 A 正式版建议重构**（PoC 之后，按需推进）：
1. `ContainerReader` 流式抽象：新增 `open_from_stream(impl Read+Seek)` 入口，使 reader 和 7-Zip 插件共享同一套打开逻辑
2. 分卷支持：实现 `IArchiveOpenVolumeCallback` 回调驱动模式

---

## 2. 7-Zip 外部格式 DLL 加载机制

> **重要提示**：7-Zip 官方 FAQ 文档化的集成方式是 7z.dll / 7za.dll COM 接口和 7za.exe 命令行。
> 外部 `Formats/` DLL 加载是**源码级能力**（`LoadCodecs.cpp`，受 `EXTERNAL_CODECS` 编译宏控制），
> 不构成官方稳定 SDK 承诺。实际可用性需**按 7-Zip 版本矩阵逐一验证**。

7-Zip 在编译启用 `EXTERNAL_CODECS` 时，会从安装目录扫描并加载外部 DLL：

```text
C:\Program Files\7-Zip\
├── Formats\   ← 归档格式插件（IInArchive / IOutArchive）
├── Codecs\    ← 压缩算法插件（ICompressCoder）
└── Plugins\   ← 文件管理器 UI 插件
```

**现状评估：**

1. 格式 DLL 放入 `Formats/` 目录，7-Zip 启动时通过 `LoadLibrary` + `GetProcAddress` 加载
2. **不是标准 Windows COM**——不走注册表，不走 `CoCreateInstance`
3. 已有第三方先例：Asar7z（Electron .asar）、Lzip7z、Forensic7z、ExFat7z、7-Zip-zstd 等
4. 开源参考：[7z-assembly](https://github.com/ikremniou/7z-assembly)（SZE 格式，完整教程）
5. **稳定性风险**：此 ABI 没有官方文档化的稳定性承诺；7-Zip 大版本升级可能变更接口签名或 vtable 布局。三方插件（如 7-Zip-zstd）通常锁定特定 7-Zip 版本区间

### 2.1 DLL 导出函数

#### 最小必需集（PoC 阶段）

| 导出函数 | 签名 | 用途 |
|----------|------|------|
| `CreateObject` | `(GUID*, GUID*, void**) -> HRESULT` | 根据 CLSID 创建 IInArchive 实例 |
| `GetNumberOfFormats` | `(UInt32*) -> HRESULT` | 返回 DLL 支持的格式数（返回 1） |
| `GetHandlerProperty2` | `(UInt32, PROPID, PROPVARIANT*) -> HRESULT` | 按格式索引查询属性（名称、扩展名、签名） |

#### 兼容推荐集（正式发布阶段）

| 导出函数 | 签名 | 用途 |
|----------|------|------|
| `GetHandlerProperty` | `(PROPID, PROPVARIANT*) -> HRESULT` | 旧版 7-Zip 回退入口（`LoadCodecs.cpp` 中 `GetHandlerProperty2` 不存在时调用此函数） |
| `GetIsArc` | `(UInt32, Func_IsArc*) -> HRESULT` | 返回格式签名快速检测函数指针，用于文件类型识别优化 |

> **来源**：`ArchiveExports.cpp` 导出全部 5 个函数；`LoadCodecs.cpp` 的 `ReadProp()` 实现 `GetHandlerProperty2 → GetHandlerProperty` 回退逻辑。

### 2.2 IInArchive 接口方法（10 个，IUnknown 之后）

| # | 方法 | 用途 |
|---|------|------|
| 1 | `Open(IInStream*, UInt64*, IArchiveOpenCallback*)` | 打开归档，解析结构 |
| 2 | `Close()` | 关闭归档 |
| 3 | `GetNumberOfItems(UInt32*)` | 返回文件总数 |
| 4 | `GetProperty(UInt32 index, PROPID, PROPVARIANT*)` | 返回第 N 个文件的属性 |
| 5 | `Extract(UInt32*, UInt32, Int32, IArchiveExtractCallback*)` | 解压指定文件（**indices 数组必须升序**） |
| 6 | `GetArchiveProperty(PROPID, PROPVARIANT*)` | 返回归档级属性 |
| 7 | `GetNumberOfProperties(UInt32*)` | 返回 Item 属性种类数 |
| 8 | `GetPropertyInfo(UInt32, BSTR*, PROPID*, VARTYPE*)` | 返回第 N 种 Item 属性的名称/ID/类型 |
| 9 | `GetNumberOfArchiveProperties(UInt32*)` | 返回归档级属性种类数 |
| 10 | `GetArchivePropertyInfo(UInt32, BSTR*, PROPID*, VARTYPE*)` | 返回第 N 种归档属性的名称/ID/类型 |

> **来源**：7-Zip 26.00 `CPP/7zip/Archive/IArchive.h`。当前 XunYu PoC 已实际实现方法 7-10 的基础属性列定义，不再是“空值占位”状态。

---

## 3. Rust 实现可行性分析

### 3.1 结论：技术可行，PoC 无需前置重构

| 条件 | 状态 |
|------|------|
| Rust 生成 Windows DLL | `crate-type = ["cdylib"]`，原生支持 |
| 导出 C ABI 函数 | `#[no_mangle] extern "system" fn`，完全支持 |
| 7-Zip 加载机制 | `LoadLibrary` + `GetProcAddress`，不需要 COM 注册 |
| COM vtable 布局 | `#[repr(C)]` struct 手动定义，完全可控 |
| xunbak 解析逻辑 | **大部分可直接复用**：manifest/blob/codec/header/footer 可直接复用；PoC 可绕过 `ContainerReader::open`，正式版再考虑流式入口 |
| 先例 | 无公开的 Rust 7-Zip 插件，但 Rust COM DLL 技术成熟 |

### 3.2 为什么选 Rust 而不是 C++

| 维度 | C++ | Rust |
|------|-----|------|
| xunbak 解析逻辑 | 需要用 C++ 重写一遍 Header/Footer/Checkpoint/Manifest/Blob 解析 | **直接复用现有 `xun::xunbak::*` 模块** |
| JSON manifest 解析 | 需引入 rapidjson 等 C++ 库 | **已有 serde_json，manifest 数据结构已定义** |
| zstd 解压 | 需链接 zstd C 库 | **已有 zstd crate 依赖** |
| blake3 校验 | 需链接 blake3 C 库 | **已有 blake3 crate 依赖** |
| CRC32C | 需引入 C 库 | **已有 crc32c crate 依赖** |
| 代码量 | ~1500-2500 行（含解析逻辑重写） | **PoC ~800-1200 行（COM 胶水 + handler + InStream 适配）** |
| 维护同步 | .xunbak 格式变更需同步 C++ 和 Rust 两份代码 | **单一代码库，零同步成本** |
| 内存安全 | 手动管理 | 编译期保证 |

### 3.3 技术挑战与应对

| 挑战 | 应对方案 |
|------|---------|
| **ContainerReader 绑定文件路径** | 当前 `open(&Path)` 直接调用 `File::open`，分卷发现用 `read_dir`。7-Zip 传入 `IInStream*`（COM 流指针），不提供路径。**PoC 解法**：绕过 `ContainerReader`，直接调用底层模块（`Header::from_bytes`、`read_manifest_record`、`read_blob_record`），这些函数接受 `impl Read` / `&[u8]`，不绑定路径。**正式版可选**：新增 `open_from_stream(impl Read+Seek)` 统一入口。 |
| COM vtable 必须与 C++ 布局完全一致 | 用 `#[repr(C)]` struct 定义 vtable，每个接口约 20 行 |
| PROPVARIANT 处理 | 封装 `set_variant_bstr()` / `set_variant_u64()` 等辅助函数 |
| IInStream 回调 | 7-Zip 传入的 COM 接口指针，通过裸指针调用 vtable 方法 |
| 引用计数（AddRef/Release） | `AtomicU32` 手动实现，~15 行 |
| 字符串编码 | manifest 中 UTF-8 path → `BSTR`（UTF-16），标准 Rust 转换 |
| **packed size 语义** | `ManifestEntry.blob_len` 含 record prefix + blob header 开销（常量定义见 `xun::xunbak::constants`，**不要硬编码数字**）。推荐在 Open 阶段 seek 到每个 entry 的 `blob_offset + RECORD_PREFIX_SIZE`，读取 `BlobHeader.stored_size` 缓存到 `Vec<u64>`。 |
| **分卷支持** | 7-Zip 的多卷入口是 `IArchiveOpenVolumeCallback`，由 7-Zip 按需回调获取下一卷流。当前 XunYu PoC 已通过 callback bridge 支持分卷 `.xunbak.001/.002/...` 打开；正式版继续增强稳定性与版本矩阵。 |

### 3.4 不适用的方案

| 方案 | 原因 |
|------|------|
| `windows` crate `#[implement]` 宏 | 7-Zip 的接口不在 Windows metadata 中，宏无法使用 |
| `com-rs` crate | 已于 2024-09 归档弃用 |
| `intercom` crate | 7-Zip 不走标准 `DllGetClassObject` 入口 |

**结论：手动 `#[repr(C)]` vtable 是最合适的路径。** 代码量可控（~300 行接口定义），且完全掌控内存布局。

---

## 4. 方案对比

### 方案 A：7-Zip 格式插件 DLL（Rust，只读）

**原理**：编写 `xunbak.dll`（Rust cdylib），实现 `IInArchive` 接口，放入 `Formats/` 目录。

**用户体验**：
```
右键 project.xunbak → 7-Zip → 打开压缩文件
→ 看到文件列表（路径、大小、时间、压缩率）
→ 双击预览 / 拖拽解压 / 解压到...
```

**技术映射**：

| 7-Zip 概念 | .xunbak 对应 | 备注 |
|------------|-------------|------|
| Archive signature | `XUNBAK\0\0`（Header 前 8 字节） | |
| Archive items | Manifest entries | |
| Item path | `entry.path`（UTF-8 → UTF-16） | |
| Item size | `entry.size`（原始大小） | |
| Packed size | 读取 `BlobHeader.stored_size`（推荐），或 `entry.blob_len - RECORD_PREFIX_SIZE - BLOB_HEADER_SIZE` | **不能直接用 `blob_len`**，它含 record prefix + blob header 开销。常量值来自 `xun::xunbak::constants`，不要硬编码数字。推荐在 `Open` 阶段 seek 到每个 blob 读取 `BlobHeader.stored_size` 缓存。 |
| Item mtime | `entry.mtime_ns` → FILETIME | 纳秒 → 100ns 精度损失可忽略 |
| Item ctime | `entry.created_time_ns` → FILETIME | |
| Item attributes | `entry.win_attributes` | |
| Extract | seek(blob_offset) → read record prefix → read blob header → read compressed data → decompress | |

**前置条件**：

| 前置项 | 当前状态 | 需要做的 |
|--------|---------|---------|
| 底层模块独立可用 | `Header::from_bytes`、`read_manifest_record`、`read_blob_record` 等接受 `impl Read` / `&[u8]`，不绑定路径 | **已满足**，PoC 可直接调用 |
| packed size | `ManifestEntry` 无 `stored_size` 字段 | 在 Open 阶段 seek 读 `BlobHeader.stored_size` 缓存到 `Vec<u64>` |
| 流式 reader | `ContainerReader::open(&Path)` 绑定文件路径 | **PoC 不需要**（绕过 ContainerReader）。正式版可选重构 |
| 分卷 | `discover_split_volumes` 依赖 `read_dir` | PoC 不支持；正式版需实现 `IArchiveOpenVolumeCallback` |

**可行性评估**：

| 维度 | 评估 |
|------|------|
| 技术难度 | 中偏高。COM 胶水 ~300 行 + handler 适配 ~400-600 行 + packed size 处理 |
| 工作量 | ~800-1200 行 Rust（COM 胶水 + 导出函数 + handler + InStream 适配） |
| 依赖 | 复用 `xun::xunbak::*`（workspace 内引用） |
| 分发 | 单个 DLL 文件（~500KB-1MB），用户放入 `Formats/` 目录 |
| 维护 | .xunbak 格式变更自动同步（同一代码库） |
| 局限 | 只读（不支持通过 7-Zip 创建 .xunbak） |

**优势**：
1. 用户零门槛：装一个 DLL，原生 7-Zip 直接打开
2. 不改动 .xunbak 格式本体
3. 不改动 7-Zip 本体
4. **复用现有 Rust 解析代码**（manifest/blob/codec/header/footer），PoC 不要求 `ContainerReader` 重构
5. **格式变更时低同步开销**

**风险**：
1. **外部格式 DLL 非官方稳定 SDK**：7-Zip FAQ 未将 `Formats/` DLL 加载文档化为官方集成方式；`LoadCodecs.cpp` 中该功能受 `EXTERNAL_CODECS` 编译宏控制。需按 7-Zip 版本矩阵逐一验证，建议锁定 7-Zip 24.x-26.x 区间
2. Rust cdylib 体积比纯 C++ DLL 略大（~500KB vs ~200KB）
3. **reader 路径绑定可绕过**：PoC 直接调用底层模块（`Header::from_bytes`、`read_manifest_record`、`read_blob_record`），不走 `ContainerReader::open`。正式版可选重构 `open_from_stream`
4. **分卷**需实现 `IArchiveOpenVolumeCallback` 回调驱动模式，与当前 reader 的文件系统枚举逻辑不兼容，PoC 阶段不支持

---

### 方案 B：`xun backup create|restore|convert` 多格式产物模型

**原理**：

- `backup create`：直接生成 `dir / xunbak / zip / 7z` 备份产物
- `backup restore`：从 `dir / xunbak / zip / 7z` 恢复到目标目录
- `backup convert`：已有备份产物之间做格式转换

**CLI 形态**：

> **命名约束**：仓库当前已有顶层 `xun export`（书签导出）命令。  
> 因此这里统一采用 `xun backup create|restore|convert` 子命令模型，不再占用顶层 `export`。

```bash
# 直接备份为 zip
xun backup create -C ./project --format zip -o project.zip

# 直接备份为 7z
xun backup create -C ./project --format 7z -o project.7z

# 直接备份为目录格式
xun backup create -C ./project --format dir -o ./output/

# 直接备份为分卷 7z
xun backup create -C ./project --format 7z -o project.7z --split-size 2g

# 已有备份产物之间做转换（可选后续能力）
xun backup convert project.xunbak --format zip -o project.zip

# 从备份产物恢复到目录
xun backup restore project.xunbak --to ./restore-output/
```

### B 方案模块结构

```text
src/
├── cli/
│   └── backup.rs                  # 新增 BackupCreateCmd / BackupRestoreCmd / BackupConvertCmd
├── commands/
│   ├── backup.rs                  # 路由 backup create|restore|convert
│   └── xunbak_export.rs           # convert/create 的导出实现
└── xunbak/
    └── export/
        ├── mod.rs                 # 对外入口：export_container()
        ├── options.rs             # ExportFormat / ExportOptions / SevenZOptions
        ├── selection.rs           # 统一选择器（create/convert 共用）
        ├── fs_source.rs           # 从文件系统源目录构建 SourceEntry
        ├── artifact_source.rs     # 从 dir/xunbak/zip/7z 产物构建 SourceEntry
        ├── source.rs              # VerifiedEntryReader / copy_verified_entry_to
        ├── metadata.rs            # 将 ManifestEntry 转换为 7z/zip/dir 元数据
        ├── sidecar.rs             # sidecar schema v1（首版固定）
        ├── verify.rs              # 导出前后校验（preflight / postflight）
        ├── progress.rs            # ExportProgressEvent / 进度聚合
        ├── output_plan.rs         # 临时文件、overwrite、原子 rename 策略
        ├── dir_writer.rs          # --format dir（复用 restore 语义）
        ├── zip_writer.rs          # --format zip（当前已实现 stored|deflated，后续扩到 bzip2|zstd|ppmd）
        ├── sevenz/
        │   ├── mod.rs             # SevenZExportWriter 入口
        │   ├── header.rs          # 单卷/分卷共用最小 header 写出
        │   ├── methods.rs         # 当前已实现 copy|lzma2，后续扩到 bzip2|deflate|ppmd|zstd
        │   ├── pack_streams.rs    # pack info / unpack info / substreams
        │   ├── writer.rs          # SevenZWriter：逻辑 7z 输出（当前默认 non-solid）
        │   ├── segmented_writer.rs# SegmentedWriter：分卷 Write+Seek 虚拟连续流
        │   └── volume_naming.rs   # .7z.001/.002 规则
        └── fallback.rs            # B3：探测 7z.exe / 7za.exe / 7zr.exe
```

**模块职责分层**：

1. `fs_source.rs` 负责“从源目录扫描出待备份条目”（`backup create`）
2. `artifact_source.rs` 负责“从已有产物读取条目”（`backup convert`）
3. `selection.rs` 负责统一处理 `file/glob/files-from/patterns-from`
4. `source.rs` 负责“把选中的 entry 暴露成**已校验的流式读取源**”
5. `metadata.rs` 负责“把源条目转成导出格式元数据”
6. `sidecar.rs` 负责 lossy 导出时的补充元数据写出
7. `verify.rs` 负责导出前后的校验策略
8. `progress.rs` 负责统一进度模型
9. `output_plan.rs` 负责临时文件 / 分卷临时基名 / 覆盖策略 / 原子切换
10. `zip_writer.rs / sevenz::writer.rs / dir_writer.rs` 负责“目标格式怎么写”
11. `sevenz::segmented_writer.rs` 只负责“分卷输出”，不理解源类型

> **边界约束（已更新）**：B1/B2 统一采用“**优先参考同类型项目，XunYu 内手写实现**”原则。
> 也就是说：
>
> 1. `zip2-master`、`sevenz-rust2-main`、`7z2600-src`、`7-Zip-zstd-master` 作为参考来源
> 2. 最终落地优先写入 XunYu 自己的 ZIP / 7z 输出层
> 3. 不把外部 crate 当成长期唯一实现载体

### 导出源抽象（适用于 B1/B2/dir）

当前 `.xunbak` 读取接口 [`read_and_verify_blob()`](/D:/100_Projects/110_Daily/XunYu/src/xunbak/reader.rs:177) 返回 `Vec<u8>`。  
若导出继续沿用这个接口，大文件会先完整读入内存，再交给 ZIP/7z writer，不符合完整设计要求。

**建议抽象**：

```rust
pub trait VerifiedEntryReader {
    fn open_entry<'a>(
        &'a self,
        entry: &'a ManifestEntry,
    ) -> Result<Box<dyn Read + 'a>, ExportError>;

    fn entry_size(&self, entry: &ManifestEntry) -> u64;
}

pub fn copy_verified_entry_to<W: Write>(
    reader: &ContainerReader,
    entry: &ManifestEntry,
    out: &mut W,
) -> Result<u64, ExportError>;
```

**约束**：

1. 默认导出路径使用 `Read -> Write` 流式复制，不以 `Vec<u8>` 为主通道
2. `dir` / `zip` / `7z` 三种格式都复用同一套“已校验源”抽象
3. 大文件导出时内存峰值应与 chunk size 成正比，而不是与文件大小成正比

**技术路径**：
1. **B1: zip 导出**：优先参考 `zip2-master` 的方法号、writer 行为、测试覆盖，在 XunYu 仓库内手写标准 `.zip` 输出层
2. **B2: 7z 导出（正式路线）**：优先参考 `refer/sevenz-rust2-main` 与 `7z2600-src`，在仓库内手写标准 `.7z` 写出模块。纯 Rust 实现，无需 C 编译器
3. **B3: 7z 导出（过渡 / fallback）**：自动探测用户系统中已安装的 7-Zip 可执行文件，按优先级依次查找 `7z.exe`（标准安装）→ `7za.exe`（Extra 独立版）→ `7zr.exe`（精简版）。找不到则提示用户安装 7-Zip 或仅使用 zip 格式

> **为什么采用“优先参考同类型项目，手写实现”**：
>
> 1. 保持主仓库对方法集、格式细节、兼容性策略的完全控制
> 2. 避免长期被外部 crate 的 API、feature、维护节奏绑定
> 3. 允许按 XunYu 的产品目标精确裁剪：
>    - ZIP：`stored / deflated / bzip2 / zstd / ppmd`
>    - 7z：`copy / lzma2 / bzip2 / deflate / ppmd / zstd`
> 4. 让方法支持、doctor、verify、兼容矩阵、sidecar 语义都统一落在 XunYu 自己的代码中

### 方案 B1 深入理解（基于 `refer/zip2-master`）

**结论**：B1 以“标准 ZIP 输出”为目标，采用“参考 `zip2-master`，在 XunYu 内手写实现”的路线。
新的方法要求是：

1. ZIP 必须支持 `stored`
2. ZIP 必须支持 `deflated`
3. ZIP 必须支持 `bzip2`
4. ZIP 必须支持 `zstd`
5. ZIP 必须支持 `ppmd`
6. ZIP 不承担 `lzma2`（已采用方案 C）

因此 B1 应抽象为 **XunYu 自研 ZIP writer backend**：

1. 以 `zip2-master` 为行为参考，不直接把 crate 当最终实现
2. ZIP 方法号、写入顺序、central directory、Zip64、extra field 由 XunYu 自己控制
3. CLI 对用户暴露的方法集以“目标能力”为准，而不是以外部库现成能力为准

**从源码确认的关键能力**：

| 能力 | 证据 | 说明 |
|------|------|------|
| 标准 ZIP 写出 | `ZipWriter::new()` / `finish()` | 适合写到 `File` 这类可 `Seek` 目标 |
| 流式 ZIP 写出 | `ZipWriter::new_stream()` | 支持非 seek writer，但 B1 当前不是刚需 |
| 文件项写入 | `start_file()` / `start_file_from_path()` | 用于单文件条目 |
| 目录项写入 | `add_directory()` / `add_directory_from_path()` | 示例明确建议显式写目录项，兼容更多解压器 |
| 压缩方法 | `compression_method()` | 已确认可参考 `Stored / Deflated / Bzip2 / Zstd / PPMd`；`LZMA2` 不进入 ZIP 目标方法集（方案 C） |
| 时间戳 | `last_modified_time()` | ZIP 原生时间精度受格式限制 |
| 权限位 | `unix_permissions()` | 只保留 Unix 权限位，不表达 Windows 属性语义 |
| Zip64 | `large_file(true)` / `set_auto_large_file()` | 支持大于 4 GiB 文件和大型 central directory |
| UTF-8 文件名 | 写入标志位与 UTF-8 处理逻辑 | 示例里包含 Unicode 文件名 |
| AES / ZipCrypto | `with_aes_encryption()` | 能力存在，但 B1 默认不启用 |

**从源码确认的边界**：

| 边界 | 现状 |
|------|------|
| Multi-disk ZIP | README 明确写 `Currently unsupported zip extensions: Multi-disk` |
| Windows 属性保真 | `unix_permissions()` 只覆盖 Unix 权限位；不能等价表达 `.xunbak` 的 `win_attributes` |
| 非 UTF-8 路径 | 示例 `write_dir.rs` 直接把 Non UTF-8 Path 视为错误 |
| 安全路径 | 示例强调拒绝路径穿越 / 非法输出路径 |

**对 B1 的设计结论**：

1. B1 输出仍坚持标准 ZIP，不改格式本体
2. B1 默认写出顺序仍参考 `ZipWriter::new(File)` 的常规模式，但实现落在 XunYu 自己的 writer 中
3. B1 默认压缩方法仍建议使用 **Deflated**
4. 若用户明确要求最大兼容性，可使用 `--method stored`
5. B1 不承诺 Windows 属性完全保真；只保证路径、内容、目录结构、mtime 和常规权限语义
6. **B1 不支持 `--split-size`**，因为 multi-disk ZIP 仍不在方案范围内
7. **XunYu ZIP 导出目标方法集调整为 `stored | deflated | bzip2 | zstd | ppmd`**
8. ZIP 不承担 `lzma2`（方案 C）

**B1 参考建议**：

1. `zip2-master`
2. `7z2600-src/DOC/Methods.txt`
3. `PKWARE APPNOTE`（若补入 `refer/`）

并在 XunYu 内保留 ZIP backend 抽象：

1. `method ids`
2. `writer`
3. `central directory`
4. `Zip64`
5. `extra fields`
6. `method routing`

目的：

1. 保持标准 ZIP 互操作
2. 同时满足新增方法需求
3. 允许实现、兼容性和诊断都掌握在 XunYu 自己代码里

**对 `xun backup --format zip` 的现实含义**：

1. `xun backup --format zip` 是最低风险互操作路径
2. 目录项应显式写入，不能只依赖带斜杠的文件路径
3. 对超大文件或大归档，需启用 `large_file(true)` 或 writer 级自动 Zip64
4. ZIP 模式下应拒绝 `--split-size`
5. ZIP 模式默认方法仍建议使用 `deflated`
6. ZIP 模式 CLI 合法值调整为：`stored | deflated | bzip2 | zstd | ppmd`
7. 对于兼容性风险更高的方法，需在文档和 doctor 输出中明确提示 7-Zip/工具兼容差异

**B1 压缩策略（建议固定规则）**：

| 条件 | 方法 |
|------|------|
| 已压缩/不可压缩扩展名（复用 `should_skip_compress()` 规则） | `stored` |
| 其余常规文本/源码/日志（默认） | `deflated` |
| 用户显式指定 `--method stored` | `stored` |
| 用户显式指定 `--method bzip2` | `bzip2` |
| 用户显式指定 `--method zstd` | `zstd` |
| 用户显式指定 `--method ppmd` | `ppmd` |

> 目标：避免对 `.jpg/.png/.zip/.7z/.zst/.mp4` 之类内容重复压缩，浪费 CPU。

**B1 sidecar 设计**：

ZIP 不能完整表达 `.xunbak` 的 `content_hash / created_time_ns / win_attributes / snapshot_id / source_root`。  
因此建议在导出根目录固定写一个保留命名空间文件：

```text
__xunyu__/export_manifest.json
```

至少包含：

```json
{
  "format": "zip",
  "snapshot_id": "...",
  "source_root": "...",
  "exported_at": "...",
  "xunyu_version": "...",
  "entries": [
    {
      "path": "src/main.rs",
      "content_hash": "...",
      "created_time_ns": 0,
      "win_attributes": 32
    }
  ]
}
```

> **适用范围**：`zip` 和 `7z` 都写 sidecar；`dir` 模式可配置开启，默认开启。

**sidecar 可见性策略**：

1. 默认写入 `__xunyu__/export_manifest.json`
2. 这是**可见 sidecar**，用户在 ZIP/7z 根目录中可以看到
3. 当前接受这种可见性，换取实现简单、命名冲突低、调试友好
4. 如用户明确不需要，后续可追加 `--no-sidecar`

### 导出前后校验（B1/B2 通用）

**导出前（preflight）**：

1. 默认先做 `.xunbak quick verify`
2. 提供 `--verify-source quick|full|paranoid|off`
3. `verify` 失败时默认拒绝导出

**导出后（postflight）**：

| 格式 | 校验方式 |
|------|---------|
| dir | 复用 restore 结果统计 + 抽样/全量哈希比对（可选） |
| zip | 用 `zip` crate reopen，检查 central directory、entry 数、可读取性 |
| 7z | 用内部 7z reader reopen smoke test；若启用 B3 fallback，可附加 `7z t` |

**默认行为**：

1. 导出前校验默认 `quick`
2. 导出后校验默认 `on`
3. 提供 `--verify-output on|off`

**B1 写入流程建议**：

1. `selection.rs` 从 manifest 选出目标文件
2. `metadata.rs` 生成 ZIP entry 路径、mtime、目录项标志
3. `zip_writer.rs` 使用 `ZipWriter::new(File)` 创建输出
4. 先显式写目录项 `add_directory(...)`
5. 再逐文件 `start_file(...)` + `std::io::copy(...)`
6. 对超大文件启用 `large_file(true)` 或 writer 级自动 Zip64
7. `finish()` 写 central directory

### 方案 B2 深入理解（基于 `refer/sevenz-rust2-main`）

**已从源码确认的能力**：

| 能力 | 证据 | 说明 |
|------|------|------|
| 标准 `.7z` 写出 | `ArchiveWriter::create/new` + `finish()` | `writer.rs` 直接写 7z 签名头和 start header |
| 目录/文件打包 | `compress_to_path()`、`push_source_path()`、`push_source_path_non_solid()` | `util/compress.rs` 同时支持目录和单文件 |
| solid / non-solid | `push_source_path()` vs `push_source_path_non_solid()` | non-solid 更适合后续按单文件解压 |
| 自定义压缩方法链 | `set_content_methods()` | 可组合 LZMA/LZMA2/BZIP2/PPMd，以及可选 Deflate/Zstd 等 |
| AES-256 + 头部加密 | `AesEncoderOptions`、`set_encrypt_header()`、加密测试 | 已有加密写入与密码读取测试 |
| 文件元数据 | `ArchiveEntry::from_path()` | 自动带入 mtime/ctime/atime，并暴露 `windows_attributes` |
| 多线程 LZMA2 | `Lzma2Options::from_level_mt()`、`mt_decompress.rs` | 编码/解码都已有多线程入口 |

**已确认的限制 / 缺口**：

| 限制 | 现状 |
|------|------|
| 多卷 `.7z` 输出 | **未发现公开 API / 示例 / 测试**，需在 xun 自研写出层单独设计 |
| 部分 codec 需 feature | `brotli` / `deflate` / `lz4` / `zstd` 是可选 cargo feature |
| 默认压缩策略 | 默认 `ArchiveWriter::new()` 使用 `LZMA2` |
| solid block 边界 | `util/compress.rs` 中使用 `MAX_BLOCK_SIZE = 4 GiB` 对 solid 块分片 |

**对 `xun backup --format 7z` 的现实含义**：

1. `xun backup --format 7z` 应优先落在 **non-solid** 路径，保证用户后续用 7-Zip 按文件解压体验更稳定
2. 若追求更高压缩率，可追加 `--solid` 选项映射到对应的 solid writer 语义
3. 时间戳和 Windows 属性可以随归档带出，但恢复保真度仍需单独回归测试
4. 多卷 `.7z` 不能继续作为“未承诺能力”悬空，完整设计必须单列输出层方案
5. 7z 导出的目标方法集调整为：`copy | lzma2 | zstd | ppmd | bzip2 | deflate`
6. 实现策略建议：先稳定 `copy | lzma2` 主路径，再补 `zstd | ppmd | bzip2 | deflate`，但在方案层这些已属于正式支持范围，而不是 future flag

### 方案 B2 的分卷设计（完整方案要求）

**CLI 形态**：

```bash
xun backup -C ./project --format 7z -o project.7z --split-size 2g
```

**输出命名**：

```text
project.7z.001
project.7z.002
project.7z.003
...
```

**核心原则**：

1. `.7z` 分卷是**一个逻辑连续字节流**的分段，不是每卷一个独立的小归档
2. 分卷边界可以落在压缩数据流中间，不要求按文件边界、block 边界或 header 边界对齐
3. `xunbak` 输入是否分卷，与 `.7z` 输出是否分卷**解耦**
4. `xunbak` reader 先恢复逻辑文件视图，`.7z` writer 再按目标卷大小切分输出流

**建议抽象**：

| 抽象 | 职责 |
|------|------|
| `SevenZWriter` | 负责 7z 逻辑结构写出（signature header、pack streams、end header） |
| `SegmentedWriter` | 实现 `Write + Seek`，把一个逻辑输出流映射到多个 `.7z.001/.002/...` 文件 |
| `VolumeNaming` | 负责基名和 `.001/.002/.003` 规则 |

**`SegmentedWriter` 接口草案**：

```rust
pub struct SegmentedWriter {
    base_path: PathBuf,      // e.g. project.7z
    split_size: u64,         // per-volume max size
    current_index: u16,      // 0 => .001
    current_file: File,
    current_len: u64,        // current volume bytes written
    global_pos: u64,         // logical continuous offset
    total_len: u64,          // logical stream length
    volumes: Vec<PathBuf>,
}

impl SegmentedWriter {
    pub fn create(base_path: impl AsRef<Path>, split_size: u64) -> io::Result<Self>;
    pub fn volume_paths(&self) -> &[PathBuf];
    pub fn logical_position(&self) -> u64;
    pub fn finish(self) -> io::Result<Vec<PathBuf>>;
}

impl Write for SegmentedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
    fn flush(&mut self) -> io::Result<()>;
}

impl Seek for SegmentedWriter {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64>;
}
```

**接口语义约束**：

1. `write()` 面向**逻辑连续流**，内部按 `split_size` 自动轮转到下一卷
2. `seek()` **不承诺完全通用**；当前阶段只保证满足 7z writer 的必要 seek 场景：
   - 初始化预留 header 后继续顺序写
   - 结束时回到逻辑偏移 `0` 回写 start header
   - 查询当前位置相关的 seek
3. `logical_position()` 返回全局偏移，不是当前卷内偏移
4. `finish()` 返回全部卷路径，供 CLI 输出和后续验证
5. 最小 `split_size` 必须大于 7z signature header + 最小 end header 的保守上限，否则直接报错

> 若后续需要支持任意随机 seek，需单独引入卷索引、按卷 reopen 和全局偏移到卷内偏移映射；这不属于当前阶段范围。

**句柄管理策略**：

1. 当前实现仅长期持有**当前卷**的 `File`
2. 回写首卷 start header 时，临时 reopen 首卷文件句柄
3. 不常驻持有所有卷句柄，避免句柄数量与卷数线性增长
4. `finish()` 负责 flush 并关闭全部打开句柄

**为什么必须有 `SegmentedWriter`**：

`refer/sevenz-rust2-main` 的 `ArchiveWriter<W: Write + Seek>` 在初始化时先 `seek` 跳过 signature header，结束时再 `seek(0)` 回写 start header。若要支持分卷且保持流式导出，必须提供一个**虚拟连续流**：

1. 初始化时能在逻辑偏移 `0` 预留 start header
2. 顺序写入压缩数据时能自动跨卷轮转
3. 结束时能回到首卷起始位置补写 start header
4. `stream_position()` 返回全局逻辑偏移，而不是当前卷内偏移

**实现边界**：

| 项目 | 设计 |
|------|------|
| 首卷 | 包含 signature header 和前段 pack streams |
| 中间卷 | 纯数据分片，无独立归档头 |
| 末卷 | 包含末段 pack streams 和 end header |
| 校验 | 依赖 7z 自身 header / CRC 语义，不额外叠加 xunbak footer |
| 打开方式 | 由 7-Zip / 兼容工具从 `.001` 入口按多卷规则打开 |

**分卷兼容矩阵（必须验证）**：

| 工具 | 预期 |
|------|------|
| 7-Zip 24.x | 支持从 `.001` 打开和解压 |
| 7-Zip 26.x | 支持从 `.001` 打开和解压 |
| NanaZip | 应支持多卷 `.7z` |
| Windows Explorer | 预期不支持 `.7z` / `.7z.001` |

**验证项**：

1. 从 `.001` 打开
2. 全量解压
3. 指定单文件解压
4. `test archive` / CRC 检查

### `xun backup create|convert` CLI 参数设计

#### `xun backup create`

```bash
xun backup create -C <source_dir> --format 7z -o <target>
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `-C, --dir` | path | 源目录 |
| `--format` | enum | `dir | xunbak | zip | 7z` |
| `-o, --output` | path | 输出目标 |
| `--split-size` | size | 仅 `--format 7z|xunbak` 生效 |
| `--solid` | switch | 仅 `--format 7z` 生效 |
| `--method` | enum | 按 `--format` 解释 |
| `--level` | int | 压缩等级 |
| `--threads` | int | 压缩线程数 |
| `--password` | string | 当前仅 `7z` 规划可选；`zip` 当前禁用 |
| `--encrypt-header` | switch | 与 `--password` 联用 |
| `--overwrite` | enum | `ask | replace | fail` |
| `--dry-run` | switch | 预演，不写输出 |
| `--list` | switch | 输出将被纳入备份的文件清单 |
| `--json` | switch | JSON 输出 |

> `backup create` 的文件选择**不新增独立 glob 入口**，完全复用现有 backup 配置文件、scan、include/exclude/ignore 逻辑。

#### `xun backup convert`

```bash
xun backup convert <artifact> --format zip -o <target>
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `<artifact>` | positional | 输入 `dir | xunbak | zip | 7z` 产物 |
| `--format` | enum | `dir | xunbak | zip | 7z` |
| `-o, --output` | path | 输出目标 |
| `--file` | repeatable path | 只转换指定路径 |
| `--glob` | repeatable glob | 按 glob 选择转换条目 |
| `--patterns-from` | repeatable path | 从文件批量读取选择模式 |
| `--split-size` | size | 仅 `--format 7z|xunbak` 生效 |
| `--solid` | switch | 仅 `--format 7z` 生效 |
| `--method` | enum | 按 `--format` 解释 |
| `--level` | int | 压缩等级 |
| `--threads` | int | 压缩线程数 |
| `--password` | string | 当前仅 `7z` 规划可选；`zip` 当前禁用 |
| `--encrypt-header` | switch | 与 `--password` 联用 |
| `--overwrite` | enum | `ask | replace | fail` |
| `--dry-run` | switch | 预演，不写输出 |
| `--list` | switch | 输出将被转换的条目清单 |
| `--verify-source` | enum | `quick | full | paranoid | off` |
| `--verify-output` | enum | `on | off` |
| `--progress` | enum | `auto | always | off` |
| `--json` | switch | JSON 输出 |

**通用参数约束**：

1. `--split-size` 仅对 `--format 7z|xunbak` 生效；`zip/dir` 使用时报错
2. `--solid`、`--method`、`--level`、`--threads`、`--password`、`--encrypt-header` 仅对压缩格式生效
3. `--encrypt-header` 必须要求 `--password`
4. `zip` 模式下默认 `--method deflated`；若用户显式指定 `stored/bzip2/zstd/ppmd` 则按用户指定方法执行
5. `zip` 模式下不支持 `--split-size`，因为 `zip` crate 当前不支持 multi-disk ZIP
6. `zip` 模式下当前不启用 `--password` / `--encrypt-header`
7. `--dry-run` 与 `--list` 不应创建任何输出文件

**`create` / `convert` 选择器语义**：

| 命令 | 选择器语义 |
|------|------------|
| `backup create` | 针对**源目录内容**复用现有 backup 配置与 scan 逻辑 |
| `backup convert` | 针对**已有产物内部条目**做选择过滤 |

**参数解析顺序 / 冲突优先级**：

1. **格式级校验**：先根据 `--format` 校验方法、分卷、加密等是否允许
2. **源/产物类型校验**：`create` 读取源目录，`convert` 读取已有产物
3. **选择器合并**：`create` 读取现有 backup 配置与 scan 规则；`convert` 合并 `file/glob/patterns-from`
4. **输出策略校验**：校验 `--output`、`--overwrite`、目标存在性、临时路径可写性
5. **preflight 校验**：`convert` 执行 `--verify-source`；`create` 则执行路径/输入可读性检查
6. **交互阶段**：若 `overwrite=ask` 且非 `--json/--dry-run`，展示 preview 并确认
7. **执行阶段**：开始真正写出

**冲突示例**：

| 输入 | 处理 |
|------|------|
| `backup create --format zip --method lzma2` | 立即报参数错误（方案 C：ZIP 不承担 `lzma2`） |
| `backup convert --format dir --split-size 2g` | 立即报参数错误 |
| `backup convert --list --output project.zip` | 允许，但不创建输出文件 |
| `backup create --dry-run --overwrite replace` | 允许，`overwrite` 仅参与 preview，不执行写出 |

**参数解释收敛**：

| 格式 | `--method` 合法值 |
|------|------------------|
| `zip` | `stored` / `deflated` / `bzip2` / `zstd` / `ppmd` |
| `7z` | `copy` / `lzma2` / `zstd` / `ppmd` / `bzip2` / `deflate` |
| `dir` | 不接受 `--method` |

**交互设计补足**：

1. `--dry-run`：只输出选中文件数、原始字节、目标格式、目标路径，不写任何文件
2. `--list`：输出将被导出的文件清单
3. `--files-from` / `--patterns-from`：支持从文件批量读入路径/模式
4. 交互模式且 `overwrite=ask` 时，导出前展示 preview 并确认

**统一进度事件**：

```rust
pub struct ExportProgressEvent {
    pub phase: String,          // verify_source | read | compress | write | verify_output
    pub selected_files: usize,
    pub processed_files: usize,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub throughput: f64,
    pub elapsed_ms: u128,
}
```

`zip / 7z / dir` 三种格式共用一套进度输出协议。

**JSON 结果模型**：

```json
{
  "action": "export",
  "format": "zip",
  "source": "project.xunbak",
  "destination": "project.zip",
  "dry_run": false,
  "selected": 123,
  "skipped": 4,
  "bytes_in": 123456,
  "bytes_out": 78901,
  "overwrite_count": 3,
  "verify_source": "quick",
  "verify_output": "on",
  "duration_ms": 1234,
  "outputs": ["project.zip"]
}
```

约束：

1. `--list --json` 返回 item 列表 + summary
2. `--dry-run --json` 返回 summary，不创建任何输出
3. 失败时仍返回已完成阶段信息和错误列表

**退出码与失败语义**：

| 场景 | 输出 | 退出码 |
|------|------|--------|
| preflight 失败（源 verify 失败、参数非法） | 不创建正式输出 | `2` |
| 写出失败 | 清理临时产物，正式输出不落地 | `1` |
| postflight 失败 | 保留输出，但标记 `verify_failed` | `1` |
| 成功 | 输出保留 | `0` |

细化：

1. `verify_output=on` 失败时，命令视为失败，但不自动删除已生成输出
2. `dir` 模式下若抽样/全量哈希校验失败，同样归类为 `verify_failed`
3. B3 路径下外部 `7z t` 失败默认计入 `verify_failed`，不降级为 warning

### 原子输出与失败清理

**统一规则**：

1. 永远先写临时目标，再切换为正式目标
2. 导出失败时尽量清理临时产物
3. 首版不支持 resume，但必须保证失败后不污染正式目标

**具体策略**：

| 格式 | 策略 |
|------|------|
| zip | 写 `name.tmp.zip`，成功后 rename 为正式 `name.zip` |
| 7z 单卷 | 写 `name.tmp.7z`，成功后 rename 为正式 `name.7z` |
| 7z 分卷 | 写 `name.tmp.7z.001/.002/...`，全部成功后批量 rename |
| dir | 写到临时目录，成功后按 `overwrite` 策略切换/合并 |

**`dir` 模式 overwrite 语义**：

| 策略 | 语义 |
|------|------|
| `fail` | 目标存在即拒绝 |
| `replace` | 目标不存在时原子 rename；目标已存在时改为 staged mirror + replace，**不承诺严格原子** |
| `ask` | 先 preview，再由用户确认 |

> Windows 上目录替换不是简单的原子 rename；文档必须明确这一限制。

**示例**：

```bash
# 直接备份为单卷 7z
xun backup -C ./project --format 7z -o project.7z

# 直接备份为分卷 7z
xun backup -C ./project --format 7z -o project.7z --split-size 2g

# 直接备份为 non-solid 7z，只导出 src/**/*.rs
xun backup -C ./project --format 7z -o src.7z --glob "src/**/*.rs"

# 直接备份为加密 7z
xun backup -C ./project --format 7z -o secure.7z --password "***" --encrypt-header
```

**阶段建议**：

1. B2.1：单卷 `.7z`，non-solid，LZMA2，目录/文件 + 时间戳/属性
2. B2.2：`SegmentedWriter`，支持 `--split-size`
3. B2.3：solid、加密、自定义方法链

### `--format dir` 的语义与复用关系

`dir` 是一种**backup/export 输出格式**，不是“restore 的别名命令”。  
但其底层实现不需要重复造轮子，应该**复用现有 restore 内核**：

1. 复用现有路径选择：`--file` / `--glob`
2. 复用现有 preview / confirm / path_guard 规则
3. 复用现有按 `blob_offset` 排序恢复读取计划
4. 复用现有 Windows 元数据恢复能力

换言之，`dir_writer.rs` 更像是：

```text
export facade -> restore core
```

而不是另一套独立 writer。

**`dir` 与现有 `restore` 的差异表**：

| 维度 | `restore` | `backup --format dir` |
|------|-----------|------------------------------|
| 目的 | 回到工作目录/恢复快照内容 | 把 `.xunbak` 导出到独立目录 |
| 默认目标 | 当前项目根或 `--to` | 必须显式 `-o/--output` |
| 原位恢复 | 允许 | **不建议**，默认拒绝导出到源工作目录 |
| `--snapshot` | 支持 | 不支持 |
| preview/confirm | 支持 | 复用 |
| sidecar | 无 | 可选，默认开启 |
| 退出码/JSON | restore 语义 | export 语义 |

**可行性评估**：

| 维度 | 评估 |
|------|------|
| 技术难度 | 低（B1）/ 低-中（B2）/ 低（B3） |
| 工作量 | ~500 行 Rust（B1）/ ~800-1500 行 Rust（B2，参考 `sevenz-rust2` 自研最小 `.7z` 写出）/ ~200-400 行（B3，subprocess + 探测） |
| 依赖 | 参考 `zip2-master` / `sevenz-rust2-main` / `7z2600-src` 后在仓库内手写实现（B1/B2），或用户系统中的 7-Zip（B3，自动探测 `7z.exe` / `7za.exe` / `7zr.exe`） |
| 分发 | 内置于 xun CLI |
| 维护 | 低 |
| 局限 | 需要运行 xun CLI，不能在 7-Zip 中直接打开 .xunbak |

**优势**：
1. 实现最简单
2. 不依赖 7-Zip 版本
3. 可导出为任意标准格式

**劣势**：
1. 需要用户主动执行导出
2. 导出文件是全量副本（占双份空间）
3. 丢失 .xunbak 的增量/去重/verify 能力

> **推荐顺序**：先做 B1（参考 `zip2-master` 手写 ZIP 输出），再做 B2（参考 `sevenz-rust2` / `7z2600-src` 手写纯 Rust `.7z` 写入）；B3 仅作为低成本 fallback，不建议作为长期唯一方案。B 全链路纯 Rust，无 C/C++ 工具链依赖。

---

> 本文后续仅保留 **A/B 主线方案**。维护 7-Zip fork 或修改 `.xunbak` 本体格式不再进入路线讨论。

---

## 5. 功能损失对比

| 功能 | .xunbak 原生 | 方案 A（7z 插件） | 方案 B（export） |
|------|-------------|------------------|----------------|
| 浏览文件列表 | xun CLI | 7-Zip GUI | 标准工具 |
| 解压单文件 | xun backup restore --file | 7-Zip 拖拽 | 标准工具 |
| 解压全部 | xun backup restore | 7-Zip 解压到 | 标准工具 |
| 增量更新 | 原生支持 | 不支持 | 不支持 |
| verify 校验 | 原生支持 | 不支持 | 不支持 |
| compact 压缩 | 原生支持 | 不支持 | 不支持 |
| 去重 | 原生支持 | 只读，不影响 | 导出后丢失 |
| 崩溃恢复 | 原生支持 | 不适用 | 不适用 |
| 分卷浏览 | 原生支持 | 可扩展 | 可导出为分卷 7z / zip |
| 文件属性 | win_attributes | 可显示 | 取决于导出格式 |
| 时间精度 | 纳秒 | 100ns（FILETIME） | 取决于格式 |

---

## 6. 方案 A 实现草案（Rust）

### 6.1 项目结构

```text
crates/xunbak-7z/
├── Cargo.toml              # cdylib, 依赖 xun (workspace)
├── xunbak.def              # DLL 导出定义（可选，也可用 #[no_mangle]）
├── src/
│   ├── lib.rs              # DLL 入口 + 导出函数
│   ├── com/
│   │   ├── mod.rs
│   │   ├── vtable.rs       # #[repr(C)] COM vtable 定义
│   │   ├── unknown.rs      # IUnknown: QueryInterface / AddRef / Release
│   │   ├── stream.rs       # IInStream / ISequentialOutStream 包装
│   │   └── variant.rs      # PROPVARIANT 辅助函数
│   ├── handler.rs          # XunbakHandler: IInArchive 实现
│   ├── registry.rs         # 格式注册：GUID / 签名 / 属性
│   └── extract.rs          # Extract 适配层（调用 xun::xunbak::reader）
└── tests/
    └── integration.rs      # 加载 DLL + 调用 CreateObject 测试
```

### 6.2 Cargo.toml

```toml
[package]
name = "xunbak-7z"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
xun = { path = "../..", features = ["xunbak"] }   # 复用 xunbak 模块
```

### 6.3 COM vtable 定义（核心）

```rust
// src/com/vtable.rs

use std::ffi::c_void;

type HRESULT = i32;
type ULONG = u32;

/// GUID — 与 C++ 内存布局完全一致
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

/// IUnknown vtable
#[repr(C)]
pub struct IUnknownVtbl {
    pub query_interface: unsafe extern "system" fn(
        this: *mut c_void, riid: *const GUID, ppv: *mut *mut c_void,
    ) -> HRESULT,
    pub add_ref: unsafe extern "system" fn(this: *mut c_void) -> ULONG,
    pub release: unsafe extern "system" fn(this: *mut c_void) -> ULONG,
}

/// IInArchive vtable — 继承 IUnknown 后追加 10 个方法
#[repr(C)]
pub struct IInArchiveVtbl {
    // IUnknown (3 methods)
    pub unknown: IUnknownVtbl,
    // IInArchive (10 methods)
    pub open: unsafe extern "system" fn(...) -> HRESULT,
    pub close: unsafe extern "system" fn(...) -> HRESULT,
    pub get_number_of_items: unsafe extern "system" fn(...) -> HRESULT,
    pub get_property: unsafe extern "system" fn(...) -> HRESULT,
    pub extract: unsafe extern "system" fn(...) -> HRESULT,
    pub get_archive_property: unsafe extern "system" fn(...) -> HRESULT,
    pub get_number_of_properties: unsafe extern "system" fn(...) -> HRESULT,
    pub get_property_info: unsafe extern "system" fn(...) -> HRESULT,
    pub get_number_of_archive_properties: unsafe extern "system" fn(...) -> HRESULT,
    pub get_archive_property_info: unsafe extern "system" fn(...) -> HRESULT,
}
```

### 6.4 DLL 导出函数

```rust
// src/lib.rs

use std::ffi::c_void;

/// 7-Zip 调用此函数创建 IInArchive 实例
#[no_mangle]
pub unsafe extern "system" fn CreateObject(
    clsid: *const GUID,
    iid: *const GUID,
    out_object: *mut *mut c_void,
) -> HRESULT {
    // 校验 CLSID == XUNBAK_HANDLER_GUID
    // 创建 XunbakHandler 实例
    // 调用 QueryInterface 设置 out_object
}

/// 返回支持的格式数量
#[no_mangle]
pub unsafe extern "system" fn GetNumberOfFormats(
    num_formats: *mut u32,
) -> HRESULT {
    *num_formats = 1;
    0 // S_OK
}

/// 返回格式属性（按索引，推荐入口）
#[no_mangle]
pub unsafe extern "system" fn GetHandlerProperty2(
    format_index: u32,
    prop_id: u32,
    value: *mut PROPVARIANT,
) -> HRESULT {
    match prop_id {
        0x00 => set_variant_bstr(value, "xunbak"),      // kName
        0x01 => set_variant_guid(value, &HANDLER_GUID), // kClassID
        0x03 => set_variant_bstr(value, "xunbak"),      // kExtension
        0x09 => set_variant_bytes(value, b"XUNBAK\0\0"),// kSignature
        // ...
        _ => 0,
    }
}

/// 旧版回退（单格式 DLL 可直接委托给 GetHandlerProperty2）
#[no_mangle]
pub unsafe extern "system" fn GetHandlerProperty(
    prop_id: u32,
    value: *mut PROPVARIANT,
) -> HRESULT {
    GetHandlerProperty2(0, prop_id, value)
}

/// 返回签名快速检测函数（可选，正式发布推荐）
#[no_mangle]
pub unsafe extern "system" fn GetIsArc(
    format_index: u32,
    is_arc: *mut Option<unsafe extern "C" fn(*const u8, usize) -> u32>,
) -> HRESULT {
    if format_index != 0 { return 1; } // E_INVALIDARG
    *is_arc = Some(xunbak_is_arc);
    0
}

unsafe extern "C" fn xunbak_is_arc(data: *const u8, size: usize) -> u32 {
    if size >= 8 && std::slice::from_raw_parts(data, 8) == b"XUNBAK\0\0" {
        0 // k_IsArc_Res_YES
    } else {
        2 // k_IsArc_Res_NO
    }
}
```

### 6.5 Handler 核心逻辑

```rust
// src/handler.rs — 伪代码

use xun::xunbak::manifest::{ManifestBody, ManifestEntry};
use xun::xunbak::blob::{BlobHeader, read_blob_record};
use xun::xunbak::header::Header;
use xun::xunbak::footer::Footer;
use xun::xunbak::checkpoint::CheckpointPayload;
use xun::xunbak::constants::{RECORD_PREFIX_SIZE, BLOB_HEADER_SIZE};

pub struct XunbakHandler {
    ref_count: AtomicU32,
    stream: Option<InStreamAdapter>,          // 7-Zip 传入的 IInStream 包装
    manifest: Option<ManifestBody>,
    checkpoint: Option<CheckpointPayload>,
    /// 缓存每个 entry 的纯压缩数据大小（从 BlobHeader.stored_size 读取）
    packed_sizes: Vec<u64>,
}

impl XunbakHandler {
    // Open: 从 IInStream 读取 header/footer/checkpoint/manifest
    // 注意：不能调用 ContainerReader::open()，因为它要求文件路径
    fn open(&mut self, stream: *mut c_void) -> HRESULT {
        let mut adapter = InStreamAdapter::wrap(stream);
        // 1. 读 header bytes → Header::from_bytes()
        // 2. seek 到末尾读 footer → Footer::from_bytes()
        // 3. seek 到 checkpoint_offset 读 checkpoint
        // 4. seek 到 manifest_offset 读 manifest
        // 5. 计算 packed_sizes:
        //    推荐：seek 到 entry.blob_offset + RECORD_PREFIX_SIZE,
        //        读 BlobHeader, 取 stored_size
        //    常量来自 xun::xunbak::constants，不要硬编码数字
        self.stream = Some(adapter);
        0 // S_OK
    }

    // GetNumberOfItems: manifest.entries.len()
    fn get_number_of_items(&self) -> u32 { ... }

    // GetProperty: 从 manifest entry 返回属性
    fn get_property(&self, index: u32, prop_id: u32, value: *mut PROPVARIANT) {
        let entry = &self.manifest.entries[index];
        match prop_id {
            kpidPath     => set_bstr(value, &entry.path),
            kpidSize     => set_u64(value, entry.size),
            kpidPackSize => set_u64(value, self.packed_sizes[index]),
            kpidMTime    => set_filetime(value, entry.mtime_ns),
            kpidCTime    => set_filetime(value, entry.created_time_ns),
            kpidAttrib   => set_u32(value, entry.win_attributes),
            // ...
        }
    }

    // Extract: seek → read record prefix → read blob → decompress → output
    // 注意：indices 数组必须升序（7-Zip 约定）
    fn extract(&self, indices: &[u32], callback: &ExtractCallback) {
        for &idx in indices {
            let entry = &self.manifest.entries[idx];
            // seek 到 blob_offset, 读 record prefix + blob header + compressed data
            // 调用 xun::xunbak::blob::read_blob_record() 解压
            // 通过 callback.GetStream() 获取输出流
            // 写入解压后的数据
        }
    }
}
```

> **与 `ContainerReader` 的关系**：Handler 不使用 `ContainerReader::open()`（它绑定文件路径）。
> 而是直接调用底层模块（`Header::from_bytes`、`read_manifest_record`、`read_blob_record` 等），
> 这些函数接受 `impl Read` / `&[u8]` 参数，无需路径。`ContainerReader` 的流式重构是可选优化，
> 非 PoC 阻塞项。

### 6.6 IInStream 适配

7-Zip 传入 `IInStream*`（COM 接口），需要包装为 Rust 的 `Read + Seek`：

```rust
// src/com/stream.rs

/// 将 7-Zip 的 IInStream COM 指针包装为 Rust Read + Seek
pub struct InStreamAdapter {
    raw: *mut c_void,       // IInStream* 裸指针
    vtbl: *const InStreamVtbl,
}

impl std::io::Read for InStreamAdapter {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut processed: u32 = 0;
        let hr = unsafe {
            ((*self.vtbl).read)(self.raw, buf.as_mut_ptr(), buf.len() as u32, &mut processed)
        };
        if hr != 0 { return Err(...); }
        Ok(processed as usize)
    }
}

impl std::io::Seek for InStreamAdapter {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let mut new_pos: u64 = 0;
        let (offset, origin) = match pos { ... };
        let hr = unsafe {
            ((*self.vtbl).seek)(self.raw, offset, origin, &mut new_pos)
        };
        Ok(new_pos)
    }
}
```

### 6.7 编译与分发

```bash
# 编译
cargo build -p xunbak-7z --release

# 产出
target/release/xunbak_7z.dll  (~500KB-1MB)

# 安装
copy target\release\xunbak_7z.dll "C:\Program Files\7-Zip\Formats\xunbak.dll"

# 验证
7z.exe i
# 输出中应出现：
#   xunbak  xunbak  XUNBAK..
```

### 6.8 测试策略

```rust
// tests/integration.rs

#[test]
fn dll_exports_exist() {
    let lib = libloading::Library::new("target/release/xunbak_7z.dll").unwrap();
    unsafe {
        // 最小必需集
        let _: libloading::Symbol<fn(*mut u32) -> i32> =
            lib.get(b"GetNumberOfFormats").unwrap();
        let _: libloading::Symbol<fn(u32, u32, *mut c_void) -> i32> =
            lib.get(b"GetHandlerProperty2").unwrap();
        let _: libloading::Symbol<fn(*const c_void, *const c_void, *mut *mut c_void) -> i32> =
            lib.get(b"CreateObject").unwrap();
        // 兼容推荐集
        let _: libloading::Symbol<fn(u32, u32, *mut c_void) -> i32> =
            lib.get(b"GetHandlerProperty").unwrap();
        let _: libloading::Symbol<fn(u32, *mut *mut c_void) -> i32> =
            lib.get(b"GetIsArc").unwrap();
    }
}

#[test]
fn get_number_of_formats_returns_one() {
    let lib = load_dll();
    let mut count: u32 = 0;
    let hr = unsafe { get_number_of_formats(&mut count) };
    assert_eq!(hr, 0);
    assert_eq!(count, 1);
}
```

**版本矩阵测试**：正式发布前需在以下版本验证 DLL 加载和基本功能：

| 7-Zip 版本 | 备注 |
|-----------|------|
| 24.08 | 当前广泛使用版本 |
| 24.09 | |
| 26.00 | 开发参考版本 |

### 6.9 A 正式版架构：C++ 薄壳 + Rust staticlib

PoC 验证通过后，正式版推荐将 COM 适配层迁移到 C++，Rust 侧仅导出窄 C ABI。

**为什么迁移到 C++ 壳**：
- COM vtable 是 C++ 原生概念，无需 `#[repr(C)]` 手工对齐
- PROPVARIANT 操作、IInStream/ISequentialOutStream 回调在 C++ 中是标准做法
- 7-Zip 接口头文件（IArchive.h、PropID.h 等）可直接 `#include`
- Rust 不碰任何 COM 细节，只暴露 5-6 个 C 函数
- 7-Zip 接口变更由 C++ 壳吸收，Rust 侧 C ABI 不变

**项目结构**：

```text
crates/xunbak-7z/
├── rust/
│   ├── Cargo.toml            # crate-type = ["staticlib"]
│   ├── src/lib.rs            # #[no_mangle] extern "C" fn 导出
│   └── cbindgen.toml         # 自动生成 xunbak_ffi.h
├── cpp/
│   ├── CMakeLists.txt        # 构建 xunbak.dll，链接 Rust staticlib
│   ├── plugin/               # 7-Zip 接口头文件（IArchive.h 等）
│   ├── xunbak_handler.h/cpp  # IInArchive COM 实现（~400 行）
│   ├── xunbak_exports.cpp    # CreateObject / GetHandlerProperty2 等导出
│   └── xunbak_ffi.h          # Rust staticlib 的 C 头文件（cbindgen 生成）
└── tests/
    └── integration.rs
```

**C ABI 边界定义**（Rust staticlib 导出）：

```c
// xunbak_ffi.h — 由 cbindgen 从 Rust 源码生成

typedef struct XunbakArchive XunbakArchive;

/// 从内存 buffer 打开（C++ 层从 IInStream 读取完整文件后传入）
int32_t xunbak_open(const uint8_t *data, size_t len, XunbakArchive **out);

/// 关闭并释放
void xunbak_close(XunbakArchive *archive);

/// 文件总数
uint32_t xunbak_item_count(const XunbakArchive *archive);

/// 获取第 index 项的属性
/// prop_id: 0=path(UTF-16), 1=size, 2=packed_size, 3=mtime_ns, 4=ctime_ns, 5=win_attributes
int32_t xunbak_get_property(const XunbakArchive *archive, uint32_t index,
                            uint32_t prop_id, void *out_buf, size_t buf_len,
                            size_t *out_written);

/// 解压第 index 项到 caller 提供的 buffer
int32_t xunbak_extract(const XunbakArchive *archive, uint32_t index,
                       uint8_t *out_buf, size_t buf_len, size_t *out_written);

/// 获取第 index 项的原始大小（用于预分配 buffer）
int32_t xunbak_item_size(const XunbakArchive *archive, uint32_t index,
                         uint64_t *out_size);
```

**C++ 壳核心逻辑**（伪代码）：

```cpp
// xunbak_handler.cpp — COM 适配层

class XunbakInArchive : public IInArchive, public CMyUnknownImp {
    XunbakArchive* archive_ = nullptr;
    std::vector<uint8_t> file_data_;  // IInStream 读入的完整文件

public:
    STDMETHOD(Open)(IInStream* stream, const UInt64*, IArchiveOpenCallback*) override {
        // 1. 从 IInStream 读取完整文件到 file_data_
        // 2. 调用 xunbak_open(file_data_.data(), file_data_.size(), &archive_)
        return S_OK;
    }

    STDMETHOD(GetProperty)(UInt32 index, PROPID propID, PROPVARIANT* value) override {
        // 将 7-Zip PROPID 映射到 xunbak prop_id
        // 调用 xunbak_get_property(archive_, index, ...)
        // 将结果写入 PROPVARIANT
        return S_OK;
    }

    STDMETHOD(Extract)(const UInt32* indices, UInt32 numItems,
                       Int32 testMode, IArchiveExtractCallback* cb) override {
        for (UInt32 i = 0; i < numItems; i++) {
            uint64_t size;
            xunbak_item_size(archive_, indices[i], &size);
            // 通过 cb->GetStream() 获取输出流
            // 调用 xunbak_extract(archive_, indices[i], ...)
            // 写入输出流
        }
        return S_OK;
    }

    STDMETHOD(Close)() override {
        if (archive_) { xunbak_close(archive_); archive_ = nullptr; }
        return S_OK;
    }
    // ... GetNumberOfItems, GetArchiveProperty 等委托给 C ABI
};
```

**构建流程**：

```bash
# 1. 编译 Rust staticlib
cargo build -p xunbak-7z-core --release
# 产出: target/release/xunbak_7z_core.lib

# 2. cbindgen 生成头文件
cbindgen --crate xunbak-7z-core -o cpp/xunbak_ffi.h

# 3. CMake 编译 C++ DLL，链接 Rust staticlib
cmake -B build -S cpp
cmake --build build --config Release
# 产出: build/Release/xunbak.dll (~300-500KB)
```

**PoC → 正式版迁移路径**：

| 维度 | PoC（纯 Rust cdylib） | 正式版（C++ 壳 + Rust staticlib） |
|------|----------------------|-------------------------------|
| 构建 | `cargo build` 一步出 DLL | CMake + cargo，构建脚本稍复杂 |
| COM 适配 | `#[repr(C)]` 手写 vtable，风险较高 | C++ 原生，7-Zip 头文件直接 `#include` |
| 维护 | 7-Zip 接口变更需 Rust 侧同步 vtable 定义 | C++ 壳吸收接口变化，Rust 侧 C ABI 不变 |
| DLL 体积 | ~500KB-1MB（含 Rust runtime） | ~300-500KB（C++ 壳薄，Rust 静态链接） |
| 目的 | 快速验证"DLL 能被 7-Zip 加载并列出文件" | 长期可发布产品 |

---

## 7. 推荐路线图

```text
Phase 1（当前）
  └── .xunbak 核心功能完成（写入/恢复/verify/增量）

Phase 2
  └── 方案 B1（优先）：xun backup --format zip
      └── 参考 zip2-master，在 XunYu 内手写标准 ZIP 输出（当前已实现 stored/deflated，后续扩到 bzip2/zstd/ppmd）

Phase 3
  ├── 方案 B2（正式路线）：xun backup --format 7z
  │   ├── 参考 refer/sevenz-rust2-main，自研纯 Rust `.7z` 写出模块
  │   ├── B2.1：单卷 `.7z`，最小子集：LZMA2 + non-solid + 文件/目录 + 时间戳/属性
  │   ├── B2.2：`SegmentedWriter` + `--split-size`，输出 `.7z.001/.002/...`
  │   └── 由 xun CLI 统一封装调用
  │
  └── 方案 B3（过渡 / fallback）：
      └── 自动探测 7z.exe / 7za.exe / 7zr.exe

Phase 4
  └── 方案 A PoC：crates/xunbak-7z（只读 DLL）
      ├── 当前已具备：单文件 + 分卷 .xunbak 打开、list、extract
      ├── 无前置重构：直接调用底层模块，不走 ContainerReader::open
      ├── 纯 Rust core + C++ 薄壳：handler / export / InStream 适配 / volume callback 已打通
      ├── 支持 zstd + none codec
      ├── 当前已实现 list + extract 核心功能
      └── 当前已实现基础 item/archive property，后续继续扩充显示列

Phase 5（PoC 验证通过后）
  ├── 方案 A 正式版：
  │   ├── 推荐壳层：C++ 薄壳 + Rust staticlib
  │   ├── 分卷 .xunbak 支持（IArchiveOpenVolumeCallback）
  │   ├── ContainerReader 流式重构 open_from_stream（可选，若有其他需求驱动）
  │   ├── lz4 codec
  │   ├── 富属性（GetArchivePropertyInfo 完整实现）
  │   └── 版本矩阵扩展验证
  └── 若 A 风险过高，则继续保留 B1/B2 为主线
```

### 7.1 A PoC 的替代/降级路线

如果 PoC 过程中发现纯 Rust COM DLL 在 vtable 布局或流回调上问题过多，可考虑以下替代：

| 替代方案 | 描述 | 优劣 |
|---------|------|------|
| **C++ 薄壳 + Rust staticlib** | C++ 只写 CreateObject / IInArchive / PROPVARIANT 胶水层，Rust 通过 `staticlib` 或窄 C ABI 暴露 `xunbak_open` / `xunbak_list` / `xunbak_extract` | 7-Zip COM 适配更稳（C++ 原生），但引入双工具链构建复杂度 |
| **Shell verb / 右键命令** | 注册 Windows 右键菜单，执行 `xun backup --format zip ...` 后自动用 7-Zip 打开 | 用户体验接近，工程风险远小于格式 DLL，但多一步导出 |

> 这些替代方案在 PoC 阶段暂不启动，仅作为 fallback 记录。

---

## 8. 7z-assembly 参考实现要点

`refer/7z-assembly-master` 是一个完整的 C++ 格式插件参考，核心模式：

### 8.1 项目结构

```text
7z-assembly-master/
├── CMakeLists.txt                    # CMake 构建
├── src/
│   ├── 7z-assembly.h/cc             # DLL 导出 + 处理器注册（~103 行）
│   ├── utils.h/cc                   # PROPVARIANT 辅助函数
│   ├── plugin/                      # 7-Zip 接口头文件（从 7-Zip 源码复制）
│   │   ├── IArchive.h               # IInArchive / IOutArchive 定义
│   │   ├── IStream.h                # 流接口
│   │   ├── MyCom.h                  # COM 基类宏
│   │   ├── 7zTypes.h                # 基础类型
│   │   └── PropID.h                 # 属性 ID 枚举
│   └── archive/
│       ├── sze-archive.h/cc         # SZE 格式处理器（~220 行）
│       └── sze-reader.h             # 文本流解析器
```

### 8.2 处理器注册模式

```cpp
// 处理器数组
ArchiveHandler handlers[] = {
    {
        L"SZE",                          // 格式名
        SzeHandlerGuid,                  // GUID
        L"sze",                          // 扩展名
        L"",                             // 附加扩展名
        NArcInfoFlags::kByExtOnlyOpen,   // 行为标志
        {0x53, 0x45},                    // 签名字节
        true,                            // 支持更新
        []() { return new SzeInArchive(); }  // 工厂函数
    }
};
```

### 8.3 核心接口实现

SZE 处理器实现了 `IInArchive`（读取）和 `IOutArchive`（写入），核心方法：

- `Open()` — 读签名、逐条解析 `{filename|size}content` 格式
- `Extract()` — 遍历索引，通过 callback 的 `GetStream()` 获取输出流，写入内容
- `GetProperty()` — 按 `PROPID` 返回 path / size 等
- `UpdateItems()` — 通过 callback 获取新内容，写入输出流

### 8.4 对 Rust 实现的启示

| 7z-assembly 中的 C++ 模式 | Rust 等价实现 |
|--------------------------|-------------|
| `CMyUnknownImp` 基类 + 引用计数宏 | `AtomicU32` ref_count + 手动 AddRef/Release |
| `Z7_IFACES_IMP_UNK_2(IInArchive, IOutArchive)` | `#[repr(C)]` vtable struct + 函数指针 |
| `PROPVARIANT` 设置辅助函数 | `set_variant_bstr()` / `set_variant_u64()` |
| `std::vector<File>` | `Vec<ManifestEntry>`（直接复用 xunbak manifest） |
| `plugin/` 目录下的接口头文件 | `com/vtable.rs` 中 `#[repr(C)]` 翻译 |

**关键不同：** 7z-assembly 的 `Open()` 需要自己解析格式；Rust 版调用 xunbak 底层模块（`Header::from_bytes`、`read_manifest_record`、`read_blob_record` 等），但**不能直接调用 `ContainerReader::open()`**——后者绑定文件路径，而 7-Zip 只给 `IInStream*` 流指针。

---

## 9. 许可与分发合规

> **来源**：[7-Zip FAQ](https://www.7-zip.org/faq.html)

### 9.1 方案 A（格式插件 DLL）

`xunbak.dll` 设计上**不包含任何 7-Zip 源码**——它仅实现 COM 接口约定（vtable 布局），与 7-Zip 通过 `LoadLibrary` + `GetProcAddress` 动态交互。因此：

- **预计可独立许可**，不受 7-Zip LGPL 约束
- ⚠️ 但 7-Zip FAQ 对"把 7-Zip 代码包装进 DLL"明确要求 LGPL 义务。虽然本方案不复制/改写 7-Zip 代码，仅参照其头文件定义接口布局，但边界可能存在争议。**发布前需做一次法务确认**
- 用户需自行安装 7-Zip（我们不分发 7-Zip 二进制）
- 安装说明应注明："需要 7-Zip 24.08 或更高版本"

### 9.2 方案 B（export 命令）

| 导出格式 | 依赖 | 许可义务 |
|---------|------|---------|
| zip | 参考 `zip2-master`，仓库内手写实现 | 无新增外部二进制义务 |
| 7z（仓库内自研） | 参考 `sevenz-rust2` / `7z2600-src`，纯 Rust 自有实现 | 无新增外部二进制义务 |
| 7z（调用用户已安装的 7-Zip） | 自动探测 `7z.exe` / `7za.exe` / `7zr.exe` | 重新分发义务较低；仍建议保留 attribution / compliance notice，并在发布前法务确认 |
| 7z（随产品分发 7-Zip 二进制） | ⚠️ 需遵守 LGPL | 必须：(1) 声明使用 7-Zip (2) 说明 GNU LGPL (3) 提供 [7-zip.org](https://www.7-zip.org) 链接 |

**决定**：方案 B 全链路纯 Rust，无 C/C++ 工具链依赖。推荐落地顺序：先 B1（参考 `zip2-master` 手写 ZIP 输出），再 B2（参考 `sevenz-rust2` / `7z2600-src` 手写 `.7z` 输出）；B3（自动探测 `7z.exe` > `7za.exe` > `7zr.exe`）仅作为过渡 fallback。C/C++ 仅在方案 A 正式版中作为 COM 薄壳出现（见 §6.9）。

---

## 10. 参考来源

1. [7z-assembly](https://github.com/ikremniou/7z-assembly) — 开源 7-Zip 格式插件教程（SZE 格式，本地副本 `refer/7z-assembly-master`）
2. [TC4Shell 7-Zip 插件集](https://www.tc4shell.com/en/7zip/) — Asar7z / Lzip7z / Forensic7z 等商业插件
3. [7-Zip-zstd](https://github.com/mcmilk/7-Zip-zstd) — zstd/brotli/lz4 codec 插件
4. 7-Zip 26.00 源码 `CPP/7zip/Archive/IArchive.h` — IInArchive 接口定义
5. 7-Zip 26.00 源码 `CPP/7zip/Common/RegisterArc.h` — 格式注册宏
6. 7-Zip 26.00 源码 `CPP/7zip/Archive/ArchiveExports.cpp` — 全局注册表和 DLL 导出
7. [Kenny Kerr — Creating your first DLL in Rust](https://kennykerr.ca/rust-getting-started/creating-your-first-dll.html)
8. [MSRC Blog — Designing a COM Library for Rust](https://msrc.microsoft.com/blog/2019/10/designing-a-com-library-for-rust/)
9. [windows crate](https://docs.rs/crate/windows/latest) — 微软官方 Rust Windows API crate
10. [CLR Profiler in Rust](https://github.com/camdenreslink/clr-profiler) — Rust 手动 COM vtable 实现参考
11. [7-Zip 官方 FAQ](https://www.7-zip.org/faq.html)
12. [DeepWiki 7-Zip Developer Guide](https://deepwiki.com/ip7z/7zip/6-developer-guide)
13. [7-Zip 下载页](https://www.7-zip.org/download.html)
14. `refer/sevenz-rust2-main` — 本地参考实现（纯 Rust `.7z` 读写）
15. [sevenz-rust2](https://github.com/hasenbanck/sevenz-rust2) — 上游纯 Rust `.7z` 读写库
16. [sevenz-rust2 API docs](https://docs.rs/sevenz-rust2) — `ArchiveWriter` / `ArchiveReader` / `compress_to_path` API


