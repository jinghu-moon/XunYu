use super::super::common::ensure_parent_dir;
use super::*;

pub(super) fn cmd_rename(args: CtxRenameCmd) -> CliResult {
    validate_name(&args.new)?;
    let path = ctx_store_path();
    let mut store = load_store(&path);
    if args.old == args.new {
        emit_warning("Same name, nothing to do.", &[]);
        return Ok(());
    }
    if !store.profiles.contains_key(&args.old) {
        emit_warning(
            format!("Profile '{}' not found.", args.old),
            &["Hint: Run `xun ctx list` to see existing profiles."],
        );
        return Ok(());
    }
    if store.profiles.contains_key(&args.new) {
        emit_warning(
            format!("Profile '{}' already exists.", args.new),
            &["Fix: Choose a different name, or delete the existing one first."],
        );
        return Ok(());
    }
    if let Some(profile) = store.profiles.remove(&args.old) {
        store.profiles.insert(args.new.clone(), profile);
    }
    ensure_parent_dir(&path);
    save_store(&path, &store)
        .map_err(|e| CliError::new(1, format!("Failed to save ctx store: {e}")))?;
    ui_println!("Renamed ctx '{}' -> '{}'.", args.old, args.new);
    Ok(())
}
