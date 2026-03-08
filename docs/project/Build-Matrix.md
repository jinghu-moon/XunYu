# XunYu 构建矩阵

这份文档用于固化 `XunYu` 的常用构建组合、适用场景、前置条件与最小验证命令，减少后续手工试错。

## 适用范围

- 目标平台：Windows 10 / 11
- Rust 工具链：stable + MSVC
- 默认产物：`xun.exe`、`xyu.exe`
- 快捷别名：`xy` 由 `xun init powershell|bash|zsh` 注入，不单独构建二进制

## 前置条件

### 通用

- 已安装 Rust stable
- 当前仓库可直接执行 `cargo build`

### Dashboard / Diff

- 已安装 Node.js 与 `pnpm`
- `dashboard-ui/dist/` 已生成
- 推荐先执行：

```bash
corepack enable
pnpm -C dashboard-ui install
pnpm -C dashboard-ui build
```

### TUI

- 需要启用 `tui` feature
- 在 Windows 终端环境中运行体验更完整

## 构建矩阵

| 组合 | 命令 | 适用场景 | 说明 |
| --- | --- | --- | --- |
| 默认 CLI | `cargo build --bin xun --bin xyu` | 日常开发、默认能力验证 | 不带额外 feature，构建速度最快 |
| 默认 CLI（release） | `cargo build --release` | 发布基础 CLI | 生成 `target/release/xun.exe` 与 `target/release/xyu.exe` |
| Dashboard + Diff | `cargo build --bin xun --bin xyu --features "dashboard,diff"` | Web Dashboard、本地文件浏览与 Diff 能力 | 依赖已生成的 `dashboard-ui/dist/` |
| TUI | `cargo build --bin xun --bin xyu --features "tui"` | Env TUI、终端交互场景 | 仅在启用 `tui` 时编译相关入口 |
| Alias 增强 | `cargo build --release --features "alias,alias-shell-extra"` | shell alias / shim 体系 | 适合命令别名相关开发 |
| 文件运维增强 | `cargo build --release --features "lock,protect,crypt,redirect"` | 锁、保护、加密、重定向场景 | 偏运维/文件治理 |
| 图像处理（mozjpeg） | `cargo build --release --features "img,img-moz"` | 图像压缩与处理 | 使用 mozjpeg 路径 |
| 图像处理（turbo） | `cargo build --release --features "img,img-turbo"` | 图像压缩与处理 | 使用 turbo 路径 |
| 全 feature 扫描 | `cargo check --all-features` | 全量接口与 feature 兼容性检查 | 推荐用于快速发现编译层问题 |

## 推荐验证顺序

1. 默认 CLI：确认主工程能干净构建。
2. `dashboard,diff`：确认 Web 侧 feature 组合可用。
3. `tui`：确认终端交互层不被默认构建改动误伤。
4. `--all-features`：做一次全量编译检查。
5. 关键回归测试：至少执行命名与命令入口相关测试。

推荐命令：

```bash
cargo build --bin xun --bin xyu
cargo build --bin xun --bin xyu --features "dashboard,diff"
cargo build --bin xun --bin xyu --features "tui"
cargo check --all-features
cargo test --test test_naming_commands
```

## 当前已验证结果

以下组合已在 **2026-03-08** 验证通过：

- `cargo build --bin xun --bin xyu`
- `cargo build --bin xun --bin xyu --features "dashboard,diff"`
- `cargo build --bin xun --bin xyu --features "tui"`
- `cargo check --all-features`
- `cargo test --test test_naming_commands`

## 产物与命名约定

- 正式命令：`xun`
- 兼容命令：`xyu`
- 快捷别名：`xy`
- release 产物位置：`target/release/xun.exe`、`target/release/xyu.exe`

## 维护建议

- 新增 feature 后，优先把对应组合追加到本矩阵。
- 如果某个组合需要额外前置资源，必须在文档里显式写出。
- 如果构建命令发生变化，需同步更新 `README.md` 与本文件。
