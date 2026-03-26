# `.xunbak` Codec 扩展设计

> 日期：2026-03-25
> 范围：`.xunbak` 容器自身的 blob 级压缩算法扩展
> 目标：明确“值得加入 `.xunbak` 的 codec 集合、边界、优先级和最小实现路径”

---

## 1. 结论

值得加进 `.xunbak` 的 codec：

1. `none`
2. `zstd`
3. `deflate`
4. `bzip2`
5. `ppmd`
6. `lzma2`

不建议为了“和 `zip / 7z` 看齐”而引入的能力：

1. archive 级 method id
2. solid 压缩
3. 多文件跨边界共享字典

核心原因：

1. `.xunbak` 是增量、去重、可选择恢复的容器
2. 对 `.xunbak` 真正有价值的是“codec 能力”，不是 `zip / 7z` 的容器语义
3. 一旦引入 solid 或跨文件共享字典，会直接伤害 blob 独立恢复、去重复用和增量更新

---

## 2. 当前状态

当前 `.xunbak` 在代码层的真实状态是：

1. `NONE / ZSTD / LZ4 / DEFLATE / BZIP2 / PPMD / LZMA2` 已实现并可用
2. `compress / decompress / stream_hash_and_compress / copy_decompressed_to_writer` 已统一进入 codec backend
3. `backup create / convert / restore` 与 `xunbak -> dir/zip/7z` 主链已覆盖扩展 codec
4. `.xunbak` 7-Zip core 已覆盖扩展 codec 提取与未知 codec 错误路径
5. `auto` 已不再退化为 `zstd`：
   - 文本型扩展优先 `PPMD`
   - 其他可压缩内容优先 `ZSTD`
   - 不可压缩内容仍回退 `NONE`
6. 版本策略已落地：
   - `none / zstd` 继续写 `min_reader_version = 1`
   - 新 codec 写 `min_reader_version = 2`

关键代码位置：

1. codec 枚举：[constants.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/constants.rs)
2. codec 实现：[codec.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/codec.rs)
3. blob 写入 / 读取 / 流式复制：[blob.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/blob.rs)
4. writer 侧压缩入口：[writer.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/writer.rs)

---

## 3. 边界定义

这次扩展的目标是：

1. 让 `.xunbak` 的每个 blob 可以使用更多压缩算法
2. 保持每个 blob 独立压缩、独立校验、独立恢复
3. 保持 manifest / reader / verify 对 codec 的理解一致

这次扩展的非目标是：

1. 让 `.xunbak` 兼容 `zip` 或 `7z` 容器格式
2. 引入 `zip / 7z` 的 method id 体系
3. 引入 solid 压缩或 archive 级方法链
4. 为了“格式矩阵完整”而接受明显不适合 `.xunbak` 的容器语义

---

## 4. 为什么是 codec，而不是容器语义

`.xunbak` 的结构本质上是：

1. append-only 容器
2. blob 级内容去重
3. manifest 驱动的路径映射
4. 选择性恢复与校验优先

因此它更像“内容寻址的 pack 容器”，而不是传统归档格式。

对这种模型而言：

1. `codec` 决定单个 blob 如何压缩
2. 容器语义负责组织 blob、manifest、checkpoint、footer
3. 这两件事应该严格分层

也就是说：

1. 可以借用 `zip / 7z` 里已经验证过的压缩算法
2. 但不应该把 `zip / 7z` 的 archive method 语义直接搬进 `.xunbak`

---

## 5. 产品价值判断

从产品价值看，最值得的 codec 是：

1. `zstd`
   - 继续作为默认值
   - 速度和压缩率平衡最好

2. `ppmd`
   - 适合作为文本型高压缩比选项
   - 对源码、JSON、配置、日志这类内容更有意义

3. `lzma2`
   - 适合作为高压缩率但慢速的归档选项
   - 更像“长期归档模式”，不适合作为默认值

相对而言，`deflate / bzip2` 的价值较低：

1. 它们可以做
2. 但更多是“算法矩阵完整”
3. 不是 `.xunbak` 的最佳默认方向

### 5.1 当前推荐矩阵

| 场景 | 推荐 codec | 原因 | 当前依据 |
| --- | --- | --- | --- |
| 日常备份默认值 | `zstd` | 速度、压缩率、兼容性平衡最好 | 默认写 `min_reader_version = 1`，CLI 默认值保持 `zstd` |
| 文本 / 源码 / JSON / 日志 | `ppmd` | 文本型高压缩选项，适合显式选择或 `auto` 文本路由 | 已补回归测试，文本语料上 `PPMD` 具备明确压缩收益；收益不足仍会回退 `NONE` |
| 极致速度优先 | `lz4` | create / restore 更偏吞吐 | 当前基线 `backup_100_files_lz4 = 38.94 ms`，快于默认 `43.79 ms` |
| 长期归档 / 高压缩率 | `lzma2` | 压缩率导向，但明显更慢 | 当前基线 `compress_lzma2_1mb = 109.8 ms` |
| 兼容 / 方法矩阵补齐 | `deflate` / `bzip2` | 可以支持，但不应宣传为默认值 | 当前设计定位为“兼容/矩阵补齐”，不是主推模式 |

`auto` 的定位保持轻量，不承担复杂试压：

1. 文本型扩展优先 `PPMD`
2. 其他可压缩内容优先 `ZSTD`
3. 收益不足时回退 `NONE`

---

## 6. 与 `LZ4` 规划的关系

当前任务清单里已经单独加入了 `LZ4` backlog。两者不冲突：

1. `LZ4` backlog 面向“极速压缩/解压模式”
2. 本文面向 `.xunbak` codec 扩展的总体设计边界
3. `LZ4` 可以作为并行规划，但不应该掩盖 `ppmd / lzma2` 在产品定位上的长期价值

因此可以这样理解：

1. 短期实现优先级：`LZ4`
2. 长期产品价值优先级：`zstd` 默认、`ppmd` 文本高压缩、`lzma2` 慢速高压缩

---

## 7. 最小实现路径

### 7.1 扩展 `Codec` 枚举

在 [constants.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/constants.rs) 中增加：

1. `DEFLATE`
2. `BZIP2`
3. `PPMD`
4. `LZMA2`

要求：

1. 继续保持 `u8` 编码
2. 保持未知 codec 可安全表示，不 panic
3. manifest / blob header / verify 使用统一 codec 编号

### 7.2 实现统一 codec 层

在 [codec.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/codec.rs) 中补齐：

1. `compress(codec, data, level)`
2. `decompress(codec, data)`
3. `stream_hash_and_compress(reader, codec, level, chunk_size)`

要求：

1. 所有 codec 都从统一入口进入
2. `write_blob_record()` 与 `prepare_blob_record()` 不直接绑定具体算法实现
3. 保持“压缩收益不足则回退 `NONE`”的统一策略

### 7.3 补齐 blob 流式复制路径

在 [blob.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/blob.rs) 中同步补齐：

1. `read_blob_record()` 的解压路径
2. `copy_blob_record_content_to_writer()` 的流式解压路径

这是必要步骤，因为：

1. `write_blob_record()` 能写，不代表恢复链路就能读
2. `.xunbak` 的核心优势之一是“边读边恢复”，不能只做一次性解压

### 7.4 版本与兼容策略

引入新 codec 后，旧 reader 不一定能读取新容器，因此必须明确：

1. 是否提升 `min_reader_version`
2. 旧 reader 遇到新 codec 时的报错策略
3. `verify` 和 `restore` 的错误信息如何提示用户升级

相关位置：

1. header 版本校验：[header.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/header.rs)
2. 版本常量：[constants.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/constants.rs)

当前已采用的最小兼容策略：

1. `write_version` 维持 `1`
2. `reader_version` 提升到 `2`
3. 仅当容器实际使用扩展 codec 时，才写 `min_reader_version = 2`
4. `none / zstd` 产物继续保持 `min_reader_version = 1`

兼容矩阵：

| 容器实际使用的 codec | `min_reader_version` | 旧 reader 行为 | 新 reader 行为 |
| --- | ---: | --- | --- |
| `none` / `zstd` | `1` | 可继续读取 | 可读取 |
| `lz4` / `deflate` / `bzip2` / `ppmd` / `lzma2` | `2` | 明确拒绝并提示升级 | 可读取 |

---

## 8. 推荐实现顺序

### 第一批

1. `LZ4`
2. `PPMD`

原因：

1. `LZ4` 能补上极速模式
2. `PPMD` 能补上文本型高压缩模式
3. 这两者的产品定位最清晰、互补性最强

### 第二批

1. `LZMA2`

原因：

1. 作为“高压缩率、低速度”的归档选项有明确价值
2. 但不适合作为默认，也不应抢先于 `LZ4 / PPMD`

### 第三批

1. `DEFLATE`
2. `BZIP2`

原因：

1. 两者可以支持
2. 但更多解决“算法矩阵完整性”
3. 在 `.xunbak` 场景下，不是最强产品卖点

---

## 9. 测试要求

每新增一种 codec，至少补以下测试：

1. blob 写入 / 读取 roundtrip
2. 流式复制 `copy_blob_record_content_to_writer()` roundtrip
3. `backup create --format xunbak` 成功
4. `backup restore` 成功
5. `backup convert xunbak -> dir/zip/7z` 成功
6. `verify full` 能发现 codec 不匹配或损坏
7. 未支持 reader 的错误路径清晰

若 codec 是高风险实现（例如 `ppmd / lzma2`），再补：

1. 大文件 roundtrip
2. 文本型样本压缩收益测试
3. 性能基线记录

---

## 10. 决策摘要

最终建议是：

1. `.xunbak` 应扩展 codec，但不应引入 `zip / 7z` 容器语义
2. 真正值得加入的是 `none / zstd / deflate / bzip2 / ppmd / lzma2`
3. 产品价值上优先强调：
   - `zstd` 继续做默认
   - `ppmd` 做文本高压缩
   - `lzma2` 做归档高压缩
4. `deflate / bzip2` 可以做，但不应被包装成核心卖点
5. `auto` 应保持“文本优先 `PPMD`、通用内容优先 `ZSTD`、不可压缩回退 `NONE`”的轻量策略
6. 实现上先改 codec 枚举，再统一 codec 层，再补流式读取与版本兼容

---

## 11. 相关文档

1. [Backup-Optimization-Tasks.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Backup-Optimization-Tasks.md)
2. [xunbak-design-review-round3.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/xunbak-design-review-round3.md)
3. [Xunbak-Tasks.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Xunbak-Tasks.md)
4. [Xunbak-Codec-Expansion-Tasks.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Xunbak-Codec-Expansion-Tasks.md)
