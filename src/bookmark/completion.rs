use crate::bookmark::debug::BookmarkTiming;
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
    let store = match Store::load_or_default(&file) {
        Ok(store) => store,
        Err(_) => return Vec::new(),
    };
    timing.mark("store_load");

    let items = completion_candidates_from_store(
        prefix_lower,
        cwd.map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into())),
        &store,
        crate::store::now_secs(),
        Some(&mut timing),
    );
    let count = items.len();
    timing.mark("map");
    timing.finish(&[
        ("bookmarks", store.bookmarks.len().to_string()),
        ("results", count.to_string()),
        ("prefix_len", prefix_lower.len().to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmark_core::BookmarkSource;
    use crate::bookmark_state::Store;
    use std::path::Path;

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
}
