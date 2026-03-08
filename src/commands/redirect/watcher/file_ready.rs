use std::io;
use std::path::Path;
use std::time::Duration;

use crate::windows::handle_query;

pub(super) enum FileReady {
    Ready,
    NotReady(String),
}

pub(super) fn file_ready(path: &Path, settle_ms: u64) -> io::Result<FileReady> {
    let meta1 = std::fs::metadata(path)?;
    let len1 = meta1.len();
    std::thread::sleep(Duration::from_millis(settle_ms));
    let meta2 = std::fs::metadata(path)?;
    let len2 = meta2.len();
    if len1 != len2 {
        return Ok(FileReady::NotReady("writing".to_string()));
    }

    // Exclusive open attempts to detect in-use/locked writers without requiring lock feature.
    let mut opts = std::fs::OpenOptions::new();
    opts.read(true).write(true);
    #[allow(clippy::useless_conversion)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        opts.share_mode(0);
    }
    match opts.open(path) {
        Ok(f) => {
            drop(f);
            Ok(FileReady::Ready)
        }
        Err(e) if matches!(e.raw_os_error(), Some(32) | Some(33) | Some(5)) => {
            // sharing violation / lock violation / access denied
            let reason = lock_reason(path, e.raw_os_error());
            Ok(FileReady::NotReady(reason))
        }
        Err(e) => Err(e),
    }
}

fn lock_reason(path: &Path, code: Option<i32>) -> String {
    let Some(code) = code else {
        return "not_ready".to_string();
    };
    let mut reason = match code {
        32 => "sharing_violation".to_string(),
        33 => "lock_violation".to_string(),
        5 => "access_denied".to_string(),
        _ => format!("not_ready:{code}"),
    };

    match handle_query::get_locking_processes(&[path]) {
        Ok(lockers) if lockers.is_empty() => reason,
        Ok(lockers) => {
            let parts: Vec<String> = lockers
                .into_iter()
                .take(3)
                .map(|l| format!("{}:{}", l.pid, sanitize_locker_name(&l.name)))
                .collect();
            if parts.is_empty() {
                return reason;
            }
            reason.push_str(" locked_by=");
            reason.push_str(&parts.join(","));
            reason
        }
        Err(e) => {
            reason.push_str(&format!(
                " lock_query_failed(stage={},code={})",
                e.stage_name(),
                e.code
            ));
            reason
        }
    }
}

fn sanitize_locker_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }
    // Keep log lines short and stable.
    let mut s = trimmed.replace(['\t', '\r', '\n'], " ");
    if s.len() > 48 {
        s.truncate(48);
    }
    s
}
