use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::util::normalize_path;

pub(super) struct RetryItem {
    pub(super) path: PathBuf,
    pub(super) reason: String,
    next_due: Instant,
}

#[derive(Default)]
pub(super) struct RetryQueue {
    items: VecDeque<RetryItem>,
    seen: HashSet<String>,
}

impl RetryQueue {
    pub(super) fn push(&mut self, path: PathBuf, reason: String) {
        let key = normalize_path(&path.to_string_lossy());
        if !self.seen.insert(key) {
            return;
        }
        self.items.push_back(RetryItem {
            path,
            reason,
            next_due: Instant::now() + Duration::from_millis(200),
        });
    }

    pub(super) fn pop_due(&mut self, retry_ms: u64, limit: usize) -> Option<Vec<RetryItem>> {
        let now = Instant::now();
        let mut due = Vec::new();
        let mut remaining = VecDeque::new();
        while let Some(item) = self.items.pop_front() {
            if item.next_due <= now && (limit == 0 || due.len() < limit) {
                due.push(item);
            } else {
                remaining.push_back(item);
            }
        }
        self.items = remaining;
        if due.is_empty() {
            return None;
        }
        // reset seen keys for due items so they can be requeued on failure
        for d in &due {
            self.seen.remove(&normalize_path(&d.path.to_string_lossy()));
        }
        for d in &mut due {
            d.next_due = Instant::now() + Duration::from_millis(retry_ms);
        }
        Some(due)
    }

    pub(super) fn sample_paths(&self, max: usize) -> Vec<String> {
        self.items
            .iter()
            .take(max)
            .map(|it| it.path.to_string_lossy().to_string())
            .collect()
    }
}
