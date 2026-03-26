# `.xunbak` Codec 扩展 — TDD 分阶段任务清单

> 依据：[Xunbak-Codec-Expansion-Plan.md](./Xunbak-Codec-Expansion-Plan.md)
> 标记：`[ ]` 待办　`[-]` 进行中　`[x]` 完成
> 原则：**红-绿-重构**。每个任务先写失败测试，再写最小实现，最后重构。
> 范围：只扩展 `.xunbak` 容器自身的 blob codec；**不**引入 `zip / 7z` 的容器语义。
> 术语：
> `codec 层` = 纯算法压缩/解压与流式复制
> `容器层` = blob / manifest / checkpoint / footer / split
> `内容级 verify` = 真实解码并校验 payload，而不是仅 reopen 结构

---

## Phase 0：基线冻结与边界确认

### 0.1 当前已完成能力

- [x] **测试**：`Codec::NONE` 往返正确
- [x] **测试**：`Codec::ZSTD` 往返正确
- [x] **测试**：未知 `Codec` 可安全表示，不 panic
- [x] **测试**：`blob record` 当前可稳定处理 `NONE / ZSTD`
- [x] **测试**：`backup create --format xunbak` 已支持 `none / zstd / zstd:N`
- [x] **测试**：`backup restore / convert xunbak -> dir/zip/7z` 的既有 `NONE / ZSTD` 路径稳定

### 0.2 当前已知缺口

- [x] **现状**：`LZ4 / LZMA` 只有参数与枚举预留，真实压缩/解压尚未实现
- [x] **现状**：`copy_blob_record_content_to_writer()` 只支持 `NONE / ZSTD`
- [x] **现状**：`CompressionMode::Auto` 目前未形成独立策略，只是退化到 `ZSTD`
- [x] **现状**：CLI 里仍存在 `lzma` 命名，而目标设计应统一为 `lzma2`

### 0.3 非目标冻结

- [x] 明确 `.xunbak` 不引入 archive 级 method id
- [x] 明确 `.xunbak` 不引入 solid 压缩
- [x] 明确 `.xunbak` 不引入多文件跨边界共享字典
- [x] 明确这次扩展只解决“blob codec”，不改 `.xunbak` 容器语义

---

## Phase 1：Codec 编码表、命名与版本策略

### 1.1 Codec 编码表冻结

- [x] **测试**：现有 `NONE / ZSTD / LZ4` 编码值保持稳定
- [x] **测试**：新增 `DEFLATE / BZIP2 / PPMD / LZMA2` 编码值固定且可 roundtrip
- [x] **测试**：未知 codec 仍返回 `is_known() = false`，不 panic
- [x] 扩展 [constants.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/constants.rs) 的 `Codec` 枚举

### 1.2 `lzma` → `lzma2` 命名收敛

- [x] **测试**：CLI 明确接受 `lzma2`
- [x] **测试**：旧参数 `lzma` 的行为被明确固定
  要么兼容映射到 `lzma2`
  要么直接报错并给出升级提示
- [x] **测试**：内部 `CompressionMode` / `Codec` 命名统一为 `Lzma2`
- [x] 收敛 [codec.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/codec.rs) 与 `backup/app/xunbak.rs` 的命名

### 1.3 版本兼容策略

- [x] **测试**：旧 reader 遇到新 codec 时返回明确错误，不 panic
- [x] **测试**：新增 codec 后 `min_reader_version` 策略稳定且可验证
- [x] **测试**：新 writer 对旧 codec 产物不引入 breaking change
  已覆盖 `none / zstd` 的 legacy/create 逻辑产物对照与 `min_reader_version = 1` 回归；若未来要做“旧 reader 二进制”级别回归，可再补外部兼容工件
- [x] 明确 `XUNBAK_WRITE_VERSION / XUNBAK_READER_VERSION` 与新增 codec 的关系

---

## Phase 2：统一 codec backend 边界

### 2.1 codec 层接口

- [x] **测试**：统一入口支持 `compress(codec, data, level)`
- [x] **测试**：统一入口支持 `decompress(codec, data)`
- [x] **测试**：统一入口支持 `stream_hash_and_compress(reader, codec, level, chunk_size)`
- [x] **测试**：新增 `copy_decompressed_to_writer(codec, reader, compressed_len, writer)` 或等价接口
- [x] 实现 `.xunbak` 自己的 codec backend 抽象

### 2.2 抽象边界

- [x] **测试**：共享层只处理字节流压缩/解压，不依赖 `zip / 7z` method id
- [x] **测试**：共享层不依赖 `.xunbak` 的 blob header / manifest 结构
- [x] **测试**：容器层仍独立维护自身 header / record / metadata 逻辑
- [x] 将“算法函数”和“容器语义”严格拆开

### 2.3 收益阈值与通用 helper

- [x] **测试**：所有 codec 统一复用 `compression_is_beneficial()`
- [x] **测试**：`should_skip_compress()` 仍只做路径/扩展名规则，不夹带 codec 专属策略
- [x] **测试**：每个 codec 的 fallback 到 `NONE` 逻辑一致
- [x] 收敛 codec 共用 helper，避免算法分支各自复制逻辑

---

## Phase 3：`LZ4` 第一阶段

### 3.1 codec 层

- [x] **测试**：`Codec::LZ4` 小数据压缩/解压往返正确
- [x] **测试**：`Codec::LZ4` 空数据往返正确
- [x] **测试**：`Codec::LZ4` 1 MiB 数据往返正确
- [x] **测试**：`stream_hash_and_compress(Codec::LZ4)` 结果 hash 正确
- [x] 实现 `LZ4` backend

### 3.2 blob 层

- [x] **测试**：`write_blob_record(..., Codec::LZ4)` 写出合法 blob
- [x] **测试**：`read_blob_record()` 可解出 `LZ4` blob
- [x] **测试**：`copy_blob_record_content_to_writer()` 可流式解出 `LZ4`
- [x] **测试**：收益不足时 `LZ4` 正确回退到 `NONE`
- [x] 将 `LZ4` 接入 [blob.rs](/D:/100_Projects/110_Daily/XunYu/src/xunbak/blob.rs)

### 3.3 命令集成

- [x] **测试**：`xun backup --container project.xunbak --compression lz4` 成功
- [x] **测试**：`backup create --format xunbak --compression lz4` 成功
- [x] **测试**：`backup convert <artifact> --format xunbak --method lz4` 成功
- [x] **测试**：`backup restore archive.xunbak` 可恢复 `LZ4` 内容
- [x] **测试**：`backup convert xunbak -> dir/zip/7z` 可读取 `LZ4` blob

### 3.4 性能基线

- [x] **bench**：记录 `LZ4` create 吞吐
- [x] **bench**：记录 `LZ4` restore 吞吐
- [x] **bench**：对比 `LZ4` 与 `zstd(1)` 的 create / restore 吞吐
- [x] 记录 `LZ4` 的 CPU/内存/压缩率基线

---

## Phase 4：`PPMD` 文本高压缩模式

### 4.1 codec 层

- [x] **测试**：`Codec::PPMD` 文本样本压缩/解压往返正确
- [x] **测试**：`Codec::PPMD` 二进制样本往返正确
- [x] **测试**：`Codec::PPMD` 流式压缩 hash 正确
- [x] **测试**：超大输入不因内部计数溢出而 panic
- [x] 实现 `PPMD` backend

### 4.2 blob 与流式复制

- [x] **测试**：`read_blob_record()` 可解出 `PPMD` blob
- [x] **测试**：`copy_blob_record_content_to_writer()` 对 `PPMD` 不一次性分配整块输出
- [x] **测试**：`PPMD` 的 fallback 到 `NONE` 策略仍成立
- [x] 将 `PPMD` 接入流式读取路径

### 4.3 端到端

- [x] **测试**：`backup create --format xunbak --compression ppmd` 成功
- [x] **测试**：`backup convert ... --format xunbak --method ppmd` 成功
- [x] **测试**：`backup restore` 可恢复 `PPMD` 内容
- [x] **测试**：`backup convert xunbak -> dir/zip/7z` 可读取 `PPMD` blob
- [x] **测试**：文本型样本上 `PPMD` 压缩率优于 `zstd(1)` 或至少有明确收益

---

## Phase 5：`LZMA2` 高压缩模式

### 5.1 codec 层

- [x] **测试**：`Codec::LZMA2` 小文件往返正确
- [x] **测试**：`Codec::LZMA2` 1 MiB 样本往返正确
- [x] **测试**：`stream_hash_and_compress(Codec::LZMA2)` hash 正确
- [x] 实现 `LZMA2` backend

### 5.2 blob 与读取路径

- [x] **测试**：`read_blob_record()` 可解出 `LZMA2` blob
- [x] **测试**：`copy_blob_record_content_to_writer()` 支持 `LZMA2` 流式解压
- [x] **测试**：压缩收益不足时回退 `NONE`
- [x] 接入 blob 读写与流式复制路径

### 5.3 端到端

- [x] **测试**：`backup create --format xunbak --compression lzma2` 成功
- [x] **测试**：若保留 legacy `lzma` 别名，其行为与 `lzma2` 一致或明确拒绝
- [x] **测试**：`backup convert ... --format xunbak --method lzma2` 成功
- [x] **测试**：restore / convert 路径可完整读取 `LZMA2` blob
- [x] **测试**：文档与 CLI 明确说明 `LZMA2` 为慢速高压缩选项

---

## Phase 6：`DEFLATE / BZIP2` 矩阵补齐

### 6.1 `DEFLATE`

- [x] **测试**：`Codec::DEFLATE` 压缩/解压往返正确
- [x] **测试**：blob 读写与流式复制正确
- [x] **测试**：`backup create/convert xunbak` 可用 `deflate`
- [x] 接入 `DEFLATE` backend 与端到端路径

### 6.2 `BZIP2`

- [x] **测试**：`Codec::BZIP2` 压缩/解压往返正确
- [x] **测试**：blob 读写与流式复制正确
- [x] **测试**：`backup create/convert xunbak` 可用 `bzip2`
- [x] 接入 `BZIP2` backend 与端到端路径

### 6.3 定位与提示

- [x] **测试**：CLI / 文档不把 `deflate / bzip2` 宣传为默认推荐 codec
- [x] **测试**：帮助文案强调它们属于“兼容/矩阵补齐”而非主推模式
- [x] 收敛产品文案与帮助输出

---

## Phase 7：create / convert / restore / plugin 全链路接入

### 7.1 create 路径

- [x] **测试**：`xun backup --container` 与 `backup create --format xunbak` 对新 codec 行为一致
- [x] **测试**：`--no-compress` 继续强制 `NONE`
- [x] **测试**：默认不显式指定 codec 时继续走 `ZSTD`
- [x] 收敛 create 路径对 codec 的入口语义

### 7.2 convert -> xunbak 路径

- [x] **测试**：`backup convert <dir|zip|7z|xunbak> --format xunbak --method <codec>` 对所有新 codec 生效
- [x] **测试**：错误 codec 返回稳定错误与 fix hint
- [x] **测试**：`--split-size` 与新 codec 组合行为稳定
- [x] 收敛 convert 路径对 codec 的入口语义

### 7.3 restore / convert from xunbak

- [x] **测试**：`backup restore archive.xunbak` 对所有支持 codec 都能恢复
- [x] **测试**：`backup convert archive.xunbak --format dir|zip|7z` 对所有支持 codec 都能读取
- [x] **测试**：selective restore / glob restore 不因 codec 不同而退化
- [x] 扩展 restore / convert 的 codec 覆盖矩阵

### 7.4 `.xunbak` 7-Zip 插件回归

- [x] **测试**：插件可列出含新 codec 的 `.xunbak`
- [x] **测试**：插件可提取含新 codec 的 `.xunbak`
- [x] **测试**：插件在未知 codec 上给出明确错误，而不是崩溃
- [x] 把新 codec 纳入插件 smoke / acceptance 清单

---

## Phase 8：verify、错误模型与兼容语义

### 8.1 verify 语义

- [x] **测试**：`quick / full / paranoid` 对新 codec 都能稳定运行
- [x] **测试**：损坏 payload 时 `full verify` 能定位 codec 相关错误
- [x] **测试**：`copy_blob_record_content_to_writer()` 的 hash 校验错误能反馈到 restore / convert
- [x] 扩展 verify 路径的 codec 覆盖

### 8.2 错误模型

- [x] **测试**：未知 codec 返回 `UnsupportedCodec`
- [x] **测试**：编码失败 / 解码失败错误信息包含 codec 名称
- [x] **测试**：CLI 层 fix hint 清晰，不暴露底层 panic/opaque error
- [x] 收敛 codec 错误类型与上抛文案

### 8.3 前后版本兼容

- [x] **测试**：旧容器继续可被新 reader 打开
- [x] **测试**：新 codec 容器被旧 reader 拒绝时错误可理解
- [x] **测试**：`min_reader_version` 仅在必要时提升
- [x] 明确并落地升级策略

---

## Phase 9：`auto` 模式与推荐策略

### 9.1 `auto` 语义校准

- [x] **测试**：`auto` 不再简单退化为 `zstd`
- [x] **测试**：`auto` 在不可压缩内容上回退到 `NONE`
- [x] **测试**：`auto` 在可压缩文本上能选出预期 codec
- [x] 定义并实现 `auto` 选择策略

### 9.2 推荐策略

- [x] **测试**：默认推荐仍为 `zstd`
- [x] **测试**：文本型样本对 `ppmd` 有明确收益时给出合理推荐依据
- [x] **测试**：归档型场景对 `lzma2` 的性能提示稳定
- [x] 固化 codec 推荐矩阵

### 9.3 文档与帮助

- [x] **测试**：帮助文案中的 codec 列表与真实实现一致
- [x] **测试**：`auto / none / zstd / lz4 / ppmd / lzma2 / deflate / bzip2` 的说明一致
- [x] 更新 CLI help、设计文档和兼容矩阵

---

## Phase 10：性能基线与综合验收

### 10.1 单项 benchmark

- [x] **bench**：`compress_lz4_1mb`
- [x] **bench**：`compress_ppmd_text_corpus`
- [x] **bench**：`compress_lzma2_1mb`
- [x] **bench**：`compress_deflate_1mb`
- [x] **bench**：`compress_bzip2_1mb`
- [x] **bench**：restore 吞吐对比

### 10.2 端到端矩阵

- [x] **测试矩阵**：`create xunbak` × 全 codec
- [x] **测试矩阵**：`convert -> xunbak` × 全 codec
- [x] **测试矩阵**：`restore xunbak` × 全 codec
- [x] **测试矩阵**：`convert xunbak -> dir/zip/7z` × 全 codec
- [x] **测试矩阵**：`.xunbak` plugin extract × 全 codec

### 10.3 大文件与内存边界

- [x] **测试**：大文件 `LZ4` 路径内存峰值不随 `raw_size` 线性爆炸
- [x] **测试**：大文件 `PPMD` / `LZMA2` 路径不因一次性解压导致失控分配
- [x] **测试**：大文件 convert / restore 仍保持流式复制
- [x] 记录大文件场景下的内存/吞吐基线

### 10.4 综合验收

- [x] `cargo test --test test_xunbak --features xunbak`
- [x] `cargo test --test module_backup_restore --features xunbak -- --test-threads=1`
- [x] `cargo bench --bench xunbak_bench_divan --features xunbak`
- [x] 插件便携 / 系统联调脚本回归

---

## 依赖关系

```text
Phase 0（基线冻结）
  ├─→ Phase 1（编码表/命名/版本）
  └─→ Phase 2（codec backend 边界）

Phase 1 ─┐
Phase 2 ─┼─→ Phase 3（LZ4）
         ├─→ Phase 4（PPMD）
         ├─→ Phase 5（LZMA2）
         └─→ Phase 6（DEFLATE/BZIP2）

Phase 3/4/5/6 ─→ Phase 7（create/convert/restore/plugin 接入）
Phase 7 ─→ Phase 8（verify/兼容）
Phase 8 ─→ Phase 9（auto/推荐策略）
Phase 9 ─→ Phase 10（性能与综合验收）
```

---

## 建议执行顺序

### 第 1 批

1. Phase 1：编码表 / 命名 / 版本策略
2. Phase 2：统一 codec backend
3. Phase 3：`LZ4`

### 第 2 批

1. Phase 4：`PPMD`
2. Phase 5：`LZMA2`

### 第 3 批

1. Phase 6：`DEFLATE / BZIP2`
2. Phase 7：全链路接入
3. Phase 8：verify / 兼容

### 第 4 批

1. Phase 9：`auto` 与推荐策略
2. Phase 10：bench / 大文件 / 综合验收

---

## 验证命令建议

```bash
# 基础编译
cargo check --features xunbak

# xunbak 核心测试
cargo test --test test_xunbak --features xunbak

# backup / restore 黑盒
cargo test --test module_backup_restore --features xunbak -- --test-threads=1

# bench
cargo bench --bench xunbak_bench_divan --features xunbak

# 7-Zip 插件联调
./scripts/test_xunbak_7z_plugin_portable.ps1 -Config Debug
./scripts/test_xunbak_7z_plugin_system.ps1 -Config Debug
```
