use super::*;

// --- Audit ---

#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct AuditQuery {
    limit: Option<usize>,
    search: Option<String>,
    action: Option<String>,
    result: Option<String>,
    from: Option<u64>,
    to: Option<u64>,
    cursor: Option<usize>,
    format: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct AuditEntry {
    timestamp: u64,
    action: String,
    target: String,
    user: String,
    params: String,
    result: String,
    reason: String,
}

#[derive(Serialize)]
pub(in crate::commands::dashboard) struct AuditStats {
    total: usize,
    by_action: HashMap<String, usize>,
    by_result: HashMap<String, usize>,
}

#[derive(Serialize)]
pub(in crate::commands::dashboard) struct AuditResponse {
    entries: Vec<AuditEntry>,
    stats: AuditStats,
    next_cursor: Option<String>,
}

fn csv_escape(input: &str) -> String {
    if input.contains(',') || input.contains('"') || input.contains('\n') || input.contains('\r') {
        let mut out = String::with_capacity(input.len() + 2);
        out.push('"');
        for ch in input.chars() {
            if ch == '"' {
                out.push('"');
            }
            out.push(ch);
        }
        out.push('"');
        out
    } else {
        input.to_string()
    }
}

pub(in crate::commands::dashboard) async fn get_audit(Query(q): Query<AuditQuery>) -> Response {
    let limit = q.limit.unwrap_or(200).clamp(1, 2000);
    let format = q.format.as_deref().unwrap_or("json").to_ascii_lowercase();
    if format != "json" && format != "csv" {
        return (StatusCode::BAD_REQUEST, "invalid format").into_response();
    }
    let audit_path = audit_file_path();

    let mut entries = read_audit_tail(&audit_path, 2 * 1024 * 1024, 5000).unwrap_or_default();

    if let Some(from) = q.from {
        entries.retain(|e| e.timestamp >= from);
    }
    if let Some(to) = q.to {
        entries.retain(|e| e.timestamp <= to);
    }

    if q.search.is_some() || q.action.is_some() || q.result.is_some() {
        let search = q.search.as_deref().map(|s| s.to_ascii_lowercase());
        let action = q.action.as_deref().map(|s| s.to_ascii_lowercase());
        let result = q.result.as_deref().map(|s| s.to_ascii_lowercase());
        entries.retain(|e| {
            if let Some(a) = &action
                && e.action.to_ascii_lowercase() != *a
            {
                return false;
            }
            if let Some(r) = &result
                && e.result.to_ascii_lowercase() != *r
            {
                return false;
            }
            if let Some(s) = &search {
                let hay = format!(
                    "{}\n{}\n{}\n{}\n{}\n{}",
                    e.action, e.target, e.user, e.params, e.result, e.reason
                )
                .to_ascii_lowercase();
                hay.contains(s)
            } else {
                true
            }
        });
    }

    let stats = compute_audit_stats(&entries);
    let total = entries.len();
    let offset = q.cursor.unwrap_or(0);
    if offset >= total {
        entries.clear();
    } else {
        entries = entries.into_iter().skip(offset).collect();
    }
    entries.truncate(limit);

    let page_len = entries.len();
    let next_cursor = if offset + page_len < total {
        Some((offset + page_len).to_string())
    } else {
        None
    };

    if format == "csv" {
        let mut out = String::from("timestamp,action,target,user,params,result,reason\n");
        for e in entries {
            out.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                e.timestamp,
                csv_escape(&e.action),
                csv_escape(&e.target),
                csv_escape(&e.user),
                csv_escape(&e.params),
                csv_escape(&e.result),
                csv_escape(&e.reason)
            ));
        }
        ([(header::CONTENT_TYPE, "text/csv; charset=utf-8")], out).into_response()
    } else {
        Json(AuditResponse {
            entries,
            stats,
            next_cursor,
        })
        .into_response()
    }
}

pub(in crate::commands::dashboard) fn latest_audit_entries(limit: usize) -> Vec<AuditEntry> {
    read_audit_tail(&audit_file_path(), 2 * 1024 * 1024, limit.clamp(1, 500)).unwrap_or_default()
}

pub(in crate::commands::dashboard) fn audit_entry_count() -> usize {
    let path = audit_file_path();
    let Ok(content) = std::fs::read_to_string(path) else {
        return 0;
    };
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn audit_file_path() -> std::path::PathBuf {
    let mut p = crate::bookmark::storage::db_path();
    p.set_file_name("audit.jsonl");
    p
}

fn read_audit_tail(
    path: &std::path::Path,
    max_bytes: usize,
    max_lines: usize,
) -> std::io::Result<Vec<AuditEntry>> {
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    let start = len.saturating_sub(max_bytes as u64);
    f.seek(SeekFrom::Start(start))?;

    let mut s = String::new();
    f.read_to_string(&mut s)?;

    let mut lines: Vec<&str> = s.lines().collect();
    if start > 0 && !lines.is_empty() {
        lines.remove(0); // potentially partial line
    }
    if lines.is_empty() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for line in lines.into_iter().rev().take(max_lines) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            out.push(AuditEntry {
                timestamp: v.get("timestamp").and_then(|x| x.as_u64()).unwrap_or(0),
                action: v
                    .get("action")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                target: v
                    .get("target")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                user: v
                    .get("user")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                params: v
                    .get("params")
                    .map(|x| match x {
                        serde_json::Value::String(s) => s.clone(),
                        _ => x.to_string(),
                    })
                    .unwrap_or_default(),
                result: v
                    .get("result")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                reason: v
                    .get("reason")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
            });
        }
    }
    Ok(out)
}

fn compute_audit_stats(entries: &[AuditEntry]) -> AuditStats {
    let mut by_action: HashMap<String, usize> = HashMap::new();
    let mut by_result: HashMap<String, usize> = HashMap::new();
    for e in entries {
        *by_action.entry(e.action.clone()).or_insert(0) += 1;
        *by_result.entry(e.result.clone()).or_insert(0) += 1;
    }
    AuditStats {
        total: entries.len(),
        by_action,
        by_result,
    }
}

