use super::super::common::ensure_parent_dir;
use super::*;

pub(super) fn cmd_del(args: CtxDelCmd) -> CliResult {
    let path = ctx_store_path();
    let mut store = load_store(&path);
    if store.profiles.remove(&args.name).is_none() {
        emit_warning(
            format!("Profile '{}' not found.", args.name),
            &["Hint: Run `xun ctx list` to see existing profiles."],
        );
        return Ok(());
    }
    ensure_parent_dir(&path);
    save_store(&path, &store)
        .map_err(|e| CliError::new(1, format!("Failed to save ctx store: {e}")))?;
    ui_println!("Deleted ctx '{}'.", args.name);
    Ok(())
}
