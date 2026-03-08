use std::env;
use std::path::Path;
use std::process::Command;

use console::Term;
use dialoguer::{FuzzySelect, theme::ColorfulTheme};

use crate::cli::{OpenCmd, WorkspaceCmd, ZCmd};
use crate::fuzzy::{FuzzyIndex, matches_tag};
use crate::model::Entry;
use crate::output::{CliError, CliResult, can_interact};
use crate::store::{Lock, append_visit, db_path, load, now_secs};
use crate::util::has_cmd;

pub(crate) fn cmd_z(args: ZCmd) -> CliResult {
    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock")).ok();
    let db = load(&file);

    let tag = args.tag.clone().or_else(|| {
        env::var("XUN_DEFAULT_TAG")
            .ok()
            .filter(|v| !v.trim().is_empty())
    });

    let cwd = std::env::current_dir().ok();
    let cwd_str = cwd.as_ref().and_then(|p| p.to_str());

    let index = FuzzyIndex::from_db(&db);
    let mut scored: Vec<(f64, String, Entry)> = index.search(
        args.pattern.as_deref().unwrap_or(""),
        tag.as_deref(),
        cwd_str,
    );

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    if scored.is_empty() {
        ui_println!("No matches found.");
        return Ok(());
    }

    let interactive = can_interact();
    let selected_entry = if scored.len() == 1 || !interactive {
        Some((scored[0].1.clone(), scored[0].2.clone()))
    } else {
        let items: Vec<String> = scored
            .iter()
            .map(|(_, k, e)| format!("{: <14} {}", k, e.path))
            .collect();

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select bookmark")
            .default(0)
            .items(&items)
            .interact_on(&Term::stderr());

        match selection {
            Ok(idx) => Some((scored[idx].1.clone(), scored[idx].2.clone())),
            Err(_) => None,
        }
    };

    if let Some((name, entry)) = selected_entry {
        let _ = append_visit(&file, &name, now_secs());

        out_println!("__CD__:{}", entry.path);
    }
    Ok(())
}

fn open_in_explorer(path: &Path) {
    match open_in_explorer_spec(path) {
        OpenInExplorerSpec::CmdStart(p) => {
            let _ = Command::new("cmd").args(["/C", "start", ""]).arg(p).spawn();
        }
        OpenInExplorerSpec::Explorer(p) => {
            let _ = Command::new("explorer.exe").arg(p).spawn();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OpenInExplorerSpec {
    CmdStart(std::path::PathBuf),
    Explorer(std::path::PathBuf),
}

fn open_in_explorer_spec(path: &Path) -> OpenInExplorerSpec {
    if path.is_file() {
        OpenInExplorerSpec::CmdStart(path.to_path_buf())
    } else {
        OpenInExplorerSpec::Explorer(path.to_path_buf())
    }
}
pub(crate) fn cmd_open(args: OpenCmd) -> CliResult {
    if args.pattern.is_none() {
        if let Ok(p) = env::current_dir() {
            open_in_explorer(&p);
        }
        return Ok(());
    }

    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock")).ok();
    let db = load(&file);

    let cwd = std::env::current_dir().ok();
    let cwd_str = cwd.as_ref().and_then(|p| p.to_str());

    let index = FuzzyIndex::from_db(&db);
    let mut scored: Vec<(f64, String, Entry)> = index.search(
        args.pattern.as_deref().unwrap_or(""),
        args.tag.as_deref(),
        cwd_str,
    );

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    if scored.is_empty() {
        ui_println!("No matches found.");
        return Ok(());
    }

    let selected_entry = if scored.len() == 1 || !can_interact() {
        Some((scored[0].1.clone(), scored[0].2.clone()))
    } else {
        let items: Vec<String> = scored
            .iter()
            .map(|(_, k, e)| format!("{: <14} {}", k, e.path))
            .collect();

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Open bookmark")
            .default(0)
            .items(&items)
            .interact_on(&Term::stderr());

        match selection {
            Ok(idx) => Some((scored[idx].1.clone(), scored[idx].2.clone())),
            Err(_) => None,
        }
    };

    if let Some((_name, entry)) = selected_entry {
        if !Path::new(&entry.path).exists() {
            return Err(CliError::with_details(
                2,
                format!("Path not found: {}", entry.path),
                &[
                    "Hint: Run `xun list` to review existing bookmarks.",
                    "Fix: Update the bookmark with `xun set <name> <path>`.",
                ],
            ));
        }
        open_in_explorer(Path::new(&entry.path));
    }
    Ok(())
}

pub(crate) fn cmd_workspace(args: WorkspaceCmd) -> CliResult {
    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock")).ok();
    let db = load(&file);

    let mut entries: Vec<(String, Entry)> = db
        .iter()
        .filter(|(_, e)| matches_tag(e, Some(args.tag.as_str())))
        .map(|(k, e)| (k.clone(), e.clone()))
        .filter(|(_, e)| Path::new(&e.path).exists())
        .collect();

    if entries.is_empty() {
        return Err(CliError::with_details(
            2,
            format!("Tag '{}' has no valid bookmarks.", args.tag),
            &[
                "Hint: Use `xun list -t <tag>` to see bookmarks for the tag.",
                "Fix: Add bookmarks with `xun set <name> <path> -t <tag>`.",
            ],
        ));
    }

    if !has_cmd("wt") {
        return Err(CliError::with_details(
            1,
            "wt (Windows Terminal) not found.".to_string(),
            &["Fix: Install Windows Terminal or use `xun open <name>` instead."],
        ));
    }

    let first = entries.remove(0);
    out_println!("__CD__:{}", first.1.path);
    ui_println!("-> {}", first.0);

    for (name, e) in entries {
        let _ = Command::new("wt").args(wt_new_tab_args(&e.path)).spawn();
        ui_println!("+ {} ({})", name, e.path);
    }
    Ok(())
}

fn wt_new_tab_args(starting_dir: &str) -> [&str; 5] {
    [
        "--window",
        "0",
        "new-tab",
        "--startingDirectory",
        starting_dir,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_explorer_spec_uses_cmd_start_for_files_and_explorer_for_dirs() {
        let dir = std::env::temp_dir().join("xun-open-spec-test");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("a.txt");
        let _ = std::fs::write(&file, "data");

        assert!(matches!(
            open_in_explorer_spec(&file),
            OpenInExplorerSpec::CmdStart(_)
        ));
        assert!(matches!(
            open_in_explorer_spec(&dir),
            OpenInExplorerSpec::Explorer(_)
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wt_new_tab_args_matches_expected_shape() {
        let args = wt_new_tab_args(r"C:\tmp");
        assert_eq!(
            args,
            ["--window", "0", "new-tab", "--startingDirectory", r"C:\tmp"]
        );
    }
}
