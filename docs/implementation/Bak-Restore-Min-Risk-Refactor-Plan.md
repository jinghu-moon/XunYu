# Bak/Restore 最小风险重构方案

生成时间：2026-03-21

> 状态说明（2026-03-21）：阶段 0 到阶段 4 已完成实施。`src/commands/bak/restore.rs` 已删除，恢复链路已迁到 `src/commands/restore.rs` 与 `src/commands/restore_core.rs`。下文主体保留为实施前方案与分阶段设计，用于追溯，不代表当前代码现状。

## 1. 目标与边界

本方案用于重构当前 `bak/restore` 相关实现，目标是：

1. 删除 `src/commands/bak/restore.rs`，让恢复逻辑回到 `restore` 领域。
2. 消除 `restore.rs` 与 `bak/restore.rs` 之间的平行实现。
3. 在结构迁移前先修复已确认的行为问题，避免“重构后继续带 bug”。
4. 保持 CLI 外部契约稳定，不新增用户可见命令，不改命令字，不改参数语义。

本轮强约束：

1. 先修行为问题，再做模块搬迁。
2. 不把 `bak::diff`、`bak::scan` 强行抽成通用文件系统模块。
3. 不在本轮引入“大而全”的 `fs_ops.rs`。
4. 每个阶段结束后都必须保持可编译、可测试、可单独回退。

---

## 2. 当前代码现状

当前恢复逻辑散落在两处：

1. `src/commands/restore.rs`
2. `src/commands/bak/restore.rs`

其中已经确认的重复点包括：

1. `collect_files_recursive`
2. `is_safe_zip_entry`
3. 目录恢复复制循环
4. zip 恢复提取循环
5. 恢复统计与错误处理的平行实现

当前职责边界也不自然：

1. `bak/restore.rs` 并不是 `bak` 子命令入口的一部分，它实际只是在被 `restore.rs` 当工具模块调用。
2. `bak.rs` 已不再暴露 `bak restore` 行为，但仍保留了 `pub(crate) mod restore;`。
3. `cli/bak.rs` 的注释仍写着 ``restore <name>``，与实际实现不一致。

---

## 3. 已确认的真实问题

以下问题优先级高于结构重构，因为它们已经影响行为正确性：

| 问题 | 当前位置 | 影响 |
| --- | --- | --- |
| zip 恢复预览失真 | `bak/restore.rs` 的 `show_restore_preview()` | 预览对 zip 备份可能错误显示“no files will be changed” |
| zip glob 恢复缺少安全校验 | `restore.rs` 的 `restore_glob_from_zip()` | 不安全 entry 可能绕过统一校验逻辑 |
| zip 单文件恢复未命中时误报成功 | `bak/restore.rs` 的 `restore_from_zip()` + `restore.rs` 的 `restore_single_file()` | `--file` 指定不存在文件时仍可能返回成功 |
| 目录恢复会带回 `.bak-meta.json` | 目录备份恢复全量复制 | 备份元数据泄漏回工作目录 |
| `bak` 注释与实际行为不一致 | `cli/bak.rs` | 文档和代码认知错位 |

结论：

1. 这轮不能只做“模块换位置”。
2. 必须先建立行为护栏，再做删除和抽取。

---

## 4. 本轮非目标

以下内容不在本轮内，避免范围失控：

1. 不重写 `bak::diff` 的快照比较模型。
2. 不重写 `bak::scan` 的 include/exclude/.gitignore 逻辑。
3. 不统一项目内所有 glob 实现。
4. 不把 `restore` 目录结构整体改造成 `src/commands/restore/mod.rs`。
5. 不修改 `bak` 的外部命令行为和存储格式。

说明：

1. `bak::diff` 是备份版本差异语义，不是通用 IO。
2. `bak::scan` 承载的是备份扫描规则，不是纯目录遍历。
3. 项目内已有多套 glob 逻辑，贸然“一次收敛”风险高，容易引入跨模块回归。

---

## 5. 目标结构

本轮建议落到如下结构：

```text
src/commands/
├── bak.rs
├── restore.rs
├── restore_core.rs        # 新增：restore 私有内核
└── bak/
    ├── baseline.rs
    ├── config.rs
    ├── diff.rs
    ├── find.rs
    ├── meta.rs
    ├── scan.rs
    ├── zip.rs
    └── ...                # 保持不动
```

职责边界：

1. `restore.rs`
   - 负责 CLI 编排。
   - 负责备份源解析。
   - 负责恢复前预览。
   - 负责 `--snapshot` 触发。

2. `restore_core.rs`
   - 负责目录/zip 恢复执行。
   - 负责相对路径和 zip entry 安全校验。
   - 负责恢复统计。
   - 只服务于 `restore`，不作为“全局通用 IO 模块”。

3. `bak/*`
   - 保留备份专用逻辑。
   - `diff.rs`、`scan.rs`、`meta.rs` 不迁出。

4. `bak/restore.rs`
   - 最终删除。

---

## 6. 为什么不建议直接做大 `fs_ops.rs`

`fs_ops.rs` 看起来能一次吃掉重复代码，但它会把三类不同语义混在一起：

1. 纯文件复制原语。
2. 恢复场景特有的安全语义。
3. 选择器语义，例如 `all`、`--file`、`--glob`、未命中是否报错。

这会带来三个问题：

1. 抽象过大。
   - `copy_from_zip(filter)` 这种接口能表达“匹配”，但很难自然表达“单文件未命中要报错、glob 未命中可以正常返回、跳过不安全 entry 需要单独计数”。

2. 边界错误。
   - 把 `glob_match`、路径安全、目录遍历、复制循环全塞进 `fs_ops`，会形成新的“杂物间模块”。

3. 后续可维护性差。
   - 当前真实共享的是“恢复内核”，不是整个项目的“文件系统原语层”。

因此本轮采用更窄的命名和边界：

1. 只新增 `restore_core.rs`。
2. 只承载恢复执行所需的底层能力。
3. 不承诺为其他命令复用。

---

## 7. 最小风险实施原则

1. 行为优先。
   - 先保证 zip 预览、单文件未命中、安全校验、元数据泄漏等问题被修正。

2. 迁移最小化。
   - 第一轮不改 `restore` 命令的公开函数签名。

3. 抽象收窄。
   - 只抽 `restore` 私有内核，不做全局公共层。

4. 单向依赖。
   - `restore.rs` 依赖 `restore_core.rs`。
   - `bak` 不依赖 `restore`。

5. 小步交付。
   - 每一步都应是“独立正确”的中间态。

---

## 8. 分阶段执行方案

## 阶段 0：测试护栏

目的：

1. 在不迁移模块前，先锁定当前和目标行为。

新增或补强测试：

1. `restore --file` 从 zip 恢复时，指定不存在文件应返回非零退出。
2. `restore --glob` 从 zip 恢复时，遇到不安全 entry 应跳过并计入失败或跳过统计。
3. 目录恢复不应把 `.bak-meta.json` 恢复到目标目录。
4. zip 恢复预览应正确显示 overwrite/new，而不是默认空结果。
5. `bak` CLI 帮助或注释不再暗示 `bak restore`。

阶段门禁：

1. `cargo check`
2. `cargo test restore_cmd_`

说明：

1. 先加测试再改代码，能避免“重构后行为变化却无守护”。

---

## 阶段 1：先修行为问题，不搬模块

目的：

1. 在当前文件布局不变的前提下，先把已确认 bug 修掉。

建议修改：

1. 修复 zip 预览。
   - 不再用 `scan_files(backup_src, ...)` 扫 zip。
   - 预览改为基于“备份源快照清单 vs 当前工作区快照清单”比较。
   - 优先复用 `bak::baseline::read_baseline()` 读取目录和 zip 的统一清单。

2. 修复 zip glob 安全校验缺失。
   - `restore_glob_from_zip()` 在匹配成功后，写出前必须执行与全量恢复一致的 `is_safe_zip_entry()` 校验。

3. 修复 zip 单文件未命中误报成功。
   - `restore_from_zip()` 或新的 zip 执行路径必须返回“是否匹配到目标文件”。
   - `restore_single_file()` 在未命中时应返回错误，而不是固定 `Ok((1, 0))`。

4. 修复目录恢复的元数据泄漏。
   - 恢复时显式跳过 `.bak-meta.json`。
   - 如果未来还有 manifest 文件，也应一起走“恢复时排除备份内部元数据”的规则。

5. 修正文案。
   - `cli/bak.rs` 的注释删除 ``restore <name>``。

阶段结束标准：

1. 行为问题已修复。
2. 仍然保留现有模块结构。
3. 尚未删除 `bak/restore.rs`。

---

## 阶段 2：领域归位

目的：

1. 先把不该属于 `bak` 的业务逻辑搬回 `restore`，但暂时不抽底层复制内核。

迁移动作：

1. 将 `backup_source_path()` 从 `bak/restore.rs` 移到 `restore.rs`。
2. 将 `show_restore_preview()` 从 `bak/restore.rs` 移到 `restore.rs`。
3. 将 `restore.rs` 中对 `bak_restore::...` 的调用改为本文件私有函数调用。

此阶段不做的事：

1. 不动目录/zip 复制循环。
2. 不抽 `restore_core.rs`。
3. 不删除 `bak/restore.rs`。

原因：

1. 先完成“逻辑归属纠正”，再处理“底层重复收敛”，可以降低一次性迁移量。

阶段结束标准：

1. `bak/restore.rs` 中只剩真正还未迁出的底层实现。
2. `restore.rs` 不再依赖 `bak` 域中的恢复业务函数。

---

## 阶段 3：抽取 `restore_core.rs`

目的：

1. 把目录/zip 恢复执行收敛到一个 restore 私有内核中，消除平行实现。

建议接口：

```rust
pub(crate) enum RestoreSelector {
    All,
    File(std::path::PathBuf),
    Glob(String),
}

pub(crate) struct RestoreStats {
    pub(crate) restored: usize,
    pub(crate) failed: usize,
    pub(crate) matched: usize,
    pub(crate) skipped_unsafe: usize,
}

pub(crate) fn is_safe_rel_path(rel: &std::path::Path) -> Result<(), CliError>;

pub(crate) fn restore_from_dir(
    src_dir: &std::path::Path,
    dest_root: &std::path::Path,
    selector: &RestoreSelector,
    dry_run: bool,
) -> Result<RestoreStats, CliError>;

pub(crate) fn restore_from_zip(
    zip_path: &std::path::Path,
    dest_root: &std::path::Path,
    selector: &RestoreSelector,
    dry_run: bool,
) -> Result<RestoreStats, CliError>;
```

设计说明：

1. 使用 `RestoreSelector`，而不是 `Option<Fn>`。
   - 这样可以直接表达 `All`、`File`、`Glob` 三种业务语义。

2. 使用 `RestoreStats`，而不是裸 `(usize, usize)`。
   - 后续可以自然承载 `matched`、`skipped_unsafe` 等信息。
   - 能解决“单文件未命中但 restored/fail 仍难以表达”的问题。

3. `restore_core.rs` 仅服务于 `restore.rs`。
   - 不对外承诺通用性。

迁移方式：

1. 先把 `restore.rs` 目录恢复循环迁入 `restore_core.rs`。
2. 再把 `bak/restore.rs` 中的 zip/目录恢复实现迁入。
3. 迁移完成后，`restore.rs` 只做参数组装和结果输出。

阶段结束标准：

1. `restore.rs` 中不再保留目录/zip 复制循环。
2. 所有恢复执行入口都走 `restore_core.rs`。

---

## 阶段 4：删除 `bak/restore.rs`

目的：

1. 完成边界收口。

动作：

1. 删除 `src/commands/bak/restore.rs`。
2. 删除 `src/commands/bak.rs` 中的 `pub(crate) mod restore;`。
3. 清理已失效的 `use super::bak::restore as bak_restore;`。
4. 重新运行恢复相关测试。

阶段结束标准：

1. `restore` 不再依赖 `bak/restore.rs`。
2. `bak` 目录中不再出现恢复子模块。

---

## 9. 关于预览逻辑的建议

当前预览逻辑不适合继续依赖 `bak::scan + bak::diff`，原因如下：

1. `scan_files()` 面向工作目录扫描，不适合 zip 文件输入。
2. `compute_diff()` 的输入模型是 `HashMap<String, PathBuf>`，而预览只需要“清单比较”，不需要真实源路径。

因此建议本轮把预览逻辑改为 restore 自有实现：

1. 使用 `bak::baseline::read_baseline(root)` 读取当前工作区清单。
2. 使用同一个 `read_baseline(backup_src)` 读取目录备份或 zip 备份清单。
3. 在 `restore.rs` 内实现一个轻量的 `build_restore_preview()`，只输出：
   - overwrite 列表
   - new 列表
   - 合计数量

好处：

1. 立即修复 zip 预览。
2. 消除 `restore` 对 `bak::scan` 和 `bak::diff` 的不自然依赖。
3. 不影响 `bak` 本身的 diff/scan 设计。

---

## 10. 测试与验收

建议门禁：

1. `cargo check`
2. `cargo test restore_cmd_`
3. `cargo test bak_`

建议重点覆盖：

1. 按名称恢复目录备份。
2. 按名称恢复 zip 备份。
3. `--file` 从目录恢复。
4. `--file` 从 zip 恢复。
5. `--file` 未命中时报错。
6. `--glob` 从目录恢复。
7. `--glob` 从 zip 恢复。
8. `--glob` 无匹配时正常退出。
9. `--snapshot` 与 `--dry-run` 组合。
10. 恢复后目标目录中不出现 `.bak-meta.json`。
11. zip 预览结果与实际恢复内容一致。

验收标准：

1. `restore` 外部 CLI 参数和用法保持不变。
2. 删除 `bak/restore.rs` 后，恢复路径只有一个实现来源。
3. zip 与目录备份的恢复行为在安全校验和统计语义上保持一致。
4. 不引入新的公共“杂物间模块”。

---

## 11. 风险与控制

主要风险：

1. 统计语义变化。
   - 从裸 `(restored, failed)` 升级到 `RestoreStats` 后，命令层输出可能变化。

2. preview 行为变化。
   - 旧逻辑虽然不正确，但已有用户心智；修正后输出数量可能变大。

3. 隐性依赖遗漏。
   - 删除 `bak/restore.rs` 时，可能还有未扫描到的调用点。

控制方式：

1. 先测试后迁移。
2. 每阶段结束都跑 `cargo check` 和恢复相关测试。
3. 删除文件前先用全文搜索确认调用点清零。

---

## 12. 推荐实施顺序

建议严格按以下顺序执行：

1. 阶段 0：先补测试。
2. 阶段 1：先修行为问题。
3. 阶段 2：再做业务归位。
4. 阶段 3：最后抽 `restore_core.rs`。
5. 阶段 4：确认无引用后删除 `bak/restore.rs`。

不建议的顺序：

1. 先建大 `fs_ops.rs` 再慢慢回填行为。
2. 先删 `bak/restore.rs` 再补测试。
3. 同一轮同时处理 glob 收敛、scan 收敛、diff 收敛。

---

## 13. 最终建议

这轮最稳妥的做法不是“做一个通用组件”，而是：

1. 先把恢复逻辑从 `bak` 子域中拿出来。
2. 先修 zip 预览、安全校验、单文件未命中、元数据泄漏这四个真实问题。
3. 然后只抽一个 restore 私有内核。
4. 把 glob 全局统一、扫描模型统一、差异模型统一都放到后续独立议题。

一句话总结：

1. 先纠正边界和行为，再消除重复；只做 restore 私有重构，不做全项目通用层设计。
