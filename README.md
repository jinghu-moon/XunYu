# XunYu (迅羽)

> **XunYu** 是一款专为 Windows 设计、采用 Rust 编写的高性能命令行工具集与本地管理工作台。  
> 旨在通过极致的性能和统一的体验，填补 Windows 环境下路径管理、环境治理、文件安全及系统运维的工具链空白。

[![License](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows-0078d7.svg)](https://www.microsoft.com/windows)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

---

## 🌟 项目愿景

在 Windows 上提供“生产力级”的运维体验：
- **快**：基于 Rust 2024，零运行时开销，毫秒级响应。
- **全**：涵盖从书签跳转到 ACL 深度维护、从视频压缩到环境变量治理的方方面面。
- **稳**：专为 Windows 平台优化，深度集成 Win32 API。
- **美**：内置现代化 Dashboard，支持 Glassmorphism（毛玻璃）特效。

---

## ✨ 核心能力矩阵

XunYu 的功能通过不同的模块和 Cargo Feature 进行组织：

### 1. 核心运维 (Core Operations)
| 命令 | 描述 | 关键特性 |
| :--- | :--- | :--- |
| `xun z` | 书签与快速跳转 | 类似 zoxide 的频率/时间权重路径跳转。 |
| `xun ctx` | 上下文管理 | 为不同项目切换工作环境（环境变量、路径、代理）。 |
| `xun port` | 端口与进程 | 快速查询端口占用 (`ports`) 并支持一键终止 (`kill`)。 |
| `xun proxy` | 网络代理 | 命令行快速开关系统代理，支持延迟测试与环境执行。 |

### 2. 文件与存储系统 (File & Storage)
| 命令 | 描述 | 关键特性 |
| :--- | :--- | :--- |
| `xun xunbak` | 高性能备份 | **.xunbak** 容器设计，支持增量、分卷、压缩与校验。 |
| `xun rm` | 深度清理 | 支持 Windows 保留名文件强制删除、延迟重启删除。 |
| `xun diff` | 差异对比 | 快速对比目录或文件差异，支持可视化 Web 展示。 |
| `xun brn` | 批量重命名 | 支持正则、模板的极速批量文件重命名。 |

### 3. 系统安全与 ACL (Security & ACL)
| 命令 | 描述 | 关键特性 |
| :--- | :--- | :--- |
| `xun acl` | 权限治理 | 可能是 Windows 下最强的 ACL 维护工具：备份、还原、继承修复、孤儿清理。 |
| `xun vault` | 保密柜 | 基于 age 协议的本地加密存储，保护敏感配置。 |
| `xun lock` | 文件锁定 | 查询文件被哪个进程占用，并支持强制解锁。 |
| `xun protect` | 文件保护 | 设置目录为只读或受保护状态，防止意外修改。 |

### 4. Windows 桌面增强 (Desktop & Workflow)
| 命令 | 描述 | 关键特性 |
| :--- | :--- | :--- |
| `xun desktop` | 桌面管理器 | 管理窗口布局、热键映射、系统主题切换。 |
| `xun awake` | 保持唤醒 | 防止 Windows 进入睡眠模式，支持定时与进程关联。 |
| `xun hosts` | Hosts 管理 | 安全、快速地修改 hosts 文件，支持分组与状态切换。 |
| `xun redirect` | 规则引擎 | 重定向文件路径或端口，实现灵活的工作流路由。 |

### 5. 多媒体处理 (Media)
| 命令 | 描述 | 关键特性 |
| :--- | :--- | :--- |
| `xun video` | 视频工作流 | 快速压缩、封装转换、信息分析（依赖 ffmpeg）。 |
| `xun img` | 图像优化 | 支持 mozjpeg/turbojpeg 的极速压缩与格式转换。 |

---

## 🖥️ Dashboard 工作台

启用 `dashboard` feature 后，XunYu 会在二进制中嵌入一个由 **Vite + Vue 3** 驱动的 Web UI。

- **Triple-Guard**：三重安全防护，高风险操作实时拦截。
- **可视化 Diff**：在浏览器中直观查看目录差异。
- **环境看板**：一键切换项目上下文，实时监控系统指标。

> [!TIP]
> 运行 `xun serve` 即可启动工作台（默认端口 8321）。

---

## 🚀 快速开始

### 安装要求
- Windows 10 / 11
- [Rust](https://rustup.rs/) (推荐使用 `stable-x86_64-pc-windows-msvc`)
- Node.js & pnpm (仅当需要从源码构建 Dashboard 时)

### 构建指南
XunYu 提供了灵活的构建组合以平衡产物体积与功能：

```powershell
# 1. 基础版本 (仅包含常用文件与系统操作)
cargo build --release

# 2. 增强运维版 (包含加密、ACL 维护与别名体系)
cargo build --release --features "crypt,acl,alias"

# 3. 图像处理版 (mozjpeg 优化)
cargo build --release --features "img,img-moz"

# 4. 全功能工作台版 (包含 Web UI 与 Diff 可视化)
pnpm -C dashboard-ui install
pnpm -C dashboard-ui build
cargo build --release --features "dashboard,diff,xunbak,desktop"
```

### 命令约定
- 正式命令：`xun`
- 兼容二进制：`xyu`
- 快捷别名：`xy`（推荐在已加载 shell 集成后使用）

运行以下命令将 `xy` 别名自动注入你的 Shell：
```powershell
xun init powershell | Out-String | Invoke-Expression
```

---

## 📦 .xunbak 容器设计

`.xunbak` 是项目内建的高性能备份格式，旨在替代传统的压缩备份方案：
1. **单文件优先**：默认生成单一容器文件，易于管理与归档。
2. **增量更新**：基于 BLAKE3 校验，仅记录变化的文件 Blob。
3. **安全校验**：内建哈希与 CRC32C 双重校验，确保数据完整性。
4. **高性能**：直接集成 zstd 压缩，在保持高压缩比的同时提供极速恢复体验。

详情请参考：[Single-File-Xunbak-Design.md](docs/implementation/Single-File-Xunbak-Design.md)

---

## 📚 文档指南

| 维度 | 文档入口 |
| :--- | :--- |
| **新手入门** | [快速上手](intro/README.md) \| [安装指南](intro/Install.md) |
| **详细手册** | [Feature 索引](intro/Features.md) \| [CLI 详述](intro/cli/README.md) |
| **深度设计** | [架构设计](docs/README.md) \| [命名策略](docs/project/Naming-Strategy.md) |
| **子系统** | [ACL 治理](intro/acl/README.md) \| [环境变量](intro/env/README.md) \| [Redirect 引擎](intro/redirect/README.md) |

---

## 🤝 贡献与许可

- **License**: 本项目采用 [AGPL-3.0-or-later](LICENSE) 许可证。
- **Contributing**: 欢迎提交 Issue 或 Pull Request。请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。

---

*“让 Windows 开发不再是二等公民。”*




