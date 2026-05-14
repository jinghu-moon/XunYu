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
    pub sub: DesktopSubCommand,
}

/// Desktop 子命令枚举（14 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopSubCommand {
    /// Manage desktop daemon
    Daemon(DesktopDaemonArgs),
    /// Manage hotkey bindings
    Hotkey(DesktopHotkeyArgs),
    /// Manage key remaps
    Remap(DesktopRemapArgs),
    /// Manage snippets
    Snippet(DesktopSnippetArgs),
    /// Manage layouts
    Layout(DesktopLayoutArgs),
    /// Manage workspaces
    Workspace(DesktopWorkspaceArgs),
    /// Manage windows
    Window(DesktopWindowArgs),
    /// Manage theme
    Theme(DesktopThemeArgs),
    /// Manage awake mode
    Awake(DesktopAwakeArgs),
    /// Pick a color
    Color(DesktopColorArgs),
    /// Manage hosts file
    Hosts(DesktopHostsArgs),
    /// Manage installed apps
    App(DesktopAppArgs),
    /// Launch desktop TUI
    Tui(DesktopTuiArgs),
    /// Run a command
    Run(DesktopRunArgs),
}

// ── Daemon 嵌套子命令 ────────────────────────────────────────────

/// Manage desktop daemon.
#[derive(Parser, Debug, Clone)]
pub struct DesktopDaemonArgs {
    #[command(subcommand)]
    pub sub: DesktopDaemonSubCommand,
}

/// Daemon 子命令枚举（4 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopDaemonSubCommand {
    /// Start desktop daemon
    Start(DesktopDaemonStartArgs),
    /// Stop desktop daemon
    Stop(DesktopDaemonStopArgs),
    /// Show desktop daemon status
    Status(DesktopDaemonStatusArgs),
    /// Reload desktop daemon config
    Reload(DesktopDaemonReloadArgs),
}

/// Start desktop daemon.
#[derive(Parser, Debug, Clone)]
pub struct DesktopDaemonStartArgs {
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
pub struct DesktopDaemonStopArgs {}

/// Show desktop daemon status.
#[derive(Parser, Debug, Clone)]
pub struct DesktopDaemonStatusArgs {}

/// Reload desktop daemon config.
#[derive(Parser, Debug, Clone)]
pub struct DesktopDaemonReloadArgs {}

// ── Hotkey 嵌套子命令 ────────────────────────────────────────────

/// Manage hotkey bindings.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHotkeyArgs {
    #[command(subcommand)]
    pub sub: DesktopHotkeySubCommand,
}

/// Hotkey 子命令枚举（3 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopHotkeySubCommand {
    /// Bind a global hotkey to an action
    Bind(DesktopHotkeyBindArgs),
    /// Unbind a hotkey
    Unbind(DesktopHotkeyUnbindArgs),
    /// List hotkey bindings
    List(DesktopHotkeyListArgs),
}

/// Bind a global hotkey to an action.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHotkeyBindArgs {
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
pub struct DesktopHotkeyUnbindArgs {
    /// hotkey string
    pub hotkey: String,
}

/// List hotkey bindings.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHotkeyListArgs {}

// ── Remap 嵌套子命令 ─────────────────────────────────────────────

/// Manage key remaps.
#[derive(Parser, Debug, Clone)]
pub struct DesktopRemapArgs {
    #[command(subcommand)]
    pub sub: DesktopRemapSubCommand,
}

/// Remap 子命令枚举（4 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopRemapSubCommand {
    /// Add a remap rule
    Add(DesktopRemapAddArgs),
    /// Remove a remap rule
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopRemapRemoveArgs),
    /// List remap rules
    List(DesktopRemapListArgs),
    /// Clear remap rules
    Clear(DesktopRemapClearArgs),
}

/// Add a remap rule.
#[derive(Parser, Debug, Clone)]
pub struct DesktopRemapAddArgs {
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
pub struct DesktopRemapRemoveArgs {
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
pub struct DesktopRemapListArgs {}

/// Clear remap rules.
#[derive(Parser, Debug, Clone)]
pub struct DesktopRemapClearArgs {
    /// dry run
    #[arg(long)]
    pub dry_run: bool,
}

// ── Snippet 嵌套子命令 ───────────────────────────────────────────

/// Manage snippets.
#[derive(Parser, Debug, Clone)]
pub struct DesktopSnippetArgs {
    #[command(subcommand)]
    pub sub: DesktopSnippetSubCommand,
}

/// Snippet 子命令枚举（4 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopSnippetSubCommand {
    /// Add a snippet
    Add(DesktopSnippetAddArgs),
    /// Remove a snippet
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopSnippetRemoveArgs),
    /// List snippets
    List(DesktopSnippetListArgs),
    /// Clear snippets
    Clear(DesktopSnippetClearArgs),
}

/// Add a snippet.
#[derive(Parser, Debug, Clone)]
pub struct DesktopSnippetAddArgs {
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
pub struct DesktopSnippetRemoveArgs {
    /// trigger text
    pub trigger: String,
}

/// List snippets.
#[derive(Parser, Debug, Clone)]
pub struct DesktopSnippetListArgs {}

/// Clear snippets.
#[derive(Parser, Debug, Clone)]
pub struct DesktopSnippetClearArgs {}

// ── Layout 嵌套子命令 ────────────────────────────────────────────

/// Manage layouts.
#[derive(Parser, Debug, Clone)]
pub struct DesktopLayoutArgs {
    #[command(subcommand)]
    pub sub: DesktopLayoutSubCommand,
}

/// Layout 子命令枚举（5 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopLayoutSubCommand {
    /// Create a layout template
    #[command(name = "add", alias = "new")]
    Add(DesktopLayoutNewArgs),
    /// Apply a layout
    Apply(DesktopLayoutApplyArgs),
    /// Preview a layout
    Preview(DesktopLayoutPreviewArgs),
    /// List layouts
    List(DesktopLayoutListArgs),
    /// Remove a layout
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopLayoutRemoveArgs),
}

/// Create a layout template.
#[derive(Parser, Debug, Clone)]
pub struct DesktopLayoutNewArgs {
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
pub struct DesktopLayoutApplyArgs {
    /// layout name
    pub name: String,

    /// move existing windows
    #[arg(long)]
    pub move_existing: bool,
}

/// Preview a layout.
#[derive(Parser, Debug, Clone)]
pub struct DesktopLayoutPreviewArgs {
    /// layout name
    pub name: String,
}

/// List layouts.
#[derive(Parser, Debug, Clone)]
pub struct DesktopLayoutListArgs {}

/// Remove a layout.
#[derive(Parser, Debug, Clone)]
pub struct DesktopLayoutRemoveArgs {
    /// layout name
    pub name: String,
}

// ── Workspace 嵌套子命令 ─────────────────────────────────────────

/// Manage workspaces.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWorkspaceArgs {
    #[command(subcommand)]
    pub sub: DesktopWorkspaceSubCommand,
}

/// Workspace 子命令枚举（4 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopWorkspaceSubCommand {
    /// Save current workspace
    Save(DesktopWorkspaceSaveArgs),
    /// Launch a workspace
    Launch(DesktopWorkspaceLaunchArgs),
    /// List workspaces
    List(DesktopWorkspaceListArgs),
    /// Remove a workspace
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopWorkspaceRemoveArgs),
}

/// Save current workspace.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWorkspaceSaveArgs {
    /// workspace name
    pub name: String,

    /// record name only
    #[arg(long)]
    pub name_only: bool,
}

/// Launch a workspace.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWorkspaceLaunchArgs {
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
pub struct DesktopWorkspaceListArgs {}

/// Remove a workspace.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWorkspaceRemoveArgs {
    /// workspace name
    pub name: String,
}

// ── Window 嵌套子命令 ────────────────────────────────────────────

/// Manage windows.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWindowArgs {
    #[command(subcommand)]
    pub sub: DesktopWindowSubCommand,
}

/// Window 子命令枚举（5 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopWindowSubCommand {
    /// Focus a window
    Focus(DesktopWindowFocusArgs),
    /// Move a window
    Move(DesktopWindowMoveArgs),
    /// Resize a window
    Resize(DesktopWindowResizeArgs),
    /// Set window transparency
    Transparent(DesktopWindowTransparentArgs),
    /// Toggle window always-on-top
    Top(DesktopWindowTopArgs),
}

/// Focus a window.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWindowFocusArgs {
    /// app name
    #[arg(long)]
    pub app: Option<String>,

    /// window title
    #[arg(long)]
    pub title: Option<String>,
}

/// Move a window.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWindowMoveArgs {
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
pub struct DesktopWindowResizeArgs {
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
pub struct DesktopWindowTransparentArgs {
    /// alpha value
    #[arg(long)]
    pub alpha: u8,

    /// app name
    #[arg(long)]
    pub app: Option<String>,
}

/// Toggle window always-on-top.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWindowTopArgs {
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
pub struct DesktopThemeArgs {
    #[command(subcommand)]
    pub sub: DesktopThemeSubCommand,
}

/// Theme 子命令枚举（4 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopThemeSubCommand {
    /// Set theme
    Set(DesktopThemeSetArgs),
    /// Toggle theme
    Toggle(DesktopThemeToggleArgs),
    /// Schedule theme
    Schedule(DesktopThemeScheduleArgs),
    /// Show theme status
    Status(DesktopThemeStatusArgs),
}

/// Set theme.
#[derive(Parser, Debug, Clone)]
pub struct DesktopThemeSetArgs {
    /// theme mode: light|dark
    pub mode: String,
}

/// Toggle theme.
#[derive(Parser, Debug, Clone)]
pub struct DesktopThemeToggleArgs {}

/// Schedule theme.
#[derive(Parser, Debug, Clone)]
pub struct DesktopThemeScheduleArgs {
    /// light time
    #[arg(long)]
    pub light: Option<String>,

    /// dark time
    #[arg(long)]
    pub dark: Option<String>,
}

/// Show theme status.
#[derive(Parser, Debug, Clone)]
pub struct DesktopThemeStatusArgs {}

// ── Awake 嵌套子命令 ─────────────────────────────────────────────

/// Manage awake mode.
#[derive(Parser, Debug, Clone)]
pub struct DesktopAwakeArgs {
    #[command(subcommand)]
    pub sub: DesktopAwakeSubCommand,
}

/// Awake 子命令枚举（3 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopAwakeSubCommand {
    /// Enable awake mode
    On(DesktopAwakeOnArgs),
    /// Disable awake mode
    Off(DesktopAwakeOffArgs),
    /// Show awake status
    Status(DesktopAwakeStatusArgs),
}

/// Enable awake mode.
#[derive(Parser, Debug, Clone)]
pub struct DesktopAwakeOnArgs {
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
pub struct DesktopAwakeOffArgs {}

/// Show awake status.
#[derive(Parser, Debug, Clone)]
pub struct DesktopAwakeStatusArgs {}

// ── Color 命令 ───────────────────────────────────────────────────

/// Pick a color.
#[derive(Parser, Debug, Clone)]
pub struct DesktopColorArgs {
    /// copy to clipboard
    #[arg(long)]
    pub copy: bool,
}

// ── Hosts 嵌套子命令 ─────────────────────────────────────────────

/// Manage hosts file.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHostsArgs {
    #[command(subcommand)]
    pub sub: DesktopHostsSubCommand,
}

/// Hosts 子命令枚举（3 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopHostsSubCommand {
    /// Add a hosts entry
    Add(DesktopHostsAddArgs),
    /// Remove a hosts entry
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopHostsRemoveArgs),
    /// List hosts entries
    List(DesktopHostsListArgs),
}

/// Add a hosts entry.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHostsAddArgs {
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
pub struct DesktopHostsRemoveArgs {
    /// hostname
    pub host: String,

    /// dry run
    #[arg(long)]
    pub dry_run: bool,
}

/// List hosts entries.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHostsListArgs {}

// ── App 嵌套子命令 ───────────────────────────────────────────────

/// Manage installed apps.
#[derive(Parser, Debug, Clone)]
pub struct DesktopAppArgs {
    #[command(subcommand)]
    pub sub: DesktopAppSubCommand,
}

/// App 子命令枚举（1 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopAppSubCommand {
    /// List installed apps
    List(DesktopAppListArgs),
}

/// List installed apps.
#[derive(Parser, Debug, Clone)]
pub struct DesktopAppListArgs {}

// ── Tui 命令 ─────────────────────────────────────────────────────

/// Launch desktop TUI.
#[derive(Parser, Debug, Clone)]
pub struct DesktopTuiArgs {}

// ── Run 命令 ─────────────────────────────────────────────────────

/// Run a command.
#[derive(Parser, Debug, Clone)]
pub struct DesktopRunArgs {
    /// command line
    pub command: String,
}
