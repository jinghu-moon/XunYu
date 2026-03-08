use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use super::matcher;
use crate::config::{RedirectProfile, RedirectUnmatched};

#[derive(Debug, Clone)]
pub(crate) struct ExplainRuleLine {
    pub(crate) rule_name: String,
    pub(crate) ok: bool,
    pub(crate) details: String,
    pub(crate) rendered_dest_dir: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ExplainOutcome {
    pub(crate) lines: Vec<ExplainRuleLine>,
    pub(crate) matched_rule: Option<String>,
    pub(crate) rendered_dest_file: Option<String>,
    pub(crate) note: Option<String>,
}

fn file_name_only(raw: &str) -> String {
    Path::new(raw)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(raw)
        .to_string()
}

fn render_dest_template_preview(dest_raw: &str, file_name: &str) -> String {
    let file_name = file_name_only(file_name);
    let stem = Path::new(&file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = Path::new(&file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let mut out = dest_raw.to_string();
    out = out.replace("{name}", stem);
    out = out.replace("{ext}", ext);
    out
}

fn resolve_dest_dir_preview(source: &Path, dest_raw: &str, file_name: &str) -> PathBuf {
    let rendered = render_dest_template_preview(dest_raw, file_name);
    let p = PathBuf::from(rendered);
    if p.is_absolute() { p } else { source.join(p) }
}

pub(crate) fn explain_one(profile: &RedirectProfile, file_name: &str) -> ExplainOutcome {
    let file_name = file_name_only(file_name);
    let mut lines = Vec::new();

    let mut matched_rule: Option<String> = None;
    let mut rendered_dest_dir: Option<String> = None;

    for r in &profile.rules {
        let detail = matcher::explain_rule_pure(&file_name, r);
        let dest_dir = resolve_dest_dir_preview(Path::new("."), &r.dest, &file_name);
        lines.push(ExplainRuleLine {
            rule_name: r.name.clone(),
            ok: detail.matched,
            details: detail.summary,
            rendered_dest_dir: dest_dir.to_string_lossy().to_string(),
        });
        if matched_rule.is_none() && detail.matched {
            matched_rule = Some(r.name.clone());
            rendered_dest_dir = Some(dest_dir.to_string_lossy().to_string());
        }
    }

    let mut note = None;
    if matched_rule.is_none() {
        match &profile.unmatched {
            RedirectUnmatched::Skip => {
                note = Some("unmatched=skip".to_string());
            }
            RedirectUnmatched::Archive { dest, .. } => {
                let dest_dir = resolve_dest_dir_preview(Path::new("."), dest, &file_name);
                matched_rule = Some("(unmatched)".to_string());
                rendered_dest_dir = Some(dest_dir.to_string_lossy().to_string());
            }
        }
    }

    let rendered_dest_file = rendered_dest_dir
        .as_deref()
        .map(|d| Path::new(d).join(&file_name).to_string_lossy().to_string());

    ExplainOutcome {
        lines,
        matched_rule,
        rendered_dest_file,
        note,
    }
}

pub(crate) fn read_simulate_input_lines() -> io::Result<Vec<String>> {
    let stdin = io::stdin();
    let mut out = Vec::new();
    for line in stdin.lock().lines() {
        let line = line?;
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        out.push(s.to_string());
    }
    Ok(out)
}
