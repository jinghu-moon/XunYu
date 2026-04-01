# xun bookmark 阶段性总结

> 更新时间：2026-04-01  
> 状态：阶段收口  
> 关联文档：bookmark-PRD.md · bookmark-Benchmark-Suite.md · bookmark-Binary-Cache-Design.md · bookmark-SQLite-Evaluation.md · bookmark-Lightweight-Runtime-View-Evaluation.md

---

## 1. 当前定位

当前 bookmark 组件已经完成从早期“顶层零散命令”到“正式子命名空间组件”的收口。

当前正式入口：

- `xun bookmark <sub>`
- `bm <sub>`

当前主线定位：

> **bookmark 已经是一套完整的显式书签 + 自动学习 + 统一查询 + Shell 集成的开发者导航组件。**

---

## 2. 已落地主线能力

### 2.1 命令面

当前正式命令面已经稳定为：

- 导航：
  `z / zi / o / oi`
- 显式管理：
  `save / set / delete / rename / pin / unpin`
- 标签与列表：
  `tag / list / recent / stats / keys / all`
- 治理：
  `check / gc / dedup`
- 集成：
  `import / export / init / learn / touch`
- 历史：
  `undo / redo`

### 2.2 数据模型

当前数据模型已经稳定支持：

- `explicit / imported / learned`
- `pinned`
- `tags`
- `workspace`
- `desc`
- `visit_count / last_visited / frecency_score`

### 2.3 查询与排序

当前主线查询能力已经具备：

- 多 token AND 匹配
- name / basename / path segment / tag 混合匹配
- frecency 排序
- workspace / child / global / base scope
- `--why / --score / --preview`
- completion 与 query core 一致

### 2.4 集成能力

当前主线集成能力已经具备：

- PowerShell / bash / fish init 模板
- auto learn
- 外部生态导入
- `bm` 轻量高频入口

---

## 3. 当前主线架构

### 3.1 存储

当前正式主线仍然是：

- 主库：`~/.xun.bookmark.json`
- binary cache：`.xun.bookmark.cache`
- 访问/历史相关日志

原则上：

> **JSON 主库仍然是唯一事实源，binary cache 只是 fast-load 派生层。**

### 3.2 历史模型

当前 undo / redo 已经不是早期 snapshot-based，而是：

> **delta-based 历史模型**

这使得 bookmark 的长期可维护性和回滚粒度已经达到正式可用水平。

### 3.3 性能主线

当前真正有效的主线优化包括：

- compact JSON 主库
- `rkyv` binary cache
- embedded index
- completion/query 热点拆解与定向优化
- `bm` 高 频入口

---

## 4. 当前性能结论

根据 [bookmark-Benchmark-Suite.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-Benchmark-Suite.md) 当前样本：

### 4.1 主线结论

- `rkyv` binary cache 是有效优化
- `Store::load` 相比 compact JSON 已出现显著下降
- `bm` 仍然是高频使用时更轻的入口
- `xun` 与 `bm` 的固定差值更多来自总入口装载成本

### 4.2 当前热点

当前热点已经不再是 query 内核本身，而是：

- cache-hit 后的 payload 恢复
- 以及 `xun` 总入口的装载成本

一句话：

> **bookmark 内核已经不慢，当前主线剩余成本更多是载入和入口层成本。**

---

## 5. 已完成但暂停推进的实验

### 5.1 轻量运行时视图

`lightweight runtime view` 实验已经完整做过一轮：

- 契约层
- 借用模型层
- 缓存读取层
- 查询层
- 消费层
- 边界层
- 性能层

结论记录见：

- [bookmark-Lightweight-Runtime-View-Evaluation.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-Lightweight-Runtime-View-Evaluation.md)
- [bookmark-Lightweight-Runtime-View-TDD-TaskList.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-Lightweight-Runtime-View-TDD-TaskList.md)

当前最终决策：

> **实验完成，暂停推进；代码已回退，文档保留为记录。**

原因很简单：

- 工程正确性成立
- 在部分大规模 cache-hit 只读热路径上有收益
- 但收益不够稳定，不足以支撑继续作为当前主线推进

### 5.2 SQLite

SQLite 评估也已完成，但当前结论仍然是：

> **暂不切 SQLite**

原因：

- 当前 JSON + binary cache 主线仍然能承载正式阶段
- SQLite 的复杂度暂时高于当前收益

---

## 6. 当前正式结论

当前 bookmark 的最准确阶段判断是：

1. **主线功能已经完整可用**
2. **主线性能优化已经形成有效闭环**
3. **实验性路线已做验证，但不继续作为主线投入**

一句话总结：

> **bookmark 当前已经具备正式可用的功能与性能基线；后续应以稳定维护、边际优化和总入口瘦身为主，而不是再开启高复杂度新路线。**

---

## 7. 下一步建议

当前更值得继续投入的方向是：

### 7.1 第一优先级

- 继续瘦身 `xun` 总入口
- 保持 `bm` 作为高频主入口

### 7.2 第二优先级

- 继续优化主线 cache-hit 恢复成本
- 仅在现有主线内做边际性能收口

### 7.3 暂不建议

- 不继续推进 lightweight runtime view 主线化
- 不立即切 SQLite

---

## 8. 参考文档索引

- [bookmark-PRD.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-PRD.md)
- [bookmark-Benchmark-Suite.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-Benchmark-Suite.md)
- [bookmark-Binary-Cache-Design.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-Binary-Cache-Design.md)
- [bookmark-Binary-Cache-TDD-TaskList.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-Binary-Cache-TDD-TaskList.md)
- [bookmark-SQLite-Evaluation.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-SQLite-Evaluation.md)
- [bookmark-Undo-Redo-Evaluation.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-Undo-Redo-Evaluation.md)
- [bookmark-Lightweight-Runtime-View-Evaluation.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-Lightweight-Runtime-View-Evaluation.md)
- [bookmark-Lightweight-Runtime-View-TDD-TaskList.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/bookmark/bookmark-Lightweight-Runtime-View-TDD-TaskList.md)
