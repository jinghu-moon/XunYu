# Backup/Restore 当前状态评估

生成时间：2026-03-22

## 1. 结论摘要

截至 2026-03-22，`backup` / `restore` 已经完成从“历史 bak/restore 混用”到“正式边界收敛”的阶段。

当前总体评价：

1. **功能**：`backup` 已接近第一阶段完成态，`restore` 主链路已稳定可用。
2. **性能**：主要热点已经收敛到 copy 路径，扫描、baseline、diff 不再是核心瓶颈。
3. **交互**：对开发者工具场景已经够用，但还没有达到产品级 CLI 体验。
4. **工程状态**：模块边界明显优于重构前，测试覆盖也较完整，适合继续做定向增强。

一句话判断：

> 当前 `backup` / `restore` 处于“主能力可用、热点清晰、下一轮应该做收口与增强”的状态，而不是需要再次大拆的状态。

---

## 2. 当前代码边界

当前核心文件：

1. `backup` 命令入口：`src/commands/backup.rs`
2. `backup` 热路径：
   - `src/commands/backup/scan.rs`
   - `src/commands/backup/baseline.rs`
   - `src/commands/backup/diff.rs`
   - `src/commands/backup/retention.rs`
   - `src/commands/backup/list.rs`
   - `src/commands/backup/find.rs`
   - `src/commands/backup/verify.rs`
3. `restore` 命令入口：`src/commands/restore.rs`
4. `restore` 执行内核：`src/commands/restore_core.rs`
5. 复制后端：`src/windows/file_copy.rs`

命名状态：

1. 正式名：`backup` / `restore`
2. 别名：`bak` / `rst`

判断：

1. 命名层已经清晰。
2. 模块职责已经比早期状态整齐很多。
3. 继续优化应以小步增强为主，不建议再次做大范围架构重写。

---

## 3. 功能评价

### 3.1 backup

当前已经具备：

1. 创建备份
2. 目录型备份与 zip 型备份
3. `list`
4. `find`
5. `verify`
6. `incremental`
7. `skipIfUnchanged`
8. retention
9. `useGitignore`
10. 旧配置名 `.svconfig.json` -> `.xun-bak.json` 自动迁移

评价：

1. 作为开发者工具，`backup` 的主能力已经比较完整。
2. 配置项与 CLI 项之间已经形成基本闭环。
3. `skipIfUnchanged` 已经进入“值得默认依赖”的能力层级。

短板：

1. `find` 已支持 `since/until` 时间过滤，但查询能力还可以继续补更细的筛选维度。
2. `verify` 的用户感知仍然偏弱，尤其是 manifest 生成与 feature 的关系不够直观。
3. `backup` 查询能力已经切到正式子命令结构，但时间过滤等高级查询参数仍未完整暴露。

### 3.2 restore

当前已经具备：

1. 从目录备份恢复
2. 从 zip 备份恢复
3. 全量恢复
4. `--file`
5. `--glob`
6. `--to`
7. `--snapshot`
8. `--dry-run`
9. 非安全路径拦截
10. 跳过备份内部元文件

评价：

1. `restore` 主链路已经可用且稳定。
2. `restore_core.rs` 的收口是有效的，消除了早期边界混乱。
3. 目录恢复与 zip 恢复已经能共用一部分核心约束。

短板：

1. 预览逻辑仍是轻量 heuristic，不是真正的执行前差异计算。
2. `restore` 目前没有结构化输出模式，脚本消费体验一般。
3. 对用户来说，“会覆盖哪些文件”的反馈还不够强。

---

## 4. 性能评价

## 4.1 已经做对的事

当前性能优化方向是正确的，主要体现在：

1. `scan`
2. `baseline`
3. `diff`
4. `copy`

这四段已经被阶段计时覆盖：

1. `backup` 支持：
   - `XUN_CMD_TIMING=1`
   - `XUN_BACKUP_TIMING=1`
   - `XUN_BAK_TIMING=1`
2. `restore` 支持：
   - `XUN_CMD_TIMING=1`
   - `XUN_RESTORE_TIMING=1`

当前结论已经比较明确：

1. `backup` 主要瓶颈在 `copy`
2. `restore` 主要瓶颈也在 `copy`
3. `scan`、`baseline`、`diff` 已不再是主要矛盾

### 4.2 已落地的关键优化

#### backup

1. unchanged 文件 hardlink 复用已进入主链路
2. `skipIfUnchanged` 已进入主链路
3. `CopyFile2` backend 已可切换，但默认仍保持 `Std`

判断：

1. unchanged hardlink 是当前最有价值的优化，收益真实，风险可控。
2. 默认不切 `CopyFile2` 是正确决策，因为当前基准没有证明它在现有样本下明显更优。

#### restore

1. bulk 恢复路径已经接入统一 copy backend
2. zip 恢复也有独立计时和安全校验

判断：

1. `restore` 性能路径比早期清晰很多。
2. 但仍有一处目录恢复分支保留 `fs::copy`，复制策略还未做到完全统一。

### 4.3 性能上的未完成项

当前最明显的未完成项：

1. `restore_from_dir()` 的某条全量路径仍使用 `fs::copy`，而不是统一走 `copy_file()` backend。
2. 查询路径 `list / find / verify` 还没有针对“大量历史备份”场景做专门优化。
3. 性能测试虽已存在，但尚未成为默认回归门槛。

总结：

> 性能层目前不是“没有优化”，而是“主链路热点已解决第一轮，下一轮该从 copy 深挖转向查询路径与一致性收口”。

---

## 5. 交互评价

### 5.1 优点

1. 命令命名已统一，学习成本下降。
2. `backup` 输出对开发者足够直观。
3. `restore --snapshot`、`--dry-run`、`--to` 都是很实用的安全/工作流选项。
4. 有 timing 输出，便于性能调试。

### 5.2 不足

1. `backup` 的基础查询子命令已经明确，但高级查询能力与结构化输出仍可继续增强。
2. `restore` 预览偏保守实现，不够精准。
3. 结构化输出不足，不利于后续 Dashboard 或脚本侧消费。
4. 用户无法快速区分本次操作是：
   - created
   - skipped
   - verified
   - restored
   - partially_failed

判断：

1. 当前交互层更像“工程师自用 CLI”。
2. 如果要继续向 Dashboard 或自动化场景推进，必须补结构化输出与动作态回执。

---

## 6. 测试与质量评价

当前 `backup / restore` 的测试情况是正面的。

### 6.1 功能测试

`module_backup_restore` 当前共有 47 个测试，覆盖了：

1. 创建备份
2. 压缩/非压缩
3. 增量
4. retention
5. `skipIfUnchanged`
6. `list/find/verify`
7. 全量恢复
8. `--file`
9. `--glob`
10. `--snapshot`
11. `--dry-run`
12. zip 恢复安全边界

### 6.2 性能测试

已存在特殊测试入口：

1. `tests/special/performance.rs`

其中包含：

1. `backup full`
2. `backup incremental`
3. `restore dir`
4. `restore zip`

评价：

1. 功能测试已经达到“主链路受保护”的程度。
2. 性能测试已经建立，但更多是专项验证，不是持续门槛。

---

## 7. 当前主要风险

### 7.1 功能风险

1. `verify` 的 feature 语义与用户认知仍可能不一致。
2. `find` 的查询能力与 CLI 暴露能力不完全对齐。

### 7.2 性能风险

1. `restore` 的复制后端尚未完全统一。
2. 历史备份数量继续增长后，`list/find/verify` 可能成为新的热点。

### 7.3 交互风险

1. 预览和真实执行存在信息层级差。
2. 缺少统一 JSON 回执，会限制后续自动化接入。

---

## 8. 下一轮优先级建议

### P0：先做

1. 统一 `restore` 目录恢复路径的 copy backend，消除 `fs::copy` 残留分支。
2. 明确 `verify` 的产品语义：
   - 要么默认可用并保证 manifest 默认生成
   - 要么显式 gated，不再让默认构建呈现半可用状态
3. 给 `backup list/find/verify` 与 `restore` 补结构化输出模式。

### P1：随后做

1. 把 `backup` 的字符串式 `op_args` 收敛成正式子命令结构。
2. 提升 `restore` 预览准确度，避免“将覆盖但预览不明显”的情况。
3. 优化 `list/find/verify` 在大备份集下的读取和排序路径。

### P2：最后做

1. 继续尝试 `CopyFile2` 或其他 Win32 copy backend 的策略化切换
2. 评估是否需要项目级文件缓存
3. 再决定是否考虑更大粒度的仓库模型升级

---

## 9. 最终评价

当前的 `backup / restore` 不需要再次做大范围架构重写。

更准确的判断是：

1. **功能主链路已完成第一阶段建设**
2. **性能第一轮优化已做对**
3. **交互与产品语义仍有明显提升空间**
4. **下一轮应该做收口与增强，而不是再次重构骨架**

建议决策：

> 后续把 `backup / restore` 视为“重点增强模块”，而不是“待抢救模块”。
