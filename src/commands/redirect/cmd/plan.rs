use std::path::{Path, PathBuf};

use crate::output::{CliError, CliResult};
use crate::store::now_secs;

use super::super::engine;
use super::super::plan::{self, PLAN_VERSION, PlanFile};
use super::super::render::render_preview_summary;

pub(super) fn run_plan(
    plan_path_raw: &str,
    source: &Path,
    profile_name: &str,
    profile: &crate::config::RedirectProfile,
    copy: bool,
) -> CliResult {
    let planned = engine::plan_redirect(source, profile, copy);
    let plan_path = PathBuf::from(plan_path_raw);
    if let Some(parent) = plan_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = PlanFile {
        version: PLAN_VERSION,
        created_ts: now_secs(),
        source: plan::path_to_string(source),
        profile: profile_name.to_string(),
        items: planned.items,
    };
    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| CliError::new(1, format!("Failed to serialize plan: {e}")))?;
    std::fs::write(&plan_path, json)
        .map_err(|e| CliError::new(1, format!("Failed to write plan file: {e}")))?;
    ui_println!("Plan written: {}", plan_path.display());
    render_preview_summary(&planned.results, copy);
    Ok(())
}
