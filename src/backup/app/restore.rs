use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

use crate::backup::artifact::common::{
    is_7z_artifact_path, is_xunbak_artifact_path, is_zip_artifact_path,
};
use crate::backup::artifact::entry::{file_attributes, system_time_to_unix_ns};
use crate::backup::artifact::reader::{open_entry_reader, sort_entries_for_read_locality};
use crate::backup::artifact::sevenz::{restore_7z_entries, restore_7z_single};
use crate::backup::artifact::source::read_artifact_entries;
use crate::backup::common::cli::{
    backup_named_artifact_path, backup_not_found_error, path_display, unsafe_restore_path_error,
};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RestoreStats {
    restored: usize,
    failed: usize,
}

impl RestoreStats {
    fn new(restored: usize, failed: usize) -> Self {
        Self { restored, failed }
    }

    fn single_success() -> Self {
        Self::new(1, 0)
    }

    fn status_label(&self) -> &'static str {
        if self.failed == 0 {
            "ok"
        } else {
            "partial_failed"
        }
    }
}

impl From<(usize, usize)> for RestoreStats {
    fn from(value: (usize, usize)) -> Self {
        Self::new(value.0, value.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RestoreArtifactKind {
    Dir,
    Zip,
    SevenZ,
    Xunbak,
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
        let mode = RestoreMode::from_args(args.file.as_deref(), args.glob.as_deref());
        let source_kind = detect_restore_artifact_kind(&src);
        show_restore_preview(&dest_root, &src, source_kind, mode);
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
    let mode = RestoreMode::from_args(args.file.as_deref(), args.glob.as_deref());
    let source_kind = detect_restore_artifact_kind(&src);
    let stats = execute_restore_request(&src, source_kind, &dest_root, mode, args.dry_run)?;

    if args.json {
        let summary = build_restore_execution_summary(
            &src,
            &dest_root,
            mode,
            args.dry_run,
            args.snapshot && !args.dry_run,
            stats,
        );
        out_println!(
            "{}",
            serde_json::to_string_pretty(&summary).unwrap_or_default()
        );
    }

    let elapsed = t_start.elapsed();
    if timing {
        emit_restore_timing(
            "execute",
            elapsed,
            Some(format!(
                "mode={} restored={} failed={}",
                mode.label(),
                stats.restored,
                stats.failed
            )),
        );
        emit_restore_timing("total", t_total.elapsed(), None);
    }
    eprintln!(
        "Restored: {}  Failed: {}  Time: {:.2}s",
        stats.restored,
        stats.failed,
        elapsed.as_secs_f64()
    );
    if stats.failed > 0 {
        return Err(CliError::new(
            1,
            format!("{} file(s) failed to restore.", stats.failed),
        ));
    }
    Ok(())
}

/// 解析备份源路径：直接路径 > 备份目录查找
fn resolve_backup_src(backups_root: &Path, name_or_path: &str) -> Result<PathBuf, CliError> {
    let p = PathBuf::from(name_or_path);
    if p.is_dir()
        || p.is_file()
        || is_xunbak_artifact_path(&p) && PathBuf::from(format!("{}.001", p.display())).exists()
        || is_7z_artifact_path(&p) && PathBuf::from(format!("{}.001", p.display())).exists()
    {
        return Ok(p);
    }
    backup_named_artifact_path(backups_root, name_or_path)
        .ok_or_else(|| backup_not_found_error(name_or_path))
}

fn detect_restore_artifact_kind(path: &Path) -> RestoreArtifactKind {
    if is_xunbak_artifact_path(path) {
        RestoreArtifactKind::Xunbak
    } else if is_zip_artifact_path(path) {
        RestoreArtifactKind::Zip
    } else if is_7z_artifact_path(path) {
        RestoreArtifactKind::SevenZ
    } else {
        RestoreArtifactKind::Dir
    }
}

fn validate_restore_file_request(file: &str) -> Result<PathBuf, CliError> {
    let rel = PathBuf::from(file);
    if rel.is_absolute() || rel.components().any(|c| c == Component::ParentDir) {
        return Err(unsafe_restore_path_error(file));
    }

    let mut policy = PathPolicy::for_output();
    policy.allow_relative = true;
    let result = validate_paths([file], &policy);
    if !result.issues.is_empty() {
        return Err(unsafe_restore_path_error(file));
    }

    Ok(rel)
}

fn execute_restore_request(
    src: &Path,
    source_kind: RestoreArtifactKind,
    dest_root: &Path,
    mode: RestoreMode<'_>,
    dry_run: bool,
) -> Result<RestoreStats, CliError> {
    match source_kind {
        RestoreArtifactKind::Xunbak => {
            #[cfg(feature = "xunbak")]
            {
                crate::backup::app::xunbak::restore_container(
                    src,
                    dest_root,
                    mode.selected_file(),
                    mode.selected_glob(),
                    dry_run,
                )
                .map(Into::into)
            }
            #[cfg(not(feature = "xunbak"))]
            {
                Err(CliError::with_details(
                    2,
                    "xunbak restore is not enabled in this build",
                    &["Fix: Rebuild with `--features xunbak`."],
                ))
            }
        }
        RestoreArtifactKind::Zip => match mode {
            RestoreMode::All => {
                restore_core::restore_many_from_zip(src, dest_root, dry_run, |_| true)
                    .map(Into::into)
            }
            RestoreMode::File(file) => {
                let rel = validate_restore_file_request(file)?;
                restore_core::restore_from_zip(src, dest_root, Some(&rel), dry_run)?;
                Ok(RestoreStats::single_success())
            }
            RestoreMode::Glob(glob_pat) => {
                restore_core::restore_many_from_zip(src, dest_root, dry_run, |name| {
                    glob_match(glob_pat, name)
                })
                .map(Into::into)
            }
        },
        RestoreArtifactKind::SevenZ => match mode {
            RestoreMode::All => {
                restore_7z_entries(src, dest_root, dry_run, |_| true).map(Into::into)
            }
            RestoreMode::File(file) => {
                let rel = validate_restore_file_request(file)?;
                restore_7z_single(src, dest_root, &rel.to_string_lossy(), dry_run)?;
                Ok(RestoreStats::single_success())
            }
            RestoreMode::Glob(glob_pat) => {
                restore_7z_entries(src, dest_root, dry_run, |name| glob_match(glob_pat, name))
                    .map(Into::into)
            }
        },
        RestoreArtifactKind::Dir => match mode {
            RestoreMode::All => {
                Ok(
                    restore_core::restore_many_from_dir(src, dest_root, dry_run, |_, _| true)
                        .into(),
                )
            }
            RestoreMode::File(file) => {
                let rel = validate_restore_file_request(file)?;
                restore_core::restore_from_dir(src, dest_root, Some(&rel), dry_run)?;
                Ok(RestoreStats::single_success())
            }
            RestoreMode::Glob(glob_pat) => Ok(restore_core::restore_many_from_dir(
                src,
                dest_root,
                dry_run,
                |_, rel_str| glob_match(glob_pat, rel_str),
            )
            .into()),
        },
    }
}

fn build_restore_execution_summary(
    source: &Path,
    destination: &Path,
    mode: RestoreMode<'_>,
    dry_run: bool,
    snapshot: bool,
    stats: RestoreStats,
) -> RestoreExecutionSummary {
    RestoreExecutionSummary {
        action: if dry_run {
            "preview".to_string()
        } else {
            "restore".to_string()
        },
        status: stats.status_label().to_string(),
        source: path_display(source),
        destination: path_display(destination),
        mode: mode.label().to_string(),
        dry_run,
        snapshot,
        restored: stats.restored,
        failed: stats.failed,
    }
}

/// restore 前展示将被覆盖的文件列表（modify/new 文件数）
fn show_restore_preview(
    root: &Path,
    backup_src: &Path,
    source_kind: RestoreArtifactKind,
    mode: RestoreMode<'_>,
) {
    let entries = match build_restore_preview_items_for_kind(backup_src, source_kind, root, mode) {
        Ok(items) => items,
        Err(err) => {
            eprintln!("  preview unavailable: {}", err.message);
            return;
        }
    };
    let summary = build_restore_preview_summary(&entries, 20);
    if summary.is_empty() {
        eprintln!("  (no files will be changed)");
        return;
    }

    eprintln!("Files to be restored:");
    for e in &summary.visible_items {
        let tag = match e.kind {
            RestorePreviewKind::Overwrite => "overwrite",
            RestorePreviewKind::New => "new",
        };
        eprintln!("  [{tag}] {}", e.rel);
    }
    if summary.hidden_count > 0 {
        eprintln!("  ... and {} more", summary.hidden_count);
    }
    eprintln!(
        "  Total: {} overwrite, {} new",
        summary.overwrite_count, summary.new_count
    );
}

#[derive(Debug, Clone, Copy)]
enum RestoreMode<'a> {
    All,
    File(&'a str),
    Glob(&'a str),
}

impl<'a> RestoreMode<'a> {
    fn from_args(file: Option<&'a str>, glob: Option<&'a str>) -> Self {
        match (file, glob) {
            (Some(file), _) => Self::File(file),
            (None, Some(glob)) => Self::Glob(glob),
            (None, None) => Self::All,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::File(_) => "file",
            Self::Glob(_) => "glob",
        }
    }

    fn selected_file(&self) -> Option<&'a str> {
        match self {
            Self::File(path) => Some(*path),
            _ => None,
        }
    }

    fn selected_glob(&self) -> Option<&'a str> {
        match self {
            Self::Glob(pattern) => Some(*pattern),
            _ => None,
        }
    }

    fn matches_path(&self, rel: &str) -> bool {
        match self {
            RestoreMode::All => true,
            RestoreMode::File(file) => rel.eq_ignore_ascii_case(&file.replace('/', "\\")),
            RestoreMode::Glob(glob) => glob_match(glob, &rel.replace('\\', "/")),
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct RestorePreviewSummary {
    overwrite_count: usize,
    new_count: usize,
    visible_items: Vec<RestorePreviewItem>,
    hidden_count: usize,
}

impl RestorePreviewSummary {
    fn is_empty(&self) -> bool {
        self.overwrite_count == 0 && self.new_count == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewFastPathDecision {
    New,
    Overwrite,
    Unchanged,
    NeedContentCheck,
}

#[cfg_attr(not(test), allow(dead_code))]
fn build_restore_preview_items(
    backup_src: &Path,
    dest_root: &Path,
    mode: RestoreMode<'_>,
) -> Result<Vec<RestorePreviewItem>, CliError> {
    build_restore_preview_items_for_kind(
        backup_src,
        detect_restore_artifact_kind(backup_src),
        dest_root,
        mode,
    )
}

fn build_restore_preview_items_for_kind(
    backup_src: &Path,
    source_kind: RestoreArtifactKind,
    dest_root: &Path,
    mode: RestoreMode<'_>,
) -> Result<Vec<RestorePreviewItem>, CliError> {
    match source_kind {
        RestoreArtifactKind::Zip => {
            build_restore_preview_items_from_zip(backup_src, dest_root, mode)
        }
        RestoreArtifactKind::SevenZ | RestoreArtifactKind::Xunbak => {
            build_restore_preview_items_from_artifact(backup_src, dest_root, mode)
        }
        RestoreArtifactKind::Dir => {
            build_restore_preview_items_from_dir(backup_src, dest_root, mode)
        }
    }
}

fn build_restore_preview_items_from_dir(
    backup_src: &Path,
    dest_root: &Path,
    mode: RestoreMode<'_>,
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

            let dst = preview_destination_path(dest_root, &rel);
            maybe_push_preview_item(&mut items, classify_preview_path(&path, &dst, &rel)?);
        }
    }

    sort_restore_preview_items(&mut items);
    Ok(items)
}

fn build_restore_preview_items_from_zip(
    backup_src: &Path,
    dest_root: &Path,
    mode: RestoreMode<'_>,
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

        let dst = preview_destination_path(dest_root, &rel);
        maybe_push_preview_item(&mut items, classify_preview_reader(&mut entry, &dst, &rel)?);
    }

    sort_restore_preview_items(&mut items);
    Ok(items)
}

fn build_restore_preview_items_from_artifact(
    backup_src: &Path,
    dest_root: &Path,
    mode: RestoreMode<'_>,
) -> Result<Vec<RestorePreviewItem>, CliError> {
    let mut entries = read_artifact_entries(backup_src)?;
    if entries.first().is_some_and(|entry| {
        !matches!(
            entry.kind,
            crate::backup::artifact::entry::SourceKind::XunbakArtifact
        )
    }) {
        sort_entries_for_read_locality(&mut entries)?;
    }
    let mut items = Vec::new();
    for entry in entries {
        let rel = entry.path.replace('/', "\\");
        if !mode.matches_path(&rel) {
            continue;
        }
        let dst = preview_destination_path(dest_root, &rel);
        let fast_path = classify_preview_item_fast(
            entry.size,
            entry.mtime_ns,
            entry.win_attributes,
            &dst,
            &rel,
        )?;
        match preview_kind_from_fast_path(fast_path) {
            Some(kind) => {
                push_restore_preview_item(&mut items, rel, kind);
                continue;
            }
            None if matches!(fast_path, PreviewFastPathDecision::Unchanged) => continue,
            None => {}
        }
        let mut reader = open_entry_reader(&entry)?;
        maybe_push_preview_item(
            &mut items,
            classify_preview_reader(&mut reader, &dst, &rel)?,
        );
    }
    sort_restore_preview_items(&mut items);
    Ok(items)
}

fn preview_destination_path(dest_root: &Path, rel: &str) -> PathBuf {
    dest_root.join(rel.replace('\\', std::path::MAIN_SEPARATOR_STR))
}

fn build_restore_preview_summary(
    items: &[RestorePreviewItem],
    max_show: usize,
) -> RestorePreviewSummary {
    let overwrite_count = items
        .iter()
        .filter(|item| item.kind == RestorePreviewKind::Overwrite)
        .count();
    let new_count = items
        .iter()
        .filter(|item| item.kind == RestorePreviewKind::New)
        .count();
    let visible_items = items.iter().take(max_show).cloned().collect::<Vec<_>>();
    let hidden_count = items.len().saturating_sub(visible_items.len());

    RestorePreviewSummary {
        overwrite_count,
        new_count,
        visible_items,
        hidden_count,
    }
}

fn sort_restore_preview_items(items: &mut [RestorePreviewItem]) {
    items.sort_by(|left, right| left.rel.cmp(&right.rel));
}

fn push_restore_preview_item(
    items: &mut Vec<RestorePreviewItem>,
    rel: impl Into<String>,
    kind: RestorePreviewKind,
) {
    items.push(RestorePreviewItem {
        rel: rel.into(),
        kind,
    });
}

fn maybe_push_preview_item(items: &mut Vec<RestorePreviewItem>, item: Option<RestorePreviewItem>) {
    if let Some(item) = item {
        items.push(item);
    }
}

fn preview_kind_from_fast_path(decision: PreviewFastPathDecision) -> Option<RestorePreviewKind> {
    match decision {
        PreviewFastPathDecision::New => Some(RestorePreviewKind::New),
        PreviewFastPathDecision::Overwrite => Some(RestorePreviewKind::Overwrite),
        PreviewFastPathDecision::Unchanged | PreviewFastPathDecision::NeedContentCheck => None,
    }
}

fn classify_preview_item_from_diff<F>(
    dst: &Path,
    rel: &str,
    differs: F,
) -> Result<Option<RestorePreviewItem>, CliError>
where
    F: FnOnce() -> Result<bool, CliError>,
{
    if !dst.exists() {
        return Ok(Some(RestorePreviewItem {
            rel: rel.to_string(),
            kind: RestorePreviewKind::New,
        }));
    }

    if differs()? {
        return Ok(Some(RestorePreviewItem {
            rel: rel.to_string(),
            kind: RestorePreviewKind::Overwrite,
        }));
    }

    Ok(None)
}

fn classify_preview_path(
    src: &Path,
    dst: &Path,
    rel: &str,
) -> Result<Option<RestorePreviewItem>, CliError> {
    classify_preview_item_from_diff(dst, rel, || paths_differ(src, dst))
}

fn classify_preview_reader<R: Read>(
    entry: &mut R,
    dst: &Path,
    rel: &str,
) -> Result<Option<RestorePreviewItem>, CliError> {
    classify_preview_item_from_diff(dst, rel, || reader_differs_from_file(entry, dst))
}

fn classify_preview_item_fast(
    source_size: u64,
    source_mtime_ns: Option<u64>,
    source_win_attributes: u32,
    dst: &Path,
    _rel: &str,
) -> Result<PreviewFastPathDecision, CliError> {
    if !dst.exists() {
        return Ok(PreviewFastPathDecision::New);
    }

    let dst_meta =
        fs::metadata(dst).map_err(|e| CliError::new(1, format!("Preview read failed: {e}")))?;
    if !dst_meta.is_file() || dst_meta.len() != source_size {
        return Ok(PreviewFastPathDecision::Overwrite);
    }

    let dst_mtime_ns = dst_meta.modified().ok().map(system_time_to_unix_ns);
    let dst_attributes = file_attributes(&dst_meta);
    let attributes_match = source_win_attributes == 0 || source_win_attributes == dst_attributes;
    if source_mtime_ns.is_some() && source_mtime_ns == dst_mtime_ns && attributes_match {
        return Ok(PreviewFastPathDecision::Unchanged);
    }

    Ok(PreviewFastPathDecision::NeedContentCheck)
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
    use std::path::Path;
    use tempfile::tempdir;

    #[cfg(feature = "xunbak")]
    use crate::backup::artifact::reader::{
        cached_xunbak_reader_count_for_tests, clear_xunbak_reader_cache_for_tests,
        copy_entry_to_path,
    };
    #[cfg(feature = "xunbak")]
    use crate::backup::artifact::source::read_artifact_entries;

    use super::{
        RestoreArtifactKind, RestoreMode, RestorePreviewItem, RestorePreviewKind, RestoreStats,
        build_restore_execution_summary, build_restore_preview_items,
        build_restore_preview_summary, detect_restore_artifact_kind, glob_match,
        restore_timing_enabled_with,
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
            build_restore_preview_items(backup.path(), dest.path(), RestoreMode::File("a.txt"))
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
            build_restore_preview_items(backup.path(), dest.path(), RestoreMode::All).unwrap();
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
            build_restore_preview_items(&zip_path, dest.path(), RestoreMode::Glob("**/*.txt"))
                .unwrap();
        assert!(
            items.is_empty(),
            "unchanged matched file should not appear in preview"
        );
    }

    #[cfg(feature = "xunbak")]
    #[test]
    fn restore_preview_items_xunbak_fast_path_skips_opening_content_streams() {
        clear_xunbak_reader_cache_for_tests();
        let dir = tempdir().unwrap();
        let source = dir.path().join("a.txt");
        std::fs::write(&source, "hello xunbak").unwrap();
        let input_entry = crate::backup::artifact::entry::SourceEntry {
            path: "a.txt".to_string(),
            source_path: Some(source),
            size: 12,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: crate::backup::artifact::entry::SourceKind::DirArtifact,
        };
        let artifact = dir.path().join("artifact.xunbak");
        crate::backup::artifact::xunbak::write_entries_to_xunbak(
            &[&input_entry],
            &artifact,
            &dir.path().display().to_string(),
            &crate::xunbak::writer::BackupOptions {
                codec: crate::xunbak::constants::Codec::NONE,
                auto_compression: false,
                zstd_level: 1,
                split_size: None,
            },
            crate::backup_formats::OverwriteMode::Fail,
        )
        .unwrap();

        let artifact_entries = read_artifact_entries(&artifact).unwrap();
        let restore_dest = tempdir().unwrap();
        let dest_path = restore_dest.path().join("a.txt");
        copy_entry_to_path(&artifact_entries[0], &dest_path).unwrap();

        clear_xunbak_reader_cache_for_tests();
        let items =
            build_restore_preview_items(&artifact, restore_dest.path(), RestoreMode::All).unwrap();

        assert!(items.is_empty());
        assert_eq!(cached_xunbak_reader_count_for_tests(), 0);
    }

    #[test]
    fn restore_timing_enabled_accepts_command_and_restore_env_names() {
        let env = HashMap::from([("XUN_CMD_TIMING", OsString::from("1"))]);
        assert!(restore_timing_enabled_with(|name| env.get(name).cloned()));

        let env = HashMap::from([("XUN_RESTORE_TIMING", OsString::from("1"))]);
        assert!(restore_timing_enabled_with(|name| env.get(name).cloned()));
    }

    #[test]
    fn detect_restore_artifact_kind_distinguishes_supported_sources() {
        assert_eq!(
            detect_restore_artifact_kind(Path::new("backup.zip")),
            RestoreArtifactKind::Zip
        );
        assert_eq!(
            detect_restore_artifact_kind(Path::new("backup.7z")),
            RestoreArtifactKind::SevenZ
        );
        assert_eq!(
            detect_restore_artifact_kind(Path::new("backup.xunbak")),
            RestoreArtifactKind::Xunbak
        );
        assert_eq!(
            detect_restore_artifact_kind(Path::new("backup-dir")),
            RestoreArtifactKind::Dir
        );
    }

    #[test]
    fn restore_execution_summary_uses_mode_label_and_partial_status() {
        let summary = build_restore_execution_summary(
            Path::new("source.zip"),
            Path::new("dest"),
            RestoreMode::Glob("**/*.txt"),
            false,
            true,
            RestoreStats::new(3, 1),
        );

        assert_eq!(summary.action, "restore");
        assert_eq!(summary.status, "partial_failed");
        assert_eq!(summary.mode, "glob");
        assert!(summary.snapshot);
        assert_eq!(summary.restored, 3);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn restore_stats_status_label_matches_failure_count() {
        assert_eq!(RestoreStats::new(2, 0).status_label(), "ok");
        assert_eq!(RestoreStats::new(2, 1).status_label(), "partial_failed");
        assert_eq!(RestoreStats::single_success(), RestoreStats::new(1, 0));
    }

    #[test]
    fn restore_preview_summary_counts_and_truncates_items() {
        let items = vec![
            RestorePreviewItem {
                rel: "a.txt".to_string(),
                kind: RestorePreviewKind::Overwrite,
            },
            RestorePreviewItem {
                rel: "b.txt".to_string(),
                kind: RestorePreviewKind::New,
            },
            RestorePreviewItem {
                rel: "c.txt".to_string(),
                kind: RestorePreviewKind::Overwrite,
            },
        ];

        let summary = build_restore_preview_summary(&items, 2);
        assert_eq!(summary.overwrite_count, 2);
        assert_eq!(summary.new_count, 1);
        assert_eq!(summary.visible_items, items[..2].to_vec());
        assert_eq!(summary.hidden_count, 1);
        assert!(!summary.is_empty());
    }

    #[test]
    fn restore_preview_summary_empty_items_is_empty() {
        let summary = build_restore_preview_summary(&[], 20);
        assert_eq!(summary.overwrite_count, 0);
        assert_eq!(summary.new_count, 0);
        assert!(summary.visible_items.is_empty());
        assert_eq!(summary.hidden_count, 0);
        assert!(summary.is_empty());
    }
}
