# Backup 优化路线图

> 面向当前 XunYu `backup` 体系的下一阶段优化建议。
> 覆盖传统 `backup`、`.xunbak`、7-Zip 插件、恢复链路、校验链路。

---

## 1. 当前状态

### 1.0 实现原则

后续 `backup` 优化统一采用：

1. **优先参考同类型项目**
2. **在 XunYu 内手写相关功能**

也就是说：

1. `zip2-master`
2. `sevenz-rust2-main`
3. `7z2600-src`
4. `7-Zip-zstd-master`
5. `7z-assembly-master`

这些项目优先作为**参考来源**，而不是长期直接依赖的最终实现。

这样做的目的：

1. 保持方法集、兼容性策略、诊断与 doctor 能力都掌握在 XunYu 自己代码中
2. 避免长期被外部 crate API 和 feature 绑定
3. 允许按 XunYu 自己的产品目标裁剪行为

当前 `backup` 体系已经具备：

1. 统一命令域：
   - `backup`
   - `backup create`
   - `backup restore`
   - `backup convert`
   - `backup list`
   - `backup verify`
   - `backup find`
2. 多种产物格式：
   - `dir`
   - `zip`
   - `7z`
   - `xunbak`
3. 传统 `backup` 的哈希驱动增量：
   - `.bak-manifest.json` 作为权威真相
   - `hash_cache` 作为性能优化
   - `diff-mode=auto|hash|meta`
4. `.xunbak` 7-Zip 只读插件 PoC：
   - 单文件可打开
   - 分卷可打开
   - `7-Zip l/x` 可联调
   - 脚本层已具备 `build / install / uninstall / smoke / portable / system / accept` 工具链
5. 导出方法当前状态：
   - ZIP 已实现 `stored / deflated`
   - 7z 已实现 `copy / lzma2`

这意味着现在的重点已经不是“补功能空白”，而是：

1. 提升大文件与大目录性能
2. 降低安装与使用门槛
3. 补齐体验型和运维型功能
4. 提升可发布性与长期维护性

---

## 2. 高优先级优化

### 2.1 7-Zip 插件安装器与文件关联

#### 当前问题

当前 `.xunbak` 7-Zip DLL 已经可以工作，但仍然依赖：

1. 手工复制 `xunbak.dll` 到 `7-Zip/Formats`
2. 手工给 `.xunbak` 配置文件关联

补充说明：

1. **脚本层安装/卸载已经存在**
2. 缺的是统一 CLI 入口与 doctor/关联管理

这会导致：

1. 用户感觉“明明做了 DLL，但双击不一定行”
2. 环境迁移成本高
3. 排障成本高

#### 建议

补一套正式命令或脚本：

```text
xun xunbak plugin install
xun xunbak plugin uninstall
xun xunbak plugin doctor
```

能力包括：

1. 自动探测 7-Zip 安装目录
2. 复制/删除 `xunbak.dll`
3. 检查 DLL 是否存在
4. 检查 `.xunbak` 是否已关联到 `7zFM.exe`
5. 输出环境诊断报告

#### 价值

这是把“PoC 可用”提升为“用户真正可用”的第一步。

---

### 2.2 优化 7-Zip 插件的大文件打开路径

#### 当前问题

在 C++ 插件层的 `OpenCore()` 中，若 callback 打开失败，会退回：

1. 读取整个流到内存
2. 再调用 Rust 的 `xunbak_open(...)`

这意味着：

1. 大 `.xunbak` 打开时内存峰值高
2. 首次打开延迟可能明显增加
3. 分卷 / 大容器的扩展性受限

#### 建议

优先级很高的改造是：

1. 尽量始终走 `xunbak_open_with_callbacks`
2. 避免 fallback 到整文件 `ReadAll`
3. 如果必须 fallback，则加文件大小阈值
4. 对大文件直接给出明确错误，而不是隐式吃内存

#### 相关位置

1. [xunbak_handler.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_handler.cpp)
2. [lib.rs](/D:/100_Projects/110_Daily/XunYu/crates/xunbak-7z-core/src/lib.rs)

#### 价值

这是 `.xunbak` 7-Zip 预览链路里最值得先做的性能优化。

---

### 2.3 传统 backup 接入真实 `file_id`

#### 当前问题

当前哈希增量已经成立，但 rename-only 场景仍主要靠内容 hash 命中。  
基线也已经显示：

1. 热缓存很快
2. 冷缓存仍然贵
3. `file_id` 还未真正消费

#### 建议

在 Windows 上补真实 `file_id` 采集，并接入：

1. `scan.rs`
2. `hash_cache`
3. `diff`
4. rename-only 识别

#### 价值

1. 减少 rename-only 场景的哈希重算
2. 提升冷路径增量性能
3. 增强 Windows-first 语义正确性

---

### 2.4 手写 ZIP / 7z 方法集扩展

#### 当前问题

当前多格式导出在方法集上仍是保守实现：

1. ZIP 只实现了 `stored / deflated`
2. 7z 只实现了 `copy / lzma2`

而当前产品目标已经收敛为：

1. ZIP：`stored / deflated / bzip2 / zstd / ppmd`
2. 7z：`copy / lzma2 / bzip2 / deflate / ppmd / zstd`

#### 建议

按“参考同类型项目，XunYu 内手写实现”的原则推进：

1. ZIP：
   - 参考 `zip2-master`
   - 在 XunYu 内手写 ZIP writer backend
2. 7z：
   - 参考 `sevenz-rust2-main` + `7z2600-src`
   - 在 XunYu 内手写 7z writer / method routing

#### 价值

1. 让方法集能力与产品目标一致
2. 避免长期被外部库的 API 和 feature 集绑定
3. 让兼容矩阵、doctor、verify、sidecar 语义都掌握在 XunYu 自己的代码中

---

## 3. 中优先级优化

### 3.1 插件属性列与归档级显示增强

#### 当前问题

当前插件已经实现了最小可用的 `IInArchive` 子集，但 GUI 侧展示还可以继续增强。

#### 建议

继续补强：

1. 文件数
2. 卷数
3. 物理大小
4. packed size
5. method / codec
6. split / volume 提示

#### 相关位置

1. [xunbak_handler.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_handler.cpp)
2. [xunbak_exports.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_exports.cpp)

#### 价值

让 `.xunbak` 在 7-Zip 里看起来更像“原生格式”，而不是“只能打开的自定义格式”。

---

### 3.2 恢复路径的顺序读取与 reader 复用

#### 当前问题

恢复链路已经功能完整，但仍有进一步优化空间：

1. 多文件恢复时可减少重复 open
2. `.xunbak` / `7z` 多文件恢复时可进一步优化顺序读
3. preview 阶段可减少不必要内容读取

#### 建议

1. 批量恢复时复用 reader
2. 优化多文件恢复的读取顺序
3. 为 preview 增加“属性级快速路径”

#### 相关位置

1. [restore.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/app/restore.rs)
2. [reader.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/artifact/reader.rs)

---

### 3.3 传统 backup 报告继续增强

#### 当前已有

当前已输出：

1. `new / modified / reused / deleted`
2. `hash_checked_files`
3. `hash_cache_hits`
4. `hash_computed_files`
5. `hardlinked_files`

#### 建议继续补

1. rename-only count
2. reused bytes
3. cache hit ratio
4. baseline source
5. diff mode
6. copied logical ratio

#### 相关位置

1. [backup.rs](/D:/100_Projects/110_Daily/XunYu/src/commands/backup.rs)
2. [meta.rs](/D:/100_Projects/110_Daily/XunYu/src/backup/legacy/meta.rs)

---

### 3.4 `verify` 分级模式

#### 当前问题

当前 `verify` 已能做完整性校验，但粒度仍偏单一。

#### 建议

补：

1. quick
2. full
3. manifest-only
4. existence-only

并增强错误输出：

1. 首个错误路径
2. 卷号
3. item 名称
4. packed / unpacked 失败原因

---

## 4. 低优先级优化

### 4.1 `.xunbak` 安装器体验完善

在已有脚本基础上补：

1. Release 包装
2. 版本号
3. 卸载残留清理
4. 多 7-Zip 安装目录选择

---

### 4.2 分卷打开的懒加载优化

继续减少：

1. 首次探测卷时的开销
2. footer / checkpoint 重复读取
3. 多卷 reader 重复 seek

#### 相关位置

1. [lib.rs](/D:/100_Projects/110_Daily/XunYu/crates/xunbak-7z-core/src/lib.rs)

---

### 4.3 导出链路元数据对齐

在 `dir / zip / 7z / xunbak` 互转时继续统一：

1. mtime / ctime
2. readonly / hidden
3. sidecar manifest
4. packed size / codec 展示

---

## 5. 性能优化重点结论

### 5.1 传统 backup

从当前基线看：

1. 热缓存 hash diff 已接近 metadata diff
2. 冷缓存 hash diff 仍明显更贵
3. 全量模式收益主要来自 hardlink

因此最值得继续优化的是：

1. `file_id`
2. 冷路径 hash
3. scan / diff 的命中率

### 5.2 7-Zip 插件

最关键的性能点不是 `GetProperty`，而是：

1. `OpenCore()` 是否整文件读入内存
2. 分卷 callback 是否高效
3. extract 是否真正流式

---

## 6. 建议执行顺序

如果只做 3 件事，建议按这个顺序：

### 第 1 阶段

1. 插件安装器 + 文件关联
2. 插件 doctor / 自检

### 第 2 阶段

1. 去掉 7-Zip 插件的大文件 `ReadAll` fallback
2. 做 Release 构建与版本矩阵

### 第 3 阶段

1. 传统 backup 接入真实 `file_id`
2. 补更多冷缓存性能基线

---

## 7. 总结

当前 `backup` 体系已经进入“从可用走向好用”的阶段。  
最有价值的下一步，不是继续横向扩功能，而是：

1. 降低 `.xunbak` 7-Zip 的安装与使用门槛
2. 降低大容器预览的内存与打开成本
3. 继续压缩传统 backup 冷路径增量成本

如果这三件事做完，整体体验会比现在再上一个台阶。
