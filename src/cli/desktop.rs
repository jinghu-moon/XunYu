use argh::FromArgs;

#[cfg(feature = "desktop")]
/// Desktop control commands.
#[derive(FromArgs)]
#[argh(subcommand, name = "desktop")]
pub struct DesktopCmd {
    #[argh(subcommand)]
    pub cmd: DesktopSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(FromArgs)]
#[argh(subcommand)]
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
#[derive(FromArgs)]
#[argh(subcommand, name = "daemon")]
pub struct DesktopDaemonCmd {
    #[argh(subcommand)]
    pub cmd: DesktopDaemonSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum DesktopDaemonSubCommand {
    Start(DesktopDaemonStartCmd),
    Stop(DesktopDaemonStopCmd),
    Status(DesktopDaemonStatusCmd),
    Reload(DesktopDaemonReloadCmd),
}

#[cfg(feature = "desktop")]
/// Start desktop daemon.
#[derive(FromArgs)]
#[argh(subcommand, name = "start")]
pub struct DesktopDaemonStartCmd {
    /// suppress UI output
    #[argh(switch, short = 'q')]
    pub quiet: bool,

    /// disable tray icon
    #[argh(switch)]
    pub no_tray: bool,

    /// run elevated if needed
    #[argh(switch)]
    pub elevated: bool,
}

#[cfg(feature = "desktop")]
/// Stop desktop daemon.
#[derive(FromArgs)]
#[argh(subcommand, name = "stop")]
pub struct DesktopDaemonStopCmd {}

#[cfg(feature = "desktop")]
/// Show desktop daemon status.
#[derive(FromArgs)]
#[argh(subcommand, name = "status")]
pub struct DesktopDaemonStatusCmd {}

#[cfg(feature = "desktop")]
/// Reload desktop daemon config.
#[derive(FromArgs)]
#[argh(subcommand, name = "reload")]
pub struct DesktopDaemonReloadCmd {}

#[cfg(feature = "desktop")]
/// Manage hotkey bindings.
#[derive(FromArgs)]
#[argh(subcommand, name = "hotkey")]
pub struct DesktopHotkeyCmd {
    #[argh(subcommand)]
    pub cmd: DesktopHotkeySubCommand,
}

#[cfg(feature = "desktop")]
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum DesktopHotkeySubCommand {
    Bind(DesktopHotkeyBindCmd),
    Unbind(DesktopHotkeyUnbindCmd),
    List(DesktopHotkeyListCmd),
}

#[cfg(feature = "desktop")]
/// Bind a global hotkey to an action.
#[derive(FromArgs)]
#[argh(subcommand, name = "bind")]
pub struct DesktopHotkeyBindCmd {
    /// hotkey string, e.g. ctrl+alt+t
    #[argh(positional)]
    pub hotkey: String,

    /// action, e.g. run:wt.exe
    #[argh(positional)]
    pub action: String,

    /// app filter
    #[argh(option)]
    pub app: Option<String>,
}

#[cfg(feature = "desktop")]
/// Unbind a hotkey.
#[derive(FromArgs)]
#[argh(subcommand, name = "unbind")]
pub struct DesktopHotkeyUnbindCmd {
    /// hotkey string
    #[argh(positional)]
    pub hotkey: String,
}

#[cfg(feature = "desktop")]
/// List hotkey bindings.
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct DesktopHotkeyListCmd {}

#[cfg(feature = "desktop")]
/// Manage key remaps.
#[derive(FromArgs)]
#[argh(subcommand, name = "remap")]
pub struct DesktopRemapCmd {
    #[argh(subcommand)]
    pub cmd: DesktopRemapSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum DesktopRemapSubCommand {
    Add(DesktopRemapAddCmd),
    Remove(DesktopRemapRemoveCmd),
    List(DesktopRemapListCmd),
    Clear(DesktopRemapClearCmd),
}

#[cfg(feature = "desktop")]
/// Add a remap rule.
#[derive(FromArgs)]
#[argh(subcommand, name = "add")]
pub struct DesktopRemapAddCmd {
    /// from hotkey
    #[argh(positional)]
    pub from: String,

    /// to target
    #[argh(positional)]
    pub to: String,

    /// app filter
    #[argh(option)]
    pub app: Option<String>,

    /// match exact app name
    #[argh(switch)]
    pub exact: bool,

    /// dry run
    #[argh(switch)]
    pub dry_run: bool,
}

#[cfg(feature = "desktop")]
/// Remove a remap rule.
#[derive(FromArgs)]
#[argh(subcommand, name = "remove")]
pub struct DesktopRemapRemoveCmd {
    /// from hotkey
    #[argh(positional)]
    pub from: String,

    /// to target
    #[argh(positional)]
    pub to: Option<String>,

    /// dry run
    #[argh(switch)]
    pub dry_run: bool,
}

#[cfg(feature = "desktop")]
/// List remap rules.
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct DesktopRemapListCmd {}

#[cfg(feature = "desktop")]
/// Clear remap rules.
#[derive(FromArgs)]
#[argh(subcommand, name = "clear")]
pub struct DesktopRemapClearCmd {
    /// dry run
    #[argh(switch)]
    pub dry_run: bool,
}

#[cfg(feature = "desktop")]
/// Manage snippets.
#[derive(FromArgs)]
#[argh(subcommand, name = "snippet")]
pub struct DesktopSnippetCmd {
    #[argh(subcommand)]
    pub cmd: DesktopSnippetSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum DesktopSnippetSubCommand {
    Add(DesktopSnippetAddCmd),
    Remove(DesktopSnippetRemoveCmd),
    List(DesktopSnippetListCmd),
    Clear(DesktopSnippetClearCmd),
}

#[cfg(feature = "desktop")]
/// Add a snippet.
#[derive(FromArgs)]
#[argh(subcommand, name = "add")]
pub struct DesktopSnippetAddCmd {
    /// trigger text
    #[argh(positional)]
    pub trigger: String,

    /// expansion text
    #[argh(positional)]
    pub expand: String,

    /// app filter
    #[argh(option)]
    pub app: Option<String>,

    /// trigger immediately
    #[argh(switch)]
    pub immediate: bool,

    /// paste via clipboard
    #[argh(switch)]
    pub clipboard: bool,
}

#[cfg(feature = "desktop")]
/// Remove a snippet.
#[derive(FromArgs)]
#[argh(subcommand, name = "remove")]
pub struct DesktopSnippetRemoveCmd {
    /// trigger text
    #[argh(positional)]
    pub trigger: String,
}

#[cfg(feature = "desktop")]
/// List snippets.
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct DesktopSnippetListCmd {}

#[cfg(feature = "desktop")]
/// Clear snippets.
#[derive(FromArgs)]
#[argh(subcommand, name = "clear")]
pub struct DesktopSnippetClearCmd {}

#[cfg(feature = "desktop")]
/// Manage layouts.
#[derive(FromArgs)]
#[argh(subcommand, name = "layout")]
pub struct DesktopLayoutCmd {
    #[argh(subcommand)]
    pub cmd: DesktopLayoutSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum DesktopLayoutSubCommand {
    New(DesktopLayoutNewCmd),
    Apply(DesktopLayoutApplyCmd),
    Preview(DesktopLayoutPreviewCmd),
    List(DesktopLayoutListCmd),
    Remove(DesktopLayoutRemoveCmd),
}

#[cfg(feature = "desktop")]
/// Create a layout template.
#[derive(FromArgs)]
#[argh(subcommand, name = "new")]
pub struct DesktopLayoutNewCmd {
    /// layout name
    #[argh(positional)]
    pub name: String,

    /// layout type
    #[argh(option, short = 't')]
    pub layout_type: String,

    /// rows count
    #[argh(option)]
    pub rows: Option<u32>,

    /// cols count
    #[argh(option)]
    pub cols: Option<u32>,

    /// gap size
    #[argh(option)]
    pub gap: Option<u32>,
}

#[cfg(feature = "desktop")]
/// Apply a layout.
#[derive(FromArgs)]
#[argh(subcommand, name = "apply")]
pub struct DesktopLayoutApplyCmd {
    /// layout name
    #[argh(positional)]
    pub name: String,

    /// move existing windows
    #[argh(switch)]
    pub move_existing: bool,
}

#[cfg(feature = "desktop")]
/// Preview a layout.
#[derive(FromArgs)]
#[argh(subcommand, name = "preview")]
pub struct DesktopLayoutPreviewCmd {
    /// layout name
    #[argh(positional)]
    pub name: String,
}

#[cfg(feature = "desktop")]
/// List layouts.
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct DesktopLayoutListCmd {}

#[cfg(feature = "desktop")]
/// Remove a layout.
#[derive(FromArgs)]
#[argh(subcommand, name = "remove")]
pub struct DesktopLayoutRemoveCmd {
    /// layout name
    #[argh(positional)]
    pub name: String,
}

#[cfg(feature = "desktop")]
/// Manage workspaces.
#[derive(FromArgs)]
#[argh(subcommand, name = "workspace")]
pub struct DesktopWorkspaceCmd {
    #[argh(subcommand)]
    pub cmd: DesktopWorkspaceSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum DesktopWorkspaceSubCommand {
    Save(DesktopWorkspaceSaveCmd),
    Launch(DesktopWorkspaceLaunchCmd),
    List(DesktopWorkspaceListCmd),
    Remove(DesktopWorkspaceRemoveCmd),
}

#[cfg(feature = "desktop")]
/// Save current workspace.
#[derive(FromArgs)]
#[argh(subcommand, name = "save")]
pub struct DesktopWorkspaceSaveCmd {
    /// workspace name
    #[argh(positional)]
    pub name: String,

    /// record name only
    #[argh(switch)]
    pub name_only: bool,
}

#[cfg(feature = "desktop")]
/// Launch a workspace.
#[derive(FromArgs)]
#[argh(subcommand, name = "launch")]
pub struct DesktopWorkspaceLaunchCmd {
    /// workspace name
    #[argh(positional)]
    pub name: String,

    /// move existing windows
    #[argh(switch)]
    pub move_existing: bool,

    /// monitor offset
    #[argh(option)]
    pub monitor_offset: Option<i32>,
}

#[cfg(feature = "desktop")]
/// List workspaces.
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct DesktopWorkspaceListCmd {}

#[cfg(feature = "desktop")]
/// Remove a workspace.
#[derive(FromArgs)]
#[argh(subcommand, name = "remove")]
pub struct DesktopWorkspaceRemoveCmd {
    /// workspace name
    #[argh(positional)]
    pub name: String,
}

#[cfg(feature = "desktop")]
/// Manage windows.
#[derive(FromArgs)]
#[argh(subcommand, name = "window")]
pub struct DesktopWindowCmd {
    #[argh(subcommand)]
    pub cmd: DesktopWindowSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum DesktopWindowSubCommand {
    Focus(DesktopWindowFocusCmd),
    Move(DesktopWindowMoveCmd),
    Resize(DesktopWindowResizeCmd),
    Transparent(DesktopWindowTransparentCmd),
    Top(DesktopWindowTopCmd),
}

#[cfg(feature = "desktop")]
/// Focus a window.
#[derive(FromArgs)]
#[argh(subcommand, name = "focus")]
pub struct DesktopWindowFocusCmd {
    /// app name
    #[argh(option)]
    pub app: Option<String>,

    /// window title
    #[argh(option)]
    pub title: Option<String>,
}

#[cfg(feature = "desktop")]
/// Move a window.
#[derive(FromArgs)]
#[argh(subcommand, name = "move")]
pub struct DesktopWindowMoveCmd {
    /// x position
    #[argh(option)]
    pub x: i32,

    /// y position
    #[argh(option)]
    pub y: i32,

    /// app name
    #[argh(option)]
    pub app: Option<String>,
}

#[cfg(feature = "desktop")]
/// Resize a window.
#[derive(FromArgs)]
#[argh(subcommand, name = "resize")]
pub struct DesktopWindowResizeCmd {
    /// width
    #[argh(option)]
    pub width: i32,

    /// height
    #[argh(option)]
    pub height: i32,

    /// app name
    #[argh(option)]
    pub app: Option<String>,
}

#[cfg(feature = "desktop")]
/// Set window transparency.
#[derive(FromArgs)]
#[argh(subcommand, name = "transparent")]
pub struct DesktopWindowTransparentCmd {
    /// alpha value
    #[argh(option)]
    pub alpha: u8,

    /// app name
    #[argh(option)]
    pub app: Option<String>,
}

#[cfg(feature = "desktop")]
/// Toggle window always-on-top.
#[derive(FromArgs)]
#[argh(subcommand, name = "top")]
pub struct DesktopWindowTopCmd {
    /// enable topmost
    #[argh(switch)]
    pub enable: bool,

    /// disable topmost
    #[argh(switch)]
    pub disable: bool,

    /// app name
    #[argh(option)]
    pub app: Option<String>,
}

#[cfg(feature = "desktop")]
/// Manage theme.
#[derive(FromArgs)]
#[argh(subcommand, name = "theme")]
pub struct DesktopThemeCmd {
    #[argh(subcommand)]
    pub cmd: DesktopThemeSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum DesktopThemeSubCommand {
    Set(DesktopThemeSetCmd),
    Toggle(DesktopThemeToggleCmd),
    Schedule(DesktopThemeScheduleCmd),
    Status(DesktopThemeStatusCmd),
}

#[cfg(feature = "desktop")]
/// Set theme.
#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
pub struct DesktopThemeSetCmd {
    /// theme mode: light|dark
    #[argh(positional)]
    pub mode: String,
}

#[cfg(feature = "desktop")]
/// Toggle theme.
#[derive(FromArgs)]
#[argh(subcommand, name = "toggle")]
pub struct DesktopThemeToggleCmd {}

#[cfg(feature = "desktop")]
/// Schedule theme.
#[derive(FromArgs)]
#[argh(subcommand, name = "schedule")]
pub struct DesktopThemeScheduleCmd {
    /// light time
    #[argh(option)]
    pub light: Option<String>,

    /// dark time
    #[argh(option)]
    pub dark: Option<String>,
}

#[cfg(feature = "desktop")]
/// Show theme status.
#[derive(FromArgs)]
#[argh(subcommand, name = "status")]
pub struct DesktopThemeStatusCmd {}

#[cfg(feature = "desktop")]
/// Manage awake mode.
#[derive(FromArgs)]
#[argh(subcommand, name = "awake")]
pub struct DesktopAwakeCmd {
    #[argh(subcommand)]
    pub cmd: DesktopAwakeSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum DesktopAwakeSubCommand {
    On(DesktopAwakeOnCmd),
    Off(DesktopAwakeOffCmd),
    Status(DesktopAwakeStatusCmd),
}

#[cfg(feature = "desktop")]
/// Enable awake mode.
#[derive(FromArgs)]
#[argh(subcommand, name = "on")]
pub struct DesktopAwakeOnCmd {
    /// duration string
    #[argh(option)]
    pub duration: Option<String>,

    /// expire at time
    #[argh(option)]
    pub expire_at: Option<String>,

    /// keep display on
    #[argh(switch)]
    pub display_on: bool,
}

#[cfg(feature = "desktop")]
/// Disable awake mode.
#[derive(FromArgs)]
#[argh(subcommand, name = "off")]
pub struct DesktopAwakeOffCmd {}

#[cfg(feature = "desktop")]
/// Show awake status.
#[derive(FromArgs)]
#[argh(subcommand, name = "status")]
pub struct DesktopAwakeStatusCmd {}

#[cfg(feature = "desktop")]
/// Pick a color.
#[derive(FromArgs)]
#[argh(subcommand, name = "color")]
pub struct DesktopColorCmd {
    /// copy to clipboard
    #[argh(switch)]
    pub copy: bool,
}

#[cfg(feature = "desktop")]
/// Manage hosts file.
#[derive(FromArgs)]
#[argh(subcommand, name = "hosts")]
pub struct DesktopHostsCmd {
    #[argh(subcommand)]
    pub cmd: DesktopHostsSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum DesktopHostsSubCommand {
    Add(DesktopHostsAddCmd),
    Remove(DesktopHostsRemoveCmd),
    List(DesktopHostsListCmd),
}

#[cfg(feature = "desktop")]
/// Add a hosts entry.
#[derive(FromArgs)]
#[argh(subcommand, name = "add")]
pub struct DesktopHostsAddCmd {
    /// hostname
    #[argh(positional)]
    pub host: String,

    /// ip address
    #[argh(positional)]
    pub ip: String,

    /// dry run
    #[argh(switch)]
    pub dry_run: bool,
}

#[cfg(feature = "desktop")]
/// Remove a hosts entry.
#[derive(FromArgs)]
#[argh(subcommand, name = "remove")]
pub struct DesktopHostsRemoveCmd {
    /// hostname
    #[argh(positional)]
    pub host: String,

    /// dry run
    #[argh(switch)]
    pub dry_run: bool,
}

#[cfg(feature = "desktop")]
/// List hosts entries.
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct DesktopHostsListCmd {}

#[cfg(feature = "desktop")]
/// Manage installed apps.
#[derive(FromArgs)]
#[argh(subcommand, name = "app")]
pub struct DesktopAppCmd {
    #[argh(subcommand)]
    pub cmd: DesktopAppSubCommand,
}

#[cfg(feature = "desktop")]
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum DesktopAppSubCommand {
    List(DesktopAppListCmd),
}

#[cfg(feature = "desktop")]
/// List installed apps.
#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
pub struct DesktopAppListCmd {}

#[cfg(feature = "desktop")]
/// Launch desktop TUI.
#[derive(FromArgs)]
#[argh(subcommand, name = "tui")]
pub struct DesktopTuiCmd {}

#[cfg(feature = "desktop")]
/// Run a command.
#[derive(FromArgs)]
#[argh(subcommand, name = "run")]
pub struct DesktopRunCmd {
    /// command line
    #[argh(positional)]
    pub command: String,
}
