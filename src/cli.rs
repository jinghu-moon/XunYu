//! cli — 类型 re-export 层
//!
//! 所有 clap 类型定义已迁移至 `xun_core::*_cmd`。
//! 本模块仅做 re-export，保持 `crate::cli::*` 导入路径兼容。

// ── 外部定义模块（bookmark 类型）──────────────────────────
#[path = "bookmark/cli_namespace.rs"]
mod bookmark;
#[path = "bookmark/cli_commands.rs"]
mod bookmarks;

// ── 从 xun_core re-export ─────────────────────────────────

// Config
pub use crate::xun_core::config_cmd::{
    ConfigCmd, ConfigEditCmd, ConfigGetCmd, ConfigSetCmd, ConfigSubCommand,
};

// Ctx
pub use crate::xun_core::ctx_cmd::{
    CtxCmd, CtxDelCmd, CtxListCmd, CtxOffCmd, CtxRenameCmd, CtxSetCmd, CtxShowCmd, CtxSubCommand,
    CtxUseCmd,
};

// Video
pub use crate::xun_core::video_cmd::{
    VideoCmd, VideoCompressCmd, VideoProbeCmd, VideoRemuxCmd, VideoSubCommand,
};

// Port / Proc
pub use crate::xun_core::port_cmd::{KillCmd, PortsCmd};
pub use crate::xun_core::proc_cmd::{PkillCmd, PsCmd};

// Proxy
pub use crate::xun_core::proxy_cmd::{
    ProxyDetectCmd, ProxyExecCmd, ProxyOffCmd, ProxyOnCmd, ProxyStatusCmd, ProxyTestCmd,
};

// Init / Completion
pub use crate::xun_core::completion_cmd::{CompleteCmd, CompletionCmd};

// Tree / Find
pub use crate::xun_core::find_cmd::FindCmd;
pub use crate::xun_core::tree_cmd::TreeCmd;

// Env (dispatch.rs uses xun_core::env_cmd directly)

// Backup
pub use crate::xun_core::backup_cmd::{
    BackupCmd, BackupConvertCmd, BackupCreateCmd, BackupRestoreCmd, BackupSubCommand,
};

// ACL
pub use crate::xun_core::acl_cmd::{
    AclAddCmd, AclAuditCmd, AclBackupCmd, AclBatchCmd, AclCmd, AclConfigCmd, AclCopyCmd,
    AclDiffCmd, AclEffectiveCmd, AclInheritCmd, AclOrphansCmd, AclOwnerCmd, AclPurgeCmd,
    AclRemoveCmd, AclRepairCmd, AclRestoreCmd, AclSubCommand, AclViewCmd,
};

// Alias (feature-gated)
#[cfg(feature = "alias")]
pub use crate::xun_core::alias_cmd::{
    AliasAddArgs, AliasAppAddArgs, AliasAppLsArgs, AliasAppRmArgs,
    AliasAppScanArgs, AliasAppSubCommand, AliasCmd, AliasExportArgs, AliasFindArgs, AliasImportArgs,
    AliasLsArgs, AliasRmArgs, AliasSetupArgs, AliasSubCommand,
};

// Bookmark (external definition)
pub use bookmark::{
    AllCmd, CheckCmd, DedupCmd, DeleteCmd, ExportCmd, GcCmd, ImportCmd, KeysCmd, ListCmd, OpenCmd,
    RecentCmd, RenameCmd, SaveCmd, SetCmd, StatsCmd, TagAddCmd, TagAddBatchCmd, TagCmd,
    TagListCmd, TagRemoveCmd, TagRenameCmd, TagSubCommand, TouchCmd, ZCmd,
};
pub use bookmarks::{
    BookmarkCmd, BookmarkInitCmd, BookmarkSubCommand, LearnCmd, OiCmd, PinCmd, RedoCmd, UndoCmd,
    UnpinCmd, ZiCmd,
};

// Lock (feature-gated)
#[cfg(feature = "lock")]
pub use crate::xun_core::lock_cmd::{LockCmd, LockSubCommand, LockWhoCmd, MvCmd, RenFileCmd};

// Protect (feature-gated)
#[cfg(feature = "protect")]
pub use crate::xun_core::protect_cmd::{
    ProtectClearCmd, ProtectCmd, ProtectSetCmd, ProtectStatusCmd, ProtectSubCommand,
};

// Crypt (feature-gated)
#[cfg(feature = "crypt")]
pub use crate::xun_core::crypt_cmd::{DecryptCmd, EncryptCmd};

// Vault (feature-gated)
#[cfg(feature = "crypt")]
pub use crate::xun_core::vault_cmd::{
    VaultCleanupCmd, VaultCmd, VaultDecCmd, VaultEncCmd, VaultInspectCmd, VaultRecoverKeyCmd,
    VaultResumeCmd, VaultRewrapCmd, VaultSubCommand, VaultVerifyCmd,
};

// Dashboard (feature-gated)
#[cfg(feature = "dashboard")]
pub use crate::xun_core::dashboard_cmd::ServeCmd;

// Redirect (feature-gated)
#[cfg(feature = "redirect")]
pub use crate::xun_core::redirect_cmd::RedirectCmd;

// Desktop (feature-gated)
#[cfg(feature = "desktop")]
pub use crate::xun_core::desktop_cmd::{
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

// Fs (feature-gated)
#[cfg(feature = "fs")]
pub use crate::xun_core::fs_cmd::RmCmd;

// Batch rename (feature-gated)
#[cfg(feature = "batch_rename")]
pub use crate::xun_core::brn_cmd::BrnCmd;

// Img (feature-gated)
#[cfg(feature = "img")]
pub use crate::xun_core::img_cmd::ImgCmd;

// Verify / Xunbak (feature-gated)
#[cfg(feature = "xunbak")]
pub use crate::xun_core::verify_cmd::VerifyCmd;
#[cfg(feature = "xunbak")]
pub use crate::xun_core::xunbak_cmd::{
    XunbakCmd, XunbakPluginCmd, XunbakPluginDoctorCmd, XunbakPluginInstallCmd,
    XunbakPluginSubCommand, XunbakPluginUninstallCmd, XunbakSubCommand,
};

// Diff (feature-gated)
#[cfg(feature = "diff")]
#[allow(unused_imports)]
pub use crate::commands::diff::DiffCmd;

// ── Xun 顶层类型（runtime.rs 需要）───────────────────────
pub use crate::xun_core::dispatch::Xun;
