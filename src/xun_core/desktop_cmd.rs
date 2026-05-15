//! Desktop CLI 定义（clap derive）
//!
//! 新架构的 desktop 命令定义，替代 argh 版本。
//! 14 个顶层子命令，多个嵌套子命令组。

use clap::{Parser, Subcommand};

// ── Desktop 主命令 ───────────────────────────────────────────────

/// Desktop control commands.
#[derive(Parser, Debug, Clone)]
#[command(name = "desktop", about = "Desktop control commands")]
pub struct DesktopCmd {
    #[command(subcommand)]
    pub cmd: DesktopSubCommand,
}

/// Desktop 子命令枚举（14 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopSubCommand {
    /// Manage desktop daemon
    Daemon(DesktopDaemonCmd),
    /// Manage hotkey bindings
    Hotkey(DesktopHotkeyCmd),
    /// Manage key remaps
    Remap(DesktopRemapCmd),
    /// Manage snippets
    Snippet(DesktopSnippetCmd),
    /// Manage layouts
    Layout(DesktopLayoutCmd),
    /// Manage workspaces
    Workspace(DesktopWorkspaceCmd),
    /// Manage windows
    Window(DesktopWindowCmd),
    /// Manage theme
    Theme(DesktopThemeCmd),
    /// Manage awake mode
    Awake(DesktopAwakeCmd),
    /// Pick a color
    Color(DesktopColorCmd),
    /// Manage hosts file
    Hosts(DesktopHostsCmd),
    /// Manage installed apps
    App(DesktopAppCmd),
    /// Launch desktop TUI
    Tui(DesktopTuiCmd),
    /// Run a command
    Run(DesktopRunCmd),
}

// ── Daemon 嵌套子命令 ────────────────────────────────────────────

/// Manage desktop daemon.
#[derive(Parser, Debug, Clone)]
pub struct DesktopDaemonCmd {
    #[command(subcommand)]
    pub cmd: DesktopDaemonSubCommand,
}

/// Daemon 子命令枚举（4 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopDaemonSubCommand {
    /// Start desktop daemon
    Start(DesktopDaemonStartCmd),
    /// Stop desktop daemon
    Stop(DesktopDaemonStopCmd),
    /// Show desktop daemon status
    Status(DesktopDaemonStatusCmd),
    /// Reload desktop daemon config
    Reload(DesktopDaemonReloadCmd),
}

/// Start desktop daemon.
#[derive(Parser, Debug, Clone)]
pub struct DesktopDaemonStartCmd {
    /// suppress UI output
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// disable tray icon
    #[arg(long)]
    pub no_tray: bool,

    /// run elevated if needed
    #[arg(long)]
    pub elevated: bool,
}

/// Stop desktop daemon.
#[derive(Parser, Debug, Clone)]
pub struct DesktopDaemonStopCmd {}

/// Show desktop daemon status.
#[derive(Parser, Debug, Clone)]
pub struct DesktopDaemonStatusCmd {}

/// Reload desktop daemon config.
#[derive(Parser, Debug, Clone)]
pub struct DesktopDaemonReloadCmd {}

// ── Hotkey 嵌套子命令 ────────────────────────────────────────────

/// Manage hotkey bindings.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHotkeyCmd {
    #[command(subcommand)]
    pub cmd: DesktopHotkeySubCommand,
}

/// Hotkey 子命令枚举（3 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopHotkeySubCommand {
    /// Bind a global hotkey to an action
    Bind(DesktopHotkeyBindCmd),
    /// Unbind a hotkey
    Unbind(DesktopHotkeyUnbindCmd),
    /// List hotkey bindings
    List(DesktopHotkeyListCmd),
}

/// Bind a global hotkey to an action.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHotkeyBindCmd {
    /// hotkey string, e.g. ctrl+alt+t
    pub hotkey: String,

    /// action, e.g. run:wt.exe
    pub action: String,

    /// app filter
    #[arg(long)]
    pub app: Option<String>,
}

/// Unbind a hotkey.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHotkeyUnbindCmd {
    /// hotkey string
    pub hotkey: String,
}

/// List hotkey bindings.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHotkeyListCmd {}

// ── Remap 嵌套子命令 ─────────────────────────────────────────────

/// Manage key remaps.
#[derive(Parser, Debug, Clone)]
pub struct DesktopRemapCmd {
    #[command(subcommand)]
    pub cmd: DesktopRemapSubCommand,
}

/// Remap 子命令枚举（4 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopRemapSubCommand {
    /// Add a remap rule
    Add(DesktopRemapAddCmd),
    /// Remove a remap rule
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopRemapRemoveCmd),
    /// List remap rules
    List(DesktopRemapListCmd),
    /// Clear remap rules
    Clear(DesktopRemapClearCmd),
}

/// Add a remap rule.
#[derive(Parser, Debug, Clone)]
pub struct DesktopRemapAddCmd {
    /// from hotkey
    pub from: String,

    /// to target
    pub to: String,

    /// app filter
    #[arg(long)]
    pub app: Option<String>,

    /// match exact app name
    #[arg(long)]
    pub exact: bool,

    /// dry run
    #[arg(long)]
    pub dry_run: bool,
}

/// Remove a remap rule.
#[derive(Parser, Debug, Clone)]
pub struct DesktopRemapRemoveCmd {
    /// from hotkey
    pub from: String,

    /// to target
    pub to: Option<String>,

    /// dry run
    #[arg(long)]
    pub dry_run: bool,
}

/// List remap rules.
#[derive(Parser, Debug, Clone)]
pub struct DesktopRemapListCmd {}

/// Clear remap rules.
#[derive(Parser, Debug, Clone)]
pub struct DesktopRemapClearCmd {
    /// dry run
    #[arg(long)]
    pub dry_run: bool,
}

// ── Snippet 嵌套子命令 ───────────────────────────────────────────

/// Manage snippets.
#[derive(Parser, Debug, Clone)]
pub struct DesktopSnippetCmd {
    #[command(subcommand)]
    pub cmd: DesktopSnippetSubCommand,
}

/// Snippet 子命令枚举（4 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopSnippetSubCommand {
    /// Add a snippet
    Add(DesktopSnippetAddCmd),
    /// Remove a snippet
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopSnippetRemoveCmd),
    /// List snippets
    List(DesktopSnippetListCmd),
    /// Clear snippets
    Clear(DesktopSnippetClearCmd),
}

/// Add a snippet.
#[derive(Parser, Debug, Clone)]
pub struct DesktopSnippetAddCmd {
    /// trigger text
    pub trigger: String,

    /// expansion text
    pub expand: String,

    /// app filter
    #[arg(long)]
    pub app: Option<String>,

    /// trigger immediately
    #[arg(long)]
    pub immediate: bool,

    /// paste via clipboard
    #[arg(long)]
    pub clipboard: bool,
}

/// Remove a snippet.
#[derive(Parser, Debug, Clone)]
pub struct DesktopSnippetRemoveCmd {
    /// trigger text
    pub trigger: String,
}

/// List snippets.
#[derive(Parser, Debug, Clone)]
pub struct DesktopSnippetListCmd {}

/// Clear snippets.
#[derive(Parser, Debug, Clone)]
pub struct DesktopSnippetClearCmd {}

// ── Layout 嵌套子命令 ────────────────────────────────────────────

/// Manage layouts.
#[derive(Parser, Debug, Clone)]
pub struct DesktopLayoutCmd {
    #[command(subcommand)]
    pub cmd: DesktopLayoutSubCommand,
}

/// Layout 子命令枚举（5 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopLayoutSubCommand {
    /// Create a layout template
    #[command(name = "add", alias = "new")]
    Add(DesktopLayoutNewCmd),
    /// Apply a layout
    Apply(DesktopLayoutApplyCmd),
    /// Preview a layout
    Preview(DesktopLayoutPreviewCmd),
    /// List layouts
    List(DesktopLayoutListCmd),
    /// Remove a layout
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopLayoutRemoveCmd),
}

/// Create a layout template.
#[derive(Parser, Debug, Clone)]
pub struct DesktopLayoutNewCmd {
    /// layout name
    pub name: String,

    /// layout type
    #[arg(short = 't', long)]
    pub layout_type: String,

    /// rows count
    #[arg(long)]
    pub rows: Option<u32>,

    /// cols count
    #[arg(long)]
    pub cols: Option<u32>,

    /// gap size
    #[arg(long)]
    pub gap: Option<u32>,
}

/// Apply a layout.
#[derive(Parser, Debug, Clone)]
pub struct DesktopLayoutApplyCmd {
    /// layout name
    pub name: String,

    /// move existing windows
    #[arg(long)]
    pub move_existing: bool,
}

/// Preview a layout.
#[derive(Parser, Debug, Clone)]
pub struct DesktopLayoutPreviewCmd {
    /// layout name
    pub name: String,
}

/// List layouts.
#[derive(Parser, Debug, Clone)]
pub struct DesktopLayoutListCmd {}

/// Remove a layout.
#[derive(Parser, Debug, Clone)]
pub struct DesktopLayoutRemoveCmd {
    /// layout name
    pub name: String,
}

// ── Workspace 嵌套子命令 ─────────────────────────────────────────

/// Manage workspaces.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWorkspaceCmd {
    #[command(subcommand)]
    pub cmd: DesktopWorkspaceSubCommand,
}

/// Workspace 子命令枚举（4 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopWorkspaceSubCommand {
    /// Save current workspace
    Save(DesktopWorkspaceSaveCmd),
    /// Launch a workspace
    Launch(DesktopWorkspaceLaunchCmd),
    /// List workspaces
    List(DesktopWorkspaceListCmd),
    /// Remove a workspace
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopWorkspaceRemoveCmd),
}

/// Save current workspace.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWorkspaceSaveCmd {
    /// workspace name
    pub name: String,

    /// record name only
    #[arg(long)]
    pub name_only: bool,
}

/// Launch a workspace.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWorkspaceLaunchCmd {
    /// workspace name
    pub name: String,

    /// move existing windows
    #[arg(long)]
    pub move_existing: bool,

    /// monitor offset
    #[arg(long)]
    pub monitor_offset: Option<i32>,
}

/// List workspaces.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWorkspaceListCmd {}

/// Remove a workspace.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWorkspaceRemoveCmd {
    /// workspace name
    pub name: String,
}

// ── Window 嵌套子命令 ────────────────────────────────────────────

/// Manage windows.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWindowCmd {
    #[command(subcommand)]
    pub cmd: DesktopWindowSubCommand,
}

/// Window 子命令枚举（5 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopWindowSubCommand {
    /// Focus a window
    Focus(DesktopWindowFocusCmd),
    /// Move a window
    Move(DesktopWindowMoveCmd),
    /// Resize a window
    Resize(DesktopWindowResizeCmd),
    /// Set window transparency
    Transparent(DesktopWindowTransparentCmd),
    /// Toggle window always-on-top
    Top(DesktopWindowTopCmd),
}

/// Focus a window.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWindowFocusCmd {
    /// app name
    #[arg(long)]
    pub app: Option<String>,

    /// window title
    #[arg(long)]
    pub title: Option<String>,
}

/// Move a window.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWindowMoveCmd {
    /// x position
    #[arg(long)]
    pub x: i32,

    /// y position
    #[arg(long)]
    pub y: i32,

    /// app name
    #[arg(long)]
    pub app: Option<String>,
}

/// Resize a window.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWindowResizeCmd {
    /// width
    #[arg(long)]
    pub width: i32,

    /// height
    #[arg(long)]
    pub height: i32,

    /// app name
    #[arg(long)]
    pub app: Option<String>,
}

/// Set window transparency.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWindowTransparentCmd {
    /// alpha value
    #[arg(long)]
    pub alpha: u8,

    /// app name
    #[arg(long)]
    pub app: Option<String>,
}

/// Toggle window always-on-top.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWindowTopCmd {
    /// enable topmost
    #[arg(long)]
    pub enable: bool,

    /// disable topmost
    #[arg(long)]
    pub disable: bool,

    /// app name
    #[arg(long)]
    pub app: Option<String>,
}

// ── Theme 嵌套子命令 ─────────────────────────────────────────────

/// Manage theme.
#[derive(Parser, Debug, Clone)]
pub struct DesktopThemeCmd {
    #[command(subcommand)]
    pub cmd: DesktopThemeSubCommand,
}

/// Theme 子命令枚举（4 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopThemeSubCommand {
    /// Set theme
    Set(DesktopThemeSetCmd),
    /// Toggle theme
    Toggle(DesktopThemeToggleCmd),
    /// Schedule theme
    Schedule(DesktopThemeScheduleCmd),
    /// Show theme status
    Status(DesktopThemeStatusCmd),
}

/// Set theme.
#[derive(Parser, Debug, Clone)]
pub struct DesktopThemeSetCmd {
    /// theme mode: light|dark
    pub mode: String,
}

/// Toggle theme.
#[derive(Parser, Debug, Clone)]
pub struct DesktopThemeToggleCmd {}

/// Schedule theme.
#[derive(Parser, Debug, Clone)]
pub struct DesktopThemeScheduleCmd {
    /// light time
    #[arg(long)]
    pub light: Option<String>,

    /// dark time
    #[arg(long)]
    pub dark: Option<String>,
}

/// Show theme status.
#[derive(Parser, Debug, Clone)]
pub struct DesktopThemeStatusCmd {}

// ── Awake 嵌套子命令 ─────────────────────────────────────────────

/// Manage awake mode.
#[derive(Parser, Debug, Clone)]
pub struct DesktopAwakeCmd {
    #[command(subcommand)]
    pub cmd: DesktopAwakeSubCommand,
}

/// Awake 子命令枚举（3 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopAwakeSubCommand {
    /// Enable awake mode
    On(DesktopAwakeOnCmd),
    /// Disable awake mode
    Off(DesktopAwakeOffCmd),
    /// Show awake status
    Status(DesktopAwakeStatusCmd),
}

/// Enable awake mode.
#[derive(Parser, Debug, Clone)]
pub struct DesktopAwakeOnCmd {
    /// duration string
    #[arg(long)]
    pub duration: Option<String>,

    /// expire at time
    #[arg(long)]
    pub expire_at: Option<String>,

    /// keep display on
    #[arg(long)]
    pub display_on: bool,
}

/// Disable awake mode.
#[derive(Parser, Debug, Clone)]
pub struct DesktopAwakeOffCmd {}

/// Show awake status.
#[derive(Parser, Debug, Clone)]
pub struct DesktopAwakeStatusCmd {}

// ── Color 命令 ───────────────────────────────────────────────────

/// Pick a color.
#[derive(Parser, Debug, Clone)]
pub struct DesktopColorCmd {
    /// copy to clipboard
    #[arg(long)]
    pub copy: bool,
}

// ── Hosts 嵌套子命令 ─────────────────────────────────────────────

/// Manage hosts file.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHostsCmd {
    #[command(subcommand)]
    pub cmd: DesktopHostsSubCommand,
}

/// Hosts 子命令枚举（3 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopHostsSubCommand {
    /// Add a hosts entry
    Add(DesktopHostsAddCmd),
    /// Remove a hosts entry
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopHostsRemoveCmd),
    /// List hosts entries
    List(DesktopHostsListCmd),
}

/// Add a hosts entry.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHostsAddCmd {
    /// hostname
    pub host: String,

    /// ip address
    pub ip: String,

    /// dry run
    #[arg(long)]
    pub dry_run: bool,
}

/// Remove a hosts entry.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHostsRemoveCmd {
    /// hostname
    pub host: String,

    /// dry run
    #[arg(long)]
    pub dry_run: bool,
}

/// List hosts entries.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHostsListCmd {}

// ── App 嵌套子命令 ───────────────────────────────────────────────

/// Manage installed apps.
#[derive(Parser, Debug, Clone)]
pub struct DesktopAppCmd {
    #[command(subcommand)]
    pub cmd: DesktopAppSubCommand,
}

/// App 子命令枚举（1 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopAppSubCommand {
    /// List installed apps
    List(DesktopAppListCmd),
}

/// List installed apps.
#[derive(Parser, Debug, Clone)]
pub struct DesktopAppListCmd {}

// ── Tui 命令 ─────────────────────────────────────────────────────

/// Launch desktop TUI.
#[derive(Parser, Debug, Clone)]
pub struct DesktopTuiCmd {}

// ── Run 命令 ─────────────────────────────────────────────────────

/// Run a command.
#[derive(Parser, Debug, Clone)]
pub struct DesktopRunCmd {
    /// command line
    pub command: String,
}

// ============================================================
// CommandSpec 实现
// ============================================================

#[cfg(feature = "desktop")]
use crate::xun_core::command::CommandSpec;
#[cfg(feature = "desktop")]
use crate::xun_core::context::CmdContext;
#[cfg(feature = "desktop")]
use crate::xun_core::error::XunError;
#[cfg(feature = "desktop")]
use crate::xun_core::value::Value;

/// desktop 命令。
#[cfg(feature = "desktop")]
pub struct DesktopCmdSpec {
    pub args: DesktopCmd,
}

#[cfg(feature = "desktop")]
impl CommandSpec for DesktopCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::commands::desktop::cmd_desktop(self.args.clone())
            ?;
        Ok(Value::Null)
    }
}
