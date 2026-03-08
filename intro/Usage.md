# 使用约定与数据文件

## 全局约定（非常重要）

- `stdout`：机器可读内容（JSON/TSV 等）
- `stderr`：交互 UI 与表格（更适合人看）
- `XUN_UI=1`：强制表格输出（即便被管道重定向）
- 全局选项：`--no-color`（或 `NO_COLOR=1`）、`-q/--quiet`、`-v/--verbose`、`--non-interactive`
- 环境变量：`XUN_QUIET`、`XUN_VERBOSE`、`XUN_NON_INTERACTIVE`

常用格式选项（多数命令支持）：`-f <auto|table|tsv|json>`。  
说明：目前使用 `argh`，不支持共享 option group，因此 `--format` 在各子命令里重复定义；如需彻底消除重复，需要迁移到 `clap`。

Windows 路径在 PowerShell/CMD 中不需要转义反斜杠：`xun set proj D:\Repo\MyProj -t work,rust`。  
仅在 JSON/字符串字面量中需要使用双反斜杠：`"path": "D:\\Repo\\MyProj"`。

## 命令分层与默认行为

- 默认可用命令族：`acl`、书签、`config`、`ctx`、`proxy`、`ports/kill/ps/pkill`、`bak`、`tree`、`find`、`delete/del`、`rm`
- 需 feature 的命令族：`alias`、`lock`、`protect`、`crypt`、`redirect`、`serve`、`diff`、`brn`、`cstat`、`img`
- `ports` 默认展示常见开发端口（3000-3999/5000-5999/8000-8999/4173/5173）；查看全部请加 `--all`
- `ps`/`pkill` 用于进程检索与终止；支持按名称、PID 或窗口标题（`-w`）

## 数据与配置文件

| 名称 | 默认路径 | 用途 |
| --- | --- | --- |
| 书签数据库 | `%USERPROFILE%\.xun.json` | 书签数据（可用 `XUN_DB` 覆盖） |
| 访问日志（WAL） | `%USERPROFILE%\.xun.visits.jsonl` | 高频访问记录（合并回书签库以更新 frecency） |
| 全局配置 | `%USERPROFILE%\.xun.config.json` | `protect` / `redirect` 等模块配置 |
| Context 配置 | `%USERPROFILE%\.xun.ctx.json` | ctx profile（可用 `XUN_CTX_FILE` 覆盖） |
| 代理配置 | `%USERPROFILE%\.xun.proxy.json` | `proxy` / `pon` / `poff` 持久化 |
| Context 会话 | `%TEMP%\xun-ctx-<pid>.json` | ctx 会话态（由 `XUN_CTX_STATE` 指定） |
| 审计日志 | `%USERPROFILE%\audit.jsonl` | JSON Lines（用于追溯/undo 等；与 `XUN_DB` 同目录） |
| 备份配置 | `.svconfig.json` | `bak` 在工作目录自动创建 |
| 树忽略 | `.xunignore` | `redirect`/`tree` 等忽略规则（类 `.gitignore`） |

## Dashboard 基础 API（需 `--features dashboard`）

- 启动：`xun serve --port 9527`
- 监听地址：`127.0.0.1:<port>`（仅本机）
- 基础能力：书签管理、端口管理、代理配置、全局配置、审计查询

### 常用基础端点

| Method | Endpoint | 说明 |
| --- | --- | --- |
| GET | `/api/bookmarks` | 列书签 |
| POST | `/api/bookmarks/{name}` | 新增/更新书签 |
| DELETE | `/api/bookmarks/{name}` | 删除书签 |
| GET | `/api/ports` | 端口列表 |
| POST | `/api/ports/kill/{port}` | 按端口终止进程 |
| GET | `/api/proxy/status` | 代理状态 |
| GET/POST | `/api/proxy/config` | 读取/设置代理配置 |
| GET/POST/PUT | `/api/config` | 读取/patch/覆盖全局配置 |
| GET | `/api/audit` | 查询审计日志 |

## Dashboard Diff（需 `--features "dashboard,diff"`）

- 启动：`xun serve --port 9527`
- 监听地址：`127.0.0.1:<port>`（仅本机）
- 典型能力：文件树浏览、语义/行级 diff、格式转换、语法校验、WebSocket 热刷新

### 关键 API

| Method | Endpoint | 说明 |
| --- | --- | --- |
| GET | `/api/files?path=` | 列目录 |
| GET | `/api/files/search?root=&query=&limit=` | 递归搜索（并行） |
| GET | `/api/info?path=` | 文件元信息（语言/行数/大小） |
| GET | `/api/content?path=&offset=&limit=` | 分块读取文本内容 |
| POST | `/api/diff` | diff 计算（`old_path/new_path/mode/algorithm/...`） |
| POST | `/api/convert` | 配置格式转换（toml/yaml/json/json5） |
| POST | `/api/validate` | 配置语法校验（按路径或内容） |
| WS | `/ws` | 文件变化推送（`connected`/`file_changed`/`refresh`） |

### 搜索性能统计 Header（`/api/files/search`）

- `x-xun-search-total-ms`
- `x-xun-search-scan-ms`
- `x-xun-search-sort-ms`
- `x-xun-search-scanned`
- `x-xun-search-matched`
- `server-timing`（`scan/sort/total`）
