# XunYu 文档索引

`intro/` 目录收录的是上手、使用与模块导读文档；设计、实现与测试相关文档请看 `../docs/project/`。

## 通用入口

- 安装与构建：`./Install.md`
- 使用约定与数据文件：`./Usage.md`
- Shell 集成与补全：`./Shell.md`
- 功能与 Cargo feature：`./Features.md`

## CLI 入口

- CLI 子目录索引：`./cli/README.md`
- CLI 模块导读：`./cli/CLI-Modules.md`
- 日常 CLI 模块导读：`./cli/Daily-CLI-Modules.md`
- 工程辅助 CLI 模块导读：`./cli/Tooling-CLI-Modules.md`
- 扩展能力 CLI 模块导读：`./cli/Extended-CLI-Modules.md`
- 命令速查：`./cli/Commands.md`
- 常用示例：`./cli/Examples.md`
- 命令示例（自动生成）：`./cli/Commands-Generated.md`

## 子系统入口

- Env 子系统索引：`./env/README.md`
- Env 模块导读：`./env/Env-Modules.md`
- Env Core 内部结构：`./env/Env-Core-Internals.md`
- ACL 子系统索引：`./acl/README.md`
- ACL 模块导读：`./acl/ACL-Modules.md`
- ACL 内部结构：`./acl/ACL-Internals.md`
- Redirect 子系统索引：`./redirect/README.md`
- Redirect 模块导读：`./redirect/Redirect-Modules.md`
- Diff 子系统索引：`./diff/README.md`
- Diff 模块导读：`./diff/Diff-Modules.md`

## Dashboard 入口

- Dashboard 使用说明：`./dashboard/Dashboard-Usage.md`
- Dashboard 组件导读：`./dashboard/Dashboard-Components.md`
- Dashboard 基础骨架：`./dashboard/Dashboard-Foundation.md`
- Dashboard 共享组件索引：`./dashboard/shared/README.md`
- Dashboard 业务面板索引：`./dashboard/panels/README.md`
- Dashboard Env 子系统索引：`./dashboard/env/README.md`
- Dashboard Diff 子系统索引：`./dashboard/diff/README.md`

## 当前能力速览

- 默认可用：书签命令族、`config`、`ctx`、`proxy`、`ports/kill/ps/pkill`、`bak`、`tree`、`find`、`delete/del`、`rm`、`env`、`video`、`acl`
- 可选 feature：`alias`、`lock`、`protect`、`crypt`、`redirect`、`dashboard`、`diff`、`batch_rename`（`brn`）、`cstat`、`img`
- 启用 `dashboard,diff` 后，除 `xun diff` 外，还会提供 Dashboard 的文件浏览、Diff、转换、校验与实时刷新能力

## 推荐阅读顺序

1. 先读 `./Install.md`，把构建与运行路径跑通。
2. 再读 `./Features.md`，明确功能与 Cargo feature 的对应关系。
3. 如果你要理解 CLI 入口与命令分发，继续读 `./cli/CLI-Modules.md`。
4. 如果你偏向高频日常命令，读 `./cli/Daily-CLI-Modules.md`。
5. 如果你偏向工程工作流，读 `./cli/Tooling-CLI-Modules.md`。
6. 如果你要深入复杂子系统，再读 `./env/Env-Modules.md`、`./env/Env-Core-Internals.md`、`./acl/ACL-Modules.md`、`./acl/ACL-Internals.md`。
7. 如果你要理解 Web UI，先读 `./dashboard/Dashboard-Usage.md`、`./dashboard/Dashboard-Foundation.md`、`./dashboard/Dashboard-Components.md`。
