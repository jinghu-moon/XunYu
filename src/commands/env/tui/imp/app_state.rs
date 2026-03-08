use super::*;

pub(super) struct App {
    pub(super) manager: EnvManager,
    pub(super) scope: EnvScope,
    pub(super) panel: Panel,
    pub(super) vars: Vec<EnvVar>,
    pub(super) filtered_vars: Vec<usize>,
    pub(super) var_query: String,
    pub(super) var_state: ListState,
    pub(super) paths: Vec<String>,
    pub(super) path_state: ListState,
    pub(super) snapshots: Vec<SnapshotMeta>,
    pub(super) snapshot_state: ListState,
    pub(super) profiles: Vec<EnvProfileMeta>,
    pub(super) profile_state: ListState,
    pub(super) audit_entries: Vec<EnvAuditEntry>,
    pub(super) audit_state: ListState,
    pub(super) doctor: Option<DoctorReport>,
    pub(super) status: String,
    pub(super) is_elevated: bool,
    pub(super) show_help: bool,
}

impl App {
    pub(super) fn new() -> Self {
        let mut app = Self {
            manager: EnvManager::new(),
            scope: EnvScope::User,
            panel: Panel::Vars,
            vars: Vec::new(),
            filtered_vars: Vec::new(),
            var_query: String::new(),
            var_state: ListState::default(),
            paths: Vec::new(),
            path_state: ListState::default(),
            snapshots: Vec::new(),
            snapshot_state: ListState::default(),
            profiles: Vec::new(),
            profile_state: ListState::default(),
            audit_entries: Vec::new(),
            audit_state: ListState::default(),
            doctor: None,
            status: "ready".to_string(),
            is_elevated: uac::is_elevated(),
            show_help: false,
        };
        app.refresh_all();
        app
    }

    pub(super) fn refresh_all(&mut self) {
        match self.manager.list_vars(self.scope) {
            Ok(v) => {
                self.vars = v;
                self.rebuild_var_filter();
            }
            Err(e) => self.status = e.to_string(),
        }
        match self.manager.path_entries(self.scope) {
            Ok(v) => {
                self.paths = v;
                sync_selection(&mut self.path_state, self.paths.len());
            }
            Err(e) => self.status = e.to_string(),
        }
        match self.manager.snapshot_list() {
            Ok(v) => {
                self.snapshots = v;
                sync_selection(&mut self.snapshot_state, self.snapshots.len());
            }
            Err(e) => self.status = e.to_string(),
        }
        match self.manager.profile_list() {
            Ok(v) => {
                self.profiles = v;
                sync_selection(&mut self.profile_state, self.profiles.len());
            }
            Err(e) => self.status = e.to_string(),
        }
        match self.manager.audit_list(200) {
            Ok(mut v) => {
                v.reverse();
                self.audit_entries = v;
                sync_selection(&mut self.audit_state, self.audit_entries.len());
            }
            Err(e) => self.status = e.to_string(),
        }
    }

    pub(super) fn rebuild_var_filter(&mut self) {
        self.filtered_vars.clear();
        if self.var_query.trim().is_empty() {
            self.filtered_vars.extend(0..self.vars.len());
        } else {
            let query = self.var_query.to_ascii_lowercase();
            self.filtered_vars
                .extend(self.vars.iter().enumerate().filter_map(|(idx, v)| {
                    let name = v.name.to_ascii_lowercase();
                    let value = v.raw_value.to_ascii_lowercase();
                    if name.contains(&query) || value.contains(&query) {
                        Some(idx)
                    } else {
                        None
                    }
                }));
        }
        sync_selection(&mut self.var_state, self.filtered_vars.len());
    }

    pub(super) fn move_cursor(&mut self, delta: i32) {
        let (len, state) = match self.panel {
            Panel::Vars => (self.filtered_vars.len(), &mut self.var_state),
            Panel::Path => (self.paths.len(), &mut self.path_state),
            Panel::Snapshots => (self.snapshots.len(), &mut self.snapshot_state),
            Panel::Profiles => (self.profiles.len(), &mut self.profile_state),
            Panel::History => (self.audit_entries.len(), &mut self.audit_state),
            Panel::Doctor | Panel::Io => return,
        };
        if len == 0 {
            return;
        }
        let cur = state.selected().unwrap_or(0) as i32;
        let next = (cur + delta).rem_euclid(len as i32) as usize;
        state.select(Some(next));
    }

    pub(super) fn current_var(&self) -> Option<EnvVar> {
        let view_idx = self.var_state.selected()?;
        let idx = *self.filtered_vars.get(view_idx)?;
        self.vars.get(idx).cloned()
    }

    pub(super) fn current_path(&self) -> Option<String> {
        let idx = self.path_state.selected()?;
        self.paths.get(idx).cloned()
    }

    pub(super) fn current_snapshot_id(&self) -> Option<String> {
        let idx = self.snapshot_state.selected()?;
        self.snapshots.get(idx).map(|s| s.id.clone())
    }

    pub(super) fn current_snapshot(&self) -> Option<SnapshotMeta> {
        let idx = self.snapshot_state.selected()?;
        self.snapshots.get(idx).cloned()
    }

    pub(super) fn current_profile_name(&self) -> Option<String> {
        let idx = self.profile_state.selected()?;
        self.profiles.get(idx).map(|p| p.name.clone())
    }
}

pub(super) fn sync_selection(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
        return;
    }
    let idx = state.selected().unwrap_or(0).min(len - 1);
    state.select(Some(idx));
}

pub(super) fn default_profile_name() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("profile-{}", ts)
}

pub(super) fn map_env_err(err: crate::env_core::types::EnvError) -> CliError {
    CliError::new(err.exit_code(), err.to_string())
}

pub(super) fn trim_for_ui(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let mut out = String::new();
    for ch in value.chars().take(max.saturating_sub(1)) {
        out.push(ch);
    }
    out.push('…');
    out
}

pub(super) fn normalize_path_key_for_ui(value: &str) -> String {
    expand_percent_vars(value).to_ascii_lowercase()
}

pub(super) fn path_entry_exists(value: &str) -> bool {
    let expanded = expand_percent_vars(value);
    std::path::Path::new(&expanded).exists()
}

pub(super) fn expand_percent_vars(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '%' {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && chars[j] != '%' {
                j += 1;
            }
            if j < chars.len() && j > start {
                let key: String = chars[start..j].iter().collect();
                if let Ok(v) = std::env::var(&key) {
                    out.push_str(&v);
                } else {
                    out.push('%');
                    out.push_str(&key);
                    out.push('%');
                }
                i = j + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

pub(super) fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
