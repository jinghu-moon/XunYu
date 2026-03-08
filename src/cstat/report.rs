// cstat/report.rs
//
// Aggregation types for per-language statistics and issue lists.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::cstat::scanner::FileStat;

// ─── Public data structures ──────────────────────────────────────────────────

#[derive(Default, Clone, Serialize, Deserialize)]
pub(crate) struct LangStat {
    pub name: String,
    pub files: u32,
    pub code: u32,
    pub comment: u32,
    pub blank: u32,
    pub bytes: u64,
}

impl LangStat {
    pub fn total_lines(&self) -> u32 {
        self.code + self.comment + self.blank
    }
}

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct Issues {
    pub empty: Vec<String>,
    pub large: Vec<(String, u32)>,
    pub tmp: Vec<String>,
    pub dup: Vec<Vec<String>>,
}

impl Issues {
    pub fn is_empty(&self) -> bool {
        self.empty.is_empty() && self.large.is_empty() && self.tmp.is_empty() && self.dup.is_empty()
    }
}

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct Report {
    pub stats: Vec<LangStat>,
    pub issues: Issues,
}

// ─── Aggregation ─────────────────────────────────────────────────────────────

pub(crate) fn accumulate(
    map: &mut HashMap<String, LangStat>,
    lang_name: &str,
    file_stat: &FileStat,
) {
    let entry = map.entry(lang_name.to_owned()).or_insert_with(|| LangStat {
        name: lang_name.to_owned(),
        ..Default::default()
    });
    entry.files += 1;
    entry.code += file_stat.code;
    entry.comment += file_stat.comment;
    entry.blank += file_stat.blank;
    entry.bytes += file_stat.bytes;
}

/// Convert accumulation map to sorted Vec (descending by code lines).
pub(crate) fn finalize(map: HashMap<String, LangStat>) -> Vec<LangStat> {
    let mut v: Vec<LangStat> = map.into_values().collect();
    v.sort_by(|a, b| b.code.cmp(&a.code));
    v
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub(crate) fn fmt_bytes(b: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if b >= GB {
        format!("{:.1} GB", b as f64 / GB as f64)
    } else if b >= MB {
        format!("{:.1} MB", b as f64 / MB as f64)
    } else if b >= KB {
        format!("{:.1} KB", b as f64 / KB as f64)
    } else {
        format!("{} B", b)
    }
}
