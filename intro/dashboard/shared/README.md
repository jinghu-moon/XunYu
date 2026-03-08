# Dashboard 共享组件与 UI 工具索引

这一组文档对应 `dashboard-ui/src/components` 中偏壳层、导航、反馈和通用 UI 的组件，以及 `dashboard-ui/src/ui` 中被多个面板复用的前端工具函数。

如果你已经看过 `../Dashboard-Foundation.md`，这里可以继续往下读“公共组件 / UI 工具层”；如果你还没看过，建议先看 `../Dashboard-Components.md` 和 `../Dashboard-Foundation.md`。

## 这一组解决什么问题

- 顶层导航如何组织
- 全局命令面板如何唤起和执行
- 主题与密度如何持久化
- 全局 loading / toast 如何统一出口
- 通用按钮与骨架屏如何复用
- CSV / JSON 导出、标签着色等 UI 工具如何被多个面板共享

## 建议阅读顺序

1. `./Dashboard-Capsule-Tabs.md`
2. `./Dashboard-Command-Palette.md`
3. `./Dashboard-Density-Toggle.md`
4. `./Dashboard-Theme-Toggle.md`
5. `./Dashboard-Global-Feedback.md`
6. `./Dashboard-Feedback-Store.md`
7. `./Dashboard-Export-Utils.md`
8. `./Dashboard-Tag-Utils.md`
9. `./Dashboard-Skeleton-Table.md`
10. `./Dashboard-Button.md`

## 文档清单

- 导航壳：`./Dashboard-Capsule-Tabs.md`
- 全局命令面板：`./Dashboard-Command-Palette.md`
- 密度切换：`./Dashboard-Density-Toggle.md`
- 主题切换：`./Dashboard-Theme-Toggle.md`
- 全局反馈渲染层：`./Dashboard-Global-Feedback.md`
- 反馈状态源：`./Dashboard-Feedback-Store.md`
- 导出工具：`./Dashboard-Export-Utils.md`
- 标签分类工具：`./Dashboard-Tag-Utils.md`
- 通用骨架屏：`./Dashboard-Skeleton-Table.md`
- 通用按钮：`./Dashboard-Button.md`

## 和其它目录的关系

- 一级业务工作台见 `../panels/README.md`
- Env 子系统见 `../env/README.md`
- Diff 子系统见 `../diff/README.md`
- 整体壳层与 API / 类型边界见 `../Dashboard-Foundation.md`
