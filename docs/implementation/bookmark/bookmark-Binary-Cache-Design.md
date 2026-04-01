# xun bookmark 二进制缓存设计方案

> 更新时间：2026-04-01  
> 关联文档：bookmark-PRD.md · Bookmarks-Feature-Roadmap.md · bookmark-Benchmark-Suite.md · bookmark-SQLite-Evaluation.md

---

## 1. 背景

bookmark 当前已经完成：

- `xun bookmark <sub>` / `bm <sub>` 命名空间收口
- 统一 query core
- `z / zi / o / oi`
- `explicit / imported / learned / pinned`
- delta-based `undo / redo`
- 持久化倒排索引
- compact JSON 主库存储

但从 2026-03-31 的 release 实测看，20k+ 场景下主瓶颈仍然明显集中在 `store_load`：

- `Store::load(20_000)`：约 `107ms`
- `Store::load(20_000)` compact：约 `83ms`
- `Store::load(50_000)`：约 `263ms`
- `Store::load(50_000)` compact：约 `203ms`

`XUN_BM_LOAD_TIMING=1` 的分阶段统计也说明：

- 20k compact：`parse_store_file` 约 `64~70ms`
- 50k compact：`parse_store_file` 约 `160~163ms`
- `normalize` 只占 `10~25ms`

结论很明确：

> **当前主瓶颈是 JSON 解析本身，而不是 query 排序、倒排索引召回或规范化。**

因此，需要一个新的加载层设计，优先解决 **重复启动 CLI 时的全量 JSON 解析成本**。

---

## 1.1 建议审核结论（2026-03-31）

针对本轮外部建议，审核结果如下：

1. **将倒排索引并入二进制缓存**
   - 问题存在：当前单独保留 `.index.json` 会增加一次额外文件读取与 JSON 解析。
   - 结论：**采纳**。
   - 落地方式：二进制缓存 payload 中同时包含 `bookmarks` 与可选 `index`，不再把持久化索引设计成独立 JSON sidecar。

2. **缓存重建加入轻量文件锁，避免并发踩踏**
   - 问题存在：CLI 高频并发调用时，多个进程可能同时发现 cache 失效并重复重建。
   - 结论：**采纳**。
   - 落地方式：在 cache 写入和替换前加轻量排他锁；读路径不抢锁，失效回退 JSON，重建由抢到锁的进程负责。

3. **强化失效判定（仅 `mtime + len` 不够）**
   - 问题存在：部分文件系统/挂载环境下时间戳精度不足，`mtime + len` 可能漏判。
   - 结论：**采纳**。
   - 落地方式：使用 `source_len + source_modified_ms + source_hash` 三元组校验，其中 `source_hash` 为硬性字段。

4. **`mmap` / 异步重建 / unchecked access**
   - `mmap`：问题是否存在尚未被当前基准证明。当前热点是解析，不是读文件本身。**不作为 Phase 1 默认方案**，保留为 Phase 3 基准实验项。
   - 异步重建：建议有效，但 CLI 进程生命周期短，实现复杂度与收益暂不成正比。**不作为 Phase 1 默认方案**。
   - `unchecked access` 环境开关：当前没有证据表明 checked access 是主要瓶颈。**暂不开放**，先坚持 checked access。

5. **固定头部 + 手写字节序写入**
   - 问题存在：如果把头部也交给通用序列化，会增加调试与快速校验复杂度。
   - 结论：**采纳**。
   - 落地方式：固定长度头部，使用小端序手写编码；payload 才交给二进制序列化框架。

一句话结论：

> **采纳“索引并入 cache、文件锁、强校验、固定头部”这四项；`mmap / 异步重建 / unchecked` 先保留为后续基准驱动的优化项。**

---

## 2. 目标

这份方案只解决一个核心问题：

> **让 bookmark 在中大规模数据集下的重复加载，绕开高成本 JSON 解析。**

具体目标：

1. 保留 JSON 主库作为事实源与人工可修复格式
2. 为 bookmark 增加二进制 fast-load cache
3. 第二次及后续加载优先走二进制缓存
4. 保持命令面、query core、shell 集成、undo/redo 语义不变
5. 优化重点放在 `store_load`，不是重做后端心智

---

## 3. 非目标

这份方案**不**试图解决这些问题：

1. 不把 JSON 主库替换掉
2. 不引入复杂 SQL 查询能力
3. 不改变 bookmark 的产品模型
4. 不处理 Dashboard 完整视图同步
5. 不把 `undo / redo` 改成数据库事件系统

也就是说：

> **本方案是“加载加速层设计”，不是“后端迁移方案”。**

---

## 4. 竞品与资料参考

### 4.1 zoxide：二进制数据库优先，而不是 SQLite

本地参考：

- [zoxide Cargo.toml](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/zoxide-main/Cargo.toml)
- [zoxide db/mod.rs](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/zoxide-main/src/db/mod.rs)
- [zoxide db/stream.rs](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/zoxide-main/src/db/stream.rs)

观察到的关键事实：

1. zoxide 当前主数据库不是 SQLite，而是自己的二进制文件 `db.zo`
2. 它直接依赖 `bincode`
3. 加载后以内存中的目录数组工作，再按需保存
4. 查询流在内存中排序、过滤和懒删除死链

这对 bookmark 的启发是：

> **如果目标是高频 CLI 启动性能，二进制数据库 / 二进制缓存是已经被竞品验证过的路线。**

### 4.2 rkyv：zero-copy 思路对 fast-load 更贴题

官方资料：

- [rkyv docs.rs](https://docs.rs/rkyv/latest/rkyv/)
- 本地 README：[rkyv README](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/README.md)
- 本地设计文档：[zero-copy-deserialization.md](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/book_src/zero-copy-deserialization.md)
- 本地验证文档：[validation.md](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/book_src/validation.md)

关键点：

1. rkyv 官方定位就是 Rust 的 zero-copy deserialization framework
2. 它强调：性能优先，很多类型可接近原生读取性能
3. 它支持 validation / checked access
4. 它很适合“单机本地、重复加载、读多写少”的场景

对 bookmark 的启发是：

> **如果问题核心是“每次 CLI 启动都要重新解析同一份大状态”，那么 zero-copy 风格的二进制缓存比继续压 JSON 或直接上 SQLite 更对症。**

### 4.3 SQLite：适合下一阶段后端，不是当前最短路径

官方资料：

- [About SQLite](https://www.sqlite.org/about.html)
- [SQLite Query Planner](https://www.sqlite.org/queryplanner.html)

SQLite 官方强调：

1. 它是单文件、无服务进程、事务型数据库
2. 它擅长索引、组合查询、覆盖索引、分页、并发读写一致性

对 bookmark 的启发是：

- SQLite 很适合解决复杂过滤、分页、组合索引和并发写
- 但它不是当前 `store_load` JSON 解析成本的最短路径解

所以：

> **SQLite 是长期后端演进候选，而不是当前 bookmark “加载加速”问题的最优先方案。**

---

## 5. 方案选择

### 5.1 备选方案

#### 方案 A：继续只用 compact JSON

优点：

- 改动小
- 可读性强
- 不引入额外格式

缺点：

- 仍然要每次做全量 JSON 解析
- 无法根治 `store_load` 热点

结论：

> 有效，但不是治本。

#### 方案 B：JSON 主库 + 二进制缓存 sidecar

优点：

- JSON 仍是事实源
- 启动可优先读 binary cache
- 风险可控
- 对当前 `store_load` 最对症

缺点：

- 需要 cache 失效策略
- 需要双文件一致性设计

结论：

> **推荐。**

#### 方案 C：直接切 SQLite

优点：

- 长期架构能力更完整
- 索引与组合查询能力更强

缺点：

- 实现成本显著更高
- 不一定是当前 `store_load` 的最短路径

结论：

> 暂不作为当前阶段首选。

### 5.2 推荐结论

> **采用“JSON 主库 + 二进制快速加载缓存 sidecar”方案。**

---

## 6. 总体架构

### 6.1 文件角色

建议形成两层 + 一个锁文件：

1. **主库**
   - `~/.xun.bookmark.json`
   - 人类可读
   - 事实源

2. **二进制缓存**
   - `~/.xun.bookmark.cache`
   - 面向 fast-load
   - 由主库派生，不作为编辑源
   - payload 内可同时包含 `bookmarks` 与持久化倒排索引

3. **缓存锁文件**
   - `~/.xun.bookmark.cache.lock`
   - 仅在 cache 重建/替换时使用
   - 不参与事实数据表达

### 6.2 加载优先级

```text
Store::load(path)
  -> 先读主库文件元数据（len / mtime / hash / schema_version）
  -> 尝试匹配二进制缓存头
  -> 匹配成功：直接加载二进制缓存
  -> 匹配失败：回退 JSON 主库
       -> 解析
       -> 规范化
       -> 在持锁条件下重建二进制缓存
```

### 6.3 保存流程

```text
Store::save(path)
  -> 写 compact JSON 主库（事实源）
  -> 读取主库最终 mtime / len
  -> 生成新的二进制缓存（含可选持久化索引）
```

注意顺序：

> **必须先写主库，再写 cache。cache 只能跟随主库，不允许反向主导主库。**

---

## 7. 二进制缓存格式设计

### 7.1 文件名

建议：

```text
.xun.bookmark.cache
```

理由：

- 明确它是 cache，不是第二主库
- 与 `.json` 主库语义区分清楚

### 7.2 头部设计

缓存文件必须带头部元数据，且建议使用**固定长度头部**，并手写小端序编码。

建议头部字段：

| 偏移 | 大小 | 字段 | 说明 |
|---|---:|---|---|
| 0 | 8 | `magic` | 文件魔数，标识 `.xun.bookmark.cache` |
| 8 | 4 | `cache_version` | cache 格式版本 |
| 12 | 4 | `schema_version` | bookmark schema 版本 |
| 16 | 8 | `source_len` | JSON 主库字节数 |
| 24 | 8 | `source_modified_ms` | JSON 主库 mtime（毫秒） |
| 32 | 8 | `source_hash` | JSON 主库内容哈希 |
| 40 | 4 | `flags` | 预留位；例如 bit0=checked layout，bit1=包含索引 |
| 44 | 8 | `payload_len` | payload 字节长度 |

总长度：`52 bytes`

说明：

- 不单独保留 `payload_codec` 字段，Phase 1 固定只支持一种 codec
- 如果未来更换 codec，直接提升 `cache_version`
- `flags` 只做前向扩展，不影响 v1 判定

### 7.3 失效判断

满足任一条件即失效：

1. `cache_version` 不匹配
2. `schema_version` 不匹配
3. `source_len` 不匹配
4. `source_modified_ms` 不匹配
5. `source_hash` 不匹配
6. 反序列化失败
7. validation 失败

失效后行为：

- 忽略 cache
- 回退 JSON 主库
- 成功加载后重建 cache

### 7.4 payload 内容

缓存 payload 不需要保留完整 `Store` 运行时状态。

应保留：

- `bookmarks: Vec<Bookmark>`
- `index: Option<PersistedBookmarkIndex>`

不需要保留：

- `dirty_count`
- `last_save_at`
- `OnceLock<BookmarkIndex>`
- 其他运行时字段

这意味着缓存表示的是：

> **可直接用于重建 `Store` 的“加载态数据”，不是运行时对象快照。**

---

## 8. 编码格式选择

### 8.1 候选对比

#### `bincode`

优点：

- 简单
- 生态成熟
- zoxide 已验证

缺点：

- 仍然是反序列化，不是 zero-copy
- 对当前“反序列化本身就是热点”的问题，收益可能有限

#### `rkyv`

优点：

- 更适合 fast-load
- 可走 checked access
- 更贴近 bookmark 当前瓶颈

缺点：

- 集成复杂度高于 bincode
- 类型稳定性和格式约束要更早设计清楚

### 8.2 推荐选择

推荐优先选择：

> **`rkyv` 作为 bookmark 二进制缓存的首选实现。**

原因：

1. 当前问题不是“写得慢”，而是“读得慢”
2. 当前热点不是业务逻辑，而是解析成本
3. zero-copy / near-zero-copy 更贴题

但实现策略要保守：

- 默认走 checked access
- 只在明确需要时评估 unchecked

### 8.2.1 依赖审核结论

基于 2026-03-31 的官方资料与本地参考，当前依赖选型结论如下：

| 依赖 | 是否适合作为默认方案 | 结论 |
|---|---|---|
| `rkyv` | 是 | **采纳**。最贴合当前 `store_load` 瓶颈。 |
| `postcard` | 否 | 可作为“小体积优先”备选，但不适合作为 Phase 1 默认。 |
| `musli_zerocopy` | 否 | 技术方向合理，但当前不作为默认方案。 |
| `bincode` | 否 | 不作为默认方案，但理由不是“已废弃”，而是它不解决当前热点。 |
| `xxhash-rust` (`xxh3_64`) | 是 | **采纳**。用于 `source_hash`。 |
| `fslock` | 否 | Windows-only 场景下优先复用现有轻量锁实现，不新增依赖。 |
| `pidlock` | 否 | 更适合单实例守护型锁，不是当前 cache 重建锁的最短路径。 |

### 8.2.2 具体判断

#### `rkyv`

官方 docs.rs 与本地文档都明确强调：

- zero-copy / near-zero-copy 读取
- 支持 validation / checked access
- 非常适合重复加载、本地读多写少场景

结论：

> **适合，且是当前默认首选。**

#### `postcard`

官方 docs.rs 明确把它定位为：

- `#![no_std]` focused serializer / deserializer
- 资源效率优先

这说明它的优势在：

- 体积小
- 编解码简单

但它不是 zero-copy。

结论：

> **适合作为体积优先备选，不适合作为当前默认方案。**

#### `musli_zerocopy`

官方 docs.rs 明确写到：

- 提供 zero-copy primitives
- 支持 incremental validation

技术上是成立的，而且方向正确。

但从当前 bookmark 的现实出发：

- 现有 `Bookmark` 结构更接近普通 Rust/Serde 数据模型
- `rkyv` 对当前结构的接入成本更低
- `musli_zerocopy` 更适合作为后续实验路线，而不是当前 Phase 1 默认实现

结论：

> **不否定，但暂不作为默认方案。**

#### `bincode`

本地参考显示：

- zoxide 目前确实仍在用 `bincode = 1.3.1`

官方 docs.rs 也显示：

- bincode 2 / 3 仍在演进，并提供从 1 到 2 的 migration guide

所以：

> “bincode 已废弃，因此不能用” 这个判断并不准确。

bookmark 不选它的真正原因是：

- 它仍然要做反序列化
- 当前热点就是反序列化本身
- 因此它不是最对症的默认方案

结论：

> **不作为默认方案，但不是因为它已经不可用。**

#### `xxhash-rust`

官方 docs.rs 显示：

- 提供 `xxh3_64`
- 适合 one-shot hash

对当前 bookmark 的 `source_hash` 来说，它的角色只是：

- 快速判断主库内容是否变更

不是安全签名。

因此 `xxh3_64` 非常合适。

结论：

> **适合，且建议直接采用 `xxhash-rust` 的 `xxh3_64`。**

#### `fslock`

官方 docs.rs 显示它提供的是：

- per-handle 文件锁

这本身没有问题。

但 bookmark 当前已经有 Windows-only 轻量文件锁实现，且 cache 重建锁需求也很简单：

- 只在写 cache 时抢锁
- 读路径不加锁

所以此时再增加 `fslock`，收益不明显。

结论：

> **能用，但当前没必要引入。**

#### `pidlock`

官方 docs.rs 显示它更偏向：

- PID-based file lock
- stale lock detection
- 单实例运行控制

这类锁更适合：

- 守护进程
- 后台单实例服务

而 bookmark 当前 cache 重建是：

- 短时、快速、面向文件替换的互斥

不需要 PID 语义。

结论：

> **当前不适合做默认锁方案。**

### 8.3 关于 `mmap`

`mmap` 的建议是合理的，但当前不应默认采用。

原因：

1. 当前基准已经证明主热点在 JSON 解析，而不是文件读取
2. `rkyv` 本身不要求必须 `mmap` 才能发挥 zero-copy 思路
3. `mmap` 会增加平台差异、文件生命周期与错误处理复杂度

因此建议：

- **Phase 1**：先使用 `std::fs::read + checked access`
- **Phase 3**：若 binary cache 已显著提速，再额外基准 `mmap + rkyv`

结论：

> `mmap` 是潜在增强项，不是当前默认方案。

---

## 9. 安全与鲁棒性

### 9.1 默认使用 checked access

基于 rkyv 官方 validation 设计，建议：

- 默认使用 `access` / checked access
- 不默认使用 unchecked access

理由：

- bookmark cache 是本地文件，但可能损坏、截断或被外部工具误改
- checked access 的额外成本仍远低于全量 JSON 解析
- 安全默认值更合理

### 9.2 原子写与文件锁

cache 与主库一样，必须：

- 先写 `.tmp`
- 再 rename

此外，cache 重建/替换前必须持有轻量排他锁。

建议：

- 锁文件：`.xun.bookmark.cache.lock`
- 只在 cache 写阶段加锁
- 读路径不阻塞，失效时直接回退 JSON
- 只有获得锁的进程负责真正重建 cache

这样可以避免多个 CLI 进程同时解析 JSON 并重复写 cache，造成并发踩踏。

### 9.3 容错行为

cache 损坏时不得中断主流程。

正确行为：

```text
cache 失败 -> 记录调试日志 -> 回退 JSON -> 持锁重建 cache
```

bookmark 的可用性必须优先于 cache 的完美性。

### 9.4 关于异步重建

“回退 JSON 后异步重建 cache”在原则上是有效建议，但不作为 Phase 1 默认方案。

原因：

1. CLI 进程生命周期短，后台线程并不可靠
2. 子进程式后台重建会引入更多并发、锁和清理复杂度
3. 当前首要目标是先验证 binary cache 本身的收益

因此建议：

- **Phase 1/2**：同步、持锁、可预测地重建 cache
- **Phase 3**：如果首个 miss 的写回耗时仍然明显，再评估后台子进程重建

### 9.5 关于 unchecked access

不建议在 Phase 1 就暴露 `unchecked` 的用户开关。

原因：

1. 当前没有证据显示 checked validation 是主瓶颈
2. 本地 cache 文件也可能损坏或被误改
3. checked access 的成本仍远低于 JSON 解析

因此建议：

- 默认并坚持 checked access
- 如后续确有需要，仅保留内部 benchmark 开关，不先公开给用户

---

## 10. 与现有实现的边界

这次设计不应改变这些边界：

- 命令面不变
- query core 不变
- undo / redo 语义不变
- shell init / completion 不感知后端变化
- JSON 主库继续保留

所以本次改动应主要落在：

- `src/bookmark/state.rs`
- `src/bookmark/storage.rs` 或新的 `src/bookmark/cache.rs`

更推荐：

```text
src/bookmark/cache.rs
```

让 cache 逻辑从 `state.rs` 中拆出，避免继续把状态层和加载优化耦合在一起。

---

## 11. 建议的数据流

### 11.1 首次加载

```text
JSON 主库存在
cache 不存在
-> 读 JSON
-> normalize
-> 返回 Store
-> 写 cache
```

### 11.2 二次加载

```text
JSON 主库存在
cache 存在且元数据匹配
-> 直接读 cache
-> 返回 Store
```

### 11.3 主库被修改

```text
save
-> 写 compact JSON
-> 写 binary cache（含可选持久化索引）
```

### 11.4 cache 失效

```text
cache 元数据不匹配 / validation 失败
-> 忽略 cache
-> 回退 JSON
-> 重建 cache
```

---

## 12. 实施阶段建议

### Phase 1：格式与加载骨架

- 新增 `cache.rs`
- 定义 cache header
- 定义 `load_cache / write_cache / invalidate`
- 先实现 metadata 匹配与回退策略

### Phase 2：缓存加载态 payload

- 缓存 `bookmarks`
- 内嵌持久化 `index`
- 不缓存 undo/redo 运行时状态
- 覆盖 `Store::load` 与 `Store::save`

### Phase 3：release 验证

固定验证：

- `Store::load(20k)`
- `Store::load(50k)`
- `xun bookmark z --list`
- `bm z --list`
- `xun __complete bookmark z`

对比三组：

1. 原始主库
2. compact JSON 主库
3. compact JSON + binary cache

### Phase 4：是否保留 cache 默认开启

只有满足以下条件才默认开启：

- 20k+ 场景比 compact JSON 明显更快
- 5k 场景无明显回退
- 损坏回退逻辑稳定

截至 2026-04-01，这个门槛在 20k / 50k 的 release 实测上已经满足：

- `Store::load(20k)`：约 `155ms -> 34ms`
- `Store::load(50k)`：约 `396ms -> 86ms`
- `xun bookmark z --list(20k, compact -> warm cache hit)`：约 `237ms -> 65ms`
- `xun bookmark z --list(50k, compact -> warm cache hit)`：约 `520ms -> 119ms`

---

## 13. 风险与取舍

### 风险一：cache 复杂度上升

应对：

- 保持 JSON 主库唯一事实源
- cache 只做派生层

### 风险二：rkyv 格式/类型稳定性管理

应对：

- 明确 `cache_version`
- schema / cache 两条版本线分开

### 风险三：本地损坏文件导致异常

应对：

- checked access
- 容错回退 JSON

### 风险四：cache 未必比 compact JSON 更快

应对：

- 先做最小原型
- 用 release 实测决策

---

## 14. 最终建议

当前 bookmark 的“治本型”优化路线，建议按优先级排序如下：

1. **JSON 主库继续 compact 化**
2. **引入 `rkyv` 风格二进制 fast-load cache**
3. **只有在复杂查询 / 并发写需求成形后，再进入 SQLite**

一句话结论：

> **对当前 bookmark 来说，最值得做的不是立刻切 SQLite，而是保留 JSON 主库作为事实源，引入一个受控、可回退、带校验的二进制快速加载缓存层。**

---

## 15. 参考资料

### 在线官方资料

- SQLite 官方：<https://www.sqlite.org/about.html>
- SQLite Query Planner：<https://www.sqlite.org/queryplanner.html>
- rkyv 官方文档：<https://docs.rs/rkyv/latest/rkyv/>

### 本地参考资料

- [zoxide Cargo.toml](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/zoxide-main/Cargo.toml)
- [zoxide db/mod.rs](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/zoxide-main/src/db/mod.rs)
- [zoxide db/stream.rs](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/zoxide-main/src/db/stream.rs)
- [rkyv README](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/README.md)
- [rkyv zero-copy-deserialization](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/book_src/zero-copy-deserialization.md)
- [rkyv validation](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/book_src/validation.md)

---

## 16. 当前实现结论（2026-04-01）

截至 2026-04-01，binary cache 已经不再是早期的 JSON payload 原型，而是：

- 固定 `52-byte` header
- `xxh3_64` source hash
- 内嵌持久化索引的 `rkyv` payload
- 轻量文件锁 + 原子替换

当前实现对齐的官方资料与 API：

- `rkyv` 官方高层 checked API：`to_bytes` / `from_bytes`
- `rkyv` 官方 `AlignedVec<const ALIGNMENT: usize = 16>` 语义
- `rkyv` 官方 validation 模型

实现细节上，payload 读取前会先复制进 `AlignedVec<16>`，原因不是业务逻辑，而是固定 52-byte header 会让 payload 在原始文件中的偏移不再天然满足默认对齐要求。

当前 release 实测结果表明：

- `Store::load(20k, compact, cache disabled)`：约 `155ms`
- `Store::load(20k, warm binary cache hit)`：约 `34ms`
- `Store::load(50k, compact, cache disabled)`：约 `396ms`
- `Store::load(50k, warm binary cache hit)`：约 `86ms`
- `xun bookmark z --list(20k, compact -> warm cache hit)`：约 `237ms -> 65ms`
- `xun bookmark z --list(50k, compact -> warm cache hit)`：约 `520ms -> 119ms`
- `xun __complete bookmark z(20k, compact -> warm cache hit)`：约 `255ms -> 98ms`
- `xun __complete bookmark z(50k, compact -> warm cache hit)`：约 `602ms -> 206ms`

这说明：

> **`rkyv` binary cache 已经证明自己值得保留；当前主问题不再是“cache 是否有效”，而是“命令外层还剩哪些成本可以继续压缩”。**

换句话说：

- header / 锁 / 失效策略：方向正确，已经落地
- payload 编码：`rkyv` 路线成立，早期 JSON payload 结论已经作废
- 下一步优化重点：命令外层、working set、以及是否需要 `mmap`
