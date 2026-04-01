# xun bookmark Benchmark 套件

> 更新时间：2026-04-01
> 关联文档：bookmark-PRD.md · Bookmarks-Feature-Roadmap.md · bookmark-TDD-TaskList.md
>
> 轻量运行时视图状态同步：**实验完成，暂停推进；代码已回退，本文保留基准结果作为记录**

---

## 1. 目标

这份文档定义 bookmark 组件当前已经落地的性能测试与基准入口，作为后续性能回归的统一执行说明。

> 判定口径：**以 release 结果为准**。debug / smoke 路径只用于热点定位与开发期诊断，不作为最终性能结论。

当前目标分三层：

1. query core release benchmark
2. CLI 端到端性能 smoke test
3. `xun bookmark` 与 `bm` 入口对照

---

## 2. 已落地入口

### 2.1 Divan benchmark

文件：

- [bookmark_bench_divan.rs](/D:/100_Projects/110_Daily/XunYu/benches/bookmark_bench_divan.rs)

当前覆盖：

- `bookmark_query_list`
- `bookmark_completion`
- `bookmark_query_explain`
- `bookmark_query_workspace_scope`
- `bookmark_store_load`

数据规模：

- `1_000`
- `5_000`
- `10_000`
- `20_000`
- `50_000`

执行命令：

```powershell
cargo bench --bench bookmark_bench_divan
```

快速编译检查：

```powershell
cargo bench --bench bookmark_bench_divan --no-run
```

---

### 2.2 专项性能测试

文件：

- [bookmark_performance.rs](/D:/100_Projects/110_Daily/XunYu/tests/special/bookmark_performance.rs)

当前覆盖：

- `perf_bookmark_z_list_5000`
- `perf_bookmark_complete_5000`
- `perf_bookmark_complete_working_set_peak`
- `perf_bookmark_complete_memory_attribution_matrix`
- `perf_bookmark_z_list_elapsed_budget`
- `perf_bookmark_release_end_to_end`
- `perf_bm_release_end_to_end`
- `perf_bookmark_release_compare_matrix`
- `perf_bookmark_store_load_20000`
- `perf_bookmark_store_load_50000`
- `perf_bookmark_store_load_20000_compact`
- `perf_bookmark_store_load_50000_compact`
- `perf_bookmark_release_compare_matrix_20000`
- `perf_bookmark_release_compare_matrix_20000_compact`
- `perf_bookmark_release_compare_matrix_20000_binary_cache`
- `perf_bookmark_release_compare_matrix_50000`
- `perf_bookmark_release_compare_matrix_50000_compact`
- `perf_bookmark_release_compare_matrix_50000_binary_cache`
- `perf_release_startup_compare_version_only`
- `perf_bookmark_release_complete_20000_lightweight_view`
- `perf_bookmark_release_complete_50000_lightweight_view`

执行命令：

```powershell
cargo test --test special_bookmark_performance -- --ignored --nocapture
```

单项执行：

```powershell
cargo test --test special_bookmark_performance perf_bookmark_release_compare_matrix -- --ignored --nocapture
```

---

## 3. 环境变量门槛

### 3.1 debug / smoke

| 变量 | 含义 |
|---|---|
| `XUN_TEST_BM_ITEMS` | 数据集大小，默认 `5000` |
| `XUN_TEST_BM_Z_ITERS` | debug `bookmark z --list` 迭代次数 |
| `XUN_TEST_BM_COMPLETE_ITERS` | debug completion 迭代次数 |
| `XUN_TEST_BM_Z_SINGLE_MAX_MS` | debug 单次 `bookmark z --list` 门槛 |
| `XUN_TEST_BM_Z_LIST_AVG_MS` | debug 平均 `bookmark z --list` 门槛 |
| `XUN_TEST_BM_COMPLETE_AVG_MS` | debug completion 平均门槛 |
| `XUN_TEST_BM_COMPLETE_WS_MAX` | completion 工作集峰值门槛 |

### 3.2 release

| 变量 | 含义 |
|---|---|
| `XUN_TEST_BM_RELEASE_ITERS` | release 端到端迭代次数 |
| `XUN_TEST_BM_STORE_LOAD_ITERS` | `Store::load` 专项迭代次数 |
| `XUN_TEST_BM_RELEASE_ITERS_LARGE` | 20k release 对照迭代次数 |
| `XUN_TEST_BM_RELEASE_ITERS_HUGE` | 50k release 对照迭代次数 |
| `XUN_BM_LOAD_TIMING` | 输出 `Store::load` 分阶段耗时与 working set 增量 |
| `XUN_TEST_BM_RELEASE_Z_AVG_MS` | `xun bookmark z --list` release 平均门槛 |
| `XUN_TEST_BM_RELEASE_COMPLETE_AVG_MS` | `xun __complete bookmark z` release 平均门槛 |
| `XUN_TEST_BM_LITE_RELEASE_Z_AVG_MS` | `bm z --list` release 平均门槛 |
| `_BM_INDEX_MIN_ITEMS` | 自适应倒排索引启用阈值，默认 `20000` |

---

## 4. 当前参考结果

> 下列数字为本机 2026-04-01 的实测结果，仅作为当前基线样本，不代表 CI 固定值。

### 4.1 release query benchmark

- `bookmark_query_list(5000)`：约 `2.81ms`
- `bookmark_completion(5000)`：约 `4.46ms`
- `bookmark_query_workspace_scope(5000)`：约 `1.70ms`
- `bookmark_query_explain(5000)`：约 `2.78ms`

### 4.2 release CLI 对照

- `xun bookmark z --list`：约 `80ms`
- `bm z --list`：约 `56ms`
- `xun bookmark zi`：约 `112ms`
- `bm zi`：约 `102ms`
- `xun __complete bookmark z`：约 `56ms`
- `bm completion backend`：约 `45ms`

补充启动基线：

- `xun --version`：约 `27~51ms`
- `bm --version`：约 `17~28ms`
- `xun.exe`：约 `4.79 MiB`
- `bm.exe`：约 `1.03 MiB`

结论：

- `xun` 与 `bm` 的一部分差值发生在进入 bookmark 业务逻辑之前
- 这部分差值更像是总入口装载成本，而不是 `Store::load` 或 query core 的差异
- 继续压缩 `xun` 的方式，应优先围绕总入口瘦身，而不是继续打 bookmark 内核

### 4.3 release timing 拆分

在 `XUN_BM_TIMING=1` 下，5000 条数据的单次样例约为：

```text
bookmark timing [z] db_path=0ms store_load=6ms build_spec=0ms build_ctx=0ms query=2ms handle=0ms total=9ms
```

结论：

- bookmark 组件内部处理在 5k 量级仍较轻
- 端到端剩余耗时主要来自 CLI 进程启动成本与 `store_load`
- 持久化倒排索引已落地，但 20k+ 场景主瓶颈仍然主要是 `store_load`

### 4.4 大库 `store_load` 与 release 对照

> 2026-04-01 起，这里的 binary cache 已经切到 `rkyv` 二进制 payload，并按 `rkyv` 官方高层 checked API 实现：写入使用 `to_bytes`，读取使用 `from_bytes`，由于固定 52-byte header 会打破 payload 对齐，读取前会先拷贝进 `AlignedVec<16>`。

`Store::load` 纯加载对照：

- `Store::load(20_000, compact, cache disabled)`：约 `155ms`
- `Store::load(20_000, compact, warm binary cache hit)`：约 `34ms`
- `Store::load(50_000, compact, cache disabled)`：约 `396ms`
- `Store::load(50_000, compact, warm binary cache hit)`：约 `86ms`

release 命令级对照：

- `xun bookmark z --list`（20k, raw JSON, cache disabled）：约 `252ms`
- `xun bookmark z --list`（20k, compact JSON, cache disabled）：约 `223~237ms`
- `xun bookmark z --list`（20k, warm binary cache hit）：约 `65ms`
- `bm z --list`（20k, raw JSON, cache disabled）：约 `235ms`
- `bm z --list`（20k, compact JSON, cache disabled）：约 `208ms`
- `bm z --list`（20k, warm binary cache hit）：约 `54ms`
- `xun __complete bookmark z`（20k, raw JSON, cache disabled）：约 `258ms`
- `xun __complete bookmark z`（20k, compact JSON, cache disabled）：约 `255ms`
- `xun __complete bookmark z`（20k, warm binary cache hit）：约 `73~98ms`
- `xun bookmark z --list`（50k, raw JSON, cache disabled）：约 `843ms`
- `xun bookmark z --list`（50k, compact JSON, cache disabled）：约 `520ms`
- `xun bookmark z --list`（50k, warm binary cache hit）：约 `119ms`
- `bm z --list`（50k, raw JSON, cache disabled）：约 `504ms`
- `bm z --list`（50k, compact JSON, cache disabled）：约 `499ms`
- `bm z --list`（50k, warm binary cache hit）：约 `111ms`
- `xun __complete bookmark z`（50k, raw JSON, cache disabled）：约 `613ms`
- `xun __complete bookmark z`（50k, compact JSON, cache disabled）：约 `602ms`
- `xun __complete bookmark z`（50k, warm binary cache hit）：约 `133~206ms`
- `xun __complete bookmark z`（20k, cache-hit owned vs lightweight）：约 `55ms vs 56ms`
- `xun __complete bookmark z`（50k, cache-hit owned vs lightweight）：约 `94ms vs 98ms`
- `xun __complete bookmark z`（20k, 串行 warm-hit timing 样本）：约 `owned 25ms vs borrowed 18ms`
- `xun __complete bookmark z`（50k, 串行 warm-hit timing 样本）：约 `owned 60ms vs borrowed 46ms`

结论：

- 20k 以上场景当前仍明显受 `store_load` 主导
- `Store::load` 的主要热点仍然是 JSON 解析本身
- `rkyv` binary cache 已经把 `Store::load` 压到原 compact JSON 的约 `22%`
- 命令级收益也已经明确，不再是早期“JSON payload cache”那种无效原型
- 对 `z --list`，20k 提速约 `3.6x`，50k 提速约 `4.4x`
- 对 `__complete`，20k 提速约 `2.6x`，50k 提速约 `2.9x`
- 旧结论“binary cache + JSON payload 不值得默认开启”已经失效；当前应以 `rkyv` payload 结果为准
- `xun bookmark ...` 已增加前置 fast-path，能绕开顶层大 `argh` 解析；对普通 bookmark 命令有局部收益，但不会改变 `xun` 与 `bm` 的总装载体积差
- `bookmark completion/query` 已追加一轮轻量优化：
  候选召回后再计算 `global_max`，并在 name/basename 命中高分时跳过路径分段与 subsequence 模糊评分
- 分段调试显示，20k 热命中时 `complete.bookmark` 里：
  `query_recall≈1ms`、`query_rank≈4ms`，其它 query 子阶段接近 `0ms`
- 这说明 completion 当前剩余主要成本已重新回到 `store_load` 和总入口装载，而不再是 query 内核本身
- 进一步拆解 cache-hit 的 `store_load` 可见：
  - 20k 热命中样本：`read_cache_file≈1ms`、`deserialize_cache_payload≈15ms`、`materialize_cache_payload≈2ms`、`deserialize_cache_index≈0ms`
  - 50k 热命中样本：`read_cache_file≈2~3ms`、`deserialize_cache_payload≈4ms`、`materialize_cache_payload≈12ms`、`deserialize_cache_index≈6ms`
- 因此下一阶段如果继续打性能，优先级应是：
  `cache payload materialize` > `总入口装载` > `query 内核`
- 当前已落地的有效收口：
  - embedded index 恢复跳过指纹重算与二次清洗
  - binary cache payload 移除 `name_norm / path_norm` 这类可推导字段，加载时再重建
  - embedded index 从 archived 结构直接恢复，索引结构从 `BTreeMap` 收口为按 term 排序的 `Vec`
- lightweight runtime view 的实验性结论：
  - 工程正确性已验证
  - cache-hit 下 borrowed 路径能显著压低 `store_load`
  - 会把更多成本转移到 `query_recall / query_rank`
  - 在部分样本中 `__complete` release 对照与串行 warm-hit 样本显示净收益
  - 但收益不够稳定，20k 档位存在波动，当前不足以继续作为主线推进
  - 当前最终决策是：实验完成，暂停推进，代码已回退，结果保留为记录
- 已验证无收益并回退的尝试：
  - 将 persisted index 项从 `usize` 压到 `u32`

### 4.5 completion 内存归因

- 空 store：约 `2.43 MiB`
- 5k，无索引：约 `25.38 MiB`
- 5k，强制索引冷启动：约 `24.65 MiB`
- 5k，强制索引热启动：约 `24.87 MiB`
- 20k，强制索引冷启动：约 `68.66 MiB`
- 20k，强制索引热启动：约 `71.86 MiB`

结论：

- 5k 场景下，bookmark 本体大约贡献 `22.95 MiB`
- 20k 场景下，bookmark 本体大约贡献 `66~69 MiB`
- 当前 completion 内存压力主要来自主库数据本身，而不是空进程基线

### 4.6 `Store::load` 调试结论

> 本节仅用于解释热点来源，不作为性能判定结论。

在 `XUN_BM_LOAD_TIMING=1` 下，20k / 50k 的样例显示：

- 20k：`parse_store_file` 约 `91~105ms`，`normalize` 约 `2~4ms`
- 50k：`parse_store_file` 约 `232~240ms`，`normalize` 约 `6ms`
- compact 主库后：
  - 20k：`parse_store_file` 约 `63~66ms`
  - 50k：`parse_store_file` 约 `160~163ms`

结论：

- 当前 `store_load` 的主热点是 JSON 解析，不是规范化
- 后续再继续优化时，应优先围绕主库存储体积与解析成本，而不是继续堆查询层技巧

---

## 5. 推荐执行顺序

1. 开发阶段：`cargo bench --bench bookmark_bench_divan --no-run`
2. 局部优化后：`cargo bench --bench bookmark_bench_divan`
3. 端到端确认：`cargo test --test special_bookmark_performance -- --ignored --nocapture`
4. 双入口对照：`perf_bookmark_release_compare_matrix`

---

## 6. 结论

当前 bookmark 已具备：

- 可重复执行的 benchmark 入口
- 可通过环境变量设门槛的性能 smoke test
- `xun bookmark` vs `bm` 的 release 对照
- `store_load / query / handle` 阶段级 timing 调试

后续如要继续正式化，可以直接把这些命令挂入 CI。
