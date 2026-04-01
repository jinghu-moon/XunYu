use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use console::Term;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use crate::bookmark::cache::{SourceFingerprint, load_cache_owner_checked, store_cache_path};
use crate::bookmark::debug::BookmarkTiming;
use crate::bookmark::lightweight::{
    classify_query_route, query_context_from_owner, query_lightweight_with_timing,
    RankedBookmarkView,
};
use crate::bookmark::path_probe::{BookmarkPathStatus, path_status};
use crate::bookmark::storage::db_path;
use crate::bookmark_core::{QueryContext, QueryScope};
use crate::bookmark_query::{query, BookmarkQuerySpec, QueryAction, QueryFormat, RankedBookmark};
use crate::bookmark_state::Store;
use crate::cli::{OiCmd, OpenCmd, ZCmd, ZiCmd};
use crate::config;
use crate::output::{CliError, CliResult, can_interact};
use crate::store::now_secs;

pub(crate) fn cmd_z(args: ZCmd) -> CliResult {
    let mut timing = BookmarkTiming::new("z");
    let file = db_path();
    timing.mark("db_path");
    let spec = build_query_spec_for_z(&args)?;
    timing.mark("build_spec");
    if let Some(handled) = try_handle_read_only_query_results_borrowed(
        &file,
        &spec,
        ConsumerKind::Jump,
        &mut timing,
    )? {
        timing.finish(&[
            ("bookmarks", handled.bookmark_count.to_string()),
            ("results", handled.result_count.to_string()),
            ("runtime_view", "borrowed".to_string()),
        ]);
        return Ok(());
    }
    let mut store = Store::load_or_default(&file)
        .map_err(|err| CliError::new(1, format!("Failed to load bookmark store: {err}")))?;
    timing.mark("store_load");
    let ctx = QueryContext::from_env_and_store(&store);
    timing.mark("build_ctx");
    let ranked = query(&spec, &store, &ctx, now_secs());
    timing.mark("query");
    let result_count = ranked.len();
    handle_query_results(&file, &mut store, spec, ranked, ConsumerKind::Jump)?;
    timing.mark("handle");
    timing.finish(&[
        ("bookmarks", store.bookmarks.len().to_string()),
        ("results", result_count.to_string()),
        ("runtime_view", "owned".to_string()),
    ]);
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
    if args.patterns.is_empty()
        && !args.list
        && !args.score
        && !args.why
        && !args.preview
        && !args.json
        && !args.tsv
    {
        if let Ok(p) = env::current_dir() {
            open_in_explorer(&p);
        }
        return Ok(());
    }

    let mut timing = BookmarkTiming::new("o");
    let file = db_path();
    timing.mark("db_path");
    let spec = build_query_spec_for_open(&args)?;
    timing.mark("build_spec");
    if let Some(handled) = try_handle_read_only_query_results_borrowed(
        &file,
        &spec,
        ConsumerKind::Open,
        &mut timing,
    )? {
        timing.finish(&[
            ("bookmarks", handled.bookmark_count.to_string()),
            ("results", handled.result_count.to_string()),
            ("runtime_view", "borrowed".to_string()),
        ]);
        return Ok(());
    }
    let mut store = Store::load_or_default(&file)
        .map_err(|err| CliError::new(1, format!("Failed to load bookmark store: {err}")))?;
    timing.mark("store_load");
    let ctx = QueryContext::from_env_and_store(&store);
    timing.mark("build_ctx");
    let ranked = query(&spec, &store, &ctx, now_secs());
    timing.mark("query");
    let result_count = ranked.len();
    handle_query_results(&file, &mut store, spec, ranked, ConsumerKind::Open)?;
    timing.mark("handle");
    timing.finish(&[
        ("bookmarks", store.bookmarks.len().to_string()),
        ("results", result_count.to_string()),
        ("runtime_view", "owned".to_string()),
    ]);
    Ok(())
}

pub(crate) fn cmd_zi(args: ZiCmd) -> CliResult {
    let mut timing = BookmarkTiming::new("zi");
    let file = db_path();
    timing.mark("db_path");
    let spec = build_query_spec_for_zi(&args)?;
    timing.mark("build_spec");
    if let Some(handled) = try_handle_read_only_query_results_borrowed(
        &file,
        &spec,
        ConsumerKind::Jump,
        &mut timing,
    )? {
        timing.finish(&[
            ("bookmarks", handled.bookmark_count.to_string()),
            ("results", handled.result_count.to_string()),
            ("runtime_view", "borrowed".to_string()),
        ]);
        return Ok(());
    }
    let mut store = Store::load_or_default(&file)
        .map_err(|err| CliError::new(1, format!("Failed to load bookmark store: {err}")))?;
    timing.mark("store_load");
    let ctx = QueryContext::from_env_and_store(&store);
    timing.mark("build_ctx");
    let ranked = query(&spec, &store, &ctx, now_secs());
    timing.mark("query");
    let result_count = ranked.len();
    handle_query_results(&file, &mut store, spec, ranked, ConsumerKind::Jump)?;
    timing.mark("handle");
    timing.finish(&[
        ("bookmarks", store.bookmarks.len().to_string()),
        ("results", result_count.to_string()),
        ("runtime_view", "owned".to_string()),
    ]);
    Ok(())
}

pub(crate) fn cmd_oi(args: OiCmd) -> CliResult {
    let mut timing = BookmarkTiming::new("oi");
    let file = db_path();
    timing.mark("db_path");
    let spec = build_query_spec_for_oi(&args)?;
    timing.mark("build_spec");
    if let Some(handled) = try_handle_read_only_query_results_borrowed(
        &file,
        &spec,
        ConsumerKind::Open,
        &mut timing,
    )? {
        timing.finish(&[
            ("bookmarks", handled.bookmark_count.to_string()),
            ("results", handled.result_count.to_string()),
            ("runtime_view", "borrowed".to_string()),
        ]);
        return Ok(());
    }
    let mut store = Store::load_or_default(&file)
        .map_err(|err| CliError::new(1, format!("Failed to load bookmark store: {err}")))?;
    timing.mark("store_load");
    let ctx = QueryContext::from_env_and_store(&store);
    timing.mark("build_ctx");
    let ranked = query(&spec, &store, &ctx, now_secs());
    timing.mark("query");
    let result_count = ranked.len();
    handle_query_results(&file, &mut store, spec, ranked, ConsumerKind::Open)?;
    timing.mark("handle");
    timing.finish(&[
        ("bookmarks", store.bookmarks.len().to_string()),
        ("results", result_count.to_string()),
        ("runtime_view", "owned".to_string()),
    ]);
    Ok(())
}

#[derive(Clone, Copy)]
enum ConsumerKind {
    Jump,
    Open,
}

struct ReadOnlyBorrowedHandled {
    bookmark_count: usize,
    result_count: usize,
}

fn build_query_spec_for_z(args: &ZCmd) -> CliResult<BookmarkQuerySpec> {
    let list_mode = args.list || args.score || args.why || args.preview || args.json || args.tsv;
    Ok(BookmarkQuerySpec {
        keywords: args.patterns.clone(),
        tag: args.tag.clone().or_else(default_tag_from_env),
        scope: resolve_scope(args.global, args.child, args.base.as_deref(), args.workspace.as_deref())?,
        action: if list_mode {
            QueryAction::List
        } else {
            QueryAction::JumpFirst
        },
        limit: args.limit.or_else(|| default_query_limit(list_mode)),
        explain: args.score || args.why,
        why: args.why,
        preview: args.preview,
        output_fmt: resolve_format(args.json, args.tsv),
    })
}

fn build_query_spec_for_open(args: &OpenCmd) -> CliResult<BookmarkQuerySpec> {
    let list_mode = args.list || args.score || args.why || args.preview || args.json || args.tsv;
    Ok(BookmarkQuerySpec {
        keywords: args.patterns.clone(),
        tag: args.tag.clone().or_else(default_tag_from_env),
        scope: resolve_scope(args.global, args.child, args.base.as_deref(), args.workspace.as_deref())?,
        action: if list_mode {
            QueryAction::List
        } else {
            QueryAction::OpenFirst
        },
        limit: args.limit.or_else(|| default_query_limit(list_mode)),
        explain: args.score || args.why,
        why: args.why,
        preview: args.preview,
        output_fmt: resolve_format(args.json, args.tsv),
    })
}

fn build_query_spec_for_zi(args: &ZiCmd) -> CliResult<BookmarkQuerySpec> {
    let list_mode = args.list || args.score || args.why || args.preview || args.json || args.tsv;
    Ok(BookmarkQuerySpec {
        keywords: args.patterns.clone(),
        tag: args.tag.clone().or_else(default_tag_from_env),
        scope: resolve_scope(args.global, args.child, args.base.as_deref(), args.workspace.as_deref())?,
        action: if list_mode {
            QueryAction::List
        } else {
            QueryAction::Interactive
        },
        limit: args.limit.or_else(|| default_query_limit(list_mode)),
        explain: args.score || args.why,
        why: args.why,
        preview: args.preview,
        output_fmt: resolve_format(args.json, args.tsv),
    })
}

fn build_query_spec_for_oi(args: &OiCmd) -> CliResult<BookmarkQuerySpec> {
    let list_mode = args.list || args.score || args.why || args.preview || args.json || args.tsv;
    Ok(BookmarkQuerySpec {
        keywords: args.patterns.clone(),
        tag: args.tag.clone().or_else(default_tag_from_env),
        scope: resolve_scope(args.global, args.child, args.base.as_deref(), args.workspace.as_deref())?,
        action: if list_mode {
            QueryAction::List
        } else {
            QueryAction::OpenInteractive
        },
        limit: args.limit.or_else(|| default_query_limit(list_mode)),
        explain: args.score || args.why,
        why: args.why,
        preview: args.preview,
        output_fmt: resolve_format(args.json, args.tsv),
    })
}

fn default_tag_from_env() -> Option<String> {
    env::var("XUN_DEFAULT_TAG")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

fn resolve_scope(
    global: bool,
    child: bool,
    base: Option<&str>,
    workspace: Option<&str>,
) -> CliResult<QueryScope> {
    if global {
        return Ok(QueryScope::Global);
    }
    if child {
        return Ok(QueryScope::Child);
    }
    if let Some(base) = base {
        return Ok(QueryScope::BaseDir(PathBuf::from(base)));
    }
    if let Some(workspace) = workspace {
        return Ok(QueryScope::Workspace(workspace.to_string()));
    }
    match config::bookmark_default_scope().to_ascii_lowercase().as_str() {
        "global" => Ok(QueryScope::Global),
        "child" => Ok(QueryScope::Child),
        _ => Ok(QueryScope::Auto),
    }
}

fn default_query_limit(enabled: bool) -> Option<usize> {
    enabled.then(config::bookmark_default_list_limit)
}

fn resolve_format(json: bool, tsv: bool) -> QueryFormat {
    if json {
        QueryFormat::Json
    } else if tsv {
        QueryFormat::Tsv
    } else {
        QueryFormat::Text
    }
}

fn handle_query_results(
    file: &Path,
    store: &mut Store,
    spec: BookmarkQuerySpec,
    ranked: Vec<RankedBookmark>,
    kind: ConsumerKind,
) -> CliResult {
    if ranked.is_empty() {
        ui_println!("No matches found.");
        return Ok(());
    }

    if spec.preview {
        ui_println!("Preview mode: no jump/open will be executed.");
        print_ranked_results(&ranked, spec.output_fmt, true);
        return Ok(());
    }
    if spec.why {
        print_why(&ranked[0]);
        return Ok(());
    }
    if matches!(spec.action, QueryAction::List) || spec.explain {
        print_ranked_results(&ranked, spec.output_fmt, spec.explain);
        return Ok(());
    }

    let selected = match spec.action {
        QueryAction::Interactive | QueryAction::OpenInteractive => {
            select_interactive(&ranked).unwrap_or_else(|| ranked[0].clone())
        }
        _ => ranked[0].clone(),
    };
    let top = &selected;
    let status = path_status(Path::new(&top.bookmark.path));
    if matches!(status, BookmarkPathStatus::Missing) {
        return Err(CliError::with_details(
            2,
            format!(
                "Bookmark '{}' points to a missing path.",
                top.bookmark.name.as_deref().unwrap_or("<unnamed>")
            ),
            &[
                format!("Path: {}", top.bookmark.path),
                "Fix: Update the bookmark path or run `xun bookmark gc`.".to_string(),
            ],
        ));
    }

    store
        .record_visit_by_id(&top.bookmark.id, now_secs())
        .map_err(|err| CliError::new(1, format!("Failed to update bookmark visit: {err}")))?;
    store
        .save(file, now_secs())
        .map_err(|err| CliError::new(1, format!("Failed to persist bookmark visit: {err}")))?;

    if ranked.len() > 1 && ranked[1].final_score > 0.0 {
        let gap = 1.0 - (ranked[1].final_score / ranked[0].final_score.max(1.0));
        if gap < 0.15 {
            ui_println!("Hint: close matches available, use `xun bookmark zi ...` to inspect.");
        }
    }

    match kind {
        ConsumerKind::Jump => {
            if config::bookmark_echo() {
                ui_println!("{}", top.bookmark.path);
            }
            out_println!("__BM_CD__ {}", top.bookmark.path);
        }
        ConsumerKind::Open => {
            open_in_explorer(Path::new(&top.bookmark.path));
        }
    }
    Ok(())
}

fn try_handle_read_only_query_results_borrowed(
    file: &Path,
    spec: &BookmarkQuerySpec,
    kind: ConsumerKind,
    timing: &mut BookmarkTiming,
) -> CliResult<Option<ReadOnlyBorrowedHandled>> {
    if std::env::var_os("XUN_BM_DISABLE_LIGHTWEIGHT_VIEW").is_some() {
        return Ok(None);
    }
    if classify_query_route(spec).is_none() {
        return Ok(None);
    }
    let fingerprint = match SourceFingerprint::from_path(file) {
        Ok(fingerprint) => fingerprint,
        Err(_) => return Ok(None),
    };
    let owner = match load_cache_owner_checked(
        &store_cache_path(file),
        crate::bookmark::migration::CURRENT_SCHEMA_VERSION,
        &fingerprint,
        None,
    ) {
        Ok(Some(owner)) => owner,
        Ok(None) => return Ok(None),
        Err(_) => return Ok(None),
    };
    timing.mark("store_load");
    let ctx = query_context_from_owner(
        std::env::current_dir().unwrap_or_else(|_| ".".into()),
        &owner,
    )
    .map_err(|err| CliError::new(1, format!("Failed to build lightweight query context: {err}")))?;
    timing.mark("build_ctx");
    let ranked = query_lightweight_with_timing(spec, &owner, &ctx, now_secs(), Some(timing))
        .map_err(|err| CliError::new(1, format!("Failed to run lightweight query: {err}")))?;
    timing.mark("query");
    let result_count = ranked.len();
    let bookmark_count = owner
        .rows()
        .map(|rows| rows.len())
        .unwrap_or_default();
    handle_query_results_borrowed(spec, ranked, kind)?;
    timing.mark("handle");
    Ok(Some(ReadOnlyBorrowedHandled {
        bookmark_count,
        result_count,
    }))
}

fn handle_query_results_borrowed(
    spec: &BookmarkQuerySpec,
    ranked: Vec<RankedBookmarkView<'_>>,
    _kind: ConsumerKind,
) -> CliResult {
    if ranked.is_empty() {
        ui_println!("No matches found.");
        return Ok(());
    }

    if spec.preview {
        ui_println!("Preview mode: no jump/open will be executed.");
        print_ranked_results_borrowed(&ranked, spec.output_fmt, true);
        return Ok(());
    }
    if spec.why {
        print_why_borrowed(&ranked[0]);
        return Ok(());
    }
    if matches!(spec.action, QueryAction::List) || spec.explain {
        print_ranked_results_borrowed(&ranked, spec.output_fmt, spec.explain);
        return Ok(());
    }

    Ok(())
}

fn select_interactive(ranked: &[RankedBookmark]) -> Option<RankedBookmark> {
    if ranked.is_empty() {
        return None;
    }
    if !can_interact() {
        return ranked.first().cloned();
    }

    let items: Vec<String> = ranked
        .iter()
        .map(|item| {
            format!(
                "{:>7.2}  {:<18} {}",
                item.final_score,
                item.bookmark.name.as_deref().unwrap_or("(unnamed)"),
                item.bookmark.path
            )
        })
        .collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select bookmark")
        .default(0)
        .items(&items)
        .interact_on(&Term::stderr());
    match selection {
        Ok(index) => ranked.get(index).cloned(),
        Err(_) => None,
    }
}

fn print_ranked_results(ranked: &[RankedBookmark], format: QueryFormat, with_scores: bool) {
    match format {
        QueryFormat::Json => {
            let items: Vec<serde_json::Value> = ranked
                .iter()
                .map(|item| {
                    serde_json::json!({
                        "path": item.bookmark.path,
                        "name": item.bookmark.name,
                        "score": item.final_score,
                        "source": format!("{:?}", item.bookmark.source).to_ascii_lowercase(),
                        "pinned": item.bookmark.pinned,
                        "match": item.factors.match_score,
                        "frecency": item.factors.frecency_mult,
                        "scope": item.factors.scope_mult,
                        "source_mult": item.factors.source_mult,
                        "pin_mult": item.factors.pin_mult
                    })
                })
                .collect();
            out_println!("{}", serde_json::Value::Array(items));
        }
        QueryFormat::Tsv => {
            for item in ranked {
                if with_scores {
                    out_println!(
                        "{}\t{}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}",
                        item.bookmark.name.as_deref().unwrap_or(""),
                        item.bookmark.path,
                        item.final_score,
                        item.factors.match_score,
                        item.factors.frecency_mult,
                        item.factors.scope_mult,
                        item.factors.source_mult,
                        item.factors.pin_mult
                    );
                } else {
                    out_println!(
                        "{}\t{}\t{:.2}",
                        item.bookmark.name.as_deref().unwrap_or(""),
                        item.bookmark.path,
                        item.final_score
                    );
                }
            }
        }
        QueryFormat::Text => {
            for (idx, item) in ranked.iter().enumerate() {
                if with_scores {
                    out_println!(
                        "{:>2}. {:.2} match={:.2} frecency={:.2} scope={:.2} source={:.2} pin={:.2} {}",
                        idx + 1,
                        item.final_score,
                        item.factors.match_score,
                        item.factors.frecency_mult,
                        item.factors.scope_mult,
                        item.factors.source_mult,
                        item.factors.pin_mult,
                        item.bookmark.path
                    );
                } else {
                    out_println!(
                        "{:>2}. {:.2} {}",
                        idx + 1,
                        item.final_score,
                        item.bookmark.path
                    );
                }
            }
        }
    }
}

fn print_why(item: &RankedBookmark) {
    out_println!("Jump to: {}", item.bookmark.path);
    out_println!("Reason:");
    out_println!("  MatchScore   {:.2}", item.factors.match_score);
    out_println!("  FrecencyMult {:.2}", item.factors.frecency_mult);
    out_println!("  ScopeMult    {:.2}", item.factors.scope_mult);
    out_println!("  SourceMult   {:.2}", item.factors.source_mult);
    out_println!("  PinMult      {:.2}", item.factors.pin_mult);
    out_println!("  FinalScore   {:.2}", item.final_score);
}

fn print_ranked_results_borrowed(
    ranked: &[RankedBookmarkView<'_>],
    format: QueryFormat,
    with_scores: bool,
) {
    match format {
        QueryFormat::Json => {
            let items: Vec<serde_json::Value> = ranked
                .iter()
                .map(|item| {
                    serde_json::json!({
                        "path": item.row.path(),
                        "name": item.row.name(),
                        "score": item.final_score,
                        "source": format!("{:?}", item.row.source()).to_ascii_lowercase(),
                        "pinned": item.row.pinned(),
                        "match": item.factors.match_score,
                        "frecency": item.factors.frecency_mult,
                        "scope": item.factors.scope_mult,
                        "source_mult": item.factors.source_mult,
                        "pin_mult": item.factors.pin_mult
                    })
                })
                .collect();
            out_println!("{}", serde_json::Value::Array(items));
        }
        QueryFormat::Tsv => {
            for item in ranked {
                if with_scores {
                    out_println!(
                        "{}\t{}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}",
                        item.row.name().unwrap_or(""),
                        item.row.path(),
                        item.final_score,
                        item.factors.match_score,
                        item.factors.frecency_mult,
                        item.factors.scope_mult,
                        item.factors.source_mult,
                        item.factors.pin_mult
                    );
                } else {
                    out_println!(
                        "{}\t{}\t{:.2}",
                        item.row.name().unwrap_or(""),
                        item.row.path(),
                        item.final_score
                    );
                }
            }
        }
        QueryFormat::Text => {
            for (idx, item) in ranked.iter().enumerate() {
                if with_scores {
                    out_println!(
                        "{:>2}. {:.2} match={:.2} frecency={:.2} scope={:.2} source={:.2} pin={:.2} {}",
                        idx + 1,
                        item.final_score,
                        item.factors.match_score,
                        item.factors.frecency_mult,
                        item.factors.scope_mult,
                        item.factors.source_mult,
                        item.factors.pin_mult,
                        item.row.path()
                    );
                } else {
                    out_println!(
                        "{:>2}. {:.2} {}",
                        idx + 1,
                        item.final_score,
                        item.row.path()
                    );
                }
            }
        }
    }
}

fn print_why_borrowed(item: &RankedBookmarkView<'_>) {
    out_println!("Jump to: {}", item.row.path());
    out_println!("Reason:");
    out_println!("  MatchScore   {:.2}", item.factors.match_score);
    out_println!("  FrecencyMult {:.2}", item.factors.frecency_mult);
    out_println!("  ScopeMult    {:.2}", item.factors.scope_mult);
    out_println!("  SourceMult   {:.2}", item.factors.source_mult);
    out_println!("  PinMult      {:.2}", item.factors.pin_mult);
    out_println!("  FinalScore   {:.2}", item.final_score);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
    fn path_status_marks_missing_local_paths() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("missing");
        assert_eq!(path_status(&missing), BookmarkPathStatus::Missing);
    }

    #[test]
    fn path_status_skips_unc_paths() {
        let unc = Path::new(r"\\server\share\folder");
        assert_eq!(path_status(unc), BookmarkPathStatus::Unknown);
    }
}
