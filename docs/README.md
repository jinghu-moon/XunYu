# 设计与研发文档索引

`docs/` 目录放的是设计、实施、测试与评审文档；上手与使用说明请看 `../intro/README.md`。

## 总入口

- 项目说明类目录索引：`./project/README.md`
- 实施方案类目录索引：`./implementation/README.md`
- 测试 / 演示夹具：`./fixtures/`

## 项目设计与约定

- CLI 输出与交互约定：`./project/CLI-UI.md`
- Dashboard 设计：`./project/Dashboard-Design.md`
- Dashboard 扩展路线图：`./project/Dashboard-Expansion-Roadmap.md`
- P0 实施方案：`./project/P0-Execution-Plan.md`
- P0 完成说明：`./project/P0-Completion-Notes.md`
- P1 实施方案：`./project/P1-Execution-Plan.md`
- P1 完成说明：`./project/P1-Completion-Notes.md`
- P2 进展说明：`./project/P2-Progress-Notes.md`
- 命名与命令策略：`./project/Naming-Strategy.md`
- 构建矩阵：`./project/Build-Matrix.md`
- 性能 / 定制优化：`./project/Performance-Optimization-Customization.md`
- 图像建议文档：`./project/image-recommand.md`

## EnvMgr 专题

- EnvMgr 集成方案（v4）：`./envmgr-integration-plan.md`
- EnvMgr 分阶段任务清单：`./envmgr-integration-tasks.md`
- EnvMgr 使用手册：`./envmgr-usage.md`
- EnvMgr 手工验收清单：`./envmgr-manual-checklist.md`
- EnvMgr 常见问题：`./envmgr-faq.md`
- EnvMgr 发布说明：`./envmgr-release-notes.md`
- EnvMgr 已知限制：`./envmgr-known-limitations.md`
- EnvMgr Smoke 结果归档：`./envmgr-smoke-report.md`

## 实施方案与任务

- Find 设计：`./implementation/Find-Design.md`
- Find 任务清单：`./implementation/Find-Tasks.md`
- 补全设计：`./implementation/Completion-Design.md`
- 补全清单：`./implementation/Completion-Checklist.md`
- 补全性能基准：`./implementation/Completion-Benchmark.md`
- Context Switch 设计：`./implementation/Context-Switch-Design.md`
- Context Switch 任务清单：`./implementation/Context-Switch-Tasks.md`
- Redirect 设计：`./implementation/Redirect-Design.md`
- 文件解锁 / 保护 / 加密计划：`./implementation/File-Unlock-Protection-Encryption-Plan.md`
- 任务阶段清单：`./implementation/Phase-Tasks.md`
- redirect 分阶段任务：`./implementation/Task2.md`
- Diff 组件实施方案：`./implementation/diff-implementation-plan.md`
- 视频压缩 / 重封装规格：`./implementation/Video-Compress-Remux-Spec.md`
- 拆分重构计划：`./refactor-split-plan.md`

## 开发测试与评审

- 开发与测试入口：`./implementation/Dev-Test.md`
- 测试环境：`./implementation/Test-Env.md`
- 测试清单：`./implementation/Tests-List.md`
- 测试重构：`./implementation/Tests-Refactor.md`
- 评审记录：`./implementation/Review.txt`

## 建议阅读顺序

1. 先看 `./project/README.md`，建立项目设计文档的范围感。
2. 如果你关注 CLI 和 Dashboard 的交互约定，接着看 `./project/CLI-UI.md` 与 `./project/Dashboard-Design.md`。
3. 如果你当前在看 EnvMgr，优先走 `./envmgr-integration-plan.md` → `./envmgr-integration-tasks.md` → `./envmgr-usage.md`。
4. 如果你当前在看功能实施，进入 `./implementation/README.md`，再按专题跳 `Find`、`Completion`、`Context Switch`、`Redirect`、`Diff`。
5. 如果你准备落地或验证实现，再看 `./implementation/Dev-Test.md`、`./implementation/Test-Env.md`、`./implementation/Tests-List.md`。
6. 最后按需查 `./implementation/Review.txt`、`./envmgr-release-notes.md`、`./envmgr-known-limitations.md` 和 `./envmgr-smoke-report.md`。


