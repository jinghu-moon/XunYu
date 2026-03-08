# Redirect 子系统索引

这一组文档对应 `xun redirect` 及其规则引擎。它不是简单的移动文件命令，而是一个支持 profile、plan/apply、undo、watch、dry-run 和审计的分类子系统。

如果你想从“规则驱动文件归档”角度理解项目，这里是入口。

## 这一组解决什么问题

- `redirect` 为什么是模式路由器而不是单次执行命令
- CLI 定义层、模式层、引擎层、监听层如何拆分
- `plan/apply`、`undo`、`watch`、`dry-run` 这些能力怎么组合成完整工作流

## 建议阅读顺序

1. `./Redirect-Modules.md`
2. `../dashboard/panels/Dashboard-Redirect-Panel.md`

## 文档清单

- Redirect 模块总览：`./Redirect-Modules.md`
- Dashboard Redirect 工作台：`../dashboard/panels/Dashboard-Redirect-Panel.md`

## 和其他目录的关系

- CLI 总入口见 `../cli/README.md`
- Diff 子系统见 `../diff/README.md`
- Dashboard 业务工作台见 `../dashboard/panels/README.md`
