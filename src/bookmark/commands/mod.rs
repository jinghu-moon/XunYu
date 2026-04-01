use crate::bookmark::undo::record_undo_batch;
use crate::bookmark::storage::db_path;
use crate::cli::{BookmarkCmd, BookmarkSubCommand, OpenCmd, PinCmd, UnpinCmd};
use crate::output::{CliError, CliResult};

pub(crate) mod io;
pub(crate) mod integration;
pub(crate) mod list;
pub(crate) mod maintenance;
pub(crate) mod mutate;
pub(crate) mod navigation;
pub(crate) mod tags;
pub(crate) mod undo;

pub(crate) use io::cmd_export;
pub(crate) use integration::{cmd_bookmark_init, cmd_bookmark_import, cmd_learn};
pub(crate) use list::{cmd_all, cmd_keys, cmd_list, cmd_recent, cmd_stats};
pub(crate) use maintenance::cmd_check;
pub(crate) use maintenance::{cmd_dedup, cmd_gc};
pub(crate) use mutate::{cmd_rename, cmd_save, cmd_set, cmd_touch, delete_bookmark};
pub(crate) use navigation::{cmd_oi, cmd_open, cmd_z, cmd_zi};
pub(crate) use tags::cmd_tag;
pub(crate) use undo::{cmd_redo, cmd_undo};

pub(crate) fn cmd_bookmark(args: BookmarkCmd) -> CliResult {
    match args.cmd {
        BookmarkSubCommand::Z(a) => cmd_z(a),
        BookmarkSubCommand::Zi(a) => cmd_zi(a),
        BookmarkSubCommand::O(a) => cmd_open(a),
        BookmarkSubCommand::Oi(a) => cmd_oi(a),
        BookmarkSubCommand::Open(a) => cmd_open(OpenCmd {
            patterns: a.patterns,
            tag: a.tag,
            list: a.list,
            score: a.score,
            why: a.why,
            preview: a.preview,
            limit: a.limit,
            json: a.json,
            tsv: a.tsv,
            global: a.global,
            child: a.child,
            base: a.base,
            workspace: a.workspace,
        }),
        BookmarkSubCommand::Save(a) => cmd_save(a),
        BookmarkSubCommand::Set(a) => cmd_set(a),
        BookmarkSubCommand::Delete(a) => delete_bookmark(&a.name, a.yes),
        BookmarkSubCommand::Tag(a) => cmd_tag(a),
        BookmarkSubCommand::Pin(a) => cmd_pin(a),
        BookmarkSubCommand::Unpin(a) => cmd_unpin(a),
        BookmarkSubCommand::Undo(a) => cmd_undo(a),
        BookmarkSubCommand::Redo(a) => cmd_redo(a),
        BookmarkSubCommand::Rename(a) => cmd_rename(a),
        BookmarkSubCommand::List(a) => cmd_list(a),
        BookmarkSubCommand::Recent(a) => cmd_recent(a),
        BookmarkSubCommand::Stats(a) => cmd_stats(a),
        BookmarkSubCommand::Check(a) => cmd_check(a),
        BookmarkSubCommand::Gc(a) => cmd_gc(a),
        BookmarkSubCommand::Dedup(a) => cmd_dedup(a),
        BookmarkSubCommand::Export(a) => cmd_export(a),
        BookmarkSubCommand::Import(a) => cmd_bookmark_import(a),
        BookmarkSubCommand::Init(a) => cmd_bookmark_init(a),
        BookmarkSubCommand::Learn(a) => cmd_learn(a),
        BookmarkSubCommand::Touch(a) => cmd_touch(a),
        BookmarkSubCommand::Keys(a) => cmd_keys(a),
        BookmarkSubCommand::All(a) => cmd_all(a),
    }
}

fn cmd_pin(args: PinCmd) -> CliResult {
    let file = db_path();
    let mut store = crate::bookmark_state::Store::load_or_default(&file)
        .map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let before = store.clone();
    store
        .pin(&args.name)
        .map_err(|e| CliError::new(2, e.to_string()))?;
    store
        .save(&file, crate::store::now_secs())
        .map_err(|e| CliError::new(1, format!("Failed to save store: {e}")))?;
    let after = store.clone();
    if let Err(err) = record_undo_batch(&file, "pin", &before, &after) {
        crate::output::emit_warning(
            format!("Undo history not recorded: {}", err.message),
            &[],
        );
    }
    ui_println!("Pinned '{}'.", args.name);
    Ok(())
}

fn cmd_unpin(args: UnpinCmd) -> CliResult {
    let file = db_path();
    let mut store = crate::bookmark_state::Store::load_or_default(&file)
        .map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let before = store.clone();
    store
        .unpin(&args.name)
        .map_err(|e| CliError::new(2, e.to_string()))?;
    store
        .save(&file, crate::store::now_secs())
        .map_err(|e| CliError::new(1, format!("Failed to save store: {e}")))?;
    let after = store.clone();
    if let Err(err) = record_undo_batch(&file, "unpin", &before, &after) {
        crate::output::emit_warning(
            format!("Undo history not recorded: {}", err.message),
            &[],
        );
    }
    ui_println!("Unpinned '{}'.", args.name);
    Ok(())
}
