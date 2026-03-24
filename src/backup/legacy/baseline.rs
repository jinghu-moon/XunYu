use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

pub(crate) struct FileMeta {
    pub(crate) size: u64,
    pub(crate) modified: SystemTime,
}

pub(crate) fn read_baseline(prev: &Path) -> HashMap<String, FileMeta> {
    let mut old = HashMap::new();
    if prev.extension().is_some_and(|e| e == "zip") && prev.is_file() {
        read_baseline_zip(prev, &mut old);
    } else if prev.is_dir() {
        read_baseline_dir(prev, prev, &mut old);
    }
    old
}

fn is_backup_internal_name(name: &str) -> bool {
    matches!(name, ".bak-meta.json" | ".bak-manifest.json")
}

fn read_baseline_zip(zip_path: &Path, old: &mut HashMap<String, FileMeta>) {
    let Ok(file) = fs::File::open(zip_path) else {
        return;
    };
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return;
    };
    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index(i) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('/', "\\");
        if name.is_empty() {
            continue;
        }
        if is_backup_internal_name(name.rsplit('\\').next().unwrap_or(&name)) {
            continue;
        }
        let modified = entry
            .last_modified()
            .map(zip_datetime_to_systime)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        old.insert(
            name,
            FileMeta {
                size: entry.size(),
                modified,
            },
        );
    }
}

fn read_baseline_dir(dir: &Path, base: &Path, old: &mut HashMap<String, FileMeta>) {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(rd) = fs::read_dir(&current) else {
            continue;
        };
        for entry in rd.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let rel = path.strip_prefix(base).unwrap_or(&path);
            if is_backup_internal_name(rel.file_name().and_then(|s| s.to_str()).unwrap_or_default())
            {
                continue;
            }
            old.insert(
                rel_key(rel),
                FileMeta {
                    size: meta.len(),
                    modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                },
            );
        }
    }
}

fn rel_key(rel: &Path) -> String {
    let value = rel.to_string_lossy();
    if value.contains('/') {
        value.replace('/', "\\")
    } else {
        value.into_owned()
    }
}

fn zip_datetime_to_systime(dt: zip::DateTime) -> SystemTime {
    // Convert zip DateTime fields to unix timestamp (days-from-civil, O(1))
    fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
        // https://howardhinnant.github.io/date_algorithms.html#days_from_civil
        let y = y - if m <= 2 { 1 } else { 0 };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let m = m + if m > 2 { -3 } else { 9 };
        let doy = (153 * m + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468
    }

    let y = dt.year() as i64;
    let m = dt.month() as i64;
    let d = dt.day() as i64;
    let hh = dt.hour() as i64;
    let mm = dt.minute() as i64;
    let ss = dt.second() as i64;

    let days = days_from_civil(y, m, d);
    let secs = days.saturating_mul(86_400) + hh * 3_600 + mm * 60 + ss;
    if secs <= 0 {
        SystemTime::UNIX_EPOCH
    } else {
        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs as u64)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::read_baseline;

    #[test]
    fn read_baseline_dir_skips_internal_backup_files() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "ok").unwrap();
        std::fs::write(dir.path().join(".bak-meta.json"), "{}").unwrap();
        std::fs::write(dir.path().join(".bak-manifest.json"), "{}").unwrap();

        let baseline = read_baseline(dir.path());
        assert!(baseline.contains_key("a.txt"));
        assert!(!baseline.contains_key(".bak-meta.json"));
        assert!(!baseline.contains_key(".bak-manifest.json"));
        assert_eq!(baseline.len(), 1);
    }
}
