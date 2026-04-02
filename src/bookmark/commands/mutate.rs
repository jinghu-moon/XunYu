use std::env;
use std::path::{Path, PathBuf};

use crate::bookmark::storage::db_path;
use crate::bookmark::undo::record_undo_batch;
use console::Term;
use dialoguer::{Confirm, theme::ColorfulTheme};

use crate::bookmark_state::Store;
use crate::cli::{RenameCmd, SaveCmd, SetCmd, TouchCmd};
use crate::output::{CliError, CliResult, can_interact, emit_warning};
use crate::path_guard::{PathPolicy, validate_paths};
use crate::store::now_secs;
use crate::util::parse_tags;

pub(crate) fn cmd_save(args: SaveCmd) -> CliResult {
    let file = db_path();
    let mut store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let before = store.clone();

    let path = env::current_dir()
        .unwrap_or_else(|_| Path::new(".").to_path_buf())
        .to_string_lossy()
        .to_string();

    let name = args.name.unwrap_or_else(|| {
        Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("bookmark")
            .to_string()
    });

    let tags = args.tag.map(|t| parse_tags(&t)).unwrap_or_default();
    let is_new = !store
        .bookmarks
        .iter()
        .any(|bookmark| bookmark.name.as_deref().is_some_and(|existing| existing.eq_ignore_ascii_case(&name)));
    store
        .set(&name, &path, Path::new("."), home_dir().as_deref(), now_secs())
        .map_err(|e| CliError::new(1, format!("Failed to set bookmark: {e}")))?;
    store
        .set_explicit_metadata(&name, tags, args.desc.unwrap_or_default())
        .map_err(|e| CliError::new(1, format!("Failed to update bookmark metadata: {e}")))?;
    if args.workspace.is_some() {
        store
            .set_explicit_workspace(&name, normalize_workspace_arg(args.workspace.as_deref()))
            .map_err(|e| CliError::new(1, format!("Failed to update bookmark workspace: {e}")))?;
    }
    store
        .save(&file, now_secs())
        .map_err(|e| CliError::new(1, format!("Failed to save store: {e}")))?;
    let after = store.clone();
    if let Err(err) = record_undo_batch(&file, "save", &before, &after) {
        emit_warning(format!("Undo history not recorded: {}", err.message), &[]);
    }

    if is_new {
        ui_println!("Saved '{}' -> {}", name, path);
    } else {
        ui_println!("Updated '{}' -> {}", name, path);
    }
    Ok(())
}

pub(crate) fn cmd_set(args: SetCmd) -> CliResult {
    let file = db_path();
    let mut store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let before = store.clone();

    let raw_path = args.path.unwrap_or_else(|| {
        env::current_dir()
            .unwrap_or_else(|_| Path::new(".").to_path_buf())
            .to_string_lossy()
            .to_string()
    });
    let path = resolve_bookmark_path(&raw_path)?;

    let tags = args.tag.map(|t| parse_tags(&t)).unwrap_or_default();

    let is_new = !store
        .bookmarks
        .iter()
        .any(|bookmark| {
            bookmark
                .name
                .as_deref()
                .is_some_and(|existing| existing.eq_ignore_ascii_case(&args.name))
        });
    store
        .set(&args.name, &path, Path::new("."), home_dir().as_deref(), now_secs())
        .map_err(|e| CliError::new(1, format!("Failed to set bookmark: {e}")))?;
    store
        .set_explicit_metadata(&args.name, tags, args.desc.unwrap_or_default())
        .map_err(|e| CliError::new(1, format!("Failed to update bookmark metadata: {e}")))?;
    if args.workspace.is_some() {
        store
            .set_explicit_workspace(
                &args.name,
                normalize_workspace_arg(args.workspace.as_deref()),
            )
            .map_err(|e| CliError::new(1, format!("Failed to update bookmark workspace: {e}")))?;
    }
    store
        .save(&file, now_secs())
        .map_err(|e| CliError::new(1, format!("Failed to save store: {e}")))?;
    let after = store.clone();
    if let Err(err) = record_undo_batch(&file, "set", &before, &after) {
        emit_warning(format!("Undo history not recorded: {}", err.message), &[]);
    }

    if !Path::new(&path).exists() {
        emit_warning(
            format!("Path does not exist: {}", path),
            &["Hint: Create the path first, or double-check the spelling."],
        );
    }

    if is_new {
        ui_println!("Saved '{}' -> {}", args.name, path);
    } else {
        ui_println!("Updated '{}' -> {}", args.name, path);
    }
    Ok(())
}

pub(crate) fn delete_bookmark(name: &str, yes: bool) -> CliResult {
    let file = db_path();
    let mut store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let before = store.clone();

    if !store
        .bookmarks
        .iter()
        .any(|bookmark| bookmark.name.as_deref().is_some_and(|existing| existing.eq_ignore_ascii_case(name)))
    {
        emit_warning(
            format!("Bookmark '{}' not found.", name),
            &["Hint: Run `xun bookmark list` to see existing bookmarks."],
        );
        return Ok(());
    }

    if can_interact() && !yes {
        let ans = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Delete bookmark '{}' ?", name))
            .default(false)
            .interact_on(&Term::stderr());

        match ans {
            Ok(true) => {}
            _ => {
                ui_println!("Cancelled.");
                return Ok(());
            }
        }
    }

    store
        .delete_explicit(name)
        .map_err(|e| CliError::new(1, format!("Failed to delete bookmark: {e}")))?;
    store
        .save(&file, now_secs())
        .map_err(|e| CliError::new(1, format!("Failed to save store: {e}")))?;
    let after = store.clone();
    if let Err(err) = record_undo_batch(&file, "delete", &before, &after) {
        emit_warning(format!("Undo history not recorded: {}", err.message), &[]);
    }
    ui_println!("Deleted '{}'", name);
    Ok(())
}

pub(crate) fn cmd_touch(args: TouchCmd) -> CliResult {
    let file = db_path();
    let mut store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    if let Err(_err) = store.touch_explicit(&args.name, now_secs()) {
        emit_warning(
            format!("Bookmark '{}' not found.", args.name),
            &["Hint: Run `xun bookmark list` to see existing bookmarks."],
        );
    } else {
        store
            .save(&file, now_secs())
            .map_err(|e| CliError::new(1, format!("Failed to save store: {e}")))?;
    }
    Ok(())
}

pub(crate) fn cmd_rename(args: RenameCmd) -> CliResult {
    let file = db_path();
    let mut store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let before = store.clone();

    if args.old == args.new {
        emit_warning("Same name, nothing to do.", &[]);
        return Ok(());
    }
    match store.rename(&args.old, &args.new) {
        Ok(_) => {
            store
                .save(&file, now_secs())
                .map_err(|e| CliError::new(1, format!("Failed to save store: {e}")))?;
            let after = store.clone();
            if let Err(err) = record_undo_batch(&file, "rename", &before, &after) {
                emit_warning(format!("Undo history not recorded: {}", err.message), &[]);
            }
            ui_println!("Renamed '{}' -> '{}'.", args.old, args.new);
        }
        Err(err) => {
            emit_warning(err.to_string(), &[]);
        }
    }
    Ok(())
}

fn resolve_bookmark_path(raw: &str) -> CliResult<String> {
    let input = raw.trim();
    if input.is_empty() {
        return Err(CliError::new(2, "Invalid bookmark path."));
    }

    let path = PathBuf::from(input);
    let absolute = if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .map_err(|e| CliError::new(1, format!("Failed to get current directory: {e}")))?
            .join(path)
    };

    let validation = validate_paths(
        vec![absolute.to_string_lossy().to_string()],
        &PathPolicy::for_output(),
    );
    if !validation.issues.is_empty() {
        let details: Vec<String> = validation
            .issues
            .iter()
            .map(|issue| format!("Invalid path: {} ({})", issue.raw, issue.detail))
            .collect();
        return Err(CliError::with_details(
            2,
            "Invalid bookmark path.".to_string(),
            &details,
        ));
    }

    validation
        .ok
        .first()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| CliError::new(2, "Invalid bookmark path."))
}

fn home_dir() -> Option<PathBuf> {
    env::var("USERPROFILE")
        .ok()
        .or_else(|| env::var("HOME").ok())
        .map(PathBuf::from)
}

fn normalize_workspace_arg(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
