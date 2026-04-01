#![allow(dead_code)]

//! Contract and borrowed-model layer for stage-1 lightweight runtime view.
//!
//! The lightweight runtime view is an internal acceleration path only. It is
//! intentionally constrained to cache-hit read scenarios and must not leak into
//! the public CLI/config surface or mutation flows.

use rkyv::{access, rancor::Error as RkyvError, util::AlignedVec};

use crate::bookmark::cache::{CachePayload, CachedBookmark};
use crate::bookmark::index::{BookmarkIndex, PersistedBookmarkIndex};
use crate::bookmark::debug::BookmarkTiming;
use crate::bookmark_core::{
    compute_final_score, compute_match_score_parts, compute_scope_mult_parts, frecency_mult,
    normalize_name, pin_mult, source_mult, BookmarkRecordView, BookmarkSource, QueryContext,
    ScoreFactors,
};
use crate::bookmark_query::{BookmarkQuerySpec, QueryAction};

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LightweightRouteKind {
    Complete,
    QueryList,
    QueryWhy,
    QueryPreview,
    List,
    Recent,
    Stats,
    Keys,
    All,
    ActionExecution,
    Set,
    Save,
    Delete,
    Tag,
    Pin,
    Unpin,
    Rename,
    Import,
    Learn,
    Touch,
    Undo,
    Redo,
    Gc,
    Dedup,
    Check,
    Export,
    Init,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LightweightOutputParitySurface {
    CompletionCandidates,
    RankedTsv,
    RankedJson,
    RankedText,
    ExplainText,
    PreviewText,
    BookmarkListTsv,
    BookmarkListJson,
    BookmarkListTable,
    RecentTsv,
    StatsTsv,
    KeysText,
    AllJson,
}

pub(crate) const STAGE1_LIGHTWEIGHT_ROUTES: &[LightweightRouteKind] = &[
    LightweightRouteKind::Complete,
    LightweightRouteKind::QueryList,
    LightweightRouteKind::QueryWhy,
    LightweightRouteKind::QueryPreview,
    LightweightRouteKind::List,
    LightweightRouteKind::Recent,
    LightweightRouteKind::Stats,
    LightweightRouteKind::Keys,
    LightweightRouteKind::All,
];

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const OUTPUT_PARITY_SURFACES: &[LightweightOutputParitySurface] = &[
    LightweightOutputParitySurface::CompletionCandidates,
    LightweightOutputParitySurface::RankedTsv,
    LightweightOutputParitySurface::RankedJson,
    LightweightOutputParitySurface::RankedText,
    LightweightOutputParitySurface::ExplainText,
    LightweightOutputParitySurface::PreviewText,
    LightweightOutputParitySurface::BookmarkListTsv,
    LightweightOutputParitySurface::BookmarkListJson,
    LightweightOutputParitySurface::BookmarkListTable,
    LightweightOutputParitySurface::RecentTsv,
    LightweightOutputParitySurface::StatsTsv,
    LightweightOutputParitySurface::KeysText,
    LightweightOutputParitySurface::AllJson,
];

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn stage1_can_use_lightweight_view(
    cache_hit: bool,
    route: LightweightRouteKind,
) -> bool {
    cache_hit && STAGE1_LIGHTWEIGHT_ROUTES.contains(&route)
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn stage1_requires_owned_store(route: LightweightRouteKind) -> bool {
    !STAGE1_LIGHTWEIGHT_ROUTES.contains(&route)
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn classify_query_route(spec: &BookmarkQuerySpec) -> Option<LightweightRouteKind> {
    match spec.action {
        QueryAction::Complete => Some(LightweightRouteKind::Complete),
        QueryAction::List if spec.why => Some(LightweightRouteKind::QueryWhy),
        QueryAction::List if spec.preview => Some(LightweightRouteKind::QueryPreview),
        QueryAction::List => Some(LightweightRouteKind::QueryList),
        QueryAction::JumpFirst
        | QueryAction::Interactive
        | QueryAction::OpenFirst
        | QueryAction::OpenInteractive => None,
    }
}

pub(crate) struct BookmarkArchivedPayloadOwner {
    bytes: AlignedVec<16>,
}

impl BookmarkArchivedPayloadOwner {
    pub(crate) fn from_aligned_bytes(bytes: AlignedVec<16>) -> Self {
        Self { bytes }
    }

    #[cfg(test)]
    pub(crate) fn from_test_payload(payload: &CachePayload) -> Self {
        let bytes = rkyv::to_bytes::<RkyvError>(payload).expect("serialize cache payload");
        Self { bytes }
    }

    pub(crate) fn rows(&self) -> Result<BookmarkArchivedRows<'_>, RkyvError> {
        let payload = self.archived_payload()?;
        Ok(BookmarkArchivedRows {
            rows: payload.bookmarks.as_slice(),
        })
    }

    pub(crate) fn index_view(&self) -> Result<Option<BookmarkArchivedIndexView<'_>>, RkyvError> {
        let payload = self.archived_payload()?;
        let Some(index) = payload.index.as_ref() else {
            return Ok(None);
        };
        Ok(Some(BookmarkArchivedIndexView {
            archived: index,
            bookmark_count: payload.bookmarks.len(),
        }))
    }

    fn archived_payload(&self) -> Result<&rkyv::Archived<CachePayload>, RkyvError> {
        access::<rkyv::Archived<CachePayload>, RkyvError>(&self.bytes)
    }
}

pub(crate) struct BookmarkArchivedRows<'a> {
    rows: &'a [rkyv::Archived<CachedBookmark>],
}

impl<'a> BookmarkArchivedRows<'a> {
    pub(crate) fn len(&self) -> usize {
        self.rows.len()
    }

    pub(crate) fn get(&self, index: usize) -> Option<BookmarkArchivedRow<'a>> {
        self.rows.get(index).map(BookmarkArchivedRow::new)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = BookmarkArchivedRow<'a>> + 'a {
        self.rows.iter().map(BookmarkArchivedRow::new)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BookmarkArchivedRow<'a> {
    archived: &'a rkyv::Archived<CachedBookmark>,
}

impl<'a> BookmarkArchivedRow<'a> {
    fn new(archived: &'a rkyv::Archived<CachedBookmark>) -> Self {
        Self { archived }
    }

    pub(crate) fn id(&self) -> &str {
        self.archived.id.as_str()
    }

    pub(crate) fn name(&self) -> Option<&str> {
        self.archived.name.as_ref().map(|value| value.as_str())
    }

    pub(crate) fn path(&self) -> &str {
        self.archived.path.as_str()
    }

    pub(crate) fn source(&self) -> BookmarkSource {
        match self.archived.source {
            1 => BookmarkSource::Imported,
            2 => BookmarkSource::Learned,
            _ => BookmarkSource::Explicit,
        }
    }

    pub(crate) fn pinned(&self) -> bool {
        self.archived.pinned
    }

    pub(crate) fn desc(&self) -> &str {
        self.archived.desc.as_str()
    }

    pub(crate) fn workspace(&self) -> Option<&str> {
        self.archived.workspace.as_ref().map(|value| value.as_str())
    }

    pub(crate) fn created_at(&self) -> u64 {
        self.archived.created_at.to_native()
    }

    pub(crate) fn last_visited(&self) -> Option<u64> {
        self.archived.last_visited.as_ref().map(|value| value.to_native())
    }

    pub(crate) fn visit_count(&self) -> Option<u32> {
        self.archived.visit_count.as_ref().map(|value| value.to_native())
    }

    pub(crate) fn frecency_score(&self) -> f64 {
        self.archived.frecency_score.to_native()
    }

    pub(crate) fn tags(&self) -> BookmarkArchivedTags<'a> {
        BookmarkArchivedTags {
            iter: self.archived.tags.as_slice().iter(),
        }
    }

    pub(crate) fn to_record_projection(&self) -> BookmarkArchivedRecordProjection {
        BookmarkArchivedRecordProjection {
            name: self.name().map(str::to_string),
            name_norm: self.name().map(normalize_name),
            path: self.path().to_string(),
            path_norm: self.path().to_ascii_lowercase(),
            tags: self.tags().map(str::to_string).collect(),
            source: self.source(),
            pinned: self.pinned(),
            visit_count: self.visit_count(),
            last_visited: self.last_visited(),
            frecency_score: self.frecency_score(),
            workspace: self.workspace().map(str::to_string),
        }
    }
}

pub(crate) struct BookmarkArchivedTags<'a> {
    iter: std::slice::Iter<'a, rkyv::Archived<String>>,
}

impl<'a> Iterator for BookmarkArchivedTags<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|value| value.as_str())
    }
}

pub(crate) struct BookmarkArchivedRecordProjection {
    name: Option<String>,
    name_norm: Option<String>,
    path: String,
    path_norm: String,
    tags: Vec<String>,
    source: BookmarkSource,
    pinned: bool,
    visit_count: Option<u32>,
    last_visited: Option<u64>,
    frecency_score: f64,
    workspace: Option<String>,
}

impl BookmarkArchivedRecordProjection {
    pub(crate) fn as_record_view(&self) -> BookmarkRecordView<'_> {
        BookmarkRecordView {
            name: self.name.as_deref(),
            name_norm: self.name_norm.as_deref(),
            path: &self.path,
            path_norm: &self.path_norm,
            tags: &self.tags,
            source: self.source,
            pinned: self.pinned,
            visit_count: self.visit_count,
            last_visited: self.last_visited,
            frecency_score: self.frecency_score,
            workspace: self.workspace.as_deref(),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BookmarkArchivedIndexView<'a> {
    archived: &'a rkyv::Archived<PersistedBookmarkIndex>,
    bookmark_count: usize,
}

impl<'a> BookmarkArchivedIndexView<'a> {
    pub(crate) fn lookup_prefix(&self, token: &str) -> Vec<usize> {
        if token.is_empty() {
            return Vec::new();
        }

        let entries = self.archived.terms.as_slice();
        let start = entries.partition_point(|entry| entry.term.as_str() < token);
        let mut hits = Vec::new();
        for entry in entries.iter().skip(start) {
            if !entry.term.starts_with(token) {
                break;
            }
            hits.extend(
                entry
                    .ids
                    .as_slice()
                    .iter()
                    .map(|idx| idx.to_native() as usize)
                    .filter(|idx| *idx < self.bookmark_count),
            );
        }
        if hits.len() > 1 {
            hits.sort_unstable();
            hits.dedup();
        }
        hits
    }

    pub(crate) fn to_owned_index(&self) -> Option<BookmarkIndex> {
        BookmarkIndex::from_archived_embedded_persisted(self.archived, self.bookmark_count)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct RankedBookmarkView<'a> {
    pub(crate) row: BookmarkArchivedRow<'a>,
    pub(crate) factors: ScoreFactors,
    pub(crate) final_score: f64,
}

pub(crate) fn query_context_from_owner(
    cwd: std::path::PathBuf,
    owner: &BookmarkArchivedPayloadOwner,
) -> Result<QueryContext, RkyvError> {
    let cwd_key = cwd.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
    let rows = owner.rows()?;
    let workspace = rows
        .iter()
        .filter(|row| row.workspace().is_some())
        .filter(|row| cwd_key.starts_with(&row.path().to_ascii_lowercase()))
        .max_by_key(|row| row.path().len())
        .and_then(|row| row.workspace().map(str::to_string));

    Ok(QueryContext { cwd, cwd_key, workspace })
}

pub(crate) fn query_lightweight_with_timing<'a>(
    spec: &BookmarkQuerySpec,
    owner: &'a BookmarkArchivedPayloadOwner,
    ctx: &QueryContext,
    now: u64,
    mut timing: Option<&mut BookmarkTiming>,
) -> Result<Vec<RankedBookmarkView<'a>>, RkyvError> {
    let rows = owner.rows()?;
    if rows.len() == 0 {
        return Ok(Vec::new());
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

    let recalled = recall_candidate_indices_lightweight(owner, &tokens)?;
    let mut candidates: Vec<BookmarkArchivedRow<'a>> = match recalled {
        Some(indices) => indices
            .into_iter()
            .filter_map(|idx| rows.get(idx))
            .collect(),
        None => rows.iter().collect(),
    };
    mark_lightweight_timing(&mut timing, "query_recall");

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    candidates.retain(|row| matches_tag_filter_row(*row, tag_filter));
    mark_lightweight_timing(&mut timing, "query_filter");
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let global_max = candidates
        .iter()
        .map(|row| frecency_mult(row.visit_count(), row.last_visited(), row.frecency_score(), 100.0, now))
        .fold(1.0, f64::max);
    mark_lightweight_timing(&mut timing, "query_global_max");

    let mut ranked: Vec<RankedBookmarkView<'a>> = candidates
        .into_iter()
        .filter_map(|row| {
            let name_norm = row.name().map(normalize_name);
            let path_norm = row.path().to_ascii_lowercase();
            let match_score = if tokens.is_empty() {
                1.0
            } else {
                compute_match_score_parts(&tokens, name_norm.as_deref(), &path_norm, |token| {
                    row.tags().any(|tag| tag.eq_ignore_ascii_case(token))
                })
            };
            if match_score <= 0.0 {
                return None;
            }
            let scope_mult = compute_scope_mult_parts(&path_norm, row.workspace(), ctx, &spec.scope);
            if scope_mult <= 0.0 {
                return None;
            }
            let factors = ScoreFactors {
                match_score,
                frecency_mult: frecency_mult(
                    row.visit_count(),
                    row.last_visited(),
                    row.frecency_score(),
                    global_max,
                    now,
                ),
                scope_mult,
                source_mult: source_mult(row.source()),
                pin_mult: pin_mult(row.pinned()),
            };
            Some(RankedBookmarkView {
                row,
                final_score: compute_final_score(factors),
                factors,
            })
        })
        .collect();
    mark_lightweight_timing(&mut timing, "query_rank");

    let effective_limit = match spec.limit {
        Some(limit) => Some(limit),
        None => match spec.action {
            QueryAction::JumpFirst | QueryAction::OpenFirst => Some(2),
            _ => None,
        },
    };
    if let Some(limit) = effective_limit.filter(|limit| *limit < ranked.len()) {
        let nth = limit.saturating_sub(1);
        ranked.select_nth_unstable_by(nth, rank_cmp_lightweight);
        ranked.truncate(limit);
    }
    ranked.sort_by(rank_cmp_lightweight);
    mark_lightweight_timing(&mut timing, "query_topk");

    mark_lightweight_timing(&mut timing, "query_materialize");
    Ok(ranked)
}

fn recall_candidate_indices_lightweight(
    owner: &BookmarkArchivedPayloadOwner,
    tokens: &[String],
) -> Result<Option<Vec<usize>>, RkyvError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let rows = owner.rows()?;
    if rows.len() < BookmarkIndex::index_min_items() {
        return Ok(None);
    }

    let mut current: Option<Vec<usize>> = None;
    let index = owner.index_view()?;

    for token in tokens {
        let mut hits = index
            .as_ref()
            .map(|index| index.lookup_prefix(token))
            .unwrap_or_default();
        if hits.is_empty() {
            hits = fallback_scan_token_lightweight(&rows, token);
        }
        if hits.is_empty() {
            return Ok(Some(Vec::new()));
        }
        current = Some(match current {
            None => hits,
            Some(prev) => intersect_sorted(prev, hits),
        });
        if current.as_ref().is_some_and(|ids| ids.is_empty()) {
            return Ok(Some(Vec::new()));
        }
    }

    Ok(current)
}

fn fallback_scan_token_lightweight(rows: &BookmarkArchivedRows<'_>, token: &str) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter_map(|(idx, row)| {
            let name_norm = row.name().map(normalize_name);
            let path_norm = row.path().to_ascii_lowercase();
            (compute_match_score_parts(&[token], name_norm.as_deref(), &path_norm, |tag| {
                row.tags().any(|value| value.eq_ignore_ascii_case(tag))
            }) > 0.0)
                .then_some(idx)
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

fn matches_tag_filter_row(row: BookmarkArchivedRow<'_>, tag: Option<&str>) -> bool {
    match tag {
        Some(tag) if !tag.trim().is_empty() => row.tags().any(|existing| existing.eq_ignore_ascii_case(tag)),
        _ => true,
    }
}

fn rank_cmp_lightweight(
    a: &RankedBookmarkView<'_>,
    b: &RankedBookmarkView<'_>,
) -> std::cmp::Ordering {
    b.final_score
        .total_cmp(&a.final_score)
        .then_with(|| b.row.pinned().cmp(&a.row.pinned()))
        .then_with(|| source_rank(b.row.source()).cmp(&source_rank(a.row.source())))
        .then_with(|| a.row.path().cmp(b.row.path()))
}

fn source_rank(source: BookmarkSource) -> u8 {
    match source {
        BookmarkSource::Explicit => 0,
        BookmarkSource::Imported => 1,
        BookmarkSource::Learned => 2,
    }
}

fn mark_lightweight_timing(timing: &mut Option<&mut BookmarkTiming>, label: &'static str) {
    if let Some(timing) = timing.as_mut() {
        (*timing).mark(label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmark::cache::CachedBookmark;
    use crate::bookmark::index::BookmarkIndex;
    use crate::bookmark_core::QueryScope;
    use crate::bookmark_query::{query, QueryFormat};
    use crate::bookmark_state::Store;
    use std::path::{Path, PathBuf};

    fn query_spec(action: QueryAction) -> BookmarkQuerySpec {
        BookmarkQuerySpec {
            keywords: vec!["client".to_string()],
            tag: None,
            scope: QueryScope::Auto,
            action,
            limit: Some(20),
            explain: false,
            why: false,
            preview: false,
            output_fmt: QueryFormat::Text,
        }
    }

    fn sample_payload() -> CachePayload {
        let mut store = crate::bookmark_state::Store::new();
        let cwd = Path::new("C:/work");
        store
            .set("client-api", "C:/work/client-api", cwd, None, 10)
            .unwrap();
        store
            .set_explicit_metadata(
                "client-api",
                vec!["team".to_string(), "api".to_string()],
                "main service".to_string(),
            )
            .unwrap();
        store.bookmarks[0].workspace = Some("xunyu".to_string());
        store.bookmarks[0].visit_count = Some(7);
        store.bookmarks[0].last_visited = Some(42);
        store.bookmarks[0].frecency_score = 3.5;

        CachePayload {
            bookmarks: store
                .bookmarks
                .iter()
                .map(CachedBookmark::from_bookmark)
                .collect(),
            index: Some(BookmarkIndex::to_persisted(&store.bookmarks)),
        }
    }

    fn parity_store() -> Store {
        let mut store = Store::new();
        let cwd = Path::new("C:/work");
        store
            .set("client-api", "C:/work/client-api", cwd, None, 10)
            .unwrap();
        store
            .set("client-web", "C:/work/client-web", cwd, None, 10)
            .unwrap();
        store
            .set("docs-api", "C:/docs/api", cwd, None, 10)
            .unwrap();
        store
            .set_explicit_metadata(
                "client-api",
                vec!["team".to_string(), "api".to_string()],
                "main service".to_string(),
            )
            .unwrap();
        store
            .set_explicit_metadata(
                "client-web",
                vec!["team".to_string(), "web".to_string()],
                "frontend".to_string(),
            )
            .unwrap();
        store
            .set_explicit_metadata(
                "docs-api",
                vec!["docs".to_string()],
                "documentation".to_string(),
            )
            .unwrap();
        store.bookmarks[0].workspace = Some("xunyu".to_string());
        store.bookmarks[1].workspace = Some("xunyu".to_string());
        store.bookmarks[2].workspace = Some("docs".to_string());
        store.bookmarks[0].visit_count = Some(20);
        store.bookmarks[1].visit_count = Some(10);
        store.bookmarks[2].visit_count = Some(5);
        store.bookmarks[0].last_visited = Some(100);
        store.bookmarks[1].last_visited = Some(90);
        store.bookmarks[2].last_visited = Some(80);
        store.bookmarks[0].frecency_score = 9.0;
        store.bookmarks[1].frecency_score = 6.0;
        store.bookmarks[2].frecency_score = 3.0;
        store
    }

    fn parity_owner(store: &Store) -> BookmarkArchivedPayloadOwner {
        let payload = CachePayload {
            bookmarks: store
                .bookmarks
                .iter()
                .map(CachedBookmark::from_bookmark)
                .collect(),
            index: Some(BookmarkIndex::to_persisted(&store.bookmarks)),
        };
        BookmarkArchivedPayloadOwner::from_test_payload(&payload)
    }

    fn parity_ctx(store: &Store) -> QueryContext {
        QueryContext::from_cwd_and_store(PathBuf::from("C:/work/client-api/src"), store)
    }

    #[test]
    fn lightweight_view_not_used_for_cache_miss() {
        for route in STAGE1_LIGHTWEIGHT_ROUTES {
            assert!(
                !stage1_can_use_lightweight_view(false, *route),
                "route {:?} should be disabled on cache miss",
                route
            );
        }
    }

    #[test]
    fn mutation_commands_still_require_owned_store() {
        for route in [
            LightweightRouteKind::ActionExecution,
            LightweightRouteKind::Set,
            LightweightRouteKind::Save,
            LightweightRouteKind::Delete,
            LightweightRouteKind::Tag,
            LightweightRouteKind::Pin,
            LightweightRouteKind::Unpin,
            LightweightRouteKind::Rename,
            LightweightRouteKind::Import,
            LightweightRouteKind::Learn,
            LightweightRouteKind::Touch,
            LightweightRouteKind::Undo,
            LightweightRouteKind::Redo,
            LightweightRouteKind::Gc,
            LightweightRouteKind::Dedup,
            LightweightRouteKind::Check,
            LightweightRouteKind::Export,
            LightweightRouteKind::Init,
        ] {
            assert!(
                stage1_requires_owned_store(route),
                "route {:?} must stay on owned store in stage 1",
                route
            );
        }
    }

    #[test]
    fn stage1_lightweight_read_whitelist_fixed() {
        assert_eq!(
            STAGE1_LIGHTWEIGHT_ROUTES,
            &[
                LightweightRouteKind::Complete,
                LightweightRouteKind::QueryList,
                LightweightRouteKind::QueryWhy,
                LightweightRouteKind::QueryPreview,
                LightweightRouteKind::List,
                LightweightRouteKind::Recent,
                LightweightRouteKind::Stats,
                LightweightRouteKind::Keys,
                LightweightRouteKind::All,
            ]
        );
    }

    #[test]
    fn query_route_classification_matches_stage1_contract() {
        assert_eq!(
            classify_query_route(&query_spec(QueryAction::Complete)),
            Some(LightweightRouteKind::Complete)
        );

        let mut list_spec = query_spec(QueryAction::List);
        assert_eq!(
            classify_query_route(&list_spec),
            Some(LightweightRouteKind::QueryList)
        );

        list_spec.why = true;
        assert_eq!(
            classify_query_route(&list_spec),
            Some(LightweightRouteKind::QueryWhy)
        );

        list_spec.why = false;
        list_spec.preview = true;
        assert_eq!(
            classify_query_route(&list_spec),
            Some(LightweightRouteKind::QueryPreview)
        );

        assert_eq!(classify_query_route(&query_spec(QueryAction::JumpFirst)), None);
        assert_eq!(classify_query_route(&query_spec(QueryAction::Interactive)), None);
        assert_eq!(classify_query_route(&query_spec(QueryAction::OpenFirst)), None);
        assert_eq!(classify_query_route(&query_spec(QueryAction::OpenInteractive)), None);
    }

    #[test]
    fn lightweight_view_output_parity_contract_fixed() {
        assert_eq!(
            OUTPUT_PARITY_SURFACES,
            &[
                LightweightOutputParitySurface::CompletionCandidates,
                LightweightOutputParitySurface::RankedTsv,
                LightweightOutputParitySurface::RankedJson,
                LightweightOutputParitySurface::RankedText,
                LightweightOutputParitySurface::ExplainText,
                LightweightOutputParitySurface::PreviewText,
                LightweightOutputParitySurface::BookmarkListTsv,
                LightweightOutputParitySurface::BookmarkListJson,
                LightweightOutputParitySurface::BookmarkListTable,
                LightweightOutputParitySurface::RecentTsv,
                LightweightOutputParitySurface::StatsTsv,
                LightweightOutputParitySurface::KeysText,
                LightweightOutputParitySurface::AllJson,
            ]
        );
    }

    #[test]
    fn archived_payload_owner_keeps_buffer_alive() {
        let owner = BookmarkArchivedPayloadOwner::from_test_payload(&sample_payload());
        let rows = owner.rows().unwrap();
        assert_eq!(rows.len(), 1);
        let row = rows.get(0).unwrap();
        assert_eq!(row.path(), "C:/work/client-api");
        assert_eq!(row.name(), Some("client-api"));
    }

    #[test]
    fn bookmark_archived_row_exposes_read_only_fields() {
        let owner = BookmarkArchivedPayloadOwner::from_test_payload(&sample_payload());
        let row = owner.rows().unwrap().get(0).unwrap();
        assert!(!row.id().is_empty());
        assert_eq!(row.name(), Some("client-api"));
        assert_eq!(row.path(), "C:/work/client-api");
        assert_eq!(row.source(), BookmarkSource::Explicit);
        assert!(!row.pinned());
        assert_eq!(row.desc(), "main service");
        assert_eq!(row.workspace(), Some("xunyu"));
        assert_eq!(row.created_at(), 10);
        assert_eq!(row.last_visited(), Some(42));
        assert_eq!(row.visit_count(), Some(7));
        assert!((row.frecency_score() - 3.5).abs() < 0.0001);
    }

    #[test]
    fn archived_row_can_project_to_bookmark_record_view() {
        let owner = BookmarkArchivedPayloadOwner::from_test_payload(&sample_payload());
        let row = owner.rows().unwrap().get(0).unwrap();
        let projection = row.to_record_projection();
        let view = projection.as_record_view();
        assert_eq!(view.name, Some("client-api"));
        assert_eq!(view.name_norm, Some("client-api"));
        assert_eq!(view.path, "C:/work/client-api");
        assert_eq!(view.path_norm, "c:/work/client-api");
        assert_eq!(view.workspace, Some("xunyu"));
    }

    #[test]
    fn ranked_bookmark_view_contains_row_and_score_factors() {
        let owner = BookmarkArchivedPayloadOwner::from_test_payload(&sample_payload());
        let row = owner.rows().unwrap().get(0).unwrap();
        let ranked = RankedBookmarkView {
            row,
            factors: ScoreFactors {
                match_score: 80.0,
                frecency_mult: 1.2,
                scope_mult: 1.3,
                source_mult: 1.2,
                pin_mult: 1.0,
            },
            final_score: 149.76,
        };
        assert_eq!(ranked.row.name(), Some("client-api"));
        assert!((ranked.final_score - 149.76).abs() < 0.0001);
    }

    #[test]
    fn archived_row_tags_iterate_without_owned_clone() {
        let owner = BookmarkArchivedPayloadOwner::from_test_payload(&sample_payload());
        let row = owner.rows().unwrap().get(0).unwrap();
        let tags: Vec<&str> = row.tags().collect();
        assert_eq!(tags, vec!["team", "api"]);
    }

    #[test]
    fn archived_index_view_restores_lookup_structure() {
        let owner = BookmarkArchivedPayloadOwner::from_test_payload(&sample_payload());
        let index = owner.index_view().unwrap().unwrap();
        assert_eq!(index.lookup_prefix("clie"), vec![0]);
        assert_eq!(index.lookup_prefix("tea"), vec![0]);
        assert_eq!(index.to_owned_index().unwrap().lookup_prefix("api"), vec![0]);
    }

    #[test]
    fn borrowed_query_order_matches_owned_query_order() {
        let store = parity_store();
        let owner = parity_owner(&store);
        let ctx = parity_ctx(&store);
        let spec = BookmarkQuerySpec {
            keywords: vec!["client".to_string()],
            action: QueryAction::List,
            ..BookmarkQuerySpec::default()
        };

        let owned = query(&spec, &store, &ctx, 200);
        let borrowed = query_lightweight_with_timing(&spec, &owner, &ctx, 200, None).unwrap();

        let owned_paths: Vec<&str> = owned.iter().map(|item| item.bookmark.path.as_str()).collect();
        let borrowed_paths: Vec<&str> = borrowed.iter().map(|item| item.row.path()).collect();
        assert_eq!(borrowed_paths, owned_paths);
    }

    #[test]
    fn borrowed_query_respects_limit() {
        let store = parity_store();
        let owner = parity_owner(&store);
        let ctx = parity_ctx(&store);
        let spec = BookmarkQuerySpec {
            keywords: vec!["client".to_string()],
            action: QueryAction::List,
            limit: Some(1),
            ..BookmarkQuerySpec::default()
        };

        let borrowed = query_lightweight_with_timing(&spec, &owner, &ctx, 200, None).unwrap();
        assert_eq!(borrowed.len(), 1);
        assert_eq!(borrowed[0].row.name(), Some("client-api"));
    }

    #[test]
    fn borrowed_query_respects_scope() {
        let store = parity_store();
        let owner = parity_owner(&store);
        let ctx = parity_ctx(&store);
        let spec = BookmarkQuerySpec {
            keywords: vec!["api".to_string()],
            scope: QueryScope::Workspace("xunyu".to_string()),
            action: QueryAction::List,
            ..BookmarkQuerySpec::default()
        };

        let borrowed = query_lightweight_with_timing(&spec, &owner, &ctx, 200, None).unwrap();
        let paths: Vec<&str> = borrowed.iter().map(|item| item.row.path()).collect();
        assert_eq!(paths, vec!["C:/work/client-api"]);
    }

    #[test]
    fn borrowed_query_respects_tag_filter() {
        let store = parity_store();
        let owner = parity_owner(&store);
        let ctx = parity_ctx(&store);
        let spec = BookmarkQuerySpec {
            keywords: vec!["client".to_string()],
            tag: Some("web".to_string()),
            action: QueryAction::List,
            ..BookmarkQuerySpec::default()
        };

        let borrowed = query_lightweight_with_timing(&spec, &owner, &ctx, 200, None).unwrap();
        let paths: Vec<&str> = borrowed.iter().map(|item| item.row.path()).collect();
        assert_eq!(paths, vec!["C:/work/client-web"]);
    }
}
