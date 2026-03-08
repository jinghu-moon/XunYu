use super::context::AliasCtx;
use super::*;

pub(super) fn cmd_sync(ctx: &AliasCtx) -> Result<()> {
    let cfg = ctx.load()?;
    ctx.sync_shims(&cfg)?;
    ctx.sync_shells(&cfg, None)?;
    let (registered, removed) = apppaths::sync_apppaths(&cfg)?;
    ui_println!("App Paths synced: +{registered} / -{removed}");
    Ok(())
}

pub(super) fn cmd_app_sync(ctx: &AliasCtx) -> Result<()> {
    let cfg = ctx.load()?;
    let all_entries = shim_gen::config_to_sync_entries(&cfg);
    let report = shim_gen::sync_all(
        &all_entries,
        &ctx.shims_dir,
        &ctx.template_path,
        &ctx.template_gui_path,
    )?;
    for (name, err) in report.errors {
        ui_println!("Warning: app sync error [{name}]: {err}");
    }
    let (registered, removed) = apppaths::sync_apppaths(&cfg)?;
    ui_println!("App aliases synced: apppaths +{registered} / -{removed}");
    Ok(())
}
