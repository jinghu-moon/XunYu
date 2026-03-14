use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};

/// A single audit log entry written as one JSON Line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// ISO-8601 timestamp.
    pub ts: String,
    /// Command / operation name (e.g. `"AddPermission"`).
    pub action: String,
    /// Target filesystem path.
    pub path: String,
    /// Human-readable detail string.
    pub details: String,
    /// `true` when the operation completed without error.
    pub success: bool,
    /// Error message when `success == false`.
    pub error: String,
    /// `DOMAIN\User` of the process that performed the action.
    pub run_as: String,
}

impl AuditEntry {
    /// Convenience constructor.
    pub fn new(
        action: impl Into<String>,
        path: impl Into<String>,
        details: impl Into<String>,
        success: bool,
        error: impl Into<String>,
    ) -> Self {
        Self {
            ts: Local::now().to_rfc3339(),
            action: action.into(),
            path: path.into(),
            details: details.into(),
            success,
            error: error.into(),
            run_as: current_user(),
        }
    }

    /// Build a success entry.
    pub fn ok(
        action: impl Into<String>,
        path: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::new(action, path, details, true, "")
    }

    /// Build a failure entry.
    pub fn fail(
        action: impl Into<String>,
        path: impl Into<String>,
        details: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self::new(action, path, details, false, error)
    }
}

/// Append-only JSON Lines audit log with automatic line-count rolling.
pub struct AuditLog {
    path: PathBuf,
    max_lines: usize,
}

impl AuditLog {
    pub fn new(path: PathBuf, max_lines: usize) -> Self {
        Self { path, max_lines }
    }

    /// Append `entry` as a single JSON line.
    ///
    /// Creates the file (and parent directories) if they do not exist.
    /// After writing, triggers [`Self::rotate_if_needed`] if the line count
    /// exceeds `max_lines`.
    pub fn append(&self, entry: &AuditEntry) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("audit: cannot create dir {}", parent.display()))?;
        }

        let line = serde_json::to_string(entry).context("audit: failed to serialize entry")?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("audit: cannot open {}", self.path.display()))?;

        writeln!(file, "{}", line)
            .with_context(|| format!("audit: failed to write to {}", self.path.display()))?;

        self.rotate_if_needed()
            .context("audit: rotation failed (log was written)")?;

        Ok(())
    }

    /// Append multiple entries as JSON Lines, then rotate once.
    pub fn append_many(&self, entries: &[AuditEntry]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("audit: cannot create dir {}", parent.display()))?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("audit: cannot open {}", self.path.display()))?;

        for entry in entries {
            let line = serde_json::to_string(entry).context("audit: failed to serialize entry")?;
            writeln!(file, "{}", line)
                .with_context(|| format!("audit: failed to write to {}", self.path.display()))?;
        }

        self.rotate_if_needed()
            .context("audit: rotation failed (log was written)")?;

        Ok(())
    }

    /// Append multiple entries as JSON Lines without triggering rotation.
    pub fn append_many_no_rotate(&self, entries: &[AuditEntry]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("audit: cannot create dir {}", parent.display()))?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("audit: cannot open {}", self.path.display()))?;

        for entry in entries {
            let line = serde_json::to_string(entry).context("audit: failed to serialize entry")?;
            writeln!(file, "{}", line)
                .with_context(|| format!("audit: failed to write to {}", self.path.display()))?;
        }

        Ok(())
    }

    /// Keep only the last `max_lines` lines if the file has grown beyond that.
    ///
    /// Reads the tail of the file, then rewrites the whole file.  Called
    /// automatically by [`append`] and is cheap when the line count is within
    /// limits (fast-path: count lines; only rewrites when over limit).
    pub fn rotate_if_needed(&self) -> Result<()> {
        let lines = self.count_lines()?;
        if lines <= self.max_lines {
            return Ok(());
        }

        // Read the tail we want to keep
        let keep = self.tail_raw(self.max_lines)?;

        // Overwrite the file
        let mut file = File::create(&self.path)
            .with_context(|| format!("audit: cannot truncate {}", self.path.display()))?;
        for line in &keep {
            writeln!(file, "{}", line).with_context(|| "audit: failed to write during rotation")?;
        }
        Ok(())
    }

    /// Count the number of non-empty lines in the log file.
    fn count_lines(&self) -> Result<usize> {
        if !self.path.exists() {
            return Ok(0);
        }
        let file = File::open(&self.path)
            .with_context(|| format!("audit: cannot open {}", self.path.display()))?;
        let count = BufReader::new(file)
            .lines()
            .filter(|l| l.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false))
            .count();
        Ok(count)
    }

    /// Return the last `n` raw JSON strings (newest last).
    fn tail_raw(&self, n: usize) -> Result<Vec<String>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let file = File::open(&self.path)
            .with_context(|| format!("audit: cannot open {}", self.path.display()))?;
        let all: Vec<String> = BufReader::new(file)
            .lines()
            .filter_map(|l| {
                let l = l.ok()?;
                if l.trim().is_empty() { None } else { Some(l) }
            })
            .collect();

        let start = if all.len() > n { all.len() - n } else { 0 };
        Ok(all[start..].to_vec())
    }

    /// Return the last `n` deserialized entries (newest last).
    pub fn tail(&self, n: usize) -> Result<Vec<AuditEntry>> {
        let raw = self.tail_raw(n)?;
        raw.iter()
            .map(|line| {
                serde_json::from_str::<AuditEntry>(line)
                    .with_context(|| format!("audit: failed to parse line: {line}"))
            })
            .collect()
    }

    /// Export every entry in the log as a CSV file at `dest`.
    ///
    /// Returns the number of rows written.
    pub fn export_csv(&self, dest: &Path) -> Result<usize> {
        if !self.path.exists() {
            return Ok(0);
        }
        let file = File::open(&self.path)
            .with_context(|| format!("audit: cannot open {}", self.path.display()))?;

        let mut wtr = csv::Writer::from_path(dest)
            .with_context(|| format!("audit: cannot create CSV at {}", dest.display()))?;

        // Header
        wtr.write_record([
            "Timestamp",
            "Action",
            "Path",
            "Details",
            "Success",
            "Error",
            "RunAs",
        ])?;

        let mut count = 0usize;
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let e: AuditEntry = serde_json::from_str(&line)
                .with_context(|| format!("audit: cannot parse line: {line}"))?;
            let success = if e.success { "true" } else { "false" };
            wtr.write_record([
                e.ts.as_str(),
                e.action.as_str(),
                e.path.as_str(),
                e.details.as_str(),
                success,
                e.error.as_str(),
                e.run_as.as_str(),
            ])?;
            count += 1;
        }
        wtr.flush()?;
        Ok(count)
    }
}

// ── Platform helpers ──────────────────────────────────────────────────────────

/// Return `DOMAIN\User` of the current Windows process.
pub fn current_user() -> String {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::System::WindowsProgramming::GetUserNameW;
    use windows::core::PWSTR;

    unsafe {
        let mut buf = vec![0u16; 256];
        let mut len = buf.len() as u32;
        if GetUserNameW(PWSTR(buf.as_mut_ptr()), &mut len).is_ok() && len > 1 {
            let s = OsString::from_wide(&buf[..(len as usize - 1)]);
            return s.to_string_lossy().into_owned();
        }
    }
    // Fallback: environment variable
    std::env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_log() -> (AuditLog, PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "aclmgr_audit_test_{}.jsonl",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        let log = AuditLog::new(path.clone(), 5);
        (log, path)
    }

    #[test]
    fn append_and_tail() {
        let (log, path) = temp_log();

        for i in 0..3 {
            log.append(&AuditEntry::ok("Test", "/some/path", format!("detail {i}")))
                .unwrap();
        }

        let entries = log.tail(10).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].details, "detail 0");
        assert_eq!(entries[2].details, "detail 2");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn rotation_trims_to_max() {
        let (log, path) = temp_log(); // max_lines = 5

        for i in 0..8 {
            log.append(&AuditEntry::ok("Test", "/p", format!("entry {i}")))
                .unwrap();
        }

        let entries = log.tail(10).unwrap();
        // Should have been trimmed to 5
        assert_eq!(
            entries.len(),
            5,
            "expected 5 after rotation, got {}",
            entries.len()
        );
        // Oldest kept should be entry 3 (entries 0-2 rolled out)
        assert_eq!(entries[0].details, "entry 3");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn tail_returns_newest_last() {
        let (log, path) = temp_log();
        for i in 0..3 {
            log.append(&AuditEntry::ok("T", "/p", format!("n{i}")))
                .unwrap();
        }
        let tail = log.tail(2).unwrap();
        assert_eq!(tail.len(), 2);
        assert_eq!(tail[0].details, "n1");
        assert_eq!(tail[1].details, "n2");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn export_csv_writes_rows() {
        let (log, path) = temp_log();
        for i in 0..3 {
            log.append(&AuditEntry::ok("Act", "/path", format!("d{i}")))
                .unwrap();
        }
        let csv_path = path.with_extension("csv");
        let n = log.export_csv(&csv_path).unwrap();
        assert_eq!(n, 3);
        let content = std::fs::read_to_string(&csv_path).unwrap();
        assert!(content.contains("Timestamp"));
        assert!(content.contains("Act"));
        std::fs::remove_file(&path).ok();
        std::fs::remove_file(&csv_path).ok();
    }
}
