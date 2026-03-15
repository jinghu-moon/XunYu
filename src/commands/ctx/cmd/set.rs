use super::super::common::ensure_parent_dir;
use super::*;
use crate::path_guard::{PathPolicy, validate_paths};

pub(super) fn cmd_set(args: CtxSetCmd) -> CliResult {
    validate_name(&args.name)?;

    let path = ctx_store_path();
    let mut store = load_store(&path);
    let is_new = !store.profiles.contains_key(&args.name);

    let mut profile = store.profiles.get(&args.name).cloned().unwrap_or_default();

    if is_new && args.path.is_none() && profile.path.trim().is_empty() {
        return Err(CliError::with_details(
            2,
            "Missing --path for new profile.".to_string(),
            &["Fix: Use `xun ctx set <name> --path <dir>`."],
        ));
    }

    if let Some(p) = args.path.as_ref() {
        let mut policy = PathPolicy::for_output();
        policy.allow_relative = true;
        let validation = validate_paths(vec![p.clone()], &policy);
        if !validation.issues.is_empty() {
            let details: Vec<String> = validation
                .issues
                .iter()
                .map(|issue| format!("Invalid path: {} ({})", issue.raw, issue.detail))
                .collect();
            return Err(CliError::with_details(
                2,
                "Invalid ctx path.".to_string(),
                &details,
            ));
        }
        if let Some(valid) = validation.ok.first() {
            profile.path = valid.to_string_lossy().to_string();
        } else {
            profile.path = p.clone();
        }
    }
    if profile.path.trim().is_empty() {
        return Err(CliError::with_details(
            2,
            "Profile path is empty.".to_string(),
            &["Fix: Use `xun ctx set <name> --path <dir>`."],
        ));
    }

    apply_proxy_updates(&mut profile.proxy, &args)?;

    if let Some(tag_raw) = args.tag {
        if tag_raw.trim() == "-" {
            profile.tags.clear();
        } else {
            profile.tags = parse_tags(&tag_raw);
        }
    }

    let mut env_updates = Vec::new();
    if let Some(file) = args.env_file {
        let items = load_env_file(Path::new(&file))?;
        env_updates.extend(items);
    }
    for raw in args.env {
        env_updates.push(parse_env_kv(&raw)?);
    }
    if !env_updates.is_empty() {
        for (k, v) in env_updates {
            profile.env.insert(k, v);
        }
    }

    store.profiles.insert(args.name.clone(), profile.clone());

    ensure_parent_dir(&path);
    save_store(&path, &store)
        .map_err(|e| CliError::new(1, format!("Failed to save ctx store: {e}")))?;

    if !Path::new(&profile.path).exists() {
        emit_warning(
            format!("Path does not exist: {}", profile.path),
            &["Hint: Create the path first, or double-check the spelling."],
        );
    }

    if is_new {
        ui_println!("Saved ctx '{}' -> {}", args.name, profile.path);
    } else {
        ui_println!("Updated ctx '{}' -> {}", args.name, profile.path);
    }
    Ok(())
}
