use crate::bookmark::debug::BookmarkTiming;
use crate::bookmark::cache::{SourceFingerprint, load_cache_owner_checked, store_cache_path};
use crate::bookmark::lightweight::{query_context_from_owner, query_lightweight_with_timing};
use crate::bookmark::storage::db_path;
use crate::bookmark_core::{QueryContext, QueryScope};
use crate::bookmark_query::{BookmarkQuerySpec, QueryAction, QueryFormat, query_with_timing};
use crate::bookmark_state::Store;

pub(crate) fn bookmark_completion_candidates(
    prefix_lower: &str,
    cwd: Option<&str>,
) -> Vec<String> {
    let mut timing = BookmarkTiming::new("complete.bookmark");
    let file = db_path();
    timing.mark("db_path");
    let cwd = cwd
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()));
    let now = crate::store::now_secs();

    let fingerprint = match SourceFingerprint::from_path(&file) {
        Ok(fingerprint) => Some(fingerprint),
        Err(_) => None,
    };
    if std::env::var_os("XUN_BM_DISABLE_LIGHTWEIGHT_VIEW").is_none()
        && let Some(fingerprint) = fingerprint.as_ref()
        && let Ok(Some(owner)) = load_cache_owner_checked(
            &store_cache_path(&file),
            crate::bookmark::migration::CURRENT_SCHEMA_VERSION,
            fingerprint,
            None,
        )
        && let Ok(ctx) = query_context_from_owner(cwd.clone(), &owner)
    {
        timing.mark("store_load");
        let items = completion_candidates_from_owner(
            prefix_lower,
            &owner,
            &ctx,
            now,
            Some(&mut timing),
        );
        let count = items.len();
        timing.mark("map");
        timing.finish(&[
            ("bookmarks", owner.rows().map(|rows| rows.len()).unwrap_or(0).to_string()),
            ("results", count.to_string()),
            ("prefix_len", prefix_lower.len().to_string()),
            ("runtime_view", "borrowed".to_string()),
        ]);
        return items;
    }

    let store = match Store::load_or_default(&file) {
        Ok(store) => store,
        Err(_) => return Vec::new(),
    };
    timing.mark("store_load");

    let items = completion_candidates_from_store(
        prefix_lower,
        cwd,
        &store,
        now,
        Some(&mut timing),
    );
    let count = items.len();
    timing.mark("map");
    timing.finish(&[
        ("bookmarks", store.bookmarks.len().to_string()),
        ("results", count.to_string()),
        ("prefix_len", prefix_lower.len().to_string()),
        ("runtime_view", "owned".to_string()),
    ]);
    items
}

fn completion_candidates_from_store(
    prefix_lower: &str,
    cwd: std::path::PathBuf,
    store: &Store,
    now: u64,
    timing: Option<&mut BookmarkTiming>,
) -> Vec<String> {
    let ctx = QueryContext::from_cwd_and_store(cwd, &store);

    let spec = BookmarkQuerySpec {
        keywords: if prefix_lower.is_empty() {
            Vec::new()
        } else {
            vec![prefix_lower.to_string()]
        },
        tag: None,
        scope: QueryScope::Auto,
        action: QueryAction::Complete,
        limit: Some(20),
        explain: false,
        why: false,
        preview: false,
        output_fmt: QueryFormat::Tsv,
    };

    query_with_timing(&spec, &store, &ctx, now, timing)
        .into_iter()
        .map(|item| item.bookmark.name.unwrap_or(item.bookmark.path))
        .collect()
}

fn completion_candidates_from_owner(
    prefix_lower: &str,
    owner: &crate::bookmark::lightweight::BookmarkArchivedPayloadOwner,
    ctx: &QueryContext,
    now: u64,
    timing: Option<&mut BookmarkTiming>,
) -> Vec<String> {
    let spec = BookmarkQuerySpec {
        keywords: if prefix_lower.is_empty() {
            Vec::new()
        } else {
            vec![prefix_lower.to_string()]
        },
        tag: None,
        scope: QueryScope::Auto,
        action: QueryAction::Complete,
        limit: Some(20),
        explain: false,
        why: false,
        preview: false,
        output_fmt: QueryFormat::Tsv,
    };

    query_lightweight_with_timing(&spec, owner, ctx, now, timing)
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.row.name().unwrap_or(item.row.path()).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmark::cache::{CachePayload, CachedBookmark, SourceFingerprint, store_cache_path, write_cache_payload_atomic};
    use crate::bookmark::index::BookmarkIndex;
    use crate::bookmark_core::BookmarkSource;
    use crate::bookmark_state::Store;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn completion_uses_workspace_context_from_cwd() {
        let mut store = Store::new();
        let cwd = Path::new("C:/dev");
        store.set("client-api", "C:/dev/client-api", cwd, None, 10).unwrap();
        store.set("misc-client", "D:/other/misc-client", cwd, None, 10).unwrap();
        store.bookmarks[0].workspace = Some("xunyu".to_string());
        store.bookmarks[1].workspace = Some("other".to_string());
        store.bookmarks[0].source = BookmarkSource::Explicit;
        store.bookmarks[1].source = BookmarkSource::Explicit;

        let items = completion_candidates_from_store(
            "client",
            std::path::PathBuf::from("C:/dev/client-api/src"),
            &store,
            20,
            None,
        );

        assert_eq!(items.first().map(String::as_str), Some("client-api"));
    }

    #[test]
    fn completion_prefers_borrowed_query_on_cache_hit() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".xun.bookmark.json");
        let debug = dir.path().join("completion-debug.log");
        fs::write(&source, br#"{"schema_version":1,"bookmarks":[]}"#).unwrap();

        let mut store = Store::new();
        store
            .set("client-api", "C:/dev/client-api", Path::new("C:/dev"), None, 10)
            .unwrap();
        store
            .set_explicit_metadata(
                "client-api",
                vec!["team".to_string()],
                "main".to_string(),
            )
            .unwrap();
        let fingerprint = SourceFingerprint::from_path(&source).unwrap();
        let payload = CachePayload {
            bookmarks: store
                .bookmarks
                .iter()
                .map(CachedBookmark::from_bookmark)
                .collect(),
            index: Some(BookmarkIndex::to_persisted(&store.bookmarks)),
        };
        write_cache_payload_atomic(&store_cache_path(&source), 1, &fingerprint, &payload).unwrap();

        unsafe {
            std::env::set_var("_BM_DATA_FILE", &source);
            std::env::set_var("XUN_BM_DEBUG_FILE", &debug);
        }
        let items = bookmark_completion_candidates("client", Some("C:/dev/client-api/src"));
        unsafe {
            std::env::remove_var("_BM_DATA_FILE");
            std::env::remove_var("XUN_BM_DEBUG_FILE");
        }

        assert_eq!(items.first().map(String::as_str), Some("client-api"));
        let line = fs::read_to_string(&debug).unwrap();
        assert!(line.contains("runtime_view=borrowed"));
    }
}
