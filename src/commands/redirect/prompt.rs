use std::path::Path;

use crate::config::{RedirectOnConflict, RedirectProfile};
use crate::output::{CliResult, can_interact};

use super::errors::err2;
use super::plan::{self, ConflictAction, PlanItem};

fn confirm_overwrite(yes: bool, overwrite_n: usize) -> CliResult<bool> {
    if overwrite_n == 0 {
        return Ok(true);
    }
    if !can_interact() && !yes {
        return Err(err2(
            "on_conflict=overwrite requires explicit confirmation.",
            &["Fix: Add --yes, or change on_conflict to rename_new/skip/trash in config."],
        ));
    }
    if can_interact() && !yes {
        let prompt = format!(
            "on_conflict=overwrite: {overwrite_n} file(s) in destination may be replaced. Overwrite?"
        );
        let ok = dialoguer::Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()
            .unwrap_or(false);
        return Ok(ok);
    }
    Ok(true)
}

pub(crate) fn confirm_overwrite_for_profile(
    profile: &RedirectProfile,
    yes: bool,
    items: &[PlanItem],
) -> CliResult<bool> {
    let mut n = 0usize;
    for it in items {
        let dst_exists = Path::new(&it.dst).exists();
        if !dst_exists {
            continue;
        }
        if matches!(it.conflict_action, Some(ConflictAction::Overwrite)) {
            n += 1;
            continue;
        }
        if it.conflict_action.is_none()
            && matches!(profile.on_conflict, RedirectOnConflict::Overwrite)
        {
            n += 1;
        }
    }
    confirm_overwrite(yes, n)
}

pub(crate) fn prompt_conflicts(items: &mut Vec<PlanItem>) -> CliResult {
    if !can_interact() {
        return Err(err2(
            "on_conflict=ask requires interactive mode.",
            &["Fix: Run in an interactive terminal, or change on_conflict in config."],
        ));
    }

    let mut apply_all: Option<ConflictAction> = None;
    let choices: Vec<(&str, ConflictAction, bool)> = vec![
        ("skip", ConflictAction::Skip, false),
        ("rename_new", ConflictAction::RenameNew, false),
        ("rename_date", ConflictAction::RenameDate, false),
        ("rename_existing", ConflictAction::RenameExisting, false),
        ("overwrite", ConflictAction::Overwrite, false),
        ("trash", ConflictAction::Trash, false),
        ("hash_dedup", ConflictAction::HashDedup, false),
        ("skip (apply to all)", ConflictAction::Skip, true),
        ("rename_new (apply to all)", ConflictAction::RenameNew, true),
        (
            "rename_date (apply to all)",
            ConflictAction::RenameDate,
            true,
        ),
        (
            "rename_existing (apply to all)",
            ConflictAction::RenameExisting,
            true,
        ),
        ("overwrite (apply to all)", ConflictAction::Overwrite, true),
        ("trash (apply to all)", ConflictAction::Trash, true),
        ("hash_dedup (apply to all)", ConflictAction::HashDedup, true),
    ];

    for it in items.iter_mut() {
        let dst_path = Path::new(&it.dst);
        if !dst_path.exists() {
            continue;
        }
        if it.conflict_action.is_some() {
            continue;
        }
        if let Some(a) = apply_all {
            it.conflict_action = Some(a);
            continue;
        }

        let src_name = Path::new(&it.src)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&it.src);

        let dst_fp = plan::fingerprint_path(dst_path);
        let src_fp = plan::fingerprint_path(Path::new(&it.src));
        let (dst_size, dst_mtime) = dst_fp.map(|f| (f.size, f.mtime_ts)).unwrap_or((0, 0));
        let (src_size, src_mtime) = src_fp.map(|f| (f.size, f.mtime_ts)).unwrap_or((0, 0));
        ui_println!(
            "Conflict: {src_name} already exists at {}",
            dst_path.display()
        );
        ui_println!("  Existing: {} bytes, mtime={}", dst_size, dst_mtime);
        ui_println!("  Incoming: {} bytes, mtime={}", src_size, src_mtime);

        let labels: Vec<String> = choices.iter().map(|(s, _, _)| s.to_string()).collect();
        let idx = dialoguer::Select::new()
            .with_prompt("Choose conflict action")
            .items(&labels)
            .default(1)
            .interact()
            .unwrap_or(0);
        let action = choices
            .get(idx)
            .map(|(_, a, _)| *a)
            .unwrap_or(ConflictAction::Skip);
        let all = choices.get(idx).map(|(_, _, all)| *all).unwrap_or(false);
        it.conflict_action = Some(action);
        if all {
            apply_all = Some(action);
        }
    }

    Ok(())
}
