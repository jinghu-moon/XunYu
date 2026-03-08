# Env 子系统索引

这一组文档对应 `xun env` 以及 `env_core` 相关内容。它是项目里最完整、最垂直切分的命令子系统之一。

如果你想理解 CLI、Dashboard、TUI 如何共享同一套环境变量领域能力，这里是主入口。

## 这一组解决什么问题

- `xun env` 为什么更像独立产品而不是普通子命令
- `src/cli/env/*`、`src/commands/env/*`、`src/env_core/*` 如何分层
- `EnvManager` 与底层核心能力是怎么收口的
- 哪些能力属于“命令面”，哪些属于“领域面”

## 建议阅读顺序

1. `./Env-Modules.md`
2. `./Env-Core-Internals.md`

## 文档清单

- `env` 模块总览：`./Env-Modules.md`
- `env_core` 内部结构：`./Env-Core-Internals.md`

## 和其他目录的关系

- CLI 总入口见 `../cli/README.md`
- Dashboard Env 工作台见 `../dashboard/env/README.md`
- ACL 子系统见 `../acl/README.md`
- Diff 子系统见 `../diff/README.md`
