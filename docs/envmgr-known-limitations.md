# EnvMgr 已知限制清单

## 1. 平台限制

- 注册表环境变量读写是 Windows 专属能力。
- 非 Windows 平台保持可编译，但相关能力会返回明确不可用错误。

## 2. 权限限制

- `--scope system` 的写入通常需要管理员权限。
- 权限不足时会返回权限错误，不会回退到静默写入。

## 3. PATH 诊断保守性

- `doctor` 对含 `%VAR%` 占位符的 PATH 条目可能标记为 missing。
- 该结果是保守检查，需要结合实际机器环境判断。

## 4. Web Run 默认关闭

- `/api/env/run` 默认关闭，需显式开启：
  - `xun env config set allow_run true`
  - 或临时 `ENVMGR_ALLOW_RUN=1`
- 建议仅在本地受控开发环境使用。

## 5. 回归 warning 现状

- 当前仓库存在历史 warning（deprecated/unused 等）。
- 不阻断 EnvMgr 功能验收；以 `cargo check --all-features` 与 `cargo test --all-features` 通过作为回归门槛。
