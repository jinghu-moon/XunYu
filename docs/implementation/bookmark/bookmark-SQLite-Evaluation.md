# xun bookmark SQLite 评估

> 更新时间：2026-03-31  
> 关联文档：bookmark-PRD.md · Bookmarks-Feature-Roadmap.md · bookmark-Benchmark-Suite.md

---

## 1. 目标

这份文档回答三个问题：

1. 当前 bookmark 是否应该立即从 JSON 主存储切换到 SQLite
2. 什么时候值得切
3. 如果切，迁移边界和最小方案是什么

结论先行：

> **当前不建议立即切 SQLite。应保留现有 JSON 主存储作为 v1 正式实现，把 SQLite 作为“数据规模继续上升或并发写场景出现后”的后续迁移路径。**

---

## 2. 当前实现基线

### 2.1 主存储形态

当前 bookmark 主存储：

- 主库：`~/.xun.bookmark.json`
- 访问日志：`~/.xun.bookmark.visits.jsonl`
- 历史：`.xun.bookmark.undo.log` / `.xun.bookmark.redo.log`

对应实现：

- [state.rs](/D:/100_Projects/110_Daily/XunYu/src/bookmark/state.rs)
- [storage.rs](/D:/100_Projects/110_Daily/XunYu/src/bookmark/storage.rs)
- [undo.rs](/D:/100_Projects/110_Daily/XunYu/src/bookmark/undo.rs)

### 2.2 当前优化状态

当前已经落地的关键优化：

- 统一 query core
- 持久化倒排索引（JSON sidecar）+ 进程内索引缓存
- delta-based `undo / redo`
- 进程内配置缓存
- `bm.exe` 轻量高频入口
- release 端到端性能 smoke test

### 2.3 当前性能基线

以 [bookmark-Benchmark-Suite.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-Benchmark-Suite.md) 当前样本为准：

- `bookmark_query_list(5000)`：约 `2.98ms`
- `bookmark_completion(5000)`：约 `4.83ms`
- `xun bookmark z --list`：约 `44~66ms`（受进程启动波动影响）
- `bm z --list`：约 `32~41ms`

阶段 timing 样例显示：

```text
bookmark timing [z] db_path=0ms store_load=14~15ms build_spec=0ms build_ctx=0ms query=13~15ms handle=0ms total=28~31ms
```

这说明：

- 当前瓶颈仍主要集中在 `store_load + query`
- 但整体仍处于“可用且已优化”的状态

---

## 3. SQLite 能带来什么

### 3.1 理论优势

SQLite 对 bookmark 的潜在收益主要在四类场景：

1. **更大规模数据集**
   当前 5k~10k 条数据仍可接受；当数据进入 20k~100k 量级时，JSON 反序列化和全量内存构建会更吃亏。

2. **更复杂查询**
   如果后续需要：
   - 多条件过滤
   - workspace/tag/source/pin 的组合查询
   - 排序前候选集裁剪
   - 更稳定的分页  
   SQLite 的索引能力会更自然。

3. **更细粒度写入**
   当前写操作是整库重写。若未来写频率更高，SQLite 可以把写放到单条记录级别。

4. **多消费者并发**
   如果后续 Dashboard、CLI、后台 hook 同时频繁读写，SQLite 的一致性和锁语义会比手工 JSON + WAL 更稳。

### 3.2 不会自动解决的问题

SQLite **不会**自动解决这些问题：

1. **进程启动成本**
   当前 `xun bookmark` 和 `bm` 的差异主要来自 CLI 启动路径，SQLite 并不能消除这部分固定成本。

2. **shell 集成成本**
   `init`、completion、wrapper 心智不会因为换数据库而变简单。

3. **网络路径 dead-link 检查**
   这是路径探测策略问题，不是底层存储问题。

4. **undo/redo 的语义设计**
   undo/redo 是历史建模问题，不是数据库类型问题。

---

## 4. 当前不立即迁移的原因

### 4.1 当前 JSON 方案还没有到“明显失控”

目前 JSON + 访问日志 + undo/redo 日志的组合，已经具备：

- 可读性
- 易调试
- 零外部依赖
- 可直接手工修复
- 单文件分发简单

对当前 bookmark v1 来说，这些优势仍然很重要。

### 4.2 SQLite 会显著增加实现复杂度

一旦切 SQLite，需要额外解决：

- schema 设计
- migration 链
- 初始化/损坏恢复
- 事务边界
- WAL/日志模式
- 备份/导出策略
- 跨入口兼容
- 测试 fixture 全量重写

对当前阶段来说，这些复杂度并不便宜。

### 4.3 现在最大的痛点还不是“JSON 太慢”

根据当前基线：

- 小中规模查询已经被优化到可接受范围
- `bm.exe` 已经显著降低高频入口成本
- 持久化倒排索引 + 进程内缓存已部分解决候选召回问题

所以现在直接切 SQLite，收益并不一定高于成本。

---

## 5. 建议的切换触发条件

建议采用明确触发条件，而不是“觉得该切了就切”。

### 5.1 性能触发

满足以下任一条件时，进入 SQLite 实施阶段：

- 常见数据集稳定超过 `20_000` 条
- `bm z --list` release 平均值长期高于 `50ms`
- completion 平均值长期高于 `80ms`
- `store_load` 在 release timing 中稳定超过 `25ms`

### 5.2 功能触发

满足以下任一条件时，进入 SQLite 实施阶段：

- 需要稳定分页与过滤组合查询
- Dashboard/CLI/后台 hook 出现明显并发读写
- 需要记录级更新而不再接受整库重写

---

## 6. 推荐迁移策略

### 6.1 不做“直接替换”

推荐策略：

1. 保留 JSON 作为 v1 正式主存储
2. 在 v2 引入 SQLite 可选后端
3. 先实现“读 JSON → 导入 SQLite”
4. 迁移稳定后再决定是否切默认后端

### 6.2 最小 SQLite 设计

如果未来进入实施阶段，建议最小表结构如下：

#### `bookmarks`

- `id TEXT PRIMARY KEY`
- `name TEXT NULL`
- `name_norm TEXT NULL`
- `path TEXT NOT NULL`
- `path_norm TEXT NOT NULL`
- `source TEXT NOT NULL`
- `pinned INTEGER NOT NULL`
- `desc TEXT NOT NULL`
- `workspace TEXT NULL`
- `created_at INTEGER NOT NULL`
- `last_visited INTEGER NULL`
- `visit_count INTEGER NULL`
- `frecency_score REAL NOT NULL`

索引建议：

- `UNIQUE(name_norm) WHERE name_norm IS NOT NULL`
- `INDEX(path_norm)`
- `INDEX(workspace)`
- `INDEX(source, pinned)`

#### `bookmark_tags`

- `bookmark_id TEXT NOT NULL`
- `tag_norm TEXT NOT NULL`
- `tag_display TEXT NOT NULL`

索引建议：

- `INDEX(bookmark_id)`
- `INDEX(tag_norm)`

#### `bookmark_events`

用于替代当前访问日志和历史操作日志：

- `id INTEGER PRIMARY KEY`
- `kind TEXT NOT NULL`
- `bookmark_id TEXT NULL`
- `payload_json TEXT NOT NULL`
- `created_at INTEGER NOT NULL`

---

## 7. 与现有代码的边界

如果未来迁移，建议保持这些边界不变：

- 命令面不变：`bookmark` 子命令树不改
- query core 保持接口稳定
- shell init / completion 不感知后端差异
- undo/redo 语义不变

也就是说：

> **换的是存储后端，不是 bookmark 的产品心智。**

---

## 8. 当前建议

当前阶段的建议非常明确：

### 8.1 现在不切 SQLite

理由：

- 当前 JSON 方案仍能承载 v1
- 性能已通过轻入口、索引、缓存做过一轮优化
- 现在切换成本高于收益

### 8.2 保持评估结论待命

现在应该做的是：

- 保留本评估文档
- 在 benchmark 文档里持续记录数据
- 真正触发条件出现后再进入迁移实现

---

## 9. 一句话结论

> **SQLite 对 bookmark 来说是“明确可行的下一阶段后端”，但不是当前阶段必须立刻切换的后端。当前最合理策略是保留 JSON 主存储，把 SQLite 作为有触发条件的 v2 迁移方案。**
