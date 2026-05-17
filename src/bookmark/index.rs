use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::bookmark_state::Bookmark;

#[derive(Debug, Clone)]
pub(crate) struct BookmarkIndex {
    terms: Vec<IndexTermEntry>,
    /// O(1) exact path lookup: normalized path → bookmark index.
    path_map: HashMap<String, usize>,
    /// Frecency scores for each bookmark index (parallel to bookmarks array).
    frecency_scores: Vec<f64>,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct PersistedBookmarkIndex {
    pub(crate) version: u32,
    pub(crate) bookmark_count: usize,
    pub(crate) fingerprint: String,
    pub(crate) terms: Vec<IndexTermEntry>,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct IndexTermEntry {
    pub(crate) term: String,
    pub(crate) ids: Vec<usize>,
}

const INDEX_FILE_VERSION: u32 = 1;

impl BookmarkIndex {
    pub(crate) fn build(bookmarks: &[Bookmark]) -> Self {
        let mut terms: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut path_map = HashMap::with_capacity(bookmarks.len());
        let mut frecency_scores = Vec::with_capacity(bookmarks.len());

        for (idx, bookmark) in bookmarks.iter().enumerate() {
            // Build term index.
            let mut seen = HashSet::new();
            for term in bookmark_terms(bookmark) {
                if seen.insert(term.clone()) {
                    terms.entry(term).or_default().push(idx);
                }
            }
            // Build path map for O(1) exact lookup.
            path_map.insert(bookmark.path_norm.clone(), idx);
            // Store frecency score.
            frecency_scores.push(bookmark.frecency_score);
        }
        for ids in terms.values_mut() {
            ids.sort_unstable();
            ids.dedup();
        }
        Self {
            terms: terms
                .into_iter()
                .map(|(term, ids)| IndexTermEntry { term, ids })
                .collect(),
            path_map,
            frecency_scores,
        }
    }

    pub(crate) fn index_min_items() -> usize {
        std::env::var("_BM_INDEX_MIN_ITEMS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1_000)
    }

    pub(crate) fn index_file_path(db_path: &Path) -> PathBuf {
        db_path.with_extension("index.json")
    }

    pub(crate) fn maybe_load_persisted(db_path: &Path, bookmarks: &[Bookmark]) -> Option<Self> {
        Self::maybe_load_persisted_with_threshold(db_path, bookmarks, Self::index_min_items())
    }

    pub(crate) fn sync_persisted(db_path: &Path, bookmarks: &[Bookmark]) -> io::Result<()> {
        Self::sync_persisted_with_threshold(db_path, bookmarks, Self::index_min_items())
    }

    pub(crate) fn to_persisted(bookmarks: &[Bookmark]) -> PersistedBookmarkIndex {
        PersistedBookmarkIndex {
            version: INDEX_FILE_VERSION,
            bookmark_count: bookmarks.len(),
            fingerprint: fingerprint(bookmarks),
            terms: Self::build(bookmarks).terms,
        }
    }

    pub(crate) fn from_persisted(
        persisted: PersistedBookmarkIndex,
        bookmarks: &[Bookmark],
    ) -> Option<Self> {
        if persisted.version != INDEX_FILE_VERSION
            || persisted.bookmark_count != bookmarks.len()
            || persisted.fingerprint != fingerprint(bookmarks)
        {
            return None;
        }
        let terms = sanitize_terms(persisted.terms, persisted.bookmark_count);
        let mut path_map = HashMap::with_capacity(bookmarks.len());
        let mut frecency_scores = Vec::with_capacity(bookmarks.len());
        for (idx, bookmark) in bookmarks.iter().enumerate() {
            path_map.insert(bookmark.path_norm.clone(), idx);
            frecency_scores.push(bookmark.frecency_score);
        }
        Some(Self { terms, path_map, frecency_scores })
    }

    pub(crate) fn from_archived_embedded_persisted(
        persisted: &rkyv::Archived<PersistedBookmarkIndex>,
        bookmarks: &[Bookmark],
    ) -> Option<Self> {
        if persisted.version.to_native() != INDEX_FILE_VERSION
            || persisted.bookmark_count.to_native() as usize != bookmarks.len()
        {
            return None;
        }

        let terms = persisted
            .terms
            .as_slice()
            .iter()
            .map(|entry| IndexTermEntry {
                term: entry.term.as_str().to_string(),
                ids: entry
                    .ids
                    .as_slice()
                    .iter()
                    .map(|idx| idx.to_native() as usize)
                    .filter(|idx| *idx < bookmarks.len())
                    .collect(),
            })
            .collect();
        let mut path_map = HashMap::with_capacity(bookmarks.len());
        let mut frecency_scores = Vec::with_capacity(bookmarks.len());
        for (idx, bookmark) in bookmarks.iter().enumerate() {
            path_map.insert(bookmark.path_norm.clone(), idx);
            frecency_scores.push(bookmark.frecency_score);
        }
        Some(Self { terms, path_map, frecency_scores })
    }

    /// O(1) exact path lookup. Returns the bookmark index for the given normalized path.
    pub(crate) fn lookup_exact_path(&self, path_norm: &str) -> Option<usize> {
        self.path_map.get(path_norm).copied()
    }

    /// Prefix lookup with frecency-sorted results.
    ///
    /// Returns bookmark indices sorted by frecency score (descending).
    pub(crate) fn lookup_prefix(&self, token: &str) -> Vec<usize> {
        if token.is_empty() {
            return Vec::new();
        }
        let start = self
            .terms
            .partition_point(|entry| entry.term.as_str() < token);
        let mut hits = Vec::new();
        for entry in self.terms.iter().skip(start) {
            if !entry.term.starts_with(token) {
                break;
            }
            hits.extend_from_slice(&entry.ids);
        }
        if hits.len() > 1 {
            hits.sort_unstable();
            hits.dedup();
            // Sort by frecency score descending.
            hits.sort_unstable_by(|&a, &b| {
                let fa = self.frecency_scores.get(a).copied().unwrap_or(0.0);
                let fb = self.frecency_scores.get(b).copied().unwrap_or(0.0);
                fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        hits
    }

    fn maybe_load_persisted_with_threshold(
        db_path: &Path,
        bookmarks: &[Bookmark],
        min_items: usize,
    ) -> Option<Self> {
        if bookmarks.len() < min_items {
            return None;
        }
        let path = Self::index_file_path(db_path);
        let content = fs::read(&path).ok()?;
        let persisted: PersistedBookmarkIndex = serde_json::from_slice(&content).ok()?;
        Self::from_persisted(persisted, bookmarks)
    }

    fn sync_persisted_with_threshold(
        db_path: &Path,
        bookmarks: &[Bookmark],
        min_items: usize,
    ) -> io::Result<()> {
        let path = Self::index_file_path(db_path);
        if bookmarks.len() < min_items {
            let _ = fs::remove_file(path);
            return Ok(());
        }

        let persisted = Self::to_persisted(bookmarks);
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, serde_json::to_vec(&persisted)?)?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }
}

fn bookmark_terms(bookmark: &Bookmark) -> Vec<String> {
    let mut out = Vec::new();

    if let Some(name_norm) = bookmark.name_norm.as_deref() {
        push_term_set(name_norm, &mut out);
    }

    let basename = bookmark.path_norm.rsplit('/').next().unwrap_or("");
    if !basename.is_empty() {
        push_term_set(basename, &mut out);
    }

    for segment in bookmark.path_norm.split('/') {
        if !segment.is_empty() {
            push_term_set(segment, &mut out);
        }
    }

    for tag in &bookmark.tags {
        let tag_norm = tag.to_ascii_lowercase();
        if !tag_norm.is_empty() {
            out.push(tag_norm);
        }
    }

    out
}

fn push_term_set(raw: &str, out: &mut Vec<String>) {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return;
    }
    out.push(normalized.clone());

    for token in normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        out.push(token.to_string());
    }
}

fn fingerprint(bookmarks: &[Bookmark]) -> String {
    let mut hasher = blake3::Hasher::new();
    for (index, bookmark) in bookmarks.iter().enumerate() {
        hasher.update(index.to_string().as_bytes());
        hasher.update(bookmark.id.as_bytes());
        hasher.update(bookmark.name_norm.as_deref().unwrap_or("").as_bytes());
        hasher.update(bookmark.path_norm.as_bytes());
        for tag in &bookmark.tags {
            hasher.update(tag.as_bytes());
        }
        hasher.update(&[0xff]);
    }
    hasher.finalize().to_hex().to_string()
}

fn sanitize_terms(terms: Vec<IndexTermEntry>, bookmark_count: usize) -> Vec<IndexTermEntry> {
    let mut decoded = Vec::with_capacity(terms.len());
    for entry in terms {
        let mut ids: Vec<usize> = entry
            .ids
            .into_iter()
            .filter(|idx| *idx < bookmark_count)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        decoded.push(IndexTermEntry {
            term: entry.term,
            ids,
        });
    }
    decoded.sort_by(|left, right| left.term.cmp(&right.term));
    decoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmark_state::Store;
    use std::path::Path;
    use tempfile::tempdir;

    fn make_store() -> Store {
        let mut store = Store::new();
        let cwd = Path::new("C:/work");
        store
            .set("client-api", "C:/work/projects/client-api", cwd, None, 10)
            .unwrap();
        store
            .set("docs-web", "C:/work/repos/docs-web", cwd, None, 10)
            .unwrap();
        store
            .set_explicit_metadata("docs-web", vec!["team".to_string()], String::new())
            .unwrap();
        store
    }

    #[test]
    fn prefix_lookup_matches_name_and_path_terms() {
        let store = make_store();
        let index = BookmarkIndex::build(&store.bookmarks);
        let hits = index.lookup_prefix("clie");
        assert_eq!(hits, vec![0]);

        let hits = index.lookup_prefix("proj");
        assert_eq!(hits, vec![0]);

        let hits = index.lookup_prefix("doc");
        assert_eq!(hits, vec![1]);
    }

    #[test]
    fn prefix_lookup_matches_tag_terms() {
        let store = make_store();
        let index = BookmarkIndex::build(&store.bookmarks);
        let hits = index.lookup_prefix("tea");
        assert_eq!(hits, vec![1]);
    }

    #[test]
    fn persisted_index_roundtrip_restores_terms() {
        let store = make_store();
        let dir = tempdir().unwrap();
        let db = dir.path().join(".xun.bookmark.json");

        BookmarkIndex::sync_persisted_with_threshold(&db, &store.bookmarks, 0).unwrap();
        let loaded = BookmarkIndex::maybe_load_persisted_with_threshold(&db, &store.bookmarks, 0)
            .expect("expected persisted index");

        assert_eq!(loaded.lookup_prefix("clie"), vec![0]);
        assert_eq!(loaded.lookup_prefix("tea"), vec![1]);
    }

    #[test]
    fn persisted_index_mismatch_is_ignored() {
        let store = make_store();
        let mut changed = make_store();
        changed
            .set("extra", "C:/work/extra", Path::new("C:/work"), None, 20)
            .unwrap();
        let dir = tempdir().unwrap();
        let db = dir.path().join(".xun.bookmark.json");

        BookmarkIndex::sync_persisted_with_threshold(&db, &store.bookmarks, 0).unwrap();
        assert!(
            BookmarkIndex::maybe_load_persisted_with_threshold(&db, &changed.bookmarks, 0)
                .is_none()
        );
    }

    #[test]
    fn exact_path_lookup_returns_correct_index() {
        let store = make_store();
        let index = BookmarkIndex::build(&store.bookmarks);

        assert_eq!(
            index.lookup_exact_path("c:/work/projects/client-api"),
            Some(0)
        );
        assert_eq!(
            index.lookup_exact_path("c:/work/repos/docs-web"),
            Some(1)
        );
        assert_eq!(
            index.lookup_exact_path("c:/work/nonexistent"),
            None
        );
    }

    #[test]
    fn prefix_lookup_returns_frecency_sorted() {
        let mut store = Store::new();
        let cwd = Path::new("C:/work");
        store.set("alpha", "C:/work/alpha", cwd, None, 10).unwrap();
        store.set("beta", "C:/work/beta", cwd, None, 10).unwrap();
        store.set("gamma", "C:/work/gamma", cwd, None, 10).unwrap();
        // Set different frecency scores.
        store.bookmarks[0].frecency_score = 1.0;
        store.bookmarks[1].frecency_score = 3.0;
        store.bookmarks[2].frecency_score = 2.0;

        let index = BookmarkIndex::build(&store.bookmarks);
        // Use "work" as prefix - all three bookmarks have "work" in their path.
        let hits = index.lookup_prefix("work");
        // Should be sorted by frecency descending: beta(3.0), gamma(2.0), alpha(1.0)
        assert_eq!(hits, vec![1, 2, 0]);
    }
}
