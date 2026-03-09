use super::*;

pub(super) fn handle_panel_key(app: &mut App, key: KeyCode) -> CliResult {
    match app.panel {
        Panel::Vars => handle_vars_key(app, key),
        Panel::Path => handle_path_key(app, key),
        Panel::Snapshots => handle_snapshot_key(app, key),
        Panel::Profiles => handle_profiles_key(app, key),
        Panel::History => handle_history_key(app, key),
        Panel::Doctor => handle_doctor_key(app, key),
        Panel::Io => handle_io_key(app, key),
    }
}

pub(super) fn handle_vars_key(app: &mut App, key: KeyCode) -> CliResult {
    match key {
        KeyCode::Char('/') => {
            let query = prompt_text("Search vars (name/value, empty=all)", &app.var_query)?;
            app.var_query = query.trim().to_string();
            app.rebuild_var_filter();
            app.status = format!(
                "vars filtered: {}/{}",
                app.filtered_vars.len(),
                app.vars.len()
            );
        }
        KeyCode::Char('C') => {
            app.var_query.clear();
            app.rebuild_var_filter();
            app.status = "vars filter cleared".to_string();
        }
        KeyCode::Char('n') => {
            if let Some((name, value)) = prompt_new_var()? {
                app.manager
                    .set_var(app.scope, &name, &value, false)
                    .map_err(map_env_err)?;
                app.status = format!("set {}", name);
                app.refresh_all();
            }
        }
        KeyCode::Char('e') => {
            if let Some((name, current)) = app.current_var().map(|v| (v.name, v.raw_value))
                && let Some(value) = prompt_edit_var(&name, &current)?
            {
                app.manager
                    .set_var(app.scope, &name, &value, false)
                    .map_err(map_env_err)?;
                app.status = format!("updated {}", name);
                app.refresh_all();
            }
        }
        KeyCode::Char('d') => {
            if let Some(var) = app.current_var()
                && prompt_yes_no(&format!("Delete {}?", var.name))?
            {
                app.manager
                    .delete_var(app.scope, &var.name)
                    .map_err(map_env_err)?;
                app.status = format!("deleted {}", var.name);
                app.refresh_all();
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn handle_path_key(app: &mut App, key: KeyCode) -> CliResult {
    match key {
        KeyCode::Char('a') => {
            if let Some(entry) = prompt_path_entry("Add PATH entry (tail)")? {
                app.manager
                    .path_add(app.scope, &entry, false)
                    .map_err(map_env_err)?;
                app.status = "PATH appended".to_string();
                app.refresh_all();
            }
        }
        KeyCode::Char('A') => {
            if let Some(entry) = prompt_path_entry("Add PATH entry (head)")? {
                app.manager
                    .path_add(app.scope, &entry, true)
                    .map_err(map_env_err)?;
                app.status = "PATH prepended".to_string();
                app.refresh_all();
            }
        }
        KeyCode::Char('d') => {
            if let Some(entry) = app.current_path()
                && prompt_yes_no(&format!("Remove PATH entry?\n{}", entry))?
            {
                app.manager
                    .path_remove(app.scope, &entry)
                    .map_err(map_env_err)?;
                app.status = "PATH removed".to_string();
                app.refresh_all();
            }
        }
        KeyCode::Char('H') => {
            if let Some(entry) = app.current_path() {
                app.manager
                    .path_remove(app.scope, &entry)
                    .map_err(map_env_err)?;
                app.manager
                    .path_add(app.scope, &entry, true)
                    .map_err(map_env_err)?;
                app.status = "PATH entry moved to head".to_string();
                app.refresh_all();
            }
        }
        KeyCode::Char('T') => {
            if let Some(entry) = app.current_path() {
                app.manager
                    .path_remove(app.scope, &entry)
                    .map_err(map_env_err)?;
                app.manager
                    .path_add(app.scope, &entry, false)
                    .map_err(map_env_err)?;
                app.status = "PATH entry moved to tail".to_string();
                app.refresh_all();
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn handle_snapshot_key(app: &mut App, key: KeyCode) -> CliResult {
    match key {
        KeyCode::Char('c') => {
            let desc = prompt_text("Snapshot description", "manual snapshot")?;
            let meta = app
                .manager
                .snapshot_create(Some(&desc))
                .map_err(map_env_err)?;
            app.status = format!("snapshot {}", meta.id);
            app.refresh_all();
        }
        KeyCode::Char('R') => {
            if let Some(id) = app.current_snapshot_id()
                && prompt_yes_no(&format!("Restore snapshot {} ?", id))?
            {
                app.manager
                    .snapshot_restore(app.scope, Some(&id), false)
                    .map_err(map_env_err)?;
                app.status = format!("restored {}", id);
                app.refresh_all();
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn handle_doctor_key(app: &mut App, key: KeyCode) -> CliResult {
    match key {
        KeyCode::Char('g') => {
            let report = app.manager.doctor_run(app.scope).map_err(map_env_err)?;
            app.status = format!("doctor issues={}", report.issues.len());
            app.doctor = Some(report);
        }
        KeyCode::Char('f') => {
            if prompt_yes_no("Apply doctor fixes?")? {
                let fixed = app.manager.doctor_fix(app.scope).map_err(map_env_err)?;
                app.status = format!("fixed {}", fixed.fixed);
                app.doctor = app.manager.doctor_run(app.scope).ok();
                app.refresh_all();
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn handle_profiles_key(app: &mut App, key: KeyCode) -> CliResult {
    match key {
        KeyCode::Char('c') => {
            let default_name = default_profile_name();
            let name = prompt_text("Profile name", &default_name)?;
            if !name.trim().is_empty() {
                let meta = app
                    .manager
                    .profile_capture(name.trim(), app.scope)
                    .map_err(map_env_err)?;
                app.status = format!("captured profile {}", meta.name);
                app.refresh_all();
            }
        }
        KeyCode::Char('a') | KeyCode::Enter => {
            if let Some(name) = app.current_profile_name() {
                let prompt = format!("Apply profile {} to {} scope?", name, app.scope);
                if prompt_yes_no(&prompt)? {
                    let meta = app
                        .manager
                        .profile_apply(&name, Some(app.scope))
                        .map_err(map_env_err)?;
                    app.status = format!("applied profile {} ({})", meta.name, meta.var_count);
                    app.refresh_all();
                }
            }
        }
        KeyCode::Char('d') => {
            if let Some(name) = app.current_profile_name() {
                let prompt = format!("Delete profile {}?", name);
                if prompt_yes_no(&prompt)? {
                    let deleted = app.manager.profile_delete(&name).map_err(map_env_err)?;
                    app.status = if deleted {
                        format!("deleted profile {}", name)
                    } else {
                        format!("profile {} not found", name)
                    };
                    app.refresh_all();
                }
            }
        }
        KeyCode::Char('v') => {
            if let Some(name) = app.current_profile_name() {
                let diff = app
                    .manager
                    .profile_diff(&name, Some(app.scope))
                    .map_err(map_env_err)?;
                app.status = format!("profile {} diff changes={}", name, diff.total_changes());
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn handle_history_key(app: &mut App, key: KeyCode) -> CliResult {
    if let KeyCode::Char('u') = key {
        handle_undo(app)?;
    }
    Ok(())
}

pub(super) fn handle_undo(app: &mut App) -> CliResult {
    if !prompt_yes_no(&format!(
        "Undo last change by restoring latest snapshot for {} scope?",
        app.scope
    ))? {
        app.status = "undo cancelled".to_string();
        return Ok(());
    }
    let restored = app
        .manager
        .snapshot_restore(app.scope, None, true)
        .map_err(map_env_err)?;
    app.status = format!("undo restored {}", restored.id);
    app.refresh_all();
    Ok(())
}

pub(super) fn handle_io_key(app: &mut App, key: KeyCode) -> CliResult {
    match key {
        KeyCode::Char('x') => {
            if let Some((format, path)) = prompt_export_target()? {
                let data = app
                    .manager
                    .export_vars(app.scope, format)
                    .map_err(map_env_err)?;
                std::fs::write(&path, data).map_err(|e| CliError::new(1, format!("{e}")))?;
                app.status = format!("exported {}", path.display());
            }
        }
        KeyCode::Char('i') => {
            if let Some((path, strategy, dry_run)) = prompt_import_source()? {
                let preview = app
                    .manager
                    .import_file(app.scope, &path, strategy, true)
                    .map_err(map_env_err)?;

                if dry_run {
                    app.status = format!(
                        "import preview added={} updated={} skipped={}",
                        preview.added, preview.updated, preview.skipped
                    );
                } else {
                    let prompt = format!(
                        "Import preview: added={} updated={} skipped={}. Apply now?",
                        preview.added, preview.updated, preview.skipped
                    );
                    if prompt_yes_no(&prompt)? {
                        let res = app
                            .manager
                            .import_file(app.scope, &path, strategy, false)
                            .map_err(map_env_err)?;
                        app.status = format!(
                            "import applied added={} updated={} skipped={}",
                            res.added, res.updated, res.skipped
                        );
                        app.refresh_all();
                    } else {
                        app.status = "import cancelled after preview".to_string();
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
