use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::cli::RestoreCmd;
use crate::output::{CliError, CliResult, can_interact};
use crate::path_guard::{PathPolicy, validate_paths};

use super::bak::config as bak_config;
use super::bak::restore as bak_restore;

use bak_config::BakConfig;

/// zip entry 路径安全校验（复用 path_guard）
fn is_safe_zip_entry(name: &str) -> bool {
    let mut policy = PathPolicy::for_output();
    policy.allow_relative = true;
    validate_paths([name], &policy).issues.is_empty()
}

pub(crate) fn cmd_restore(args: RestoreCmd) -> CliResult {
    let root = match &args.dir {
        Some(d) => PathBuf::from(d),
        None => std::env::current_dir()
            .map_err(|e| CliError::new(1, format!("Failed to get current directory: {e}")))?,
    };
    let cfg = bak_config::load_config(&root);
    let backups_root = root.join(&cfg.storage.backups_dir);

    // 解析备份源路径
    let src = resolve_backup_src(&root, &backups_root, &args.name_or_path)?;

    // --snapshot：还原前先备份当前状态
    if args.snapshot && !args.dry_run {
        eprintln!("Creating pre-restore snapshot...");
        run_snapshot_bak(&root, &cfg)?;
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
        bak_restore::show_restore_preview(&dest_root, &cfg, &src);
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
        // --glob 模式：收集匹配文件，并行还原
        restore_with_glob(&src, &dest_root, glob_pat, args.dry_run)?
    } else if let Some(ref file) = args.file {
        // --file 单文件模式
        restore_single_file(&src, &dest_root, file, args.dry_run)?
    } else {
        // 全量还原
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
        return Err(CliError::new(1, format!("{failed} file(s) failed to restore.")));
    }
    Ok(())
}

/// 解析备份源路径：直接路径 > 备份目录查找
fn resolve_backup_src(
    _root: &Path,
    backups_root: &Path,
    name_or_path: &str,
) -> Result<PathBuf, CliError> {
    let p = PathBuf::from(name_or_path);
    if p.is_dir() || p.is_file() {
        return Ok(p);
    }
    bak_restore::backup_source_path(backups_root, name_or_path).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Backup not found: {name_or_path}"),
            &[
                "Fix: Run `xun bak list` to see available backups.",
                "Fix: Pass a direct path to a backup dir or .zip file.",
            ],
        )
    })
}

/// 全量还原，返回 (restored, failed)
fn restore_all(src: &Path, dest_root: &Path, dry_run: bool) -> Result<(usize, usize), CliError> {
    if src.extension().and_then(|e| e.to_str()) == Some("zip") {
        restore_all_from_zip(src, dest_root, dry_run)
    } else {
        restore_all_from_dir(src, dest_root, dry_run)
    }
}

fn restore_all_from_dir(
    src_dir: &Path,
    dest_root: &Path,
    dry_run: bool,
) -> Result<(usize, usize), CliError> {
    let entries = collect_files_recursive(src_dir);

    let restored = AtomicUsize::new(0);
    let fail_count = AtomicUsize::new(0);

    entries.par_iter().for_each(|src_path| {
        let rel = match src_path.strip_prefix(src_dir) {
            Ok(r) => r,
            Err(_) => return,
        };
        let dst = dest_root.join(rel);
        if dry_run {
            eprintln!("DRY RUN: would restore {}", rel.display());
            restored.fetch_add(1, Ordering::Relaxed);
            return;
        }
        if let Some(p) = dst.parent() {
            let _ = fs::create_dir_all(p);
        }
        match fs::copy(src_path, &dst) {
            Ok(_) => { restored.fetch_add(1, Ordering::Relaxed); }
            Err(e) => {
                eprintln!("Error restoring {}: {e}", rel.display());
                fail_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    });

    Ok((restored.load(Ordering::Relaxed), fail_count.load(Ordering::Relaxed)))
}

fn restore_all_from_zip(
    zip_path: &Path,
    dest_root: &Path,
    dry_run: bool,
) -> Result<(usize, usize), CliError> {
    let f = fs::File::open(zip_path)
        .map_err(|e| CliError::new(1, format!("Open zip failed: {e}")))?;
    let mut archive = zip::ZipArchive::new(f)
        .map_err(|e| CliError::new(1, format!("Read zip failed: {e}")))?;

    let mut restored = 0usize;
    let mut failed = 0usize;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| CliError::new(1, format!("Zip entry error: {e}")))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_owned();
        let name_norm = name.replace('\\', "/");
        // path_guard 校验：拒绝路径穿越等
        if !is_safe_zip_entry(&name_norm) {
            eprintln!("Skipping unsafe zip entry: {name}");
            failed += 1;
            continue;
        }
        let rel = PathBuf::from(name.replace('/', "\\"));
        let dst = dest_root.join(&rel);
        if dry_run {
            eprintln!("DRY RUN: would restore {}", rel.display());
            restored += 1;
            continue;
        }
        if let Some(parent) = dst.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match fs::File::create(&dst) {
            Ok(mut out) => {
                if let Err(e) = std::io::copy(&mut entry, &mut out) {
                    eprintln!("Error writing {}: {e}", rel.display());
                    failed += 1;
                } else {
                    restored += 1;
                }
            }
            Err(e) => {
                eprintln!("Error creating {}: {e}", rel.display());
                failed += 1;
            }
        }
    }
    Ok((restored, failed))
}

/// 单文件还原
fn restore_single_file(
    src: &Path,
    dest_root: &Path,
    file: &str,
    dry_run: bool,
) -> Result<(usize, usize), CliError> {
    let rel = PathBuf::from(file);
    // 拒绝绝对路径和路径穿越（含 .. 组件）
    if rel.is_absolute() || rel.components().any(|c| c == std::path::Component::ParentDir) {
        return Err(CliError::with_details(
            2,
            format!("Unsafe restore path: {file}"),
            &["Fix: Use a relative path without '..' components."],
        ));
    }
    // 用 path_guard 校验非法字符等
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
        bak_restore::restore_from_zip(src, dest_root, Some(&rel), dry_run)?;
    } else {
        bak_restore::restore_from_dir(src, dest_root, Some(&rel), dry_run)?;
    }
    Ok((1, 0))
}

/// glob 模式还原
fn restore_with_glob(
    src: &Path,
    dest_root: &Path,
    glob_pat: &str,
    dry_run: bool,
) -> Result<(usize, usize), CliError> {
    if src.extension().and_then(|e| e.to_str()) == Some("zip") {
        restore_glob_from_zip(src, dest_root, glob_pat, dry_run)
    } else {
        restore_glob_from_dir(src, dest_root, glob_pat, dry_run)
    }
}

fn restore_glob_from_dir(
    src_dir: &Path,
    dest_root: &Path,
    glob_pat: &str,
    dry_run: bool,
) -> Result<(usize, usize), CliError> {
    let entries = collect_files_recursive(src_dir);

    let restored = AtomicUsize::new(0);
    let fail_count = AtomicUsize::new(0);

    entries.par_iter().for_each(|src_path| {
        let rel = match src_path.strip_prefix(src_dir) {
            Ok(r) => r,
            Err(_) => return,
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if !glob_match(glob_pat, &rel_str) {
            return;
        }
        let dst = dest_root.join(rel);
        if dry_run {
            eprintln!("DRY RUN: would restore {}", rel.display());
            restored.fetch_add(1, Ordering::Relaxed);
            return;
        }
        if let Some(p) = dst.parent() {
            let _ = fs::create_dir_all(p);
        }
        match fs::copy(src_path, &dst) {
            Ok(_) => { restored.fetch_add(1, Ordering::Relaxed); }
            Err(e) => {
                eprintln!("Error restoring {}: {e}", rel.display());
                fail_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    });

    Ok((restored.load(Ordering::Relaxed), fail_count.load(Ordering::Relaxed)))
}

fn restore_glob_from_zip(
    zip_path: &Path,
    dest_root: &Path,
    glob_pat: &str,
    dry_run: bool,
) -> Result<(usize, usize), CliError> {
    let f = fs::File::open(zip_path)
        .map_err(|e| CliError::new(1, format!("Open zip failed: {e}")))?;
    let mut archive = zip::ZipArchive::new(f)
        .map_err(|e| CliError::new(1, format!("Read zip failed: {e}")))?;

    let mut restored = 0usize;
    let mut failed = 0usize;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| CliError::new(1, format!("Zip entry error: {e}")))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_owned();
        let name_norm = name.replace('\\', "/");
        if !glob_match(glob_pat, &name_norm) {
            continue;
        }
        let rel = PathBuf::from(name.replace('/', "\\"));
        let dst = dest_root.join(&rel);
        if dry_run {
            eprintln!("DRY RUN: would restore {}", rel.display());
            restored += 1;
            continue;
        }
        if let Some(parent) = dst.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match fs::File::create(&dst) {
            Ok(mut out) => {
                if let Err(e) = std::io::copy(&mut entry, &mut out) {
                    eprintln!("Error writing {}: {e}", rel.display());
                    failed += 1;
                } else {
                    restored += 1;
                }
            }
            Err(e) => {
                eprintln!("Error creating {}: {e}", rel.display());
                failed += 1;
            }
        }
    }
    Ok((restored, failed))
}

/// 递归收集目录下所有文件（不依赖 walkdir）
fn collect_files_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    collect_recursive_inner(dir, &mut result);
    result
}

fn collect_recursive_inner(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_recursive_inner(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// snapshot：调用 cmd_bak 备份当前状态（desc = pre_restore）
fn run_snapshot_bak(root: &Path, _cfg: &BakConfig) -> CliResult {
    use crate::cli::BakCmd;
    let args = BakCmd {
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
    super::bak::cmd_bak(args)
}

/// 简易 glob 匹配（无外部依赖）
/// - `*`  匹配单段内任意字符（不跨 `/`）
/// - `**` 跨目录匹配（可匹配零或多个路径段）
/// - `?`  匹配单个任意字符（不含 `/`）
fn glob_match(pattern: &str, path: &str) -> bool {
    glob_match_parts(pattern.as_bytes(), path.as_bytes())
}

fn glob_match_parts(pat: &[u8], s: &[u8]) -> bool {
    // 消耗 pat 开头的 ** 段（** 或 **/ ）
    if pat.starts_with(b"**") {
        let rest_pat = if pat.len() > 2 && pat[2] == b'/' {
            &pat[3..] // 跳过 **/
        } else {
            &pat[2..] // 跳过 **
        };
        // ** 匹配零个路径段
        if glob_match_parts(rest_pat, s) {
            return true;
        }
        // ** 匹配一个或多个路径段：跳过 s 中下一个 / 后继续尝试
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
        (None, _) | (_, None) if pat == b"*" => true, // 末尾 * 匹配空
        (None, _) | (Some(_), None) => false,
        (Some(b'*'), _) => {
            // 单段 *：不跨 /
            if s[0] == b'/' {
                return false;
            }
            // * 匹配零字符
            if glob_match_parts(&pat[1..], s) {
                return true;
            }
            // * 匹配一个字符（非 /）
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
    use super::glob_match;

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
}
