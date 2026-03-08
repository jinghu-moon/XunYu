# xun 文件解锁、防误删保护与文件加密方案

> 目标：在 Windows 场景下，统一提供“可控删除（含解锁）+ 防误删防移动防重命名 + 文件加密”能力，并遵循主流 CLI 设计（安全默认、可脚本化、可审计）。

---

## 1. 设计目标与边界

### 1.1 目标

1. **文件被占用时可定位并处理**：自动识别占用进程，按策略解锁，完成删除/移动/重命名。
2. **防误操作保护**：可对文件/目录施加"删除/移动/重命名保护"策略。
3. **文件加密**：支持系统级加密与跨平台加密两种模式。
4. **CLI 一致性**：统一 `--dry-run`、`--format`、`--yes`、明确退出码，所有查询命令支持 `--format auto|table|tsv|json`。
5. **批量操作进度反馈**：使用 `indicatif` 提供进度条，非 TTY 环境自动降级静默。

### 1.2 非目标（首期）

- 不做内核驱动级强制防护（仅用户态能力）。
- 不做企业级 KMS（首期先做本地密钥与可插拔预留）。
- 不承诺绕过系统权限模型（管理员权限仍按 Windows 规则执行）。

---

## 2. 总体能力模型（统一到一个工作流）

对删除/移动/重命名统一走同一流水线：

1. **PreCheck**：权限、路径类型、策略检查（是否受保护、是否加密、是否锁定）。
2. **Policy Gate**：根据命令参数决定是否允许自动解锁/强制终止/重启后执行。
3. **Action**：执行 delete/move/rename，批量操作时通过 `indicatif::ProgressBar` 展示进度。
4. **Fallback**：失败时进入 lock detect -> unlock -> retry；仍失败可登记 reboot。
5. **Audit**：输出结构化结果（文本 + JSON）并写审计日志。

### 2.1 性能/速度/占用优化要点（补充）

1. **按需触发锁检测**：仅在 delete/move/rename 真实失败且符合共享冲突（打开句柄未包含 `FILE_SHARE_DELETE`）时才进入 Restart Manager 流程，避免每次操作都做进程扫描开销。（依据：DeleteFile 在存在未共享删除的打开句柄时会失败）  
2. **批量注册资源**：`RmRegisterResources` 支持一次注册多个文件/服务/进程，避免逐文件频繁调用，按批/组件注册资源以降低调用次数。  
3. **远程共享直拒延迟删除**：`MOVEFILE_DELAY_UNTIL_REBOOT` 不支持远程共享路径，`--on-reboot` 需在远程路径场景直接提示不可用并退出。  
4. **EFS 前置能力检测**：加密前先做能力检测与约束校验（详见 5.1.1），避免对不支持的卷或对象重复尝试。  

---

## 3. 能力 A：文件解锁与删除/移动/重命名

### 3.1 主流实现参考

- 使用 **Restart Manager** 查询占用进程（`RmStartSession` / `RmRegisterResources` / `RmGetList`）。
- 对“当前无法立即删除”的场景，使用 `MoveFileEx(..., MOVEFILE_DELAY_UNTIL_REBOOT)` 做重启后处理。
- 保持“安全默认”：不默认杀进程，需显式开关。

### 3.2 命令设计

> **命名说明**：现有 CLI 中 `del`/`delete` 用于删除书签，`rm`/`mv`/`ren` 用于文件系统操作，两者语义不同。帮助文本中需明确标注 `rm` 为"文件删除"以避免混淆。

```bash
# 仅探测占用（支持 --format auto|table|tsv|json）
xun lock who "C:\data\a.txt"

# 删除时自动解锁（不强杀）
xun rm "C:\data\a.txt" --unlock

# 删除时允许强制结束占用进程（交互模式弹确认，非交互需 --yes）
xun rm "C:\data\a.txt" --unlock --force-kill

# 失败后登记重启删除（需管理员权限，否则退出码 3）
xun rm "C:\data\a.txt" --unlock --on-reboot

# 移动/重命名同策略
xun mv a.txt b.txt --unlock
xun ren a.txt b.txt --unlock --force-kill
```

### 3.3 参数约定

- `--unlock`：失败后自动进入解锁流程。
- `--force-kill`：允许结束占用进程（高风险）。交互模式下触发二次确认弹窗；非交互模式下必须搭配 `--yes` 才执行。
- `--on-reboot`：最终失败时登记重启后执行。需管理员权限，非管理员直接返回退出码 3 并提示"需要管理员权限"。
- `--dry-run`：仅展示计划动作，不执行。
- `--format`：输出格式 `auto|table|tsv|json`（与现有命令一致，替代原 `--json`）。

### 3.4 安全策略

- 默认不结束进程；只报告 locker 列表。
- 系统关键进程黑名单禁止 kill（如 `csrss.exe`、`wininit.exe`、`lsass.exe`、`services.exe`）。
- 非交互模式下，没有 `--yes` 不执行破坏性操作。

### 3.5 退出码

- `0`：成功。
- `2`：参数错误（与现有 CLI 一致）。
- `3`：权限不足（如非管理员执行 `--on-reboot`）。
- `10`：检测到占用但未授权强制处理。
- `11`：已尝试解锁仍失败。
- `20`：已登记重启后执行（当前未完成）。

---

## 4. 能力 B：防删除/防移动/防重命名（逆向保护）

该能力用于“先保护，再按授权解锁”。建议分两层：

### 4.1 软保护层（xun 元数据策略）

在 `~/.xun/config.json` 的 `protect` 字段下记录路径规则（与 `proxy` 字段平级，避免配置碎片化）：

```json
{
  "protect": {
    "rules": [
      {
        "path": "C:/data/report",
        "deny": ["delete", "move", "rename"],
        "require": ["--force", "--reason"]
      }
    ]
  }
}
```

执行 `rm/mv/ren` 时先拦截：
- 命中规则且未满足条件 -> 直接拒绝。
- 仅在 `--force --reason "..." --yes` 下放行。

**优点**：跨文件系统、实现快、与 CLI 行为一致。
**缺点**：只能约束通过 xun 触发的操作，不能拦截外部程序。

### 4.2 系统保护层（NTFS ACL）

对目录/文件设置 ACL：
- 收紧 `DELETE` 与 `FILE_DELETE_CHILD` 权限。
- 可配“仅管理员可删/可改名”。

命令建议：

```bash
xun protect set "C:\data\report" --deny delete,move,rename
xun protect status "C:\data\report"
xun protect clear "C:\data\report"
```

**说明**：Windows 上“重命名/移动/删除”在权限模型上耦合较强，需通过 ACL 组合做策略折中；文档中明确“可重命名但不可删除”在部分场景不易完全分离。

### 4.3 两层联动建议

- 默认启用软保护层（可解释、可审计、跨卷稳定）。
- `--system-acl` 作为增强模式，面向管理员场景。
- 所有策略变更写审计日志：操作人、时间、路径、原因。

---

## 5. 能力 C：文件加密

建议提供两种主流模式：

### 5.1 模式 1：系统原生加密（EFS）

- 适用：Windows 本机使用、对体验要求高（透明加解密）。
- 实现：调用 EFS 相关 API（如 `EncryptFile`/`DecryptFile`）或系统工具。
- 优点：用户透明、性能好。
- 局限：依赖 NTFS 与当前用户证书环境，跨机迁移复杂。

#### 5.1.1 运行约束与预检（补充）

1. `EncryptFile` 需要对目标文件 **独占访问**；文件被占用时会失败，应先提示并建议解锁。
2. **SMB 3.0 Continuous Availability** 场景不支持 EFS（包含 TFO、SO、CsvFS、ReFS），需明确提示并拒绝执行。
3. 加密前使用 `GetVolumeInformation` 检测 `FS_FILE_ENCRYPTION`，不支持则直接报错。用户传入的路径需先 canonicalize 再提取卷根（如 `C:\`），以正确处理相对路径。
4. 不可加密对象：压缩文件、系统文件、系统目录、根目录（应直接拒绝并提示原因）。

命令建议（扁平化设计，避免 3 层嵌套，与现有 CLI 风格一致）：

```bash
xun encrypt "C:\data\a.txt" --efs
xun decrypt "C:\data\a.txt" --efs
xun encrypt "C:\data\a.txt" --efs --status   # 查询加密状态
```

### 5.2 模式 2：应用层加密（age/AES-GCM）

- 适用：跨平台传输、备份场景、零信任存储。
- 建议优先 `age` 格式（生态成熟，CLI 友好），或在内部实现 `AES-256-GCM` + 强 KDF（Argon2id）。
- 密钥来源：
  - 公钥加密（推荐）
  - 口令加密（需 KDF + salt + 参数版本化）

命令建议（扁平化，与 EFS 共用 `encrypt`/`decrypt` 子命令，通过参数区分模式）：

```bash
xun encrypt report.docx --to age1... --out report.docx.age
xun decrypt report.docx.age --identity key.txt --out report.docx
xun encrypt report.docx.age --rekey --from oldkey.txt --to age1...
xun encrypt report.docx.age --verify   # 完整性校验
```

> **说明**：`--efs` 走系统原生加密，无 `--efs` 时走应用层加密（age）。`age` crate 体积较大（~500KB-1MB），通过 Cargo feature gate `--features crypt` 隔离，默认不编译。

### 5.3 加密安全基线

1. **优先认证加密**：默认使用 AEAD（如 AES-GCM/CCM 或 age 标准实现）。  
2. **禁止自研算法**：不引入自定义或弱加密方案。  
3. **随机数要求**：密钥、nonce、salt 必须来自安全随机源（CSPRNG）。  
4. **完整性校验**：若非 AEAD，需采用 Encrypt-then-MAC 等完整性方案。  
5. **元数据最小化**：可选不泄露明文文件名，并提供 `xun encrypt <path> --verify` 完整性校验。  

---

## 6. 统一 CLI 交互规范（主流设计对齐）

1. **安全默认**：危险动作需显式参数触发。
2. **可预演**：所有危险命令支持 `--dry-run`。
3. **可脚本**：稳定 `--format json` 输出与退出码（替代原 `--json`，与现有命令统一）。
4. **可追责**：策略变化与强制动作写审计日志。
5. **可恢复**：删除失败时支持重启后执行和回滚提示。
6. **可观测**：批量操作通过 `indicatif` 展示进度条，非 TTY 自动降级。  

### 6.1 交互与可用性细节（偏交互版）

1. **失败即解释**：当删除失败时明确提示"占用/权限/路径类型"等原因，并给出下一步建议（如 `lock who` / `--unlock` / `--on-reboot`）。
2. **渐进式升级**：`--unlock` 只尝试合规解锁；`--force-kill` 在交互模式下弹二次确认，非交互模式下必须搭配 `--yes`。
3. **交互默认安全**：交互式提示默认选项为"否"，非交互场景没有 `--yes` 直接拒绝。
4. **结构化输出（最小字段）**：通过 `--format json` 输出机器可读结果，字段保持稳定且尽量少：`ok`、`code`、`action`、`target`、`message`；当涉及占用时可选 `locked_by`，当有建议动作时可选 `suggested_action`。
5. **长耗时反馈**：批量操作使用 `indicatif::ProgressBar` 展示进度；大文件移动可选 `MoveFileWithProgress` 提供进度回调；非 TTY 场景自动降级静默。  

#### 6.1.1 `--format json` 输出示例

```json
{
  "ok": false,
  "code": 11,
  "action": "rm",
  "target": "C:\\data\\a.txt",
  "message": "文件被占用",
  "locked_by": [{ "pid": 1234, "name": "EXCEL.EXE" }],
  "suggested_action": "xun lock who \"C:\\data\\a.txt\""
}
```

### 6.2 审计日志规范

- **路径**：跟随 `db_path()` 同目录，即 `~/.xun/audit.jsonl`。
- **格式**：JSON Lines（每行一条记录），便于 `grep`/`jq` 处理。
- **轮转**：单文件上限 10MB，超限时重命名为 `audit.jsonl.1` 并新建。
- **字段**：`timestamp`、`action`、`target`、`user`、`params`、`result`、`reason`（可选）。
- **Dashboard 可视化**：Web UI 的 Audit 面板已接入审计日志，支持筛选与 CSV/JSON 导出（不改变日志格式）。

```json
{"timestamp":"2026-02-20T14:30:00+08:00","action":"rm","target":"C:\\data\\a.txt","user":"seeyuer","params":{"unlock":true,"force_kill":false},"result":"ok"}
```

### 6.3 进度条规范

- **依赖**：`indicatif` 0.17（与现有 `console` 同作者，零额外传递依赖）。
- **触发条件**：批量操作（目录递归删除/移动/加密）且目标项 ≥ 10 个时启用。
- **样式**：`{spinner:.green} [{bar:30}] {pos}/{len} {msg}`。
- **降级**：非 TTY 环境（管道/重定向）自动静默，与 `can_interact()` 兼容。

---

## 7. 架构与代码落位建议

### 7.1 文件布局

```
src/commands/
  lock.rs              # lock who + rm/mv/ren --unlock 入口
  protect.rs           # protect set/clear/status
  crypt.rs             # encrypt/decrypt 入口（统一分发 EFS / age）
src/windows/
  restart_manager.rs   # Restart Manager FFI 封装
  reboot_ops.rs        # MoveFileExW 封装
  efs.rs               # EFS FFI 封装（EncryptFileW / DecryptFileW）
  volume.rs            # GetVolumeInformation 封装（能力检测）
src/acl/
  mod.rs               # ACL 模块入口
  reader.rs            # 读取/解析 ACL
  writer.rs            # 写入/变更 ACL
  diff.rs              # ACL 差异对比
  export.rs            # 备份/恢复/CSV
  audit.rs             # ACL 审计日志
  repair.rs            # ACL 修复/批处理
  effective.rs         # 有效权限计算
  orphan.rs            # 孤儿 SID 检测
  parse.rs             # 权限/继承解析
  privilege.rs         # 特权启用封装
src/security/
  age_crypto.rs        # age 加密（feature-gated: --features crypt）
  audit.rs             # 审计日志（JSON Lines）
```

### 7.2 Cargo feature gate

```toml
[features]
default = []
lock    = []                          # Phase 1: Restart Manager + reboot ops
protect = []                          # Phase 2: ACL 保护
crypt   = ["dep:age"]                 # Phase 3: age 加密（~500KB 增量）

[dependencies]
age = { version = "0.10", optional = true }
```

> **说明**：`lock`/`protect` feature 仅控制 `windows-sys` 的额外 features 编译（`Win32_System_RestartManager`、`Win32_Storage_FileSystem`、`Win32_Security`），不引入新 crate。默认不编译这三个能力，按需启用，保持基础二进制体积最小。

---

## 8. 分阶段实施（建议）

### Phase 1（P0）— feature: `lock`

- `lock who` 命令（支持 `--format auto|table|tsv|json`）。
- `rm/mv/ren --unlock` 主流程 + Restart Manager 探测。
- `--force-kill`（交互弹确认 / 非交互需 `--yes`）+ `--dry-run`。
- `--on-reboot` 兜底（非管理员返回退出码 3）。
- `indicatif` 进度条集成。

### Phase 2（P1）— feature: `protect`

- `protect` 软规则层（deny delete/move/rename），规则存入 `~/.xun/config.json`。
- 审计日志（`~/.xun/audit.jsonl`，JSON Lines，10MB 轮转）。
- `--reason` 参数与关键目录模板。

### Phase 3（P2）— feature: `crypt`

- `protect --system-acl` 增强模式（NTFS ACL）。
- 新增 `xun acl` 子命令：view/add/remove/purge/diff/batch/effective/copy/backup/restore/inherit/owner/orphans/repair/audit/config。
- `encrypt/decrypt --efs`（系统加密）。
- `encrypt/decrypt --to/--identity`（age 应用层加密，feature-gated）。

---

## 9. 风险与规避

- **误杀关键进程**：进程黑名单 + 默认不杀 + 二次确认。
- **权限不足**：清晰报错（退出码 3）并提示"需要管理员权限"，`--on-reboot` 非管理员直接拒绝。
- **ACL 误配置**：提供 `protect snapshot/restore`。
- **密钥丢失**：强提醒备份，`encrypt` 默认不覆盖原文。
- **跨卷/网络盘差异**：文档标注行为差异并测试覆盖；`--on-reboot` 远程路径直接报错。
- **二进制体积膨胀**：通过 Cargo feature gate 隔离 `lock`/`protect`/`crypt`，默认不编译，保持基础体积最小。

---

## 10. 测试清单（关键）

### 10.1 功能与报错测试（示例）

1. 文件被占用：`lock who` 能定位锁定进程，删除失败时给出下一步建议。
2. `--unlock` 无强杀：仅尝试合规解锁，失败时退出码为 11。
3. `--unlock --force-kill`：交互模式弹确认后完成操作，退出码为 0。
4. `--on-reboot`：注册成功返回 20；远程共享路径应直接报错并退出；非管理员返回 3。
5. 软保护规则生效：未带 `--force --reason --yes` 拒绝执行。
6. ACL 模式：普通用户不可删，管理员可恢复。
7. `encrypt/decrypt`：加解密回环、篡改检测、错误密钥失败。
8. `--format json`：字段稳定、兼容脚本解析。
9. 参数错误：路径不存在/无权限/无效参数时返回 2 并输出明确原因；权限不足返回 3。

示例命令：

```bash
xun lock who "C:\data\busy.txt"
xun rm "C:\data\busy.txt" --unlock
xun rm "C:\data\busy.txt" --unlock --force-kill
xun rm "\\server\share\busy.txt" --on-reboot
xun protect set "C:\data\report" --deny delete,move,rename
xun rm "C:\data\report\a.txt"
xun encrypt "C:\data\a.txt" --efs
xun encrypt "C:\data\a.txt" --to age1... --out "C:\data\a.txt.age"
```

### 10.2 性能测试（吞吐/延迟）

1. 批量删除：对 1k/10k 文件目录执行 `rm`，记录总耗时与平均延迟。  
2. 批量移动：对大目录执行 `mv`，比较开启/关闭 `--unlock` 的耗时差异。  
3. 树形扫描：`tree` 在 50k+ 文件下的耗时与输出行数。  

示例命令（PowerShell）：

```powershell
Measure-Command { xun rm "D:\data\big\*" --unlock --dry-run }
Measure-Command { xun mv "D:\data\big" "D:\data\big_moved" --unlock }
Measure-Command { xun tree "D:\data\big" --depth 4 }
```

### 10.3 速度测试（冷启动/热启动）

1. 冷启动：清空缓存后执行 `xun lock who`，记录首次耗时。  
2. 热启动：连续执行同一命令 10 次，记录 P50/P95。  
3. 交互路径：`z` / `open` 在交互与非交互模式下的耗时差异。  

示例命令（PowerShell）：

```powershell
1..10 | ForEach-Object { Measure-Command { xun lock who "C:\data\a.txt" } | Select-Object TotalMilliseconds }
```

### 10.4 占用测试（CPU/内存/句柄）

1. CPU 峰值：大目录操作期间的 CPU 峰值与平均值。  
2. 内存峰值：`tree`/`bak` 在大目录下的峰值内存。  
3. 句柄泄漏：多次重复操作后句柄数稳定。  

示例命令（PowerShell）：

```powershell
Get-Process xun | Select-Object CPU, WorkingSet, HandleCount
```

### 10.5 报错与回退测试（健壮性）

1. 非法路径：空路径、包含非法字符、超长路径。  
2. 权限不足：普通用户对系统目录执行 `rm/mv/ren`。  
3. 远程/网络盘：`--on-reboot` 必须明确不可用。  
4. 进程白名单：尝试 `--force-kill` 作用于系统关键进程时必须拦截。  
5. JSON 输出：错误路径也要输出稳定字段并返回非 0。  

### 10.6 负载与并发测试

1. 多进程并发：同时运行 5 个 `rm --unlock`，观察锁检测稳定性。  
2. 长时间运行：持续执行 `tree` 与 `bak` 1 小时，验证无资源泄漏。  

---

## 11. 推荐最终命令集（草案）

```bash
# ── 文件解锁与操作（Phase 1: --features lock）──
xun lock who <path> [--format auto|table|tsv|json]
xun rm <path> [--unlock] [--force-kill] [--on-reboot] [--dry-run] [--format json]
xun mv <src> <dst> [--unlock] [--force-kill] [--dry-run]
xun ren <src> <dst> [--unlock] [--force-kill] [--dry-run]

# ── 防误操作保护（Phase 2: --features protect）──
xun protect set <path> --deny delete,move,rename [--system-acl]
xun protect clear <path> [--system-acl]
xun protect status <path> [--format auto|table|tsv|json]

# ── 文件加密（Phase 3: --features crypt）──
xun encrypt <path> --efs                              # EFS 系统加密
xun decrypt <path> --efs                              # EFS 系统解密
xun encrypt <path> --to <recipient> [--out <file>]    # age 加密
xun decrypt <path.age> --identity <keyfile> [--out <file>]  # age 解密
xun encrypt <path.age> --verify                       # 完整性校验
```

---

## 12. 结论

这是一个可组合的三能力方案：

- **解锁与执行动作**：解决“文件在用”导致的操作失败。
- **逆向保护**：防止误删、误移动、误重命名。
- **文件加密**：保护静态数据与跨环境传输。

按 Phase 1 -> Phase 3 逐步落地，可以在不大幅改造现有命令体系的前提下，快速形成差异化能力。

---

## 参考资料

- Restart Manager `RmRegisterResources`（Microsoft Learn）
  https://learn.microsoft.com/en-us/windows/win32/api/restartmanager/nf-restartmanager-rmregisterresources
- MoveFileEx `MOVEFILE_DELAY_UNTIL_REBOOT`（Microsoft Learn）
  https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexa
- MoveFileWithProgress（Microsoft Learn）
  https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefilewithprogressa
- DeleteFileW 语义与共享限制（Microsoft Learn）
  https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-deletefilew
- File Security and Access Rights（Microsoft Learn）
  https://learn.microsoft.com/en-us/windows/win32/fileio/file-security-and-access-rights
- File Access Rights Constants（Microsoft Learn）
  https://learn.microsoft.com/en-us/windows/win32/fileio/file-access-rights-constants
- File Encryption / EFS（Microsoft Learn）
  https://learn.microsoft.com/en-us/windows/win32/fileio/file-encryption
- EncryptFileA（Microsoft Learn）
  https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-encryptfilea
- OWASP Cryptographic Storage Cheat Sheet
  https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html
- age 项目（GitHub）
  https://github.com/FiloSottile/age
- age Rust crate（docs.rs）
  https://docs.rs/age/
