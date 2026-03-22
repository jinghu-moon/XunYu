use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use dialoguer::{Confirm, Input};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use std::io;
use std::process::Command;
use std::sync::{Arc, Mutex};

use crate::config::{
    DesktopAwakeConfig, DesktopDaemonConfig, DesktopLayout, DesktopRemap, DesktopSnippet,
    DesktopThemeConfig, DesktopWorkspace,
};
use crate::desktop::awake::{AwakeMode, AwakeState};
use crate::desktop::theme::ThemeMode;
use crate::output::{CliError, CliResult};
use crate::windows::window_api;

pub(crate) fn run_desktop_tui() -> CliResult {
    let mut app = App::new();

    enable_raw_mode().map_err(|e| CliError::new(1, format!("tui init: {e}")))?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| CliError::new(1, format!("{e}")))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| CliError::new(1, format!("{e}")))?;

    let result = run_loop(&mut terminal, &mut app);

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> CliResult {
    loop {
        terminal
            .draw(|f| draw_ui(f, app))
            .map_err(|e| CliError::new(1, format!("tui draw: {e}")))?;

        let ev = event::read().map_err(|e| CliError::new(1, format!("tui event: {e}")))?;
        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => break,
                KeyCode::Tab => app.next_panel(),
                KeyCode::Up | KeyCode::Char('k') => app.move_cursor(-1),
                KeyCode::Down | KeyCode::Char('j') => app.move_cursor(1),
                KeyCode::Char('r') | KeyCode::F(5) => app.refresh(),
                _ => app.handle_action(key.code),
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Apps,
    Hotkeys,
    Remaps,
    Snippets,
    Layouts,
    Workspaces,
    Hosts,
    Theme,
    Awake,
    Daemon,
}

impl Panel {
    fn label(self) -> &'static str {
        match self {
            Self::Apps => "Apps",
            Self::Hotkeys => "Hotkeys",
            Self::Remaps => "Remaps",
            Self::Snippets => "Snippets",
            Self::Layouts => "Layouts",
            Self::Workspaces => "Workspaces",
            Self::Hosts => "Hosts",
            Self::Theme => "Theme",
            Self::Awake => "Awake",
            Self::Daemon => "Daemon",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Self::Apps => "enter: run",
            Self::Hotkeys => "a:add d:remove",
            Self::Remaps => "a:add d:remove c:clear",
            Self::Snippets => "a:add d:remove c:clear",
            Self::Layouts => "a:new d:remove enter:apply",
            Self::Workspaces => "a:save d:remove enter:launch",
            Self::Hosts => "a:add d:remove",
            Self::Theme => "enter:toggle l:light d:dark f:follow",
            Self::Awake => "enter:toggle o:display_on",
            Self::Daemon => "m:quiet n:no_tray",
        }
    }
}

struct App {
    panels: Vec<Panel>,
    panel_idx: usize,
    menu_state: ListState,
    list_state: ListState,
    status: String,
    apps: Vec<crate::desktop::apps::InstalledApp>,
    bindings: Vec<crate::config::DesktopBinding>,
    remaps: Vec<DesktopRemap>,
    snippets: Vec<DesktopSnippet>,
    layouts: Vec<DesktopLayout>,
    workspaces: Vec<DesktopWorkspace>,
    hosts: Vec<crate::desktop::hosts::HostEntry>,
    theme_cfg: DesktopThemeConfig,
    awake_cfg: DesktopAwakeConfig,
    daemon_cfg: DesktopDaemonConfig,
    awake_state: Arc<Mutex<AwakeState>>,
    awake_mode: AwakeMode,
    theme_mode: ThemeMode,
}

impl App {
    fn new() -> Self {
        let panels = vec![
            Panel::Apps,
            Panel::Hotkeys,
            Panel::Remaps,
            Panel::Snippets,
            Panel::Layouts,
            Panel::Workspaces,
            Panel::Hosts,
            Panel::Theme,
            Panel::Awake,
            Panel::Daemon,
        ];
        let awake_state = AwakeState::new();
        let mut app = Self {
            panels,
            panel_idx: 0,
            menu_state: ListState::default(),
            list_state: ListState::default(),
            status: "ready".to_string(),
            apps: Vec::new(),
            bindings: Vec::new(),
            remaps: Vec::new(),
            snippets: Vec::new(),
            layouts: Vec::new(),
            workspaces: Vec::new(),
            hosts: Vec::new(),
            theme_cfg: DesktopThemeConfig::default(),
            awake_cfg: DesktopAwakeConfig::default(),
            daemon_cfg: DesktopDaemonConfig::default(),
            awake_state,
            awake_mode: AwakeMode::Off,
            theme_mode: ThemeMode::Light,
        };
        app.menu_state.select(Some(0));
        app.refresh();
        app
    }

    fn panel(&self) -> Panel {
        self.panels[self.panel_idx]
    }

    fn next_panel(&mut self) {
        if self.panels.is_empty() {
            return;
        }
        self.panel_idx = (self.panel_idx + 1) % self.panels.len();
        self.menu_state.select(Some(self.panel_idx));
        self.reset_list_state();
    }

    fn refresh(&mut self) {
        let cfg = crate::config::load_config();
        self.bindings = cfg.desktop.bindings;
        self.remaps = cfg.desktop.remaps;
        self.snippets = cfg.desktop.snippets;
        self.layouts = cfg.desktop.layouts;
        self.workspaces = cfg.desktop.workspaces;
        self.theme_cfg = cfg.desktop.theme;
        self.awake_cfg = cfg.desktop.awake;
        self.daemon_cfg = cfg.desktop.daemon;
        self.apps = crate::desktop::apps::list_installed_apps();
        match crate::desktop::hosts::list_entries() {
            Ok(entries) => self.hosts = entries,
            Err(err) => {
                self.hosts.clear();
                self.status = err.message;
            }
        }
        self.theme_mode = crate::desktop::theme::get_current_theme();
        if let Ok(state) = self.awake_state.lock() {
            self.awake_mode = state.mode.clone();
        }
        self.reset_list_state();
    }

    fn reset_list_state(&mut self) {
        let len = self.current_list_len();
        if len == 0 {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn move_cursor(&mut self, delta: i32) {
        let len = self.current_list_len();
        if len == 0 {
            return;
        }
        let cur = self.list_state.selected().unwrap_or(0) as i32;
        let next = (cur + delta).rem_euclid(len as i32) as usize;
        self.list_state.select(Some(next));
    }

    fn current_list_len(&self) -> usize {
        match self.panel() {
            Panel::Apps => self.apps.len(),
            Panel::Hotkeys => self.bindings.len(),
            Panel::Remaps => self.remaps.len(),
            Panel::Snippets => self.snippets.len(),
            Panel::Layouts => self.layouts.len(),
            Panel::Workspaces => self.workspaces.len(),
            Panel::Hosts => self.hosts.len(),
            Panel::Theme | Panel::Awake | Panel::Daemon => 1,
        }
    }

    fn build_list_items(&self) -> Vec<ListItem<'static>> {
        match self.panel() {
            Panel::Apps => self
                .apps
                .iter()
                .map(|app| ListItem::new(trim_text(&app.name, 42)))
                .collect(),
            Panel::Hotkeys => self
                .bindings
                .iter()
                .map(|b| ListItem::new(format!("{} -> {}", b.hotkey, b.action)))
                .collect(),
            Panel::Remaps => self
                .remaps
                .iter()
                .map(|r| ListItem::new(format!("{} -> {}", r.from, r.to)))
                .collect(),
            Panel::Snippets => self
                .snippets
                .iter()
                .map(|s| ListItem::new(format!("{} => {}", s.trigger, trim_text(&s.expand, 32))))
                .collect(),
            Panel::Layouts => self
                .layouts
                .iter()
                .map(|l| ListItem::new(format!("{} ({})", l.name, l.template.layout_type)))
                .collect(),
            Panel::Workspaces => self
                .workspaces
                .iter()
                .map(|w| ListItem::new(format!("{} ({} apps)", w.name, w.apps.len())))
                .collect(),
            Panel::Hosts => self
                .hosts
                .iter()
                .map(|h| ListItem::new(format!("{} {}", h.ip, h.host)))
                .collect(),
            Panel::Theme => vec![ListItem::new(format!(
                "Current theme: {}",
                self.theme_mode.label()
            ))],
            Panel::Awake => vec![ListItem::new(format!(
                "Default display on: {}",
                self.awake_cfg.default_display_on
            ))],
            Panel::Daemon => vec![ListItem::new(format!(
                "quiet={} no_tray={}",
                self.daemon_cfg.quiet, self.daemon_cfg.no_tray
            ))],
        }
    }

    fn detail_text(&self) -> String {
        let idx = self.list_state.selected().unwrap_or(0);
        match self.panel() {
            Panel::Apps => self
                .apps
                .get(idx)
                .map(|app| format!("Name: {}\nPath: {}", app.name, app.path))
                .unwrap_or_else(|| "No apps loaded.".to_string()),
            Panel::Hotkeys => self
                .bindings
                .get(idx)
                .map(|b| {
                    let app = b.app.as_deref().unwrap_or("any");
                    format!("Hotkey: {}\nAction: {}\nApp: {}", b.hotkey, b.action, app)
                })
                .unwrap_or_else(|| "No hotkeys configured.".to_string()),
            Panel::Remaps => self
                .remaps
                .get(idx)
                .map(|r| {
                    let app = r.app.as_deref().unwrap_or("any");
                    let exact = if r.exact { "exact" } else { "partial" };
                    format!(
                        "From: {}\nTo: {}\nApp: {}\nMatch: {}",
                        r.from, r.to, app, exact
                    )
                })
                .unwrap_or_else(|| "No remaps configured.".to_string()),
            Panel::Snippets => self
                .snippets
                .get(idx)
                .map(|s| {
                    let app = s.app.as_deref().unwrap_or("any");
                    let mode = if s.immediate {
                        "immediate"
                    } else {
                        "terminator"
                    };
                    let paste = s.paste.as_deref().unwrap_or("sendinput");
                    format!(
                        "Trigger: {}\nExpand: {}\nApp: {}\nMode: {}\nPaste: {}",
                        s.trigger, s.expand, app, mode, paste
                    )
                })
                .unwrap_or_else(|| "No snippets configured.".to_string()),
            Panel::Layouts => self
                .layouts
                .get(idx)
                .map(|l| {
                    format!(
                        "Name: {}\nType: {}\nRows: {:?}\nCols: {:?}\nGap: {:?}\nBindings: {}",
                        l.name,
                        l.template.layout_type,
                        l.template.rows,
                        l.template.cols,
                        l.template.gap,
                        l.bindings.len()
                    )
                })
                .unwrap_or_else(|| "No layouts configured.".to_string()),
            Panel::Workspaces => self
                .workspaces
                .get(idx)
                .map(|w| {
                    let apps = w
                        .apps
                        .iter()
                        .map(|a| a.path.as_str())
                        .take(6)
                        .collect::<Vec<_>>()
                        .join("\n");
                    format!("Name: {}\nApps: {}\n{}", w.name, w.apps.len(), apps)
                })
                .unwrap_or_else(|| "No workspaces configured.".to_string()),
            Panel::Hosts => self
                .hosts
                .get(idx)
                .map(|h| {
                    let comment = h.comment.as_deref().unwrap_or("-");
                    format!("IP: {}\nHost: {}\nComment: {}", h.ip, h.host, comment)
                })
                .unwrap_or_else(|| "No hosts entries.".to_string()),
            Panel::Theme => format!(
                "Current: {}\nFollow nightlight: {}\nSchedule light: {:?}\nSchedule dark: {:?}",
                self.theme_mode.label(),
                self.theme_cfg.follow_nightlight,
                self.theme_cfg.schedule_light_at,
                self.theme_cfg.schedule_dark_at
            ),
            Panel::Awake => format!(
                "State: {}\nDefault display on: {}",
                self.awake_mode.display_str(),
                self.awake_cfg.default_display_on
            ),
            Panel::Daemon => format!(
                "Daemon mode: foreground\nQuiet: {}\nNo tray: {}",
                self.daemon_cfg.quiet, self.daemon_cfg.no_tray
            ),
        }
    }

    fn selected_index(&self) -> Option<usize> {
        let idx = self.list_state.selected()?;
        if idx < self.current_list_len() {
            Some(idx)
        } else {
            None
        }
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
    }

    fn set_error(&mut self, err: CliError) {
        if err.message.trim().is_empty() {
            self.status = "error".to_string();
        } else {
            self.status = format!("error: {}", err.message);
        }
    }

    fn handle_action(&mut self, code: KeyCode) {
        let result = match (self.panel(), code) {
            (Panel::Theme, KeyCode::Char('l')) => self.action_theme_set(ThemeMode::Light),
            (Panel::Theme, KeyCode::Char('d')) => self.action_theme_set(ThemeMode::Dark),
            (Panel::Theme, KeyCode::Char('f')) => self.action_theme_follow_toggle(),
            (Panel::Awake, KeyCode::Char('o')) => self.action_awake_display_toggle(),
            (Panel::Daemon, KeyCode::Char('m')) => self.action_daemon_quiet_toggle(),
            (Panel::Daemon, KeyCode::Char('n')) => self.action_daemon_no_tray_toggle(),
            (_, KeyCode::Char('a')) => self.action_add(),
            (_, KeyCode::Char('d')) => self.action_remove(),
            (_, KeyCode::Char('c')) => self.action_clear(),
            (_, KeyCode::Enter) => self.action_enter(),
            _ => return,
        };
        if let Err(err) = result {
            self.set_error(err);
        }
    }

    fn action_add(&mut self) -> CliResult {
        match self.panel() {
            Panel::Hotkeys => self.add_hotkey(),
            Panel::Remaps => self.add_remap(),
            Panel::Snippets => self.add_snippet(),
            Panel::Layouts => self.add_layout(),
            Panel::Workspaces => self.save_workspace(),
            Panel::Hosts => self.add_host(),
            _ => {
                self.set_status("action not supported");
                Ok(())
            }
        }
    }

    fn action_remove(&mut self) -> CliResult {
        match self.panel() {
            Panel::Hotkeys => self.remove_hotkey(),
            Panel::Remaps => self.remove_remap(),
            Panel::Snippets => self.remove_snippet(),
            Panel::Layouts => self.remove_layout(),
            Panel::Workspaces => self.remove_workspace(),
            Panel::Hosts => self.remove_host(),
            _ => {
                self.set_status("action not supported");
                Ok(())
            }
        }
    }

    fn action_clear(&mut self) -> CliResult {
        match self.panel() {
            Panel::Remaps => self.clear_remaps(),
            Panel::Snippets => self.clear_snippets(),
            _ => {
                self.set_status("action not supported");
                Ok(())
            }
        }
    }

    fn action_enter(&mut self) -> CliResult {
        match self.panel() {
            Panel::Apps => self.run_selected_app(),
            Panel::Layouts => self.apply_layout(),
            Panel::Workspaces => self.launch_workspace(),
            Panel::Theme => self.action_theme_toggle(),
            Panel::Awake => self.action_awake_toggle(),
            _ => {
                self.set_status("action not supported");
                Ok(())
            }
        }
    }

    fn run_selected_app(&mut self) -> CliResult {
        let idx = self
            .selected_index()
            .ok_or_else(|| CliError::new(2, "No app selected."))?;
        let app = self
            .apps
            .get(idx)
            .ok_or_else(|| CliError::new(2, "App not found."))?;
        Command::new(&app.path)
            .spawn()
            .map_err(|e| CliError::new(2, format!("Failed to launch {}: {e}", app.path)))?;
        self.set_status(format!("launched: {}", app.name));
        Ok(())
    }

    fn add_hotkey(&mut self) -> CliResult {
        let Some(hotkey) = prompt_optional("Hotkey")? else {
            self.set_status("cancelled");
            return Ok(());
        };
        if let Some(reason) = unremappable_reason(&hotkey) {
            return Err(CliError::with_details(
                2,
                format!("Hotkey not allowed: {hotkey}"),
                &[reason],
            ));
        }
        if crate::desktop::hotkey::parse_hotkey(&hotkey).is_none() {
            return Err(CliError::with_details(
                2,
                format!("Invalid hotkey: {hotkey}"),
                &["Fix: use format like ctrl+alt+t."],
            ));
        }
        let Some(action) = prompt_optional("Action")? else {
            self.set_status("cancelled");
            return Ok(());
        };
        let app = normalize_option(prompt_optional("App (optional)")?);

        let mut cfg = crate::config::load_config();
        let mut updated = false;
        for binding in cfg.desktop.bindings.iter_mut() {
            if hotkey_eq(&binding.hotkey, &hotkey) && option_eq(&binding.app, &app) {
                binding.action = action.clone();
                binding.app = app.clone();
                updated = true;
                break;
            }
        }
        if !updated {
            cfg.desktop.bindings.push(crate::config::DesktopBinding {
                hotkey: hotkey.clone(),
                action: action.clone(),
                app: app.clone(),
            });
        }
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();

        let app_label = app.as_deref().unwrap_or("any");
        if updated {
            self.set_status(format!(
                "hotkey updated: {} -> {} [{}]",
                hotkey, action, app_label
            ));
        } else {
            self.set_status(format!(
                "hotkey added: {} -> {} [{}]",
                hotkey, action, app_label
            ));
        }
        Ok(())
    }

    fn remove_hotkey(&mut self) -> CliResult {
        let idx = self
            .selected_index()
            .ok_or_else(|| CliError::new(2, "No hotkey selected."))?;
        let binding = self
            .bindings
            .get(idx)
            .ok_or_else(|| CliError::new(2, "Hotkey not found."))?
            .clone();
        if !prompt_yes_no(&format!("Remove hotkey {}?", binding.hotkey), false)? {
            self.set_status("cancelled");
            return Ok(());
        }

        let mut cfg = crate::config::load_config();
        let before = cfg.desktop.bindings.len();
        cfg.desktop.bindings.retain(|b| {
            !(hotkey_eq(&b.hotkey, &binding.hotkey)
                && option_eq(&b.app, &binding.app)
                && b.action == binding.action)
        });
        let removed = before - cfg.desktop.bindings.len();
        if removed == 0 {
            return Err(CliError::new(2, "Hotkey not found."));
        }
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();
        self.set_status(format!(
            "hotkey removed: {} ({} binding(s))",
            binding.hotkey, removed
        ));
        Ok(())
    }

    fn add_remap(&mut self) -> CliResult {
        let Some(from) = prompt_optional("From hotkey")? else {
            self.set_status("cancelled");
            return Ok(());
        };
        if let Some(reason) = unremappable_reason(&from) {
            return Err(CliError::with_details(
                2,
                format!("Remap source not allowed: {from}"),
                &[reason],
            ));
        }
        if crate::desktop::hotkey::parse_hotkey(&from).is_none() {
            return Err(CliError::with_details(
                2,
                format!("Invalid remap source: {from}"),
                &["Fix: use format like ctrl+alt+1."],
            ));
        }

        let Some(to) = prompt_optional("To target (hotkey/disable/text:...)")? else {
            self.set_status("cancelled");
            return Ok(());
        };
        if let Some(text) = to.strip_prefix("text:") {
            if text.trim().is_empty() {
                return Err(CliError::new(2, "Remap text is empty."));
            }
        } else if !to.eq_ignore_ascii_case("disable")
            && crate::desktop::hotkey::parse_hotkey(&to).is_none()
        {
            return Err(CliError::with_details(
                2,
                format!("Invalid remap target: {to}"),
                &["Fix: use a hotkey, disable, or text:<value>."],
            ));
        }

        let app = normalize_option(prompt_optional("App (optional)")?);
        let exact = prompt_yes_no("Match exact app name?", false)?;

        let mut cfg = crate::config::load_config();
        let mut updated = false;
        for rule in cfg.desktop.remaps.iter_mut() {
            if hotkey_eq(&rule.from, &from) && option_eq(&rule.app, &app) {
                rule.to = to.clone();
                rule.app = app.clone();
                rule.exact = exact;
                updated = true;
                break;
            }
        }
        if !updated {
            cfg.desktop.remaps.push(crate::config::DesktopRemap {
                from: from.clone(),
                to: to.clone(),
                app: app.clone(),
                exact,
            });
        }
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();

        let app_label = app.as_deref().unwrap_or("any");
        let suffix = if exact { "exact" } else { "partial" };
        if updated {
            self.set_status(format!(
                "remap updated: {} -> {} [{} | {}]",
                from, to, app_label, suffix
            ));
        } else {
            self.set_status(format!(
                "remap added: {} -> {} [{} | {}]",
                from, to, app_label, suffix
            ));
        }
        Ok(())
    }

    fn remove_remap(&mut self) -> CliResult {
        let idx = self
            .selected_index()
            .ok_or_else(|| CliError::new(2, "No remap selected."))?;
        let rule = self
            .remaps
            .get(idx)
            .ok_or_else(|| CliError::new(2, "Remap not found."))?
            .clone();
        if !prompt_yes_no(&format!("Remove remap {}?", rule.from), false)? {
            self.set_status("cancelled");
            return Ok(());
        }

        let mut cfg = crate::config::load_config();
        let before = cfg.desktop.remaps.len();
        cfg.desktop.remaps.retain(|r| {
            !(hotkey_eq(&r.from, &rule.from)
                && hotkey_eq(&r.to, &rule.to)
                && option_eq(&r.app, &rule.app)
                && r.exact == rule.exact)
        });
        let removed = before - cfg.desktop.remaps.len();
        if removed == 0 {
            return Err(CliError::new(2, "Remap not found."));
        }
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();
        self.set_status(format!("remap removed: {} ({})", rule.from, removed));
        Ok(())
    }

    fn clear_remaps(&mut self) -> CliResult {
        let mut cfg = crate::config::load_config();
        if cfg.desktop.remaps.is_empty() {
            self.set_status("no remap rules");
            return Ok(());
        }
        if !prompt_yes_no("Clear all remap rules?", false)? {
            self.set_status("cancelled");
            return Ok(());
        }
        let count = cfg.desktop.remaps.len();
        cfg.desktop.remaps.clear();
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();
        self.set_status(format!("remap rules cleared: {}", count));
        Ok(())
    }

    fn add_snippet(&mut self) -> CliResult {
        let Some(trigger) = prompt_optional("Trigger")? else {
            self.set_status("cancelled");
            return Ok(());
        };
        let Some(expand) = prompt_optional("Expand")? else {
            self.set_status("cancelled");
            return Ok(());
        };
        let app = normalize_option(prompt_optional("App (optional)")?);
        let immediate = prompt_yes_no("Trigger immediately?", false)?;
        let clipboard = prompt_yes_no("Paste via clipboard?", false)?;
        let paste = if clipboard {
            Some("clipboard".to_string())
        } else {
            None
        };

        let mut cfg = crate::config::load_config();
        let mut updated = false;
        for snippet in cfg.desktop.snippets.iter_mut() {
            if trigger_eq(&snippet.trigger, &trigger) && option_eq(&snippet.app, &app) {
                snippet.expand = expand.clone();
                snippet.immediate = immediate;
                snippet.app = app.clone();
                snippet.paste = paste.clone();
                updated = true;
                break;
            }
        }
        if !updated {
            cfg.desktop.snippets.push(crate::config::DesktopSnippet {
                trigger: trigger.clone(),
                expand: expand.clone(),
                app: app.clone(),
                immediate,
                paste: paste.clone(),
            });
        }
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();

        let app_label = app.as_deref().unwrap_or("any");
        let mode = if immediate { "immediate" } else { "terminator" };
        let paste_label = paste.as_deref().unwrap_or("sendinput");
        if updated {
            self.set_status(format!(
                "snippet updated: {} [{} | {} | {}]",
                trigger, app_label, mode, paste_label
            ));
        } else {
            self.set_status(format!(
                "snippet added: {} [{} | {} | {}]",
                trigger, app_label, mode, paste_label
            ));
        }
        Ok(())
    }

    fn remove_snippet(&mut self) -> CliResult {
        let idx = self
            .selected_index()
            .ok_or_else(|| CliError::new(2, "No snippet selected."))?;
        let snippet = self
            .snippets
            .get(idx)
            .ok_or_else(|| CliError::new(2, "Snippet not found."))?
            .clone();
        if !prompt_yes_no(&format!("Remove snippet {}?", snippet.trigger), false)? {
            self.set_status("cancelled");
            return Ok(());
        }

        let mut cfg = crate::config::load_config();
        let before = cfg.desktop.snippets.len();
        cfg.desktop
            .snippets
            .retain(|s| !trigger_eq(&s.trigger, &snippet.trigger));
        let removed = before - cfg.desktop.snippets.len();
        if removed == 0 {
            return Err(CliError::new(2, "Snippet not found."));
        }
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();
        self.set_status(format!(
            "snippet removed: {} ({} item(s))",
            snippet.trigger, removed
        ));
        Ok(())
    }

    fn clear_snippets(&mut self) -> CliResult {
        let mut cfg = crate::config::load_config();
        if cfg.desktop.snippets.is_empty() {
            self.set_status("no snippets configured");
            return Ok(());
        }
        if !prompt_yes_no("Clear all snippets?", false)? {
            self.set_status("cancelled");
            return Ok(());
        }
        let count = cfg.desktop.snippets.len();
        cfg.desktop.snippets.clear();
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();
        self.set_status(format!("snippets cleared: {}", count));
        Ok(())
    }

    fn add_layout(&mut self) -> CliResult {
        let Some(name) = prompt_optional("Layout name")? else {
            self.set_status("cancelled");
            return Ok(());
        };
        let rows = prompt_u32("Rows", 2)?;
        let cols = prompt_u32("Cols", 2)?;
        let gap = prompt_u32("Gap", 8)?;

        let mut cfg = crate::config::load_config();
        if let Some(existing) = cfg.desktop.layouts.iter_mut().find(|l| l.name == name) {
            existing.template.layout_type = "grid".to_string();
            existing.template.rows = Some(rows);
            existing.template.cols = Some(cols);
            existing.template.gap = Some(gap);
        } else {
            cfg.desktop.layouts.push(crate::config::DesktopLayout {
                name: name.clone(),
                template: crate::config::DesktopLayoutTemplate {
                    layout_type: "grid".to_string(),
                    rows: Some(rows),
                    cols: Some(cols),
                    gap: Some(gap),
                },
                bindings: std::collections::BTreeMap::new(),
            });
        }
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();
        self.set_status(format!(
            "layout saved: {} (grid {}x{} gap={})",
            name, rows, cols, gap
        ));
        Ok(())
    }

    fn remove_layout(&mut self) -> CliResult {
        let idx = self
            .selected_index()
            .ok_or_else(|| CliError::new(2, "No layout selected."))?;
        let layout = self
            .layouts
            .get(idx)
            .ok_or_else(|| CliError::new(2, "Layout not found."))?
            .clone();
        if !prompt_yes_no(&format!("Remove layout {}?", layout.name), false)? {
            self.set_status("cancelled");
            return Ok(());
        }

        let mut cfg = crate::config::load_config();
        let before = cfg.desktop.layouts.len();
        cfg.desktop.layouts.retain(|l| l.name != layout.name);
        if cfg.desktop.layouts.len() == before {
            return Err(CliError::new(2, "Layout not found."));
        }
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();
        self.set_status(format!("layout removed: {}", layout.name));
        Ok(())
    }

    fn apply_layout(&mut self) -> CliResult {
        let idx = self
            .selected_index()
            .ok_or_else(|| CliError::new(2, "No layout selected."))?;
        let name = self
            .layouts
            .get(idx)
            .ok_or_else(|| CliError::new(2, "Layout not found."))?
            .name
            .clone();
        let move_existing = prompt_yes_no("Move existing windows?", false)?;
        let (moved, skipped) = apply_layout_by_name(&name, move_existing)?;
        if skipped > 0 {
            self.set_status(format!(
                "layout applied: {} (moved {}, skipped {})",
                name, moved, skipped
            ));
        } else {
            self.set_status(format!("layout applied: {} (moved {})", name, moved));
        }
        Ok(())
    }

    fn save_workspace(&mut self) -> CliResult {
        let Some(name) = prompt_optional("Workspace name")? else {
            self.set_status("cancelled");
            return Ok(());
        };
        let capture = prompt_yes_no("Capture current apps?", true)?;

        let mut apps: Vec<crate::config::DesktopWorkspaceApp> = Vec::new();
        if capture {
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            for proc in crate::proc::list_all(true)
                .into_iter()
                .filter(|p| !p.window_title.trim().is_empty())
            {
                let exe = if proc.exe_path.is_empty() {
                    proc.name
                } else {
                    proc.exe_path
                };
                let key = exe.to_lowercase();
                if !seen.insert(key) {
                    continue;
                }
                let rect = window_api::find_hwnd_by_pid(proc.pid)
                    .ok()
                    .and_then(|hwnd| window_api::get_window_rect(hwnd).ok())
                    .map(|rect| [rect.left, rect.top, rect.right, rect.bottom]);
                apps.push(crate::config::DesktopWorkspaceApp {
                    path: exe,
                    args: None,
                    rect,
                });
            }
        }

        let mut cfg = crate::config::load_config();
        if let Some(existing) = cfg.desktop.workspaces.iter_mut().find(|w| w.name == name) {
            existing.apps = apps;
        } else {
            cfg.desktop
                .workspaces
                .push(crate::config::DesktopWorkspace {
                    name: name.clone(),
                    apps,
                });
        }
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();
        self.set_status(format!("workspace saved: {}", name));
        Ok(())
    }

    fn launch_workspace(&mut self) -> CliResult {
        let idx = self
            .selected_index()
            .ok_or_else(|| CliError::new(2, "No workspace selected."))?;
        let name = self
            .workspaces
            .get(idx)
            .ok_or_else(|| CliError::new(2, "Workspace not found."))?
            .name
            .clone();
        let move_existing = prompt_yes_no("Move existing windows?", false)?;
        let offset = prompt_i32("Monitor offset", 0)?;
        let (moved, launched) = launch_workspace_by_name(&name, move_existing, offset)?;
        let mut msg = format!(
            "workspace launched: {} (moved {}, launched {})",
            name, moved, launched
        );
        if move_existing {
            msg.push_str(" [move existing]");
        }
        if offset != 0 {
            msg.push_str(&format!(" [offset {}]", offset));
        }
        self.set_status(msg);
        Ok(())
    }

    fn remove_workspace(&mut self) -> CliResult {
        let idx = self
            .selected_index()
            .ok_or_else(|| CliError::new(2, "No workspace selected."))?;
        let workspace = self
            .workspaces
            .get(idx)
            .ok_or_else(|| CliError::new(2, "Workspace not found."))?
            .clone();
        if !prompt_yes_no(&format!("Remove workspace {}?", workspace.name), false)? {
            self.set_status("cancelled");
            return Ok(());
        }

        let mut cfg = crate::config::load_config();
        let before = cfg.desktop.workspaces.len();
        cfg.desktop.workspaces.retain(|w| w.name != workspace.name);
        if cfg.desktop.workspaces.len() == before {
            return Err(CliError::new(2, "Workspace not found."));
        }
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();
        self.set_status(format!("workspace removed: {}", workspace.name));
        Ok(())
    }

    fn add_host(&mut self) -> CliResult {
        if !prompt_yes_no("Edit hosts file?", false)? {
            self.set_status("cancelled");
            return Ok(());
        }
        let Some(ip) = prompt_optional("IP")? else {
            self.set_status("cancelled");
            return Ok(());
        };
        let Some(host) = prompt_optional("Host")? else {
            self.set_status("cancelled");
            return Ok(());
        };
        crate::desktop::hosts::add_entry(&ip, &host)?;
        self.refresh();
        self.set_status(format!("hosts added: {} {}", ip, host));
        Ok(())
    }

    fn remove_host(&mut self) -> CliResult {
        let idx = self
            .selected_index()
            .ok_or_else(|| CliError::new(2, "No host selected."))?;
        let entry = self
            .hosts
            .get(idx)
            .ok_or_else(|| CliError::new(2, "Host not found."))?
            .clone();
        if !prompt_yes_no(&format!("Remove host {}?", entry.host), false)? {
            self.set_status("cancelled");
            return Ok(());
        }
        let removed = crate::desktop::hosts::remove_entry(&entry.host)?;
        if !removed {
            return Err(CliError::new(2, "Host not found."));
        }
        self.refresh();
        self.set_status(format!("hosts removed: {}", entry.host));
        Ok(())
    }

    fn action_theme_toggle(&mut self) -> CliResult {
        let mode = crate::desktop::theme::toggle_theme()?;
        self.refresh();
        self.set_status(format!("theme switched: {}", mode.label()));
        Ok(())
    }

    fn action_theme_set(&mut self, mode: ThemeMode) -> CliResult {
        crate::desktop::theme::set_theme(&mode)?;
        self.refresh();
        self.set_status(format!("theme set: {}", mode.label()));
        Ok(())
    }

    fn action_theme_follow_toggle(&mut self) -> CliResult {
        let mut cfg = crate::config::load_config();
        cfg.desktop.theme.follow_nightlight = !cfg.desktop.theme.follow_nightlight;
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();
        self.set_status(format!(
            "follow nightlight: {}",
            cfg.desktop.theme.follow_nightlight
        ));
        Ok(())
    }

    fn action_awake_toggle(&mut self) -> CliResult {
        match self.awake_mode {
            AwakeMode::Off => {
                crate::desktop::awake::awake_indefinite(
                    self.awake_cfg.default_display_on,
                    &self.awake_state,
                )?;
                self.refresh();
                self.set_status("awake on");
            }
            _ => {
                crate::desktop::awake::cancel_awake(&self.awake_state);
                self.refresh();
                self.set_status("awake off");
            }
        }
        Ok(())
    }

    fn action_awake_display_toggle(&mut self) -> CliResult {
        let mut cfg = crate::config::load_config();
        cfg.desktop.awake.default_display_on = !cfg.desktop.awake.default_display_on;
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();
        self.set_status(format!(
            "default display on: {}",
            cfg.desktop.awake.default_display_on
        ));
        Ok(())
    }

    fn action_daemon_quiet_toggle(&mut self) -> CliResult {
        let mut cfg = crate::config::load_config();
        cfg.desktop.daemon.quiet = !cfg.desktop.daemon.quiet;
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();
        self.set_status(format!("daemon quiet: {}", cfg.desktop.daemon.quiet));
        Ok(())
    }

    fn action_daemon_no_tray_toggle(&mut self) -> CliResult {
        let mut cfg = crate::config::load_config();
        cfg.desktop.daemon.no_tray = !cfg.desktop.daemon.no_tray;
        crate::config::save_config(&cfg)
            .map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        self.refresh();
        self.set_status(format!("daemon no_tray: {}", cfg.desktop.daemon.no_tray));
        Ok(())
    }
}

fn draw_ui(f: &mut ratatui::Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(f.area());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " xun desktop tui ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("| "),
        Span::styled(app.panel().label(), Style::default().fg(Color::Cyan)),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, layout[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(1)])
        .split(layout[1]);

    let menu_items: Vec<ListItem> = app
        .panels
        .iter()
        .map(|p| ListItem::new(p.label()))
        .collect();
    let menu = List::new(menu_items)
        .block(Block::default().borders(Borders::ALL).title("Panels"))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(menu, body[0], &mut app.menu_state);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(body[1]);

    let items = app.build_list_items();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Items"))
        .highlight_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(list, right[0], &mut app.list_state);

    let detail = Paragraph::new(app.detail_text())
        .block(Block::default().borders(Borders::ALL).title("Detail"));
    f.render_widget(detail, right[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::raw(" q/esc: quit  tab: next  ↑/↓: move  r: refresh  "),
        Span::raw("| "),
        Span::styled(app.panel().hint(), Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled(&app.status, Style::default().fg(Color::Magenta)),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, layout[2]);
}

fn trim_text(input: &str, max: usize) -> String {
    if input.chars().count() <= max {
        return input.to_string();
    }
    let mut out = String::new();
    for ch in input.chars().take(max.saturating_sub(1)) {
        out.push(ch);
    }
    out.push('…');
    out
}

fn prompt_interactive<T, F>(f: F) -> CliResult<T>
where
    F: FnOnce() -> Result<T, dialoguer::Error>,
{
    disable_raw_mode().map_err(|e| CliError::new(1, format!("{e}")))?;
    execute!(io::stdout(), LeaveAlternateScreen).map_err(|e| CliError::new(1, format!("{e}")))?;
    let result = f().map_err(|e| CliError::new(1, format!("prompt failed: {e}")));
    execute!(io::stdout(), EnterAlternateScreen).map_err(|e| CliError::new(1, format!("{e}")))?;
    enable_raw_mode().map_err(|e| CliError::new(1, format!("{e}")))?;
    result
}

fn prompt_optional(prompt: &str) -> CliResult<Option<String>> {
    prompt_interactive(|| {
        let value: String = Input::new().with_prompt(prompt).interact_text()?;
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed))
        }
    })
}

fn prompt_yes_no(prompt: &str, default: bool) -> CliResult<bool> {
    prompt_interactive(|| {
        Confirm::new()
            .with_prompt(prompt)
            .default(default)
            .interact()
    })
}

fn prompt_u32(prompt: &str, default: u32) -> CliResult<u32> {
    let raw = prompt_interactive(|| {
        Input::new()
            .with_prompt(prompt)
            .default(default.to_string())
            .interact_text()
    })?;
    raw.trim()
        .parse::<u32>()
        .map_err(|_| CliError::new(2, format!("Invalid number: {raw}")))
}

fn prompt_i32(prompt: &str, default: i32) -> CliResult<i32> {
    let raw = prompt_interactive(|| {
        Input::new()
            .with_prompt(prompt)
            .default(default.to_string())
            .interact_text()
    })?;
    raw.trim()
        .parse::<i32>()
        .map_err(|_| CliError::new(2, format!("Invalid number: {raw}")))
}

fn normalize_hotkey(raw: &str) -> String {
    raw.to_lowercase().replace(' ', "")
}

fn hotkey_eq(a: &str, b: &str) -> bool {
    normalize_hotkey(a) == normalize_hotkey(b)
}

fn trigger_eq(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

fn normalize_option(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn option_eq(left: &Option<String>, right: &Option<String>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        _ => false,
    }
}

fn unremappable_reason(hotkey: &str) -> Option<&'static str> {
    let normalized = normalize_hotkey(hotkey);
    for (key, reason) in crate::desktop::hotkey::UNREMAPPABLE_KEYS {
        if normalized == *key {
            return Some(*reason);
        }
    }
    None
}

fn build_grid_layout(
    template: &crate::config::DesktopLayoutTemplate,
) -> crate::desktop::layout::GridLayout {
    crate::desktop::layout::GridLayout {
        rows: template.rows.unwrap_or(2).max(1) as usize,
        cols: template.cols.unwrap_or(2).max(1) as usize,
        gap: template.gap.unwrap_or(8) as i32,
        padding: 0,
        monitor: 0,
    }
}

fn map_window_api_error(action: &str, err: window_api::WindowApiError) -> CliError {
    match err {
        window_api::WindowApiError::NotFound => CliError::new(2, "No matching window found."),
        window_api::WindowApiError::OsError { action: op, code } => CliError::with_details(
            2,
            format!("Failed to {action} window."),
            &[
                format!("Win32: {op} (code={code})"),
                "Fix: retry with admin rights.".to_string(),
            ],
        ),
    }
}

fn apply_layout_by_name(name: &str, move_existing: bool) -> CliResult<(usize, usize)> {
    use std::collections::HashSet;

    let cfg = crate::config::load_config();
    let layout = cfg
        .desktop
        .layouts
        .iter()
        .find(|l| l.name == name)
        .ok_or_else(|| CliError::new(2, format!("Layout not found: {name}")))?;

    if layout.template.layout_type.to_lowercase() != "grid" {
        return Err(CliError::with_details(
            2,
            format!("Unsupported layout type: {}", layout.template.layout_type),
            &["Fix: use grid in Phase 1."],
        ));
    }

    let grid = build_grid_layout(&layout.template);
    let template = crate::desktop::layout::LayoutTemplate::Grid(grid);
    let monitor = crate::desktop::layout::get_monitor_area(0)
        .ok_or_else(|| CliError::new(2, "Monitor 0 not found."))?;
    let zones = crate::desktop::layout::compute_zones(&template, &monitor);
    if zones.is_empty() {
        return Err(CliError::new(2, "No zones computed."));
    }

    let mut assignments: Vec<(u32, crate::desktop::layout::ZoneRect)> = Vec::new();
    let mut used_pids: HashSet<u32> = HashSet::new();
    let mut used_zones: HashSet<usize> = HashSet::new();
    let mut skipped = 0usize;

    if !layout.bindings.is_empty() {
        for (app, zone_index) in &layout.bindings {
            if *zone_index >= zones.len() {
                return Err(CliError::new(
                    2,
                    format!("Zone index out of range: {zone_index}"),
                ));
            }
            if used_zones.contains(zone_index) {
                skipped += 1;
                continue;
            }
            if let Some(proc) = crate::proc::find_by_name(app)
                .into_iter()
                .find(|p| !p.window_title.trim().is_empty())
            {
                if used_pids.insert(proc.pid) {
                    used_zones.insert(*zone_index);
                    assignments.push((proc.pid, zones[*zone_index].clone()));
                }
            } else {
                skipped += 1;
            }
        }

        if move_existing {
            let mut remaining: Vec<usize> = (0..zones.len())
                .filter(|i| !used_zones.contains(i))
                .collect();
            let windows = crate::proc::list_all(false);
            for proc in windows
                .into_iter()
                .filter(|p| !p.window_title.trim().is_empty())
            {
                if remaining.is_empty() {
                    break;
                }
                if used_pids.contains(&proc.pid) {
                    continue;
                }
                let zone_index = remaining.remove(0);
                used_pids.insert(proc.pid);
                assignments.push((proc.pid, zones[zone_index].clone()));
            }
        }
    } else {
        if !move_existing {
            return Err(CliError::with_details(
                2,
                "Layout has no bindings.".to_string(),
                &["Fix: add bindings or use move existing."],
            ));
        }
        let windows: Vec<_> = crate::proc::list_all(false)
            .into_iter()
            .filter(|p| !p.window_title.trim().is_empty())
            .collect();
        for (zone, proc) in zones.iter().zip(windows.iter()) {
            assignments.push((proc.pid, zone.clone()));
        }
    }

    if assignments.is_empty() {
        return Err(CliError::new(2, "No windows matched for layout."));
    }

    let mut moved = 0usize;
    for (pid, zone) in assignments {
        let hwnd =
            window_api::find_hwnd_by_pid(pid).map_err(|e| map_window_api_error("apply", e))?;
        let rect = window_api::WindowRect {
            left: zone.x,
            top: zone.y,
            right: zone.x + zone.w,
            bottom: zone.y + zone.h,
        };
        window_api::apply_window_rect(hwnd, rect).map_err(|e| map_window_api_error("apply", e))?;
        moved += 1;
    }

    Ok((moved, skipped))
}

fn launch_workspace_by_name(
    name: &str,
    move_existing: bool,
    offset: i32,
) -> CliResult<(usize, usize)> {
    use std::collections::HashSet;

    let cfg = crate::config::load_config();
    let workspace = cfg
        .desktop
        .workspaces
        .iter()
        .find(|w| w.name == name)
        .ok_or_else(|| CliError::new(2, format!("Workspace not found: {name}")))?;

    if workspace.apps.is_empty() {
        return Err(CliError::new(2, "Workspace has no apps to launch."));
    }

    let mut moved = 0usize;
    let mut launched = 0usize;
    let mut seen: HashSet<String> = HashSet::new();

    for app in &workspace.apps {
        if !seen.insert(app.path.to_lowercase()) {
            continue;
        }

        let existing = crate::proc::find_by_name(&app.path)
            .into_iter()
            .find(|p| !p.window_title.trim().is_empty());

        if move_existing {
            if let Some(proc) = existing {
                if let Ok(hwnd) = window_api::find_hwnd_by_pid(proc.pid) {
                    if let Some(rect) = app.rect {
                        let rect = window_api::WindowRect {
                            left: rect[0] + offset,
                            top: rect[1],
                            right: rect[2] + offset,
                            bottom: rect[3],
                        };
                        window_api::apply_window_rect(hwnd, rect)
                            .map_err(|e| map_window_api_error("apply", e))?;
                    }
                    moved += 1;
                    continue;
                }
            }
        }

        let mut cmd = Command::new(&app.path);
        if let Some(args_raw) = &app.args {
            cmd.args(args_raw.split_whitespace());
        }
        cmd.spawn()
            .map_err(|e| CliError::new(2, format!("Failed to launch {}: {e}", app.path)))?;
        launched += 1;
    }

    Ok((moved, launched))
}
