use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(test)]
use std::io::Write;

use crate::bookmark::migration::{
    CURRENT_SCHEMA_VERSION, LegacyEntry, detect_schema_version, parse_legacy_entries,
};
use crate::config;
use crate::model::Entry;

pub(crate) type Db = BTreeMap<String, Entry>;

pub(crate) fn db_path() -> PathBuf {
    if let Some(p) = env::var("_BM_DATA_FILE").ok().filter(|v| !v.trim().is_empty()) {
        return PathBuf::from(p);
    }
    if let Some(p) = env::var("XUN_DB").ok().filter(|v| !v.trim().is_empty()) {
        return PathBuf::from(p);
    }
    let cfg = config::load_config();
    if !cfg.bookmark.data_file.trim().is_empty() {
        return PathBuf::from(cfg.bookmark.data_file);
    }
    let userprofile = env::var("USERPROFILE").ok();
    db_path_from_env(None, userprofile.as_deref())
}

fn db_path_from_env(xun_db: Option<&str>, userprofile: Option<&str>) -> PathBuf {
    if let Some(p) = xun_db {
        return PathBuf::from(p);
    }
    PathBuf::from(userprofile.unwrap_or(".")).join(".xun.bookmark.json")
}

#[cfg_attr(not(any(test, feature = "dashboard")), allow(dead_code))]
pub(crate) struct Lock(#[allow(dead_code)] fs::File);

impl Lock {
    #[cfg_attr(not(any(test, feature = "dashboard")), allow(dead_code))]
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

pub(crate) fn load_strict(path: &Path) -> io::Result<Db> {
    let content = match fs::read(path) {
        Ok(content) => content,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Db::default()),
        Err(err) => return Err(err),
    };
    let value: serde_json::Value = serde_json::from_slice(&content).map_err(|err| {
        io::Error::new(io::ErrorKind::InvalidData, format!("bookmark db parse error: {err}"))
    })?;
    let mut db: Db = match detect_schema_version(&value) {
        Some(version) if version > CURRENT_SCHEMA_VERSION => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("bookmark db unsupported schema_version: {version}"),
            ));
        }
        Some(version) if version == CURRENT_SCHEMA_VERSION => {
            let parsed: DbFile = serde_json::from_value(value).map_err(|err| {
                io::Error::new(io::ErrorKind::InvalidData, format!("bookmark db parse error: {err}"))
            })?;
            parsed
                .bookmarks
                .into_iter()
                .map(|record| {
                    let name = record.name.clone();
                    (name, record.into_entry())
                })
                .collect()
        }
        Some(version) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("bookmark db unsupported schema_version: {version}"),
            ));
        }
        None => {
            let entries = parse_legacy_entries(value).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "bookmark db missing schema_version",
                )
            })?;
            entries
                .into_iter()
                .map(|(name, entry)| (name, legacy_into_entry(entry)))
                .collect()
        }
    };
    apply_visit_log(path, &mut db);
    Ok(db)
}

#[cfg_attr(not(any(test, feature = "dashboard")), allow(dead_code))]
pub(crate) fn load(path: &Path) -> Db {
    load_strict(path).unwrap_or_default()
}

#[cfg_attr(not(any(test, feature = "dashboard")), allow(dead_code))]
pub(crate) fn save_db(path: &Path, db: &Db) -> io::Result<()> {
    let max_age = crate::config::bookmark_max_age();
    let total: u64 = db.values().map(|e| e.visit_count as u64).sum();
    let db_to_save = if max_age > 0 && total > max_age {
        age_db(db, max_age)
    } else {
        std::borrow::Cow::Borrowed(db)
    };

    let tmp = path.with_extension("tmp");
    let file = DbFile {
        schema_version: Some(1),
        bookmarks: db_to_save
            .as_ref()
            .iter()
            .map(|(name, entry)| BookmarkRecord::from_parts(name, entry))
            .collect(),
    };
    let json = serde_json::to_string_pretty(&file)?;
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)?;

    let _ = fs::remove_file(visit_log_path(path));
    Ok(())
}

#[cfg_attr(not(any(test, feature = "dashboard")), allow(dead_code))]
fn age_db(db: &Db, max_age: u64) -> std::borrow::Cow<'_, Db> {
    let factor =
        (db.values().map(|entry| entry.visit_count as u64).sum::<u64>() as f64) / (0.9 * max_age as f64);
    let aged: Db = db
        .iter()
        .filter_map(|(k, e)| {
            let new_count = ((e.visit_count as f64) / factor) as u32;
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

#[derive(serde::Serialize, serde::Deserialize)]
struct VisitLogLine {
    name: String,
    ts: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct DbFile {
    schema_version: Option<u32>,
    #[serde(default)]
    bookmarks: Vec<BookmarkRecord>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct BookmarkRecord {
    name: String,
    path: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    visit_count: Option<u32>,
    #[serde(default)]
    last_visited: Option<u64>,
}

impl BookmarkRecord {
    #[cfg_attr(not(any(test, feature = "dashboard")), allow(dead_code))]
    fn from_parts(name: &str, entry: &Entry) -> Self {
        Self {
            name: name.to_string(),
            path: entry.path.clone(),
            tags: entry.tags.clone(),
            visit_count: Some(entry.visit_count),
            last_visited: Some(entry.last_visited),
        }
    }

    fn into_entry(self) -> Entry {
        Entry {
            path: self.path,
            tags: self.tags,
            visit_count: self.visit_count.unwrap_or(0),
            last_visited: self.last_visited.unwrap_or(0),
        }
    }
}

fn legacy_into_entry(entry: LegacyEntry) -> Entry {
    Entry {
        path: entry.path,
        tags: entry.tags,
        visit_count: entry.visit_count,
        last_visited: entry.last_visited,
    }
}

fn visit_log_path(db_file_path: &Path) -> PathBuf {
    if let Some(p) = env::var("_BM_VISIT_LOG_FILE")
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        return PathBuf::from(p);
    }
    if db_file_path != db_path() {
        return db_file_path.with_extension("visits.jsonl");
    }
    let cfg = config::load_config();
    if !cfg.bookmark.visit_log_file.trim().is_empty() {
        return PathBuf::from(cfg.bookmark.visit_log_file);
    }
    db_file_path.with_extension("visits.jsonl")
}

#[cfg(test)]
const VISIT_LOG_MAX_BYTES: u64 = 64 * 1024;

#[cfg(test)]
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
        if let Ok(db) = load_strict(db_path) {
            let _ = save_db(db_path, &db);
        }
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
    use crate::store::now_secs;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
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
    fn load_strict_invalid_json_returns_error() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("bad.json");
        fs::write(&p, "{not-json").unwrap();

        let err = load_strict(&p).err().expect("expected invalid json error");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("bookmark db parse error"));
    }

    #[test]
    fn load_valid_json_parses_entries() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("db.json");

        let raw = serde_json::json!({
            "schema_version": 1,
            "bookmarks": [
                {
                    "name": "k",
                    "path": "C:\\tmp",
                    "tags": ["A", "b"],
                    "visit_count": 2,
                    "last_visited": 3
                }
            ]
        });

        fs::write(&p, serde_json::to_string(&raw).unwrap()).unwrap();

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
        let raw = fs::read_to_string(&p).unwrap();
        assert!(raw.contains("\"schema_version\": 1"));
        assert!(raw.contains("\"bookmarks\""));

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
    fn load_strict_missing_schema_version_returns_error() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("db.json");
        fs::write(&p, r#"{"bookmarks":[]}"#).unwrap();

        let err = load_strict(&p).err().expect("expected invalid data");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("missing schema_version"));
    }

    #[test]
    fn load_strict_legacy_map_migrates() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("legacy.json");
        fs::write(
            &p,
            r#"{
  "home": {
    "path": "C:/work/home",
    "tags": ["work"],
    "visit_count": 2,
    "last_visited": 3
  }
}"#,
        )
        .unwrap();

        let loaded = load_strict(&p).unwrap();
        let entry = loaded.get("home").unwrap();
        assert_eq!(entry.path, "C:/work/home");
        assert_eq!(entry.visit_count, 2);
        assert_eq!(entry.last_visited, 3);
    }

    #[test]
    fn load_strict_future_schema_version_returns_error() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("db.json");
        fs::write(&p, r#"{"schema_version":2,"bookmarks":[]}"#).unwrap();

        let err = load_strict(&p).err().expect("expected invalid data");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("unsupported schema_version"));
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

        let l1 = Lock::acquire(&lock_path).expect("acquire lock");
        drop(l1);

        let l2 = Lock::acquire(&lock_path).expect("re-acquire lock");
        drop(l2);
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
            dir.path().join(".xun.bookmark.json")
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
