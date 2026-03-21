use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::cli::RestoreCmd;
use crate::output::{CliError, CliResult, can_interact};
use crate::path_guard::{PathPolicy, validate_paths};

use super::backup::config as backup_config;
use super::backup::{FileMeta, read_baseline};
use super::restore_core;

use backup_config::BackupConfig;

pub(crate) fn cmd_restore(args: RestoreCmd) -> CliResult {
    let root = match &args.dir {
        Some(d) => PathBuf::from(d),
        None => std::env::current_dir()
            .map_err(|e| CliError::new(1, format!("Failed to get current directory: {e}")))?,
    };
    let cfg = backup_config::load_config(&root);
    let backups_root = root.join(&cfg.storage.backups_dir);

    // 解析备份源路径
    let src = resolve_backup_src(&backups_root, &args.name_or_path)?;

    // --snapshot：还原前先备份当前状态
    if args.snapshot && !args.dry_run {
        eprintln!("Creating pre-restore snapshot...");
        run_snapshot_backup(&root, &cfg)?;
    }

    // 确定目标根目录
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

    // 交互确认
    if !args.yes && can_interact() {
        show_restore_preview(&dest_root, &cfg, &src);
        let ok = dialoguer::Confirm::new()
            .with_prompt("Restore may overwrite files. Continue?")
            .default(false)
            .interact()
            .unwrap_or(false);
        if !ok {
            return Err(CliError::new(3, "Cancelled."));
        }
    }

    let t_start = std::time::Instant::now();
    let (restored, failed) = if let Some(ref glob_pat) = args.glob {
        restore_with_glob(&src, &dest_root, glob_pat, args.dry_run)?
    } else if let Some(ref file) = args.file {
        restore_single_file(&src, &dest_root, file, args.dry_run)?
    } else {
        restore_all(&src, &dest_root, args.dry_run)?
    };

    let elapsed = t_start.elapsed();
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
    if p.is_dir() || p.is_file() {
        return Ok(p);
    }
    backup_source_path(backups_root, name_or_path).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Backup not found: {name_or_path}"),
            &[
                "Fix: Run `xun backup list` to see available backups.",
                "Fix: Pass a direct path to a backup dir or .zip file.",
            ],
        )
    })
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
    None
}

/// 全量还原，返回 (restored, failed)
fn restore_all(src: &Path, dest_root: &Path, dry_run: bool) -> Result<(usize, usize), CliError> {
    if src.extension().and_then(|e| e.to_str()) == Some("zip") {
        restore_core::restore_many_from_zip(src, dest_root, dry_run, |_| true)
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
    } else {
        restore_core::restore_from_dir(src, dest_root, Some(&rel), dry_run)?;
    }
    Ok((1, 0))
}

/// restore 前展示将被覆盖的文件列表（modify/new 文件数）
fn show_restore_preview(root: &Path, cfg: &BackupConfig, backup_src: &Path) {
    let backup_snapshot = read_baseline(backup_src);
    let current_snapshot = read_baseline(root);
    let entries = build_restore_preview_items(&backup_snapshot, &current_snapshot);

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
    let _ = cfg;
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
    backup_snapshot: &HashMap<String, FileMeta>,
    current_snapshot: &HashMap<String, FileMeta>,
) -> Vec<RestorePreviewItem> {
    let mut items = Vec::new();

    for (rel, backup_meta) in backup_snapshot {
        if restore_core::is_backup_internal_name(rel) {
            continue;
        }
        match current_snapshot.get(rel) {
            Some(current_meta)
                if backup_meta.size != current_meta.size
                    || backup_meta.modified > current_meta.modified =>
            {
                items.push(RestorePreviewItem {
                    rel: rel.clone(),
                    kind: RestorePreviewKind::Overwrite,
                });
            }
            Some(_) => {}
            None => items.push(RestorePreviewItem {
                rel: rel.clone(),
                kind: RestorePreviewKind::New,
            }),
        }
    }

    items.sort_by(|a, b| a.rel.cmp(&b.rel));
    items
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
    } else {
        Ok(restore_core::restore_many_from_dir(
            src,
            dest_root,
            dry_run,
            |_, rel_str| glob_match(glob_pat, rel_str),
        ))
    }
}

/// snapshot：调用 cmd_backup 备份当前状态（desc = pre_restore）
fn run_snapshot_backup(root: &Path, _cfg: &BackupConfig) -> CliResult {
    use crate::cli::BackupCmd;
    let args = BackupCmd {
        op_args: vec![],
        msg: Some("pre_restore".to_string()),
        dir: Some(root.to_string_lossy().into_owned()),
        dry_run: false,
        no_compress: false,
        retain: None,
        include: vec![],
        exclude: vec![],
        incremental: false,
    };
    super::backup::cmd_backup(args)
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
    use std::time::{Duration, SystemTime};

    use super::{RestorePreviewItem, RestorePreviewKind, build_restore_preview_items, glob_match};
    use crate::commands::backup::FileMeta;

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
    fn restore_preview_items_skip_internal_files_and_detect_changes() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(200);
        let older = SystemTime::UNIX_EPOCH + Duration::from_secs(100);

        let mut backup_snapshot = HashMap::new();
        backup_snapshot.insert(
            "a.txt".to_string(),
            FileMeta {
                size: 10,
                modified: now,
            },
        );
        backup_snapshot.insert(
            "b.txt".to_string(),
            FileMeta {
                size: 20,
                modified: now,
            },
        );
        backup_snapshot.insert(
            ".bak-meta.json".to_string(),
            FileMeta {
                size: 1,
                modified: now,
            },
        );

        let mut current_snapshot = HashMap::new();
        current_snapshot.insert(
            "a.txt".to_string(),
            FileMeta {
                size: 9,
                modified: older,
            },
        );

        let items = build_restore_preview_items(&backup_snapshot, &current_snapshot);
        assert_eq!(
            items,
            vec![
                RestorePreviewItem {
                    rel: "a.txt".to_string(),
                    kind: RestorePreviewKind::Overwrite,
                },
                RestorePreviewItem {
                    rel: "b.txt".to_string(),
                    kind: RestorePreviewKind::New,
                },
            ]
        );
    }
}
