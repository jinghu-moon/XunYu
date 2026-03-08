# Diff 子系统索引

这一组文档对应 `xun diff` 以及 Dashboard Diff 工作台相关的底层能力。它把 CLI adapter、底层 diff 引擎、Dashboard handlers 和前端面板串成一条链。

如果你想理解项目里的文件与配置对比能力，这里是主入口。

## 这一组解决什么问题

- `xun diff` 为什么和其他命令一样有独立领域层
- CLI、底层算法、Dashboard 接口和前端工作台如何共享同一套 diff 结果
- 文本 diff、AST diff、配置语义 diff 分别落在哪一层

## 建议阅读顺序

1. `./Diff-Modules.md`
2. `../dashboard/diff/README.md`

## 文档清单

- Diff 模块总览：`./Diff-Modules.md`
- Dashboard Diff 工作台索引：`../dashboard/diff/README.md`

## 和其他目录的关系

- CLI 总入口见 `../cli/README.md`
- Redirect 规则与文件处理见 `../redirect/README.md`
- Dashboard Diff 子系统见 `../dashboard/diff/README.md`
