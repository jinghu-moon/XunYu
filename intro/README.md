# XunYu 文档索引

`intro/` 目录放的是上手与使用说明；设计、实现与测试文档请看 `../docs/README.md`。

## 通用入口

- 安装与构建：`./Install.md`
- 使用约定与数据文件：`./Usage.md`
- Shell 集成与补全：`./Shell.md`
- 功能与编译特性：`./Features.md`

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
- Env Core 内部结构导读：`./env/Env-Core-Internals.md`
- ACL 子系统索引：`./acl/README.md`
- ACL 模块导读：`./acl/ACL-Modules.md`
- ACL 内部结构导读：`./acl/ACL-Internals.md`
- Redirect 子系统索引：`./redirect/README.md`
- Redirect 模块导读：`./redirect/Redirect-Modules.md`
- Diff 子系统索引：`./diff/README.md`
- Diff 模块导读：`./diff/Diff-Modules.md`

## Dashboard 入口

- Dashboard 组件导读：`./dashboard/Dashboard-Components.md`
- Dashboard 基础骨架导读：`./dashboard/Dashboard-Foundation.md`
- Dashboard 共享组件索引：`./dashboard/shared/README.md`
- Dashboard 业务面板索引：`./dashboard/panels/README.md`
- Dashboard Env 子系统索引：`./dashboard/env/README.md`
- Dashboard Diff 子系统索引：`./dashboard/diff/README.md`

## Dashboard 细分入口

- 共享组件：`./dashboard/shared/Dashboard-Capsule-Tabs.md`、`./dashboard/shared/Dashboard-Command-Palette.md`、`./dashboard/shared/Dashboard-Density-Toggle.md`、`./dashboard/shared/Dashboard-Theme-Toggle.md`、`./dashboard/shared/Dashboard-Global-Feedback.md`、`./dashboard/shared/Dashboard-Skeleton-Table.md`、`./dashboard/shared/Dashboard-Button.md`
- 业务面板：`./dashboard/panels/Dashboard-Home-Panel.md`、`./dashboard/panels/Dashboard-Bookmarks-Panel.md`、`./dashboard/panels/Dashboard-Config-Panel.md`、`./dashboard/panels/Dashboard-Proxy-Panel.md`、`./dashboard/panels/Dashboard-Ports-Panel.md`、`./dashboard/panels/Dashboard-Redirect-Panel.md`、`./dashboard/panels/Dashboard-Audit-Panel.md`
- Env 子组件：`./dashboard/env/Dashboard-Env-Panel.md`、`./dashboard/env/Dashboard-Env-Vars-Table.md`、`./dashboard/env/Dashboard-Env-Path-Editor.md`、`./dashboard/env/Dashboard-Env-Snapshots-Panel.md`、`./dashboard/env/Dashboard-Env-Doctor-Panel.md`、`./dashboard/env/Dashboard-Env-Diff-Panel.md`、`./dashboard/env/Dashboard-Env-Graph-Panel.md`、`./dashboard/env/Dashboard-Env-Profiles-Panel.md`、`./dashboard/env/Dashboard-Env-Schema-Panel.md`、`./dashboard/env/Dashboard-Env-Annotations-Panel.md`、`./dashboard/env/Dashboard-Env-Template-Run-Panel.md`、`./dashboard/env/Dashboard-Env-Import-Export-Panel.md`、`./dashboard/env/Dashboard-Env-Audit-Panel.md`、`./dashboard/env/Dashboard-Env-Var-History-Drawer.md`
- Diff 子组件：`./dashboard/diff/Dashboard-Diff-Panel.md`、`./dashboard/diff/Dashboard-Diff-FileManager.md`、`./dashboard/diff/Dashboard-Diff-File-Preview.md`、`./dashboard/diff/Dashboard-Diff-Options.md`、`./dashboard/diff/Dashboard-Diff-Convert-Panel.md`、`./dashboard/diff/Dashboard-Diff-Stats.md`、`./dashboard/diff/Dashboard-Diff-Config-Tree.md`、`./dashboard/diff/Dashboard-Diff-Code-Panel.md`、`./dashboard/diff/Dashboard-Diff-Line-Panel.md`、`./dashboard/diff/Dashboard-Diff-Viewer.md`、`./dashboard/diff/Dashboard-Diff-File-Browser.md`、`./dashboard/diff/Dashboard-Diff-Relations.md`


## 当前能力速览

- 默认可用：书签命令族、`config`、`ctx`、`proxy`、`ports/kill/ps/pkill`、`bak`、`tree`、`find`、`delete/del`、`rm`、`env`、`video`、`acl`。
- 可选 feature：`alias`、`lock`、`protect`、`crypt`、`redirect`、`dashboard`、`diff`、`batch_rename`（`brn`）、`cstat`、`img`。
- 如果启用 `dashboard,diff`：除了 `xun diff` 外，还会提供 Dashboard 的文件浏览、diff、转换、校验和 WebSocket 实时刷新能力。
- 如果你想从“一个命令模块一个命令模块”的角度理解 CLI，请直接看 `./cli/CLI-Modules.md`。
- 如果你想先从高频日常命令入手，请看 `./cli/Daily-CLI-Modules.md`。
- 如果你想从配置、上下文、搜索、备份、删除这条工程工作流理解 CLI，请看 `./cli/Tooling-CLI-Modules.md`。
- 如果你想继续理解 feature 型 / 专项型命令族，请看 `./cli/Extended-CLI-Modules.md`。
- 如果你想单独深入 `env` 子系统，请先看 `./env/Env-Modules.md`，再看 `./env/Env-Core-Internals.md`。
- 如果你想单独深入 Windows ACL 子系统，请先看 `./acl/ACL-Modules.md`，再看 `./acl/ACL-Internals.md`。
- 如果你想单独深入规则驱动的文件重定向，请看 `./redirect/Redirect-Modules.md`。
- 如果你想单独深入 `xun diff` 与 Dashboard Diff 工作台，请看 `./diff/Diff-Modules.md`。
- 如果你要从“一个组件一个组件”的角度理解 Dashboard，请先看 `./dashboard/Dashboard-Components.md`。
- 如果你还想理解 Dashboard 的壳层、公共组件与 API / 类型边界，再看 `./dashboard/Dashboard-Foundation.md`。
- 如果你想继续拆 Dashboard 壳层公共组件，可接着看 `./dashboard/shared/Dashboard-Capsule-Tabs.md`、`./dashboard/shared/Dashboard-Command-Palette.md`、`./dashboard/shared/Dashboard-Density-Toggle.md`、`./dashboard/shared/Dashboard-Theme-Toggle.md`、`./dashboard/shared/Dashboard-Global-Feedback.md`、`./dashboard/shared/Dashboard-Skeleton-Table.md`、`./dashboard/shared/Dashboard-Button.md`。
- 如果你想继续按具体面板拆 Dashboard，可继续看 `./dashboard/panels/Dashboard-Bookmarks-Panel.md`、`./dashboard/panels/Dashboard-Config-Panel.md`、`./dashboard/panels/Dashboard-Proxy-Panel.md`。
- 如果你想继续看总览、端口和重定向工作台，可接着看 `./dashboard/panels/Dashboard-Home-Panel.md`、`./dashboard/panels/Dashboard-Ports-Panel.md`、`./dashboard/panels/Dashboard-Redirect-Panel.md`。
- 如果你想继续看审计、Diff 容器和 Env 总容器，可接着看 `./dashboard/panels/Dashboard-Audit-Panel.md`、`./dashboard/diff/Dashboard-Diff-Panel.md`、`./dashboard/env/Dashboard-Env-Panel.md`。
- 如果你想继续拆 Env 的编辑与分析子组件，可接着看 `./dashboard/env/Dashboard-Env-Vars-Table.md`、`./dashboard/env/Dashboard-Env-Path-Editor.md`、`./dashboard/env/Dashboard-Env-Snapshots-Panel.md`、`./dashboard/env/Dashboard-Env-Doctor-Panel.md`、`./dashboard/env/Dashboard-Env-Diff-Panel.md`、`./dashboard/env/Dashboard-Env-Graph-Panel.md`。
- 如果你想继续补齐 Env 的治理与收尾子组件，可接着看 `./dashboard/env/Dashboard-Env-Profiles-Panel.md`、`./dashboard/env/Dashboard-Env-Schema-Panel.md`、`./dashboard/env/Dashboard-Env-Annotations-Panel.md`、`./dashboard/env/Dashboard-Env-Template-Run-Panel.md`、`./dashboard/env/Dashboard-Env-Import-Export-Panel.md`、`./dashboard/env/Dashboard-Env-Audit-Panel.md`、`./dashboard/env/Dashboard-Env-Var-History-Drawer.md`。
- 如果你想继续深入 Diff 子组件，可接着看 `./dashboard/diff/Dashboard-Diff-FileManager.md`、`./dashboard/diff/Dashboard-Diff-File-Preview.md`、`./dashboard/diff/Dashboard-Diff-Options.md`、`./dashboard/diff/Dashboard-Diff-Convert-Panel.md`、`./dashboard/diff/Dashboard-Diff-Stats.md`、`./dashboard/diff/Dashboard-Diff-Config-Tree.md`、`./dashboard/diff/Dashboard-Diff-Code-Panel.md`、`./dashboard/diff/Dashboard-Diff-Line-Panel.md`、`./dashboard/diff/Dashboard-Diff-Viewer.md`、`./dashboard/diff/Dashboard-Diff-File-Browser.md`。

## 建议阅读顺序

1. 先看 `./Install.md`，把可执行构建组合和 Dashboard 前端构建方式跑通。
2. 再看 `./Features.md`，明确哪些能力受 Cargo feature 控制。
3. 如果你要读 CLI 代码，再看 `./cli/CLI-Modules.md`，它把入口、分发和各命令族职责串起来了。
4. 如果你想先建立对高频日常命令的直觉，就先补 `./cli/Daily-CLI-Modules.md`。
5. 如果你想从工程工作流角度理解配置、上下文、搜索、备份和删除，再看 `./cli/Tooling-CLI-Modules.md`。
6. 如果你想继续看 feature 型和专项型能力，再看 `./cli/Extended-CLI-Modules.md`。
7. 如果你要深入最复杂的 CLI 子系统，先看 `./env/Env-Modules.md`，再看 `./env/Env-Core-Internals.md`；然后看 `./acl/ACL-Modules.md`，再看 `./acl/ACL-Internals.md`。
8. 如果你要看规则引擎与文件对比能力，再看 `./redirect/Redirect-Modules.md` 和 `./diff/Diff-Modules.md`。
9. 如果你要读 Web UI，先看 `./dashboard/Dashboard-Components.md`，再看 `./dashboard/Dashboard-Foundation.md`。
10. 如果你要继续拆 Dashboard 壳层公共组件，建议按 `./dashboard/shared/Dashboard-Capsule-Tabs.md` → `./dashboard/shared/Dashboard-Command-Palette.md` → `./dashboard/shared/Dashboard-Density-Toggle.md` → `./dashboard/shared/Dashboard-Theme-Toggle.md` → `./dashboard/shared/Dashboard-Global-Feedback.md` → `./dashboard/shared/Dashboard-Skeleton-Table.md` → `./dashboard/shared/Dashboard-Button.md` 的顺序读。
11. 如果你要继续按工作台理解具体面板，接着看 `./dashboard/panels/Dashboard-Bookmarks-Panel.md`、`./dashboard/panels/Dashboard-Config-Panel.md`、`./dashboard/panels/Dashboard-Proxy-Panel.md`。
12. 如果你要继续补齐总览与系统工作台，再看 `./dashboard/panels/Dashboard-Home-Panel.md`、`./dashboard/panels/Dashboard-Ports-Panel.md`、`./dashboard/panels/Dashboard-Redirect-Panel.md`。
13. 如果你要理解审计、Diff 与 Env 这三条更偏基础设施的工作台，再看 `./dashboard/panels/Dashboard-Audit-Panel.md`、`./dashboard/diff/Dashboard-Diff-Panel.md`、`./dashboard/env/Dashboard-Env-Panel.md`。
14. 如果你要继续拆 Env 的编辑与分析子组件，建议按 `./dashboard/env/Dashboard-Env-Vars-Table.md` → `./dashboard/env/Dashboard-Env-Path-Editor.md` → `./dashboard/env/Dashboard-Env-Snapshots-Panel.md` → `./dashboard/env/Dashboard-Env-Doctor-Panel.md` → `./dashboard/env/Dashboard-Env-Diff-Panel.md` → `./dashboard/env/Dashboard-Env-Graph-Panel.md` 的顺序读。
15. 如果你要继续拆 Env 的治理与收尾子组件，建议按 `./dashboard/env/Dashboard-Env-Profiles-Panel.md` → `./dashboard/env/Dashboard-Env-Schema-Panel.md` → `./dashboard/env/Dashboard-Env-Annotations-Panel.md` → `./dashboard/env/Dashboard-Env-Template-Run-Panel.md` → `./dashboard/env/Dashboard-Env-Import-Export-Panel.md` → `./dashboard/env/Dashboard-Env-Audit-Panel.md` → `./dashboard/env/Dashboard-Env-Var-History-Drawer.md` 的顺序读。
16. 如果你要继续钻 Diff 子组件，建议按 `./dashboard/diff/Dashboard-Diff-FileManager.md` → `./dashboard/diff/Dashboard-Diff-File-Preview.md` → `./dashboard/diff/Dashboard-Diff-Options.md` → `./dashboard/diff/Dashboard-Diff-Convert-Panel.md` → `./dashboard/diff/Dashboard-Diff-Stats.md` → `./dashboard/diff/Dashboard-Diff-Config-Tree.md` → `./dashboard/diff/Dashboard-Diff-Code-Panel.md` → `./dashboard/diff/Dashboard-Diff-Line-Panel.md` → `./dashboard/diff/Dashboard-Diff-Viewer.md` → `./dashboard/diff/Dashboard-Diff-File-Browser.md` 的顺序读。
17. 最后按需查 `./Usage.md`、`./cli/Commands.md` 和 `./cli/Examples.md`。




## Dashboard ??

- Dashboard ?????`./dashboard/Dashboard-Usage.md`
