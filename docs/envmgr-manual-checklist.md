# EnvMgr 最小人工验收清单（收口版）

> 目标：一次性关闭剩余人工项（`P3.6`、`P5.4` 页面联调、`Gate-C`、`Gate-E`、`DoD`）。
>
> 预计耗时：`10~15` 分钟  
> 平台：Windows  
> 约束：仅使用 `user` 作用域；所有临时变量均使用 `XUN_ENVMGR_MANUAL_*` 前缀并在末尾清理。

---

## 0. 预检查（1 分钟）

```powershell
cargo check --all-features
pnpm -C "dashboard-ui" build
```

通过标准：
- 两条命令均 `PASS`。

---

## 1. TUI 验收（P3.6 / Gate-C，约 6 分钟）

### 1.1 启动与退出恢复

```powershell
cargo run --features tui -- env tui
```

在 TUI 中执行：
1. 进入后界面正常渲染，无乱码。
2. 按 `q` 退出（或按内置退出键）。

通过标准：
- 退出后终端状态正常（无残留 raw mode、无光标异常、可继续输入命令）。

### 1.2 手测链路 A：set -> snapshot -> del -> restore

在 TUI 中执行（作用域 `user`）：
1. 新增变量：`XUN_ENVMGR_MANUAL_TUI_A=ok`
2. 创建快照（描述可写 `manual-tui-a`）
3. 删除变量 `XUN_ENVMGR_MANUAL_TUI_A`
4. 从刚创建的快照执行恢复

退出 TUI 后验证：

```powershell
target/debug/xun.exe env get "XUN_ENVMGR_MANUAL_TUI_A" --scope user
```

通过标准：
- `get` 能读到该变量，表示恢复链路有效。

### 1.3 手测链路 B：doctor -> fix -> diff-live

在 TUI 中执行（作用域 `user`）：
1. 运行 doctor
2. 若存在可修复项，执行 fix
3. 打开 diff/live 视图确认可查看最新差异

通过标准：
- doctor/fix/diff 页面可操作，无阻断错误或崩溃。

---

## 2. Dashboard 页面联调（P5.4 / Gate-E，约 5 分钟）

### 2.1 启动服务

```powershell
target/debug/xun.exe serve --port 7071
```

浏览器打开：
- `http://127.0.0.1:7071`
- 切换到 `Env` 标签页

### 2.2 页面交互最小闭环

在 Env 面板执行（作用域 `user`）：
1. 新增变量：`XUN_ENVMGR_MANUAL_WEB=ok`
2. 删除变量：`XUN_ENVMGR_MANUAL_WEB`
3. 创建快照（任意描述）
4. 点击 doctor run（可选再点 fix）
5. 导入 dry-run（任意小内容，如 `A=1`）
6. 执行一次导出（任意格式）
7. 查看 diff-live 区域是否刷新
8. 观察 WS 状态显示 `connected`

通过标准：
- 页面操作无阻断 bug（可继续操作，不出现致命弹窗/白屏）。
- 关键链路均能得到成功反馈。

---

## 3. 清理（1 分钟）

```powershell
target/debug/xun.exe env del "XUN_ENVMGR_MANUAL_TUI_A" --scope user -y
target/debug/xun.exe env del "XUN_ENVMGR_MANUAL_WEB" --scope user -y
```

---

## 4. 回填清单（执行完成后）

在 `docs/envmgr-integration-tasks.md` 回填：
- `P3.6` 三项从 `[-]` -> `[x]`
- `P5.4` 中 `xun serve + 页面联调通过` 从 `[-]` -> `[x]`
- `Gate-C`、`Gate-E` 从 `[-]` -> `[x]`
- `DoD 全项通过` 从 `[-]` -> `[x]`

建议同步在“本轮验证记录”增加一行：
- `manual checklist (TUI + Dashboard)`：PASS（含清理）
