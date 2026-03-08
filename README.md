# XunYu

> A Windows-first Rust CLI and local dashboard for paths, proxies, envs, and file workflows.  
> 一个面向 Windows 的 Rust CLI 与本地 Dashboard，覆盖路径、代理、环境变量和文件工作流。

`XunYu` 进一步覆盖路径跳转、书签管理、端口查询与终止、备份、文件搜索、删除、Env 治理、ACL 维护、Redirect 规则引擎、Diff 对比等本地开发与运维能力。

> 当前命令约定：正式命令是 `xun`；`xyu` 是已提供的兼容命令；`xy` 是推荐快捷别名，并会由 `xun init powershell|bash|zsh` 自动注入。
>
> 快速查参数：`xun --help` 或 `xun <cmd> --help`

## 命名与命令约定

- 项目名：`XunYu`
- 正式命令名：`xun`
- 可接受备选：`xyu`（已提供兼容二进制入口）
- 快捷别名：`xy`（推荐在已加载 shell 集成后使用，不作为主命令宣传）
- 详细说明：`docs/project/Naming-Strategy.md`

## 许可证

- 本项目采用 `GNU Affero General Public License v3.0 or later`（`AGPL-3.0-or-later`）。
- 仓库根目录 `LICENSE` 保存的是 GNU AGPL v3 官方许可证正文；项目口径额外明确允许按“v3 或更高版本”使用，两者不冲突。

## 主要能力

- 默认 CLI 能力：书签、`config`、`ctx`、`proxy`、`ports/kill/ps/pkill`、`bak`、`tree`、`find`、`delete/del`、`rm`、`env`、`video`、`acl`
- 可选 feature：`alias`、`lock`、`protect`、`crypt`、`redirect`、`dashboard`、`diff`、`batch_rename`、`cstat`、`img`
- Dashboard 工作台：总览、路径与上下文、网络与代理、环境与配置、文件与安全、集成与自动化、媒体与转换、统计与诊断；若启用 `diff`，文件浏览、转换、校验与 Diff 可视化能力会收口到“文件与安全”工作台

- Dashboard P0 收口：已统一 Triple-Guard、高风险回执、最近任务安全重放、Recipe MVP 与诊断中心 MVP

## 环境要求

- Windows 10 / 11
- Rust stable（MSVC 工具链）
- 如果需要构建 Dashboard：Node.js + `pnpm`

## 快速开始

```bash
git clone <your-repo-url>
cd Xun

# 基础能力（默认）
cargo build --release

# Dashboard + Diff（推荐查看 Web UI 时使用）
cargo build --release --features "dashboard,diff"

# 全功能
cargo build --release --all-features
```

构建产物：`target/release/xun.exe`、`target/release/xyu.exe`

## 常用构建组合

- 只用 CLI：`cargo build --release`
- 文件运维增强：`cargo build --release --features "lock,protect,crypt,redirect"`
- Dashboard + Diff：`cargo build --release --features "dashboard,diff"`
- Alias 体系：`cargo build --release --features "alias,alias-shell-extra"`
- 图像处理（mozjpeg）：`cargo build --release --features "img,img-moz"`
- 图像处理（turbo）：`cargo build --release --features "img,img-turbo"`

## Dashboard 前端构建

`dashboard` feature 会把 `dashboard-ui/dist/` 静态资源嵌入到 Rust 二进制中。因此从源码构建 Dashboard 时，请先生成前端产物：

```bash
corepack enable
pnpm -C dashboard-ui install
pnpm -C dashboard-ui build
cargo build --release --features "dashboard,diff"
```

## 快速示例

```bash
# 删除书签（别名）
xun delete -bm <name>

# 删除 Windows 保留名文件（默认只匹配保留名）
xun delete <path>

# 允许删除任意文件名
xun delete --any <path>

# 模拟运行（不实际删除）
xun delete --any --what-if <path>

# 删除失败后登记重启删除
xun delete --any --on-reboot <path>
```

## 文档入口

- 上手 / 使用入口：`intro/README.md`
- 安装与构建：`intro/Install.md`
- 功能与 feature 索引：`intro/Features.md`
- CLI 入口：`intro/cli/README.md`
- Env 子系统：`intro/env/README.md`
- ACL 子系统：`intro/acl/README.md`
- Redirect 子系统：`intro/redirect/README.md`
- Diff 子系统：`intro/diff/README.md`
- Dashboard 使用说明：`intro/dashboard/Dashboard-Usage.md`
- Dashboard 组件导读：`intro/dashboard/Dashboard-Components.md`
- Dashboard 共享 / 面板 / Env / Diff 索引：`intro/dashboard/shared/README.md`、`intro/dashboard/panels/README.md`、`intro/dashboard/env/README.md`、`intro/dashboard/diff/README.md`
- 设计与研发文档：`docs/README.md`
- 命名与命令策略：`docs/project/Naming-Strategy.md`
- 构建矩阵：`docs/project/Build-Matrix.md`
- Dashboard 扩展路线图：`docs/project/Dashboard-Expansion-Roadmap.md`

## 仓库导航

- `src/`：CLI 定义、命令实现、底层子系统与 Windows 封装
- `dashboard-ui/`：Vite + Vue 3 前端工作台
- `intro/`：上手、使用、单模块 / 单组件导读
- `docs/`：设计、实施、测试与评审文档
- `tools/`：辅助脚本与生成工具

## 说明

- 本项目以 Windows 为主要目标平台。
- 很多能力通过 Cargo feature 按需启用，请优先参考 `intro/Features.md`。
- 当前命名策略已落地：默认继续使用 `xun`；同时提供 `xyu` 兼容入口，并在 `xun init powershell|bash|zsh` 中注入 `xy` 快捷别名。
- 如果你想从“理解项目”角度入手，最推荐的顺序是：`intro/README.md` → `docs/README.md` → 相应子目录 `README.md`。



