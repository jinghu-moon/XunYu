use std::env;
use std::path::Path;

use console::Term;
use dialoguer::{Confirm, theme::ColorfulTheme};

use crate::cli::{RenameCmd, SaveCmd, SetCmd, TouchCmd};
use crate::output::{CliError, CliResult, can_interact, emit_warning};
use crate::path_guard::{PathPolicy, validate_paths};
use crate::store::{Lock, append_visit, db_path, load, now_secs, save_db};
use crate::util::parse_tags;

pub(crate) fn cmd_save(args: SaveCmd) -> CliResult {
    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock"))
        .map_err(|e| CliError::new(1, format!("Failed to acquire db lock: {e}")))?;
    let mut db = load(&file);

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
    let is_new = !db.contains_key(&name);
    let mut entry = db.get(&name).cloned().unwrap_or_default();
    entry.path = path.clone();
    if !tags.is_empty() {
        entry.tags = tags;
    }

    db.insert(name.clone(), entry);
    save_db(&file, &db).map_err(|e| CliError::new(1, format!("Failed to save db: {e}")))?;

    if is_new {
        ui_println!("Saved '{}' -> {}", name, path);
    } else {
        ui_println!("Updated '{}' -> {}", name, path);
    }
    Ok(())
}

pub(crate) fn cmd_set(args: SetCmd) -> CliResult {
    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock"))
        .map_err(|e| CliError::new(1, format!("Failed to acquire db lock: {e}")))?;
    let mut db = load(&file);

    let path = args.path.unwrap_or_else(|| {
        env::current_dir()
            .unwrap_or_else(|_| Path::new(".").to_path_buf())
            .to_string_lossy()
            .to_string()
    });
    let mut policy = PathPolicy::for_output();
    policy.allow_relative = true;
    let validation = validate_paths(vec![path.clone()], &policy);
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
    let path = validation
        .ok
        .first()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or(path);

    let tags = args.tag.map(|t| parse_tags(&t)).unwrap_or_default();

    let is_new = !db.contains_key(&args.name);
    let mut entry = db.get(&args.name).cloned().unwrap_or_default();
    entry.path = path.clone();
    if !tags.is_empty() {
        entry.tags = tags;
    }

    db.insert(args.name.clone(), entry);
    save_db(&file, &db).map_err(|e| CliError::new(1, format!("Failed to save db: {e}")))?;

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
    let _lock = Lock::acquire(&file.with_extension("lock"))
        .map_err(|e| CliError::new(1, format!("Failed to acquire db lock: {e}")))?;
    let mut db = load(&file);

    if !db.contains_key(name) {
        emit_warning(
            format!("Bookmark '{}' not found.", name),
            &["Hint: Run `xun list` to see existing bookmarks."],
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

    db.remove(name);
    save_db(&file, &db).map_err(|e| CliError::new(1, format!("Failed to save db: {e}")))?;
    ui_println!("Deleted '{}'", name);
    Ok(())
}

pub(crate) fn cmd_touch(args: TouchCmd) -> CliResult {
    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock"))
        .map_err(|e| CliError::new(1, format!("Failed to acquire db lock: {e}")))?;
    let db = load(&file);
    if db.contains_key(&args.name) {
        let _ = append_visit(&file, &args.name, now_secs());
    } else {
        emit_warning(
            format!("Bookmark '{}' not found.", args.name),
            &["Hint: Run `xun list` to see existing bookmarks."],
        );
    }
    Ok(())
}

pub(crate) fn cmd_rename(args: RenameCmd) -> CliResult {
    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock"))
        .map_err(|e| CliError::new(1, format!("Failed to acquire db lock: {e}")))?;
    let mut db = load(&file);

    if args.old == args.new {
        emit_warning("Same name, nothing to do.", &[]);
        return Ok(());
    }
    if !db.contains_key(&args.old) {
        emit_warning(
            format!("Bookmark '{}' not found.", args.old),
            &["Hint: Run `xun list` to see existing bookmarks."],
        );
        return Ok(());
    }
    if db.contains_key(&args.new) {
        emit_warning(
            format!("Bookmark '{}' already exists.", args.new),
            &["Fix: Choose a different name, or delete the existing one first."],
        );
        return Ok(());
    }
    if let Some(entry) = db.remove(&args.old) {
        db.insert(args.new.clone(), entry);
        save_db(&file, &db).map_err(|e| CliError::new(1, format!("Failed to save db: {e}")))?;
        ui_println!("Renamed '{}' -> '{}'.", args.old, args.new);
    }
    Ok(())
}
