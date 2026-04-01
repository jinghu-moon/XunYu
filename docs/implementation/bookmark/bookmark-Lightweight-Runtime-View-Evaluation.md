# xun bookmark 轻量运行时视图评估

> 更新时间：2026-04-01  
> 关联文档：bookmark-PRD.md · bookmark-Benchmark-Suite.md · bookmark-Binary-Cache-Design.md

---

## 1. 目标

这份文档回答四个问题：

1. 当前 bookmark 是否值得引入“轻量运行时视图”
2. 如果值得，它应该解决哪一段热点
3. 哪些命令适合优先迁移
4. 推荐的实现边界是什么

结论先行：

> **这条路线在工程上已经证明可行，但按当前 release 实测，还没有形成稳定的端到端净收益。结论应从“建议推进”调整为“保留为实验性路径，暂不默认扩张”。**

---

## 2. 当前基线

### 2.1 当前运行时模型

bookmark 现在的主运行时对象仍然是：

- [Store](/D:/100_Projects/110_Daily/XunYu/src/bookmark/state.rs)
- [Bookmark](/D:/100_Projects/110_Daily/XunYu/src/bookmark/state.rs)

即使 binary cache 命中，当前流程仍然会把缓存中的 bookmark 数据恢复成完整 owned `Bookmark` 列表，然后再继续 query / list / completion。

当前 cache-hit 路径核心代码：

- [load_cache_store_data_checked](/D:/100_Projects/110_Daily/XunYu/src/bookmark/cache.rs)
- [Store::load](/D:/100_Projects/110_Daily/XunYu/src/bookmark/state.rs)

### 2.2 当前热点基线

根据 [bookmark-Benchmark-Suite.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-Benchmark-Suite.md) 当前样本：

- `20k` 热命中：
  - `store_load≈22ms`
  - `deserialize_cache_payload≈15ms`
  - `materialize_cache_payload≈2ms`
  - `query_rank≈4ms`

- `50k` 热命中：
  - `store_load≈33ms`
  - `deserialize_cache_payload≈4ms`
  - `materialize_cache_payload≈12ms`
  - `deserialize_cache_index≈6ms`
  - `query_rank≈12ms`

这说明：

1. query 内核已经不是当前最大问题
2. 当前最大剩余成本已经转移到 **cache-hit 后的数据实体化**

### 2.3 当前实验结果

Phase A-G 已经实现并验证了 lightweight runtime view 第一阶段：

- 契约层、借用模型层、缓存读取层、查询层、消费层、边界层均已打通
- `__complete`、`bookmark ... --list/--why/--preview`、`list/recent/stats/keys/all` 都已具备 borrowed 读路径
- cache-hit 与 cache-miss 的用户可见输出一致

但 release 对照结果是：

- `20k` 热命中 `__complete`：`owned≈55ms`，`lightweight≈56ms`
- `50k` 热命中 `__complete`：`owned≈94ms`，`lightweight≈98ms`

这说明：

> **当前 lightweight runtime view 还没有证明自己在端到端层面优于现有 owned 路径。**

---

## 3. 为什么这个问题真实存在

### 3.1 官方资料给出的关键事实

`rkyv` 官方文档明确把 zero-copy 的核心收益描述为：

- 直接通过 `access` 访问 archived 数据，而不是先做完整 deserialize
- `ArchivedVec::as_slice()` 可直接读 archived 列表
- `ArchivedString::as_str()` 可直接读 archived 字符串
- `ArchivedBTreeMap` / archived map 可直接迭代

官方资料：

- docs.rs `access / from_bytes`：<https://docs.rs/rkyv/latest/rkyv/>
- `ArchivedVec`：<https://docs.rs/rkyv/latest/rkyv/vec/struct.ArchivedVec.html>
- `ArchivedString`：<https://docs.rs/rkyv/latest/rkyv/string/struct.ArchivedString.html>
- `ArchivedBTreeMap`：<https://docs.rs/rkyv/latest/rkyv/collections/btree_map/struct.ArchivedBTreeMap.html>

本地参考：

- [rkyv README](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/README.md)
- [rkyv zero-copy-deserialization](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/book_src/zero-copy-deserialization.md)
- [rkyv ArchivedVec](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/rkyv/src/vec.rs)
- [rkyv ArchivedString](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/rkyv/src/string/mod.rs)
- [rkyv ArchivedBTreeMap impl](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/rkyv/src/impls/alloc/collections/btree_map.rs)

核心含义非常直接：

> **只要我们在命令执行中仍然把 archived 数据完整恢复成 owned `Bookmark`，就没有真正吃满 zero-copy 路线的收益。**

### 3.2 `refer/rkyv-main` 源码给出的实现可行性

本地源码已经把“borrowed 只读视图”所需的关键能力展示得很明确：

1. `ArchivedVec::as_slice()` 直接暴露 archived slice  
   见 [vec.rs](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/rkyv/src/vec.rs)

2. `ArchivedString::as_str()` 直接暴露 `&str`  
   见 [string/mod.rs](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/rkyv/src/string/mod.rs)

3. archived `BTreeMap` 可以直接 `iter()`，并在迭代中读取：
   - key: `k.as_str()`
   - value: `v.to_native()`
   见 [btree_map.rs](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/rkyv/src/impls/alloc/collections/btree_map.rs)

这三点组合起来，已经足够支撑：

- bookmark row 借用 archived string / vec
- tag 列表借用 archived vec
- embedded index 借用 archived map 或 archived term 列表

换句话说：

> **从 `rkyv` 本身能力来看，lightweight runtime view 不是“理论可行”，而是已经有直接的容器访问模型可以落地。**

### 3.3 `refer/zoxide-main` 源码给出的竞品启发

本地 zoxide 参考里，最有价值的不是数据库格式本身，而是它的查询流形态：

- [zoxide cmd/query.rs](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/zoxide-main/src/cmd/query.rs)
- [zoxide db/stream.rs](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/zoxide-main/src/db/stream.rs)

可以观察到几个关键点：

1. `Query::run()` 打开数据库后，直接进入查询流，不做额外中间模型
2. `Stream::new()` 先按 score 排序一次，然后通过 `next()` 流式返回命中项
3. `filter_by_keywords()` 直接在 lowercased path 上做反向匹配，不构造额外结果对象树
4. list / first / interactive 三种消费方式，共享同一条流

这对 bookmark 的启发不是“照抄 zoxide 的结构”，而是：

> **高频查询路径的正确方向，是减少中间 owned 结构、尽量让筛选和消费贴近底层数据表示。**

### 3.2 当前实现还在哪一步做了“重新付费”

虽然 binary cache 已经避免了 JSON parse，但 cache-hit 之后当前仍然要：

1. 把 archived bookmark 逐条 materialize 成 owned `Bookmark`
2. 为这些 `Bookmark` 重建 `name_norm / path_norm`
3. 为后续 query/list/completion 再遍历这些 owned 数据

所以当前热路径已经从“JSON parse”变成：

> **archived bytes -> owned runtime objects**

结合 `refer` 里的源码证据，这一步其实正是当前 bookmark 与 `rkyv` / zoxide 最不一致的地方：

- `rkyv` 提供了直接借用 archived 数据的容器 API
- zoxide 查询路径则尽量避免额外实体化
- 而 bookmark 目前仍然在 cache-hit 后完整恢复 owned `Bookmark`

---

## 4. 轻量运行时视图是什么

这里的“轻量运行时视图”，不是一个新的持久化格式，而是：

> **一组只在进程内短生命周期存在的、直接借用 archived cache 数据的只读视图对象。**

它与当前 owned `Store` 的区别：

### 4.1 当前 owned 模型

- 数据完全 owned
- 适合读写混合
- 能直接 `record_visit / save / mutate`
- cache-hit 时恢复成本较高

### 4.2 lightweight runtime view

- 只读
- 借用 archived bytes
- 更适合 `completion / list / explain / preview` 这类读路径
- 不适合直接承担 mutation

### 4.3 它更像 zoxide 的“查询流”，而不是新的主存储

从 `refer/zoxide-main` 看，竞品的高频路径核心特征是：

- 数据一旦载入，查询阶段尽量直接流式消费
- 不为 list / first / interactive 三种消费模式重复构造额外中间状态

因此，这里的 lightweight runtime view 更合适的理解是：

> **一个贴近 archived cache 的查询流视图，而不是第二套长期状态存储。**

---

## 5. 候选方案评估

### 5.1 方案 A：维持现状

做法：

- 继续统一走 owned `Store`
- 不新增 runtime view

优点：

- 简单
- 没有双路径心智
- 不增加生命周期复杂度

缺点：

- cache-hit 的主要剩余热点不会被根治
- `50k` 下已经明确看到 `materialize_cache_payload` 仍然是主要成本之一

结论：

> **不推荐作为长期方案。**

### 5.2 方案 B：新增“只读 lightweight view”，只覆盖 cache-hit 读路径

做法：

- 保留 owned `Store` 不动
- 新增 `BookmarkArchivedView<'a>` / `BookmarkRowView<'a>` 一类借用型对象
- 对 cache-hit 的纯读路径直接走 archived view

优点：

- 精准命中当前热点
- 不破坏现有 mutation 体系
- 与 `rkyv` 官方 zero-copy 路线一致
- 风险相对可控

缺点：

- 需要维护两条读取路径
- query 层需要增加 borrowed 结果类型
- 输出层需要适配 borrowed 数据

结论：

> **在“技术方向是否可行”这个层面推荐；在“是否继续默认推进”这个层面暂不推荐。**

### 5.3 方案 C：全面用 lightweight view 替换 owned `Store`

做法：

- 让 bookmark 所有命令默认都围绕 borrowed view 运转
- mutation 时再专门做 promote / copy-back

优点：

- 理论上最激进
- 所有读路径都能受益

缺点：

- 实现复杂度显著上升
- 会强耦合 lifetimes、promotion、mutation 语义
- 很容易让当前 `Store` / `Bookmark` / undo / save 模型整体失稳

结论：

> **当前不推荐。**

---

## 6. 哪些命令最适合优先迁移

### 6.1 第一优先级：纯只读高频命令

这些命令最适合率先使用 lightweight runtime view：

- `__complete bookmark z ...`
- `bookmark z --list`
- `bookmark zi --list`
- `bookmark o --list`
- `bookmark oi --list`
- `bookmark ... --why`
- `bookmark ... --preview`

原因：

- 它们不会修改 store
- 它们已经明确位于高频热路径
- 它们对“只读 borrowed 结果”的适配成本最低

### 6.2 第二优先级：纯展示命令

- `bookmark list`
- `bookmark recent`
- `bookmark stats`
- `bookmark keys`
- `bookmark all`

原因：

- 这些命令本质也是只读
- 但它们不一定像 `__complete` 一样高频，所以优先级低于第一组

### 6.3 暂不建议第一阶段迁移的命令

- `bookmark z`
- `bookmark zi`
- `bookmark o`
- `bookmark oi`

原因：

这些命令最终会调用：

- [record_visit_by_id](/D:/100_Projects/110_Daily/XunYu/src/bookmark/state.rs)
- [Store::save](/D:/100_Projects/110_Daily/XunYu/src/bookmark/state.rs)

也就是它们虽然前半段是读，但后半段是写。  
这类命令更适合在第二阶段采用：

> **前半段 borrowed query，最终命中后只 promote 单条或按需 promote store**

而不是第一阶段就全量改造。

---

## 7. 对现有代码的契合度

当前代码里其实已经有一个很好的切入点：

- [BookmarkRecordView](/D:/100_Projects/110_Daily/XunYu/src/bookmark/core.rs)

这说明 query 核心本来就是“借用视角友好”的。  
问题不在 query 打分公式，而在于：

- 当前 `query()` 仍然返回 owned `RankedBookmark`
- 输出层和 command handler 仍然默认依赖 owned `Bookmark`

所以从架构上看，lightweight runtime view 的切入方式不是“重写 query core”，而是：

1. 新增 borrowed record / ranked view
2. 让读路径优先返回 borrowed 结果
3. 只有真正需要 mutation 时才 promote

这与现有结构是兼容的。

再结合 `refer/rkyv-main` 的 archived 容器源码，可以更具体地收口成：

1. `BookmarkArchivedRow<'a>` 不需要发明新格式，只要包装 archived fields 即可
2. `BookmarkRecordView` 已经是 query core 的借用接口，天然适合作为桥接层
3. `RankedBookmarkView<'a>` 只需要把当前 `RankedBookmark` 的 owned `Bookmark` 替换成 borrowed row 引用

所以从实现切入点看：

> **这一方案更接近“结果承载层替换”，而不是“评分逻辑重写”。**

---

## 8. 推荐实现边界

### 8.1 第一阶段目标

只做一件事：

> **让 cache-hit 的只读 query/list/completion 路径，不再先恢复完整 owned `Bookmark` 列表。**

### 8.2 第一阶段不做的事

- 不替换主存储
- 不替换 undo / redo
- 不改变命令面
- 不改变 JSON 主库事实源地位
- 不改变 mutation 路径的 `Store` 语义

### 8.3 推荐技术形态

建议新增两层：

1. `BookmarkArchivedRow<'a>`
   - 直接借用 archived cache 中的字段
   - 提供 `id / name / path / tags / source / pinned / workspace` 等只读访问

2. `RankedBookmarkView<'a>`
   - 替代当前只读场景下的 `RankedBookmark`
   - 输出层直接消费 borrowed 结果

然后保留现有：

- `Bookmark`
- `Store`
- `RankedBookmark`

只给 mutation / save / undo 路径继续使用。

### 8.4 基于 `refer` 的进一步收口

结合 `rkyv-main` 与 `zoxide-main`，推荐的第一阶段技术形态应再补两条约束：

1. **视图必须自带底层 payload owner**
   原因：
   archived `&str` / `&[T]` 都依赖底层 buffer 生命周期，不能裸返回借用对象。

2. **消费层优先走“流式输出”而不是“先收集再打印”**
   原因：
   zoxide 的 `Stream` 已经证明，高频查询路径里“迭代即消费”比“先做完整实体列表”更稳。

---

## 9. 风险评估

### 9.1 风险一：双路径维护成本

如果同时存在：

- borrowed read path
- owned mutate path

就必须防止两者行为漂移。

应对：

- 把评分逻辑继续收敛在同一个 query core
- 只把“结果承载类型”和“读取来源”做分叉

### 9.2 风险二：lifetime 复杂度上升

borrowed view 必须依赖 aligned payload buffer 的生命周期。

应对：

- view 对象必须明确拥有其底层 buffer owner
- 不允许把 borrowed rows 脱离 cache buffer 单独逃逸

### 9.3 风险三：调试复杂度上升

当前 owned `Bookmark` 很直观；borrowed view 会增加理解门槛。

应对：

- 仅把它限制在高频只读路径
- mutation 仍然统一走 owned `Store`

---

## 10. 结论

当前结论已经更新为：

1. **lightweight runtime view 的工程正确性已证明**
2. **它适合继续保留为实验性路径**
3. **但当前不建议把它继续扩张成默认主路径**

一句话建议：

> **这一方案应暂时停在“实验性实现 + 结果存档”阶段，把 owned `Store` 继续保留为正式默认路径。**

---

## 11. 建议的实施门槛

如果进入实现阶段，建议把验收门槛定成：

### 11.1 性能门槛

- `20k` 热命中 `xun __complete bookmark z` 必须稳定优于当前 owned 路径
- `50k` 热命中 `xun __complete bookmark z` 必须稳定优于当前 owned 路径
- 不能只是把 `store_load` 的成本转移到 `query_recall / query_rank`

### 11.2 语义门槛

- 排序结果与当前 owned 路径完全一致
- `--list / --why / --preview / __complete` 输出完全一致
- mutation 路径行为不回归

截至 2026-04-01：

- 语义门槛：**已满足**
- 性能门槛：**未满足**

---

## 12. 参考资料

官方资料：

- rkyv docs.rs：<https://docs.rs/rkyv/latest/rkyv/>
- ArchivedVec：<https://docs.rs/rkyv/latest/rkyv/vec/struct.ArchivedVec.html>
- ArchivedString：<https://docs.rs/rkyv/latest/rkyv/string/struct.ArchivedString.html>
- ArchivedBTreeMap：<https://docs.rs/rkyv/latest/rkyv/collections/btree_map/struct.ArchivedBTreeMap.html>

本地参考：

- [rkyv README](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/README.md)
- [rkyv zero-copy-deserialization](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/book_src/zero-copy-deserialization.md)
- [rkyv ArchivedVec](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/rkyv/src/vec.rs)
- [rkyv ArchivedString](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/rkyv/src/string/mod.rs)
- [rkyv ArchivedBTreeMap impl](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/rkyv-main/rkyv/src/impls/alloc/collections/btree_map.rs)
- [zoxide query](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/zoxide-main/src/cmd/query.rs)
- [zoxide stream](/D:/100_Projects/110_Daily/XunYu/refer/bookmarks/zoxide-main/src/db/stream.rs)
- [bookmark-Benchmark-Suite.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-Benchmark-Suite.md)
- [state.rs](/D:/100_Projects/110_Daily/XunYu/src/bookmark/state.rs)
- [cache.rs](/D:/100_Projects/110_Daily/XunYu/src/bookmark/cache.rs)
- [query.rs](/D:/100_Projects/110_Daily/XunYu/src/bookmark/query.rs)
