# EnvMgr 使用手册

## 1. 目标与架构边界

EnvMgr 已集成到 `xun`，统一入口为：

- CLI：`xun env ...`
- TUI：`xun env tui`
- Web API：`/api/env/*`
- Dashboard：`Env` 面板

业务逻辑只在 `src/env_core/*`，CLI/TUI/Web 仅做适配与展示。

---

## 2. 运行前提

- 平台：Windows（注册表环境变量读写能力基于 `winreg`）
- `--scope user`：普通权限即可
- `--scope system`：通常需要管理员权限
- 所有写入统一走：`lock -> snapshot -> write -> broadcast`

---

## 3. CLI 快速使用

### 3.1 查询与检查

```powershell
xun env list --scope user -f table
xun env status --scope all --format text
xun env get PATH --scope user
xun env doctor --scope user --format text
xun env diff-live --scope user --format json
xun env diff-live --scope user --since 2026-03-01 --format json
xun env graph PATH --scope all --max-depth 8 --format text
```

### 3.2 PATH 维护

```powershell
xun env path add "C:/tools/bin" --scope user --head
xun env path rm "C:/tools/bin" --scope user
```

### 3.3 快照与恢复

```powershell
xun env snapshot create --desc "before-change"
xun env snapshot list
xun env snapshot restore --latest -y
xun env snapshot prune --keep 80
```

### 3.4 导入导出

```powershell
xun env export --scope user --format json --out "./.tmp/env.json"
xun env export-all --scope all --out "./.tmp/xun-env-all.zip"
xun env import "./.tmp/env.json" --scope user --dry-run
xun env import "./.tmp/env.json" --scope user --mode overwrite -y
Get-Content "./.tmp/env.env" | xun env import --stdin --scope user --dry-run
xun env export-live --scope all --format dotenv --out "./.tmp/live.env"
```

### 3.5 模板、运行、注释

```powershell
xun env template "Path=%PATH%" --scope user
xun env template "Path=%PATH%" --scope user --validate-only --format json

# run（默认继承合并后的环境）
xun env run -- cmd /c "echo ok"
xun env run --schema-check -- notify -- cmd /c "echo ok"

# 注释
xun env annotate set JAVA_HOME "JDK 安装目录"
xun env annotate list --format json
```

### 3.6 配置（config.toml）

```powershell
xun env config path
xun env config show
xun env config set allow_run true
xun env config set max_snapshots 100
xun env config set general.snapshot_every_secs 60
xun env config get snapshot_every_secs
```

默认配置文件位置：

- Windows：`%APPDATA%/xun/env/config.toml`
- 可通过环境变量覆盖：`XUN_ENV_CONFIG_DIR`（优先）或 `ENVMGR_CONFIG_DIR`
- 定时快照开关：`snapshot_every_secs`（或 `general.snapshot_every_secs` 别名），`0` 表示关闭

---

## 4. TUI 使用

启动：

```powershell
xun env tui
```

当前包含面板：

- Variables（查询/编辑/删除）
- PATH Editor（分段维护）
- Snapshots（创建/恢复）
- Doctor（巡检/修复）
- Import/Export（导入导出）
- Profiles（抓取/应用/删除/差异）
- History（审计与变量历史）
- Schema & Validate（规则维护与校验）
- Annotations（变量注释）
- Template/Export-Live/Run（模板扩展、实时导出、受控执行）

---

## 5. Dashboard 与 Web API

启动 dashboard：

```powershell
xun serve --port 7071
```

若 `snapshot_every_secs > 0`，`serve` 进程会按该周期自动创建 `auto-snapshot`。

浏览器访问：`http://127.0.0.1:7071`，切换到 `Env` 标签页。

Env 面板新增：

- 状态摘要条（vars/snapshots/profiles/schema/audit）
- Import/Export 的 `Export ZIP` 按钮
- 导入文本区支持拖拽 `.env/.json/.reg/.csv` 文件直接填充
- Variables 表格类型徽章（基于后端 `inferred_kind` 推断结果）

关键接口：

- `GET /api/env/ping`
- `GET /api/env/status`
- `GET /api/env/vars`
- `POST /api/env/vars/{name}`
- `DELETE /api/env/vars/{name}`
- `POST /api/env/path/add`
- `POST /api/env/path/remove`
- `GET/POST /api/env/snapshots`
- `DELETE /api/env/snapshots?keep=80`
- `POST /api/env/snapshots/restore`
- `POST /api/env/doctor/run`
- `POST /api/env/doctor/fix`
- `POST /api/env/import`
- `GET /api/env/export`
- `GET /api/env/export-all`
- `GET /api/env/export-live`
- `GET /api/env/diff-live`
- `GET /api/env/graph`
- `GET /api/env/audit`
- `GET /api/env/vars/{name}/history`
- `GET /api/env/profiles`
- `POST /api/env/profiles/{name}/capture`
- `POST /api/env/profiles/{name}/apply`
- `GET /api/env/profiles/{name}/diff`
- `DELETE /api/env/profiles/{name}`
- `GET /api/env/schema`
- `POST /api/env/schema/add-required`
- `POST /api/env/schema/add-regex`
- `POST /api/env/schema/add-enum`
- `POST /api/env/schema/remove`
- `POST /api/env/schema/reset`
- `POST /api/env/validate`
- `GET /api/env/annotations`
- `GET /api/env/annotations/{name}`
- `POST /api/env/annotations/{name}`
- `DELETE /api/env/annotations/{name}`
- `POST /api/env/template/expand`
- `POST /api/env/run`
- `GET /api/env/ws`

`/api/env/run` 默认关闭，需满足其一：

- `xun env config set allow_run true`
- 环境变量 `ENVMGR_ALLOW_RUN=1`（临时覆盖）

WebSocket 首帧示例：

```json
{"type":"connected","channel":"env"}
```

`diff-live` 还支持时间基线：

- `GET /api/env/diff-live?scope=user&since=2026-03-01`

依赖图接口示例：

- `GET /api/env/graph?scope=all&name=PATH&max_depth=8`

---

## 6. 安全与回滚建议

- 生产机器优先使用 `--dry-run` 与快照
- `del/restore/import overwrite` 属于高风险操作，建议显式确认
- 对 `system` 作用域变更前，建议先在 `user` 作用域验证
- 若发生异常，优先通过 `snapshot restore` 回滚
- Web `run` 接口默认关闭，建议仅在本地受控环境临时开启

---

## 7. Smoke 脚本

```powershell
powershell -ExecutionPolicy Bypass -File "./tools/envmgr-smoke.ps1"
```

常用参数：

- `-SkipTests`：跳过 `cargo test --all-features`
- `-SkipServe`：跳过 `serve + /api/env/* + ws` 冒烟
- `-Port 7089`：指定 dashboard 冒烟端口
- `-VerifyWsChanged`：额外验证 WS `changed` 事件（会临时写入并删除一个 smoke 变量）

并发压力脚本（会执行并发 `set/get/del`，建议先在测试机执行）：

```powershell
powershell -ExecutionPolicy Bypass -File "./tools/envmgr-concurrency-smoke.ps1" -Workers 4 -Iterations 10
```

Dashboard Env 链路脚本（覆盖 CRUD/snapshot/doctor/import/export/diff）：

```powershell
powershell -ExecutionPolicy Bypass -File "./tools/envmgr-dashboard-chain-smoke.ps1" -Port 7073
```
