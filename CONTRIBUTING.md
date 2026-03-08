# 参与贡献

感谢你关注 `XunYu`。

当前仓库以主干迭代为主，**暂不发布正式版本**；欢迎通过小步、可验证的方式持续改进 CLI、Dashboard、文档与测试。

## 基本原则

- 优先保持 `XunYu` / `xun` / `xyu` / `xy` 命名口径一致。
- 优先做小而清晰的改动，避免把不相关重构混入同一批提交。
- 新增或调整能力时，优先补文档；涉及行为变化时，优先补测试。
- 危险操作相关能力必须保留预演、确认和结果说明链路。

## 本地准备

```bash
git clone https://github.com/jinghu-moon/XunYu.git
cd XunYu
```

### CLI / Rust

```bash
cargo build --release
cargo test
```

### Dashboard / 前端

```bash
pnpm -C dashboard-ui install
pnpm -C dashboard-ui build
pnpm -C dashboard-ui test
```

> 如果只改 Rust CLI 或文档，不必强制跑前端测试；如果改了 `dashboard-ui/`、接口契约或 Dashboard 文档，建议补跑前端相关命令。

## 变更范围建议

- `src/`：CLI 定义、命令实现、底层能力
- `dashboard-ui/`：Vue 3 Dashboard 前端
- `intro/`：上手与组件/模块导读
- `docs/`：设计、实现、测试与评审文档
- `tests/`：Rust 集成测试与基础回归

## 提交前检查

- 代码、文档与命名口径是否一致
- `README.md`、`intro/`、`docs/` 是否需要同步更新
- 相关命令示例是否仍以 `xun` 为正式入口
- 相关测试是否已执行，或至少说明未执行原因

## Pull Request 建议

- 标题尽量聚焦单一主题，例如：
  - `docs: clarify dashboard usage`
  - `feat: add proxy dry-run response`
  - `refactor: split env handlers`
- 描述建议包含：
  - 变更目的
  - 影响范围
  - 验证方式
  - 是否涉及危险操作链路

## 文档与变更记录

- 协作说明见 `CONTRIBUTING.md`
- 变更记录见 `CHANGELOG.md`
- 命名策略见 `docs/project/Naming-Strategy.md`

如果你准备提交较大改动，建议先在 Issue 或草稿 PR 中说明设计方向，再进入实现。
