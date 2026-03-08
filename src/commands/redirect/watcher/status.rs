use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::output::{apply_pretty_table_style, print_table};

use comfy_table::{Attribute, Cell, Color, Table};

pub(super) const WATCH_STATUS_FILE: &str = ".xun_watch_status.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WatchStatus {
    pub(crate) pid: u32,
    pub(crate) tx: String,
    pub(crate) profile: String,
    pub(crate) source: String,
    pub(crate) started_ts: u64,
    pub(crate) last_scan_ts: u64,
    pub(crate) batches: u64,
    pub(crate) events_processed: u64,
    pub(crate) retry_queue: Vec<String>,
    pub(crate) errors: u64,
}

pub(super) struct WatchStatusWriter {
    path: PathBuf,
    last_flush: Instant,
}

impl WatchStatusWriter {
    pub(super) fn new(source: &Path, _tx: &str, _profile: &str, _buffer_len: u32) -> Self {
        Self {
            path: source.join(WATCH_STATUS_FILE),
            last_flush: Instant::now() - Duration::from_secs(60),
        }
    }

    pub(super) fn flush(&mut self, status: &WatchStatus) {
        let tmp = self.path.with_extension("json.tmp");
        if let Ok(s) = serde_json::to_string_pretty(status) {
            let _ = std::fs::write(&tmp, s);
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::rename(&tmp, &self.path);
        }
        self.last_flush = Instant::now();
    }

    pub(super) fn maybe_flush(&mut self, status: &WatchStatus) {
        if self.last_flush.elapsed() >= Duration::from_secs(1) {
            self.flush(status);
        }
    }
}

pub(crate) fn read_watch_status(source: &Path) -> Result<WatchStatus, String> {
    let source_abs = source
        .canonicalize()
        .unwrap_or_else(|_| source.to_path_buf());
    let path = source_abs.join(WATCH_STATUS_FILE);
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read watch status: {} ({})", path.display(), e))?;
    serde_json::from_str::<WatchStatus>(&raw)
        .map_err(|e| format!("Invalid watch status json: {} ({})", path.display(), e))
}

pub(crate) fn render_watch_status(status: &WatchStatus, format: crate::model::ListFormat) {
    use crate::model::ListFormat;
    match format {
        ListFormat::Tsv => {
            out_println!("pid\t{}", status.pid);
            out_println!("tx\t{}", status.tx);
            out_println!("profile\t{}", status.profile);
            out_println!("source\t{}", status.source);
            out_println!("started_ts\t{}", status.started_ts);
            out_println!("last_scan_ts\t{}", status.last_scan_ts);
            out_println!("batches\t{}", status.batches);
            out_println!("events_processed\t{}", status.events_processed);
            out_println!("errors\t{}", status.errors);
            out_println!("retry_queue\t{}", status.retry_queue.join(","));
        }
        ListFormat::Json => {
            out_println!("{}", serde_json::to_string(status).unwrap_or_default());
        }
        ListFormat::Table => {
            let mut table = Table::new();
            apply_pretty_table_style(&mut table);
            table.set_header(vec![
                Cell::new("Key")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Cyan),
                Cell::new("Value")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Yellow),
            ]);
            table.add_row(vec![Cell::new("pid"), Cell::new(status.pid)]);
            table.add_row(vec![Cell::new("tx"), Cell::new(&status.tx)]);
            table.add_row(vec![Cell::new("profile"), Cell::new(&status.profile)]);
            table.add_row(vec![Cell::new("source"), Cell::new(&status.source)]);
            table.add_row(vec![Cell::new("started_ts"), Cell::new(status.started_ts)]);
            table.add_row(vec![
                Cell::new("last_scan_ts"),
                Cell::new(status.last_scan_ts),
            ]);
            table.add_row(vec![Cell::new("batches"), Cell::new(status.batches)]);
            table.add_row(vec![
                Cell::new("events_processed"),
                Cell::new(status.events_processed),
            ]);
            table.add_row(vec![Cell::new("errors"), Cell::new(status.errors)]);
            table.add_row(vec![
                Cell::new("retry_queue"),
                Cell::new(status.retry_queue.join(","))
                    .fg(Color::DarkGrey)
                    .add_attribute(Attribute::Dim),
            ]);
            print_table(&table);
        }
        ListFormat::Auto => unreachable!(),
    }
}
