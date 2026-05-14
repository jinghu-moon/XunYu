use clap::{Args, Parser, Subcommand};

#[cfg(feature = "desktop")]
/// Desktop control commands.
#[derive(Parser, Debug, Clone)]
pub struct DesktopCmd {
    #[command(subcommand)]
    pub cmd: DesktopSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopSubCommand {
    Daemon(DesktopDaemonCmd),
    Hotkey(DesktopHotkeyCmd),
    Remap(DesktopRemapCmd),
    Snippet(DesktopSnippetCmd),
    Layout(DesktopLayoutCmd),
    Workspace(DesktopWorkspaceCmd),
    Window(DesktopWindowCmd),
    Theme(DesktopThemeCmd),
    Awake(DesktopAwakeCmd),
    Color(DesktopColorCmd),
    Hosts(DesktopHostsCmd),
    App(DesktopAppCmd),
    Tui(DesktopTuiCmd),
    Run(DesktopRunCmd),
}

#[cfg(feature = "desktop")]
/// Manage desktop daemon.
#[derive(Parser, Debug, Clone)]
pub struct DesktopDaemonCmd {
    #[command(subcommand)]
    pub cmd: DesktopDaemonSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopDaemonSubCommand {
    Start(DesktopDaemonStartCmd),
    Stop(DesktopDaemonStopCmd),
    Status(DesktopDaemonStatusCmd),
    Reload(DesktopDaemonReloadCmd),
}

#[cfg(feature = "desktop")]
/// Start desktop daemon.
#[derive(Args, Debug, Clone)]
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

#[cfg(feature = "desktop")]
/// Stop desktop daemon.
#[derive(Args, Debug, Clone)]
pub struct DesktopDaemonStopCmd {}

#[cfg(feature = "desktop")]
/// Show desktop daemon status.
#[derive(Args, Debug, Clone)]
pub struct DesktopDaemonStatusCmd {}

#[cfg(feature = "desktop")]
/// Reload desktop daemon config.
#[derive(Args, Debug, Clone)]
pub struct DesktopDaemonReloadCmd {}

#[cfg(feature = "desktop")]
/// Manage hotkey bindings.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHotkeyCmd {
    #[command(subcommand)]
    pub cmd: DesktopHotkeySubCommand,
}

#[cfg(feature = "desktop")]
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopHotkeySubCommand {
    Bind(DesktopHotkeyBindCmd),
    Unbind(DesktopHotkeyUnbindCmd),
    List(DesktopHotkeyListCmd),
}

#[cfg(feature = "desktop")]
/// Bind a global hotkey to an action.
#[derive(Args, Debug, Clone)]
pub struct DesktopHotkeyBindCmd {
    /// hotkey string, e.g. ctrl+alt+t
    pub hotkey: String,

    /// action, e.g. run:wt.exe
    pub action: String,

    /// app filter
    #[arg(long)]
    pub app: Option<String>,
}

#[cfg(feature = "desktop")]
/// Unbind a hotkey.
#[derive(Args, Debug, Clone)]
pub struct DesktopHotkeyUnbindCmd {
    /// hotkey string
    pub hotkey: String,
}

#[cfg(feature = "desktop")]
/// List hotkey bindings.
#[derive(Args, Debug, Clone)]
pub struct DesktopHotkeyListCmd {}

#[cfg(feature = "desktop")]
/// Manage key remaps.
#[derive(Parser, Debug, Clone)]
pub struct DesktopRemapCmd {
    #[command(subcommand)]
    pub cmd: DesktopRemapSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopRemapSubCommand {
    Add(DesktopRemapAddCmd),
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopRemapRemoveCmd),
    List(DesktopRemapListCmd),
    Clear(DesktopRemapClearCmd),
}

#[cfg(feature = "desktop")]
/// Add a remap rule.
#[derive(Args, Debug, Clone)]
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

#[cfg(feature = "desktop")]
/// Remove a remap rule.
#[derive(Args, Debug, Clone)]
pub struct DesktopRemapRemoveCmd {
    /// from hotkey
    pub from: String,

    /// to target
    pub to: Option<String>,

    /// dry run
    #[arg(long)]
    pub dry_run: bool,
}

#[cfg(feature = "desktop")]
/// List remap rules.
#[derive(Args, Debug, Clone)]
pub struct DesktopRemapListCmd {}

#[cfg(feature = "desktop")]
/// Clear remap rules.
#[derive(Args, Debug, Clone)]
pub struct DesktopRemapClearCmd {
    /// dry run
    #[arg(long)]
    pub dry_run: bool,
}

#[cfg(feature = "desktop")]
/// Manage snippets.
#[derive(Parser, Debug, Clone)]
pub struct DesktopSnippetCmd {
    #[command(subcommand)]
    pub cmd: DesktopSnippetSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopSnippetSubCommand {
    Add(DesktopSnippetAddCmd),
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopSnippetRemoveCmd),
    List(DesktopSnippetListCmd),
    Clear(DesktopSnippetClearCmd),
}

#[cfg(feature = "desktop")]
/// Add a snippet.
#[derive(Args, Debug, Clone)]
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

#[cfg(feature = "desktop")]
/// Remove a snippet.
#[derive(Args, Debug, Clone)]
pub struct DesktopSnippetRemoveCmd {
    /// trigger text
    pub trigger: String,
}

#[cfg(feature = "desktop")]
/// List snippets.
#[derive(Args, Debug, Clone)]
pub struct DesktopSnippetListCmd {}

#[cfg(feature = "desktop")]
/// Clear snippets.
#[derive(Args, Debug, Clone)]
pub struct DesktopSnippetClearCmd {}

#[cfg(feature = "desktop")]
/// Manage layouts.
#[derive(Parser, Debug, Clone)]
pub struct DesktopLayoutCmd {
    #[command(subcommand)]
    pub cmd: DesktopLayoutSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopLayoutSubCommand {
    #[command(name = "add", alias = "new")]
    Add(DesktopLayoutNewCmd),
    Apply(DesktopLayoutApplyCmd),
    Preview(DesktopLayoutPreviewCmd),
    List(DesktopLayoutListCmd),
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopLayoutRemoveCmd),
}

#[cfg(feature = "desktop")]
/// Create a layout template.
#[derive(Args, Debug, Clone)]
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

#[cfg(feature = "desktop")]
/// Apply a layout.
#[derive(Args, Debug, Clone)]
pub struct DesktopLayoutApplyCmd {
    /// layout name
    pub name: String,

    /// move existing windows
    #[arg(long)]
    pub move_existing: bool,
}

#[cfg(feature = "desktop")]
/// Preview a layout.
#[derive(Args, Debug, Clone)]
pub struct DesktopLayoutPreviewCmd {
    /// layout name
    pub name: String,
}

#[cfg(feature = "desktop")]
/// List layouts.
#[derive(Args, Debug, Clone)]
pub struct DesktopLayoutListCmd {}

#[cfg(feature = "desktop")]
/// Remove a layout.
#[derive(Args, Debug, Clone)]
pub struct DesktopLayoutRemoveCmd {
    /// layout name
    pub name: String,
}

#[cfg(feature = "desktop")]
/// Manage workspaces.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWorkspaceCmd {
    #[command(subcommand)]
    pub cmd: DesktopWorkspaceSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopWorkspaceSubCommand {
    Save(DesktopWorkspaceSaveCmd),
    Launch(DesktopWorkspaceLaunchCmd),
    List(DesktopWorkspaceListCmd),
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopWorkspaceRemoveCmd),
}

#[cfg(feature = "desktop")]
/// Save current workspace.
#[derive(Args, Debug, Clone)]
pub struct DesktopWorkspaceSaveCmd {
    /// workspace name
    pub name: String,

    /// record name only
    #[arg(long)]
    pub name_only: bool,
}

#[cfg(feature = "desktop")]
/// Launch a workspace.
#[derive(Args, Debug, Clone)]
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

#[cfg(feature = "desktop")]
/// List workspaces.
#[derive(Args, Debug, Clone)]
pub struct DesktopWorkspaceListCmd {}

#[cfg(feature = "desktop")]
/// Remove a workspace.
#[derive(Args, Debug, Clone)]
pub struct DesktopWorkspaceRemoveCmd {
    /// workspace name
    pub name: String,
}

#[cfg(feature = "desktop")]
/// Manage windows.
#[derive(Parser, Debug, Clone)]
pub struct DesktopWindowCmd {
    #[command(subcommand)]
    pub cmd: DesktopWindowSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopWindowSubCommand {
    Focus(DesktopWindowFocusCmd),
    Move(DesktopWindowMoveCmd),
    Resize(DesktopWindowResizeCmd),
    Transparent(DesktopWindowTransparentCmd),
    Top(DesktopWindowTopCmd),
}

#[cfg(feature = "desktop")]
/// Focus a window.
#[derive(Args, Debug, Clone)]
pub struct DesktopWindowFocusCmd {
    /// app name
    #[arg(long)]
    pub app: Option<String>,

    /// window title
    #[arg(long)]
    pub title: Option<String>,
}

#[cfg(feature = "desktop")]
/// Move a window.
#[derive(Args, Debug, Clone)]
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

#[cfg(feature = "desktop")]
/// Resize a window.
#[derive(Args, Debug, Clone)]
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

#[cfg(feature = "desktop")]
/// Set window transparency.
#[derive(Args, Debug, Clone)]
pub struct DesktopWindowTransparentCmd {
    /// alpha value
    #[arg(long)]
    pub alpha: u8,

    /// app name
    #[arg(long)]
    pub app: Option<String>,
}

#[cfg(feature = "desktop")]
/// Toggle window always-on-top.
#[derive(Args, Debug, Clone)]
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

#[cfg(feature = "desktop")]
/// Manage theme.
#[derive(Parser, Debug, Clone)]
pub struct DesktopThemeCmd {
    #[command(subcommand)]
    pub cmd: DesktopThemeSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopThemeSubCommand {
    Set(DesktopThemeSetCmd),
    Toggle(DesktopThemeToggleCmd),
    Schedule(DesktopThemeScheduleCmd),
    Status(DesktopThemeStatusCmd),
}

#[cfg(feature = "desktop")]
/// Set theme.
#[derive(Args, Debug, Clone)]
pub struct DesktopThemeSetCmd {
    /// theme mode: light|dark
    pub mode: String,
}

#[cfg(feature = "desktop")]
/// Toggle theme.
#[derive(Args, Debug, Clone)]
pub struct DesktopThemeToggleCmd {}

#[cfg(feature = "desktop")]
/// Schedule theme.
#[derive(Args, Debug, Clone)]
pub struct DesktopThemeScheduleCmd {
    /// light time
    #[arg(long)]
    pub light: Option<String>,

    /// dark time
    #[arg(long)]
    pub dark: Option<String>,
}

#[cfg(feature = "desktop")]
/// Show theme status.
#[derive(Args, Debug, Clone)]
pub struct DesktopThemeStatusCmd {}

#[cfg(feature = "desktop")]
/// Manage awake mode.
#[derive(Parser, Debug, Clone)]
pub struct DesktopAwakeCmd {
    #[command(subcommand)]
    pub cmd: DesktopAwakeSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopAwakeSubCommand {
    On(DesktopAwakeOnCmd),
    Off(DesktopAwakeOffCmd),
    Status(DesktopAwakeStatusCmd),
}

#[cfg(feature = "desktop")]
/// Enable awake mode.
#[derive(Args, Debug, Clone)]
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

#[cfg(feature = "desktop")]
/// Disable awake mode.
#[derive(Args, Debug, Clone)]
pub struct DesktopAwakeOffCmd {}

#[cfg(feature = "desktop")]
/// Show awake status.
#[derive(Args, Debug, Clone)]
pub struct DesktopAwakeStatusCmd {}

#[cfg(feature = "desktop")]
/// Pick a color.
#[derive(Args, Debug, Clone)]
pub struct DesktopColorCmd {
    /// copy to clipboard
    #[arg(long)]
    pub copy: bool,
}

#[cfg(feature = "desktop")]
/// Manage hosts file.
#[derive(Parser, Debug, Clone)]
pub struct DesktopHostsCmd {
    #[command(subcommand)]
    pub cmd: DesktopHostsSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopHostsSubCommand {
    Add(DesktopHostsAddCmd),
    #[command(name = "rm", alias = "remove")]
    Rm(DesktopHostsRemoveCmd),
    List(DesktopHostsListCmd),
}

#[cfg(feature = "desktop")]
/// Add a hosts entry.
#[derive(Args, Debug, Clone)]
pub struct DesktopHostsAddCmd {
    /// hostname
    pub host: String,

    /// ip address
    pub ip: String,

    /// dry run
    #[arg(long)]
    pub dry_run: bool,
}

#[cfg(feature = "desktop")]
/// Remove a hosts entry.
#[derive(Args, Debug, Clone)]
pub struct DesktopHostsRemoveCmd {
    /// hostname
    pub host: String,

    /// dry run
    #[arg(long)]
    pub dry_run: bool,
}

#[cfg(feature = "desktop")]
/// List hosts entries.
#[derive(Args, Debug, Clone)]
pub struct DesktopHostsListCmd {}

#[cfg(feature = "desktop")]
/// Manage installed apps.
#[derive(Parser, Debug, Clone)]
pub struct DesktopAppCmd {
    #[command(subcommand)]
    pub cmd: DesktopAppSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(Subcommand, Debug, Clone)]
pub enum DesktopAppSubCommand {
    List(DesktopAppListCmd),
}

#[cfg(feature = "desktop")]
/// List installed apps.
#[derive(Args, Debug, Clone)]
pub struct DesktopAppListCmd {}

#[cfg(feature = "desktop")]
/// Launch desktop TUI.
#[derive(Args, Debug, Clone)]
pub struct DesktopTuiCmd {}

#[cfg(feature = "desktop")]
/// Run a command.
#[derive(Args, Debug, Clone)]
pub struct DesktopRunCmd {
    /// command line
    pub command: String,
}
