use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum AuditParams {
    Text(String),
    Map(BTreeMap<String, serde_json::Value>),
}

impl AuditParams {
    fn to_legacy_text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Map(m) => map_to_legacy_text(m),
        }
    }
}

impl From<&str> for AuditParams {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<String> for AuditParams {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

fn map_to_legacy_text(m: &BTreeMap<String, serde_json::Value>) -> String {
    // Keep a stable legacy format for older readers/tests.
    // Only include keys that are known to be parsed from the legacy string.
    let mut out = String::new();
    if let Some(tx) = m.get("tx").and_then(|v| v.as_str()) {
        out.push_str("tx=");
        out.push_str(tx);
    }
    if let Some(dst) = m.get("dst").and_then(|v| v.as_str()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str("dst=");
        out.push_str(dst);
    }
    if let Some(copy) = m.get("copy").and_then(|v| v.as_bool()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str("copy=");
        out.push_str(if copy { "true" } else { "false" });
    }
    out
}

pub(crate) fn audit_log(
    action: &str,
    target: &str,
    user: &str,
    params: impl Into<AuditParams>,
    result: &str,
    reason: &str,
) {
    let log_file = get_audit_file_path();
    audit_log_to_file(&log_file, action, target, user, params, result, reason);
}

#[cfg(feature = "redirect")]
pub(crate) fn audit_file_path() -> PathBuf {
    get_audit_file_path()
}

fn audit_log_to_file(
    log_file: &Path,
    action: &str,
    target: &str,
    user: &str,
    params: impl Into<AuditParams>,
    result: &str,
    reason: &str,
) {
    audit_log_to_file_with_serializer(
        log_file,
        action,
        target,
        user,
        params,
        result,
        reason,
        serde_json::to_string,
    );
}

fn audit_log_to_file_with_serializer(
    log_file: &Path,
    action: &str,
    target: &str,
    user: &str,
    params: impl Into<AuditParams>,
    result: &str,
    reason: &str,
    serializer: fn(&serde_json::Value) -> Result<String, serde_json::Error>,
) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let params = params.into();
    let mut obj = serde_json::Map::new();
    obj.insert(
        "timestamp".to_string(),
        serde_json::Value::Number(now.into()),
    );
    obj.insert(
        "action".to_string(),
        serde_json::Value::String(action.to_string()),
    );
    obj.insert(
        "target".to_string(),
        serde_json::Value::String(target.to_string()),
    );
    obj.insert(
        "user".to_string(),
        serde_json::Value::String(user.to_string()),
    );
    obj.insert(
        "params".to_string(),
        serde_json::Value::String(params.to_legacy_text()),
    );
    if let AuditParams::Map(m) = params {
        obj.insert(
            "params_json".to_string(),
            serde_json::Value::Object(m.into_iter().collect()),
        );
    }
    obj.insert(
        "result".to_string(),
        serde_json::Value::String(result.to_string()),
    );
    obj.insert(
        "reason".to_string(),
        serde_json::Value::String(reason.to_string()),
    );
    let entry = serde_json::Value::Object(obj);

    let Ok(line) = serializer(&entry) else {
        return;
    };

    // Check rotation (10MB)
    if let Ok(meta) = fs::metadata(&log_file) {
        if meta.len() >= 10 * 1024 * 1024 {
            let mut backup = log_file.to_path_buf();
            backup.set_extension("jsonl.1");
            let _ = fs::rename(&log_file, &backup);
        }
    }

    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_file) {
        let _ = writeln!(f, "{}", line);
    }
}

fn get_audit_file_path() -> PathBuf {
    audit_file_path_from_db_path(&crate::store::db_path())
}

fn audit_file_path_from_db_path(db_path: &Path) -> PathBuf {
    let mut p = db_path.to_path_buf();
    p.set_file_name("audit.jsonl");
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;
    use tempfile::tempdir;

    #[test]
    fn audit_log_writes_jsonl_with_expected_fields() {
        let dir = tempdir().unwrap();
        let log_file = dir.path().join("audit.jsonl");

        audit_log_to_file(&log_file, "act", "target", "user", "params", "ok", "reason");

        let mut s = String::new();
        File::open(&log_file)
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        let line = s.lines().next().expect("missing log line");
        let v: serde_json::Value = serde_json::from_str(line).expect("json");

        assert!(v.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0) > 0);
        assert_eq!(v.get("action").and_then(|v| v.as_str()), Some("act"));
        assert_eq!(v.get("target").and_then(|v| v.as_str()), Some("target"));
        assert_eq!(v.get("user").and_then(|v| v.as_str()), Some("user"));
        assert_eq!(v.get("params").and_then(|v| v.as_str()), Some("params"));
        assert_eq!(v.get("result").and_then(|v| v.as_str()), Some("ok"));
        assert_eq!(v.get("reason").and_then(|v| v.as_str()), Some("reason"));
    }

    #[test]
    fn audit_log_rotates_when_file_is_large() {
        let dir = tempdir().unwrap();
        let log_file = dir.path().join("audit.jsonl");
        let backup = dir.path().join("audit.jsonl.1");

        // Fast path: create a sparse/extended file to trigger rotation.
        let f = File::create(&log_file).unwrap();
        f.set_len(10 * 1024 * 1024).unwrap();
        drop(f);

        audit_log_to_file(&log_file, "a", "t", "u", "p", "r", "why");

        assert!(backup.exists(), "expected rotated backup to exist");
        assert!(log_file.exists(), "expected new log file to exist");
    }

    #[test]
    fn audit_file_path_is_next_to_db_path() {
        let db = Path::new(r"C:\tmp\.xun.json");
        assert_eq!(
            audit_file_path_from_db_path(db),
            Path::new(r"C:\tmp\audit.jsonl")
        );
    }

    #[test]
    fn audit_log_silently_returns_when_serializer_fails() {
        fn always_fail(_: &serde_json::Value) -> Result<String, serde_json::Error> {
            Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "boom",
            )))
        }

        let dir = tempdir().unwrap();
        let log_file = dir.path().join("audit.jsonl");

        audit_log_to_file_with_serializer(
            &log_file,
            "act",
            "target",
            "user",
            "params",
            "ok",
            "reason",
            always_fail,
        );

        assert!(
            !log_file.exists(),
            "log file should not be created on failure"
        );
    }
}
