use std::env;
use std::io;
use std::io::IsTerminal;

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{ContentArrangement, Table};

use crate::runtime;
use crate::store::now_secs;

#[derive(Debug, Clone)]
pub struct CliError {
    pub(crate) code: i32,
    pub(crate) message: String,
    pub(crate) details: Vec<String>,
}

pub type CliResult<T = ()> = Result<T, CliError>;

impl CliError {
    pub(crate) fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: Vec::new(),
        }
    }

    pub(crate) fn with_details(
        code: i32,
        message: impl Into<String>,
        details: &[impl AsRef<str>],
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details: details.iter().map(|s| s.as_ref().to_string()).collect(),
        }
    }
}

pub(crate) fn print_cli_error(err: &CliError) {
    if err.message.trim().is_empty() && err.details.is_empty() {
        return;
    }
    ui_println(format_args!("Error: {}", err.message));
    if err.details.is_empty() {
        ui_println(format_args!("Hint: Run `xun --help` for usage."));
    } else {
        for d in &err.details {
            ui_println(format_args!("{d}"));
        }
    }
}

pub(crate) fn emit_warning(message: impl AsRef<str>, details: &[&str]) {
    let msg = message.as_ref();
    ui_println(format_args!("Warning: {}", msg));
    for d in details {
        ui_println(format_args!("{d}"));
    }
}

pub(crate) fn can_interact() -> bool {
    can_interact_with(
        runtime::is_non_interactive(),
        io::stdin().is_terminal(),
        io::stderr().is_terminal(),
    )
}

fn can_interact_with(non_interactive: bool, stdin_is_tty: bool, stderr_is_tty: bool) -> bool {
    !non_interactive && stdin_is_tty && stderr_is_tty
}

fn force_ui_value(raw: Option<&str>) -> bool {
    matches!(raw, Some("1") | Some("true") | Some("yes"))
}

fn force_ui() -> bool {
    force_ui_value(env::var("XUN_UI").ok().as_deref())
}

pub(crate) fn prefer_table_output() -> bool {
    prefer_table_output_with(force_ui(), io::stdout().is_terminal())
}

fn prefer_table_output_with(force_ui: bool, stdout_is_tty: bool) -> bool {
    force_ui || stdout_is_tty
}

pub(crate) fn apply_pretty_table_style(table: &mut Table) {
    table.load_preset(UTF8_FULL);
    table.apply_modifier(UTF8_ROUND_CORNERS);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.use_stderr();
    if !runtime::is_no_color() {
        table.enforce_styling();
    }
}

pub(crate) fn ui_println(args: std::fmt::Arguments) {
    let msg = args.to_string();
    if runtime::is_quiet() && !looks_like_error(&msg) {
        return;
    }
    eprintln!("{msg}");
}

pub(crate) fn print_table(table: &Table) {
    if runtime::is_quiet() {
        return;
    }
    if runtime::is_no_color() {
        let s = table.to_string();
        eprintln!("{}", console::strip_ansi_codes(&s));
    } else {
        eprintln!("{table}");
    }
}

fn looks_like_error(msg: &str) -> bool {
    let s = msg.trim_start();
    let sl = s.as_bytes();
    sl.starts_with(b"error")
        || sl.starts_with(b"Error")
        || sl.starts_with(b"hint")
        || sl.starts_with(b"Hint")
        || sl.starts_with(b"fix")
        || sl.starts_with(b"Fix")
        || sl.starts_with(b"invalid")
        || sl.starts_with(b"Invalid")
        || sl.starts_with(b"fail")
        || sl.starts_with(b"Fail")
        || sl.starts_with(b"warning")
        || sl.starts_with(b"Warning")
        || sl.starts_with(b"Did you mean")
        || s.contains("not found")
        || sl.starts_with(b"usage")
        || sl.starts_with(b"Usage")
}

pub(crate) fn format_age(ts: u64) -> String {
    if ts == 0 {
        return "never".to_string();
    }
    let now = now_secs();
    let diff = now.saturating_sub(ts);
    if diff < 60 {
        format!("{}s", diff)
    } else if diff < 3_600 {
        format!("{}m", diff / 60)
    } else if diff < 86_400 {
        format!("{}h", diff / 3_600)
    } else {
        format!("{}d", diff / 86_400)
    }
}

#[cfg(feature = "crypt")]
use indicatif::{ProgressBar, ProgressStyle};

#[cfg(feature = "crypt")]
pub(crate) struct ProgressReporter {
    pb: Option<ProgressBar>,
}

#[cfg(feature = "crypt")]
impl ProgressReporter {
    pub(crate) fn new(total: u64, msg: &str) -> Self {
        if total >= 10 && can_interact() {
            let pb = ProgressBar::new(total);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{bar:30}] {pos}/{len} {msg}")
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("=>-"),
            );
            pb.set_message(msg.to_string());
            Self { pb: Some(pb) }
        } else {
            Self { pb: None }
        }
    }

    pub(crate) fn inc(&self, delta: u64) {
        if let Some(pb) = &self.pb {
            pb.inc(delta);
        }
    }

    pub(crate) fn finish_with_message(&self, msg: &str) {
        if let Some(pb) = &self.pb {
            pb.finish_with_message(msg.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_age_zero_is_never() {
        assert_eq!(format_age(0), "never");
    }

    #[test]
    fn format_age_formats_seconds_minutes_hours_days() {
        let now = now_secs();
        assert_eq!(format_age(now.saturating_sub(5)), "5s");
        assert_eq!(format_age(now.saturating_sub(90)), "1m");
        assert_eq!(format_age(now.saturating_sub(2 * 3600)), "2h");
        assert_eq!(format_age(now.saturating_sub(2 * 86400)), "2d");
    }

    #[test]
    fn prefer_table_output_can_be_forced() {
        assert!(force_ui_value(Some("1")));
        assert!(force_ui_value(Some("true")));
        assert!(force_ui_value(Some("yes")));
        assert!(!force_ui_value(Some("0")));

        assert!(prefer_table_output_with(true, false));
        assert!(prefer_table_output_with(false, true));
        assert!(!prefer_table_output_with(false, false));
    }

    #[test]
    fn can_interact_is_false_when_non_interactive() {
        assert!(!can_interact_with(true, true, true));
    }
}
