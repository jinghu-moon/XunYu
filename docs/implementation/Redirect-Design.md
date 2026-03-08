# xun redirect — 文件分类重定向设计

**版本记录**
- v0.3.1 (2026-02-22)：补充 Windows API 对齐（MoveFileEx/CopyFileEx/ReadDirectoryChangesExW）与同类工具（organize-tool/Hazel/File Juggler）竞品分析。
- v0.3.0 (2026-02-22)：基于源码审计全面对齐现有架构；补充 feature gate、config 扩展、可复用模块映射、测试策略等实现细节。
- v0.2.1 (2026-02-21)：补充规范化约束（MUST/SHOULD/MAY），明确 CLI/配置/冲突/审计/Watch 行为边界。
- v0.2.0 (2026-02-21)：对齐现有架构与 CLI 规范；明确配置路径/命令入口；补充审计字段映射与 Watch 可靠性要求。
- v0.1.0 (2026-02-20)：初版设计。

> 按规则将目录中的文件分类移动/复制到目标子目录。
> 定位：轻量整理工具，不做通用规则引擎。注重极速、安全与免打扰。

---

## 1. 命令语法

```
xun redirect [source] [选项]
xun serve [-p <port>] # Dashboard（Redirect 配置入口）
```

| 参数/选项          | 说明                                 | 默认值    |
| ------------------ | ------------------------------------ | --------- |
| `[source]`         | 待整理目录                           | 当前目录  |
| `--profile <name>` | 使用命名配置                         | `default` |
| `--watch`          | 启动静默守护监听模式，新文件自动分类 | -         |
| `--dry-run`        | 仅预览，不执行                       | -         |
| `--copy`           | 复制而非移动                         | -         |
| `-y` / `--yes`     | 跳过确认                             | -         |
| `-f` / `--format`  | 输出格式                             | `auto`    |

补充约定（与现有 CLI 规范保持一致）：
- 结果数据 → stdout；提示/进度/错误 → stderr。
- 非交互/非 TTY 场景禁止交互提示，输出稳定可脚本化。
- `--format json|tsv` 字段稳定，便于管道/脚本消费。

### 1.1 规范化约束（MUST / SHOULD / MAY）

**术语约定**：本节中的“必须/应当/可以”分别对应 MUST/SHOULD/MAY。

- **参数有效性（必须）**：`--profile` 不存在时必须返回非 0 并输出明确错误；`source` 不存在或不可读必须失败。
- **无副作用预览（必须）**：`--dry-run` 必须零副作用；输出结构与真实执行一致。
- **交互边界（必须）**：非交互场景不得进行任何确认提示；`overwrite` 在非交互场景必须要求 `--yes`，否则报错退出。
- **复制语义（必须）**：`--copy` 不得删除源文件，且审计记录必须标记 `copy=true`。
- **输出稳定性（应当）**：`--format json|tsv` 字段顺序与命名应当稳定，版本内不得破坏。

---

## 2. 规则配置

建议将配置统一收拢到默认配置文件 `~/.xun.config.json`（可用 `XUN_CONFIG` 覆盖）的 `redirect.profiles` 字段中。

```json
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          {
            "name": "图片",
            "match": {
              "ext": ["jpg", "jpeg", "png", "gif", "webp", "svg", "bmp", "ico"]
            },
            "dest": "./Images"
          },
          {
            "name": "文档",
            "match": {
              "ext": [
                "pdf",
                "doc",
                "docx",
                "xls",
                "xlsx",
                "ppt",
                "pptx",
                "txt",
                "md"
              ]
            },
            "dest": "./Documents"
          },
          {
            "name": "音频",
            "match": { "ext": ["mp3", "wav", "flac"] },
            "dest": "./Audio"
          }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_new"
      }
    }
  }
}
```

规则按数组顺序匹配，**首条命中即停止**（不重复分类）。

配置细则：
- `ext` 不带点，大小写不敏感。
- `dest` 允许相对路径，默认相对于 `source`。
- `glob` 语义与现有 `.xunignore` 保持一致（大小写与路径归一化一致）。

### 2.1 规范化约束（MUST / SHOULD / MAY）

- **配置完整性（必须）**：指定 `profile` 时，必须存在 `redirect.profiles.<name>`；`rules` 为空或缺失视为配置错误。
- **规则有效性（必须）**：每条规则必须包含 `match` 且至少包含一个匹配条件；`dest` 不能为空。
- **路径归一（应当）**：执行前应当对 `source`/`dest` 做路径归一化与大小写折叠，避免重复搬运。
- **自触发防护（应当）**：在 `--watch` 下，所有规则 `dest` 应当自动加入忽略集，避免回流重复分类。
- **`unmatched` 边界（应当）**：P0/P1 阶段仅支持 `skip`；如配置为其他值应当显式报错并提示暂不支持。

---

## 3. 跨维打击：原生平台级静默监听 (--watch)

基于 Windows 平台的极致体验追求，偏离传统轮询和臃肿跨平台库（如 `notify`），打造基于 **零外部依赖底层的 `ReadDirectoryChangesW` 原生监听 + 占用识别防护**。

### 3.1 工作流特性

1. **纯净挂载**：复用项目中现有的 `windows-sys` 直接调用 `ReadDirectoryChangesW`，建立不阻滞主进程的重叠 I/O 并行等待，实现 0% CPU 闲置消耗的事件驱动。
2. **事件解析**：按 `FILE_NOTIFY_INFORMATION` 变长链表解析事件；`Rename` 需要成对处理 `OldName/NewName`。
3. **溢出回收**：当缓冲区溢出或 `lpBytesReturned=0` 时触发“全量扫描补账”，避免丢事件。
4. **防暴毙安全锁（Restart Manager 集成）**：
   - 当探测到新文件（如下载中的 `.crdownload`）时，**不立即盲目移动**。
   - 先走 `handle_query`（支持目录/文件），仅在“全是文件路径”时 fallback 到 Restart Manager。
   - 发现占用则挂入「重试队列（Retry Queue）」，完全释出后再执行转移。
5. **彻底静默运行**：以守护进程态持续蛰伏运作。

### 3.2 规范化约束（MUST / SHOULD / MAY）

**术语约定**：本节中的“必须/应当/可以”分别对应 MUST/SHOULD/MAY。

- **异步完成模型（必须）**：`--watch` 必须以 OVERLAPPED 异步模式运行，并固定一种完成机制（IOCP / completion routine / `GetOverlappedResult`）。异步模式不得直接依赖 `lpBytesReturned`，必须以完成结果返回的字节数为准。
- **缓冲区溢出补账（必须）**：当 `ReadDirectoryChangesW` 成功返回但 `lpBytesReturned == 0`，或返回 `ERROR_NOTIFY_ENUM_DIR`，必须触发“全量扫描补账”。补账完成前不得清空重试队列。
- **网络共享限制（必须）**：若 `source` 为网络共享目录，`nBufferLength` 必须 ≤ 64KB；超过上限视为配置错误并拒绝启动 `--watch`。
- **事件解析规则（必须）**：`FILE_NOTIFY_INFORMATION` 必须以 `NextEntryOffset` 遍历；同目录重命名必须成对处理 `FILE_ACTION_RENAMED_OLD_NAME/NEW_NAME` 且保持相邻；跨目录重命名应按 `FILE_ACTION_REMOVED + FILE_ACTION_ADDED` 处理。
- **占用/权限失败处理（应当）**：出现“文件/目录存在打开句柄不可重命名”或权限不足时，应进入重试队列并记录原因，避免无限空转。

---

## 4. 图形化配置引擎：全局主控 Dashboard (Web UI)

抛弃让用户手敲繁复 JSON 规则的极客约束，但在后端坚持无臃肿窗体框架（Zero Heavy GUI framework）。
既然为 `redirect` 引入了 Web UI，这就顺理成章地成为了整个 **`xun` 宇宙的全局管理面板 (Dashboard)**。不仅能配置分类规则，还能统管其他模块。

### 4.1 统一入口：`xun serve`

- **微型内嵌服务**：内部在 `127.0.0.1:<port>` 拉起一个极轻量 HTTP 路由微服务（默认 `9527`），并在编译时通过 `rust-embed` 直接打包前端 HTML/JS SPA。
- **Dashboard 核心板块**：
  1. **规则管理 (Redirect)**：
     - 基于浏览器的富文本交互，请求本地硬盘拉起文件目录选择器。
     - **交互式匹配向导**：下拉式组合“扩展名”、“正则表达式”、“大小/时间”等条件。
     - **拖拽排序 (Drag & Drop)**：直观调整规则的上下权重优先级。
  2. **概览 (Home)**：
     - 聚合展示书签计数、端口概览、代理健康与最近审计。
  3. **端口 (Ports)**：
     - 列表视图 + PID 分组，支持过滤与 Kill。
  4. **网络代理 (Proxy)**：
     - 可视化添加、删除全局代理 URL。
     - 一键测速/连通性探活，支持 MSYS2 开关。
  5. **书签总览 (Bookmarks)**：
     - 搜索/排序/批量操作/内联编辑/导出，快速管理目录书签与 Tags。
  6. **配置 (Config)**：
     - 通过 Web UI 读写 `GlobalConfig`（P2 作为配置中心的一部分）。
  7. **审计与统计 (Audit & Stats)**：
     - 表格化审计与快速筛选，失败高亮与详情弹窗。
- **热更新与守护交互**：点击保存后 API 同步覆写配置，并对后台常驻的 `--watch` 守护进程下发热重载信号（Config Reload）。

### 4.2 规范化约束（MUST / SHOULD / MAY）

- **绑定范围（必须）**：`xun serve` 必须仅绑定 `127.0.0.1`，不开放公网访问。
- **写入安全（应当）**：配置写入应当采用原子写策略（临时文件 + 原子替换），避免中途损坏。

---

## 5. 匹配器

遵循 YAGNI：

| 匹配器      | 字段    | 示例                 | 阶段 |
| ----------- | ------- | -------------------- | ---- |
| 扩展名      | `ext`   | `["jpg", "png"]`     | P0   |
| 文件名 glob | `glob`  | `"report_*"`         | P0   |
| 正则        | `regex` | `"^\\d{4}-\\d{2}"`   | P1   |
| 文件大小    | `size`  | `">100MB"`, `"<1KB"` | P1   |
| 修改时间    | `age`   | `"<7d"`, `">30d"`    | P2   |

多个匹配条件同时出现时为 AND 关系。

---

## 6. 冲突处理

| 策略          | 说明                                    |
| ------------- | --------------------------------------- |
| `rename_new`  | 目标已存在时重命名（加序号） — **默认** |
| `rename_date` | 目标已存在时追加实时系统时间戳后缀      |
| `hash_dedup`  | 比对 SHA256，相同则剔除，不同则重命名   |
| `skip`        | 跳过已存在的文件                        |
| `overwrite`   | 覆盖（危险，非交互需 `--yes`）          |

**碰撞策略深度防御**

- `rename_new`：采用 `(n)` 递增（`a (1).txt`）。在长期 `--watch` 模式下，同名文件堆积会导致 O(N) 探测风暴，不建议大体量使用。
- `rename_date` / `hash_dedup`：解决极度重灾区的堆叠问题（如长年自动保存的 `Export.csv`），极大优化 IO 探测速度。

跨盘移动策略：
- 文件：跨盘必须使用 `copy + delete`（或 `MoveFileEx + MOVEFILE_COPY_ALLOWED`）。若拷贝成功但删除失败，视为成功但保留源文件并写入审计。
- 目录：跨盘不支持原子移动；P0/P1 阶段默认拒绝并提示。若未来支持，应采用递归 `copy + delete` 并保留失败清单。

权限要求：
- 移动/重命名必须具备源文件“删除权限”或父目录“删除子项权限”；权限不足应按 `skip` 或进入重试队列并写入审计原因。

### 6.1 规范化约束（MUST / SHOULD / MAY）

- **`overwrite` 风险隔离（必须）**：在非交互场景未提供 `--yes` 时必须拒绝执行。
- **`hash_dedup` 一致性（应当）**：哈希算法应当固定为 SHA-256；哈希失败时应当退回 `rename_new` 并记录原因。
- **失败回滚（应当）**：`copy + delete` 过程中，只有在拷贝成功后才允许删除源文件。

---

## 7. 长期静默守护防御机制 (Watch Defenders)

当 `redirect` 结合 `--watch` 成为 Windows 的全天候后台保安时，必须引入以下防御机制，防止其沦为破坏系统性能的捣蛋鬼。

### 7.1 空目录清道夫 (Empty Directory Sweeper)

- **痛点**：由解压或下载衍生的嵌套文件夹，内部文件被分类抽离后留下的幽灵空壳。
- **机制**：在每次成功移走文件后，沿路向上探测其父级目录，若绝对为空且非根/保护目录，则顺手抹除。保持 Source 源头如新。

### 7.2 I/O 阵风消除 (Debouncing & Burst Control)

- **痛点**：解压数万碎文件的源码包时引发的系统级文件事件洪流，会导致正则匹配与锁定鉴权 CPU 暴行。
- **机制**：引入“滑移缓冲窗”（如 800ms）。捕获的文件变动先推入 MPSC 缓存 Channel，等该文件事件彻底平息后，再将队列打包交付给分类和锁定验证引擎，削峰填谷。

### 7.3 未知黑洞归档 (Catch-all / Others)

- **痛点**：永远不符合 Match 的孤儿冷门格式（如 `.iso` 或杂碎配置）长期残留在 `Downloads`。
- **机制**：通过 `unmatched` 动作配置，赋予基于时间的兜底操作兜底（如：在 Source 躺平超过 30 天的未知废料，一律封存进入 `Others` 目录）。

### 7.4 缓冲区溢出回收 (Overflow Recovery)

- **痛点**：高频写入导致监听缓冲溢出，丢失事件。
- **机制**：触发溢出时强制执行一次“全量扫描 + 规则匹配”，确保最终一致。

### 7.5 规范化约束（MUST / SHOULD / MAY）

- **目录清理边界（必须）**：空目录清道夫不得删除 `source` 根目录、规则 `dest` 目录与受保护目录。
- **防抖默认值（应当）**：默认防抖窗口建议为 `800ms`，应当可通过配置调整。
- **忽略集一致性（应当）**：`.xunignore`、`protect` 与 `dest` 目录应当统一纳入监听忽略集。

---

## 8. 审计与撤销

### 8.1 transaction_id

同一次 manual redirect（或静默 watch 分发批次）将共享一个唯一的 `tx` ID，写入审计日志（路径与 `~/.xun.json` 同级的 `audit.jsonl`，由 `security/audit.rs` 统一管理）的 `params` 字段：

```jsonl
{
  "timestamp": 1740061800,
  "action": "redirect_move",
  "target": "D:/Downloads/photo.jpg",
  "user": "cli",
  "params": "tx=redirect_1740062 dst=D:/Downloads/Images/photo.jpg copy=false",
  "result": "success",
  "reason": ""
}
```

### 8.2 撤销（P1）

通过检索日志的 `tx` 标识，可用 `xun redirect --undo <tx>` 将文件全量按原位反向转移回来。

### 8.3 规范化约束（MUST / SHOULD / MAY）

- **审计完整性（必须）**：每次 `move/copy/skip` 必须写入审计日志；失败也必须记录 `result=failed` 与原因。
- **撤销语义（必须）**：当 `copy=true` 时，`--undo` 必须仅删除目标副本，不得影响源文件。
- **撤销安全（应当）**：若原路径已存在文件，`--undo` 应当遵循冲突策略且在非交互场景需要 `--yes` 才可覆盖。

---

## 9. 与现有体系防越界集成

| 核心组件    | 联动表现                                                                                        |
| ----------- | ----------------------------------------------------------------------------------------------- |
| **protect** | **规则拦截**：受保护文件绝不允许被整理扫走，触发时自动置为 `skip` 状态并输出警告。              |
| **lock**    | **安全防护**：`--watch` 伴生探测，遇锁定和暂未归档完成文件不动作，排队重试。                    |
| **audit**   | **数据埋点**：静默期间的一切操作强制落盘留痕，彻底消除盲区。                                    |
| **UI**      | **跨卷搬运**：当触发大体积长周期的跨卷拷贝/移动时提供进度显示（与现有输出规范一致）。          |

_(注意：不提供独立 `--unlock`，整理工具属非破坏性应用，遇占用按跳过或重试处理。)_

---

## 10. Windows API 对齐参考

> 本节基于 [Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/winbase/) 官方文档，明确 redirect 各阶段应使用的 Win32 API 及其关键约束。

### 10.1 文件移动：MoveFileEx / MoveFileWithProgress

**推荐 API**：`MoveFileWithProgress`（等同 `MoveFileEx` + 进度回调）。

| Flag | 值 | redirect 用途 |
| --- | --- | --- |
| `MOVEFILE_COPY_ALLOWED` | 0x2 | **必须**：跨卷移动时自动 copy+delete。若拷贝成功但删除失败，API 返回成功但保留源文件 |
| `MOVEFILE_REPLACE_EXISTING` | 0x1 | 仅 `overwrite` 策略时使用；替换目标文件（不能替换目录） |
| `MOVEFILE_WRITE_THROUGH` | 0x8 | **应当**：跨卷移动时确保数据刷盘后才返回，防止断电丢数据 |
| `MOVEFILE_DELAY_UNTIL_REBOOT` | 0x4 | 不使用（redirect 不涉及重启延迟操作） |

**关键约束**：
- 跨卷移动**目录**不支持（`MoveFileEx` 仅支持同卷目录移动）。P0/P1 阶段拒绝跨卷目录操作。
- 删除/重命名需要源文件的"删除权限"或父目录的"删除子项权限"。权限不足应 `skip` + 审计。
- 长路径（>MAX_PATH）：Windows 10 1607+ 支持 `\\?\` 前缀扩展到 32767 字符。redirect 应对 `source`/`dest` 做长路径检测并自动添加前缀。

**P3 进度回调**：`MoveFileWithProgress` 的 `LPPROGRESS_ROUTINE` 回调参数：
- `TotalFileSize` / `TotalBytesTransferred`：大文件跨卷拷贝进度
- `PROGRESS_CANCEL` 返回值：支持用户取消
- 注意：>4GB 文件的 `TotalBytesTransferred` 在某些旧版本 Windows 上存在溢出 bug，应使用 64 位值

### 10.2 文件复制：CopyFileEx / CopyFile2

**推荐 API**：`CopyFileEx`（P0）；`CopyFile2`（P3 可选升级，Win8+ 专用，更现代的参数结构）。

| Flag | 值 | redirect 用途 |
| --- | --- | --- |
| `COPY_FILE_FAIL_IF_EXISTS` | 0x1 | `skip` / `rename_new` 策略时使用，避免意外覆盖 |
| `COPY_FILE_NO_BUFFERING` | 0x1000 | **应当**：大文件（>数MB）跨卷拷贝时使用，减少文件缓存污染 |
| `COPY_FILE_RESTARTABLE` | 0x2 | P3 可选：断点续传（在目标文件中嵌入重启标记） |

**关键约束**：
- `COPY_FILE_NO_BUFFERING` 要求源文件大小为扇区对齐；不对齐时 API 自动回退到缓冲模式。
- 跨卷 `copy + delete` 模式下，**必须**先验证 copy 成功再执行 delete（设计文档 §6 已约定）。

### 10.3 目录监听：ReadDirectoryChangesW / ReadDirectoryChangesExW

**P0 不涉及**。P1 `--watch` 核心 API。

| API | 最低版本 | 优势 |
| --- | --- | --- |
| `ReadDirectoryChangesW` | XP | 广泛兼容 |
| `ReadDirectoryChangesExW` | Win10 1709 | 返回 `FILE_NOTIFY_EXTENDED_INFORMATION`（含文件 ID、时间戳、大小），减少额外 stat 调用 |

**推荐**：优先使用 `ReadDirectoryChangesExW`（`FILE_NOTIFY_INFORMATION_EXTENDED` 类），运行时检测可用性，不可用时 fallback 到 `ReadDirectoryChangesW`。

**关键约束（已在 §3.2 约定，此处补充 API 层面细节）**：
- OVERLAPPED 模式下 `lpBytesReturned` 参数**无意义**（MSDN 原文："not meaningful"），必须以完成端口/回调返回的字节数为准。
- `FILE_NOTIFY_INFORMATION` 结构体 `NextEntryOffset` 为 DWORD 对齐；`FileName` 为非 null-terminated 的 UTF-16，长度由 `FileNameLength`（字节数）决定。
- 网络共享（SMB）：缓冲区 >64KB 时行为未定义，**必须**限制 `nBufferLength ≤ 65536`。
- `ERROR_NOTIFY_ENUM_DIR`（0x03FE）：内核缓冲区溢出，**必须**触发全量补账扫描。

### 10.4 权限与安全

- 文件删除/重命名需要 `DELETE` 权限或父目录 `FILE_DELETE_CHILD` 权限。
- `SeDebugPrivilege`：项目已有 `handle_query.rs` 中的提权逻辑，`--watch` 的占用检测可直接复用。
- ACL 继承：跨卷移动时，`MoveFileEx` **不会**迁移安全描述符，目标文件继承目标目录的默认 ACL。这是预期行为，无需额外处理。

---

## 11. 同类工具竞品分析

> 参考 [organize-tool](https://github.com/tfeldmann/organize)（Python）、[Hazel](https://www.noodlesoft.com/)（macOS）、[File Juggler](https://www.filejuggler.com/)（Windows）的设计模式，提炼可借鉴的架构决策。

### 11.1 organize-tool（Python，开源）

**规则引擎架构**：YAML 配置，`locations → filters → actions` 三段式。

```yaml
rules:
  - locations: ~/Downloads
    subfolders: true
    filters:
      - extension: [jpg, png, gif]
      - size: ">10MB"
    actions:
      - move: ~/Pictures/{extension.upper()}/
```

**可借鉴点**：

| 特性 | organize-tool 做法 | xun redirect 采纳建议 |
| --- | --- | --- |
| **冲突策略** | `on_conflict: skip/overwrite/rename_new/rename_existing/trash` 五选一 | 已覆盖 skip/overwrite/rename_new；`rename_existing`（重命名旧文件而非新文件）值得 P3 考虑；`trash` 可映射为"移入回收站" |
| **重命名模板** | `{name}-{counter}{extension}`，`counter_separator` 可配置 | 当前 `rename_new` 硬编码 `(n)` 格式。P3 可引入模板系统，但 P0 保持 KISS |
| **dry-run** | `sim` 模式，输出与真实执行完全一致 | 已对齐（§1.1） |
| **子目录递归** | `subfolders: true/false`，`max_depth` 可配置 | P0 仅顶层文件。P1/P2 可引入 `recursive: true` + `max_depth` |
| **模板变量** | `{name}`、`{extension}`、`{created.year}` 等 | P3 可选。当前 `dest` 为静态路径，足够 P0-P1 |

**不采纳的设计**：
- organize-tool 的 `actions` 支持链式组合（move + rename + echo），过于复杂。xun redirect 保持"一条规则 = 一个目标目录"的简单模型。

### 11.2 Hazel（macOS，商业软件）

**规则引擎架构**：条件（Conditions）+ 动作（Actions）模型，per-folder 绑定。

**条件类型**：Name / Extension / Date Last Modified / Date Last Opened / Date Added / Kind / Size / Tags / Source URL / Contents（全文搜索）/ Subfolder depth

**动作类型**：Move / Copy / Rename / Sort into subfolder / Trash / Archive / Add tags / Remove tags / Set color label / Run shell script / Open / Display notification / Run rules on folder contents（递归下降）

**可借鉴点**：

| 特性 | Hazel 做法 | xun redirect 采纳建议 |
| --- | --- | --- |
| **Sort into subfolder** | 按日期/类型自动创建子目录（如 `2026/02/`） | P3 `dest` 模板变量可实现（`./Images/{created.year}/`） |
| **递归规则** | "Run rules on folder contents" 对子目录递归应用规则 | P1 `--watch` 的 `FILE_NOTIFY_CHANGE_DIR_NAME` 可触发子目录扫描 |
| **通知** | 分类完成后桌面通知 | P2 Dashboard 可通过 WebSocket 推送实时通知；CLI 模式下 `--quiet` 控制 |
| **Tags** | macOS 原生文件标签 | Windows 无原生标签系统，不采纳 |
| **Source URL** | 追踪文件下载来源 | Windows 的 Zone.Identifier ADS 可读取下载 URL，P3 可选 |

### 11.3 File Juggler（Windows，商业软件）

**规则引擎架构**：Monitor → If（条件）→ Then（动作）三段式，原生 Windows 集成。

**核心特性**：
- **文件夹监控**：基于 `ReadDirectoryChangesW`，支持多目录同时监听
- **条件**：File name / Extension / Date created / Date modified / Size / File content（PDF 文本提取）/ Subfolder name
- **动作**：Move / Copy / Rename / Delete / Extract archive / Open with / Run program
- **变量系统**：`%filename%`、`%ext%`、`%date%`、`%counter%` 等
- **空目录清理**：内置选项，移走文件后自动删除空父目录

**可借鉴点**：

| 特性 | File Juggler 做法 | xun redirect 采纳建议 |
| --- | --- | --- |
| **多目录监听** | 一个规则可绑定多个 source 目录 | P0 单目录；P2 配置中可扩展 `sources: [...]` 数组 |
| **PDF 内容匹配** | 提取 PDF 文本做正则匹配 | 不采纳（超出轻量工具定位） |
| **空目录清理** | 可选开关 | 已在 §7.1 设计，P1 实现 |
| **稳定性检测** | 文件大小稳定后才处理（防止下载中文件被移动） | 已在 §3.1 通过占用检测实现；可补充"文件大小稳定窗口"作为辅助判断 |
| **计数器变量** | `%counter%` 自动递增 | `rename_new` 的 `(n)` 序号已覆盖此需求 |

### 11.4 综合决策矩阵

| 决策点 | xun redirect 选择 | 理由 |
| --- | --- | --- |
| 配置格式 | JSON（嵌入 `~/.xun.config.json`） | 与现有配置体系统一；YAML 需额外依赖 |
| 规则模型 | `match → dest`（一对一） | KISS；不做 action 链 |
| 冲突策略 | 5 种（rename_new/rename_date/hash_dedup/skip/overwrite） | 覆盖主流场景；`trash` 和 `rename_existing` 留作 P3 |
| 监听引擎 | `ReadDirectoryChangesExW` + fallback `ReadDirectoryChangesW` | 原生零依赖；Extended 版减少 stat 调用 |
| 文件移动 API | `MoveFileWithProgress`（`MOVEFILE_COPY_ALLOWED \| MOVEFILE_WRITE_THROUGH`） | 跨卷自动 copy+delete + 刷盘保证 + 进度回调 |
| 文件复制 API | `CopyFileEx`（`COPY_FILE_FAIL_IF_EXISTS`） | 防意外覆盖；大文件可加 `NO_BUFFERING` |
| 子目录递归 | P0 不递归，P1 可选 | YAGNI；Downloads 场景顶层文件为主 |
| 模板变量 | P3 可选 | P0 静态 `dest` 路径足够 |
| 文件稳定性检测 | 占用检测 + 文件大小稳定窗口（P1） | 双重保险：handle_query 检测进程占用 + 连续两次 stat 大小不变 |

---

## 12. 现有架构对齐与可复用模块映射

> 本节基于 v0.3.0 源码审计补充，明确 redirect 实现时可直接复用的现有模块和需要扩展的接缝。

### 12.1 Feature Gate 策略

redirect 应新增独立 feature `redirect`，与现有 `lock`/`protect`/`crypt`/`dashboard` 并列：

```toml
# Cargo.toml
[features]
redirect = []  # P0 无额外依赖；P1 watch 复用已有 windows-sys features
```

- `cli.rs` 中 `SubCommand::Redirect` 使用 `#[cfg(feature = "redirect")]` 守卫。
- `commands/mod.rs` 中 `dispatch` 分支同理。
- `main.rs` 中 `mod redirect` 条件编译。
- P1 的 `--watch` 依赖 `windows-sys` 的 `Win32_Storage_FileSystem`（已在 `Cargo.toml` 中启用）。

### 12.2 配置扩展

当前 `config.rs` 的 `GlobalConfig` 结构体：

```rust
pub(crate) struct GlobalConfig {
    pub(crate) tree: TreeConfig,
    pub(crate) proxy: ProxyConfig,
    #[cfg(feature = "protect")]
    pub(crate) protect: ProtectConfig,
}
```

需要新增：

```rust
#[cfg(feature = "redirect")]
#[serde(default)]
pub(crate) redirect: RedirectConfig,
```

其中 `RedirectConfig` 包含 `profiles: HashMap<String, RedirectProfile>`。

注意事项：
- `save_config` 当前仅在 `#[cfg(feature = "protect")]` 下可用。redirect 的 Dashboard 配置写入（P2）需要将 `save_config` 的 feature gate 扩展为 `#[cfg(any(feature = "protect", feature = "redirect"))]`，或直接去掉 feature gate。
- 配置写入应复用 `store.rs` 的原子写模式（`tmp + rename`），当前 `save_config_to_path` 使用的是直接 `fs::write`，建议统一改为原子写。

### 12.3 可复用模块清单

| 现有模块 | 路径 | redirect 复用点 |
| --- | --- | --- |
| **glob_match** | `util.rs:101` | `glob` 匹配器直接复用，语义与 `.xunignore` 一致 |
| **matches_patterns** | `util.rs:130` | 目录/文件 glob 模式匹配 |
| **read_ignore_file** | `util.rs:76` | 读取 `.xunignore` 构建忽略集 |
| **normalize_glob_path** | `util.rs:63` | 路径归一化（小写 + 正斜杠） |
| **handle_query::get_locking_processes** | `windows/handle_query.rs:375` | 占用检测（handle_query 优先，fallback Restart Manager） |
| **restart_manager::LockerInfo** | `windows/restart_manager.rs:15` | 占用进程信息结构体 |
| **safety::ensure_safe_target** | `windows/safety.rs:12` | 系统目录黑名单保护 |
| **protect::is_protected** | `protect.rs:5` | 保护规则检查 |
| **protect::check_protection** | `protect.rs:24` | 保护规则校验（含 force/reason 旁路） |
| **audit::audit_log** | `security/audit.rs:8` | 审计日志写入（字段：action/target/user/params/result/reason） |
| **output::can_interact** | `output.rs:12` | 交互模式判断 |
| **output::ui_println** | `output.rs:50` | stderr 输出（提示/进度/错误） |
| **runtime::is_non_interactive** | `runtime.rs:38` | 非交互模式判断 |
| **model::ListFormat / parse_list_format** | `model.rs:27` | `--format` 参数解析（auto/table/tsv/json） |
| **store::save_db** | `store.rs:56` | 原子写模式参考（tmp + rename） |

### 12.4 审计日志路径修正

设计文档 §8.1 中写的路径 `~/.xun/audit.jsonl` 不准确。实际审计日志路径由 `security/audit.rs:88` 的 `audit_file_path_from_db_path` 决定：

```rust
fn audit_file_path_from_db_path(db_path: &Path) -> PathBuf {
    let mut p = db_path.to_path_buf();
    p.set_file_name("audit.jsonl");
    p
}
```

即与 `~/.xun.json`（书签数据库）同级，实际路径为 `~/.xun/audit.jsonl` 或 `%USERPROFILE%/audit.jsonl`（取决于 `XUN_DB` 环境变量）。redirect 审计直接调用 `audit::audit_log` 即可，无需额外路径处理。

### 12.5 Dashboard 扩展点

**后端**（`src/commands/dashboard/`）：
- `mod.rs`：axum Router + rust-embed SPA + `127.0.0.1` 绑定（已满足 §4.2 约束）
- `handlers.rs`：bookmarks/ports/proxy/config API

redirect 的 Dashboard API（P2）只需在 `build_router()` 中追加路由：

```rust
.route("/api/redirect/profiles", get(handlers::list_redirect_profiles))
.route("/api/redirect/profiles/{name}", post(handlers::upsert_redirect_profile))
.route("/api/redirect/profiles/{name}", delete(handlers::delete_redirect_profile))
```

**前端**（`dashboard-ui/`）技术栈：
- Vue 3.5 + TypeScript + Vite 7
- 依赖包含 `primevue` / `@primeuix/themes` / `@tabler/icons-vue`
- 设计系统：CSS 变量（Geist 风格），支持 dark/light 主题切换（View Transition API）
- 开发代理：Vite `server.proxy` → `/api` → `localhost:9527`
- 构建产物：`dist/`（由后端 `rust-embed` 嵌入）

**现有前端结构**：

| 文件 | 职责 |
| --- | --- |
| `src/App.vue` | 根组件，`CapsuleTabs` 切换 Home/Bookmarks/Ports/Proxy/Config/Redirect/Audit |
| `src/api.ts` | REST API 封装层（fetch-based） |
| `src/types.ts` | TypeScript 类型定义（Bookmark/PortInfo/ProxyItem/Redirect/Audit） |
| `src/components/CapsuleTabs.vue` | 胶囊标签页组件（滑动指示器 + 键盘导航） |
| `src/components/HomePanel.vue` | Dashboard 概览 |
| `src/components/BookmarksPanel.vue` | 书签 CRUD + 批量操作 + 导出 |
| `src/components/PortsPanel.vue` | 端口列表 + 过滤/分组 + kill |
| `src/components/ProxyPanel.vue` | 代理状态 + 配置/探测 |
| `src/components/ConfigPanel.vue` | 全局配置编辑 |
| `src/components/RedirectPanel.vue` | Redirect 规则 UI |
| `src/components/AuditPanel.vue` | 审计列表 + 详情 |
| `src/components/GlobalFeedback.vue` | Toast/Loading 全局反馈 |
| `src/components/CommandPalette.vue` | Command Palette（Ctrl/Cmd+K） |
| `src/components/DensityToggle.vue` | 表格密度切换 |
| `src/components/ThemeToggle.vue` | 主题切换 |
| `src/components/SkeletonTable.vue` | 表格骨架 |
| `src/components/button/` | 自定义 Button 组件（primary/secondary/danger preset） |
| `src/ui/feedback.ts` | Toast + Loading 状态 |
| `src/ui/tags.ts` | 标签分类与胶囊样式辅助 |
| `src/ui/export.ts` | CSV/JSON 导出工具 |
| `src/styles/variable.css` | CSS 变量（间距/圆角/阴影/字体） |

**redirect 前端扩展清单**（P2）：

1. `src/types.ts`：新增 `RedirectProfile`、`RedirectRule`、`MatchCondition` 类型
2. `src/api.ts`：新增 `fetchRedirectProfiles`、`upsertRedirectProfile`、`deleteRedirectProfile`
3. `src/components/RedirectPanel.vue`：规则列表 + CRUD + 拖拽排序
4. `src/App.vue`：`CapsuleTabs` 的 `tabItems` 追加 `{ value: 'redirect', label: 'Redirect' }`
5. 复用现有 `Button`、`CapsuleTabs` 组件，遵循 CSS 变量设计系统

### 12.6 CLI 参数规范对齐

现有 CLI 参数模式（`argh` 框架）：
- `--format` / `-f`：所有列表命令统一使用 `auto|table|tsv|json`
- `--yes` / `-y`：跳过确认（`dedup`、`import`、`bak`、`rm`、`mv` 等均使用）
- `--dry-run`：`bak`、`rm` 等已使用
- `--verbose` / `-v`、`--quiet` / `-q`：全局开关，通过 `runtime.rs` 管理

redirect 的 CLI 参数应完全遵循此模式，无需发明新约定。

### 12.7 测试策略

现有测试基础设施（`tests/common/mod.rs`）：
- `TestEnv`：隔离的临时目录 + 独立 DB/CONFIG 环境变量
- `run_ok` / `run_err` / `run_raw`：命令执行断言
- `HeavyTestGuard`：大规模文件系统性能测试
- `start_lock_holder`：文件占用模拟

redirect 测试应新建 `tests/test_redirect_e2e.rs`，复用 `TestEnv` 框架。

---

## 13. 渐进式实现路线 (Roadmap)

| 阶段   | 核心任务     | 描述                                                                                                                                    |
| ------ | ------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| **P0** | **基础流派** | feature gate + 模块骨架、`config` 扩展（含 `redirect.profiles`）、CLI 入口、`ext`/`glob` 匹配、同盘 Move、`rename_new` 冲突处理、`protect/audit` 接入。 |
| **P1** | **长臂管辖** | `ReadDirectoryChangesW` 原生 `--watch`、I/O 防抖池、锁占用检测（复用 `handle_query`）与重试队列、空目录清理、溢出回收。                   |
| **P2** | **视觉升维** | Dashboard 追加 Redirect 规则 CRUD API + 前端 UI、配置热装载信号。                                                                        |
| **P3** | **完备兜底** | `rename_date` / `hash_dedup` 去重、未知黑洞归档、大文件跨盘进度条展示、`--undo` 历史反向追溯。                                           |
