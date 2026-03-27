# Backup 优化路线图 — TDD 分阶段任务清单

> 依据：[Backup-Optimization-Roadmap.md](./Backup-Optimization-Roadmap.md)
> 标记：`[ ]` 待办　`[-]` 进行中　`[x]` 完成
> 原则：**红-绿-重构**。每个任务先写失败测试，再写最小实现，最后重构。
> 范围：覆盖传统 `backup`、`.xunbak` 7-Zip 插件、恢复链路、校验链路、发布与运维体验。
> 说明：本清单面向“下一阶段优化”，默认不追求兼容历史 PoC 脚本行为，但避免无必要的用户可见破坏性变更。
> 实现策略：**优先参考同类型项目，在 XunYu 内手写相关功能**；不把外部 crate 当成长期唯一实现载体。

---

## Phase 0：优化基线冻结

### 0.0 参考实现边界

- [x] 确认 `zip2-master` 作为 ZIP 行为参考，而不是最终唯一实现
- [x] 确认 `sevenz-rust2-main` / `7z2600-src` 作为 7z 行为参考，而不是最终唯一实现
- [x] 确认插件继续沿用“C++ 薄壳 + Rust 核心”分层

### 0.1 现状记录

- [x] 复核当前基线文档中的测试环境和版本信息
- [x] 确认 7-Zip 安装目录探测策略
- [x] 确认 `.xunbak` 插件 PoC 当前只读能力范围
- [x] 确认传统 `backup` 当前 `diff-mode=auto|hash|meta` 行为不变

### 0.2 基线验证

- [x] **测试**：`cargo check --tests --benches` 通过
- [x] **测试**：`cargo test --test module_backup_restore` 通过
- [x] **测试**：`.xunbak` 插件系统联调脚本当前可跑通
- [x] 将验证命令与结果写入阶段备注

---

## Phase 1：7-Zip 插件安装器与 doctor

### 1.0 已有脚本原型

- [x] 已有 `build_xunbak_7z_plugin.ps1`
- [x] 已有 `install_xunbak_7z_plugin.ps1`
- [x] 已有 `uninstall_xunbak_7z_plugin.ps1`
- [x] 已有 `smoke_xunbak_7z_plugin.ps1`
- [x] 已有 `test_xunbak_7z_plugin_portable.ps1`
- [x] 已有 `test_xunbak_7z_plugin_system.ps1`
- [x] 已有 `accept_xunbak_7z_plugin.ps1`
- [x] 说明：当前缺的是统一 `xun xunbak plugin ...` CLI 封装，而不是底层脚本能力

### 1.1 CLI 与脚本边界

- [x] 新增 `xun xunbak plugin install`
- [x] 新增 `xun xunbak plugin uninstall`
- [x] 新增 `xun xunbak plugin doctor`
- [x] 明确仅支持“已安装 7-Zip”的场景，不负责分发 7-Zip 本体

### 1.2 安装路径探测

- [x] **测试**：优先探测显式传入的 `--sevenzip-home`
- [x] **测试**：可探测 `C:/A_Softwares/7-Zip`
- [x] **测试**：可探测 `C:/Program Files/7-Zip`
- [x] **测试**：未探测到 7-Zip 时返回明确错误
- [x] 实现安装目录探测逻辑

### 1.3 插件安装

- [x] **测试**：`install` 会复制 `xunbak.dll` 到 `7-Zip/Formats`
- [x] **测试**：目标 DLL 已存在时支持覆盖或明确拒绝
- [x] **测试**：缺少本地构建产物时返回明确错误
- [x] 实现安装命令

### 1.4 插件卸载

- [x] **测试**：`uninstall` 删除 `7-Zip/Formats/xunbak.dll`
- [x] **测试**：目标 DLL 不存在时返回幂等结果
- [x] 实现卸载命令

### 1.5 doctor

- [x] **测试**：`doctor` 输出 DLL 是否存在
- [x] **测试**：`doctor` 输出 `.xunbak` 文件关联状态
- [x] **测试**：`doctor` 输出 7-Zip 主程序路径和版本
- [x] **测试**：`doctor` 输出建议修复动作
- [x] 实现诊断命令

---

## Phase 2：`.xunbak` 文件关联

### 2.1 关联状态识别

- [x] **测试**：能识别 `.xunbak` 未关联
- [x] **测试**：能识别 `.xunbak` 已关联到 `7zFM.exe`
- [x] **测试**：能识别关联到非 7-Zip 的程序
- [x] 实现关联状态探测

### 2.2 建立关联

- [x] **测试**：`install --associate` 会建立 `.xunbak -> 7zFM.exe`
- [x] **测试**：重复执行不会生成脏状态
- [x] **测试**：关联失败时返回明确错误与回滚提示
- [x] 实现关联写入逻辑

### 2.3 移除关联

- [x] **测试**：`uninstall --remove-association` 移除 `.xunbak` 关联
- [x] **测试**：若关联不是 7-Zip，不会误删第三方绑定
- [x] 实现关联移除逻辑

---

## Phase 3：7-Zip 插件大文件打开路径优化

### 3.1 callback 优先

- [x] **测试**：单文件 `.xunbak` 优先走 `xunbak_open_with_callbacks`
- [x] **测试**：分卷 `.xunbak.001` 优先走 callback volume 打开
- [x] **测试**：callback 成功时不落入整文件 `ReadAll`
- [x] 改造 `OpenCore()` 打开路径

### 3.2 fallback 收敛

- [x] **测试**：小文件 callback 失败时可安全 fallback 到内存打开
- [x] **测试**：超阈值大文件 callback 失败时不做 `ReadAll`
- [x] **测试**：大文件 fallback 被拒绝时提示明确原因
- [x] 增加 fallback 阈值策略

### 3.3 打开性能

- [x] **bench**：单文件 `.xunbak` 插件打开时间
- [x] **bench**：分卷 `.xunbak.001` 插件打开时间
- [x] **bench**：大文件 callback 路径与 memory fallback 路径对比
- [x] 记录基线

---

## Phase 4：插件属性列与归档信息增强

### 4.1 item property

- [x] **测试**：显示 `Path`
- [x] **测试**：显示 `Size`
- [x] **测试**：显示 `Packed Size`
- [x] **测试**：显示 `Method`
- [x] **测试**：显示 `Modified`
- [x] **测试**：显示 `Created`
- [x] **测试**：显示 `Attributes`
- [x] 完善 `GetProperty` / `GetPropertyInfo`

### 4.2 archive property

- [x] **测试**：显示 `Read Only`
- [x] **测试**：显示 `Files`
- [x] **测试**：显示 `Volumes`
- [x] **测试**：显示 `Physical Size`
- [x] 完善 `GetArchiveProperty` / `GetArchivePropertyInfo`

### 4.3 GUI 展示验收

- [ ] **手工清单**：7-Zip GUI 中列头显示合理
- [ ] **手工清单**：`7z l -slt` 输出关键信息完整
- [ ] **手工清单**：中文路径显示无插件侧乱码

---

## Phase 5：传统 backup 的真实 `file_id`

### 5.1 扫描阶段采集

- [x] **测试**：Windows 下可采集文件 `file_id`
- [x] **测试**：采集失败时为 `None`，不影响主流程
- [x] 在 `scan.rs` 中接入真实 `file_id`

### 5.2 hash cache 接入

- [x] **测试**：`file_id` 相同且元数据相符时命中缓存
- [x] **测试**：`file_id` 变化时正确失效缓存
- [x] 在 `hash_cache` 中消费真实 `file_id`

### 5.3 rename-only 场景

- [x] **测试**：同内容 rename-only 且 `file_id` 相同，不触发重复 hash
- [x] **测试**：路径变化但 `file_id` 相同的场景能更快命中
- [x] 优化 diff / cache 命中策略

### 5.4 性能基线

- [x] **bench**：rename-only 场景接入 `file_id` 前后对比
- [x] **bench**：冷缓存小改动场景前后对比
- [x] 记录基线

---

## Phase 6：恢复链路顺序读取与 reader 复用

### 6.1 reader 复用

- [x] **说明**：reader cache 已覆盖 `.xunbak / zip / 7z`；`zip / 7z` 当前已建立共享 reader + entry index cache，但 `7z` 的 `open_entry_reader()` 仍会把目标 entry materialize 到临时文件

- [x] **测试**：多文件恢复时同一 artifact reader 可复用
- [x] **测试**：复用 reader 不改变恢复结果
- [x] 改造 restore 内部 reader 生命周期

### 6.1.1 `zip / 7z` artifact cache 对齐

- [x] **测试**：`zip` 多文件 convert / restore 不为每个 entry 重建 `ZipArchive`
- [x] **测试**：`7z` 多文件 convert / restore 不为每个 entry 重开 `ArchiveReader`
- [x] **测试**：entry name lookup 从线性扫描收敛为索引命中
- [x] 为 `zip / 7z` 引入 reader + entry index cache，与 `.xunbak` 路径对齐

### 6.2 顺序读取

- [x] **测试**：`.xunbak` 多文件恢复按更优顺序读取
- [x] **测试**：`7z` 多文件恢复不退化为明显随机读取
- [x] **测试**：`7z` entry open path 不再经过 `read_7z_file() -> Vec<u8> -> temp file` 的整块缓冲
- [x] 优化批量恢复的调度顺序
- [x] 优化 `7z` entry reader，避免大文件双重落地与内存放大
  当前已改为共享 `ArchiveReader` + entry index cache，并通过 `EntryReader::Stream` 直接流式读取目标 entry，不再经过 `Vec<u8>` 或临时文件

### 6.3 preview 快速路径

- [x] **测试**：preview 不为全部候选文件打开内容流
- [x] **测试**：同 size/mtime/属性的预览走快速判定
- [x] 优化 preview 数据获取

### 6.4 artifact -> artifact 流式转换

- [x] **测试**：`convert -> xunbak` 在大文件场景不先完整 materialize staging 目录
- [x] **测试**：artifact 互转链路的峰值磁盘占用不因 staging 目录明显放大
- [x] 收敛 `convert -> xunbak` 的 staging copy，评估直接 artifact-to-blob 流式写入
  当前已改为 `SourceEntry -> xunbak::writer::VirtualBackupEntry` 直接流式写入，不再创建 `xunbak-convert` staging 目录

---

## Phase 7：传统 backup 报告增强

### 7.1 新增统计字段

- [x] **测试**：输出 `rename_only_count`
- [x] **测试**：输出 `reused_bytes`
- [x] **测试**：输出 `cache_hit_ratio`
- [x] **测试**：输出 `baseline_source`
- [x] 扩展命令行文本报告

### 7.2 JSON 输出

- [x] **测试**：JSON 输出包含以上字段
- [x] **测试**：字段在 `skipped / dry_run / ok` 三种状态下稳定存在
- [x] 扩展 JSON 视图结构

### 7.3 `.bak-meta.json`

- [x] **测试**：新增统计字段进入 `.bak-meta.json`
- [x] **测试**：旧 meta 缺字段时仍可读取
- [x] 扩展 meta 持久化结构

---

## Phase 8：`verify` 分级模式

### 8.1 CLI 模式

- [x] 新增 `quick`
- [x] 新增 `full`
- [x] 新增 `manifest-only`
- [x] 新增 `existence-only`

### 8.2 语义测试

- [x] **测试**：`manifest-only` 只校验 manifest 结构与条目映射
- [x] **测试**：`existence-only` 只校验文件是否存在
- [x] **测试**：`quick` 提供快速完整性检查
- [x] **测试**：`full` 执行完整内容校验

### 8.3 错误报告

- [x] **测试**：错误输出包含首个失败路径
- [x] **测试**：zip / 7z / xunbak 错误输出包含来源上下文
- [x] **测试**：分卷错误输出包含卷号或卷名
- [x] 优化错误文案

---

## Phase 9：导出链路元数据对齐

### 9.1 目录与压缩格式一致性

- [x] **测试**：`dir -> zip` 保留 mtime / readonly
- [x] **测试**：`dir -> 7z` 保留 mtime / readonly
- [x] **测试**：`xunbak -> dir/zip/7z` 的路径与时间元数据一致

### 9.2 sidecar 与 packed 信息

- [x] **测试**：sidecar 中格式信息与导出结果一致
- [x] **测试**：packed size / codec 信息在支持场景下可对齐
- [x] 优化 sidecar 与导出元数据

---

## Phase 10：手写 ZIP / 7z 方法集扩展

### 10.1 ZIP backend（方案 C）

- [-] **说明**：ZIP 目标方法集固定为 `stored / deflated / bzip2 / zstd / ppmd`，不包含 `lzma2`；当前已接入全部五种方法，其中 `ppmd` 走 XunYu 自己的纯 Rust 手写 writer + manual parser
- [x] **现状**：当前已实现 `stored / deflated / bzip2 / zstd / ppmd`
- [x] **测试**：保留当前 `stored / deflated` 行为不回归
- [x] **测试**：当前 ZIP backend 可写出 `bzip2`
- [x] **测试**：当前 ZIP backend 可写出 `zstd`
- [x] **测试**：手写 ZIP backend 可写出 `ppmd`
- [-] **测试**：ZIP local header / central directory / EOCD 结构可被 `zip` / 7-Zip reopen
  当前 `bzip2 / zstd` 可被 `zip` crate 与 7-Zip reopen，`ppmd` 可被 XunYu parser 与 7-Zip reopen；上游 `zip` crate `2.4.2` 本身仍缺 `ppmd` 解压
- [x] **测试**：Zip64 在手写 backend 下仍正确
  已完成单条目 `4 GiB + 1 MiB` 的 `ZIP ppmd` 端到端验证：纯 Rust 写出、stock `7-Zip 24.09` `7z t` 通过、XunYu 自己 `convert -> dir` 恢复并通过偏移 marker 对比
- [-] 在 XunYu 内实现 ZIP method routing / writer backend

### 10.2 7z method 扩展

- [x] **现状**：当前已实现 `copy / lzma2`
- [x] **测试**：`SevenZMethod` 扩展为 `copy / lzma2 / bzip2 / deflate / ppmd / zstd`
- [x] **测试**：显式 `--method bzip2` 生效
- [x] **测试**：显式 `--method deflate` 生效
- [x] **测试**：显式 `--method ppmd` 生效
- [x] **测试**：显式 `--method zstd` 生效
- [x] **测试**：method id 与 `7z2600-src` / `sevenz-rust2-main` 一致
- [x] 在 XunYu 内实现 7z method routing / writer options
- [x] **说明**：当前 `7z --split-size` 已改为直接写入分卷 sink，不再经过 `tmp.single.7z`
- [x] **测试**：`7z --split-size` 写出不再生成完整临时单文件归档
- [x] **测试**：大归档分卷写出时峰值磁盘占用不出现明显 2x 放大
- [x] 实现真正的 split writer / virtual continuous sink

### 10.3 兼容矩阵与提示

- [-] **测试**：ZIP `bzip2 / zstd / ppmd` 的 reopen 行为稳定
  当前已验证 `bzip2 / zstd` 可被 `zip` crate 与 7-Zip reopen；`ppmd` 可被 XunYu parser 与 7-Zip reopen
- [x] **测试**：7z `bzip2 / deflate / ppmd` 在 stock 7-Zip 可解
  当前 `ppmd` 已修复为纯 Rust 写出，可通过 stock `7-Zip 24.09` 的 `7z t`
- [x] **测试**：7z `zstd` 在支持外部 codec 的解压端可解
  已在隔离临时 7-Zip 副本中注入 `7-Zip-zstd` `zstd.dll`，并验证 `7z t` 通过
- [x] **测试**：doctor / 文档能对 `zstd` codec 兼容差异给出提示
- [x] 记录方法级兼容矩阵
- [x] **说明**：当前 `--verify-output on` 已不再只做“结构级 reopen”
- [x] **测试**：`--verify-output on` 对 ZIP / 7z 的数据区损坏可做内容级解码校验，而不只依赖结构 reopen
- [x] **测试**：无外部 `7z` 环境时，7z 输出校验仍能覆盖内容流解码
- [x] 明确并收敛“结构级 verify”和“内容级 verify”的实现语义与提示文案
  当前语义：
  1. ZIP：先解析条目结构，再逐条目内容解码；`ppmd` 走 manual parser/decoder
  2. 7z：先内部解析 archive 结构，再逐条目内容解码；若存在外部 `7z`，再追加兼容性 `7z t`
  3. 因此 `--verify-output on` 现在是“内部内容级 verify + 外部兼容性 verify（可用时）”，而不再只是 reopen

---

## Phase 11：插件发布化

### 11.1 Release 构建

- [x] **测试**：Release 模式构建出 `xunbak.dll`
- [x] **测试**：Release 插件在目标 7-Zip 版本可加载
- [x] 固化 Release 构建脚本
  当前 `build_xunbak_7z_plugin.ps1 -Config Release` 已稳定可用；限制是同一 `build/xunbak-7z-plugin` 目录暂不适合并发调用

### 11.2 版本矩阵

- [x] **测试清单**：7-Zip 24.x
- [x] **测试清单**：7-Zip 26.x
- [x] **测试清单**：便携版 / 安装版
- [x] 记录兼容矩阵
  已新增 [Xunbak-7z-Plugin-Release-Matrix.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Xunbak-7z-Plugin-Release-Matrix.md)，当前记录了 `Release + 7-Zip 24.09 / 26.00 + portable/system`

### 11.3 发布说明

- [x] 记录安装步骤
- [x] 记录卸载步骤
- [x] 记录已知限制
- [x] 记录问题排查方法
  已新增 [Xunbak-7z-Plugin-Release-Guide.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Xunbak-7z-Plugin-Release-Guide.md)

---

## Backlog：artifact capability schema 与 helper 收敛

> 审核结论：`create / convert` 当前存在多处 format capability、7z method、split size、输出枚举与路径比较的重复实现，已经具备明显漂移风险。

### H.1 capability table 收敛

- [x] **测试**：`create / convert` 对同一 format 的 method 支持矩阵一致
- [x] **测试**：`create / convert` 的参数错误与 fix hint 保持一致
- [x] 将 format capability / method whitelist 收敛为单一能力表，避免多处硬编码
  已外提到 `artifact/capabilities.rs`；ZIP/7z/xunbak 的 method/flag 校验已按 `operation + format` 单点定义，修复了 `xunbak create --method/--level` 与 `zip convert --threads` 的静默忽略问题

### H.2 shared helper 收敛

- [x] **测试**：split size 解析在 `create / convert / xunbak` 路径上一致
- [x] **测试**：输出路径计数 / numbered volume 枚举在 `create / convert` 路径上一致
- [x] **测试**：source == output 判定与 overwrite 交互逻辑在 `create / convert` 路径上一致
- [x] 下沉 `split size`、`collect numbered outputs`、`paths_equal`、overwrite 解析等重复 helper
  当前已下沉到 `artifact/common.rs`，包括 `split size`、`collect_file_or_numbered_outputs()`、`paths_equal()`、`resolve_effective_overwrite()`
  本轮继续补齐了 `artifact` 路径判定与输出统计 helper：`is_zip/7z/xunbak_artifact_path()`、`collect_artifact_output_paths()`、`compute_artifact_output_bytes()`、`throughput_bytes_per_sec()`、`maybe_fail_after_write_for_tests()`；同时修复了 `backup convert -> split xunbak` JSON 摘要里 `bytes_out` 只看单一路径、未汇总真实 outputs 的漂移问题。

### H.3 诊断文案校准

- [x] **测试**：artifact 错误提示与当前已实现格式能力一致
- [x] 清理过时帮助文案，例如仍暗示“7z artifact support 未完成”的错误提示

### H.4 create / convert 编排层收敛

> 自审结论：目前高价值重复已经从“能力表 / helper 漂移”收敛到“编排流程重复”。剩余重复主要集中在 `create.rs / convert.rs` 的 plan 生命周期、progress 发射、JSON 摘要拼装与 sidecar 写入时序。

- [x] **测试**：`create / convert` 的 plan `prepare -> write -> fail-after-write hook -> finalize/cleanup` 事务语义由统一 helper 覆盖
  已在 `artifact/output_plan.rs` 增加 `commit_output_plan()` 与对应单测，并接入 `create.rs / convert.rs / artifact::xunbak.rs`
- [x] **测试**：`create / convert` 的 progress 发射与摘要字段在同类输出上共享同一统计入口
  已在 `artifact/progress.rs` 增加 `emit_read/compress/write/verify_*_progress()`；`create.rs / convert.rs` 的 progress 事件模板不再重复拼装。`create.rs` 已补 `build_create_execution_summary()`，`convert.rs` 已补 `build_convert_selection/execution/failure_summary()`，收敛 preview / dry-run / success / failure 的 JSON 字段组装。
- [x] 收敛 `Dir/Zip/7z/Xunbak` 写出编排，避免 `create.rs / convert.rs` 再次复制 cleanup / finalize / summary 逻辑
  当前已把 `prepare -> write -> fail-after-write -> finalize/cleanup` 收口到 `commit_output_plan()`；格式差异仍保留在各自的 `temp_path/temp_base_path` 与 writer 调用层，避免过度抽象
- [x] 评估 `restore` 的 `preview / summary / 分发辅助层` 是否值得抽象
  已在 `restore.rs` 引入 `RestoreArtifactKind`、`RestoreMode`、`execute_restore_request()`、`build_restore_execution_summary()`、`build_restore_preview_items_for_kind()`，收敛了 `all / file / glob` 与 `dir / zip / 7z / xunbak` 的多处分发判断。
  `.xunbak` 的 `dry_run` 现已补齐到 `ContainerReader::dry_run_restore_all/file/glob()`；CLI 侧 `backup restore *.xunbak --dry-run` 不再假执行，能够返回真实的 planned restore 数量且不落地目标文件。
  `restore preview` 现已补 `RestorePreviewSummary`，把 `overwrite/new` 计数、最多展示条数与隐藏条数收成结构化摘要；同时将 preview 的目标路径映射、差异收集与结果排序下沉成小 helper，避免 `dir / zip / artifact` 三条 preview 链继续复制模板代码。
  `restore_core` 与 `restore.rs` 之间的 dry-run 输出与统计辅助层也已对齐：`commands::restore_core::emit_restore_dry_run()` 统一了 `dir / zip / 7z / xunbak` 的 dry-run 输出风格，`restore.rs` 增加 `RestoreStats` 收敛 `restored/failed/status` 派生逻辑，减少 tuple 解包与 `Ok((1,0))` 散落分支。
  `backup/common/cli.rs` 现已作为 app 层共享 helper：收敛了 summary 路径字符串转换、相对输入路径解析、按备份名查找 artifact、以及 restore 相关错误文案模板，减少 `create / convert / restore / xunbak / restore_core / sevenz / xunbak::reader` 之间的重复字符串和路径处理逻辑。

---

## Backlog：`xunbak` codec 扩展（`LZ4` 优先）

> 定位：这是 `.xunbak` 容器自身的 blob codec 扩展，不是把 `.xunbak` 改成 `zip / 7z` 容器。
> 当前状态：`.xunbak` 已实现 `none / zstd / lz4 / deflate / bzip2 / ppmd / lzma2`，并已打通 `create / convert / restore / verify / plugin` 主链；`auto` 当前为“文本优先 `PPMD`、其他优先 `ZSTD`、收益不足回退 `NONE`”。
> 设计说明：详见 [Xunbak-Codec-Expansion-Plan.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Xunbak-Codec-Expansion-Plan.md)
> 任务清单：详见 [Xunbak-Codec-Expansion-Tasks.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Xunbak-Codec-Expansion-Tasks.md)

### B.0 抽象边界

- [x] 明确只提取“codec 级通用函数”，不提取 `zip / 7z / xunbak` 的容器级通用 writer
- [x] 明确 `zip` / `7z` / `.xunbak` 共享层只包含：
  - 字节流压缩 / 解压
  - 流式 `copy decoded -> writer`
  - `should_skip_compress`
  - 压缩收益阈值判断
- [x] 明确不做以下抽象：
  - `ZipCompressionMethod` / `SevenZMethod` / `.xunbak Codec` 的强行合并
  - `zip` / `7z` 头部、目录结构、method id 的统一封装
  - solid / archive method chain 的跨格式复用
- [x] 先在 `.xunbak` 落地统一 codec backend，再评估哪些纯算法函数值得下沉到共享模块
- [x] 若需要共享模块，优先设计为 `compression/` 层，而不是继续堆在 `backup/artifact/` 或 `.xunbak` 容器层

### B.1 目标边界

- [x] 明确 `.xunbak` codec 扩展只引入“算法能力”，不引入 `zip / 7z` 的容器语义
- [x] 明确保持“每个 blob 独立压缩、独立校验、独立恢复”，不做 solid 模式
- [x] 文档已收敛当前产品优先级：默认 `zstd`、文本型 `ppmd`、归档型 `lzma2`、极速模式 `lz4`，`deflate / bzip2` 仅作矩阵补齐

### B.2 `LZ4` 第一阶段

- [x] **测试**：`Codec::LZ4` 的 blob 写入 / 读取 roundtrip 成功
- [x] **测试**：`copy_blob_record_content_to_writer()` 支持流式解压 `LZ4`
- [x] **测试**：`backup create --format xunbak --compression lz4` 成功
- [x] **测试**：`backup restore archive.xunbak` 可恢复 `LZ4` blob
- [x] **测试**：`backup convert xunbak -> dir/zip/7z` 可读取 `LZ4` blob
- [x] **bench**：记录 `LZ4` 与 `zstd(1)` 的 create / restore 吞吐对比
- [x] 在 `xunbak` codec 层实现 `LZ4` 压缩、解压与流式复制路径

### B.3 版本与兼容

- [x] **测试**：旧 reader 遇到新 codec 时返回明确错误，不 panic
- [x] **测试**：引入新 codec 后 `min_reader_version` 策略清晰且可验证
- [x] 明确新增 codec 的版本升级策略与兼容提示

### B.4 第二阶段候选

- [x] `lzma2` 已作为“高压缩率、低速度”的归档 codec 落地
- [x] `ppmd` 已固定为文本型高压缩比选项，而不是默认值
- [x] `bzip2 / deflate` 已评估为“可支持但不主推”的矩阵补齐 codec

---

## Phase 12：综合验收

### 12.1 7-Zip 插件

- [x] **半自动验收**：单文件 `.xunbak` 可直接在 7-Zip 中打开
- [x] **半自动验收**：分卷 `.xunbak.001` 可直接在 7-Zip 中打开
- [x] **半自动验收**：单文件提取内容正确
- [x] **半自动验收**：全量提取内容正确
  依据：`portable / system` 插件脚本在 stock `7-Zip 24.09 / 26.00` 下通过，详见 [Phase12-Semi-Acceptance-Report.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Phase12-Semi-Acceptance-Report.md)

### 12.2 传统 backup

- [x] **半自动验收**：小改动场景下 cache 命中与 hardlink 复用明显生效
- [x] **半自动验收**：rename-only 场景行为符合预期
- [x] **半自动验收**：`verify` 分级模式行为符合预期
  依据：`module_backup_restore` / `test_xunbak` 关键用例与基线日志已覆盖，详见 [Phase12-Semi-Acceptance-Report.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Phase12-Semi-Acceptance-Report.md)

### 12.3 回归

- [x] `cargo check --tests --benches`
- [x] `cargo test --lib`
- [x] `cargo test --test module_backup_restore`
- [x] 插件便携式联调脚本
- [x] 插件系统级联调脚本

---

## 依赖关系

```text
Phase 0（基线冻结）
  ├─→ Phase 1（安装器/doctor）
  ├─→ Phase 3（插件打开路径优化）
  └─→ Phase 5（file_id）

Phase 1 ─→ Phase 2（文件关联）
Phase 3 ─→ Phase 4（插件属性增强） ─→ Phase 11（发布化）
Phase 5 ─→ Phase 7（报告增强）
Phase 6（恢复优化） ─┐
Phase 8（verify 分级）├─→ Phase 12（综合验收）
Phase 9（导出对齐） ─┘
Phase 10（ZIP/7z 方法扩展） ─┘
Backlog（xunbak codec 扩展） ─┘
```

---

## 建议执行顺序

### 第 1 批

1. Phase 1：安装器 / uninstall / doctor
2. Phase 2：文件关联
3. Phase 12.1：插件手工验收

### 第 2 批

1. Phase 3：插件大文件打开路径优化
2. Phase 4：插件属性增强
3. Phase 10：手写 ZIP / 7z 方法集扩展
4. Phase 11：Release 发布化

### 第 3 批

1. Phase 5：传统 backup 接入真实 `file_id`
2. Phase 6：恢复链路顺序读取与 reader 复用
3. Phase 7：报告增强
4. Phase 8：verify 分级模式
5. Phase 9：导出链路元数据对齐

### 后续 backlog

1. `xunbak` codec 扩展：先做 `LZ4`
2. 再评估 `lzma / ppmd`
3. 最后再决定是否需要 `bzip2 / deflate`

---

## 验证命令建议

```bash
# 基础编译
cargo check --tests --benches

# 库测试
cargo test --lib

# backup / restore 黑盒
cargo test --test module_backup_restore

# 插件构建
./scripts/build_xunbak_7z_plugin.ps1 -Config Debug

# 插件联调
./scripts/test_xunbak_7z_plugin_portable.ps1 -Config Debug
./scripts/test_xunbak_7z_plugin_system.ps1 -Config Debug
```
