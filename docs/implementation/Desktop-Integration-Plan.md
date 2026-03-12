# XunYu Desktop 集成方案

> 目标：将 refer/windows 的窗口与系统控制能力融入 XunYu，统一收敛为单一命令域 `xun desktop`，并按现有 CLI 与 Dashboard 规范落地。

## 1. 目标与范围

目标：
1. 形成统一的桌面控制命令域，覆盖窗口、布局、工作区、热键、重映射、片段、主题与 Awake 等能力。
2. 保持 CLI 可脚本化与可预演，遵循 `docs/project/CLI-UI.md` 的交互规范。
3. 与 Dashboard 工作台体系对齐，提供可视化能力接入。
4. 通过独立 feature gate 控制编译体积与依赖。

范围：
1. Window 操作（激活、移动、大小、透明、置顶等）。
2. Layout 模板与应用。
3. Workspaces（保存与启动，按坐标恢复）。
4. Hotkey 全局绑定。
5. Remap 应用级键位重映射。
6. Snippet 文本片段展开。
7. Awake 休眠控制。
8. Theme 主题切换与定时。
9. Color 拾色。
10. Hosts 文件管理。
11. Shell/Run 快捷执行。

排除：
1. Media 与 Volume Control 能力。
2. Env Vars 与 Locksmith 重复实现，分别复用 `env` 与 `lock` 体系。

## 2. 原则与约束

原则：
1. KISS 优先，保持命令语义与数据结构最小化。
2. YAGNI，未明确需要的扩展功能不纳入首期。
3. DRY，复用现有 `src/windows/`、`src/commands/env/`、`src/commands/lock/` 能力。
4. SOLID，领域内聚，Win32 细节下沉。

约束：
1. CLI 交互遵循 `docs/project/CLI-UI.md`，危险动作必须支持 `--dry-run` 与 `--yes`。
2. 输出通道分离：数据到 stdout，提示到 stderr。
3. 非交互优先，非 TTY 时拒绝破坏性操作。
4. Remap/Hook 必须使用专用线程与消息循环，回调内只做轻量处理。
5. Theme 定时仅使用一次性定时器，不允许后台轮询。

## 3. 命令域设计

统一命令域：`xun desktop <resource> <action>`。

示例：
```bash
xun desktop daemon start
xun desktop hotkey bind "ctrl+alt+t" "run:wt.exe"
xun desktop remap add "capslock" "escape"
xun desktop snippet add "//cl" "console.log();" --app "Code.exe"
xun desktop layout new grid --rows 2 --cols 2 --gap 12
xun desktop layout apply dev
xun desktop workspace save dev-env
xun desktop workspace launch dev-env --move-existing
xun desktop theme set dark
xun desktop theme schedule --light 08:30 --dark 20:30
xun desktop awake on --duration 2h --display-on
xun desktop window transparent --alpha 200 --app "Code.exe"
xun desktop hosts add "example.test" "127.0.0.1"
xun desktop color pick
```

## 4. 配置模型

在 `src/config/model.rs` 增加 `desktop` 段，JSON 以 camelCase 输出。

示例配置：
```json
{
  "desktop": {
    "daemon": { "quiet": false, "noTray": false },
    "bindings": [
      { "hotkey": "ctrl+alt+t", "action": "run:wt.exe", "app": "any" }
    ],
    "remaps": [
      { "from": "ctrl+alt+1", "to": "alt+1", "app": "Code.exe", "exact": false }
    ],
    "snippets": [
      { "trigger": "//cl", "expand": "console.log($CLIPBOARD)", "app": "Code.exe", "immediate": true, "paste": "clipboard" }
    ],
    "layouts": [
      { "name": "dev", "template": { "type": "grid", "rows": 2, "cols": 2, "gap": 12 }, "bindings": { "Code.exe": 1 } }
    ],
    "workspaces": [
      { "name": "dev-env", "apps": [ { "path": "wt.exe", "args": "-d D:/proj", "rect": [0, 0, 1280, 720] } ] }
    ],
    "theme": { "followNightlight": true, "scheduleLightAt": "08:30", "scheduleDarkAt": "20:30" },
    "awake": { "defaultDisplayOn": false }
  }
}
```

## 5. 架构与模块布局

新增领域模块：
1. `src/desktop/daemon.rs`
2. `src/desktop/hotkey.rs`
3. `src/desktop/remap.rs`
4. `src/desktop/snippet.rs`
5. `src/desktop/layout.rs`
6. `src/desktop/workspace.rs`
7. `src/desktop/window.rs`
8. `src/desktop/theme.rs`
9. `src/desktop/awake.rs`
10. `src/desktop/color.rs`
11. `src/desktop/hosts.rs`
12. `src/desktop/shell.rs`

Win32 细节下沉到 `src/windows/`，新增或复用对应封装。

## 6. Hotkey System 需求与设计

功能模式：
1. 应用级局部映射（App-Specific Key Remapping）。
2. 全局命令热键（Global Command Execution）。

行为定义：
1. 应用级局部映射在前台进程匹配时生效，拦截输入并向目标应用注入替代按键。
2. 全局命令热键在系统全局侦听，拦截后直接执行预设动作，不进行按键模拟。

优先级规则：
1. 当前前台进程匹配 remap 规则时优先执行 remap，事件被消费，不触发全局命令。
2. 未匹配 remap 时允许触发全局命令。

技术约束：
1. Remap 与 Snippet 共用 `WH_KEYBOARD_LL` 专用线程，回调内仅做最小化判断与 `SendInput`。
2. 全局命令热键优先使用 `RegisterHotKey` 注册，失败则返回明确错误提示。
3. 管理员进程限制必须提示，必要时引导 `xun desktop daemon start --elevated`。

动作类型（action 语义）：
1. `run:<exe>` 启动可执行文件。
2. `shell:<cmd>` 执行 Shell 命令。
3. `uri:<url>` 打开 URI。
4. `theme_toggle`、`awake_toggle`、`layout_apply:<name>` 等内建动作。

## 7. Workspaces 设计要点

1. 保存时记录窗口精确坐标与可选 CLI 参数。
2. 启动时使用 `SW_HIDE -> MoveWindow -> SW_SHOWNORMAL`，避免最小化回归。
3. 不依赖 Snap 元数据，使用坐标恢复保证一致性。

## 8. Layout 设计要点

1. Layout 模板由网格、列、行、主副区等类型构成。
2. 应用时优先根据规则分配窗口，支持 `--move-existing`。
3. CLI 提供 `preview` 以 ASCII 可视化，便于快速校验。

## 9. Snippet 设计要点

1. 文本片段在输入缓冲内匹配触发。
2. 默认 `SendInput` 注入，遇到 Electron/UWP 使用剪贴板注入兜底。
3. `immediate` 控制是否依赖终止键触发。

## 10. Theme 与 Awake

1. Theme 支持手动切换与定时切换。
2. 定时仅使用一次性定时器，不轮询。
3. Awake 通过 `SetThreadExecutionState` 实现，支持定时与到点模式。

## 11. Hosts、Color、Shell

1. Hosts 操作需管理员权限，走 guarded 或明确提示。
2. Color 拾色仅在 Windows 平台启用。
3. Shell/Run 仅做轻量封装，保持参数透明。

## 12. Dashboard 集成

工作台：新增 `Desktop Control` 工作台。

后端接入点：
1. `src/commands/dashboard/handlers/workspaces.rs` 增加 `desktop` capability。
2. 任务走 `run` 或 `guarded`，避免新增专用 API，必要时再扩展。

前端接入点：
1. `dashboard-ui/src/workspace-tools.ts` 增加 desktop 任务组。
2. `dashboard-ui/src/components/workspaces/DesktopWorkspace.vue` 新增工作台组件。
3. `dashboard-ui/src/App.vue` 增加 tab。

## 13. Feature Gate

新增 feature：`desktop`。

策略：
1. 所有 desktop 模块使用 `#[cfg(feature = "desktop")]` 包裹。
2. Windows 相关依赖集中在 feature 内启用，避免默认编译体积膨胀。
3. `dashboard` 若展示 Desktop 工作台，需要同时启用 `desktop`。

## 14. 分阶段实施

Phase 0：基础骨架
1. 新增 `desktop` feature gate 与模块骨架。
2. CLI 命令入口与配置模型落地。

Phase 1：核心桌面能力
1. window、layout、workspace、theme、awake。
2. CLI 基础可用与最小测试。

Phase 2：热键系统
1. daemon、hotkey、remap、snippet。
2. hook 健康检查与自愈提示。

Phase 3：Dashboard
1. Desktop 工作台接入。
2. guarded 任务与审计链路打通。

## 15. 测试与验收

单元测试：
1. 热键解析与 remap 目标解析。
2. 主题定时解析与 awake 时间解析。
3. Workspace 配置序列化与兼容性。

集成验证：
1. `xun desktop daemon start` 后 hotkey、remap、snippet 触发。
2. Workspaces 启动无最小化回归。
3. Dashboard Desktop 工作台可执行 run 与 guarded。

手动检查：
1. Electron/UWP 片段注入可用。
2. 管理员窗口限制提示正确。

## 16. 风险与对策

1. Hook 静默失效。对策：健康检查探针 + 提示重启 daemon。
2. 管理员窗口不可拦截。对策：doctor 提示与可选提权运行。
3. Snap 元数据不可恢复。对策：使用精确坐标恢复。

## 17. 交付物清单

1. 方案文档：`docs/implementation/Desktop-Integration-Plan.md`。
2. CLI 与配置模型调整。
3. `src/desktop/` 领域模块。
4. Dashboard Desktop 工作台。
