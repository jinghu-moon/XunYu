# EnvMgr 集成发布说明（v4 交付）

## 1. 新增能力

- 新增 `xun env` 完整命令树（list/get/set/del/path/snapshot/doctor/import/export/diff-live/tui）
- 新增 `src/env_core/*` 统一核心能力：
  - 注册表读写
  - 快照/恢复
  - 并发锁
  - doctor/diff
  - import/export
  - 事件广播
- 新增 Dashboard API：
  - `/api/env/*`
  - `/api/env/ws`
- 新增高级 API：
  - `/api/env/annotations*`
  - `/api/env/template/expand`
  - `/api/env/export-live`
  - `/api/env/run`（受门控）
- 新增 Dashboard Env 面板与子组件（变量、PATH、快照、doctor、导入导出）
- 新增 Profiles / History / Schema / Annotations / Template-Run 面板能力
- 新增 `config.toml` 配置持久化（兼容旧 JSON 配置迁移）

## 2. 架构决策

- 统一规则：所有业务逻辑在 `env_core`，CLI/TUI/Web 仅适配
- 写入管线固定为 `lock -> snapshot -> write -> broadcast`
- Windows 专属能力通过 `cfg(windows)` 隔离，非 Windows 保持可编译与可诊断错误

## 3. 迁移建议

- 原有手工注册表脚本可迁移到 `xun env import/export`
- 先以 `--scope user` 完成验证，再切换 `--scope system`
- 在自动化流程中优先使用 `--format json` / `--dry-run`
- 需要启用 Web Run 时，优先使用配置项：`xun env config set allow_run true`

## 4. 已知限制

- `system` 作用域变更依赖管理员权限
- doctor 对含变量占位符 PATH 条目可能给出保守告警
- 当前 warning 未完全清零（不影响功能验收）
- 服务端命令执行接口默认关闭（安全默认）

详见：`./envmgr-known-limitations.md`

## 5. Smoke 验证脚本建议

```powershell
tools/envmgr-smoke.ps1
```

## 6. 验收归档建议

- 保留 `cargo test --all-features` 与 `cargo check --all-features` 输出日志
- 保留 `pnpm -C dashboard-ui build` 输出日志
- 保留 `/api/env/ping`、`/api/env/vars`、`/api/env/ws` 冒烟结果
- 结果模板：`./envmgr-smoke-report.md`
