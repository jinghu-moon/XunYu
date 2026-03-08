# XunYu Web Dashboard 设计文档

> 技术栈：Axum 0.8 (后端) + Vite + Vue 3 + TypeScript (前端) + rust-embed (嵌入)
> 触发方式：`xun serve [--port 9527]`
> 隔离策略：`#[cfg(feature = "dashboard")]` feature gate

---

## 1. 目标

为 XunYu 提供本地 Web 管理面板，CLI 正式命令仍为 `xun`，通过浏览器直观管理书签、查看端口占用、查看代理状态。

**非目标：**
- 不做远程访问/认证（仅 localhost）
- 不做实时推送（轮询即可）

---

## 2. 架构概览

```
浏览器 ──HTTP──▶ Axum (localhost:9527)
                  │
                  ├── GET /                     → 内嵌 Vue 3 单页 HTML
                  ├── GET /api/bookmarks        → store::load()
                  ├── GET /api/ports            → ports::list_tcp_listeners()
                  ├── GET /api/proxy/status     → proxy env 检测
                  ├── GET /api/config           → config::load_config()
                  ├── GET /api/audit            → security::audit 读取
                  └── /api/redirect/*           → redirect 配置（feature: redirect）
```

**关键设计：**
- 前端为独立 Vite + Vue 3 + TypeScript 工程（`dashboard-ui/`）
- 构建产物 `dashboard-ui/dist/` 通过 `rust-embed` 嵌入二进制
- `cmd_serve()` 内部创建 `tokio::runtime::Builder::new_current_thread()`
- 不改动 `main()` 的同步签名，与现有 CLI 架构完全兼容

---

## 3. 依赖变更

### Cargo.toml

```toml
[features]
dashboard = ["dep:axum", "dep:tokio", "dep:rust-embed", "dep:mime_guess"]

[dependencies]
axum       = { version = "0.8", optional = true, default-features = false, features = ["json", "tokio"] }
tokio      = { version = "1", optional = true, features = ["rt", "net", "macros"] }
rust-embed = { version = "8", optional = true }
mime_guess = { version = "2", optional = true }
```

### 前端 (dashboard-ui/package.json)

```json
{
  "dependencies": {
    "@primeuix/themes": "^2.0.3",
    "@tabler/icons-vue": "^3.37.1",
    "primevue": "^4.5.4",
    "vue": "^3.5.28"
  },
  "devDependencies": {
    "@vitejs/plugin-vue": "^6.0.4",
    "typescript": "^5.9.3",
    "vite": "^7.3.1",
    "vue-tsc": "^3.2.4"
  }
}
```

**体积影响：**
- `cargo build --release` 默认编译：不受影响（optional deps 不参与）
- `cargo build --release --features dashboard`：预计增加 ~2-3MB（含嵌入前端资源）

---

## 4. 新增文件

```
dashboard-ui/                    # Vite + Vue 3 + TypeScript 前端工程
├── package.json
├── vite.config.ts               # dev proxy → localhost:9527
├── tsconfig.json
├── tsconfig.app.json
├── index.html
└── src/
    ├── main.ts
    ├── App.vue                  # Tab 导航 (Home / Bookmarks / Ports / Proxy / Config / Redirect / Audit)
    ├── api.ts                   # fetch 封装
    ├── types.ts                 # TypeScript 类型定义
    └── components/
        ├── HomePanel.vue
        ├── BookmarksPanel.vue
        ├── PortsPanel.vue
        ├── ProxyPanel.vue
        ├── ConfigPanel.vue
        ├── RedirectPanel.vue
        ├── AuditPanel.vue
        ├── GlobalFeedback.vue
        ├── CommandPalette.vue
        ├── DensityToggle.vue
        ├── ThemeToggle.vue
        └── SkeletonTable.vue

src/commands/dashboard/
├── mod.rs                       # cmd_serve() 入口 + Router + rust-embed 静态文件
└── handlers.rs                  # API handlers
```

---

## 5. 修改文件清单

| 文件 | 改动 |
|------|------|
| `Cargo.toml` | 添加 axum/tokio/rust-embed/mime_guess optional 依赖 + dashboard feature |
| `src/ports.rs` | `PortInfo`/`Protocol` 追加 `#[derive(Serialize)]` |
| `src/cli.rs` | 添加 `#[cfg(feature = "dashboard")] Serve(ServeCmd)` |
| `src/commands/mod.rs` | 添加 dashboard 模块声明 + dispatch 分支 |

---

## 6. CLI 子命令

```rust
#[cfg(feature = "dashboard")]
#[derive(FromArgs)]
#[argh(subcommand, name = "serve")]
pub struct ServeCmd {
    /// listen port (default: 9527)
    #[argh(option, short = 'p', default = "9527")]
    pub port: u16,
}
```

用法：
```powershell
xun serve              # 默认 9527 端口
xun serve -p 8080      # 自定义端口
```

---

## 7. API 设计

### 7.1 书签

| 方法 | 路径 | 请求体 | 响应 | 复用 |
|------|------|--------|------|------|
| GET | `/api/bookmarks` | — | `[{name, path, tags, visits, last_visited}]` | `store::load()` |
| GET | `/api/bookmarks/export?format=json|tsv` | — | 列表数据 | `store::load()` |
| POST | `/api/bookmarks/import?format=json|tsv&mode=merge|overwrite` | `body` | `{added, updated, total}` | `save_db()` |
| POST | `/api/bookmarks/{name}` | `{path, tags}` | 200 / 409(锁冲突) | `Lock::acquire()` + `save_db()` |
| POST | `/api/bookmarks/{name}/rename` | `{newName}` | 200 / 409 / 404 | `Lock::acquire()` + `save_db()` |
| POST | `/api/bookmarks/batch` | `{op, names, tags}` | `{deleted|updated}` | `Lock::acquire()` + `save_db()` |
| DELETE | `/api/bookmarks/{name}` | — | 200 / 404 | `Lock::acquire()` + `save_db()` |

### 7.2 端口

| 方法 | 路径 | 响应 | 复用 |
|------|------|------|------|
| GET | `/api/ports` | `{tcp: [...], udp: [...]}` | `list_tcp_listeners()` + `list_udp_endpoints()` |
| GET | `/api/ports/icon/{pid}?size=16-256` | WebP 图标 | `win_icon_extractor` |
| POST | `/api/ports/kill/{port}` | 200 / 404 | `terminate_pid()` |
| POST | `/api/ports/kill-pid/{pid}` | 200 / 403 / 404 | `terminate_pid()` |

### 7.3 代理 & 配置

| 方法 | 路径 | 响应 | 复用 |
|------|------|------|------|
| GET | `/api/proxy/status` | `[{tool, status, address}]` | `get_system_proxy_url()` + env |
| GET | `/api/proxy/config` | `{defaultUrl, noproxy}` | `config::load_config()` |
| POST | `/api/proxy/config` | `{defaultUrl, noproxy}` | 200 | `config::save_config()` |
| GET | `/api/proxy/test` | `[{label, ok, ms, error}]` | `proxy::test_proxy` |
| POST | `/api/proxy/set` | `{url, noproxy, only}` | 200 | `proxy::set` |
| POST | `/api/proxy/del` | `{only}` | 200 | `proxy::del` |
| GET | `/api/config` | `GlobalConfig` JSON | `config::load_config()` |
| POST | `/api/config` | Config patch | 200 | `config::save_config()` |
| PUT | `/api/config` | Config replace | 200 | `config::save_config()` |

### 7.4 审计

| 方法 | 路径 | 响应 | 复用 |
|------|------|------|------|
| GET | `/api/audit?limit=&search=&action=&result=&from=&to=&cursor=&format=json|csv` | `{entries, stats, next_cursor}` / CSV | `security::audit` |

### 7.5 Redirect（feature: redirect）

| 方法 | 路径 | 请求体 | 响应 |
|------|------|--------|------|
| GET | `/api/redirect/profiles` | — | `{profiles}` |
| POST | `/api/redirect/profiles/{name}` | profile | 200 |
| DELETE | `/api/redirect/profiles/{name}` | — | 200 |
| POST | `/api/redirect/dry-run` | `{profile, rules}` | diff 预演 |

### 7.4 并发安全

写操作（POST/DELETE bookmarks, kill port）使用 `store::Lock::acquire()` 文件锁，3 秒超时。单线程 tokio runtime 下阻塞锁可接受。

---

## 8. 前端设计

### 8.1 技术选型

- Vite 7 + Vue 3 + TypeScript（标准 SFC 工程）
- 构建产物 `dashboard-ui/dist/` 通过 `rust-embed` 嵌入二进制
- 开发时 Vite dev server (5173) proxy `/api` → Axum (9527)
- 依赖列表包含 `primevue` / `@primeuix/themes` / `@tabler/icons-vue`（当前以自研组件为主）
- 设计系统：CSS 变量 + 轻量动效，支持浅色/深色与密度切换

### 8.2 页面布局

```
┌─────────────────────────────────────────┐
│  XunYu Dashboard [Home][Bookmarks][Ports][Proxy][Config][Redirect][Audit] │
│  Density  Theme                                               Cmd ⌘/Ctrl+K│
├─────────────────────────────────────────┤
│                                         │
│  (根据选中 Tab 显示对应面板)              │
│                                         │
└─────────────────────────────────────────┘
```

### 8.3 Bookmarks 面板

```
┌─ 搜索框 ──────────────────── [+ 新增] ─┐
├─────────────────────────────────────────┤
│ Name     │ Path          │ Tags │ Visits│ ×  │
│ home     │ C:\Users\xxx  │ dev  │ 42    │ 🗑 │
│ project  │ D:\code\proj  │ work │ 15    │ 🗑 │
└─────────────────────────────────────────┘
```

功能：搜索过滤、排序、列可见性、分组视图、批量操作、内联编辑、复制/打开路径、导出 CSV/JSON。

### 8.4 Ports 面板

```
┌─ [刷新] ─── [仅开发端口 ☑] ────────────┐
├─────────────────────────────────────────┤
│ Port │ PID   │ Process    │ Path  │     │
│ 3000 │ 12345 │ node.exe   │ ...   │ Kill│
│ 8080 │ 67890 │ java.exe   │ ...   │ Kill│
└─────────────────────────────────────────┘
```

功能：刷新、自动刷新、协议/进程过滤、PID 分组、Kill 进程（双确认倒计时）、导出 CSV/JSON。

### 8.5 Proxy 面板

```
┌─────────────────────────────────────────┐
│  ┌─ System ──┐  ┌─ Env ─────┐          │
│  │ ON        │  │ ON        │          │
│  │ 127..7890 │  │ 127..7890 │          │
│  └───────────┘  └───────────┘          │
│  ┌─ Git ─────┐  ┌─ npm ─────┐          │
│  │ OFF       │  │ OFF       │          │
│  │ —         │  │ —         │          │
│  └───────────┘  └───────────┘          │
└─────────────────────────────────────────┘
```

功能：查看/保存 defaultUrl+noproxy、应用/移除代理、MSYS2 开关、连通性探测（timeout/jobs）、一致性提示。

### 8.6 Home 面板

功能：聚合展示书签计数、端口摘要、代理健康与审计概览（前端聚合）。

### 8.7 Audit 面板

功能：筛选（action/result/search）、统计卡片、分页、失败高亮、详情弹窗、导出 CSV/JSON。

### 8.8 Config 面板

功能：读取/编辑全局配置（GET/POST/PUT `/api/config`），提供基础表单入口。

### 8.9 全局交互

功能：统一 Toast（success/error/warning/info）、全局 Loading、列表 Skeleton、Command Palette（Ctrl/Cmd+K）、密度切换（compact/spacious）、主题切换（system/light/dark + View Transition）。

---

## 9. 服务器启动流程

```rust
pub(crate) fn cmd_serve(args: ServeCmd) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    rt.block_on(async {
        let app = build_router();
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], args.port));
        ui_println!("Dashboard: http://localhost:{}", args.port);
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
}
```

---

## 10. 实现顺序

| 步骤 | 文件 | 内容 |
|------|------|------|
| 1 | `Cargo.toml` | 依赖 + feature（axum, tokio, rust-embed, mime_guess） |
| 2 | `src/ports.rs` | Serialize derive |
| 3 | `src/cli.rs` | ServeCmd 定义 |
| 4 | `src/commands/mod.rs` | 模块注册 + dispatch |
| 5 | `src/commands/dashboard/mod.rs` | Router + rust-embed 静态文件 + 启动 |
| 6 | `src/commands/dashboard/handlers.rs` | API handlers |
| 7 | `dashboard-ui/` | Vite + Vue 3 + TS 前端工程 |
| 8 | `cd dashboard-ui && npm install && npm run build` | 构建前端产物到 dist/ |

---

## 11. 验证

```powershell
# 1. 默认编译不受影响
cargo build

# 2. 前端构建
cd dashboard-ui && npm install && npm run build && cd ..

# 3. dashboard feature 编译（需先完成步骤 2）
cargo build --features dashboard

# 4. 启动服务
cargo run --features dashboard -- serve

# 5. 浏览器访问
# http://localhost:9527

# 6. API 测试
curl http://localhost:9527/api/bookmarks
curl http://localhost:9527/api/ports
curl http://localhost:9527/api/proxy/status
curl -X POST http://localhost:9527/api/bookmarks/test -H "Content-Type: application/json" -d "{\"path\":\"C:\\tmp\",\"tags\":[\"test\"]}"
curl -X DELETE http://localhost:9527/api/bookmarks/test

# 7. 开发模式（前后端分离热更新）
# 终端 1: cargo run --features dashboard -- serve
# 终端 2: cd dashboard-ui && npm run dev
# 浏览器访问 http://localhost:5173（Vite proxy → 9527）
```
