use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

use crate::backup::artifact::reader::open_entry_reader;
use crate::backup::artifact::sevenz::{restore_7z_entries, restore_7z_single};
use crate::backup::artifact::source::read_artifact_entries;
use crate::cli::BackupRestoreCmd;
use crate::output::{CliError, CliResult, can_interact};
use crate::path_guard::{PathPolicy, validate_paths};
use serde::Serialize;

use crate::backup::legacy::config as backup_config;
use crate::commands::restore_core;

use backup_config::BackupConfig;

fn restore_timing_enabled() -> bool {
    restore_timing_enabled_with(|name| std::env::var_os(name))
}

fn restore_timing_enabled_with<F>(mut get_env: F) -> bool
where
    F: FnMut(&str) -> Option<OsString>,
{
    ["XUN_CMD_TIMING", "XUN_RESTORE_TIMING"]
        .into_iter()
        .any(|name| get_env(name).is_some())
}

fn emit_restore_timing(label: &str, elapsed: std::time::Duration, extra: Option<String>) {
    match extra {
        Some(extra) if !extra.is_empty() => {
            eprintln!("  [{label:<10}] {:>5}ms  {extra}", elapsed.as_millis());
        }
        _ => eprintln!("  [{label:<10}] {:>5}ms", elapsed.as_millis()),
    }
}

#[derive(Serialize)]
struct RestoreExecutionSummary {
    action: String,
    status: String,
    source: String,
    destination: String,
    mode: String,
    dry_run: bool,
    snapshot: bool,
    restored: usize,
    failed: usize,
}

pub(crate) fn cmd_restore(args: BackupRestoreCmd) -> CliResult {
    let t_total = Instant::now();
    let timing = restore_timing_enabled();

    let t_config = Instant::now();
    let root = match &args.dir {
        Some(d) => PathBuf::from(d),
        None => std::env::current_dir()
            .map_err(|e| CliError::new(1, format!("Failed to get current directory: {e}")))?,
    };
    let cfg = backup_config::load_config(&root);
    let backups_root = root.join(&cfg.storage.backups_dir);
    if timing {
        emit_restore_timing(
            "config",
            t_config.elapsed(),
            Some(root.display().to_string()),
        );
    }

    // 解析备份源路径
    let t_source = Instant::now();
    let src = resolve_backup_src(&backups_root, &args.name_or_path)?;
    if timing {
        emit_restore_timing(
            "source",
            t_source.elapsed(),
            Some(src.display().to_string()),
        );
    }

    // --snapshot：还原前先备份当前状态
    if args.snapshot && !args.dry_run {
        eprintln!("Creating pre-restore snapshot...");
        let t_snapshot = Instant::now();
        run_snapshot_backup(&root, &cfg)?;
        if timing {
            emit_restore_timing("snapshot", t_snapshot.elapsed(), None);
        }
    }

    // 确定目标根目录
    let t_dest = Instant::now();
    let dest_root = match &args.to {
        Some(d) => {
            let p = PathBuf::from(d);
            if !args.dry_run {
                let _ = fs::create_dir_all(&p);
            }
            p
        }
        None => root.clone(),
    };
    if timing {
        emit_restore_timing(
            "dest",
            t_dest.elapsed(),
            Some(dest_root.display().to_string()),
        );
    }

    // 交互确认
    if !args.yes && can_interact() {
        let t_preview = Instant::now();
        show_restore_preview(
            &dest_root,
            &src,
            PreviewMode::from_args(args.file.as_deref(), args.glob.as_deref()),
        );
        let ok = dialoguer::Confirm::new()
            .with_prompt("Restore may overwrite files. Continue?")
            .default(false)
            .interact()
            .unwrap_or(false);
        if !ok {
            return Err(CliError::new(3, "Cancelled."));
        }
        if timing {
            emit_restore_timing("preview", t_preview.elapsed(), None);
        }
    }

    let t_start = Instant::now();
    let (restored, failed) = if is_xunbak_path(&src) {
        #[cfg(feature = "xunbak")]
        {
            crate::backup::app::xunbak::restore_container(
                &src,
                &dest_root,
                args.file.as_deref(),
                args.glob.as_deref(),
            )?
        }
        #[cfg(not(feature = "xunbak"))]
        {
            return Err(CliError::with_details(
                2,
                "xunbak restore is not enabled in this build",
                &["Fix: Rebuild with `--features xunbak`."],
            ));
        }
    } else if let Some(ref glob_pat) = args.glob {
        restore_with_glob(&src, &dest_root, glob_pat, args.dry_run)?
    } else if let Some(ref file) = args.file {
        restore_single_file(&src, &dest_root, file, args.dry_run)?
    } else {
        restore_all(&src, &dest_root, args.dry_run)?
    };

    if args.json {
        let mode = if args.glob.is_some() {
            "glob"
        } else if args.file.is_some() {
            "file"
        } else {
            "all"
        };
        let action = if args.dry_run { "preview" } else { "restore" };
        let summary = RestoreExecutionSummary {
            action: action.to_string(),
            status: if failed == 0 {
                "ok".to_string()
            } else {
                "partial_failed".to_string()
            },
            source: src.display().to_string(),
            destination: dest_root.display().to_string(),
            mode: mode.to_string(),
            dry_run: args.dry_run,
            snapshot: args.snapshot && !args.dry_run,
            restored,
            failed,
        };
        out_println!(
            "{}",
            serde_json::to_string_pretty(&summary).unwrap_or_default()
        );
    }

    let elapsed = t_start.elapsed();
    if timing {
        let mode = if args.glob.is_some() {
            "glob"
        } else if args.file.is_some() {
            "file"
        } else {
            "all"
        };
        emit_restore_timing(
            "execute",
            elapsed,
            Some(format!("mode={mode} restored={restored} failed={failed}")),
        );
        emit_restore_timing("total", t_total.elapsed(), None);
    }
    eprintln!(
        "Restored: {}  Failed: {}  Time: {:.2}s",
        restored,
        failed,
        elapsed.as_secs_f64()
    );
    if failed > 0 {
        return Err(CliError::new(
            1,
            format!("{failed} file(s) failed to restore."),
        ));
    }
    Ok(())
}

/// 解析备份源路径：直接路径 > 备份目录查找
fn resolve_backup_src(backups_root: &Path, name_or_path: &str) -> Result<PathBuf, CliError> {
    let p = PathBuf::from(name_or_path);
    if p.is_dir()
        || p.is_file()
        || p.extension().and_then(|e| e.to_str()) == Some("xunbak")
            && PathBuf::from(format!("{}.001", p.display())).exists()
        || p.extension().and_then(|e| e.to_str()) == Some("7z")
            && PathBuf::from(format!("{}.001", p.display())).exists()
    {
        return Ok(p);
    }
    backup_source_path(backups_root, name_or_path).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Backup not found: {name_or_path}"),
            &[
                "Fix: Run `xun backup list` to see available backups.",
                "Fix: Pass a direct path to a backup dir, .zip, .7z, or .xunbak file.",
            ],
        )
    })
}

fn is_xunbak_path(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("xunbak")
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".xunbak.001"))
}

fn backup_source_path(backups_root: &Path, name: &str) -> Option<PathBuf> {
    let candidate = backups_root.join(name);
    if candidate.is_dir() || candidate.is_file() {
        return Some(candidate);
    }
    let zip = backups_root.join(format!("{name}.zip"));
    if zip.is_file() {
        return Some(zip);
    }
    let sevenz = backups_root.join(format!("{name}.7z"));
    if sevenz.is_file() {
        return Some(sevenz);
    }
    None
}

/// 全量还原，返回 (restored, failed)
fn restore_all(src: &Path, dest_root: &Path, dry_run: bool) -> Result<(usize, usize), CliError> {
    if src.extension().and_then(|e| e.to_str()) == Some("zip") {
        restore_core::restore_many_from_zip(src, dest_root, dry_run, |_| true)
    } else if is_7z_path(src) {
        restore_7z_entries(src, dest_root, dry_run, |_| true)
    } else {
        Ok(restore_core::restore_many_from_dir(
            src,
            dest_root,
            dry_run,
            |_, _| true,
        ))
    }
}

/// 单文件还原
fn restore_single_file(
    src: &Path,
    dest_root: &Path,
    file: &str,
    dry_run: bool,
) -> Result<(usize, usize), CliError> {
    let rel = PathBuf::from(file);
    if rel.is_absolute() || rel.components().any(|c| c == Component::ParentDir) {
        return Err(CliError::with_details(
            2,
            format!("Unsafe restore path: {file}"),
            &["Fix: Use a relative path without '..' components."],
        ));
    }

    let mut policy = PathPolicy::for_output();
    policy.allow_relative = true;
    let result = validate_paths([file], &policy);
    if !result.issues.is_empty() {
        return Err(CliError::with_details(
            2,
            format!("Unsafe restore path: {file}"),
            &["Fix: Use a relative path without '..' components."],
        ));
    }

    if src.extension().and_then(|e| e.to_str()) == Some("zip") {
        restore_core::restore_from_zip(src, dest_root, Some(&rel), dry_run)?;
    } else if is_7z_path(src) {
        restore_7z_single(src, dest_root, &rel.to_string_lossy(), dry_run)?;
    } else {
        restore_core::restore_from_dir(src, dest_root, Some(&rel), dry_run)?;
    }
    Ok((1, 0))
}

/// restore 前展示将被覆盖的文件列表（modify/new 文件数）
fn show_restore_preview(root: &Path, backup_src: &Path, mode: PreviewMode<'_>) {
    let entries = match build_restore_preview_items(backup_src, root, mode) {
        Ok(items) => items,
        Err(err) => {
            eprintln!("  preview unavailable: {}", err.message);
            return;
        }
    };

    let overwrite_count = entries
        .iter()
        .filter(|e| e.kind == RestorePreviewKind::Overwrite)
        .count();
    let new_count = entries
        .iter()
        .filter(|e| e.kind == RestorePreviewKind::New)
        .count();

    if overwrite_count == 0 && new_count == 0 {
        eprintln!("  (no files will be changed)");
        return;
    }

    eprintln!("Files to be restored:");
    let max_show = 20usize;
    let mut shown = 0;
    for e in &entries {
        if shown < max_show {
            let tag = match e.kind {
                RestorePreviewKind::Overwrite => "overwrite",
                RestorePreviewKind::New => "new",
            };
            eprintln!("  [{tag}] {}", e.rel);
        }
        shown += 1;
    }
    if shown > max_show {
        eprintln!("  ... and {} more", shown - max_show);
    }
    eprintln!("  Total: {} overwrite, {} new", overwrite_count, new_count);
}

#[derive(Debug, Clone, Copy)]
enum PreviewMode<'a> {
    All,
    File(&'a str),
    Glob(&'a str),
}

impl<'a> PreviewMode<'a> {
    fn from_args(file: Option<&'a str>, glob: Option<&'a str>) -> Self {
        match (file, glob) {
            (Some(file), _) => Self::File(file),
            (None, Some(glob)) => Self::Glob(glob),
            (None, None) => Self::All,
        }
    }

    fn matches_path(&self, rel: &str) -> bool {
        match self {
            PreviewMode::All => true,
            PreviewMode::File(file) => rel.eq_ignore_ascii_case(&file.replace('/', "\\")),
            PreviewMode::Glob(glob) => glob_match(glob, &rel.replace('\\', "/")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RestorePreviewKind {
    Overwrite,
    New,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RestorePreviewItem {
    rel: String,
    kind: RestorePreviewKind,
}

fn build_restore_preview_items(
    backup_src: &Path,
    dest_root: &Path,
    mode: PreviewMode<'_>,
) -> Result<Vec<RestorePreviewItem>, CliError> {
    if backup_src.extension().and_then(|e| e.to_str()) == Some("zip") {
        build_restore_preview_items_from_zip(backup_src, dest_root, mode)
    } else if is_7z_path(backup_src) {
        build_restore_preview_items_from_artifact(backup_src, dest_root, mode)
    } else {
        build_restore_preview_items_from_dir(backup_src, dest_root, mode)
    }
}

fn build_restore_preview_items_from_dir(
    backup_src: &Path,
    dest_root: &Path,
    mode: PreviewMode<'_>,
) -> Result<Vec<RestorePreviewItem>, CliError> {
    let mut items = Vec::new();
    let mut stack = vec![backup_src.to_path_buf()];

    while let Some(current) = stack.pop() {
        let read_dir = fs::read_dir(&current)
            .map_err(|e| CliError::new(1, format!("Preview scan failed: {e}")))?;
        for entry in read_dir.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            let rel = match path.strip_prefix(backup_src) {
                Ok(rel) => rel.to_string_lossy().replace('/', "\\"),
                Err(_) => continue,
            };
            if restore_core::is_backup_internal_name(&rel) || !mode.matches_path(&rel) {
                continue;
            }

            let dst = dest_root.join(rel.replace('\\', std::path::MAIN_SEPARATOR_STR));
            if let Some(item) = classify_preview_item(&path, &dst, &rel)? {
                items.push(item);
            }
        }
    }

    items.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(items)
}

fn build_restore_preview_items_from_zip(
    backup_src: &Path,
    dest_root: &Path,
    mode: PreviewMode<'_>,
) -> Result<Vec<RestorePreviewItem>, CliError> {
    let file = fs::File::open(backup_src)
        .map_err(|e| CliError::new(1, format!("Preview open zip failed: {e}")))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| CliError::new(1, format!("Preview read zip failed: {e}")))?;
    let mut items = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| CliError::new(1, format!("Preview zip entry failed: {e}")))?;
        if entry.is_dir() {
            continue;
        }
        let rel = entry.name().replace('/', "\\");
        if !restore_core::is_safe_zip_entry(entry.name())
            || restore_core::is_backup_internal_name(&rel)
            || !mode.matches_path(&rel)
        {
            continue;
        }

        let dst = dest_root.join(rel.replace('\\', std::path::MAIN_SEPARATOR_STR));
        if let Some(item) = classify_preview_item_zip(&mut entry, &dst, &rel)? {
            items.push(item);
        }
    }

    items.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(items)
}

fn build_restore_preview_items_from_artifact(
    backup_src: &Path,
    dest_root: &Path,
    mode: PreviewMode<'_>,
) -> Result<Vec<RestorePreviewItem>, CliError> {
    let entries = read_artifact_entries(backup_src)?;
    let mut items = Vec::new();
    for entry in entries {
        let rel = entry.path.replace('/', "\\");
        if !mode.matches_path(&rel) {
            continue;
        }
        let dst = dest_root.join(rel.replace('\\', std::path::MAIN_SEPARATOR_STR));
        let mut reader = open_entry_reader(&entry)?;
        if let Some(item) = classify_preview_item_zip(&mut reader, &dst, &rel)? {
            items.push(item);
        }
    }
    items.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(items)
}

fn classify_preview_item(
    src: &Path,
    dst: &Path,
    rel: &str,
) -> Result<Option<RestorePreviewItem>, CliError> {
    if !dst.exists() {
        return Ok(Some(RestorePreviewItem {
            rel: rel.to_string(),
            kind: RestorePreviewKind::New,
        }));
    }

    if paths_differ(src, dst)? {
        return Ok(Some(RestorePreviewItem {
            rel: rel.to_string(),
            kind: RestorePreviewKind::Overwrite,
        }));
    }

    Ok(None)
}

fn classify_preview_item_zip<R: Read>(
    entry: &mut R,
    dst: &Path,
    rel: &str,
) -> Result<Option<RestorePreviewItem>, CliError> {
    if !dst.exists() {
        return Ok(Some(RestorePreviewItem {
            rel: rel.to_string(),
            kind: RestorePreviewKind::New,
        }));
    }

    if reader_differs_from_file(entry, dst)? {
        return Ok(Some(RestorePreviewItem {
            rel: rel.to_string(),
            kind: RestorePreviewKind::Overwrite,
        }));
    }

    Ok(None)
}

fn paths_differ(src: &Path, dst: &Path) -> Result<bool, CliError> {
    let src_meta =
        fs::metadata(src).map_err(|e| CliError::new(1, format!("Preview read failed: {e}")))?;
    let dst_meta =
        fs::metadata(dst).map_err(|e| CliError::new(1, format!("Preview read failed: {e}")))?;
    if !src_meta.is_file() || !dst_meta.is_file() {
        return Ok(true);
    }
    if src_meta.len() != dst_meta.len() {
        return Ok(true);
    }

    let mut src_file =
        fs::File::open(src).map_err(|e| CliError::new(1, format!("Preview open failed: {e}")))?;
    reader_differs_from_file(&mut src_file, dst)
}

fn reader_differs_from_file<R: Read>(reader: &mut R, dst: &Path) -> Result<bool, CliError> {
    let dst_meta =
        fs::metadata(dst).map_err(|e| CliError::new(1, format!("Preview read failed: {e}")))?;
    if !dst_meta.is_file() {
        return Ok(true);
    }
    let mut dst_file =
        fs::File::open(dst).map_err(|e| CliError::new(1, format!("Preview open failed: {e}")))?;

    let mut src_buf = [0u8; 8192];
    let mut dst_buf = [0u8; 8192];
    loop {
        let src_read = reader
            .read(&mut src_buf)
            .map_err(|e| CliError::new(1, format!("Preview read failed: {e}")))?;
        let dst_read = dst_file
            .read(&mut dst_buf)
            .map_err(|e| CliError::new(1, format!("Preview read failed: {e}")))?;
        if src_read != dst_read {
            return Ok(true);
        }
        if src_read == 0 {
            return Ok(false);
        }
        if src_buf[..src_read] != dst_buf[..dst_read] {
            return Ok(true);
        }
    }
}

/// glob 模式还原
fn restore_with_glob(
    src: &Path,
    dest_root: &Path,
    glob_pat: &str,
    dry_run: bool,
) -> Result<(usize, usize), CliError> {
    if src.extension().and_then(|e| e.to_str()) == Some("zip") {
        restore_core::restore_many_from_zip(src, dest_root, dry_run, |name| {
            glob_match(glob_pat, name)
        })
    } else if is_7z_path(src) {
        restore_7z_entries(src, dest_root, dry_run, |name| glob_match(glob_pat, name))
    } else {
        Ok(restore_core::restore_many_from_dir(
            src,
            dest_root,
            dry_run,
            |_, rel_str| glob_match(glob_pat, rel_str),
        ))
    }
}

fn is_7z_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("7z"))
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".7z.001"))
}

/// snapshot：调用 cmd_backup 备份当前状态（desc = pre_restore）
fn run_snapshot_backup(root: &Path, _cfg: &BackupConfig) -> CliResult {
    use crate::cli::BackupCmd;
    let args = BackupCmd {
        cmd: None,
        msg: Some("pre_restore".to_string()),
        dir: Some(root.to_string_lossy().into_owned()),
        container: None,
        compression: None,
        split_size: None,
        dry_run: false,
        list: false,
        no_compress: false,
        retain: None,
        include: vec![],
        exclude: vec![],
        incremental: false,
        skip_if_unchanged: false,
        diff_mode: None,
        json: false,
    };
    crate::backup::app::create::cmd_backup(args)
}

/// 简易 glob 匹配（无外部依赖）
/// - `*`  匹配单段内任意字符（不跨 `/`）
/// - `**` 跨目录匹配（可匹配零或多个路径段）
/// - `?`  匹配单个任意字符（不含 `/`）
fn glob_match(pattern: &str, path: &str) -> bool {
    glob_match_parts(pattern.as_bytes(), path.as_bytes())
}

fn glob_match_parts(pat: &[u8], s: &[u8]) -> bool {
    if pat.starts_with(b"**") {
        let rest_pat = if pat.len() > 2 && pat[2] == b'/' {
            &pat[3..]
        } else {
            &pat[2..]
        };
        if glob_match_parts(rest_pat, s) {
            return true;
        }
        let mut i = 0;
        while i < s.len() {
            if s[i] == b'/' && glob_match_parts(pat, &s[i + 1..]) {
                return true;
            }
            i += 1;
        }
        return false;
    }

    match (pat.first(), s.first()) {
        (None, None) => true,
        (None, _) | (_, None) if pat == b"*" => true,
        (None, _) | (Some(_), None) => false,
        (Some(b'*'), _) => {
            if s[0] == b'/' {
                return false;
            }
            if glob_match_parts(&pat[1..], s) {
                return true;
            }
            glob_match_parts(pat, &s[1..])
        }
        (Some(b'?'), _) => {
            if s[0] == b'/' {
                return false;
            }
            glob_match_parts(&pat[1..], &s[1..])
        }
        (Some(p), Some(c)) => {
            if p == c {
                glob_match_parts(&pat[1..], &s[1..])
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::io::Write;
    use tempfile::tempdir;

    use super::{
        PreviewMode, RestorePreviewItem, RestorePreviewKind, build_restore_preview_items,
        glob_match, restore_timing_enabled_with,
    };

    #[test]
    fn glob_exact() {
        assert!(glob_match("src/main.rs", "src/main.rs"));
        assert!(!glob_match("src/main.rs", "src/lib.rs"));
    }

    #[test]
    fn glob_star() {
        assert!(glob_match("*.ts", "foo.ts"));
        assert!(!glob_match("*.ts", "src/foo.ts"));
    }

    #[test]
    fn glob_double_star() {
        assert!(glob_match("**/*.ts", "src/foo.ts"));
        assert!(glob_match("**/*.ts", "a/b/c/foo.ts"));
        assert!(!glob_match("**/*.ts", "a/b/c/foo.rs"));
    }

    #[test]
    fn glob_question() {
        assert!(glob_match("src/?.rs", "src/a.rs"));
        assert!(!glob_match("src/?.rs", "src/ab.rs"));
    }

    #[test]
    fn restore_preview_items_respect_file_mode() {
        let backup = tempdir().unwrap();
        let dest = tempdir().unwrap();
        std::fs::write(backup.path().join("a.txt"), "backup-a").unwrap();
        std::fs::write(backup.path().join("b.txt"), "backup-b").unwrap();
        std::fs::write(dest.path().join("a.txt"), "current-a").unwrap();

        let items =
            build_restore_preview_items(backup.path(), dest.path(), PreviewMode::File("a.txt"))
                .unwrap();
        assert_eq!(
            items,
            vec![RestorePreviewItem {
                rel: "a.txt".to_string(),
                kind: RestorePreviewKind::Overwrite,
            }]
        );
    }

    #[test]
    fn restore_preview_items_detect_same_size_content_change() {
        let backup = tempdir().unwrap();
        let dest = tempdir().unwrap();
        std::fs::write(backup.path().join("same.txt"), "aaaa").unwrap();
        std::fs::write(dest.path().join("same.txt"), "bbbb").unwrap();

        let items =
            build_restore_preview_items(backup.path(), dest.path(), PreviewMode::All).unwrap();
        assert_eq!(
            items,
            vec![RestorePreviewItem {
                rel: "same.txt".to_string(),
                kind: RestorePreviewKind::Overwrite,
            }]
        );
    }

    #[test]
    fn restore_preview_items_zip_respect_glob_and_skip_unchanged() {
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("preview.zip");
        let dest = tempdir().unwrap();

        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("src/a.txt", options).unwrap();
        writer.write_all(b"same").unwrap();
        writer.start_file("src/b.rs", options).unwrap();
        writer.write_all(b"skip").unwrap();
        writer.start_file("../evil.txt", options).unwrap();
        writer.write_all(b"bad").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        std::fs::write(&zip_path, bytes).unwrap();

        std::fs::create_dir_all(dest.path().join("src")).unwrap();
        std::fs::write(dest.path().join("src").join("a.txt"), "same").unwrap();

        let items =
            build_restore_preview_items(&zip_path, dest.path(), PreviewMode::Glob("**/*.txt"))
                .unwrap();
        assert!(
            items.is_empty(),
            "unchanged matched file should not appear in preview"
        );
    }

    #[test]
    fn restore_timing_enabled_accepts_command_and_restore_env_names() {
        let env = HashMap::from([("XUN_CMD_TIMING", OsString::from("1"))]);
        assert!(restore_timing_enabled_with(|name| env.get(name).cloned()));

        let env = HashMap::from([("XUN_RESTORE_TIMING", OsString::from("1"))]);
        assert!(restore_timing_enabled_with(|name| env.get(name).cloned()));
    }
}
