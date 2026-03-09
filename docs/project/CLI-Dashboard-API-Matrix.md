# XunYu CLI ↔ Dashboard ↔ API 对照矩阵

## 1. 目的

这份文档用于回答 4 个问题：

- 某个能力在 Dashboard 里属于哪个工作台
- 对应的 CLI 命令或命令族是什么
- 前端实际调用的是哪类 API
- 是否受 Cargo feature 控制

核心原则只有一条：**Dashboard 以工作台为中心，不做“每个命令一张页面”的命令墙。**

---

## 2. 共享协议

### 2.1 共享读接口

| 用途 | 前端调用 | 后端接口 |
| --- | --- | --- |
| 工作台能力探测 | `fetchWorkspaceCapabilities()` | `GET /api/workspaces/capabilities` |
| 总览摘要 | `fetchWorkspaceOverviewSummary()` | `GET /api/workspaces/overview/summary` |
| 诊断摘要 | `fetchDiagnosticsSummary()` | `GET /api/workspaces/diagnostics/summary` |
| 最近任务 | `fetchRecentTasks()` | `GET /api/workspaces/tasks/recent` |
| Recipe 列表 | `fetchWorkspaceRecipes()` | `GET /api/workspaces/recipes` |

### 2.2 共享任务接口

| 任务类型 | 前端调用 | 后端接口 | 说明 |
| --- | --- | --- | --- |
| 普通任务 | `runWorkspaceTask()` | `POST /api/workspaces/run` | 直接执行 |
| 危险任务预演 | `previewGuardedTask()` | `POST /api/workspaces/guarded/preview` | Dry-run / preview 阶段 |
| 危险任务确认执行 | `executeGuardedTask()` | `POST /api/workspaces/guarded/execute` | 必须带确认 token |
| Recipe 预演 | `previewWorkspaceRecipe()` | `POST /api/workspaces/recipes/preview` | 顺序工作流预演 |
| Recipe 执行 | `executeWorkspaceRecipe()` | `POST /api/workspaces/recipes/execute` | 顺序工作流执行 |

### 2.3 Triple-Guard 规则

凡是删除、覆盖、改 ACL、加解密、保护开关等危险动作，都必须遵守：

1. `Preview / Dry-run`
2. `User Confirm`
3. `Receipt + Audit`

在 Dashboard 中，这类动作统一通过 `guarded` 任务模型进入 `preview` / `execute` 两段式 API。

---

## 3. 工作台矩阵

### 3.1 总览（Overview）

| Dashboard 区域 | CLI / 后端来源 | API |
| --- | --- | --- |
| 概览统计卡片 | 聚合多个子系统状态 | `GET /api/workspaces/overview/summary` |
| 跳转到诊断 / 最近任务 | 工作台联动，不直接对应单一 CLI 命令 | `GET /api/workspaces/diagnostics/summary` / `GET /api/workspaces/tasks/recent` |

### 3.2 路径与上下文（Paths & Context）

| Dashboard 能力 | CLI 映射 | API 类型 | Feature |
| --- | --- | --- | --- |
| BookmarksPanel | `list / open / save / set / del / tag` | 专用读写接口 + 工作台联动 | 默认 |
| 上下文治理 | `ctx list/show/use/off/set/del/rename` | `run` | 默认 |
| 最近访问 | `recent` | `run` | 默认 |
| 统计 / 体检 | `stats / check / gc / dedup` | `run` / `guarded` | 默认 |
| 查询与聚合 | `keys / all / fuzzy` | `run` | 默认 |
| 工作区批量打开 | `ws` | `run` | 默认 |
| 本域 Recipe | `paths-context-health` | `recipes` | 默认 |

### 3.3 网络与代理（Network & Proxy）

| Dashboard 能力 | CLI 映射 | API 类型 | Feature |
| --- | --- | --- | --- |
| 代理状态与检测 | `pst / proxy detect / proxy test` | 专用接口 + `run` | 默认 |
| 代理应用与移除 | `pon / poff / proxy set / proxy del` | 专用接口 / `guarded` | 默认 |
| 端口与进程 | `ports / kill / ps / pkill` | 专用接口 / `guarded` | 默认 |
| 本域 Recipe | `proxy-diagnostics` | `recipes` | 默认 |

### 3.4 环境与配置（Environment & Config）

| Dashboard 能力 | CLI / 后端来源 | API 类型 | Feature |
| --- | --- | --- | --- |
| Env 状态、变量、快照、doctor | `env` 命令族与 env handlers | 专用接口 | 默认 |
| 配置读写 / diff / validate / convert | `config` / `diff` | 专用接口 | `diff` 为可选 |

### 3.5 文件与安全（Files & Security）

| Dashboard 能力 | CLI 映射 | API 类型 | Feature |
| --- | --- | --- | --- |
| 树 / 搜索 / 备份 / 删除 | `tree / find / bak / rm` | `run` / `guarded` | 默认 |
| 文件迁移 / 改名 | `mv / renfile` | `guarded` | 默认 |
| ACL | `acl view/add/remove/purge/diff/...` | `run` / `guarded` | 默认 |
| 锁定 / 保护 | `lock / protect` | `guarded` | `lock` / `protect` |
| 加解密 | `encrypt / decrypt` | `guarded` | `crypt` |
| Redirect / Diff | `redirect / diff` | 专用接口 / `guarded` | `redirect` / `diff` |

### 3.6 集成与自动化（Integration & Automation）

| Dashboard 能力 | CLI 映射 | API 类型 | Feature |
| --- | --- | --- | --- |
| Shell 初始化 | `init` | `run` | 默认 |
| 补全脚本 | `completion / __complete` | `run` | 默认 |
| Shell 安装向导 | `init + completion + __complete` 的 UI 闭环 | 工作台内联动 | 默认 |
| alias 基础治理 | `alias ls/find/which/sync` | `run` | `alias` |
| alias 扩展治理 | `alias setup/add/export/import` | `run` | `alias` |
| alias 危险删除 | `alias rm / alias app rm` | `guarded` | `alias` |
| app alias | `alias app add/ls/scan/which/sync` | `run` | `alias` |
| 批量重命名 | `brn` | `guarded` | `batch_rename` |
| 本域 Recipe | `integration-shell-bootstrap` | `recipes` | 默认 |

### 3.7 媒体与转换（Media & Conversion）

| Dashboard 能力 | CLI 映射 | API 类型 | Feature |
| --- | --- | --- | --- |
| 图像转换 | `img` | `run` | `img` |
| `img` 高级参数 | `svg_method / jpeg_backend / png_lossy / webp_lossy / threads / avif_threads` | `run` | `img` |
| 视频探测 | `video probe` | `run` | 默认 |
| 视频压缩 | `video compress` | `run` | 默认 |
| 无损封装转换 | `video remux` | `run` | 默认 |
| 本域 Recipe | `media-video-probe-compress / media-video-remux-validate / media-image-batch-convert` | `recipes` | 默认 |

### 3.8 统计与诊断（Statistics & Diagnostics）

| Dashboard 能力 | CLI / 后端来源 | API 类型 | Feature |
| --- | --- | --- | --- |
| Recent Tasks 全局消费层 | 多工作台回链 | `GET /api/workspaces/tasks/recent` | 默认 |
| Audit / Doctor / Governance | 诊断聚合 handlers | `GET /api/workspaces/diagnostics/summary` | 默认 |
| 目录统计 | `cstat` | `run` | `cstat` |
| 复盘型 Recipe | `statistics-cstat-review` | `recipes` | 默认 |

---

## 4. Cargo feature 与 Dashboard 能力关系

| Feature | 对应 Dashboard 能力 |
| --- | --- |
| `alias` | alias / app alias / shell alias 治理 |
| `batch_rename` | `brn` |
| `img` | `img` 图像转换与高级参数 |
| `lock` | 文件锁定相关任务 |
| `protect` | 文件保护相关任务 |
| `crypt` | `encrypt / decrypt` |
| `redirect` | Redirect 配置与治理 |
| `diff` | 文件 / 配置 diff 与可视化 |
| `cstat` | 目录统计与复盘 |
| `dashboard` | 本地 Dashboard 服务与前后端桥接 |

前端会先读取 `GET /api/workspaces/capabilities`，再决定是否展示或启用对应任务卡。

---

## 5. 校验建议

每次新增 Dashboard 能力时，建议同时核对 4 个落点：

1. `src/cli/*`：CLI 参数口径是否已经稳定
2. `dashboard-ui/src/workspace-tools.ts`：前端任务卡是否已接入
3. `dashboard-ui/src/api.ts` / 后端 handlers：协议是否匹配
4. 测试与文档：是否补了 smoke / unit / matrix 文档

建议最少回归命令：

- `cargo test --features dashboard`
- `cd dashboard-ui && npm run test`
- `cd dashboard-ui && npm run test:smoke`
- `cd dashboard-ui && npm run build`
