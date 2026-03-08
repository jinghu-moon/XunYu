# xun 文件解锁/保护/加密 — 分阶段执行任务清单

> 依据：[File-Unlock-Protection-Encryption-Plan.md](./File-Unlock-Protection-Encryption-Plan.md)
> 标记：`[ ]` 待办　`[-]` 进行中　`[x]` 完成
> 补充（2026-02）：Dashboard Web UI 迭代不影响本任务清单。

---

## Phase 0：基础设施准备

### P0.1 Cargo feature gate 搭建

- [x] `Cargo.toml` 新增 `[features]` 段：`lock`、`protect`、`crypt`
- [x] `windows-sys` 按 feature 拆分额外 features：
  - `lock` → `Win32_System_RestartManager`
  - `protect` → `Win32_Security`
  - `crypt` → `Win32_Storage_FileSystem`（EFS + Volume）
- [x] `age` 依赖标记 `optional = true`，绑定 `crypt` feature
- [x] 验证 `cargo check` 默认编译（无新 feature）零警告
- [x] 验证 `cargo check --features lock,protect,crypt` 通过

### P0.2 目录结构创建

- [x] 创建 `src/windows/mod.rs`（条件编译入口）
- [x] 创建 `src/security/mod.rs`
- [x] 在 `src/main.rs` 中添加模块声明

### P0.3 公共基础设施

- [x] `src/security/audit.rs`：审计日志模块
  - JSON Lines 格式写入 `~/.xun/audit.jsonl`
  - 字段：`timestamp`、`action`、`target`、`user`、`params`、`result`、`reason`
  - 10MB 轮转（重命名为 `.1` 并新建）
- [x] `src/commands/mod.rs` 中按 feature gate 注册新模块
- [x] 退出码常量定义（统一到一个位置）：
  - `0` 成功 / `2` 参数错误 / `3` 权限不足 / `10` 占用未授权 / `11` 解锁失败 / `20` 已登记重启

### P0.4 进度条工具封装

- [x] 基于 `indicatif` 封装 `ProgressReporter`
  - 批量操作项 ≥ 10 时启用
  - 样式：`{spinner:.green} [{bar:30}] {pos}/{len} {msg}`
  - 非 TTY 自动降级（与 `can_interact()` 兼容）

---

## Phase 1：文件解锁与操作（feature: `lock`）

### P1.1 Restart Manager FFI 封装

- [x] `src/windows/restart_manager.rs`
  - 封装 `RmStartSession` / `RmRegisterResources` / `RmGetList` / `RmEndSession`
  - 返回 `Vec<LockerInfo>` 结构体（pid、进程名、类型）
  - 支持批量注册（一次 `RmRegisterResources` 多文件）
  - 错误处理：会话创建失败、资源注册失败均返回 `Result`
- [x] 单元测试：对当前进程自身打开的文件能检测到占用

### P1.2 `lock who` 命令

- [x] `cli.rs`：新增 `LockCmd` + `LockSubCommand::Who`
  - 参数：`<path>`、`--format auto|table|tsv|json`
- [x] `src/commands/lock.rs`：`cmd_lock_who` 实现
  - 调用 Restart Manager 查询占用进程
  - 表格输出：PID、进程名、类型
  - 支持 `--format` 四种格式
- [x] 系统关键进程黑名单常量（`csrss.exe`、`wininit.exe`、`lsass.exe`、`services.exe` 等）
- [x] 集成测试：对被占用文件输出正确的 locker 列表

### P1.3 `rm` 命令（文件系统删除）

- [x] `cli.rs`：新增 `RmCmd`
  - 参数：`<path>`、`--unlock`、`--force-kill`、`--yes`、`--on-reboot`、`--dry-run`、`--format`
  - 帮助文本明确标注"文件删除"（区别于书签 `del`）
- [x] `src/commands/lock.rs`：`cmd_rm` 实现
  - 基础流程：PreCheck → 尝试删除 → 成功则返回
  - `--unlock` 流程：删除失败 → Restart Manager 探测 → 尝试关闭句柄 → 重试
  - `--force-kill` 流程：交互模式弹 `dialoguer::Confirm`；非交互需 `--yes`
  - `--on-reboot` 流程：调用 `MoveFileExW(MOVEFILE_DELAY_UNTIL_REBOOT)`
  - `--dry-run`：仅输出计划动作
  - 批量删除（目录递归）时启用进度条
- [x] 退出码：按 §3.5 规范返回 0/2/3/10/11/20
- [x] `--format json` 输出：`ok`、`code`、`action`、`target`、`message`、`locked_by`、`suggested_action`

### P1.4 `MoveFileExW` 封装

- [x] `src/windows/reboot_ops.rs`
  - 封装 `MoveFileExW(..., MOVEFILE_DELAY_UNTIL_REBOOT)`
  - 管理员权限检测：非管理员直接返回 `Err`（退出码 3）
  - 远程路径检测：UNC 路径直接拒绝并提示
- [x] 单元测试：非管理员调用返回权限错误

### P1.5 `mv` 命令（文件系统移动）

- [x] `cli.rs`：新增 `MvCmd`
  - 参数：`<src>`、`<dst>`、`--unlock`、`--force-kill`、`--yes`、`--dry-run`
- [x] `src/commands/lock.rs`：`cmd_mv` 实现
  - 复用 P1.3 的 unlock/force-kill 流程
  - 大文件移动可选 `MoveFileWithProgress` 进度回调
- [x] 集成测试：移动被占用文件 + `--unlock`

### P1.6 `ren` 命令（文件系统重命名）

- [x] `cli.rs`：新增 `RenFileCmd`（避免与书签 `RenameCmd` 冲突）
  - 参数：`<src>`、`<dst>`、`--unlock`、`--force-kill`、`--yes`、`--dry-run`
- [x] `src/commands/lock.rs`：`cmd_ren_file` 实现
  - 复用 P1.3 的 unlock/force-kill 流程
- [x] 集成测试：重命名被占用文件 + `--unlock`

### P1.7 命令注册与路由

- [x] `src/commands/mod.rs`：`#[cfg(feature = "lock")]` 条件注册 `lock`/`rm`/`mv`/`ren`
- [x] `cli.rs`：`SubCommand` 枚举中条件编译新增变体
- [x] `cargo test --features lock` 全量通过
- [x] 现有 47 个测试不受影响（`cargo test` 无 feature 仍通过）

---

## Phase 2：防误操作保护（feature: `protect`）

### P2.1 保护规则模型

- [x] `src/config.rs`：扩展 `GlobalConfig`，新增 `protect` 字段
  ```rust
  struct ProtectConfig {
      rules: Vec<ProtectRule>,
  }
  struct ProtectRule {
      path: String,
      deny: Vec<String>,       // "delete", "move", "rename"
      require: Vec<String>,    // "--force", "--reason"
  }
  ```
- [x] 规则匹配逻辑：路径前缀匹配（规范化后比较）
- [x] 单元测试：规则命中/未命中/条件满足放行

### P2.2 `protect set/clear/status` 命令

- [x] `cli.rs`：新增 `ProtectCmd` + `ProtectSubCommand`
  - `set <path> --deny delete,move,rename [--system-acl]`
  - `clear <path> [--system-acl]`
  - `status <path> [--format auto|table|tsv|json]`
- [x] `src/commands/protect.rs`：实现三个子命令
  - `set`：写入 config.json 的 protect.rules
  - `clear`：移除匹配规则
  - `status`：查询并展示当前保护状态
- [x] 所有策略变更写审计日志

### P2.3 rm/mv/ren 集成保护拦截

- [x] 在 P1.3/P1.5/P1.6 的 PreCheck 阶段插入保护规则检查
  - 命中规则且未满足 `require` 条件 → 拒绝（退出码 2 + 提示）
  - `--force --reason "..." --yes` 满足条件 → 放行 + 写审计日志
- [x] `cli.rs`：`RmCmd`/`MvCmd`/`RenFileCmd` 新增 `--force`、`--reason` 参数
- [x] 集成测试：保护规则拦截 + 强制放行

### P2.4 审计日志集成

- [x] rm/mv/ren 的 `--force-kill`、`--on-reboot` 操作写审计日志
- [x] protect set/clear 操作写审计日志
- [x] 验证 `audit.jsonl` 格式正确、轮转生效

### P2.5 关键目录模板

- [x] 预置模板：`Desktop`、`Documents`、`Downloads` 保护规则
- [x] `xun protect init` 一键应用模板（可选已推迟至统一设置）

---

## Phase 3：文件系统级硬核防护与 EFS 加密（feature: `crypt`）

> **NOTE:** 本次迭代专注 Windows 原生防御（ACL + EFS）。跨平台应用层加密 `age` 移至后续 Phase 4。

### P3.1 关键系统目录防爆破拦截

- [ ] `src/windows/mod.rs`（或新增 `crypto_guard.rs`）
  - 维护硬编码黑名单前缀：`C:\Windows`, `C:\Program Files`, `C:\Boot`, `C:\ProgramData` 及其它重要根路径。
  - 实现拦截器 `ensure_safe_target(path: &Path) -> Result<()>`。
- [ ] 将拦截器接入到所有的 ACL 操作和 EFS 操作前。

### P3.2 NTFS ACL 封装（保护与回滚）

- [x] `src/acl/` ACL 模块落地
  - 封装 `GetNamedSecurityInfoW` / `SetNamedSecurityInfoW`。
  - 已覆盖：view/add/remove/purge/diff/batch/effective/copy/backup/restore/inherit/owner/orphans/repair/audit/config。
  - 内存管理：统一使用 `LocalFree` 释放 `PSECURITY_DESCRIPTOR` 等指针。
- [ ] `protect --system-acl` 与 ACL 模块接入（含 `ACCESS_DENIED_ACE` 置顶策略）
  - **核心设计**：读取原有 DACL，插入 `DELETE` / `FILE_DELETE_CHILD` 的 Deny ACE 于 Index 0，再追加原有 ACE。
  - **说明**：当前已提供 `xun acl` 独立命令链路，保护策略接入待完成。
- [ ] 集成测试：
  - [ ] 设定 `Deny Delete` 后使用 Rust `fs::remove_file` 预期被操作系统拦截（权限不足）。
  - [ ] `restore` 后验证能否成功删除被保护对象。

### P3.3 卷加密能力感知与缓存

- [ ] `src/windows/volume.rs`
  - 封装 `GetVolumeInformationW`。
  - 路径求根溯源（`Path` -> `Prefix`）找到目标所在的卷根目录。
  - 解析 `FS_FILE_ENCRYPTION` 和 `FS_PERSISTENT_ACLS` 支持位。
  - 卷属性获取结果跨命令/多文件缓存（如使用 `OnceLock` 或哈希表）。
- [ ] 单元/手动测试：能够正确甄别 NTFS 与 FAT32/exFAT 卷支持能力抛出软提示。

### P3.4 EFS 加密封装与占用处理

- [ ] `src/windows/efs.rs`
  - 封装 `EncryptFileW` / `DecryptFileW`。
  - 针对 `ERROR_SHARING_VIOLATION` 进行专项捕捉，返回特定的错误告知 CLI 端引发“锁占用检查提示”。
  - 兼容识别系统级与只读文件加密。

### P3.5 `encrypt/decrypt` 命令与 `protect --system-acl`

- [ ] `cli.rs`：新增 `EncryptCmd` / `DecryptCmd` (`--efs`, `--status`, `--dry-run`)
- [ ] `src/commands/crypt.rs`：串联 EFS 路径验证、缓存检测并调用 P3.4 实施。
- [ ] `src/commands/protect.rs`：
  - 更新令 `protect set/clear` 适配 `--system-acl` 的分发。
  - 实施 ACL 的修改并在恢复时抹去对应的阻断 ACE。
- [ ] 敏感动作审计记录打通 `xun audit`。
  - 不覆盖原文（默认输出 `.age` 后缀）

### P3.6 `encrypt/decrypt` 命令（age 模式）

- [x] `cli.rs`：扩展 `EncryptCmd` / `DecryptCmd`
  - `encrypt <path> --to <recipient> [--out <file>]`
  - `decrypt <path.age> --identity <keyfile> [--out <file>]`
  - `encrypt <path.age> --rekey --from <old> --to <new>`
  - `encrypt <path.age> --verify`
- [x] `src/commands/crypt.rs`：age 分支实现
  - 无 `--efs` 时走 age 路径
  - 批量加密启用进度条
- [x] 加密/解密操作写审计日志

### P3.7 命令注册与路由

- [x] `src/commands/mod.rs`：`#[cfg(feature = "crypt")]` 条件注册 `encrypt`/`decrypt`
- [x] `cargo test --features lock,protect,crypt` 全量通过
- [x] 现有测试不受影响

---

## 验收检查清单

### 功能验收

- [ ] `lock who` 正确识别占用进程
- [ ] `rm --unlock` 解锁后删除成功
- [ ] `rm --force-kill` 交互模式弹确认、非交互需 `--yes`
- [x] `rm --on-reboot` 非管理员返回退出码 3
- [x] `rm --on-reboot` 远程路径直接报错
- [ ] `protect set/clear/status` 规则增删查正常
- [ ] 保护规则拦截 rm/mv/ren，`--force --reason --yes` 放行
- [x] `encrypt/decrypt --efs` 加解密回环
- [x] `encrypt/decrypt --to/--identity` age 加解密回环
- [x] `--format json` 所有命令输出字段稳定
- [x] `--dry-run` 所有危险命令不执行实际操作
- [x] 审计日志格式正确、轮转生效

### 性能验收

- [x] 批量删除 1k 文件 < 5s（无占用场景）
- [x] `lock who` 单文件探测 < 200ms
- [x] 进度条在 ≥ 10 项时正确显示、非 TTY 静默

### 兼容性验收

- [x] `cargo check`（无 feature）零警告
- [x] `cargo test`（无 feature）现有 47 测试全部通过
- [x] `cargo test --features lock` 通过
- [x] `cargo test --features lock,protect` 通过
- [x] `cargo test --features lock,protect,crypt` 通过
- [ ] release 二进制体积：无 feature < 当前基线 +5%

---

## 依赖关系

```
P0.1 ─┬─→ P0.2 ─→ P0.3 ─→ P0.4
      │
      ├─→ P1.1 ─→ P1.2 ─→ P1.3 ─┬─→ P1.5
      │                           ├─→ P1.6
      │                           └─→ P1.7
      │
      ├─→ P1.4 ─→ P1.3（--on-reboot 分支）
      │
      ├─→ P2.1 ─→ P2.2 ─→ P2.3 ─→ P2.4 ─→ P2.5
      │
      └─→ P3.1 ─┐
          P3.2 ─┼─→ P3.3 ─→ P3.4
          P3.5 ─┘         ─→ P3.6 ─→ P3.7
```
