# xun redirect — 分阶段任务清单

> 依据：[Redirect-Design.md](./Redirect-Design.md) v0.3.0
> 标记：`[ ]` 待办　`[-]` 进行中　`[x]` 完成
> 更新：2026-02-22 — 基于源码审计对齐现有架构

---

## Phase 0：基础 CLI 与规则引擎

### P0.0 模块骨架与 Feature Gate

- [x] `Cargo.toml`：新增 `redirect = []` feature（P0 无额外依赖）
- [x] `src/commands/redirect/mod.rs`：创建模块目录，声明子模块（`config`、`matcher`、`engine`、`mod`）
- [x] `src/commands/mod.rs`：添加 `#[cfg(feature = "redirect")] pub(crate) mod redirect;`
- [x] `src/main.rs`：无需改动（commands 已统一入口）
- [x] `src/commands/redirect/mod.rs`：导出 `pub(crate) fn cmd_redirect(args: RedirectCmd)`

### P0.1 CLI 入口与参数规范

- [x] `cli.rs`：新增 `RedirectCmd` 结构体（`argh` 派生），参数：`source`（positional, optional）、`--profile`、`--watch`、`--dry-run`、`--copy`、`-y`/`--yes`、`-f`/`--format`
- [x] `cli.rs`：`SubCommand` 枚举添加 `#[cfg(feature = "redirect")] Redirect(RedirectCmd)`
- [x] `commands/mod.rs`：`dispatch` 添加 `#[cfg(feature = "redirect")] SubCommand::Redirect(a) => redirect::cmd_redirect(a)`
- [x] 非交互判断复用 `output::can_interact()` + `runtime::is_non_interactive()`
- [x] `--format` 复用 `model::parse_list_format`（auto/table/tsv/json）
- [x] 输出规范：结果 → stdout（`out_println!`），提示/进度/错误 → stderr（`ui_println!`）

### P0.2 配置扩展与校验

- [x] `config.rs`：新增 `RedirectConfig`、`RedirectProfile`、`RedirectRule`、`MatchCondition` 结构体（均 `Serialize + Deserialize + Default + Clone`）
- [x] `config.rs`：`GlobalConfig` 添加 `#[cfg(feature = "redirect")] #[serde(default)] pub(crate) redirect: RedirectConfig`
- [x] `RedirectProfile` 字段：`rules: Vec<RedirectRule>`、`unmatched: String`（默认 `"skip"`）、`on_conflict: String`（默认 `"rename_new"`）
- [x] `RedirectRule` 字段：`name: String`、`match_cond: MatchCondition`（serde rename `"match"`）、`dest: String`
- [x] `MatchCondition` 字段：`ext: Vec<String>`、`glob: Option<String>`（P0 仅实现 ext + glob）
- [x] 校验逻辑：`--profile` 指定的 profile 不存在 → 非 0 退出 + 明确错误
- [x] 校验逻辑：`rules` 为空 → 配置错误
- [x] 校验逻辑：每条规则 `match` 至少一个条件 + `dest` 非空
- [x] `save_config` 的 feature gate 扩展为 `#[cfg(any(feature = "protect", feature = "redirect"))]`（为 P2 Dashboard 写入做准备）

### P0.3 规则匹配器

- [x] `commands/redirect/matcher.rs`：`ext` 匹配 — 提取文件扩展名，转小写，与 `ext` 列表比对
- [x] `glob` 匹配 — 复用 `util::glob_match`（语义与 `.xunignore` 一致）
- [x] 匹配入口函数：`fn match_file(file_name: &str, rules: &[RedirectRule]) -> Option<&RedirectRule>`
- [x] 规则顺序：首条命中即停止（短路）
- [x] 多条件 AND 关系（ext + glob 同时存在时均须满足）
- [x] `dest` 相对路径解析：相对于 `source` 目录

### P0.4 执行引擎（移动/复制/冲突）

- [x] `commands/redirect/engine.rs`：扫描 `source` 目录（非递归，仅顶层文件）
- [x] 路径归一化：`source`/`dest` 做 canonicalize + 大小写折叠
- [x] 系统安全检查：复用 `windows/safety::ensure_safe_target` 校验 dest 路径
- [x] 同盘移动使用 `std::fs::rename`（底层即 `MoveFileEx` 无 flag）
- [x] 跨盘文件移动使用 `MoveFileWithProgress`（`MOVEFILE_COPY_ALLOWED | MOVEFILE_WRITE_THROUGH`），通过 `windows-sys` 调用；跨卷自动 copy+delete + 刷盘保证
- [x] 跨盘移动目录：P0 拒绝并提示（`MoveFileEx` 不支持跨卷目录移动）
- [x] `--copy` 模式：使用 `CopyFileEx`（`COPY_FILE_FAIL_IF_EXISTS`），不删除源文件
- [x] 长路径支持：`source`/`dest` 超过 MAX_PATH 时自动添加 `\\?\` 前缀
- [x] 冲突策略 `rename_new`：目标存在时追加 `(n)` 序号（`a (1).txt`、`a (2).txt`）
- [x] 冲突策略 `skip`：目标存在时跳过
- [x] 冲突策略 `overwrite`：非交互场景未提供 `--yes` 时拒绝执行
- [x] `--dry-run`：零副作用，输出与真实执行结构一致
- [x] 目标目录不存在时自动创建（`fs::create_dir_all`）

### P0.5 保护与审计集成

- [x] `protect` 集成：移动/复制前调用 `protect::is_protected(rules, path, "move")`，命中则 `skip` + 警告
- [x] `audit` 集成：每次 move/copy/skip/fail 调用 `security::audit::audit_log`
- [x] `action` 字段：`redirect_move` / `redirect_copy` / `redirect_skip`
- [x] `params` 字段格式：`tx=<tx_id> dst=<dest_path> copy=<bool>`
- [x] `tx` 生成：`redirect_{timestamp}_{pid}` 格式，一次 CLI 执行共享同一 `tx`

### P0.6 端到端测试

- [x] `tests/test_redirect_e2e.rs`：复用 `TestEnv` 框架
- [x] 测试用例：基础 ext 分类（jpg → Images/）
- [x] 测试用例：glob 匹配（report_* → Reports/）
- [x] 测试用例：`--dry-run` 不产生副作用
- [x] 测试用例：`--copy` 保留源文件
- [x] 测试用例：`rename_new` 冲突重命名
- [x] 测试用例：profile 不存在 → 非 0 退出
- [x] 测试用例：rules 为空 → 配置错误
- [x] 测试用例：`--format json` 输出结构稳定

---

## Phase 1：原生 Watch 守护

### P1.1 `ReadDirectoryChangesExW` / `ReadDirectoryChangesW` 封装

- [x] `commands/redirect/watcher.rs`：优先使用 `ReadDirectoryChangesExW`（Win10 1709+，返回 `FILE_NOTIFY_EXTENDED_INFORMATION` 含文件大小/时间戳，减少额外 stat 调用）
- [x] 运行时检测 `ReadDirectoryChangesExW` 可用性（`GetProcAddress`），不可用时 fallback 到 `ReadDirectoryChangesW`
- [x] 复用已有 `windows-sys` 依赖（`Win32_Storage_FileSystem` features 已启用）
- [x] 固定完成模型（IOCP / completion routine / `GetOverlappedResult` 三选一）
- [x] OVERLAPPED 模式下 `lpBytesReturned` 无意义（MSDN 明确），必须以完成结果返回的字节数为准
- [x] 网络共享目录 `nBufferLength` 限制 ≤ 64KB（SMB 协议约束），超限拒绝启动

### P1.2 事件解析与溢出恢复

- [x] `FILE_NOTIFY_INFORMATION` 以 `NextEntryOffset` 遍历变长链表（DWORD 对齐）
- [x] `FileName` 为非 null-terminated UTF-16，长度由 `FileNameLength`（字节数）决定
- [x] 同目录重命名成对处理 `RENAMED_OLD_NAME/NEW_NAME`（保持相邻）
- [x] 跨目录重命名按 `REMOVED + ADDED` 处理
- [x] `lpBytesReturned == 0` 或 `ERROR_NOTIFY_ENUM_DIR` → 触发全量扫描补账
- [x] 补账完成前不得清空重试队列

### P1.3 占用检测与重试队列

- [x] 复用 `windows/handle_query::get_locking_processes`（handle_query 优先，fallback Restart Manager）
- [x] 文件大小稳定性检测：连续两次 stat（间隔 500ms）大小不变才视为写入完成（参考 File Juggler 设计，防止下载中文件被移动）
- [x] 新文件先检测占用 + 稳定性，被占用/权限不足/仍在写入 → 进入重试队列并记录原因
- [x] 重试队列定时轮询（可配置间隔），释放后执行转移
- [x] 重试队列与补账扫描互斥/顺序明确

### P1.4 防抖与忽略集

- [x] 事件缓存 MPSC channel + 防抖窗口（默认 800ms，可通过配置调整）
- [x] 忽略集统一纳入：`.xunignore`（复用 `util::read_ignore_file`）、`protect` 规则路径、所有 `dest` 目录
- [x] 防止 `dest` 回流触发重复分类

### P1.5 守护防御

- [x] 空目录清道夫：移走文件后沿路向上探测空目录并删除
- [x] 清道夫边界：禁止删除 `source` 根、`dest` 目录、受保护目录
- [x] 大量事件时启用批处理与限速

### P1.6 Watch 测试

- [x] 单元测试：事件解析（FILE_NOTIFY_INFORMATION 模拟）
- [x] 单元测试：防抖窗口合并逻辑
- [x] 集成测试：文件创建 → 自动分类（需 sleep 等待）
- [x] 集成测试：占用文件 → 重试队列（复用 `tests/common::start_lock_holder`）

---

## Phase 2：Dashboard 配置中心

### P2.1 Redirect 后端 API

> 现有 Dashboard 后端框架已就绪：axum + rust-embed + SPA fallback + `127.0.0.1:9527`

- [x] `commands/dashboard/handlers.rs`：新增 `list_redirect_profiles` / `upsert_redirect_profile` / `delete_redirect_profile` handler
- [x] `commands/dashboard/mod.rs`：`build_router()` 追加 `/api/redirect/profiles` 路由
- [x] 配置写入改为原子写（`tmp + rename`，参考 `store::save_db` 模式）
- [x] 保存后向 `--watch` 守护发送热重载信号（进程间通信机制待定：named pipe / shared memory / file signal）

### P2.2 Redirect 前端 UI

> 现有前端技术栈：Vue 3.5 + TypeScript + Vite 7 + PrimeVue 4 + Tabler Icons + CSS 变量设计系统（Geist 风格）

**类型与 API 层：**
- [x] `dashboard-ui/src/types.ts`：新增 `RedirectProfile`、`RedirectRule`、`MatchCondition` 接口
- [x] `dashboard-ui/src/api.ts`：新增 `fetchRedirectProfiles()`、`upsertRedirectProfile(name, profile)`、`deleteRedirectProfile(name)`

**组件：**
- [x] `dashboard-ui/src/components/RedirectPanel.vue`：规则列表主面板
- [x] 规则列表增删改查（复用现有 `Button` 组件 + `IconPlus`/`IconTrash` 图标）
- [x] 拖拽排序（优先级调整，可用 HTML5 Drag API 或引入轻量拖拽库）
- [x] 匹配条件编辑器（ext 多选输入、glob 文本框、regex/size/age 按阶段渐进）
- [x] `dest` 路径输入 + 校验提示
- [x] Profile 切换（下拉选择 / CapsuleTabs 子级）

**集成：**
- [x] `dashboard-ui/src/App.vue`：`tabItems` 追加 `{ value: 'redirect', label: 'Redirect' }`
- [x] `App.vue` template 追加 `<RedirectPanel v-if="tab === 'redirect'" />`
- [x] 遵循现有 CSS 变量体系（`--space-*`、`--radius-*`、`--text-*`、`--ds-background-*`），不引入额外 CSS 框架
- [x] 构建后 `dist/` 产物由后端 `rust-embed` 自动嵌入，无需额外配置

### P2.3 Dashboard 其它模块

> 现有 API 已覆盖：bookmarks CRUD、ports 列表/kill、proxy status、config 读写、audit 查询

- [x] Proxy 管理增强（新增/删除/连通性探活 — 当前仅有 status 读取）
- [x] Bookmarks 管理增强（排序/列开关/批量操作/内联编辑/导出）
- [x] Ports 管理增强（自动刷新/过滤/分组/导出）
- [x] Audit/Stats 展示（筛选/分页/失败高亮/详情/导出）
- [x] Home 概览面板（书签/端口/代理/审计摘要）
- [x] Config 面板（`/api/config` 读写入口）
- [x] 全局反馈系统（Toast/Loading/Skeleton）
- [x] Command Palette（Ctrl/Cmd+K 快速导航）
- [x] 密度/主题切换（compact/spacious + system/light/dark）

---

## Phase 3：高级能力与兜底

### P3.1 冲突策略扩展

- [x] `rename_date`（时间戳后缀）
- [x] `hash_dedup`（SHA-256 去重，失败回退 `rename_new` 并记录原因）
- [x] `rename_existing`（重命名旧文件而非新文件，参考 organize-tool 设计）
- [x] `trash`（移入回收站而非直接覆盖，参考 organize-tool 设计，使用 `IFileOperation`/`SHFileOperation`）

### P3.2 匹配器扩展

- [x] `regex` 匹配器（P1 阶段）
- [x] `size` 匹配器（`>100MB`、`<1KB`）
- [x] `age` 匹配器（`<7d`、`>30d`）

### P3.3 未命中归档

- [x] `unmatched` 支持时间条件归档（如 `>30d` → Others）
- [x] Others 目录创建与忽略集同步

### P3.4 大文件与跨盘体验

- [x] 跨盘移动使用 `MoveFileWithProgress` 的 `LPPROGRESS_ROUTINE` 回调获取 `TotalFileSize`/`TotalBytesTransferred`
- [x] 跨盘复制使用 `CopyFileEx` 的 `LPPROGRESS_ROUTINE` 回调
- [x] 大文件（>数MB）复制时追加 `COPY_FILE_NO_BUFFERING` flag 减少缓存污染
- [x] 进度回调桥接到 stderr 进度条展示（indicatif 进度条）
- [x] 支持 `PROGRESS_CANCEL` 返回值实现用户取消（Ctrl+C 信号映射）
- [x] 注意 >4GB 文件的 `TotalBytesTransferred` 溢出问题，使用 64 位值
- [x] 失败清单与审计记录补全

### P3.5 撤销机制（`--undo`）

- [x] 通过 `tx` 解析审计日志（`audit.jsonl`）并反向移动
- [x] `copy=true` 时撤销仅删除目标副本
- [x] 发生冲突时遵循冲突策略，非交互需 `--yes`

### P3.6 dest 模板变量（参考 organize-tool / Hazel）

- [x] `dest` 支持模板变量：`{name}`、`{ext}`、`{created.year}`、`{created.month}` 等
- [x] 模板解析引擎：简单 `{}` 占位符替换，不引入外部模板库
- [x] 典型用例：`./Images/{created.year}/{created.month}/` 按日期自动归档子目录

### P3.7 子目录递归（参考 organize-tool `subfolders` / Hazel 递归规则）

- [x] 配置项 `recursive: bool`（默认 `false`）+ `max_depth: u32`（默认 `1`）
- [x] `--watch` 模式下递归监听需设置 `bWatchSubtree = TRUE`
- [x] 递归扫描时 `.xunignore` / `protect` / `dest` 忽略集同样生效

---

## 验收检查清单

- [x] `redirect` 基础分类（ext/glob）正确
- [x] `--dry-run` 零副作用且输出一致
- [x] `--copy` 不影响源文件
- [x] `rename_new` 冲突策略正确
- [x] protect 规则拦截生效
- [x] `audit.jsonl` 字段完整、格式稳定（action/target/user/params/result/reason）
- [x] `--format json|tsv` 输出结构稳定
- [x] `--watch` 事件不丢失且溢出补账生效
- [x] 占用文件进入重试队列且可恢复
- [x] Dashboard redirect API CRUD 正常
- [x] `--undo` 正确回滚
