# Dashboard Env 子系统索引

这一组文档对应 `EnvPanel` 及其子组件。它是 Dashboard 里最完整、最像“前端领域子系统”的一组文档。

如果你想理解 Env 工作台如何统一管理变量、PATH、快照、doctor、schema、annotation、导入导出与审计，这组文档就是主线。

## 这一组解决什么问题

- `EnvPanel` 如何作为总容器收口状态、API 与 WebSocket
- Env 编辑、分析、治理三类能力如何拆成独立子组件
- 容器层为什么统一处理刷新、历史、审计与结果回收

## 建议阅读顺序

1. `./Dashboard-Env-Panel.md`
2. `./Dashboard-Env-Vars-Table.md`
3. `./Dashboard-Env-Path-Editor.md`
4. `./Dashboard-Env-Snapshots-Panel.md`
5. `./Dashboard-Env-Doctor-Panel.md`
6. `./Dashboard-Env-Diff-Panel.md`
7. `./Dashboard-Env-Graph-Panel.md`
8. `./Dashboard-Env-Profiles-Panel.md`
9. `./Dashboard-Env-Schema-Panel.md`
10. `./Dashboard-Env-Annotations-Panel.md`
11. `./Dashboard-Env-Template-Run-Panel.md`
12. `./Dashboard-Env-Import-Export-Panel.md`
13. `./Dashboard-Env-Audit-Panel.md`
14. `./Dashboard-Env-Var-History-Drawer.md`

## 分组阅读

- 容器总览：`./Dashboard-Env-Panel.md`
- 编辑链：`./Dashboard-Env-Vars-Table.md`、`./Dashboard-Env-Path-Editor.md`
- 分析链：`./Dashboard-Env-Snapshots-Panel.md`、`./Dashboard-Env-Doctor-Panel.md`、`./Dashboard-Env-Diff-Panel.md`、`./Dashboard-Env-Graph-Panel.md`
- 治理链：`./Dashboard-Env-Profiles-Panel.md`、`./Dashboard-Env-Schema-Panel.md`、`./Dashboard-Env-Annotations-Panel.md`
- 执行与交换：`./Dashboard-Env-Template-Run-Panel.md`、`./Dashboard-Env-Import-Export-Panel.md`
- 审计与历史：`./Dashboard-Env-Audit-Panel.md`、`./Dashboard-Env-Var-History-Drawer.md`

## 和其他目录的关系

- 顶层业务面板入口见 `../panels/README.md`
- 公共 UI 组件见 `../shared/README.md`
- 文件与配置对比链路见 `../diff/README.md`

