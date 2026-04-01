# xun bookmark undo / redo 评估

> 更新时间：2026-03-31  
> 关联文档：bookmark-PRD.md · Bookmarks-Feature-Roadmap.md · bookmark-TDD-TaskList.md · bookmark-Benchmark-Suite.md

> 2026-03-31 实施状态：本文评估结论已落地，bookmark 历史层已从 snapshot-based 重构为 delta-based。下文第 2-11 节主要保留为“重构前基线分析 + 设计推导”，不再代表当前实现状态。

---

## 1. 目标

这份文档回答四个问题：

1. 当前 bookmark 的 `undo / redo` 实现到底是什么
2. 它现在能不能用
3. 它的结构性问题在哪里
4. 下一步是否值得重构为 delta-based 历史

结论先行：

> **当前实现可用，但它本质上是“整库 snapshot-based 历史”，更适合作为快速落地版本，不适合作为 bookmark 的长期历史模型。下一阶段值得重构为“按命令分批、按条目记录 before/after 的 delta-based undo / redo”。**

---

## 2. 当前实现基线

### 2.1 当前代码形态

当前 `bookmark undo / redo` 的核心实现位于：

- [undo.rs](/D:/100_Projects/110_Daily/XunYu/src/bookmark/undo.rs)
- [state.rs](/D:/100_Projects/110_Daily/XunYu/src/bookmark/state.rs)
- [commands/undo.rs](/D:/100_Projects/110_Daily/XunYu/src/bookmark/commands/undo.rs)
- [commands/mutate.rs](/D:/100_Projects/110_Daily/XunYu/src/bookmark/commands/mutate.rs)
- [commands/integration.rs](/D:/100_Projects/110_Daily/XunYu/src/bookmark/commands/integration.rs)

当前历史文件：

- `.xun.bookmark.undo.log`
- `.xun.bookmark.redo.log`

历史上限：

- `MAX_HISTORY = 100`

### 2.2 当前数据结构

当前历史条目是：

```rust
pub(crate) struct UndoEntry {
    pub(crate) ts: u64,
    pub(crate) action: String,
    pub(crate) snapshot: StoreSnapshot,
}
```

也就是说：

- 每次写入历史，落盘的是**完整 `StoreSnapshot`**
- `StoreSnapshot` 本身包含**整库 `bookmarks: Vec<Bookmark>`**
- 不是“记录这次改了什么”，而是“记录改之前或改之后的整库状态”

对应代码证据：

- `UndoEntry.snapshot: StoreSnapshot` 见 [undo.rs](/D:/100_Projects/110_Daily/XunYu/src/bookmark/undo.rs)
- `StoreSnapshot { schema_version, bookmarks }` 见 [state.rs](/D:/100_Projects/110_Daily/XunYu/src/bookmark/state.rs)

### 2.3 当前命令流

当前写路径大致如下：

1. 变更命令先取 `store.snapshot()`
2. `push_undo_snapshot(...)` 把**当前整库快照**追加到 undo 日志
3. 执行真实变更
4. redo 栈被清空

当前 undo/redo 流程如下：

1. 先加载当前 store 并再取一份当前 snapshot
2. 读取整个 undo 或 redo 日志
3. 弹出最后 `n` 条
4. 把“当前整库 snapshot”压入对侧栈
5. 返回目标 snapshot
6. `Store::restore_from_snapshot(...)`
7. `store.save(...)` 把主库整份重写回磁盘

这说明当前模型是：

> **命令层 undo / redo = 整库状态切换，不是局部逆操作重放。**

---

## 3. 当前实现的优点

### 3.1 优点一：简单，正确性容易建立

对于快速开发期，这是当前实现最大的价值。

因为它不需要：

- 设计复杂逆操作模型
- 处理字段级冲突合并
- 维护多种操作的 forward / inverse 变换

只要快照正确，undo / redo 基本就正确。

### 3.2 优点二：覆盖范围已经够用

当前已经覆盖：

- `set`
- `save`
- `rename`
- `delete`
- `import`
- `pin`

CLI 级测试也已经存在，见：

- [bookmark_phase_j.rs](/D:/100_Projects/110_Daily/XunYu/tests/bookmark_phase_j.rs)

### 3.3 优点三：适合早期快速重构

在 bookmark 命令面、schema、query core 都还在重构阶段时，snapshot-based 历史对演进更宽容：

- 新字段只要能进入 `StoreSnapshot` 就能自动被历史覆盖
- 不需要每加一个命令就先加一套 delta 逆操作定义

所以它作为 v1 过渡实现是合理的。

---

## 4. 当前实现的结构性问题

### 4.1 问题一：写入成本与库大小强绑定

当前每次 `push_undo_snapshot(...)` 都会：

- clone 一份完整 `StoreSnapshot`
- 序列化整份 `bookmarks`
- 追加到 undo log

这意味着单次写历史的成本不是 `O(本次改动大小)`，而是近似 `O(整库大小)`。

对于 bookmark 这种：

- 高频 `set / save / learn / import`
- 数据量会持续增长
- 主库本来就已经要整库 save

的组件，这种额外整库历史开销会越来越重。

### 4.2 问题二：日志体积膨胀过快

当前每条历史都带完整 `Vec<Bookmark>`。

如果有：

- `5000` 条书签
- 每条几十到几百字节
- 历史上限 `100`

那么 undo / redo 日志的总体量会非常容易达到“主库的几十倍到上百倍量级”。

这不是偶发问题，而是数据模型天然决定的结果。

### 4.3 问题三：undo / redo 的 I/O 仍然偏重

当前 `run_undo_steps(...)` / `run_redo_steps(...)` 会：

1. 读完整个 log 文件
2. 反序列化所有条目
3. 弹出末尾若干条
4. 重写一侧日志
5. 追加另一侧日志
6. 命令层再把整个 store 重写回主库

所以现在的 undo / redo 不是轻量“回放一批变化”，而是：

> **整份历史文件读写 + 整份 store 恢复 + 整份主库重写**

这在功能上没问题，但在长期性能上不够好。

### 4.4 问题四：历史语义不透明

当前日志里只有：

- `action: "set" | "rename" | "import" | ...`
- `snapshot: ...`

这会带来两个问题：

1. 无法快速知道“这一步到底改了哪些书签”
2. 很难做更细粒度能力，例如：
   - 只回滚某一条导入结果
   - 历史 diff 展示
   - 更轻的审计/调试输出

换句话说，当前历史更像“恢复点”，不是“操作日志”。

### 4.5 问题五：历史与主存储 schema 绑得过紧

当前历史条目直接序列化 `StoreSnapshot`。

这意味着：

- 历史格式天然跟随主存储结构变化
- 每次主存储字段调整，历史兼容成本也会上升
- 如果未来切 SQLite 或局部存储模型变化，当前历史几乎不能原样复用

这不是 bug，但会限制后续演进。

---

## 5. 与 batch_rename 的对照

### 5.1 batch_rename 当前模型

`batch_rename` 的历史实现位于：

- [batch_rename/undo.rs](/D:/100_Projects/110_Daily/XunYu/src/batch_rename/undo.rs)

它的核心结构是：

```rust
pub struct UndoBatch {
    pub ts: u64,
    pub ops: Vec<UndoRecord>,
}

pub struct UndoRecord {
    pub from: String,
    pub to: String,
}
```

它记录的是：

- 一次命令对应一个 batch
- batch 里只有这次真正变动的记录
- undo / redo 操作的是 batch，而不是整库快照

### 5.2 batch_rename 模型的优势

它已经证明了几件事：

1. **append-only 双日志模型可行**
2. **按批次记录历史比整库快照更适合高频命令**
3. **批量追加、启发式 trim、legacy 迁移** 这些工程细节已经有成熟参考

也就是说，bookmark 没必要从零发明一套历史工程化方案。

### 5.3 为什么不能直接照抄 batch_rename

bookmark 和 batch_rename 的本质不同：

- `batch_rename` 的逆操作天然就是 `from -> to`
- bookmark 的变更是**结构化数据状态变更**

bookmark 里常见操作包括：

- 新增条目
- 删除条目
- 重命名显式书签
- pin / tag / desc / workspace 更新
- import merge 批量修改

所以 bookmark 不能直接用 `from/to path` 这种超窄模型。

但是：

> **它完全可以借用 batch_rename 的“按命令批次记录增量”的工程思路。**

---

## 6. 可选方案评估

### 6.1 方案 A：继续保留 snapshot-based，不做结构升级

优点：

- 改动最小
- 当前测试大体可复用
- 短期最省时间

缺点：

- 结构性问题都还在
- 日志增长和整库序列化成本不会消失
- 后续做历史调试、细粒度回滚、后端迁移都不友好

结论：

> 只适合“先能用就行”，不适合作为 bookmark 长期方案。

### 6.2 方案 B：snapshot + 局部优化

例如：

- 给历史加 gzip 压缩
- 减少 trim 次数
- 更激进地限制 history 深度
- 给日志读写加启发式优化

优点：

- 代码改动比重构小
- 可以缓和部分 I/O 问题

缺点：

- 只是在优化“错误层级”的数据模型
- 不能解决历史语义不透明
- 不能把 `O(整库大小)` 变成 `O(变更大小)`

结论：

> 可以作为过渡补丁，但不值得作为正式方向。

### 6.3 方案 C：重构为 delta-based 历史

核心思路：

- 一次命令 = 一个 history batch
- batch 内记录这次真正变更的条目
- 每条 op 记录 `before / after`
- undo 应用 `before`
- redo 应用 `after`

优点：

- 成本与“本次变更规模”相关，而不是与“整库规模”强绑定
- 历史语义清晰
- 更容易做调试、审计、后续迁移
- 更接近 bookmark 的长期架构形态

缺点：

- 需要重写历史建模
- 需要补齐更多测试
- import / dedup / gc 这类批量命令要定义 batch 语义

结论：

> **这是 bookmark 值得投入的正式方向。**

---

## 7. 推荐方案

推荐采用：

> **方案 C：按命令批次、按条目 before/after 的 delta-based undo / redo。**

注意，不建议做“通用 JSON Patch 引擎”。

bookmark 更适合的是：

- 领域专用
- 类型明确
- 可读性高
- 可直接审计

的历史模型。

---

## 8. 推荐的数据模型

建议形态：

```rust
pub struct BookmarkUndoBatch {
    pub ts: u64,
    pub action: String,
    pub ops: Vec<BookmarkUndoOp>,
}

pub enum BookmarkUndoOp {
    Create { after: Bookmark },
    Delete { before: Bookmark },
    Update { before: Bookmark, after: Bookmark },
}
```

语义如下：

- `Create`
  - forward = 创建 `after`
  - undo = 删除该条目
- `Delete`
  - forward = 删除原条目
  - undo = 恢复 `before`
- `Update`
  - forward = `before -> after`
  - undo = `after -> before`

这样做的好处：

- 不需要保存整库
- 不依赖通用 patch
- `rename / pin / tag / desc / workspace / frecency` 都统一落到 `Update`
- `import / dedup / gc` 可以自然表达为一个多 op batch

---

## 9. 推荐迁移边界

### 9.1 主存储不动，只换历史层

当前最合理的边界是：

- 主存储仍然保持现有 bookmark store
- query core 不动
- 命令面不动
- shell / completion 不动
- 只重构 `undo.rs` 与命令侧接线方式

也就是说：

> **这次重构应该是“历史层重构”，不是“bookmark 存储层重构”。**

### 9.2 不建议保历史兼容包袱

项目当前处于快速开发期，且已经允许 breaking change。

因此对 undo / redo 历史文件，建议：

- **不强求迁移旧 snapshot log**
- 直接切换到新的 delta log 格式
- 首次运行时发现旧日志，可提示并清空，或直接覆盖

原因很简单：

- 历史日志是短期状态，不是核心业务数据
- 真正必须严肃迁移的是主存储，不是临时 undo 栈

这能显著降低实现复杂度。

---

## 10. 推荐实施顺序

### Phase 1：定义 delta 历史契约

- 定义 `BookmarkUndoBatch`
- 定义 `BookmarkUndoOp`
- 统一 batch apply / undo / redo 入口

### Phase 2：先覆盖单条命令

优先改：

- `set`
- `rename`
- `delete`
- `pin`

这些命令最容易定义 delta，且价值最高。

### Phase 3：覆盖批量命令

继续接入：

- `import`
- `dedup`
- `gc`
- 其他会批量改 store 的命令

### Phase 4：补齐测试与性能基线

需要新增：

- 历史文件格式单测
- 多步 undo / redo 测试
- partial undo / redo 测试
- 新操作清空 redo 栈测试
- 批量命令 batch 语义测试
- release 级 undo / redo 基准

---

## 11. 对当前阶段的判断

当前 bookmark 的 `undo / redo`：

- **功能上已经可用**
- **工程上属于过渡实现**
- **结构上值得继续优化**

所以它不是“必须立刻推翻重做”的问题，而是：

> **已经到了值得把历史模型从 snapshot-based 升级为 delta-based 的阶段。**

---

## 12. 一句话结论

> **bookmark 当前的 undo / redo 适合当 v1 过渡实现，不适合长期保留；下一步应参考 batch_rename 的双日志批次思路，但改造成 bookmark 自己的领域型 delta 历史，而不是继续堆整库快照。**
