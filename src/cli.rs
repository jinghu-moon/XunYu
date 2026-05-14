mod acl;
#[cfg(feature = "alias")]
mod alias;
mod backup;
#[cfg(feature = "batch_rename")]
mod batch_rename;
#[path = "bookmark/cli_namespace.rs"]
mod bookmark;
#[path = "bookmark/cli_commands.rs"]
mod bookmarks;
mod config;
#[cfg(feature = "crypt")]
mod crypt;

mod ctx;
#[cfg(feature = "dashboard")]
mod dashboard;
mod defaults;
#[cfg(feature = "desktop")]
mod desktop;
mod env;
mod find;
#[cfg(feature = "fs")]
mod fs;
#[cfg(feature = "img")]
mod img;
#[cfg(feature = "lock")]
mod lock;
mod ports;
#[cfg(feature = "protect")]
mod protect;
mod proxy;
#[cfg(feature = "redirect")]
mod redirect;
mod shell;
mod tree;
#[cfg(feature = "crypt")]
mod vault;
mod video;
#[cfg(feature = "xunbak")]
mod xunbak;

use clap::{Parser, Subcommand};

#[cfg(feature = "diff")]
pub use crate::commands::diff::DiffCmd;
pub use acl::{
    AclAddCmd, AclAuditCmd, AclBackupCmd, AclBatchCmd, AclCmd, AclConfigCmd, AclCopyCmd,
    AclDiffCmd, AclEffectiveCmd, AclInheritCmd, AclOrphansCmd, AclOwnerCmd, AclPurgeCmd,
    AclRemoveCmd, AclRepairCmd, AclRestoreCmd, AclSubCommand, AclViewCmd,
};
#[cfg(feature = "alias")]
#[allow(unused_imports)]
pub use alias::{
    AliasAddCmd, AliasAppAddCmd, AliasAppCmd, AliasAppLsCmd, AliasAppRmCmd, AliasAppScanCmd,
    AliasAppSubCommand, AliasAppSyncCmd, AliasAppWhichCmd, AliasCmd, AliasExportCmd, AliasFindCmd,
    AliasImportCmd, AliasLsCmd, AliasRmCmd, AliasSetupCmd, AliasSubCommand, AliasSyncCmd,
    AliasWhichCmd,
};
#[cfg(feature = "xunbak")]
mod verify;
pub use backup::{
    BackupCmd, BackupConvertCmd, BackupCreateCmd, BackupRestoreCmd, BackupSubCommand,
};
#[cfg(feature = "batch_rename")]
pub use batch_rename::BrnCmd;
pub use bookmark::{
    AllCmd, CheckCmd, DedupCmd, DeleteCmd, ExportCmd, GcCmd, ImportCmd, KeysCmd, ListCmd, OpenCmd,
    RecentCmd, RenameCmd, SaveCmd, SetCmd, StatsCmd, TagAddCmd, TagAddBatchCmd, TagCmd,
    TagListCmd, TagRemoveCmd, TagRenameCmd, TagSubCommand, TouchCmd, ZCmd,
};
pub use bookmarks::{
    BookmarkCmd, BookmarkInitCmd, BookmarkSubCommand, LearnCmd, OiCmd,
    PinCmd, RedoCmd, UndoCmd, UnpinCmd, ZiCmd,
};
pub use config::{ConfigCmd, ConfigEditCmd, ConfigGetCmd, ConfigSetCmd, ConfigSubCommand};
#[cfg(feature = "crypt")]
pub use crypt::{DecryptCmd, EncryptCmd};

pub use ctx::{
    CtxCmd, CtxDelCmd, CtxListCmd, CtxOffCmd, CtxRenameCmd, CtxSetCmd, CtxShowCmd, CtxSubCommand,
    CtxUseCmd,
};
#[cfg(feature = "dashboard")]
pub use dashboard::ServeCmd;
#[cfg(feature = "desktop")]
pub use desktop::{
    DesktopAppCmd, DesktopAppListCmd, DesktopAppSubCommand, DesktopAwakeCmd, DesktopAwakeOffCmd,
    DesktopAwakeOnCmd, DesktopAwakeStatusCmd, DesktopAwakeSubCommand, DesktopCmd, DesktopColorCmd,
    DesktopDaemonCmd, DesktopDaemonReloadCmd, DesktopDaemonStartCmd, DesktopDaemonStatusCmd,
    DesktopDaemonStopCmd, DesktopDaemonSubCommand, DesktopHostsAddCmd, DesktopHostsCmd,
    DesktopHostsListCmd, DesktopHostsRemoveCmd, DesktopHostsSubCommand, DesktopHotkeyBindCmd,
    DesktopHotkeyCmd, DesktopHotkeyListCmd, DesktopHotkeySubCommand, DesktopHotkeyUnbindCmd,
    DesktopLayoutApplyCmd, DesktopLayoutCmd, DesktopLayoutListCmd, DesktopLayoutNewCmd,
    DesktopLayoutPreviewCmd, DesktopLayoutRemoveCmd, DesktopLayoutSubCommand, DesktopRemapAddCmd,
    DesktopRemapClearCmd, DesktopRemapCmd, DesktopRemapListCmd, DesktopRemapRemoveCmd,
    DesktopRemapSubCommand, DesktopRunCmd, DesktopSnippetAddCmd, DesktopSnippetClearCmd,
    DesktopSnippetCmd, DesktopSnippetListCmd, DesktopSnippetRemoveCmd, DesktopSnippetSubCommand,
    DesktopSubCommand, DesktopThemeCmd, DesktopThemeScheduleCmd, DesktopThemeSetCmd,
    DesktopThemeStatusCmd, DesktopThemeSubCommand, DesktopThemeToggleCmd, DesktopWindowCmd,
    DesktopWindowFocusCmd, DesktopWindowMoveCmd, DesktopWindowResizeCmd, DesktopWindowSubCommand,
    DesktopWindowTopCmd, DesktopWindowTransparentCmd, DesktopWorkspaceCmd,
    DesktopWorkspaceLaunchCmd, DesktopWorkspaceListCmd, DesktopWorkspaceRemoveCmd,
    DesktopWorkspaceSaveCmd, DesktopWorkspaceSubCommand,
};
pub use env::*;
pub use find::FindCmd;
#[cfg(feature = "fs")]
pub use fs::RmCmd;
#[cfg(feature = "img")]
pub use img::ImgCmd;
#[cfg(feature = "lock")]
pub use lock::{LockCmd, LockSubCommand, LockWhoCmd, MvCmd, RenFileCmd};
pub use ports::{KillCmd, PkillCmd, PortsCmd, PsCmd};
#[cfg(feature = "protect")]
pub use protect::{
    ProtectClearCmd, ProtectCmd, ProtectSetCmd, ProtectStatusCmd, ProtectSubCommand,
};
pub use proxy::{
    ProxyCmd, ProxyDetectCmd, ProxyExecCmd, ProxyOffCmd, ProxyOnCmd, ProxyStatusCmd,
    ProxySubCommand,
};
#[cfg(feature = "redirect")]
pub use redirect::RedirectCmd;
pub use shell::{CompleteCmd, CompletionCmd, InitCmd};
pub use tree::TreeCmd;
#[cfg(feature = "crypt")]
pub use vault::{
    VaultCleanupCmd, VaultCmd, VaultDecCmd, VaultEncCmd, VaultInspectCmd, VaultRecoverKeyCmd,
    VaultResumeCmd, VaultRewrapCmd, VaultSubCommand, VaultVerifyCmd,
};
#[cfg(feature = "xunbak")]
pub use verify::VerifyCmd;
pub use video::{VideoCmd, VideoCompressCmd, VideoProbeCmd, VideoRemuxCmd, VideoSubCommand};
#[cfg(feature = "xunbak")]
pub use xunbak::{
    XunbakCmd, XunbakPluginCmd, XunbakPluginDoctorCmd, XunbakPluginInstallCmd,
    XunbakPluginSubCommand, XunbakPluginUninstallCmd, XunbakSubCommand,
};

#[derive(Parser, Debug, Clone)]
#[command(name = "xun", about = "xun - bookmark + proxy CLI", version)]
pub struct Xun {
    /// disable ANSI colors (or set NO_COLOR=1)
    #[arg(long)]
    pub no_color: bool,

    /// suppress UI output
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// verbose output
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// force non-interactive mode
    #[arg(long)]
    pub non_interactive: bool,

    #[command(subcommand)]
    pub cmd: SubCommand,
}

#[derive(Subcommand, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum SubCommand {
    Acl(AclCmd),
    Init(InitCmd),
    Completion(CompletionCmd),
    #[command(alias = "__complete")]
    Complete(CompleteCmd),
    Bookmark(BookmarkCmd),
    Config(ConfigCmd),
    Ctx(CtxCmd),
    #[command(name = "rm", alias = "delete", alias = "del")]
    Rm(DeleteCmd),
    Proxy(ProxyCmd),
    Pon(ProxyOnCmd),
    Poff(ProxyOffCmd),
    Pst(ProxyStatusCmd),
    Px(ProxyExecCmd),
    #[command(hide = true)]
    Ports(PortsCmd),
    #[command(hide = true)]
    Kill(KillCmd),
    #[command(hide = true)]
    Ps(PsCmd),
    #[command(hide = true)]
    Pkill(PkillCmd),
    Backup(BackupCmd),
    Tree(TreeCmd),
    Find(FindCmd),
    Env(EnvCmd),
    #[cfg(feature = "alias")]
    Alias(AliasCmd),
    #[cfg(feature = "lock")]
    Lock(LockCmd),
    #[cfg(feature = "fs")]
    FsRm(RmCmd),
    #[cfg(feature = "lock")]
    Mv(MvCmd),
    #[cfg(feature = "lock")]
    RenFile(RenFileCmd),
    #[cfg(feature = "protect")]
    Protect(ProtectCmd),
    #[cfg(feature = "crypt")]
    Encrypt(EncryptCmd),
    #[cfg(feature = "crypt")]
    Decrypt(DecryptCmd),
    #[cfg(feature = "crypt")]
    Vault(VaultCmd),
    #[cfg(feature = "dashboard")]
    Serve(ServeCmd),
    #[cfg(feature = "diff")]
    Diff(DiffCmd),
    #[cfg(feature = "redirect")]
    Redirect(RedirectCmd),
    #[cfg(feature = "desktop")]
    Desktop(DesktopCmd),

    #[cfg(feature = "batch_rename")]
    Brn(BrnCmd),
    #[cfg(feature = "img")]
    Img(ImgCmd),
    Video(VideoCmd),
    #[cfg(feature = "xunbak")]
    Verify(VerifyCmd),
    #[cfg(feature = "xunbak")]
    Xunbak(XunbakCmd),
}
