use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::config::RetentionCfg;
use super::version::parse_version;

struct BackupEntry {
    version: u32,
    path: PathBuf,
    mtime_secs: u64,
}

fn collect_entries(backups_root: &Path, prefix: &str) -> Vec<BackupEntry> {
    let mut items: Vec<BackupEntry> = Vec::new();
    if let Ok(rd) = fs::read_dir(backups_root) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".meta.json") {
                continue;
            }
            if let Some(n) = parse_version(&name, prefix) {
                let mtime = e
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                items.push(BackupEntry {
                    version: n,
                    path: e.path(),
                    mtime_secs: mtime,
                });
            }
        }
    }
    items.sort_by_key(|i| i.version);
    items
}

fn remove_entry(p: &Path) {
    if p.is_dir() {
        let _ = fs::remove_dir_all(p);
    } else {
        let _ = fs::remove_file(p);
        let meta_path = p.with_extension("meta.json");
        let _ = fs::remove_file(&meta_path);
    }
}

/// 应用保留策略，返回实际删除数量
#[allow(dead_code)]
pub(crate) fn apply_retention(
    backups_root: &Path,
    prefix: &str,
    max: usize,
    batch: usize,
) -> usize {
    let cfg = RetentionCfg {
        max_backups: max,
        delete_count: batch,
        ..RetentionCfg::default()
    };
    apply_retention_policy(backups_root, prefix, &cfg)
}

/// 完整保留策略（含时间窗口，语义对标 restic --keep-daily/weekly/monthly）
///
/// - keep_daily N：每个自然日最多保留 1 个备份，最多保留 N 个不同日的代表备份（无时间范围限制）
/// - keep_weekly N：每个自然周最多保留 1 个备份，最多保留 N 个不同周的代表
/// - keep_monthly N：每个自然月最多保留 1 个备份，最多保留 N 个不同月的代表
/// - max_backups：超出此数量后，从最旧开始删除（时间窗口标记为保留的优先免删）
pub(crate) fn apply_retention_policy(
    backups_root: &Path,
    prefix: &str,
    cfg: &RetentionCfg,
) -> usize {
    if cfg.max_backups == 0 {
        return 0;
    }

    let items = collect_entries(backups_root, prefix);
    let total = items.len();
    if total <= cfg.max_backups {
        return 0;
    }

    // 标记需要保留的条目
    let mut keep = vec![false; total];

    // 时间窗口保留：从最新到最旧遍历，每个 bucket 只取最新的 1 个
    // bucket 无时间范围限制，语义为：每天/每周/每月的代表，最多 N 个不同的桶
    if cfg.keep_daily > 0 || cfg.keep_weekly > 0 || cfg.keep_monthly > 0 {
        const DAY: u64 = 86_400;
        const WEEK: u64 = 7 * DAY;
        const MONTH: u64 = 30 * DAY; // 近似

        let mut seen_days: std::collections::HashMap<u64, usize> =
            std::collections::HashMap::new();
        let mut seen_weeks: std::collections::HashMap<u64, usize> =
            std::collections::HashMap::new();
        let mut seen_months: std::collections::HashMap<u64, usize> =
            std::collections::HashMap::new();

        // 从最新到最旧：每个 bucket 只取第一次（最新）出现的
        for (i, entry) in items.iter().enumerate().rev() {
            let ts = entry.mtime_secs;

            if cfg.keep_daily > 0 {
                let bucket = ts / DAY;
                let count = seen_days.len();
                seen_days.entry(bucket).or_insert_with(|| {
                    if count < cfg.keep_daily {
                        keep[i] = true;
                    }
                    i
                });
            }
            if cfg.keep_weekly > 0 {
                let bucket = ts / WEEK;
                let count = seen_weeks.len();
                seen_weeks.entry(bucket).or_insert_with(|| {
                    if count < cfg.keep_weekly {
                        keep[i] = true;
                    }
                    i
                });
            }
            if cfg.keep_monthly > 0 {
                let bucket = ts / MONTH;
                let count = seen_months.len();
                seen_months.entry(bucket).or_insert_with(|| {
                    if count < cfg.keep_monthly {
                        keep[i] = true;
                    }
                    i
                });
            }
        }
    }

    // 超出 max_backups 的旧备份（最旧优先删除，保留时间窗口标记的）
    let overflow = total.saturating_sub(cfg.max_backups);
    let to_delete = overflow.max(cfg.delete_count).min(total.saturating_sub(1));
    let mut cleaned = 0;
    for (i, entry) in items.iter().enumerate().take(to_delete) {
        if keep[i] {
            continue;
        }
        remove_entry(&entry.path);
        cleaned += 1;
    }
    cleaned
}
