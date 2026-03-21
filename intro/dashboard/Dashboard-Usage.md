# Dashboard 使用说明

这份文档面向**实际使用 XunYu Dashboard 的开发者 / 本地运维用户**。

它不讲组件源码，而是回答下面这些问题：

- Dashboard 怎么启动？
- 8 个工作台分别做什么？
- 哪些功能需要额外 feature？
- 危险操作在 UI 里应该怎么走？
- 常见问题怎么排查？

如果你想看设计与架构，请配合阅读：

- `../../docs/project/Dashboard-Design.md`
- `../../docs/project/Dashboard-Expansion-Roadmap.md`
- `./Dashboard-Foundation.md`
- `./Dashboard-Components.md`

## 1. 适用范围

当前 Dashboard 的定位是：**本地优先的 8 工作台控制台**。

它不是远程管理面板，也不是把 CLI 帮助逐条翻译成按钮，而是把高频、可视化、可预览的能力收口到统一工作台中。

适用环境：

- Windows 10 / 11
- Rust stable + MSVC
- Node.js + `pnpm`
- 本地运行，默认地址：`http://127.0.0.1:9527`

## 2. 启动前准备

### 2.1 构建前端资源

Dashboard 由 Rust 后端嵌入前端静态资源，因此在运行 `serve` 之前，先构建前端：

```bash
corepack enable
pnpm -C dashboard-ui install
pnpm -C dashboard-ui build
```

### 2.2 最小可用启动

如果你只需要 Dashboard 基础壳层 + Diff 文件工作台：

```bash
cargo run --features "dashboard,diff" -- serve
```

### 2.3 推荐启动（尽量打开完整工作台能力）

如果你想在 Dashboard 中看到更完整的 8 工作台能力，推荐：

```bash
cargo run --features "dashboard,diff,redirect,alias,lock,protect,crypt,batch_rename,cstat,img" -- serve
```

说明：

- `fs` 是默认 feature，已默认开启。
- `xun` 是正式命令名。
- `xyu` 是兼容命令名。
- `xy` 是快捷别名，前提是你已经通过 shell 初始化把别名注入当前 shell。

### 2.4 自定义端口

如果 `9527` 被占用：

```bash
cargo run --features "dashboard,diff" -- serve -p 9528
```

## 3. 开发模式启动

如果你正在开发 Dashboard 前端，推荐前后端分离运行：

终端 1：

```bash
cargo run --features "dashboard,diff,redirect,alias,lock,protect,crypt,batch_rename,cstat,img" -- serve
```

终端 2：

```bash
pnpm -C dashboard-ui dev
```

访问：

- 前端开发地址：`http://127.0.0.1:5173`
- Vite 会把 `/api/*` 代理到本地 Axum 服务

## 4. Dashboard 顶层结构

当前 Dashboard 已收口为 **8 个工作台**：

| 工作台 | 主要内容 | 典型能力 |
| --- | --- | --- |
| 总览 | 全局摘要、能力入口 | 指标总览、工作台能力可见性 |
| 路径与上下文 | 书签与上下文切换 | Bookmarks、`ctx`、`recent`、`stats`、`gc`、`dedup` |
| 网络与代理 | 端口、进程、代理 | `ports`、`ps`、`pkill`、`kill`、`px`、Proxy 状态 |
| 环境与配置 | 环境变量与全局配置 | Config、Env 工作台 |
| 文件与安全 | 文件浏览、规则、保护、安全治理 | Diff、Redirect、`tree`、`find`、`backup`、`restore`、`rm`、`protect`、`acl`、`encrypt` |
| 集成与自动化 | shell 集成与工具化动作 | `init`、`completion`、`__complete`、`alias`、`brn` |
| 媒体与转换 | 图片和视频处理 | `img`、`video probe`、`video compress`、`video remux` |
| 统计与诊断 | 诊断、任务复盘与统计分析 | Diagnostics Center、Recent Tasks、Recipe、Audit、`cstat` |

## 5. 顶层交互约定

### 5.1 一级导航

顶部 `CapsuleTabs` 用于切换 8 个工作台。

这层是整个 Dashboard 的一级信息架构，不建议把 CLI 子命令再重新膨胀成一级页。

### 5.2 命令面板

按 `Ctrl + K` 或 `Cmd + K` 打开全局命令面板。

可用于：

- 快速跳转工作台
- 重载当前 Dashboard
- 后续扩展更多全局动作

### 5.3 主题与密度

右上角提供：

- 主题切换：`system / light / dark`
- 密度切换：紧凑 / 宽松

这两者只影响 UI 呈现，不改变 CLI 或后端行为。

### 5.4 全局反馈

所有 API 请求错误、未处理异常和任务失败都会统一进入全局反馈区。

如果某个任务失败，优先看：

- 顶部 / 全局 toast
- 任务卡片内的命令输出
- 危险动作的回执卡片

### 5.5 工作台联动

当前 Dashboard 已统一采用一套跨工作台跳转约定：

- `recent-tasks`：优先在当前工作台内聚焦本地最近任务
- `audit`：自动切到“统计与诊断”，并聚焦审计筛选结果
- `diagnostics-center`：自动切到“统计与诊断”，并聚焦对应诊断面板

目前这套联动已经覆盖：

- 路径与上下文
- 网络与代理
- 文件与安全
- 集成与自动化
- 媒体与转换

这意味着你在业务工作台里执行任务后，通常不需要手工切页去二次筛选；本地闭环和全局诊断入口已经打通。

## 6. 危险操作的使用方式

Dashboard 中的高风险动作统一遵循 **Triple-Guard**：

1. **Preview / Dry-run**
2. **User Confirm**
3. **Receipt + Audit**

也就是说，像下面这些动作不会直接执行：

- 删除文件
- 结束进程 / 释放端口
- 清理死链书签
- 批量重命名落盘
- 恢复备份
- 设置 / 清除保护规则
- 新增 ACL
- 加密 / 解密

在 UI 中，正确使用姿势是：

1. 先填写任务卡参数。
2. 点击“预览并确认”。
3. 检查弹窗中的 preview 输出与目标信息。
4. 确认后执行。
5. 查看回执卡片中的执行结果与审计动作。

### 6.1 一个重要约定

在新的工作台里，危险动作应该优先从**任务卡**进入，而不是从旧面板里的直达按钮进入。

例如：

- 网络与代理工作台中的 `PortsPanel` 在工作台模式下会关闭直接 kill 按钮。
- 真正的结束进程 / 释放端口，应通过统一 guarded 任务卡执行。

这样可以避免跳过 preview 与 receipt。

## 7. 各工作台使用建议

### 7.1 总览

适合做什么：

- 看当前系统摘要
- 确认 Dashboard 编译后哪些 feature 已开启
- 作为进入其它工作台前的总览页

如果你只是想快速判断：

- 书签量
- 端口占用规模
- Env 变量规模
- 审计积累情况

先看这里。

### 7.2 路径与上下文

适合做什么：

- 管理书签
- 切换上下文配置
- 查看最近访问 / 统计 / 健康检查
- 对书签做死链清理与去重

推荐工作流：

1. 先在 `BookmarksPanel` 做日常浏览与编辑。
2. 再在任务卡里做 `ctx`、`stats`、`gc`、`dedup` 之类偏工作流动作。
3. 危险动作（如清理、去重）优先看 preview 输出。

### 7.3 网络与代理

适合做什么：

- 查看端口占用与代理状态
- 查询本地进程
- 在代理环境下执行命令
- 有确认链路地结束进程或释放端口

推荐工作流：

1. 用 `PortsPanel` 观察。
2. 用 `ps` 任务卡定位目标。
3. 用 `pkill` 或 `kill` 任务卡做带确认的处理。
4. 代理相关状态与修改仍然主要通过 `ProxyPanel` 进行。

### 7.4 环境与配置

适合做什么：

- 查看和编辑配置
- 管理环境变量
- 做 PATH、快照、Schema、Profile、Doctor、Diff、Audit 等完整 Env 工作流

这是当前 Dashboard 中最成熟的复合工作台之一。

如果你在处理：

- PATH 治理
- 环境导入导出
- 快照与回滚
- 环境诊断

优先在这里完成。

### 7.5 文件与安全

适合做什么：

- 目录树 / 文件查找
- 文件预览 / Diff / 校验 / 转换
- Redirect 规则 dry-run
- 备份创建与恢复
- 文件删除、移动、重命名
- 保护、ACL、加解密治理

推荐工作流：

1. 先用 `DiffPanel` / `RedirectPanel` 看结构、规则、差异。
2. 如果要对比 ACL，可先把一个文件设为 `ACL 参考`，再切到目标文件同步 `acl:diff / acl:copy`。
3. 对 `acl:add / acl:purge / acl:owner / acl:inherit` 这类高风险治理，preview 会直接显示“当前 ACL vs 预期状态”的结构化差异，确认前先看这一层。
4. 再用 `tree` / `find` / `backup` / `restore` 等任务卡进入工作流。
5. 真正落地的文件变更，一律看 preview 和 receipt；如果 receipt 仍显示 `ACL 差异明细` 未对齐，优先回到任务参数排查。
6. 批量治理在确认弹窗里也会逐项显示结构化治理摘要；确认前先核对每条路径的预期变更，再执行。
7. 批量治理完成后，预演项与执行回执都可直接从批量面板跳到最近任务、审计或诊断中心，不必手工重新筛选。

这是目前最接近“本地运维控制台”的工作台。

### 7.6 集成与自动化

适合做什么：

- 生成 shell 初始化脚本
- 生成补全脚本
- 调试内部补全行为
- 管理 alias
- 批量重命名

推荐工作流：

- 新机器 / 新 shell：先跑 `init` 和 `completion`
- 日常工具链整理：用 `alias` 任务卡
- 批量改名：先 dry-run，再执行 `brn`

### 7.7 媒体与转换

适合做什么：

- 图片压缩 / 转换
- 视频探测
- 视频压缩
- 无损 remux

建议先从小样本文件试跑，确认输出目录和参数正确后，再处理更大批量内容。

### 7.8 统计与诊断

适合做什么：

- 先看诊断中心里的 doctor 总览、最近失败任务、高风险动作回执和审计时间线
- 在任务中心里回看最近成功 / 失败 / Dry Run，并按安全规则重放任务
- 用 Recipe 固化高频顺序工作流，并继续遵循 Preview -> Confirm -> Receipt
- 做代码 / 目录统计
- 扫描空文件、大文件、重复文件、临时文件

推荐用法：

1. 先打开 `DiagnosticsCenterPanel`，判断当前最值得处理的问题。
2. 再看 `RecentTasksPanel`，复盘失败任务或重放最近操作。
3. 如果某个流程会重复出现，就在 `RecipePanel` 中预演、确认执行并保存本地副本。
4. 最后再配合 `AuditPanel` 和 `cstat` 任务卡做更细的审计与统计分析。

另外，从“路径与上下文”“集成与自动化”“媒体与转换”“文件与安全”等工作台发出的审计 / 诊断跳转，也都会自动落到这里。

这个工作台现在不只是“审计页”，而是 P0-P5 全阶段收口后的全局诊断与任务复盘入口。

## 8. feature 可见性说明

Dashboard 会根据当前二进制实际编译出的 feature 显示 / 隐藏能力。

如果任务卡显示“当前构建未启用该 feature”，说明不是 UI 坏了，而是当前二进制没有打开对应 feature。

常见对应关系：

| 能力 | 相关 feature |
| --- | --- |
| Dashboard 本体 | `dashboard` |
| Diff 文件工作台 | `diff` |
| Redirect | `redirect` |
| Alias | `alias` |
| 锁 / 移动 / 重命名 | `lock` |
| 保护规则 | `protect` |
| 加解密 | `crypt` |
| 批量重命名 | `batch_rename` |
| 目录统计 | `cstat` |
| 图像转换 | `img` |

如果你想尽量体验完整工作台，优先使用推荐启动命令。

## 9. 常见问题

### 9.1 页面能打开，但很多功能卡片不可用

原因通常是 feature 没开全。

处理方式：

```bash
cargo run --features "dashboard,diff,redirect,alias,lock,protect,crypt,batch_rename,cstat,img" -- serve
```

### 9.2 `serve` 能启动，但页面空白或资源 404

通常是前端 `dist/` 没生成或过期。

处理方式：

```bash
pnpm -C dashboard-ui install
pnpm -C dashboard-ui build
cargo run --features "dashboard,diff" -- serve
```

### 9.3 前端开发模式下 API 请求失败

确认：

- Axum 后端是否已启动
- `http://127.0.0.1:9527` 是否可访问
- 是否使用了 `pnpm -C dashboard-ui dev`

### 9.4 危险动作为什么一定要多点一步确认

这是设计要求，不是多余流程。

原因是 Dashboard 中的很多动作都可能直接影响：

- 文件系统
- 进程状态
- 权限与保护规则
- 备份 / 恢复结果

因此必须保留 preview、confirm、receipt 与 audit 链路。

### 9.5 为什么有些能力仍保留在旧面板里，有些在任务卡里

原则是：

- **状态型 / 观察型能力**：保留在原面板中，例如总览、书签浏览、代理状态、Diff 预览。
- **工作流型 / 任务型能力**：收口进任务卡，例如 `ctx`、`gc`、`rm`、`brn`、`cstat`。
- **危险动作**：统一优先走 guarded 任务卡。

## 10. 推荐使用顺序

如果你第一次真正使用新的 Dashboard，推荐顺序：

1. 先完成前端构建并启动 `serve`
2. 打开总览，确认 feature 能力可见
3. 进入“统计与诊断”，先看诊断中心、最近任务和 Recipe 工作流入口
4. 进入“路径与上下文”，熟悉书签与 `ctx`
5. 进入“环境与配置”，熟悉 Env 工作台
6. 进入“文件与安全”，练习 preview / confirm / receipt 流程
7. 高频流程再回到“统计与诊断”，把它固化成 Recipe
8. 最后再用“集成与自动化”“媒体与转换”补齐外围能力

## 11. 相关文档

- 架构与设计：`../../docs/project/Dashboard-Design.md`
- 扩展路线图：`../../docs/project/Dashboard-Expansion-Roadmap.md`
- 组件地图：`./Dashboard-Components.md`
- 基础骨架：`./Dashboard-Foundation.md`
- Env 子系统索引：`./env/README.md`
- Diff 子系统索引：`./diff/README.md`
- 业务面板索引：`./panels/README.md`
- 共享组件索引：`./shared/README.md`


