# `.xunbak` 单文件容器 MVP 设计 — 第三轮审核建议

> 本轮聚焦用户新增的两个需求：**分卷支持** 和 **可选压缩算法**。这两个需求当前文档均未涉及。

---

## 1. 分卷支持

### 1.1 何时需要分卷

"单文件优先"是否与"分卷"矛盾？不矛盾。分卷是**单文件的工程退化保护**：

| 场景                   | 说明                                         |
| ---------------------- | -------------------------------------------- |
| 文件系统限制           | FAT32 单文件上限 4 GB；部分老旧 NAS 也有限制 |
| 传输介质限制           | U 盘、网盘单文件上传限制                     |
| 增量更新后容器持续膨胀 | compact 前容器可能超出预期大小               |

主流做法参考：

| 工具  | 分卷命名                                   | 元数据位置                     |
| ----- | ------------------------------------------ | ------------------------------ |
| 7z    | `file.7z.001`, `.002`, ...                 | 第一卷含 header                |
| RAR 5 | `file.part001.rar`, `.part002.rar`         | 每卷有卷号标记，第一卷含元数据 |
| ZIP   | `file.z01`, `.z02`, ... `.zip`（最后一个） | 目录在最后一个 split           |

参考：[7-Zip: 7z format](https://7-zip.org/7z.html)、[WinRAR split volume docs](https://documentation.help/WinRAR/HELPArcVolumes.htm)

### 1.2 对 `.xunbak` 的分卷建议

#### 原则：分卷是可选的，默认不分

```text
xun backup --container project.xunbak                          # 单文件，不分卷
xun backup --container project.xunbak --split-size 2G          # 分卷，每卷 2 GB
```

#### 命名规范

建议采用 7z 风格的后缀编号：

```text
project.xunbak       → 不分卷时
project.xunbak.001   → 第 1 卷
project.xunbak.002   → 第 2 卷
...
```

理由：后缀编号不改变主文件名，排序直观，与 7z/RAR 5 的用户认知一致。

#### 格式影响最小化

分卷对当前设计的改动应该 **极小**：

1. **Header** 增加一个 flag：`FLAG_SPLIT = 0x01`
2. **Header** 增加字段：`volume_index: u16`、`split_size: u64`
3. **写入逻辑**：当已写入字节接近 `split_size` 时，在当前 blob record 结束后切卷
4. **切卷规则**：**不在 blob 中间切**，只在 record 边界切卷
5. **Checkpoint + Footer** 始终在 **最后一卷**

```text
卷 1: [Header(vol=0)] [Blob 1] [Blob 2] [Blob 3]
卷 2: [Header(vol=1)] [Blob 4] [Blob 5] [Manifest] [Checkpoint] [Footer]
```

不在 blob 中间切卷的好处：

- 每个卷可以独立读取完整的 blob record
- 恢复时只需要知道目标 blob 在哪个卷（manifest entry 增加 `volume_index` 字段）
- 大幅简化实现

#### MVP 建议

> [!TIP]
> **分卷不应进入 Phase 1**。MVP 目标文件数 ≤ 50,000、容器 ≤ 10 GB，大多数场景下单文件足够。建议放在 **Phase 2 或 Phase 3**，而 Phase 1 只需预留 Header 中的 flag 位。

预留成本极低：

```text
offset  size  field
16      8     flags        ← bit 0 = FLAG_SPLIT（MVP 写 0，reader 看到非 0 也不会 panic）
```

---

## 2. 可选压缩算法

### 2.1 当前状态

文档只支持 `none | zstd`。用户需要"可选压缩算法"。

### 2.2 推荐的额外算法

基于 Rust 生态成熟度和性能定位，建议支持以下 codec 枚举：

| codec         | Rust crate                                                                             | 定位                      | 典型压缩速度   | 典型解压速度 | 压缩率 |
| ------------- | -------------------------------------------------------------------------------------- | ------------------------- | -------------- | ------------ | ------ |
| `none`        | —                                                                                      | 不压缩                    | ∞              | ∞            | 1:1    |
| `lz4`         | [`lz4_flex`](https://crates.io/crates/lz4_flex)                                        | **极速**压缩/解压         | ~660 MB/s      | ~5000 MB/s   | 低     |
| `zstd`        | [`zstd`](https://crates.io/crates/zstd)                                                | **平衡**压缩/解压（默认） | ~300 MB/s (L3) | ~1000 MB/s   | 中高   |
| `lzma` / `xz` | [`lzma-rs`](https://crates.io/crates/lzma-rs) 或 [`xz2`](https://crates.io/crates/xz2) | **高压缩率**，慢          | ~5 MB/s        | ~80 MB/s     | 高     |

> 参考数据来源：
>
> - lz4_flex benchmarks: [crates.io/crates/lz4_flex](https://crates.io/crates/lz4_flex)
> - zstd performance: [facebook/zstd benchmarks](https://github.com/facebook/zstd#benchmarks)
> - [Reddit: LZ4 vs Zstd in Rust](https://www.reddit.com/r/rust/comments/1hjhx7r/sparse_voxel_octree_rust_lz4_vs_zstd_comparison/)

**不建议 MVP 加入 brotli**：

- brotli 压缩速度比 lzma 更慢，解压速度与 zstd 接近
- 主要优势在 Web 场景（HTTP 内容编码），对本地备份无特殊价值
- Rust 生态的 brotli 库（`brotli` crate）体积较大

### 2.3 codec 编码方案

当前 blob record 中 `codec` 是一个字段。建议明确为 **u8 枚举**：

```text
0x00 = none
0x01 = zstd
0x02 = lz4
0x03 = lzma
0x04-0xFF = reserved
```

这样做的好处：

1. 单字节，开销极小
2. 旧版 reader 遇到未知 codec 可以直接报错而不是 panic
3. 每个 blob 独立记录 codec，同一个容器内可以混合不同算法

### 2.4 参考 borg 的 `auto` 模式

borg 的 `auto,zstd,3` 模式非常值得借鉴：

```text
borg create --compression auto,zstd,3 ...
```

工作方式（参考 [borg compression docs](https://borgbackup.readthedocs.io/en/stable/usage/create.html)）：

1. 先用 lz4 对数据做试压
2. 如果压缩后 ≥ 原始大小的 97%（即几乎不可压缩）
3. 则直接存 `none`
4. 否则用指定算法（如 zstd level 3）正式压缩

**对 `.xunbak` 的适配建议**：

```text
--compression auto           → 先试 lz4，可压则用 zstd(3)，不可压则 none
--compression zstd           → 强制 zstd(3)
--compression zstd:9         → 强制 zstd level 9
--compression lz4            → 强制 lz4
--compression lzma           → 强制 lzma
--compression none           → 不压缩
```

MVP 阶段只需要实现：

1. `none`
2. `zstd`（默认）
3. 自动跳过不可压缩文件（当前 §5.3 已有）

Phase 2 再加入：

1. `lz4`
2. `lzma`
3. `auto` 模式

### 2.5 对 codec 选择的性能建议

| 场景                      | 推荐 codec | 原因                        |
| ------------------------- | ---------- | --------------------------- |
| 日常开发备份（默认）      | `zstd(3)`  | 速度和压缩率的最佳平衡      |
| 频繁增量更新、追求速度    | `lz4`      | 压缩/解压极快，恢复体验最好 |
| 长期归档、空间优先        | `lzma`     | 压缩率最高，但慢            |
| 已压缩文件（媒体/压缩包） | `none`     | 避免无效 CPU 消耗           |

### 2.6 对 manifest 的影响

每个 manifest entry 已经有 `codec` 字段。这意味着：

1. 同一个快照中不同文件可以用不同 codec
2. `auto` 模式下的 codec 选择结果最终记录在 entry 的 `codec` 中
3. 恢复时读 entry 的 `codec` 字段即可知道如何解压，无需全局配置

这与当前设计 **完全兼容**，不需要改容器结构。

---

## 3. 分期建议更新

| Phase       | 分卷                                   | 压缩                                      |
| ----------- | -------------------------------------- | ----------------------------------------- |
| **Phase 1** | Header flags 预留 `FLAG_SPLIT`，不实现 | `none` + `zstd`（默认）+ 自动跳过不可压缩 |
| **Phase 2** | 实现分卷写入/读取                      | 加入 `lz4`、`lzma`、`auto` 模式           |
| **Phase 3** | 评估分卷 compact                       | 评估 zstd dictionary                      |

---

## 4. 最关键的 3 条建议

| 优先级 | 建议                                                               | 原因                                        |
| ------ | ------------------------------------------------------------------ | ------------------------------------------- |
| **P0** | codec 字段明确为 u8 枚举，Phase 1 先实现 `0x00=none` + `0x01=zstd` | 格式层预留，不增加 MVP 工作量               |
| **P1** | Header flags 预留 `FLAG_SPLIT` 位，Phase 1 不实现分卷              | 零成本预留，避免后续 breaking change        |
| **P1** | Phase 2 增加 `lz4` 和 `auto` 模式，参考 borg 的试压策略            | lz4 解压比 zstd 快 5x，对恢复速度有直接帮助 |

---

## 5. 参考资料

1. **分卷格式**
   - [7-Zip 7z format](https://7-zip.org/7z.html)
   - [WinRAR split volume docs](https://documentation.help/WinRAR/HELPArcVolumes.htm)
   - [ZIP split archive spec](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT)

2. **压缩算法对比**
   - [Facebook zstd benchmarks](https://github.com/facebook/zstd#benchmarks)
   - [lz4_flex crate](https://crates.io/crates/lz4_flex)（pure Rust LZ4，解压 5000+ MB/s）
   - [Reddit: LZ4 vs Zstd in Rust](https://www.reddit.com/r/rust/comments/1hjhx7r/)

3. **borg auto 压缩模式**
   - [borg create --compression](https://borgbackup.readthedocs.io/en/stable/usage/create.html)
   - [borg internals: per-chunk codec](https://borgbackup.readthedocs.io/en/2.0.0b19/internals/data-structures.html)

4. **Rust 压缩 crate**
   - [`zstd`](https://crates.io/crates/zstd)
   - [`lz4_flex`](https://crates.io/crates/lz4_flex)
   - [`lzma-rs`](https://crates.io/crates/lzma-rs)
   - [`xz2`](https://crates.io/crates/xz2)
