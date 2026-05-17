//! xun_core 顶层 dispatch
//!
//! 新架构入口：Xun（clap） → SubCommand → CommandSpec::run()。
//! 已迁移命令通过 execute() 走 CommandSpec；未迁移命令桥接到旧 cmd_* 函数。

use clap::{Parser, Subcommand};

use crate::xun_core::command::execute;
use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;
use crate::xun_core::value::Value;

#[derive(clap::ValueEnum, Clone, Debug, Default, PartialEq, Eq)]
pub enum OutputFormat {
    #[default]
    Auto,
    Table,
    Json,
    Tsv,
    Csv,
}

// ── 新 Xun 顶层 ─────────────────────────────────────────────

/// xun CLI 顶层入口（新架构）。
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

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Auto)]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub cmd: SubCommand,
}

// ── SubCommand 枚举 ─────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum SubCommand {
    // ── 已迁移（有 CommandSpec）────────────────────────────
    Bookmark(crate::xun_core::bookmark_cmd::BookmarkCmd),
    Proxy(crate::xun_core::proxy_cmd::ProxyCmd),
    Backup(crate::xun_core::backup_cmd::BackupCmd),
    Env(crate::xun_core::env_cmd::EnvCmd),

    #[cfg(feature = "alias")]
    Alias(crate::xun_core::alias_cmd::AliasCmd),

    // ── 已迁移命令的简写别名 ──────────────────────────────
    /// proxy show (shorthand)
    #[command(hide = true)]
    Pst(crate::xun_core::proxy_cmd::ProxyShowCmd),

    // ── 未迁移（桥接到旧 cmd_* 函数）──────────────────────
    Acl(crate::xun_core::acl_cmd::AclCmd),
    Config(crate::xun_core::config_cmd::ConfigCmd),
    Ctx(crate::xun_core::ctx_cmd::CtxCmd),
    Tree(crate::xun_core::tree_cmd::TreeCmd),
    Find(crate::xun_core::find_cmd::FindCmd),
    Video(crate::xun_core::video_cmd::VideoCmd),
    Port(crate::xun_core::port_cmd::PortsCmd),
    Proc(crate::xun_core::proc_cmd::PsCmd),
    #[command(name = "rm", alias = "delete", alias = "del", hide = true)]
    Rm(crate::cli::DeleteCmd),

    // ── Proxy shorthand（未迁移）───────────────────────────
    #[command(hide = true)]
    Pon(crate::xun_core::proxy_cmd::ProxyOnCmd),
    #[command(hide = true)]
    Poff(crate::xun_core::proxy_cmd::ProxyOffCmd),
    #[command(hide = true)]
    Px(crate::xun_core::proxy_cmd::ProxyExecCmd),

    // ── Port/Proc shorthand（未迁移）───────────────────────
    #[command(hide = true)]
    Ports(crate::xun_core::port_cmd::PortsCmd),
    #[command(hide = true)]
    Kill(crate::xun_core::port_cmd::KillCmd),
    #[command(hide = true)]
    Ps(crate::xun_core::proc_cmd::PsCmd),
    #[command(hide = true)]
    Pkill(crate::xun_core::proc_cmd::PkillCmd),

    // ── 文件系统命令（未迁移）──────────────────────────────
    #[cfg(feature = "fs")]
    FsRm(crate::xun_core::fs_cmd::RmCmd),

    #[cfg(feature = "lock")]
    Lock(crate::xun_core::lock_cmd::LockCmd),
    #[cfg(feature = "lock")]
    Mv(crate::xun_core::lock_cmd::MvCmd),
    #[cfg(feature = "lock")]
    RenFile(crate::xun_core::lock_cmd::RenFileCmd),

    #[cfg(feature = "protect")]
    Protect(crate::xun_core::protect_cmd::ProtectCmd),

    #[cfg(feature = "crypt")]
    Encrypt(crate::xun_core::crypt_cmd::EncryptCmd),
    #[cfg(feature = "crypt")]
    Decrypt(crate::xun_core::crypt_cmd::DecryptCmd),
    #[cfg(feature = "crypt")]
    Vault(crate::xun_core::vault_cmd::VaultCmd),

    #[cfg(feature = "dashboard")]
    Serve(crate::xun_core::dashboard_cmd::ServeCmd),

    #[cfg(feature = "redirect")]
    Redirect(crate::xun_core::redirect_cmd::RedirectCmd),

    #[cfg(feature = "desktop")]
    Desktop(crate::xun_core::desktop_cmd::DesktopCmd),

    #[cfg(feature = "batch_rename")]
    Brn(crate::xun_core::brn_cmd::BrnCmd),

    #[cfg(feature = "img")]
    Img(crate::xun_core::img_cmd::ImgCmd),

    #[cfg(feature = "xunbak")]
    Verify(crate::xun_core::verify_cmd::VerifyCmd),
    #[cfg(feature = "xunbak")]
    Xunbak(crate::xun_core::xunbak_cmd::XunbakCmd),

    // ── 特殊命令 ──────────────────────────────────────────
    Init(crate::xun_core::init_cmd::InitCmd),
    Completion(crate::xun_core::completion_cmd::CompletionCmd),
    #[command(hide = true)]
    Complete(crate::xun_core::completion_cmd::CompleteCmd),
}

// ── dispatch 入口 ───────────────────────────────────────────

/// 路由 SubCommand 到对应处理逻辑。
///
/// 已迁移命令通过 `execute()` 走 CommandSpec；未迁移命令桥接到旧 `cmd_*` 函数。
pub fn dispatch(
    cmd: SubCommand,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    match cmd {
        // ── Bookmark（27 子命令）───────────────────────────
        SubCommand::Bookmark(bk) => dispatch_bookmark(bk, ctx, renderer),

        // ── Proxy（3 子命令）───────────────────────────────
        SubCommand::Proxy(px) => dispatch_proxy(px, ctx, renderer),

        // ── Proxy shorthand: pst ───────────────────────────
        SubCommand::Pst(a) => {
            use crate::xun_core::proxy_cmd::*;
            execute(&ProxyShowCmdSpec { args: a }, ctx, renderer)
        }

        // ── Backup ─────────────────────────────────────────
        SubCommand::Backup(bk) => dispatch_backup(bk, ctx, renderer),

        // ── Env（50+ 子命令）───────────────────────────────
        SubCommand::Env(env_cmd) => dispatch_env(env_cmd, ctx, renderer),

        // ── Alias（feature-gated）───────────────────────────
        #[cfg(feature = "alias")]
        SubCommand::Alias(alias_cmd) => dispatch_alias(alias_cmd, ctx, renderer),

        // ── Init（打印 shell 集成脚本）────────────────────
        SubCommand::Init(init) => {
            use crate::xun_core::init_cmd::InitCmdSpec;
            execute(&InitCmdSpec { args: init }, ctx, renderer)
        }

        // ── 未迁移命令：桥接到旧 cmd_* 函数 ──────────────
        SubCommand::Completion(a) => {
            use crate::xun_core::completion_cmd::CompletionCmdSpec;
            execute(&CompletionCmdSpec { args: a }, ctx, renderer)
        }
        SubCommand::Complete(a) => {
            use crate::xun_core::completion_cmd::CompleteCmdSpec;
            execute(&CompleteCmdSpec { args: a }, ctx, renderer)
        }
        SubCommand::Acl(a) => {
            use crate::xun_core::acl_cmd::AclCmdSpec;
            execute(&AclCmdSpec { args: a }, ctx, renderer)
        }
        SubCommand::Config(a) => {
            use crate::xun_core::config_cmd::*;
            match a.cmd {
                ConfigSubCommand::Get(g) => execute(&ConfigGetCmdSpec { args: g }, ctx, renderer),
                ConfigSubCommand::Set(s) => execute(&ConfigSetCmdSpec { args: s }, ctx, renderer),
                ConfigSubCommand::Edit(e) => execute(&ConfigEditCmdSpec { args: e }, ctx, renderer),
            }
        }
        SubCommand::Ctx(a) => dispatch_ctx(a, ctx, renderer),
        SubCommand::Tree(a) => {
            use crate::xun_core::tree_cmd::TreeCmdSpec;
            execute(&TreeCmdSpec { args: a }, ctx, renderer)
        }
        SubCommand::Find(a) => {
            use crate::xun_core::find_cmd::FindCmdSpec;
            execute(&FindCmdSpec { args: a }, ctx, renderer)
        }
        SubCommand::Video(a) => dispatch_video(a, ctx, renderer),
        SubCommand::Rm(a) => {
            use crate::xun_core::delete_cmd::DeleteCmdSpec;
            execute(&DeleteCmdSpec { args: a }, ctx, renderer)
        }

        // ── Proxy shorthand ────────────────────────────────
        SubCommand::Pon(a) => {
            use crate::xun_core::proxy_cmd::ProxyOnCmdSpec;
            execute(&ProxyOnCmdSpec { args: a }, ctx, renderer)
        }
        SubCommand::Poff(a) => {
            use crate::xun_core::proxy_cmd::ProxyOffCmdSpec;
            execute(&ProxyOffCmdSpec { args: a }, ctx, renderer)
        }
        SubCommand::Px(a) => {
            use crate::xun_core::proxy_cmd::ProxyExecCmdSpec;
            execute(&ProxyExecCmdSpec { args: a }, ctx, renderer)
        }

        // ── Port/Proc ──────────────────────────────────────
        SubCommand::Port(a) | SubCommand::Ports(a) => {
            use crate::xun_core::port_cmd::PortsCmdSpec;
            execute(&PortsCmdSpec { args: a }, ctx, renderer)
        }
        SubCommand::Kill(a) => {
            use crate::xun_core::port_cmd::KillCmdSpec;
            execute(&KillCmdSpec { args: a }, ctx, renderer)
        }
        SubCommand::Ps(a) | SubCommand::Proc(a) => {
            use crate::xun_core::proc_cmd::PsCmdSpec;
            execute(&PsCmdSpec { args: a }, ctx, renderer)
        }
        SubCommand::Pkill(a) => {
            use crate::xun_core::proc_cmd::PkillCmdSpec;
            execute(&PkillCmdSpec { args: a }, ctx, renderer)
        }

        // ── Feature-gated 命令 ─────────────────────────────
        #[cfg(feature = "fs")]
        SubCommand::FsRm(a) => {
            use crate::xun_core::fs_cmd::FsRmCmdSpec;
            execute(&FsRmCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "lock")]
        SubCommand::Lock(a) => {
            use crate::xun_core::lock_cmd::LockCmdSpec;
            execute(&LockCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "lock")]
        SubCommand::Mv(a) => {
            use crate::xun_core::lock_cmd::MvCmdSpec;
            execute(&MvCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "lock")]
        SubCommand::RenFile(a) => {
            use crate::xun_core::lock_cmd::RenFileCmdSpec;
            execute(&RenFileCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "protect")]
        SubCommand::Protect(a) => {
            use crate::xun_core::protect_cmd::ProtectCmdSpec;
            execute(&ProtectCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "crypt")]
        SubCommand::Encrypt(a) => {
            use crate::xun_core::crypt_cmd::EncryptCmdSpec;
            execute(&EncryptCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "crypt")]
        SubCommand::Decrypt(a) => {
            use crate::xun_core::crypt_cmd::DecryptCmdSpec;
            execute(&DecryptCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "crypt")]
        SubCommand::Vault(a) => {
            use crate::xun_core::vault_cmd::VaultCmdSpec;
            execute(&VaultCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "dashboard")]
        SubCommand::Serve(a) => {
            use crate::xun_core::dashboard_cmd::ServeCmdSpec;
            execute(&ServeCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "redirect")]
        SubCommand::Redirect(a) => {
            use crate::xun_core::redirect_cmd::RedirectCmdSpec;
            execute(&RedirectCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "desktop")]
        SubCommand::Desktop(a) => {
            use crate::xun_core::desktop_cmd::DesktopCmdSpec;
            execute(&DesktopCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "batch_rename")]
        SubCommand::Brn(a) => {
            use crate::xun_core::brn_cmd::BrnCmdSpec;
            execute(&BrnCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "img")]
        SubCommand::Img(a) => {
            use crate::xun_core::img_cmd::ImgCmdSpec;
            execute(&ImgCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "xunbak")]
        SubCommand::Verify(a) => {
            use crate::xun_core::verify_cmd::VerifyCmdSpec;
            execute(&VerifyCmdSpec { args: a }, ctx, renderer)
        }
        #[cfg(feature = "xunbak")]
        SubCommand::Xunbak(a) => {
            use crate::xun_core::xunbak_cmd::XunbakCmdSpec;
            execute(&XunbakCmdSpec { args: a }, ctx, renderer)
        }
    }
}



// ── Init 脚本渲染 ─────────────────────────────────────────

/// 渲染 shell 集成脚本。
pub(crate) fn render_init_script(shell: &str) -> Result<String, XunError> {
    match shell.to_ascii_lowercase().as_str() {
        "powershell" | "pwsh" => Ok(POWERSHELL_INIT_SCRIPT.to_string()),
        "bash" | "zsh" => Ok(BASH_INIT_SCRIPT.to_string()),
        _ => Err(XunError::user(format!(
            "Unsupported shell: {}. Use powershell or bash.",
            shell
        ))),
    }
}

const POWERSHELL_INIT_SCRIPT: &str = r#"
$xunExe = if ($env:XUN_EXE) { $env:XUN_EXE } else { "xun.exe" }
function xun { & $xunExe @args }
Set-Alias xyu xun
Set-Alias xy xun

function _xun_apply_magic {
    param([string[]]$lines)
    $printed = @()
    foreach ($line in $lines) {
        if ($line -match "^__CD__:(.*)") {
            $target = $matches[1]
            if (Test-Path $target -PathType Container) { Set-Location $target }
        } elseif ($line -match "^__BM_CD__ (.+)$") {
            $target = $matches[1]
            if (Test-Path $target -PathType Container) { Set-Location $target }
        } elseif ($line -match "^__ENV_SET__:(.+?)=(.*)$") {
            Set-Item "Env:\$($matches[1])" $matches[2]
        } elseif ($line -match "^__ENV_UNSET__:(.+)$") {
            Remove-Item "Env:\$($matches[1])" -ErrorAction SilentlyContinue
        } elseif ($line -ne $null -and $line -ne '') {
            $printed += $line
        }
    }
    if ($printed.Count -gt 0) { $printed | ForEach-Object { Write-Output $_ } }
}

function x {
    $old = $env:XUN_UI
    $env:XUN_UI = "1"
    $out = & $xunExe @args
    if ($null -ne $old) { $env:XUN_UI = $old } else { Remove-Item Env:\XUN_UI -ErrorAction SilentlyContinue }
    if ($LASTEXITCODE -ne 0) { return }
    if ($out -is [array]) { _xun_apply_magic $out }
    elseif ($out) { _xun_apply_magic @($out) }
}

function pon { x pon @args }
function poff { x poff @args }
function pst { x pst @args }
function px { xun px @args }
function backup { xun backup @args }
function bak { xun bak @args }

try {
    $comp = & $xunExe completion powershell 2>$null
    if ($LASTEXITCODE -eq 0 -and $comp) { Invoke-Expression $comp }
} catch {}
"#;

const BASH_INIT_SCRIPT: &str = r#"
_xun_cmd() { command xun "$@"; }
alias xyu=xun
alias xy=xun

_xun_apply_magic() {
    local line
    while IFS= read -r line; do
        case "$line" in
            __CD__:*) local target="${line#__CD__:}"
                [ -d "$target" ] && cd "$target" ;;
            __BM_CD__:*) local target="${line#__BM_CD__ }"
                [ -d "$target" ] && cd "$target" ;;
            __ENV_SET__:*) local kv="${line#__ENV_SET__:}"
                export "${kv%%=*}=${kv#*=}" ;;
            __ENV_UNSET__:*) unset "${line#__ENV_UNSET__:}" ;;
            *) [ -n "$line" ] && printf '%s\n' "$line" ;;
        esac
    done
}

x() {
    local XUN_UI=1
    _xun_apply_magic < <(XUN_UI=1 command xun "$@")
}

alias pon='x pon'
alias poff='x poff'
alias pst='x pst'
alias px='xun px'
alias backup='xun backup'
alias bak='xun bak'

if command -v xun &>/dev/null; then
    eval "$(xun completion bash 2>/dev/null)"
fi
"#;

// ── Bookmark 路由 ───────────────────────────────────────────

fn dispatch_bookmark(
    bk: crate::xun_core::bookmark_cmd::BookmarkCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::bookmark_cmd::*;
    match bk.sub {
        BookmarkSubCommand::Z(a) => execute(&BookmarkZCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Zi(a) => execute(&BookmarkZiCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::O(a) => execute(&BookmarkOCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Oi(a) => execute(&BookmarkOiCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Open(a) => execute(&BookmarkOpenCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Save(a) => execute(&BookmarkSaveCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Set(a) => execute(&BookmarkSetCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Rm(a) => execute(&BookmarkRmCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Tag(tag_cmd) => dispatch_tag(tag_cmd, ctx, renderer),
        BookmarkSubCommand::Pin(a) => execute(&BookmarkPinCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Unpin(a) => execute(&BookmarkUnpinCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Undo(a) => execute(&BookmarkUndoCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Redo(a) => execute(&BookmarkRedoCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Rename(a) => execute(&BookmarkRenameCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::List(a) => execute(&BookmarkListCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Recent(a) => execute(&BookmarkRecentCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Stats(a) => execute(&BookmarkStatsCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Check(a) => execute(&BookmarkCheckCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Gc(a) => execute(&BookmarkGcCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Dedup(a) => execute(&BookmarkDedupCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Export(a) => execute(&BookmarkExportCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Import(a) => execute(&BookmarkImportCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Init(a) => execute(&BookmarkInitCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Learn(a) => execute(&BookmarkLearnCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Touch(a) => execute(&BookmarkTouchCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::Keys(a) => execute(&BookmarkKeysCmd { args: a }, ctx, renderer),
        BookmarkSubCommand::All(a) => execute(&BookmarkAllCmd { args: a }, ctx, renderer),
    }
}

fn dispatch_tag(
    tag_cmd: crate::xun_core::bookmark_cmd::BookmarkTagCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::bookmark_cmd::*;
    match tag_cmd.sub {
        TagSubCommand::Add(a) => execute(&TagAddCmd { args: a }, ctx, renderer),
        TagSubCommand::AddBatch(a) => execute(&TagAddBatchCmd { args: a }, ctx, renderer),
        TagSubCommand::Remove(a) => execute(&TagRemoveCmd { args: a }, ctx, renderer),
        TagSubCommand::List(a) => execute(&TagListCmd { args: a }, ctx, renderer),
        TagSubCommand::Rename(a) => execute(&TagRenameCmd { args: a }, ctx, renderer),
    }
}

// ── Ctx 路由 ───────────────────────────────────────────────

fn dispatch_ctx(
    ctx_cmd: crate::xun_core::ctx_cmd::CtxCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::ctx_cmd::CtxCmdSpec;
    execute(&CtxCmdSpec { args: ctx_cmd }, ctx, renderer)
}

// ── Video 路由 ──────────────────────────────────────────────

fn dispatch_video(
    video_cmd: crate::xun_core::video_cmd::VideoCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::video_cmd::*;
    match video_cmd.cmd {
        VideoSubCommand::Probe(p) => execute(&VideoProbeCmdSpec { args: p }, ctx, renderer),
        VideoSubCommand::Compress(c) => execute(&VideoCompressCmdSpec { args: c }, ctx, renderer),
        VideoSubCommand::Remux(r) => execute(&VideoRemuxCmdSpec { args: r }, ctx, renderer),
    }
}

// ── Proxy 路由 ──────────────────────────────────────────────

fn dispatch_proxy(
    px: crate::xun_core::proxy_cmd::ProxyCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::proxy_cmd::*;
    match px.cmd {
        ProxySubCommand::Show(a) => execute(&ProxyShowCmdSpec { args: a }, ctx, renderer),
        ProxySubCommand::Set(a) => execute(&ProxySetCmdSpec { args: a }, ctx, renderer),
        ProxySubCommand::Rm(a) => execute(&ProxyRmCmdSpec { args: a }, ctx, renderer),
        ProxySubCommand::Detect(a) => execute(&ProxyDetectCmdSpec { args: a }, ctx, renderer),
        ProxySubCommand::Status(a) => execute(&ProxyStatusCmdSpec { args: a }, ctx, renderer),
        ProxySubCommand::Test(a) => {
            crate::commands::proxy::ops::cmd_proxy_test(a)?;
            Ok(Value::Null)
        }
    }
}

// ── Backup 路由 ─────────────────────────────────────────────

fn dispatch_backup(
    bk: crate::xun_core::backup_cmd::BackupCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::backup_cmd::*;
    match bk.cmd {
        Some(sub) => match sub {
            BackupSubCommand::Add(a) => execute(&BackupCreateCmdSpec { args: a }, ctx, renderer),
            BackupSubCommand::Restore(a) => execute(&BackupRestoreCmdSpec { args: a }, ctx, renderer),
            BackupSubCommand::Convert(a) => execute(&BackupConvertCmdSpec { args: a }, ctx, renderer),
            BackupSubCommand::List(a) => {
                execute(&BackupListCmdSpec { args: a, dir: bk.dir.clone() }, ctx, renderer)
            }
            BackupSubCommand::Verify(a) => {
                execute(&BackupVerifyCmdSpec { args: a, dir: bk.dir.clone() }, ctx, renderer)
            }
            BackupSubCommand::Find(a) => {
                execute(&BackupFindCmdSpec { args: a, dir: bk.dir.clone() }, ctx, renderer)
            }
        },
        None => execute(&BackupDefaultCmdSpec { args: bk }, ctx, renderer),
    }
}

// ── Env 路由 ────────────────────────────────────────────────

fn dispatch_env(
    env_cmd: crate::xun_core::env_cmd::EnvCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::env_cmd::*;
    match env_cmd.sub {
        EnvSubCommand::Status(a) => execute(&EnvStatusCmd { args: a }, ctx, renderer),
        EnvSubCommand::List(a) => execute(&EnvListCmd { args: a }, ctx, renderer),
        EnvSubCommand::Search(a) => execute(&EnvSearchCmd { args: a }, ctx, renderer),
        EnvSubCommand::Show(a) => execute(&EnvShowCmd { args: a }, ctx, renderer),
        EnvSubCommand::Set(a) => execute(&EnvSetCmd { args: a }, ctx, renderer),
        EnvSubCommand::Rm(a) => execute(&EnvRmCmd { args: a }, ctx, renderer),
        EnvSubCommand::Check(a) => execute(&EnvCheckCmd { args: a }, ctx, renderer),
        EnvSubCommand::PathDedup(a) => execute(&EnvPathDedupCmd { args: a }, ctx, renderer),
        EnvSubCommand::Doctor(a) => execute(&EnvDoctorCmd { args: a }, ctx, renderer),
        EnvSubCommand::Apply(a) => execute(&EnvApplyCmd { args: a }, ctx, renderer),
        EnvSubCommand::Export(a) => execute(&EnvExportCmd { args: a }, ctx, renderer),
        EnvSubCommand::ExportAll(a) => execute(&EnvExportAllCmd { args: a }, ctx, renderer),
        EnvSubCommand::ExportLive(a) => execute(&EnvExportLiveCmd { args: a }, ctx, renderer),
        EnvSubCommand::Env(a) => execute(&EnvMergedCmd { args: a }, ctx, renderer),
        EnvSubCommand::Import(a) => execute(&EnvImportCmd { args: a }, ctx, renderer),
        EnvSubCommand::DiffLive(a) => execute(&EnvDiffLiveCmd { args: a }, ctx, renderer),
        EnvSubCommand::Graph(a) => execute(&EnvGraphCmd { args: a }, ctx, renderer),
        EnvSubCommand::Validate(a) => execute(&EnvValidateCmd { args: a }, ctx, renderer),
        EnvSubCommand::Audit(a) => execute(&EnvAuditCmd { args: a }, ctx, renderer),
        EnvSubCommand::Watch(a) => execute(&EnvWatchCmd { args: a }, ctx, renderer),
        EnvSubCommand::Template(a) => execute(&EnvTemplateCmd { args: a }, ctx, renderer),
        EnvSubCommand::Run(a) => execute(&EnvRunCmd { args: a }, ctx, renderer),
        EnvSubCommand::Tui(a) => execute(&EnvTuiCmd { args: a }, ctx, renderer),
        EnvSubCommand::Path(path_cmd) => dispatch_env_path(path_cmd, ctx, renderer),
        EnvSubCommand::Snapshot(snap_cmd) => dispatch_env_snapshot(snap_cmd, ctx, renderer),
        EnvSubCommand::Profile(prof_cmd) => dispatch_env_profile(prof_cmd, ctx, renderer),
        EnvSubCommand::Batch(batch_cmd) => dispatch_env_batch(batch_cmd, ctx, renderer),
        EnvSubCommand::Schema(schema_cmd) => dispatch_env_schema(schema_cmd, ctx, renderer),
        EnvSubCommand::Annotate(ann_cmd) => dispatch_env_annotate(ann_cmd, ctx, renderer),
        EnvSubCommand::Config(cfg_cmd) => dispatch_env_config(cfg_cmd, ctx, renderer),
    }
}

fn dispatch_env_path(
    path_cmd: crate::xun_core::env_cmd::EnvPathCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::env_cmd::*;
    match path_cmd.sub {
        EnvPathSubCommand::Add(a) => execute(&EnvPathAddCmd { args: a }, ctx, renderer),
        EnvPathSubCommand::Rm(a) => execute(&EnvPathRmCmd { args: a }, ctx, renderer),
    }
}

fn dispatch_env_snapshot(
    snap_cmd: crate::xun_core::env_cmd::EnvSnapshotCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::env_cmd::*;
    match snap_cmd.sub {
        EnvSnapshotSubCommand::Create(a) => execute(&EnvSnapshotCreateCmd { args: a }, ctx, renderer),
        EnvSnapshotSubCommand::List(a) => execute(&EnvSnapshotListCmd { args: a }, ctx, renderer),
        EnvSnapshotSubCommand::Restore(a) => execute(&EnvSnapshotRestoreCmd { args: a }, ctx, renderer),
        EnvSnapshotSubCommand::Prune(a) => execute(&EnvSnapshotPruneCmd { args: a }, ctx, renderer),
    }
}

fn dispatch_env_profile(
    prof_cmd: crate::xun_core::env_cmd::EnvProfileCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::env_cmd::*;
    match prof_cmd.sub {
        EnvProfileSubCommand::List(a) => execute(&EnvProfileListCmd { args: a }, ctx, renderer),
        EnvProfileSubCommand::Capture(a) => execute(&EnvProfileCaptureCmd { args: a }, ctx, renderer),
        EnvProfileSubCommand::Apply(a) => execute(&EnvProfileApplyCmd { args: a }, ctx, renderer),
        EnvProfileSubCommand::Diff(a) => execute(&EnvProfileDiffCmd { args: a }, ctx, renderer),
        EnvProfileSubCommand::Rm(a) => execute(&EnvProfileRmCmd { args: a }, ctx, renderer),
    }
}

fn dispatch_env_batch(
    batch_cmd: crate::xun_core::env_cmd::EnvBatchCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::env_cmd::*;
    match batch_cmd.sub {
        EnvBatchSubCommand::Set(a) => execute(&EnvBatchSetCmd { args: a }, ctx, renderer),
        EnvBatchSubCommand::Rm(a) => execute(&EnvBatchRmCmd { args: a }, ctx, renderer),
        EnvBatchSubCommand::Rename(a) => execute(&EnvBatchRenameCmd { args: a }, ctx, renderer),
    }
}

fn dispatch_env_schema(
    schema_cmd: crate::xun_core::env_cmd::EnvSchemaCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::env_cmd::*;
    match schema_cmd.sub {
        EnvSchemaSubCommand::Show(a) => execute(&EnvSchemaShowCmd { args: a }, ctx, renderer),
        EnvSchemaSubCommand::AddRequired(a) => execute(&EnvSchemaAddRequiredCmd { args: a }, ctx, renderer),
        EnvSchemaSubCommand::AddRegex(a) => execute(&EnvSchemaAddRegexCmd { args: a }, ctx, renderer),
        EnvSchemaSubCommand::AddEnum(a) => execute(&EnvSchemaAddEnumCmd { args: a }, ctx, renderer),
        EnvSchemaSubCommand::Remove(a) => execute(&EnvSchemaRemoveCmd { args: a }, ctx, renderer),
        EnvSchemaSubCommand::Reset(a) => execute(&EnvSchemaResetCmd { args: a }, ctx, renderer),
    }
}

fn dispatch_env_annotate(
    ann_cmd: crate::xun_core::env_cmd::EnvAnnotateCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::env_cmd::*;
    match ann_cmd.sub {
        EnvAnnotateSubCommand::Set(a) => execute(&EnvAnnotateSetCmd { args: a }, ctx, renderer),
        EnvAnnotateSubCommand::List(a) => execute(&EnvAnnotateListCmd { args: a }, ctx, renderer),
    }
}

fn dispatch_env_config(
    cfg_cmd: crate::xun_core::env_cmd::EnvConfigCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::env_cmd::*;
    match cfg_cmd.sub {
        EnvConfigSubCommand::Show(a) => execute(&EnvConfigShowCmd { args: a }, ctx, renderer),
        EnvConfigSubCommand::Path(a) => execute(&EnvConfigPathCmd { args: a }, ctx, renderer),
        EnvConfigSubCommand::Reset(a) => execute(&EnvConfigResetCmd { args: a }, ctx, renderer),
        EnvConfigSubCommand::Get(a) => execute(&EnvConfigGetCmd { args: a }, ctx, renderer),
        EnvConfigSubCommand::Set(a) => execute(&EnvConfigSetCmd { args: a }, ctx, renderer),
    }
}

// ── Alias 路由（feature-gated）───────────────────────────────

#[cfg(feature = "alias")]
fn dispatch_alias(
    alias_cmd: crate::xun_core::alias_cmd::AliasCmd,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::alias_cmd::*;
    let config = alias_cmd.config.clone();
    match alias_cmd.sub {
        AliasSubCommand::Setup(a) => execute(&AliasSetupCmd { args: a, config }, ctx, renderer),
        AliasSubCommand::Add(a) => execute(&AliasAddCmd { args: a, config }, ctx, renderer),
        AliasSubCommand::Rm(a) => execute(&AliasRmCmd { args: a, config }, ctx, renderer),
        AliasSubCommand::List(a) => execute(&AliasListCmd { args: a, config }, ctx, renderer),
        AliasSubCommand::Find(a) => execute(&AliasFindCmd { args: a, config }, ctx, renderer),
        AliasSubCommand::Which(a) => execute(&AliasWhichCmd { args: a, config }, ctx, renderer),
        AliasSubCommand::Sync(_a) => execute(&AliasSyncCmd { config }, ctx, renderer),
        AliasSubCommand::Export(a) => execute(&AliasExportCmd { args: a, config }, ctx, renderer),
        AliasSubCommand::Import(a) => execute(&AliasImportCmd { args: a, config }, ctx, renderer),
        AliasSubCommand::App(app_cmd) => dispatch_alias_app(app_cmd, config, ctx, renderer),
    }
}

#[cfg(feature = "alias")]
fn dispatch_alias_app(
    app_cmd: crate::xun_core::alias_cmd::AliasAppArgs,
    config: Option<String>,
    ctx: &mut CmdContext,
    renderer: &mut dyn crate::xun_core::renderer::Renderer,
) -> Result<Value, XunError> {
    use crate::xun_core::alias_cmd::*;
    match app_cmd.sub {
        AliasAppSubCommand::Add(a) => execute(&AliasAppAddCmd { args: a, config }, ctx, renderer),
        AliasAppSubCommand::Rm(a) => execute(&AliasAppRmCmd { args: a, config }, ctx, renderer),
        AliasAppSubCommand::List(a) => execute(&AliasAppLsCmd { args: a, config }, ctx, renderer),
        AliasAppSubCommand::Scan(a) => execute(&AliasAppScanCmd { args: a, config }, ctx, renderer),
        AliasAppSubCommand::Which(a) => execute(&AliasAppWhichCmd { args: a, config }, ctx, renderer),
        AliasAppSubCommand::Sync(_a) => execute(&AliasAppSyncCmd { config }, ctx, renderer),
    }
}

// ── 便捷入口 ───────────────────────────────────────────────

/// 从命令行参数运行（新架构入口）。
pub fn run_from_args(args: Xun) -> Result<Value, XunError> {
    let mut ctx = CmdContext::new()
        .with_quiet(args.quiet)
        .with_verbose(args.verbose)
        .with_non_interactive(args.non_interactive);

    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();
    let mut renderer = crate::xun_core::renderer::TerminalRenderer::new(args.no_color, &mut stdout_lock);
    dispatch(args.cmd, &mut ctx, &mut renderer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subcommand_variant_count() {
        let base_count = std::mem::size_of::<SubCommand>();
        assert!(base_count > 0, "SubCommand should not be zero-sized");
    }

    #[test]
    fn unmigrated_command_returns_error() {
        let mut ctx = CmdContext::for_test();
        let mut buf = Vec::new();
        let mut renderer = crate::xun_core::renderer::TerminalRenderer::new(true, &mut buf);

        // Use Init with unsupported shell to trigger an error
        let result = dispatch(
            SubCommand::Init(crate::xun_core::init_cmd::InitCmd {
                shell: "unsupported_shell".into(),
            }),
            &mut ctx,
            &mut renderer,
        );

        assert!(result.is_err());
    }

    #[test]
    fn pst_shorthand_parses() {
        let result = Xun::try_parse_from(["xun", "pst"]);
        assert!(result.is_ok());
    }

    #[test]
    fn rm_command_parses() {
        let result = Xun::try_parse_from(["xun", "rm", "--bookmark", "test"]);
        assert!(result.is_ok());
    }

    #[test]
    fn bookmark_subcommand_parses() {
        let result = Xun::try_parse_from(["xun", "bookmark", "list"]);
        assert!(result.is_ok());
    }
}
