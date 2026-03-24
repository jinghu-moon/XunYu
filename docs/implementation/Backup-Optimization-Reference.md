# Backup 优化参考文档

> 目的：基于本地参考项目与 XunYu 当前实现，整理一份可直接指导 `backup` 后续优化工作的参考文档。
> 范围：覆盖 ZIP 导出、7z 导出、`.xunbak` 7-Zip 插件、传统 `backup` 哈希增量、恢复与校验链路。
> 说明：本文所有结论都附带来源。来源优先使用本地参考项目与当前仓库代码；必要时引用本地参考项目中已经指向的上游规范/实现语义。
> 当前决策：**采用方案 C**，即 ZIP 不承担 `lzma2`，`lzma2` 只作为 7z 的正式支持算法。

---

## 1. 参考范围与用途

### 1.0 使用原则

本文对应的实现原则是：

1. **优先参考同类型项目**
2. **在 XunYu 内手写相关功能**

因此这些参考项目的角色是：

1. 提供 method id、行为边界、格式细节、兼容路径参考
2. 指导 XunYu 自己的 ZIP / 7z / 插件实现
3. 不默认等同于“直接作为长期唯一实现依赖”

### 1.1 本地参考项目清单

| 参考项目 | 本地位置 | 主要用途 |
|---|---|---|
| `zip` / `zip-rs` | [zip2-master](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master) | ZIP 方法支持、写出行为、方法号、Zip64 与多盘边界 |
| `sevenz-rust2` | [sevenz-rust2-main](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main) | 7z 编码器能力、方法链、feature 开关、method id |
| 7-Zip 官方源码 26.00 | [7z2600-src](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src) | 官方 method id、ZIP/7z 方法映射、内建/外部 codec 边界 |
| LZMA SDK 26.00 | [lzma2600](/D:/100_Projects/110_Daily/XunYu/refer/lzma2600) | LZMA / LZMA2 / PPMD 规范与 SDK 能力边界 |
| 7-Zip-zstd | [7-Zip-zstd-master](/D:/100_Projects/110_Daily/XunYu/refer/7-Zip-zstd-master) | 7z 外部 codec 的 ZSTD / Brotli / LZ4 / Lizard 方法 ID 与兼容实现 |
| 7z-assembly | [7z-assembly-master](/D:/100_Projects/110_Daily/XunYu/refer/7z-assembly-master) | 7-Zip 插件系统、`IInArchive` 与动态格式插件实现参考 |
| XunYu 当前实现 | [src/backup](/D:/100_Projects/110_Daily/XunYu/src/backup), [crates/xunbak-7z-core](/D:/100_Projects/110_Daily/XunYu/crates/xunbak-7z-core), [cpp/xunbak-7z-plugin](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin) | 现状与优化切入点 |

来源：

1. [zip2-master/Cargo.toml](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/Cargo.toml)
2. [sevenz-rust2-main/Cargo.toml](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/Cargo.toml)
3. [7z2600-src/DOC/Methods.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/Methods.txt)
4. [7-Zip-zstd-master/DOC/Methods-Extern.md](/D:/100_Projects/110_Daily/XunYu/refer/7-Zip-zstd-master/DOC/Methods-Extern.md)
5. [7z-assembly-master/docs/plugin-system.md](/D:/100_Projects/110_Daily/XunYu/refer/7z-assembly-master/docs/plugin-system.md)
6. [Backup-Optimization-Roadmap.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Backup-Optimization-Roadmap.md)

### 1.2 源码阅读优先级（按手写实现视角）

如果目标是“参考同类型项目，在 XunYu 内手写相关功能”，建议按下面顺序阅读：

#### 第一层：格式与方法号

1. ZIP：
   - [refer/zip2-master/src/compression.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/compression.rs)
   - [refer/zip2-master/src/spec.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/spec.rs)
   - [refer/zip2-master/src/types.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/types.rs)
2. 7z：
   - [refer/7z2600-src/DOC/Methods.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/Methods.txt)
   - [refer/7z2600-src/DOC/7zFormat.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/7zFormat.txt)
   - [refer/sevenz-rust2-main/src/archive.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/archive.rs)

#### 第二层：写出路径

1. ZIP：
   - [refer/zip2-master/src/write.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/write.rs)
2. 7z：
   - [refer/sevenz-rust2-main/src/writer.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/writer.rs)
   - [refer/sevenz-rust2-main/src/encoder.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/encoder.rs)
   - [refer/sevenz-rust2-main/src/encoder_options.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/encoder_options.rs)

#### 第三层：插件与兼容

1. 插件接口：
   - [refer/7z-assembly-master/docs/plugin-system.md](/D:/100_Projects/110_Daily/XunYu/refer/7z-assembly-master/docs/plugin-system.md)
   - [refer/7z-assembly-master/docs/plugin-api-def.md](/D:/100_Projects/110_Daily/XunYu/refer/7z-assembly-master/docs/plugin-api-def.md)
   - [refer/7z-assembly-master/docs/plugin-api-inarc.md](/D:/100_Projects/110_Daily/XunYu/refer/7z-assembly-master/docs/plugin-api-inarc.md)
2. 扩展 codec：
   - [refer/7-Zip-zstd-master/DOC/Methods-Extern.md](/D:/100_Projects/110_Daily/XunYu/refer/7-Zip-zstd-master/DOC/Methods-Extern.md)

#### 第四层：XunYu 现状落点

1. ZIP 当前实现：
   - [src/backup/artifact/zip.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/artifact/zip.rs)
   - [src/backup/app/create.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/app/create.rs)
   - [src/backup/app/convert.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/app/convert.rs)
2. 7z 当前实现：
   - [src/backup/artifact/sevenz.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/artifact/sevenz.rs)
   - [src/backup/artifact/sevenz_segmented.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/artifact/sevenz_segmented.rs)
3. 插件当前实现：
   - [crates/xunbak-7z-core/src/lib.rs](/D:/100_Projects/110_Daily/XunYu/crates/xunbak-7z-core/src/lib.rs)
   - [cpp/xunbak-7z-plugin/xunbak_handler.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_handler.cpp)
   - [cpp/xunbak-7z-plugin/xunbak_exports.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_exports.cpp)

这套顺序的含义是：

1. 先确认“格式能不能这样表达”
2. 再确认“同类项目是怎么写出来的”
3. 再确认“插件/兼容层怎么接上”
4. 最后把这些映射回 XunYu 现有模块

---

## 2. XunYu 当前实现现状

### 2.1 ZIP 导出当前只开放 `stored | deflated`

当前 XunYu 自己对 ZIP 导出的 CLI 方法限制是：

1. `backup create --format zip` 只接受 `stored` 或 `deflated`
2. `backup convert --format zip` 只接受 `stored` 或 `deflated`
3. 内部 `ZipCompressionMethod` 目前也只有：
   - `Auto`
   - `Stored`
   - `Deflated`

这意味着当前代码层的 ZIP 能力明显弱于 `zip-rs` 本身的能力边界。

来源：

1. [src/backup/artifact/zip.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/artifact/zip.rs)
2. [src/backup/app/create.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/app/create.rs)
3. [src/backup/app/convert.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/app/convert.rs)
4. [src/backup/artifact/options.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/artifact/options.rs)

### 2.2 7z 导出当前只开放 `copy | lzma2`

当前 XunYu 对 7z 导出的 CLI 方法限制是：

1. `backup create --format 7z` 只接受 `copy` 或 `lzma2`
2. `backup convert --format 7z` 只接受 `copy` 或 `lzma2`
3. `SevenZMethod` 枚举当前只有：
   - `Copy`
   - `Lzma2`

这意味着当前代码层的 7z 能力也明显弱于 `sevenz-rust2` 所证明的上限。

来源：

1. [src/backup/artifact/sevenz.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/artifact/sevenz.rs)
2. [src/backup/app/create.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/app/create.rs)
3. [src/backup/app/convert.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/app/convert.rs)
4. [src/backup/artifact/options.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/artifact/options.rs)

### 2.3 `.xunbak` 7-Zip 插件已经是“C++ 薄壳 + Rust 核心”

当前插件的结构不是纯脚本或纯 Rust COM，而是：

1. Rust 核心用 `staticlib` 输出稳定 C ABI
2. C++ 侧实现 7-Zip `IInArchive` 与导出函数
3. 分卷通过 callback 桥接回 Rust
4. 脚本层已具备 build / install / uninstall / smoke / portable / system / accept 工具链

这意味着后续插件优化应继续沿着“**C++ 只做 ABI 适配，Rust 只做容器解析**”这个方向，不要再反向耦合。

来源：

1. [crates/xunbak-7z-core/Cargo.toml](/D:/100_Projects/110_Daily/XunYu/crates/xunbak-7z-core/Cargo.toml)
2. [crates/xunbak-7z-core/src/lib.rs](/D:/100_Projects/110_Daily/XunYu/crates/xunbak-7z-core/src/lib.rs)
3. [cpp/xunbak-7z-plugin/CMakeLists.txt](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/CMakeLists.txt)
4. [cpp/xunbak-7z-plugin/xunbak_exports.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_exports.cpp)
5. [cpp/xunbak-7z-plugin/xunbak_handler.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_handler.cpp)
6. [scripts/build_xunbak_7z_plugin.ps1](/D:/100_Projects/110_Daily/XunYu/scripts/build_xunbak_7z_plugin.ps1)
7. [scripts/install_xunbak_7z_plugin.ps1](/D:/100_Projects/110_Daily/XunYu/scripts/install_xunbak_7z_plugin.ps1)
8. [scripts/uninstall_xunbak_7z_plugin.ps1](/D:/100_Projects/110_Daily/XunYu/scripts/uninstall_xunbak_7z_plugin.ps1)

---

## 3. ZIP 参考结论

### 3.0 手写 ZIP 的总体原则

基于方案 C 与当前参考，XunYu 的 ZIP 优化应遵循：

1. 参考 `zip2-master` 的方法号、header、central directory、Zip64、extra field 语义
2. 在 XunYu 内手写自己的 ZIP writer 层
3. 不把 `zip2-master` 当长期唯一输出实现

对 XunYu 的直接意义：

1. `src/backup/artifact/zip.rs` 未来更像一个“XunYu 自己的 ZIP backend”
2. 参考项目提供的是**可对照的行为与格式细节**，不是最终交付物

来源：

1. [refer/zip2-master/src/spec.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/spec.rs)
2. [refer/zip2-master/src/write.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/write.rs)
3. [refer/zip2-master/src/types.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/types.rs)

### 3.1 `zip-rs` 能力上限高于 XunYu 当前 ZIP 实现

`zip-rs` / `zip2-master` 当前的公开能力矩阵表明：

1. 写入支持：
   - `Stored`
   - `Deflate`
   - `Bzip2`
   - `ZStandard`
   - `XZ`
   - `PPMd`
2. 读取支持：
   - `LZMA`
   - `PPMd`
   - `Bzip2`
   - `ZStandard`
   - 其它若干方法

但要注意：**“枚举存在”不等于“写入一定支持”**，必须再看写入实现本身。

来源：

1. [refer/zip2-master/README.md](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/README.md)
2. [refer/zip2-master/src/compression.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/compression.rs)
3. [refer/zip2-master/Cargo.toml](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/Cargo.toml)

补充含义：

1. `CompressionMethod` 是方法号与方法语义的权威对照入口
2. XunYu 若手写 ZIP 方法支持，第一步应先复制/映射方法枚举语义，而不是先写 writer

### 3.2 `zip-rs` 对 ZIP-LZMA 是“可读，不可写”

这是当前最重要的一个交叉结论：

1. `CompressionMethod::Lzma` 在枚举和方法号里存在
2. 但写入路径里明确返回：
   - `LZMA isn't supported for compression`
3. 测试侧只看到 `tests/lzma.rs` 的解压测试，没有对应的写入成功测试

所以从 `zip2-master` 现状看：

1. ZIP-LZMA 的**读取**有证据
2. ZIP-LZMA 的**写入**没有正向支持

来源：

1. [refer/zip2-master/src/compression.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/compression.rs)
2. [refer/zip2-master/src/write.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/write.rs)
3. [refer/zip2-master/tests/lzma.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/tests/lzma.rs)

补充含义：

1. 如果 XunYu 未来想做 ZIP-LZMA，也不能简单照搬 `zip2-master` 的写入路径
2. 但方案 C 已明确：当前不做 ZIP-LZMA / ZIP-LZMA2，避免把精力投到非主路径

### 3.3 `zip-rs` 对 ZIP-PPMD / ZIP-BZIP2 / ZIP-ZSTD 有正向写入证据

从源码和测试可以确认：

1. `PPMd` 有写入实现
2. `Bzip2` 有写入实现
3. `Zstd` 有写入实现

其中 `PPMd` 还有显式压缩测试：

1. `tests/ppmd.rs` 中不仅解压，也直接用 `ZipWriter` 写入 `CompressionMethod::Ppmd`
2. `write.rs` 中 `Bzip2` / `Zstd` / `Ppmd` 都有明确 writer 分支

所以如果只看 `zip2-master`，XunYu 将 ZIP 方法集扩到：

1. `stored`
2. `deflated`
3. `bzip2`
4. `zstd`
5. `ppmd`

是有充分参考依据的。

来源：

1. [refer/zip2-master/src/write.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/write.rs)
2. [refer/zip2-master/tests/ppmd.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/tests/ppmd.rs)
3. [refer/zip2-master/src/compression.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/compression.rs)
4. [refer/zip2-master/README.md](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/README.md)

补充含义：

1. 这几个方法是当前最适合直接进入 XunYu 手写 ZIP backend 的目标
2. 尤其 `Bzip2 / Zstd / Ppmd`，已经有“方法号 + writer 分支 + 测试”的三重证据

### 3.4 ZIP-LZMA2 在当前参考集中没有标准化证据，因此按方案 C 排除

这是第二个必须明确写出的结论：

1. 7-Zip 官方 `Methods.txt` 的 ZIP 方法表里有：
   - `0E - LZMA (LZMA-zip)`
2. 同一份表中没有“ZIP-LZMA2”方法项
3. `zip2-master` 的方法枚举也只有 `Lzma`，没有 `Lzma2`

所以“**ZIP 支持 lzma2**”这个需求，和当前参考项目之间存在明显冲突。

当前已经采用 **方案 C**：

1. ZIP 不做 `lzma2`
2. `lzma2` 只作为 7z 的正式支持算法
3. ZIP 的正式目标方法集收敛为：
   - `stored`
   - `deflated`
   - `bzip2`
   - `zstd`
   - `ppmd`

因此后续实现时：

1. 不再把 ZIP-LZMA2 作为待实现项
2. 也不需要为了 ZIP 去引入一条缺乏标准方法号依据的兼容路径
3. 若未来要重新讨论 ZIP-LZMA，应作为一个新的独立需求评估，而不是从 `lzma2` 派生

来源：

1. [refer/7z2600-src/DOC/Methods.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/Methods.txt)
2. [refer/zip2-master/src/compression.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/compression.rs)
3. [refer/zip2-master/src/write.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/write.rs)
4. [refer/zip2-master/README.md](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/README.md)

### 3.5 ZIP 仍然不适合作为分卷主路径

`zip-rs` README 明确写了：

1. Multi-disk ZIP 当前不支持

因此无论方法集怎么扩，XunYu 当前都不应把 ZIP 作为分卷主路径。

对 XunYu 的意义：

1. `--split-size` 继续只对 `7z | xunbak` 生效是合理的
2. ZIP 仍应定位为“标准互操作导出格式”，不是大规模分卷备份的首选

来源：

1. [refer/zip2-master/README.md](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/README.md)
2. [src/backup/artifact/options.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/artifact/options.rs)

### 3.6 手写 ZIP 时最值得直接参考的源码点

如果要真正手写 ZIP backend，优先看这三类文件：

#### 方法与方法号

1. [refer/zip2-master/src/compression.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/compression.rs)

#### 结构与签名

1. [refer/zip2-master/src/spec.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/spec.rs)

这里给出：

1. local file header signature
2. central directory signature
3. end of central directory signature
4. Zip64 threshold 与相关 block

#### 写出路径

1. [refer/zip2-master/src/write.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/write.rs)
2. [refer/zip2-master/src/types.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/types.rs)

这里最值得参考的是：

1. `version_needed()` 如何随方法变化
2. `CompressionMethod` 与 writer 分支的关系
3. external attributes 与 system 标志的处理
4. Zip64 的触发条件

对 XunYu 的直接指导：

1. 先把 header / central directory / Zip64 三层结构在 XunYu 里写稳定
2. 再把方法 writer 插进去
3. 最后再做时间戳、权限位、sidecar 与 verify

来源：

1. [refer/zip2-master/src/spec.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/spec.rs)
2. [refer/zip2-master/src/write.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/write.rs)
3. [refer/zip2-master/src/types.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/types.rs)

---

## 4. 7z 参考结论

### 4.0 手写 7z 的总体原则

基于当前参考，XunYu 的 7z 优化应遵循：

1. 参考 `7z2600-src` 的格式结构与方法号
2. 参考 `sevenz-rust2-main` 的 writer 组织方式与 encoder 路由
3. 在 XunYu 内手写自己的 7z writer / split writer / reader 适配层

这意味着：

1. `sevenz-rust2-main` 是“如何组织纯 Rust 7z writer”的优秀参考
2. `7z2600-src` 是“method id 和格式结构”的官方基线
3. 两者要结合使用，而不是只看一个

来源：

1. [refer/7z2600-src/DOC/7zFormat.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/7zFormat.txt)
2. [refer/7z2600-src/DOC/Methods.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/Methods.txt)
3. [refer/sevenz-rust2-main/src/writer.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/writer.rs)
4. [refer/sevenz-rust2-main/src/archive.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/archive.rs)

### 4.1 `sevenz-rust2` 已证明 7z 方法集可扩到 `bzip2 / deflate / ppmd / zstd`

`sevenz-rust2` 的支持矩阵和 feature 开关显示：

1. 基础支持：
   - `COPY`
   - `LZMA`
   - `LZMA2`
2. 可选 feature 支持：
   - `BZIP2`
   - `DEFLATE`
   - `PPMD`
   - `ZSTD`
   - 还有 Brotli / LZ4 等

对 XunYu 的直接意义：

1. 你提出的 7z 目标方法集在参考实现层是“有证据支持的”
2. 当前 XunYu 只开放 `copy | lzma2`，属于实现保守，不是参考能力上限

来源：

1. [refer/sevenz-rust2-main/src/lib.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/lib.rs)
2. [refer/sevenz-rust2-main/Cargo.toml](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/Cargo.toml)

补充含义：

1. 当前 7z 目标方法集是有参考支撑的，不需要再从算法层发明
2. 重点转移到：
   - method id 正不正确
   - header / folder / streams 写得对不对
   - 解压端兼容性如何标注

### 4.2 `sevenz-rust2` 已给出这些方法的实际 method id

`archive.rs` 里可以直接看到这些 method id：

1. `LZMA2` = `[0x21]`
2. `PPMD` = `[0x03, 0x04, 0x01]`
3. `BZIP2` = `[0x04, 0x02, 0x02]`
4. `DEFLATE` = `[0x04, 0x01, 0x08]`
5. `ZSTD` = `[0x04, 0xF7, 0x11, 0x01]`

对 XunYu 的意义：

1. `LZMA2 / PPMD / BZIP2 / DEFLATE` 都有明确映射
2. `ZSTD` 也有明确映射，但它不是官方内建 method，而是外部 codec 路径

来源：

1. [refer/sevenz-rust2-main/src/archive.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/archive.rs)

补充含义：

1. `archive.rs` 可以当成 XunYu 自己 7z method registry 的直接参考
2. 当前若要手写 `SevenZMethod` 扩展，优先按这里的 method id 对齐

### 4.3 `sevenz-rust2` 的 encoder options 已具备方法级配置接口

`encoder_options.rs` 已经提供：

1. `Lzma2Options`
2. `Bzip2Options`
3. `DeflateOptions`
4. `PpmdOptions`
5. `ZstandardOptions`

并且可以转成 `EncoderConfiguration`。

对 XunYu 的意义：

1. 当前 `SevenZMethod` 只定义了 `Copy / Lzma2`，这只是包装层太窄
2. 后续可以继续按“方法枚举 + method_config 映射”扩，而不必推翻整个 writer 结构

来源：

1. [refer/sevenz-rust2-main/src/encoder_options.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/encoder_options.rs)
2. [refer/sevenz-rust2-main/src/writer.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/writer.rs)
3. [src/backup/artifact/sevenz.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/artifact/sevenz.rs)

补充含义：

1. XunYu 当前的 `method_config()` 只是一层很薄的包装
2. 后续完全可以按 `encoder_options.rs` 的类型体系扩展成：
   - `Bzip2Options`
   - `DeflateOptions`
   - `PpmdOptions`
   - `ZstandardOptions`

### 4.4 7z-ZSTD 的兼容性必须单独对待

这是 7z 方法集里最容易被误解的一点。

官方 7-Zip `Methods.txt` 显示：

1. `F7 11 01` 被保留给外部 codec 的 `ZSTD`

`7-Zip-zstd` 的 `Methods-Extern.md` 则说明：

1. `Zstandard` 使用的就是 `F7 11 01`
2. 这是外部 codec 插件路径
3. 它包含额外的 7z container header 约定

而 `sevenz-rust2` 的 `ID_ZSTD = [0x04, 0xF7, 0x11, 0x01]`，本质上就是沿着这个外部 codec 体系在走。

对 XunYu 的意义：

1. 7z `zstd` 可以做
2. 但不能默认等同于“**所有 stock 7-Zip 都天然支持**”
3. 需要把兼容性分成：
   - stock 7-Zip
   - 7-Zip-zstd
   - NanaZip
   - XunYu 自己的 reader

本文建议：

1. 7z `zstd` 进入正式目标方法集可以
2. 但实现文档与 doctor/verify 必须明确标注“兼容性取决于解压端 codec 支持”

来源：

1. [refer/7z2600-src/DOC/Methods.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/Methods.txt)
2. [refer/7-Zip-zstd-master/DOC/Methods-Extern.md](/D:/100_Projects/110_Daily/XunYu/refer/7-Zip-zstd-master/DOC/Methods-Extern.md)
3. [refer/sevenz-rust2-main/src/archive.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/archive.rs)

### 4.5 手写 7z 时最值得直接参考的源码点

#### 格式结构

1. [refer/7z2600-src/DOC/7zFormat.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/7zFormat.txt)

这里给出了：

1. signature header
2. start header
3. pack streams
4. folders
5. coders
6. files info

#### 方法号

1. [refer/7z2600-src/DOC/Methods.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/Methods.txt)
2. [refer/sevenz-rust2-main/src/archive.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/archive.rs)

#### 写出组织

1. [refer/sevenz-rust2-main/src/writer.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/writer.rs)
2. [refer/sevenz-rust2-main/src/encoder.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/encoder.rs)
3. [refer/sevenz-rust2-main/src/encoder_options.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/encoder_options.rs)

对 XunYu 的直接指导：

1. 先把最小 7z 结构写出来
2. 再按 method id 扩方法
3. 再把 `SegmentedWriter` 对齐到逻辑连续流
4. 最后再补 codec 兼容矩阵、doctor 和 verify

来源：

1. [refer/7z2600-src/DOC/7zFormat.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/7zFormat.txt)
2. [refer/7z2600-src/DOC/Methods.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/Methods.txt)
3. [refer/sevenz-rust2-main/src/writer.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/writer.rs)
4. [refer/sevenz-rust2-main/src/encoder.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/encoder.rs)
5. [refer/sevenz-rust2-main/src/encoder_options.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/encoder_options.rs)

---

## 5. 7-Zip 插件参考结论

### 5.0 手写插件的总体原则

`.xunbak` 7-Zip 插件的实现原则也应保持一致：

1. 参考 `7z-assembly-master` 的插件接口和宿主协议
2. 参考 7-Zip 官方 `Methods.txt` / 宿主行为
3. 在 XunYu 内保留“C++ 薄壳 + Rust 核心”的分层

换句话说：

1. C++ 不负责理解 `.xunbak` 格式
2. Rust 不负责模拟 7-Zip C++ 宿主接口
3. 两侧通过窄 C ABI 相连

来源：

1. [refer/7z-assembly-master/docs/plugin-system.md](/D:/100_Projects/110_Daily/XunYu/refer/7z-assembly-master/docs/plugin-system.md)
2. [cpp/xunbak-7z-plugin/xunbak_handler.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_handler.cpp)
3. [crates/xunbak-7z-core/src/lib.rs](/D:/100_Projects/110_Daily/XunYu/crates/xunbak-7z-core/src/lib.rs)

### 5.1 动态格式插件的放置位置和工作方式

`7z-assembly` 的插件系统文档明确说明：

1. 动态插件 DLL 放在 7-Zip 安装根的 `Formats/` 或 `Coders/`
2. 插件通过导出函数和 `IInArchive`/相关接口被宿主加载
3. File Manager 和命令行都会用这套插件机制

对 XunYu 的意义：

1. 现在 `.xunbak` 能被 7-Zip 打开，前提就是 `xunbak.dll` 放对位置
2. “做了 DLL 但双击不行”通常首先是安装/关联问题，不是解析逻辑问题

来源：

1. [refer/7z-assembly-master/docs/plugin-system.md](/D:/100_Projects/110_Daily/XunYu/refer/7z-assembly-master/docs/plugin-system.md)
2. [docs/implementation/Xunbak-7zip-Plugin-PoC.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Xunbak-7zip-Plugin-PoC.md)

### 5.2 `IInArchive` 最关键的是 `Open / GetProperty / Extract`

`7z-assembly` 对 `IInArchive` 的说明里，最核心的是：

1. `Open`
2. `GetNumberOfItems`
3. `GetProperty`
4. `Extract`
5. `GetArchiveProperty`
6. `GetPropertyInfo / GetArchivePropertyInfo`

对 XunYu 的意义：

1. 只读插件的第一优先级不是“实现更多导出函数”，而是把这几个核心点稳定做好
2. 当前 `.xunbak` 插件已经实现了最关键的核心路径，下一阶段更偏向性能和显示增强，而不是从零开始补接口

来源：

1. [refer/7z-assembly-master/docs/plugin-api-inarc.md](/D:/100_Projects/110_Daily/XunYu/refer/7z-assembly-master/docs/plugin-api-inarc.md)
2. [refer/7z-assembly-master/docs/plugin-api-def.md](/D:/100_Projects/110_Daily/XunYu/refer/7z-assembly-master/docs/plugin-api-def.md)
3. [cpp/xunbak-7z-plugin/xunbak_handler.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_handler.cpp)

### 5.3 手写插件时最值得直接参考的源码点

#### 插件宿主协议

1. [refer/7z-assembly-master/docs/plugin-system.md](/D:/100_Projects/110_Daily/XunYu/refer/7z-assembly-master/docs/plugin-system.md)
2. [refer/7z-assembly-master/docs/plugin-api-def.md](/D:/100_Projects/110_Daily/XunYu/refer/7z-assembly-master/docs/plugin-api-def.md)
3. [refer/7z-assembly-master/docs/plugin-api-inarc.md](/D:/100_Projects/110_Daily/XunYu/refer/7z-assembly-master/docs/plugin-api-inarc.md)

#### 当前 XunYu 插件实现

1. [cpp/xunbak-7z-plugin/xunbak_exports.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_exports.cpp)
2. [cpp/xunbak-7z-plugin/xunbak_handler.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_handler.cpp)
3. [crates/xunbak-7z-core/src/lib.rs](/D:/100_Projects/110_Daily/XunYu/crates/xunbak-7z-core/src/lib.rs)

对 XunYu 的直接指导：

1. C++ 侧继续只做 ABI 适配
2. Rust 侧继续只做容器解析与提取
3. 绝不把 `.xunbak` 内部格式逻辑重新写一份到 C++

来源：

1. [refer/7z-assembly-master/docs/plugin-api-inarc.md](/D:/100_Projects/110_Daily/XunYu/refer/7z-assembly-master/docs/plugin-api-inarc.md)
2. [cpp/xunbak-7z-plugin/xunbak_handler.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_handler.cpp)
3. [crates/xunbak-7z-core/src/lib.rs](/D:/100_Projects/110_Daily/XunYu/crates/xunbak-7z-core/src/lib.rs)

### 5.4 当前 `.xunbak` 插件已经实现了“属性列定义”

当前 C++ 插件并不是只实现了最简 `Open + Extract`，它已经有：

1. `GetArchiveProperty`
2. `GetNumberOfProperties`
3. `GetPropertyInfo`
4. `GetNumberOfArchiveProperties`
5. `GetArchivePropertyInfo`

因此后续优化不应再写成“从零补属性列”，而应写成：

1. 扩展现有属性列
2. 优化列值质量
3. 优化 GUI 展示与技术列表输出

来源：

1. [cpp/xunbak-7z-plugin/xunbak_handler.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_handler.cpp)
2. [docs/implementation/Xunbak-7zip-Plugin-PoC.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Xunbak-7zip-Plugin-PoC.md)

### 5.5 当前插件最大的性能热点是 `OpenCore()` 的整流读入 fallback

当前 `OpenCore()` 的关键行为是：

1. 先尝试 `xunbak_open_with_callbacks`
2. 如果失败，再 `ReadAll(stream, &primary_bytes)`
3. 再走 `xunbak_open(...)`

这意味着：

1. 大 `.xunbak` 在最坏路径下会整文件进内存
2. 这会抬高首开延迟和内存峰值

对 XunYu 的意义：

1. 插件性能优化的第一优先级不是 `GetProperty`
2. 而是收敛 `ReadAll` fallback
3. 需要优先保证 callback 路径稳定、并给大文件 fallback 加阈值/禁用策略

来源：

1. [cpp/xunbak-7z-plugin/xunbak_handler.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_handler.cpp)
2. [docs/implementation/Backup-Optimization-Roadmap.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Backup-Optimization-Roadmap.md)

---

## 6. 对 XunYu 后续优化的直接指导

### 6.1 ZIP 方法集建议

基于当前参考与方案 C，ZIP 目标方法集已经收敛为：

1. `stored`
2. `deflated`
3. `bzip2`
4. `zstd`
5. `ppmd`

来源：

1. [refer/zip2-master/README.md](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/README.md)
2. [refer/zip2-master/src/write.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/write.rs)
3. [refer/zip2-master/tests/ppmd.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/tests/ppmd.rs)

不进入 ZIP 目标方法集的：

1. `lzma2`

原因：

1. ZIP 方法表里只有 `LZMA-zip`
2. `zip2-master` 当前也没有 `Lzma2` 方法枚举
3. 当前产品决策已明确 `lzma2` 留给 7z

来源：

1. [refer/7z2600-src/DOC/Methods.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/Methods.txt)
2. [refer/zip2-master/src/write.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/write.rs)
3. [refer/zip2-master/src/compression.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/compression.rs)

补充含义：

1. 方案 C 让 ZIP 侧的复杂度明显下降
2. 这意味着手写 ZIP 时，不必为了 `lzma2` 设计额外兼容层
3. XunYu 可以把 ZIP 侧工作集中在：
   - `stored`
   - `deflated`
   - `bzip2`
   - `zstd`
   - `ppmd`

### 6.2 7z 方法集建议

7z 侧可以直接把目标方法集定义为：

1. `copy`
2. `lzma2`
3. `bzip2`
4. `deflate`
5. `ppmd`
6. `zstd`

但要区分两类兼容性：

#### stock 7-Zip 内建方法优先

1. `copy`
2. `lzma2`
3. `bzip2`
4. `deflate`
5. `ppmd`

这些方法都能在当前参考里找到稳定 method id。

来源：

1. [refer/7z2600-src/DOC/Methods.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/Methods.txt)
2. [refer/sevenz-rust2-main/src/archive.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/archive.rs)

#### 扩展 codec 路径

1. `zstd`

应在实现和文档里明确：

1. `zstd` 需要依赖解压端对外部 codec 的支持
2. 对 stock 7-Zip 不应默认承诺“无条件可解”
3. 需要配套兼容矩阵、doctor、验证脚本

来源：

1. [refer/7-Zip-zstd-master/DOC/Methods-Extern.md](/D:/100_Projects/110_Daily/XunYu/refer/7-Zip-zstd-master/DOC/Methods-Extern.md)
2. [refer/sevenz-rust2-main/src/archive.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/archive.rs)

### 6.3 `.xunbak` 插件优化顺序建议

最合理的顺序是：

1. 安装器 / uninstall / doctor
2. 去掉大文件 `ReadAll` fallback
3. 完善 GUI 属性列
4. 固化 Release 构建与兼容矩阵

原因：

1. 现在插件已经“能用”，第一问题是“怎么稳定地让用户用上”
2. 第二问题才是“大容器怎么更快”
3. 第三问题才是“显示更漂亮”

来源：

1. [docs/implementation/Xunbak-7zip-Plugin-PoC.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Xunbak-7zip-Plugin-PoC.md)
2. [docs/implementation/Backup-Optimization-Roadmap.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Backup-Optimization-Roadmap.md)
3. [cpp/xunbak-7z-plugin/xunbak_handler.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_handler.cpp)

### 6.4 传统 backup 优化顺序建议

传统 `backup` 侧最值得优先做的是：

1. 真实 `file_id`
2. 冷缓存 hash 降本
3. 恢复路径 reader 复用
4. verify 分级模式

原因：

1. 基线已经说明热缓存足够快，冷缓存才是主要矛盾
2. `file_id` 是最直接改善 rename-only 与冷路径命中的抓手
3. 恢复和 verify 优化属于第二层收益

来源：

1. [docs/implementation/Traditional-Backup-Hash-Incremental-Benchmarks.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Traditional-Backup-Hash-Incremental-Benchmarks.md)
2. [docs/implementation/Backup-Optimization-Roadmap.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Backup-Optimization-Roadmap.md)
3. [src/backup/legacy/scan.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/legacy/scan.rs)
4. [src/backup/app/restore.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/app/restore.rs)

---

## 7. 最终结论

### 7.1 当前最重要的技术判断

1. **ZIP：正式目标方法集为 `stored / deflated / bzip2 / zstd / ppmd`，不包含 `lzma2`**
2. **7z：`lzma2 / bzip2 / deflate / ppmd` 可直接推进，`zstd` 需要显式标注 codec 兼容性**
3. **`.xunbak` 插件当前最大的性能问题是 `OpenCore()` 的整流读入 fallback**
4. **传统 `backup` 当前最大的性能问题仍是冷缓存 hash 路径，而不是热路径**

来源：

1. [refer/zip2-master/README.md](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/README.md)
2. [refer/zip2-master/src/write.rs](/D:/100_Projects/110_Daily/XunYu/refer/zip2-master/src/write.rs)
3. [refer/7z2600-src/DOC/Methods.txt](/D:/100_Projects/110_Daily/XunYu/refer/7z2600-src/DOC/Methods.txt)
4. [refer/7-Zip-zstd-master/DOC/Methods-Extern.md](/D:/100_Projects/110_Daily/XunYu/refer/7-Zip-zstd-master/DOC/Methods-Extern.md)
5. [refer/sevenz-rust2-main/src/archive.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/archive.rs)
6. [cpp/xunbak-7z-plugin/xunbak_handler.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_handler.cpp)
7. [docs/implementation/Traditional-Backup-Hash-Incremental-Benchmarks.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Traditional-Backup-Hash-Incremental-Benchmarks.md)

### 7.2 对后续工作的直接建议

1. ZIP 侧直接按已收敛方法集推进：
   - `bzip2`
   - `zstd`
   - `ppmd`
2. 7z 侧继续扩到：
   - `bzip2`
   - `deflate`
   - `ppmd`
   - `zstd`
3. 7z `zstd` 在实现时要从一开始就连同“兼容矩阵 / doctor / 文档提示”一起做
4. 插件与传统 backup 的优化应并行推进，但优先级顺序不同：
   - 插件优先“安装器 + 打开性能”
   - 传统 backup 优先“file_id + 冷缓存”

来源：

1. [docs/implementation/Backup-Optimization-Roadmap.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Backup-Optimization-Roadmap.md)
2. [docs/implementation/Xunbak-7zip-Compat.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Xunbak-7zip-Compat.md)
3. [docs/implementation/Backup-Optimization-Tasks.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Backup-Optimization-Tasks.md)
