use std::path::PathBuf;

use crate::bookmark::debug::BookmarkTiming;
use crate::bookmark_core::{
    compute_final_score, compute_match_score, compute_scope_mult, frecency_mult, pin_mult,
    source_mult, BookmarkRecordView, BookmarkSource, QueryContext, QueryScope, ScoreFactors,
};
use crate::bookmark::index::BookmarkIndex;
use crate::bookmark_state::{Bookmark, Store};

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryAction {
    JumpFirst,
    Interactive,
    OpenFirst,
    OpenInteractive,
    List,
    Complete,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryFormat {
    Text,
    Tsv,
    Json,
}

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookmarkQuerySpec {
    pub keywords: Vec<String>,
    pub tag: Option<String>,
    pub scope: QueryScope,
    pub action: QueryAction,
    pub limit: Option<usize>,
    pub explain: bool,
    pub why: bool,
    pub preview: bool,
    pub output_fmt: QueryFormat,
}

impl Default for BookmarkQuerySpec {
    fn default() -> Self {
        Self {
            keywords: Vec::new(),
            tag: None,
            scope: QueryScope::Auto,
            action: QueryAction::JumpFirst,
            limit: None,
            explain: false,
            why: false,
            preview: false,
            output_fmt: QueryFormat::Text,
        }
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct RankedBookmark {
    pub(crate) bookmark: Bookmark,
    pub(crate) factors: ScoreFactors,
    pub(crate) final_score: f64,
}

struct RankedBookmarkRef<'a> {
    bookmark: &'a Bookmark,
    factors: ScoreFactors,
    final_score: f64,
}

#[doc(hidden)]
pub fn query(
    spec: &BookmarkQuerySpec,
    store: &Store,
    ctx: &QueryContext,
    now: u64,
) -> Vec<RankedBookmark> {
    query_with_timing(spec, store, ctx, now, None)
}

#[doc(hidden)]
pub(crate) fn query_with_timing(
    spec: &BookmarkQuerySpec,
    store: &Store,
    ctx: &QueryContext,
    now: u64,
    mut timing: Option<&mut BookmarkTiming>,
) -> Vec<RankedBookmark> {
    if store.bookmarks.is_empty() {
        return Vec::new();
    }

    let tokens: Vec<String> = spec
        .keywords
        .iter()
        .map(|keyword| keyword.trim().to_ascii_lowercase())
        .filter(|keyword| !keyword.is_empty())
        .collect();

    let tag_filter = spec
        .tag
        .as_deref()
        .map(str::trim)
        .filter(|tag| !tag.is_empty());

    let recalled = recall_candidate_indices(store, &tokens);

    let mut candidates: Vec<&Bookmark> = match recalled {
        Some(indices) => indices
            .into_iter()
            .filter_map(|idx| store.bookmarks.get(idx))
            .collect(),
        None => store.bookmarks.iter().collect(),
    };
    mark_query_timing(&mut timing, "query_recall");

    if candidates.is_empty() {
        return Vec::new();
    }

    candidates.retain(|bookmark| matches_tag_filter(bookmark, tag_filter));
    mark_query_timing(&mut timing, "query_filter");
    if candidates.is_empty() {
        return Vec::new();
    }

    let global_max = candidates
        .iter()
        .map(|bookmark| {
            frecency_mult(
                bookmark.visit_count,
                bookmark.last_visited,
                bookmark.frecency_score,
                100.0,
                now,
            )
        })
        .fold(1.0, f64::max);
    mark_query_timing(&mut timing, "query_global_max");

    let mut ranked: Vec<RankedBookmarkRef<'_>> = candidates
        .into_iter()
        .filter_map(|bookmark| {
            let view = as_view(bookmark);
            let match_score = if tokens.is_empty() {
                1.0
            } else {
                compute_match_score(&tokens, &view)
            };
            if match_score <= 0.0 {
                return None;
            }
            let scope_mult = compute_scope_mult(&view, ctx, &spec.scope);
            if scope_mult <= 0.0 {
                return None;
            }
            let factors = ScoreFactors {
                match_score,
                frecency_mult: frecency_mult(
                    bookmark.visit_count,
                    bookmark.last_visited,
                    bookmark.frecency_score,
                    global_max,
                    now,
                ),
                scope_mult,
                source_mult: source_mult(bookmark.source),
                pin_mult: pin_mult(bookmark.pinned),
            };
            let final_score = compute_final_score(factors);
            Some(RankedBookmarkRef {
                bookmark,
                factors,
                final_score,
            })
        })
        .collect();
    mark_query_timing(&mut timing, "query_rank");

    let effective_limit = query_result_limit(spec);
    if let Some(limit) = effective_limit.filter(|limit| *limit < ranked.len()) {
        let nth = limit.saturating_sub(1);
        ranked.select_nth_unstable_by(nth, rank_cmp);
        ranked.truncate(limit);
    }
    ranked.sort_by(rank_cmp);
    mark_query_timing(&mut timing, "query_topk");

    let materialized = ranked
        .into_iter()
        .map(|item| RankedBookmark {
            bookmark: item.bookmark.clone(),
            factors: item.factors,
            final_score: item.final_score,
        })
        .collect();
    mark_query_timing(&mut timing, "query_materialize");
    materialized
}

fn mark_query_timing(timing: &mut Option<&mut BookmarkTiming>, label: &'static str) {
    if let Some(timing) = timing.as_mut() {
        (*timing).mark(label);
    }
}

fn recall_candidate_indices(store: &Store, tokens: &[String]) -> Option<Vec<usize>> {
    if tokens.is_empty() {
        return None;
    }
    if store.bookmarks.len() < BookmarkIndex::index_min_items() {
        return None;
    }

    let mut current: Option<Vec<usize>> = None;
    let index = store.index();

    for token in tokens {
        let mut hits = index.lookup_prefix(token);
        if hits.is_empty() {
            hits = fallback_scan_token(store, token);
        }
        if hits.is_empty() {
            return Some(Vec::new());
        }
        current = Some(match current {
            None => hits,
            Some(prev) => intersect_sorted(prev, hits),
        });
        if current.as_ref().is_some_and(|ids| ids.is_empty()) {
            return Some(Vec::new());
        }
    }

    current
}

fn fallback_scan_token(store: &Store, token: &str) -> Vec<usize> {
    store
        .bookmarks
        .iter()
        .enumerate()
        .filter_map(|(idx, bookmark)| {
            let view = as_view(bookmark);
            (compute_match_score(&[token], &view) > 0.0).then_some(idx)
        })
        .collect()
}

fn intersect_sorted(left: Vec<usize>, right: Vec<usize>) -> Vec<usize> {
    let mut out = Vec::new();
    let mut li = 0usize;
    let mut ri = 0usize;
    while li < left.len() && ri < right.len() {
        match left[li].cmp(&right[ri]) {
            std::cmp::Ordering::Less => li += 1,
            std::cmp::Ordering::Greater => ri += 1,
            std::cmp::Ordering::Equal => {
                out.push(left[li]);
                li += 1;
                ri += 1;
            }
        }
    }
    out
}

fn query_result_limit(spec: &BookmarkQuerySpec) -> Option<usize> {
    match spec.limit {
        Some(limit) => Some(limit),
        None => match spec.action {
            QueryAction::JumpFirst | QueryAction::OpenFirst => Some(2),
            _ => None,
        },
    }
}

fn rank_cmp(a: &RankedBookmarkRef<'_>, b: &RankedBookmarkRef<'_>) -> std::cmp::Ordering {
    b.final_score
        .total_cmp(&a.final_score)
        .then_with(|| b.bookmark.pinned.cmp(&a.bookmark.pinned))
        .then_with(|| source_rank(b.bookmark.source).cmp(&source_rank(a.bookmark.source)))
        .then_with(|| a.bookmark.path.cmp(&b.bookmark.path))
}

impl QueryContext {
    #[doc(hidden)]
    pub fn from_env() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let cwd_key = normalize_cwd_key(&cwd);
        Self { cwd, cwd_key, workspace: None }
    }

    #[doc(hidden)]
    pub fn from_cwd_and_store(cwd: PathBuf, store: &Store) -> Self {
        let cwd_key = normalize_cwd_key(&cwd);
        let workspace = store
            .bookmarks
            .iter()
            .filter(|bookmark| bookmark.workspace.is_some())
            .filter(|bookmark| cwd_key.starts_with(&bookmark.path_norm))
            .max_by_key(|bookmark| bookmark.path_norm.len())
            .and_then(|bookmark| bookmark.workspace.clone());
        Self { cwd, cwd_key, workspace }
    }

    #[doc(hidden)]
    pub fn from_env_and_store(store: &Store) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::from_cwd_and_store(cwd, store)
    }
}

fn normalize_cwd_key(cwd: &std::path::Path) -> String {
    cwd.to_string_lossy().replace('\\', "/").to_ascii_lowercase()
}

fn source_rank(source: BookmarkSource) -> u8 {
    match source {
        BookmarkSource::Explicit => 0,
        BookmarkSource::Imported => 1,
        BookmarkSource::Learned => 2,
    }
}

fn matches_tag_filter(bookmark: &Bookmark, tag: Option<&str>) -> bool {
    match tag {
        Some(tag) if !tag.trim().is_empty() => bookmark
            .tags
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(tag)),
        _ => true,
    }
}

fn as_view(bookmark: &Bookmark) -> BookmarkRecordView<'_> {
    BookmarkRecordView {
        name: bookmark.name.as_deref(),
        name_norm: bookmark.name_norm.as_deref(),
        path: &bookmark.path,
        path_norm: &bookmark.path_norm,
        tags: &bookmark.tags,
        source: bookmark.source,
        pinned: bookmark.pinned,
        visit_count: bookmark.visit_count,
        last_visited: bookmark.last_visited,
        frecency_score: bookmark.frecency_score,
        workspace: bookmark.workspace.as_deref(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmark_state::Store;
    use std::path::Path;

    fn make_store() -> Store {
        let mut store = Store::new();
        let cwd = Path::new("C:/dev");
        store.set("client-api", "C:/dev/client-api", cwd, None, 10).unwrap();
        store.set("client-web", "C:/dev/client-web", cwd, None, 10).unwrap();
        store.learn("C:/dev/tmp-scratch", cwd, None, 10).unwrap();
        store.bookmarks[0].workspace = Some("xunyu".to_string());
        store.bookmarks[1].workspace = Some("xunyu".to_string());
        store
    }

    #[test]
    fn query_spec_default_values() {
        let spec = BookmarkQuerySpec::default();
        assert!(spec.keywords.is_empty());
        assert_eq!(spec.scope, QueryScope::Auto);
        assert_eq!(spec.action, QueryAction::JumpFirst);
        assert!(!spec.explain);
        assert!(!spec.why);
        assert!(!spec.preview);
        assert_eq!(spec.output_fmt, QueryFormat::Text);
    }

    #[test]
    fn query_empty_store_returns_empty() {
        let store = Store::new();
        let ctx = QueryContext {
            cwd: PathBuf::from("C:/dev"),
            cwd_key: "c:/dev".to_string(),
            workspace: None,
        };
        let result = query(&BookmarkQuerySpec::default(), &store, &ctx, 100);
        assert!(result.is_empty());
    }

    #[test]
    fn context_captures_current_dir() {
        let ctx = QueryContext::from_env();
        assert!(!ctx.cwd.as_os_str().is_empty());
    }

    #[test]
    fn context_workspace_from_store() {
        let store = make_store();
        let cwd = std::env::current_dir().unwrap();
        let manual = QueryContext::from_cwd_and_store(cwd, &store);
        assert!(manual.workspace.is_none() || manual.workspace == Some("xunyu".to_string()));
    }

    #[test]
    fn context_workspace_from_explicit_cwd_and_store() {
        let store = make_store();
        let ctx = QueryContext::from_cwd_and_store(PathBuf::from("C:/dev/client-api/src"), &store);
        assert_eq!(ctx.workspace.as_deref(), Some("xunyu"));
    }

    #[test]
    fn cmd_z_order_matches_query_ordering_constraints() {
        let store = make_store();
        let ctx = QueryContext {
            cwd: PathBuf::from("C:/dev"),
            cwd_key: "c:/dev".to_string(),
            workspace: Some("xunyu".to_string()),
        };
        let spec = BookmarkQuerySpec {
            keywords: vec!["client".to_string()],
            action: QueryAction::List,
            ..BookmarkQuerySpec::default()
        };
        let result = query(&spec, &store, &ctx, 200);
        assert!(result.len() >= 2);
        assert!(result[0].final_score >= result[1].final_score);
    }

    #[test]
    fn explicit_beats_learned_with_same_match() {
        let mut store = Store::new();
        let cwd = Path::new("C:/dev");
        store.set("client", "C:/dev/client", cwd, None, 10).unwrap();
        store.learn("C:/dev/client-tools", cwd, None, 10).unwrap();
        let ctx = QueryContext {
            cwd: PathBuf::from("C:/dev"),
            cwd_key: "c:/dev".to_string(),
            workspace: None,
        };
        let spec = BookmarkQuerySpec {
            keywords: vec!["client".to_string()],
            ..BookmarkQuerySpec::default()
        };
        let result = query(&spec, &store, &ctx, 200);
        assert_eq!(result[0].bookmark.source, BookmarkSource::Explicit);
    }

    #[test]
    fn pinned_explicit_beats_learned() {
        let mut store = Store::new();
        let cwd = Path::new("C:/dev");
        store.set("client", "C:/dev/client", cwd, None, 10).unwrap();
        store.pin("client").unwrap();
        store.learn("C:/dev/client-tools", cwd, None, 10).unwrap();
        let ctx = QueryContext {
            cwd: PathBuf::from("C:/dev"),
            cwd_key: "c:/dev".to_string(),
            workspace: None,
        };
        let spec = BookmarkQuerySpec {
            keywords: vec!["client".to_string()],
            ..BookmarkQuerySpec::default()
        };
        let result = query(&spec, &store, &ctx, 200);
        assert!(result[0].bookmark.pinned);
    }

    #[test]
    fn imported_null_visit_count_uses_seed_frecency() {
        let mut store = Store::new();
        let cwd = Path::new("C:/dev");
        store
            .import_entry("C:/dev/client-imported", cwd, None, 50.0, 10)
            .unwrap();
        let ctx = QueryContext {
            cwd: PathBuf::from("C:/dev"),
            cwd_key: "c:/dev".to_string(),
            workspace: None,
        };
        let spec = BookmarkQuerySpec {
            keywords: vec!["client".to_string()],
            ..BookmarkQuerySpec::default()
        };
        let result = query(&spec, &store, &ctx, 200);
        assert!(result[0].factors.frecency_mult > 1.0);
    }

    #[test]
    fn index_recall_does_not_change_query_order() {
        let store = make_store();
        let ctx = QueryContext {
            cwd: PathBuf::from("C:/dev"),
            cwd_key: "c:/dev".to_string(),
            workspace: Some("xunyu".to_string()),
        };
        let spec = BookmarkQuerySpec {
            keywords: vec!["client".to_string()],
            action: QueryAction::List,
            limit: Some(20),
            ..BookmarkQuerySpec::default()
        };

        let baseline = query(&spec, &store, &ctx, 200);

        unsafe {
            std::env::set_var("_BM_INDEX_MIN_ITEMS", "1");
        }
        let indexed = query(&spec, &store, &ctx, 200);
        unsafe {
            std::env::remove_var("_BM_INDEX_MIN_ITEMS");
        }

        let baseline_paths: Vec<&str> = baseline.iter().map(|item| item.bookmark.path.as_str()).collect();
        let indexed_paths: Vec<&str> = indexed.iter().map(|item| item.bookmark.path.as_str()).collect();
        assert_eq!(baseline_paths, indexed_paths);
    }
}
