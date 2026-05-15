//! Backup 业务逻辑服务
//!
//! 封装备份管理操作，供 CommandSpec 实现调用。
//! 桥接旧 backup 模块的 CliResult API 到新的 Result<Value, XunError> 模式。

use std::path::{Path, PathBuf};

use crate::xun_core::error::XunError;
use crate::xun_core::value::Value;

// ── 辅助函数 ────────────────────────────────────────────────

fn resolve_root(dir: Option<&str>) -> Result<PathBuf, XunError> {
    match dir {
        Some(d) => Ok(PathBuf::from(d)),
        None => std::env::current_dir().map_err(|e| {
            XunError::Internal(anyhow::anyhow!("Failed to get current directory: {e}"))
        }),
    }
}

fn load_backup_config(root: &Path) -> crate::backup::legacy::config::BackupConfig {
    crate::backup::legacy::config::load_config(root)
}

// ── 创建备份 ────────────────────────────────────────────────

/// 创建备份（传统目录模式）。
///
/// 桥接到 `crate::commands::backup::cmd_backup`。
/// 旧函数直接打印到 stdout，此处调用后返回成功值。
pub fn create_backup(
    msg: Option<&str>,
    dir: Option<&str>,
    dry_run: bool,
    list: bool,
    no_compress: bool,
    retain: Option<usize>,
    include: &[String],
    exclude: &[String],
    incremental: bool,
    skip_if_unchanged: bool,
    diff_mode: Option<&str>,
    json: bool,
) -> Result<Value, XunError> {
    let args = crate::cli::BackupCmd {
        cmd: None,
        msg: msg.map(|s| s.to_string()),
        dir: dir.map(|s| s.to_string()),
        container: None,
        compression: None,
        split_size: None,
        dry_run,
        list,
        no_compress,
        retain,
        include: include.to_vec(),
        exclude: exclude.to_vec(),
        incremental,
        skip_if_unchanged,
        diff_mode: diff_mode.map(|s| s.to_string()),
        json,
    };
    crate::commands::backup::cmd_backup(args)?;
    Ok(Value::String("backup created".to_string()))
}

/// 创建备份（新格式子命令 add/create）。
///
/// 桥接到 `crate::backup::app::create::cmd_backup_create`。
pub fn create_backup_artifact(
    msg: Option<&str>,
    dir: Option<&str>,
    format: Option<&str>,
    output: Option<&str>,
    compression: Option<&str>,
    split_size: Option<&str>,
    dry_run: bool,
    list: bool,
    no_compress: bool,
    retain: Option<usize>,
    include: &[String],
    exclude: &[String],
    incremental: bool,
    skip_if_unchanged: bool,
    diff_mode: Option<&str>,
    progress: Option<&str>,
    json: bool,
    no_sidecar: bool,
) -> Result<Value, XunError> {
    let cmd = crate::cli::BackupCreateCmd {
        msg: msg.map(|s| s.to_string()),
        dir: dir.map(|s| s.to_string()),
        format: format.map(|s| s.to_string()),
        output: output.map(|s| s.to_string()),
        compression: compression.map(|s| s.to_string()),
        split_size: split_size.map(|s| s.to_string()),
        solid: false,
        method: None,
        level: None,
        dry_run,
        list,
        no_compress,
        retain,
        include: include.to_vec(),
        exclude: exclude.to_vec(),
        incremental,
        skip_if_unchanged,
        diff_mode: diff_mode.map(|s| s.to_string()),
        progress: progress.map(|s| s.to_string()),
        json,
        no_sidecar,
    };
    crate::backup::app::create::cmd_backup_create(cmd)?;
    Ok(Value::String("backup artifact created".to_string()))
}

// ── 恢复备份 ────────────────────────────────────────────────

/// 从备份恢复。
///
/// 桥接到 `crate::backup::app::restore::cmd_restore`。
pub fn restore_backup(
    name_or_path: &str,
    file: Option<&str>,
    glob: Option<&str>,
    to: Option<&str>,
    snapshot: bool,
    dir: Option<&str>,
    dry_run: bool,
    yes: bool,
    json: bool,
) -> Result<Value, XunError> {
    let cmd = crate::cli::BackupRestoreCmd {
        name_or_path: name_or_path.to_string(),
        file: file.map(|s| s.to_string()),
        glob: glob.map(|s| s.to_string()),
        to: to.map(|s| s.to_string()),
        snapshot,
        dir: dir.map(|s| s.to_string()),
        dry_run,
        yes,
        json,
    };
    crate::backup::app::restore::cmd_restore(cmd)?;
    Ok(Value::String("backup restored".to_string()))
}

// ── 转换备份 ────────────────────────────────────────────────

/// 转换备份格式。
///
/// 桥接到 `crate::backup::app::convert::cmd_backup_convert`。
pub fn convert_backup(
    artifact: &str,
    format: &str,
    output: &str,
    file: &[String],
    glob: &[String],
    split_size: Option<&str>,
    level: Option<u32>,
    dry_run: bool,
    list: bool,
    json: bool,
) -> Result<Value, XunError> {
    let cmd = crate::cli::BackupConvertCmd {
        artifact: artifact.to_string(),
        format: format.to_string(),
        output: output.to_string(),
        file: file.to_vec(),
        glob: glob.to_vec(),
        patterns_from: Vec::new(),
        split_size: split_size.map(|s| s.to_string()),
        solid: false,
        method: None,
        level,
        threads: None,
        password: None,
        encrypt_header: false,
        overwrite: None,
        dry_run,
        list,
        verify_source: None,
        verify_output: None,
        progress: None,
        json,
        no_sidecar: false,
    };
    crate::backup::app::convert::cmd_backup_convert(cmd)?;
    Ok(Value::String("backup converted".to_string()))
}

// ── 列出备份 ────────────────────────────────────────────────

/// 列出可用备份。
///
/// 桥接到 `crate::backup::legacy::list::cmd_backup_list`。
pub fn list_backups(dir: Option<&str>, json: bool) -> Result<Value, XunError> {
    let root = resolve_root(dir)?;
    let cfg = load_backup_config(&root);
    crate::backup::legacy::list::cmd_backup_list(&root, &cfg, json)?;
    Ok(Value::String("backup list".to_string()))
}

// ── 验证备份 ────────────────────────────────────────────────

/// 验证备份完整性。
///
/// 桥接到 `crate::backup::legacy::verify::cmd_backup_verify`。
pub fn verify_backup(name: &str, dir: Option<&str>, json: bool) -> Result<Value, XunError> {
    let root = resolve_root(dir)?;
    let cfg = load_backup_config(&root);
    crate::backup::legacy::verify::cmd_backup_verify(&root, &cfg, name, json)
        ?;
    Ok(Value::String("backup verified".to_string()))
}

// ── 查找备份 ────────────────────────────────────────────────

/// 按标签/时间查找备份。
///
/// 桥接到 `crate::backup::legacy::find::cmd_backup_find`。
pub fn find_backup(
    tag: Option<&str>,
    since: Option<&str>,
    until: Option<&str>,
    dir: Option<&str>,
    json: bool,
) -> Result<Value, XunError> {
    let root = resolve_root(dir)?;
    let cfg = load_backup_config(&root);
    let since_bound = crate::backup::legacy::find::parse_time_filter_bound(since, false)
        ?;
    let until_bound = crate::backup::legacy::find::parse_time_filter_bound(until, true)
        ?;
    crate::backup::legacy::find::cmd_backup_find(&root, &cfg, tag, since_bound, until_bound, json)
        ?;
    Ok(Value::String("backup find".to_string()))
}
