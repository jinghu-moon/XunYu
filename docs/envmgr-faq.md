# EnvMgr FAQ

## Q1: 为什么 `diff-live` 提示 `no snapshots found`？

`diff-live` 需要基线快照。先执行：

```powershell
xun env snapshot create --desc "baseline"
```

再运行 `diff-live`。

## Q2: 为什么 `--scope system` 报权限错误？

System 作用域写入通常需要管理员权限。请以管理员身份运行终端，或改用 `--scope user`。

## Q3: 非 Windows 平台能否使用 EnvMgr 写入？

当前注册表读写能力是 Windows 专用。非 Windows 平台可编译，但会返回明确的不可用错误。

## Q4: PATH 中 `%NVM_HOME%` 等占位符被 doctor 标记为 missing，是否异常？

doctor 会按可解析目录做存在性检查。若路径使用变量占位符，可能被标记为缺失。可按实际环境决定是否修复。

## Q5: `import --mode overwrite` 会覆盖什么？

会覆盖目标作用域中同名变量。建议先执行：

```powershell
xun env import "./file" --scope user --dry-run
```

确认结果后再执行覆盖模式。

## Q6: 如何从误操作中恢复？

优先恢复快照：

```powershell
xun env snapshot list
xun env snapshot restore --id <SNAPSHOT_ID> -y
```

## Q7: Dashboard 没有实时刷新怎么办？

检查：

1. `xun serve` 是否正常运行
2. 浏览器是否能连通 `/api/env/ws`
3. 反向代理是否拦截了 WebSocket 升级

## Q8: 为什么有一些 `warning` 但构建通过？

当前存在部分历史模块 warning（如 deprecated/unused），不阻断 EnvMgr 功能。回归标准以 `cargo test --all-features` 与 `cargo check --all-features` 通过为准。

## Q9: 为什么 `/api/env/run` 返回 `env.run_disabled`？

默认出于安全原因关闭服务端执行能力。请显式启用其一：

```powershell
xun env config set allow_run true
```

或设置临时环境变量：

```powershell
$env:ENVMGR_ALLOW_RUN = "1"
```

## Q10: `config.toml` 在哪里？

可通过命令直接查看：

```powershell
xun env config path
```

默认在 Windows 为 `%APPDATA%/xun/env/config.toml`。可用 `XUN_ENV_CONFIG_DIR` 覆盖目录。
