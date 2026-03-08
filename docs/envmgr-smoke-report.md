# EnvMgr Smoke 结果归档

## 执行信息

- 日期：`2026-03-06`
- 执行人：`Codex`
- 平台：`Windows 11`
- Rust：`rustc 1.93.1 (01f6ddf75 2026-02-11)`
- Node/pnpm：`pnpm 10.30.1`

## 命令结果

1. `cargo check --all-features`：`PASS`
2. `cargo test --all-features`：`PASS`
3. `pnpm -C dashboard-ui build`：`PASS`
4. `xun env --help`：`PASS`
5. `xun env list --scope user -f json`：`PASS`
6. `xun env snapshot create --desc smoke`：`PASS`
7. `xun env diff-live --scope user --format json`：`PASS`
8. `xun env snapshot create --desc backup-smoke-*` + `xun env snapshot list`：`PASS`
9. 最新快照 JSON 结构校验（`id/created_at/user_vars/system_vars`）：`PASS`
10. `xun env set/get/del`（临时变量 `XUN_ENVMGR_SMOKE_WRITE_*`）链路验证：`PASS`
11. `tools/envmgr-smoke.ps1 -SkipTests -VerifyWsChanged`：`PASS`
12. `tools/envmgr-concurrency-smoke.ps1 -Workers 4 -Iterations 10`：`PASS`
   - 输出：`operations=120`，`baseline snapshots=34`，`final snapshots=50`（达到 `max_snapshots` 上限后裁剪生效）
13. `tools/envmgr-dashboard-chain-smoke.ps1 -Port 7073`：`PASS`（CRUD/snapshot/doctor/import/export/diff）
14. `xun env status --scope all --format text`：`PASS`
15. `GET /api/env/status?scope=all`：`PASS (200)`
16. `xun env export-all --scope user --out ./.tmp/xun-env-user.zip`：`PASS`
17. `GET /api/env/export-all?scope=user`：`PASS (200, application/zip)`
18. `manual checklist (TUI + Dashboard)`：`PASS`（含临时变量清理）
19. `xun env list --scope user -f json`：`PASS`（返回 `inferred_kind`）
20. `GET /api/env/vars?scope=user`：`PASS`（返回 `inferred_kind`）
21. `xun env diff-live --scope user --since 2026-03-01 --format json`：`PASS`
22. `xun env import --help`：`PASS`（包含 `--stdin`）
23. `echo "XUN_ENVMGR_STDIN_DRYRUN=ok" | xun env import --stdin --scope user --dry-run`：`PASS`
24. `xun env import ./.tmp/env-smoke.json --stdin --scope user --dry-run`：`PASS`（返回输入互斥错误）
25. `cargo test dep_graph -- --nocapture`：`PASS`（2 passed）
26. `xun env graph PATH --scope user --max-depth 3 --format text|json`：`PASS`
27. `pnpm -C dashboard-ui build`（含 Dependency Graph 面板）：`PASS`
28. `xun env snapshot prune --keep 99999`：`PASS`（removed=0，remaining=50）
29. `cargo check --all-features`（latest）：`PASS`（0 warnings）
30. `xun env config set/get general.snapshot_every_secs`：`PASS`（兼容 `snapshot_every_secs` 与 `general.snapshot_every_secs`）
31. `xun serve --port 7098`（短时运行）后 `xun env snapshot list -f json`：`PASS`（出现多条 `auto-snapshot`，并保持 `max_snapshots=50` 裁剪上限）

## Dashboard / API 冒烟

1. 启动：`xun serve --port 7071`：`PASS`
2. `GET /api/env/ping`：`PASS (200)`
3. `GET /api/env/vars?scope=user`：`PASS (200)`
4. `GET /api/env/schema`：`PASS (200)`
5. `GET /api/env/annotations`：`PASS (200)`
6. `POST /api/env/template/expand`：`PASS (200)`
7. `GET /api/env/export-live`：`PASS (200)`
8. `GET /api/env/export-all?scope=user`：`PASS (200, zip attachment)`
9. `WS /api/env/ws` connected 首帧：`PASS`（`{"type":"connected","channel":"env"}`）
10. `WS /api/env/ws` changed 事件：`PASS`（触发 `POST /api/env/vars/{tmp}` 后收到 `{"type":"changed", ...}`）
11. `GET /api/env/graph?scope=user&name=PATH&max_depth=3`：`PENDING`（当前自动化环境禁止后台拉起 `xun serve` 进程，需手工验证）

## 备注

- 问题记录：`none`
- 风险说明：`TUI 全链路手测与 Dashboard 页面联调仍需专项补测（见 envmgr-integration-tasks.md）`
