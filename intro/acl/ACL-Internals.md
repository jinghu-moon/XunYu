# ACL 内部结构导读

`acl` 模块是这个项目里另一个明显的“内核型子系统”。它处理的不是普通业务数据，而是 Windows 文件系统 ACL：读取、比较、写入、修复、导出、恢复、审计、孤儿 SID 治理。

这篇文档重点回答：

1. ACL 内核是如何拆模块的？
2. 读取、写入、治理三条链路分别落在哪些文件？
3. CLI 命令又是怎么接到这些内核能力上的？

## 1. 总体结构

`src/acl/mod.rs` 把 ACL 内核拆成下面几块：

- `audit`
- `diff`
- `effective`
- `error`
- `export`
- `orphan`
- `parse`
- `privilege`
- `reader`
- `repair`
- `types`
- `writer`

这说明 ACL 子系统的设计并不是“读写混在一起”，而是很明确地分成：

- **模型层**：`types`
- **读分析层**：`reader`、`parse`、`diff`、`effective`
- **写入层**：`writer`
- **治理层**：`repair`、`orphan`、`audit`、`export`
- **系统能力层**：`privilege`、`error`

## 2. 类型层：`types/mod.rs` 是 ACL 的数据字典

`src/acl/types/mod.rs` 很大，但职责很纯：把 ACL 的核心表达都放在一处。

## 2.1 权限与保护常量

这一层首先给出两类常量：

- `RIGHTS_TABLE`：把原始 rights mask 映射为短名和说明
- `PROTECTED_NAMES`：标记不应该被批量操作触碰的受保护路径名

这两个常量让 CLI 输出、批量治理和风险控制都能共享同一套规则。

## 2.2 基础枚举与标志位

这里定义了：

- `AceType`：`Allow` / `Deny`
- `TriState`：允许 / 拒绝 / 无规则
- `InheritanceFlags`
- `PropagationFlags`

其中 `InheritanceFlags` 和 `PropagationFlags` 用薄包装结构体承接原始位掩码，这让代码在保持底层兼容的同时更可读。

## 2.3 主要实体模型

最关键的几个结构体是：

- `AceEntry`：单条 ACE 规则
- `AclSnapshot`：某个路径的 ACL 快照，包含 owner、是否受保护、DACL entries
- `DiffResult`：两份 ACL 快照的差异结果
- `RepairStats`：强制修复过程的统计结果
- `EffectiveAccess`：某组 SID 在该 ACL 下的有效权限结果

也就是说，ACL 子系统的所有动作，最后基本都围绕这几种模型流转。

## 3. 读链路：先把真实 ACL 快照拿出来

## 3.1 `reader.rs`

`reader.rs` 是最底层的读取入口，核心能力包括：

- `get_acl(path)`：读取目标路径完整 ACL，返回 `AclSnapshot`
- `resolve_sid(sid)`：把原始 SID 解析成人类可读主体
- `sid_to_string(sid)`：SID 字符串化
- `list_children(path, recursive)`：枚举子路径，供递归扫描使用
- `is_protected_path(path)`：判断是否属于受保护路径

这意味着：**ACL 的读取不是只拿 DACL 条目，而是把 owner、继承保护状态、条目集合一起做成快照。**

## 3.2 `parse.rs`

`parse.rs` 承担命令参数和文本输入到 ACL 语义之间的翻译，例如：

- 解析 rights 字符串
- 解析 `Allow` / `Deny`
- 解析继承与传播相关参数

这层的意义很重要：

- CLI 输入不会直接污染底层写入逻辑
- 文本协议和内核模型之间有清晰转换层

## 3.3 `diff.rs`

`diff.rs` 提供 `diff_acl(a, b) -> DiffResult`。

它把两份 ACL 快照之间的差异抽象出来，使得：

- CLI 可以做人类可读 diff
- 导出层可以输出 CSV
- 批处理和治理逻辑也能复用同一套差异表达

## 3.4 `effective.rs`

这里负责有效权限计算，核心能力包括：

- `get_current_user_sids()`
- `resolve_user_sid(user)`
- `compute_effective_access(snapshot, user_sids)`

它并不修改 ACL，而是回答另一个更接近诊断的问题：**给定这份 ACL，某个用户最终到底拥有什么权限？**

## 4. 写链路：`writer` 是统一写门面

如果说 `reader.rs` 是读取入口，那 `writer/` 就是所有写操作的统一出口。

## 4.1 `writer/mod.rs`

这一层对外暴露的能力非常清晰：

- `lookup_account_sid`
- `add_rule`
- `remove_rules`
- `purge_principal`
- `set_owner`
- `set_access_rule_protection`
- `copy_acl`

CLI 不需要知道底层 Win32 API 细节，只需要通过这层门面调用。

## 4.2 `writer/apply/*`：按写动作继续拆细

`writer/apply/mod.rs` 又把真正写入拆成更小的实现模块：

- `sid.rs`：账户主体到 SID 的解析
- `owner.rs`：owner 变更
- `dacl.rs`：ACE 级增删、清理、principal 清除
- `copy.rs`：ACL 复制
- `common.rs`：共享底层辅助逻辑，例如宽字符路径转换

这种拆法非常值得注意：ACL 写入是高复杂度区域，但它没有变成一个超大文件，而是围绕操作类型继续拆分。

## 4.3 `inheritance.rs`

`set_access_rule_protection()` 最终落在 `inheritance.rs`。这层专门处理“是否禁用从父级继承 ACL”这件事。

也就是说，继承控制被视为和 DACL 条目编辑不同的一类操作，而不是混在 `add_rule` / `remove_rule` 里。

## 4.4 `error_map.rs`

ACL 相关错误很多来自 Win32 API。`error_map.rs` 的职责是把这些低层错误转成更适合上层理解和处理的错误语义。

## 5. 治理链路：不只会改 ACL，还会治理 ACL

ACL 子系统复杂的地方不只是“能加一条规则”，而是它还内建了成体系的治理能力。

## 5.1 `orphan.rs`

这里处理孤儿 SID：

- `scan_orphans(...)`：扫描目录树中的 orphan entries
- `purge_orphan_sids(...)`：清理孤儿 SID

这说明 ACL 子系统不仅关心“当前规则是什么”，也关心“这些规则里是否混入了已经无效的主体”。

## 5.2 `repair.rs`

这里负责强制修复能力，核心入口是：

- `force_repair(root, config, quiet) -> RepairStats`

此外它还包含类似：

- `set_owner_to_admins`
- `set_full_control_reset_inherit`

这意味着 repair 并不是简单地“重新写一遍 ACL”，而是有一套面向恢复可控状态的组合动作。

## 5.3 `privilege.rs`

ACL 修复很多时候需要额外特权。这个模块负责启用相关 Windows privilege，例如：

- `SeRestorePrivilege`
- `SeBackupPrivilege`
- `SeTakeOwnershipPrivilege`

它提供了：

- `enable_privilege(name)`
- `enable_repair_privileges()`

所以 repair 能力并不是凭空成功，而是依赖一层明确的系统特权适配。

## 5.4 `audit.rs`

这个模块负责 ACL 操作的审计记录。它让批量治理、修复和人工修改都能留下历史轨迹，方便回溯。

## 5.5 `export/`

`src/acl/export/mod.rs` 是导出与恢复的统一门面，提供：

- `backup_acl`
- `restore_acl`
- `export_diff_csv`
- `export_orphans_csv`
- `export_repair_errors_csv`
- `export_acl_csv`
- `backup_filename`
- `error_csv_filename`

内部再拆成：

- `schema.rs`
- `format.rs`
- `writer.rs`

这里的设计很清楚：

- backup / restore 解决“状态保全与回滚”
- CSV 导出解决“治理结果可审阅、可外传、可归档”

## 6. 一条完整 ACL 路径怎么走

把 ACL 子系统连起来看，大概有三条主线。

## 6.1 查看 / 分析链路

- `reader::get_acl()` 读取 `AclSnapshot`
- `diff::diff_acl()` 比较两份快照
- `effective::compute_effective_access()` 计算有效权限
- 最后由 CLI 或导出层展示结果

## 6.2 编辑链路

- CLI / 批处理把输入交给 `parse.rs`
- `writer/mod.rs` 作为统一写门面
- 再分派到 `writer/apply/*` 或 `inheritance.rs`
- 底层通过 Win32 API 完成 owner / DACL / copy / inheritance 变更

## 6.3 治理链路

- `orphan.rs` 找无效主体
- `repair.rs` 做强制修复
- `privilege.rs` 负责必要特权
- `audit.rs` 记录过程
- `export/` 导出结果与备份

因此 ACL 子系统不是简单 CRUD，而是“读取、编辑、治理、恢复”四类能力同时存在。

## 7. CLI 适配层：`commands/acl_cmd/`

用户真正敲的是 `xun acl ...`，但命令层本身并不应承载 ACL 内核逻辑。

## 7.1 `mod.rs`：总分发入口

`src/commands/acl_cmd/mod.rs` 的 `cmd_acl(args)` 按子命令分发到不同模块：

- `View`
- `Add`
- `Remove`
- `Purge`
- `Diff`
- `Batch`
- `Effective`
- `Copy`
- `Backup`
- `Restore`
- `Inherit`
- `Owner`
- `Orphans`
- `Repair`
- `Audit`
- `Config`

这说明命令层是很薄的适配壳，主要负责分发、参数和输出。

## 7.2 子模块分工

命令层继续按职责拆开：

- `view.rs`：查看、diff、effective 这类只读分析操作
- `edit.rs`：add / remove / purge / copy / inherit / owner 这类写操作
- `repair.rs`：orphan / repair 这类治理动作
- `audit.rs`：审计读取与展示
- `batch.rs`：备份、恢复、批量流程
- `config.rs`：ACL 相关配置管理
- `common.rs`：命令层共享辅助逻辑

这使得 CLI 层和内核层都保持单一职责。

## 8. 为什么 ACL 值得单独看内部结构

ACL 相关代码容易失控，原因在于它同时具备：

- Windows API 复杂度
- 权限与继承语义复杂度
- 批量目录树治理复杂度
- 审计、导出、恢复等运维复杂度

这个项目的 ACL 结构比较好的地方在于，它没有把所有逻辑塞进一个“acl.rs”大文件，而是明确分成：

- 快照读取
- 参数解析
- 差异计算
- 有效权限计算
- 写入门面与写入细分模块
- 孤儿治理与强制修复
- 审计与导出恢复
- CLI 适配

因此你在阅读时，不需要一开始就理解全部 Win32 细节，而可以先按链路拆开理解。

## 9. 推荐阅读顺序

如果你已经看过 `./ACL-Modules.md`，建议继续这样读：

1. `src/acl/mod.rs`：先建立模块地图
2. `src/acl/types/mod.rs`：先把 ACL 的核心模型认清
3. `src/acl/reader.rs`：看快照是怎么读出来的
4. `src/acl/diff.rs`、`effective.rs`：看分析能力
5. `src/acl/writer/mod.rs` 和 `src/acl/writer/apply/`：再看写链路
6. `src/acl/orphan.rs`、`repair.rs`、`privilege.rs`：最后看治理和恢复能力
7. `src/commands/acl_cmd/mod.rs`：收尾看 CLI 如何把这些能力接起来

这样读会比直接扎进 Win32 写入逻辑更稳，也更容易建立对整个 ACL 子系统的完整模型。


