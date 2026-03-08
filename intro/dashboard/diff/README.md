# Dashboard Diff 子系统索引

这一组文档对应 `DiffPanel` 及其子组件。它关注的不是单次 diff 渲染，而是从路径选择、选项配置、文件预览、格式转换到结果展示的一整条比较链路。

如果你想理解 Dashboard 中最复杂的文件对比工作台，这组文档是主入口。

## 这一组解决什么问题

- `DiffPanel` 如何编排整个 diff 工作流
- 文件选择、内容预览、结果渲染为何拆成多层组件
- 配置语义 diff、代码 diff、行级 diff 如何共享底层展示能力

## 建议阅读顺序

1. `./Dashboard-Diff-Panel.md`
2. `./Dashboard-Diff-FileManager.md`
3. `./Dashboard-Diff-File-Preview.md`
4. `./Dashboard-Diff-Options.md`
5. `./Dashboard-Diff-Convert-Panel.md`
6. `./Dashboard-Diff-Stats.md`
7. `./Dashboard-Diff-Config-Tree.md`
8. `./Dashboard-Diff-Code-Panel.md`
9. `./Dashboard-Diff-Line-Panel.md`
10. `./Dashboard-Diff-Viewer.md`
11. `./Dashboard-Diff-File-Browser.md`
12. `./Dashboard-Diff-Relations.md`

## 分组阅读

- 容器总览：`./Dashboard-Diff-Panel.md`
- 文件工作流：`./Dashboard-Diff-FileManager.md`、`./Dashboard-Diff-File-Preview.md`、`./Dashboard-Diff-Options.md`、`./Dashboard-Diff-Convert-Panel.md`
- 结果展示：`./Dashboard-Diff-Stats.md`、`./Dashboard-Diff-Config-Tree.md`、`./Dashboard-Diff-Code-Panel.md`、`./Dashboard-Diff-Line-Panel.md`、`./Dashboard-Diff-Viewer.md`
- 辅助与补注：`./Dashboard-Diff-File-Browser.md`、`./Dashboard-Diff-Relations.md`

## 和其他目录的关系

- 公共壳层与交互组件见 `../shared/README.md`
- 顶层业务面板入口见 `../panels/README.md`
- Env 子系统见 `../env/README.md`

