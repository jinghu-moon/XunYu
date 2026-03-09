use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::io::Write;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::model::Entry;

pub(crate) type Db = BTreeMap<String, Entry>;

pub(crate) fn db_path() -> PathBuf {
    let xun_db = env::var("XUN_DB").ok();
    let userprofile = env::var("USERPROFILE").ok();
    db_path_from_env(xun_db.as_deref(), userprofile.as_deref())
}

fn db_path_from_env(xun_db: Option<&str>, userprofile: Option<&str>) -> PathBuf {
    if let Some(p) = xun_db {
        return PathBuf::from(p);
    }
    PathBuf::from(userprofile.unwrap_or(".")).join(".xun.json")
}

pub(crate) struct Lock(#[allow(dead_code)] fs::File);

impl Lock {
    pub(crate) fn acquire(path: &Path) -> io::Result<Self> {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .share_mode(0)
                .open(path)
            {
                Ok(f) => return Ok(Lock(f)),
                Err(_) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => return Err(e),
            }
        }
    }
}

pub(crate) fn load(path: &Path) -> Db {
    let mut db: Db = fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    apply_visit_log(path, &mut db);
    db
}

/// Frecency aging threshold (zoxide-style). When total visit_count exceeds this,
/// all counts are decayed by 10% and entries with count < 1 are removed.
const FRECENCY_MAX_AGE: u64 = 10_000;

pub(crate) fn save_db(path: &Path, db: &Db) -> io::Result<()> {
    let total: u64 = db.values().map(|e| e.visit_count as u64).sum();
    let db_to_save = if total > FRECENCY_MAX_AGE {
        age_db(db)
    } else {
        std::borrow::Cow::Borrowed(db)
    };

    let tmp = path.with_extension("tmp");
    let json = serde_json::to_string_pretty(db_to_save.as_ref())?;
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)?;

    let _ = fs::remove_file(visit_log_path(path));
    Ok(())
}

fn age_db(db: &Db) -> std::borrow::Cow<'_, Db> {
    let aged: Db = db
        .iter()
        .filter_map(|(k, e)| {
            let new_count = ((e.visit_count as f64) * 0.9) as u32;
            if new_count < 1 {
                return None;
            }
            let mut e2 = e.clone();
            e2.visit_count = new_count;
            Some((k.clone(), e2))
        })
        .collect();
    std::borrow::Cow::Owned(aged)
}

pub(crate) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct VisitLogLine {
    name: String,
    ts: u64,
}

fn visit_log_path(db_path: &Path) -> PathBuf {
    db_path.with_extension("visits.jsonl")
}

const VISIT_LOG_MAX_BYTES: u64 = 64 * 1024;

pub(crate) fn append_visit(db_path: &Path, name: &str, ts: u64) -> io::Result<()> {
    let line = VisitLogLine {
        name: name.to_string(),
        ts,
    };
    let log_path = visit_log_path(db_path);
    {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        writeln!(
            &mut f,
            "{}",
            serde_json::to_string(&line).unwrap_or_default()
        )?;
    }
    if log_path
        .metadata()
        .map(|m| m.len() > VISIT_LOG_MAX_BYTES)
        .unwrap_or(false)
    {
        let db = load(db_path);
        let _ = save_db(db_path, &db);
    }
    Ok(())
}

fn apply_visit_log(db_path: &Path, db: &mut Db) {
    let log_path = visit_log_path(db_path);
    let Ok(content) = fs::read_to_string(&log_path) else {
        return;
    };
    for raw in content.lines() {
        let Ok(v) = serde_json::from_str::<VisitLogLine>(raw) else {
            continue;
        };
        let Some(e) = db.get_mut(&v.name) else {
            continue;
        };
        e.visit_count = e.visit_count.saturating_add(1);
        if v.ts > e.last_visited {
            e.last_visited = v.ts;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn load_missing_returns_empty_map() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("missing.json");
        let db = load(&p);
        assert!(db.is_empty());
    }

    #[test]
    fn load_invalid_json_returns_empty_map() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("bad.json");
        fs::write(&p, "{not-json").unwrap();
        let db = load(&p);
        assert!(db.is_empty());
    }

    #[test]
    fn load_valid_json_parses_entries() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("db.json");

        let mut db: Db = BTreeMap::new();
        db.insert(
            "k".to_string(),
            Entry {
                path: "C:\\tmp".to_string(),
                tags: vec!["A".to_string(), "b".to_string()],
                visit_count: 2,
                last_visited: 3,
            },
        );

        fs::write(&p, serde_json::to_string(&db).unwrap()).unwrap();

        let loaded = load(&p);
        assert_eq!(loaded.len(), 1);
        let e = loaded.get("k").expect("missing key");
        assert_eq!(e.path, "C:\\tmp");
        assert_eq!(e.tags, vec!["A", "b"]);
        assert_eq!(e.visit_count, 2);
        assert_eq!(e.last_visited, 3);
    }

    #[test]
    fn save_db_roundtrip_and_tmp_is_removed() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("db.json");

        let mut db: Db = BTreeMap::new();
        db.insert(
            "k".to_string(),
            Entry {
                path: "C:\\tmp".to_string(),
                tags: vec![],
                visit_count: 1,
                last_visited: 2,
            },
        );

        save_db(&p, &db).unwrap();
        assert!(
            !p.with_extension("tmp").exists(),
            "tmp file should be renamed away"
        );

        let loaded = load(&p);
        assert_eq!(loaded.len(), 1);
        let e = loaded.get("k").unwrap();
        assert_eq!(e.path, "C:\\tmp");
        assert_eq!(e.visit_count, 1);
        assert_eq!(e.last_visited, 2);
    }

    #[test]
    fn load_applies_visit_log_lines() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("db.json");

        let mut db: Db = BTreeMap::new();
        db.insert(
            "k".to_string(),
            Entry {
                path: "C:\\tmp".to_string(),
                tags: vec![],
                visit_count: 0,
                last_visited: 0,
            },
        );
        save_db(&p, &db).unwrap();

        append_visit(&p, "k", 100).unwrap();
        append_visit(&p, "k", 90).unwrap();

        let loaded = load(&p);
        let e = loaded.get("k").unwrap();
        assert_eq!(e.visit_count, 2);
        assert_eq!(e.last_visited, 100);
    }

    #[test]
    fn save_db_clears_visit_log_after_successful_save() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("db.json");

        let mut db: Db = BTreeMap::new();
        db.insert(
            "k".to_string(),
            Entry {
                path: "C:\\tmp".to_string(),
                tags: vec![],
                visit_count: 1,
                last_visited: 2,
            },
        );

        append_visit(&p, "k", 100).unwrap();
        assert!(visit_log_path(&p).exists());

        save_db(&p, &db).unwrap();
        assert!(!visit_log_path(&p).exists(), "expected visit log cleared");
    }

    #[test]
    fn lock_acquire_allows_reacquire_after_drop() {
        let dir = tempdir().unwrap();
        let lock_path = dir.path().join("db.lock");

        let _l1 = Lock::acquire(&lock_path).expect("acquire lock");
        drop(_l1);

        let _l2 = Lock::acquire(&lock_path).expect("re-acquire lock");
        drop(_l2);
    }

    #[test]
    fn lock_acquire_times_out_when_held_by_another_handle() {
        let dir = tempdir().unwrap();
        let lock_path = dir.path().join("db.lock");

        let _hold = Lock::acquire(&lock_path).expect("acquire lock");

        let start = Instant::now();
        assert!(Lock::acquire(&lock_path).is_err(), "expected timeout error");
        assert!(
            start.elapsed() >= Duration::from_secs(2),
            "expected retry loop; got error too fast"
        );
    }

    #[test]
    fn db_path_prefers_xun_db_env() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("custom.json");
        assert_eq!(
            db_path_from_env(Some(p.to_string_lossy().as_ref()), Some("C:\\Users\\x")),
            p
        );
    }

    #[test]
    fn db_path_falls_back_to_userprofile() {
        let dir = tempdir().unwrap();
        assert_eq!(
            db_path_from_env(None, Some(dir.path().to_string_lossy().as_ref())),
            dir.path().join(".xun.json")
        );
    }

    #[test]
    fn now_secs_is_reasonable() {
        let n = now_secs();
        assert!(n > 1_000_000_000);
        let sys = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let diff = sys.saturating_sub(n).max(n.saturating_sub(sys));
        assert!(diff <= 5, "unexpected now_secs drift: {diff}s");
    }
}
