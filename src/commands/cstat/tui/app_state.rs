use std::path::{Path, PathBuf};

use ratatui::widgets::{ListState, TableState};

use crate::cstat::report::Report;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Tab {
    Stats,
    Issues,
}

pub(super) struct App {
    pub(super) report: Report,
    pub(super) scan_path: String,
    pub(super) tab: Tab,
    pub(super) stats_state: TableState,
    pub(super) issues_state: ListState,
    pub(super) show_confirm: Option<PathBuf>,
    pub(super) message: Option<(String, bool)>,
}

impl App {
    pub(super) fn new(report: Report, scan_path: String) -> Self {
        let mut stats_state = TableState::default();
        if !report.stats.is_empty() {
            stats_state.select(Some(0));
        }
        let issues_state = ListState::default();
        App {
            report,
            scan_path,
            tab: Tab::Stats,
            stats_state,
            issues_state,
            show_confirm: None,
            message: None,
        }
    }

    pub(super) fn issue_items(&self) -> Vec<(String, Option<PathBuf>)> {
        let mut items: Vec<(String, Option<PathBuf>)> = Vec::new();
        for p in &self.report.issues.empty {
            items.push((format!("[empty]  {}", p), Some(PathBuf::from(p))));
        }
        for (p, lines) in &self.report.issues.large {
            items.push((format!("[large {}L]  {}", lines, p), Some(PathBuf::from(p))));
        }
        for p in &self.report.issues.tmp {
            items.push((format!("[tmp]  {}", p), Some(PathBuf::from(p))));
        }
        for group in &self.report.issues.dup {
            for p in group {
                items.push((format!("[dup]  {}", p), Some(PathBuf::from(p))));
            }
            items.push(("".into(), None));
        }
        items
    }

    pub(super) fn move_cursor(&mut self, delta: i32) {
        match self.tab {
            Tab::Stats => {
                let n = self.report.stats.len();
                if n == 0 {
                    return;
                }
                let cur = self.stats_state.selected().unwrap_or(0) as i32;
                let next = (cur + delta).rem_euclid(n as i32) as usize;
                self.stats_state.select(Some(next));
            }
            Tab::Issues => {
                let items = self.issue_items();
                let n = items.len();
                if n == 0 {
                    return;
                }
                let cur = self.issues_state.selected().unwrap_or(0) as i32;
                let next = (cur + delta).rem_euclid(n as i32) as usize;
                self.issues_state.select(Some(next));
            }
        }
    }

    pub(super) fn delete_selected(&mut self) {
        if self.tab != Tab::Issues {
            return;
        }
        let items = self.issue_items();
        if let Some(idx) = self.issues_state.selected()
            && let Some((_, Some(path))) = items.get(idx)
        {
            self.show_confirm = Some(path.clone());
            self.message = None;
        }
    }

    pub(super) fn remove_from_issues(&mut self, path: &Path) {
        let p = path.to_string_lossy().into_owned();
        self.report.issues.empty.retain(|x| x != &p);
        self.report.issues.large.retain(|(x, _)| x != &p);
        self.report.issues.tmp.retain(|x| x != &p);
        for group in &mut self.report.issues.dup {
            group.retain(|x| x != &p);
        }
        self.report.issues.dup.retain(|g| g.len() > 1);
    }

    pub(super) fn export_json(&mut self) {
        match serde_json::to_string_pretty(&self.report) {
            Ok(json) => {
                let out = "cstat-report.json";
                match std::fs::write(out, json) {
                    Ok(()) => self.message = Some((format!("Exported to {}", out), false)),
                    Err(e) => self.message = Some((format!("Export error: {}", e), true)),
                }
            }
            Err(e) => self.message = Some((format!("Serialise error: {}", e), true)),
        }
    }
}
