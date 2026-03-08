# Dashboard 业务面板索引

这一组文档对应 Dashboard 的一级业务工作台。它们大多直接挂在 `App.vue` 的顶层 tab 上，负责某一个独立领域的展示与操作。

和 `EnvPanel`、`DiffPanel` 这种容器型子系统不同，这一组大多是“单面板即可形成完整工作流”的业务面板。

## 这一组解决什么问题

- 首页总览如何聚合多个子系统
- 书签、配置、代理、端口等高频操作如何落到独立工作台
- Redirect 与 Audit 这类专项能力如何在 Dashboard 中独立呈现

## 建议阅读顺序

1. `./Dashboard-Home-Panel.md`
2. `./Dashboard-Bookmarks-Panel.md`
3. `./Dashboard-Config-Panel.md`
4. `./Dashboard-Proxy-Panel.md`
5. `./Dashboard-Ports-Panel.md`
6. `./Dashboard-Redirect-Panel.md`
7. `./Dashboard-Audit-Panel.md`

## 文档清单

- 总览首页：`./Dashboard-Home-Panel.md`
- 书签工作台：`./Dashboard-Bookmarks-Panel.md`
- 配置工作台：`./Dashboard-Config-Panel.md`
- 代理工作台：`./Dashboard-Proxy-Panel.md`
- 端口工作台：`./Dashboard-Ports-Panel.md`
- Redirect 工作台：`./Dashboard-Redirect-Panel.md`
- 审计工作台：`./Dashboard-Audit-Panel.md`

## 和其他目录的关系

- 公共交互与壳层组件见 `../shared/README.md`
- Env 总容器及其子组件见 `../env/README.md`
- Diff 总容器及其子组件见 `../diff/README.md`

