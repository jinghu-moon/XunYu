# ACL 模块导读

本文档专门解释 `xun acl` 这条命令树。它是一个明显偏 Windows 运维的子系统，覆盖 ACL 查看、编辑、diff、备份恢复、孤儿 SID 处理、有效权限分析与审计。

## 1. `acl` 在项目里的位置

从结构上看，`acl` 也分成三层：

```text
xun acl ...
  -> src/cli/acl.rs            # argh 子命令定义
  -> src/commands/acl_cmd/*    # CLI 执行层
  -> src/acl/*                 # ACL 领域与底层实现
```

和 `env` 不同，`acl` 当前没有像 `env_core` 那样额外抽一个单独的 manager 门面，但领域层依然非常清晰。

## 2. 顶层命令面

`src/cli/acl.rs` 当前定义了这些子命令：

- `view`
- `add`
- `remove`
- `purge`
- `diff`
- `batch`
- `effective`
- `copy`
- `backup`
- `restore`
- `inherit`
- `owner`
- `orphans`
- `repair`
- `audit`
- `config`

从命令面就能看出它覆盖了四类能力：

1. **查看与分析**：`view / diff / effective / audit`
2. **写入与编辑**：`add / remove / purge / copy / inherit / owner`
3. **可靠性与修复**：`orphans / repair / batch`
4. **可恢复性**：`backup / restore / config`

## 3. 执行层：`src/commands/acl_cmd/*`

CLI 执行层按职责拆成：

- `view.rs`
  - `cmd_view`
  - `cmd_diff`
  - `cmd_effective`
- `edit.rs`
  - `cmd_add`
  - `cmd_remove`
  - `cmd_purge`
  - `cmd_copy`
  - `cmd_inherit`
  - `cmd_owner`
- `batch.rs`
  - `cmd_batch`
  - `cmd_backup`
  - `cmd_restore`
- `repair.rs`
  - `cmd_orphans`
  - `cmd_repair`
- `audit.rs`
  - `cmd_audit`
- `config.rs`
  - `cmd_config`
- `mod.rs`
  - `cmd_acl`

这种拆法非常直观：

- `view` 一类负责“读和解释结果”
- `edit` 一类负责“写和修改权限”
- `repair` 一类负责“异常与恢复”
- `batch` 一类负责“批量处理与备份恢复”

## 4. 底层领域层：`src/acl/*`

ACL 领域层目前分成这些模块：

- `reader`：读取路径 ACL，构造 `AclSnapshot`
- `writer`：写入 / 修改 ACL
- `diff`：比较两个 ACL 快照
- `effective`：计算有效权限
- `orphan`：识别孤儿 SID
- `repair`：修复 ACL 问题
- `audit`：记录和查询 ACL 审计
- `export`：备份、恢复、CSV 导出
- `parse`：权限显示、字符串解析等辅助能力
- `privilege`：Windows 权限提升/特权辅助
- `types`：`AclSnapshot`、`AceEntry`、`DiffResult` 等核心类型
- `error`：ACL 相关错误模型

如果你把 `acl_cmd` 当成应用服务层，那么 `src/acl/*` 就是这个子系统真正的领域层。

## 5. 先理解核心数据结构

### 5.1 `AclSnapshot`

`AclSnapshot` 表示某个路径的 ACL 快照，至少包含：

- `path`
- `owner`
- `is_protected`
- `entries`

这说明项目并不是“拿到系统 ACL 就现场打印”，而是先建一个稳定的中间模型，再做 view / diff / export / repair。

### 5.2 `AceEntry`

`AceEntry` 表示单条 ACE，核心字段包括：

- `principal`
- `raw_sid`
- `rights_mask`
- `ace_type`
- `inheritance`
- `propagation`
- `is_inherited`
- `is_orphan`

这套结构支撑了很多功能：

- `view` 可以展示权限项
- `diff` 可以做集合比较
- `orphans` 可以识别孤儿 SID
- `repair` 可以针对异常项做修复

### 5.3 `DiffResult` / `TriState`

- `DiffResult` 支撑 ACL 对比输出
- `TriState` 支撑有效权限计算（允许 / 拒绝 / 无规则）

这说明 ACL 子系统不仅做“静态读取”，还做“语义分析”。

## 6. 读路径：查看、比较、有效权限

### 6.1 `view`

`view` 的核心是：

- 读取目标路径 ACL
- 组装成 `AclSnapshot`
- 以简表或详情形式输出
- 可选导出 CSV

它适合回答“当前路径到底有哪些权限项”。

### 6.2 `diff`

`diff` 会同时读取目标路径和参考路径的 ACL，然后给出：

- owner 是否不同
- inheritance 是否不同
- 仅存在于 A 的权限项
- 仅存在于 B 的权限项
- common 数量

它适合回答“两个目录的 ACL 配置差了什么”。

### 6.3 `effective`

`effective` 会基于 `AclSnapshot` 和一个用户 SID 集合计算有效权限，输出的不是原始 ACE，而是“读 / 写 / 执行 / 删除 / 改权限 / 取所有权”等结果。

它适合回答“某用户最终到底能做什么”。

## 7. 写路径：添加、删除、继承、所有者

### 7.1 `writer`

`src/acl/writer/mod.rs` 暴露了几个关键入口：

- `lookup_account_sid`
- `add_rule`
- `remove_rules`
- `purge_principal`
- `set_owner`
- `set_access_rule_protection`
- `copy_acl`

也就是说，ACL 写入链路被抽成了统一接口，而不是散落在每个命令里。

### 7.2 `writer/apply/*`

写入实现继续拆成：

- `sid.rs`：账户 / SID 解析
- `dacl.rs`：DACL 修改
- `owner.rs`：Owner 修改
- `copy.rs`：ACL 复制
- `common.rs`：公共工具

这说明 ACL 写路径已经进入“按写入动作分模块”的状态，而不是一个大文件全包。

### 7.3 对应到 CLI 命令

- `add` -> 添加 ACE
- `remove` -> 移除指定 ACE
- `purge` -> 清空某 principal 的显式规则
- `copy` -> 拷贝参考路径 ACL
- `inherit` -> 调整继承保护
- `owner` -> 调整所有者

这组命令的风险明显高于只读命令，因此阅读时要特别注意确认、审计和错误处理。

## 8. 修复与恢复路径

### 8.1 `orphans` / `repair`

- `orphans`：识别无法解析 principal 的 SID 项
- `repair`：尝试修复 ACL 问题

这两类命令说明项目考虑了 Windows ACL 长期运行中最真实的问题：账户删除、SID 残留、权限污染。

### 8.2 `backup` / `restore`

`src/acl/export/mod.rs` 提供：

- `backup_acl`
- `restore_acl`
- `export_diff_csv`
- `export_orphans_csv`
- `export_repair_errors_csv`
- `export_acl_csv`

这说明“导出”不是附带功能，而是 ACL 子系统的重要恢复与审计能力。

## 9. `batch` 与 `config`

### 9.1 `batch`

`batch` 的设计思路是：

- 输入路径集合
- 指定 action
- 对多个路径重复应用同一类 ACL 运维操作

这属于典型的运维自动化入口。

### 9.2 `config`

`config` 则是 ACL 子系统的运行时配置入口。它和全局 `config` 不同，是 ACL 域自己的配置面。

## 10. 审计路径

ACL 子系统不只是改权限，还会记审计。`cmd_diff` 这类操作执行后会把行为写入 audit，这说明：

- ACL 工具被当作“可追踪运维动作”设计
- 项目默认认为 ACL 修改是高风险行为，需要留痕

这也是 ACL 模块与普通文件工具最不一样的地方之一。

## 11. 推荐阅读顺序

### 11.1 想先看命令语义

1. `src/cli/acl.rs`
2. `src/commands/acl_cmd/mod.rs`
3. 对应的 `view.rs / edit.rs / repair.rs / batch.rs`

### 11.2 想先看底层模型

1. `src/acl/types/*`
2. `src/acl/reader.rs`
3. `src/acl/writer/*`
4. `src/acl/diff.rs`
5. `src/acl/effective.rs`

### 11.3 想先看可恢复性和治理能力

1. `src/acl/export/*`
2. `src/acl/orphan.rs`
3. `src/acl/repair.rs`
4. `src/acl/audit.rs`

## 12. 当前实现上的几个观察

- `acl` 是一个很典型的 Windows 专项能力域，和跨平台工具链模块差异明显。
- 它已经不是“调一两个 WinAPI”的薄封装，而是完整的读取、建模、分析、写入、审计、恢复系统。
- `writer` 和 `export` 的继续拆分，说明项目正在把高风险逻辑往更细粒度模块推进。
- `AclSnapshot` / `AceEntry` 这样的中间模型非常关键；如果后续继续扩展 ACL 功能，优先围绕这些抽象扩展，而不是直接在命令层拼实现。
- `acl` 和 `env` 一样，都体现出这个项目已经从“命令集合”发展到“子系统集合”。
