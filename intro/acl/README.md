# ACL 子系统索引

这一组文档对应 `xun acl` 以及底层 ACL 领域实现。它偏向 Windows 权限运维，覆盖查看、编辑、diff、修复、备份恢复与审计。

如果你想从“权限管理子系统”角度理解项目，这里是入口。

## 这一组解决什么问题

- `xun acl` 的命令面覆盖了哪些权限治理能力
- 执行层为什么按 `view`、`edit`、`batch`、`repair`、`audit`、`config` 分组
- 底层 ACL 领域层承担哪些平台能力
- 备份恢复、孤儿 SID 处理和审计为什么被视为一等能力

## 建议阅读顺序

1. `./ACL-Modules.md`
2. `./ACL-Internals.md`

## 文档清单

- ACL 模块总览：`./ACL-Modules.md`
- ACL 内部结构：`./ACL-Internals.md`

## 和其他目录的关系

- CLI 总入口见 `../cli/README.md`
- Env 子系统见 `../env/README.md`
- Redirect 子系统见 `../redirect/README.md`
