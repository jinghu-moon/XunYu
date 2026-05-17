use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::bookmark::cache::{CachePayload, CachedBookmark, SourceFingerprint, load_cache_store_data_mmap, store_cache_path, write_cache_payload_atomic};
use crate::bookmark::index::BookmarkIndex;
use crate::bookmark::debug::BookmarkLoadTiming;
use crate::bookmark::undo::{BookmarkUndoBatch, BookmarkUndoOp};
use crate::bookmark_core::{normalize_name, normalize_path, BookmarkSource, NormalizePathError};
use crate::bookmark::migration::{CURRENT_SCHEMA_VERSION, LegacyEntry, detect_schema_version, parse_legacy_entries};

#[doc(hidden)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bookmark {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) name_norm: Option<String>,
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) path_norm: String,
    #[serde(default = "default_source")]
    pub(crate) source: BookmarkSource,
    #[serde(default)]
    pub(crate) pinned: bool,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) desc: String,
    #[serde(default)]
    pub(crate) workspace: Option<String>,
    #[serde(default)]
    pub(crate) created_at: u64,
    #[serde(default)]
    pub(crate) last_visited: Option<u64>,
    #[serde(default)]
    pub(crate) visit_count: Option<u32>,
    #[serde(default)]
    pub(crate) frecency_score: f64,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct Store {
    pub schema_version: u32,
    pub bookmarks: Vec<Bookmark>,
    dirty_count: u32,
    last_save_at: u64,
    storage_path: Option<PathBuf>,
    index: OnceLock<BookmarkIndex>,
}

impl Clone for Store {
    fn clone(&self) -> Self {
        Self {
            schema_version: self.schema_version,
            bookmarks: self.bookmarks.clone(),
            dirty_count: self.dirty_count,
            last_save_at: self.last_save_at,
            storage_path: self.storage_path.clone(),
            index: OnceLock::new(),
        }
    }
}

impl PartialEq for Store {
    fn eq(&self, other: &Self) -> bool {
        self.schema_version == other.schema_version
            && self.bookmarks == other.bookmarks
            && self.dirty_count == other.dirty_count
            && self.last_save_at == other.last_save_at
    }
}

#[doc(hidden)]
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("bookmark store missing schema_version")]
    MissingSchemaVersion,
    #[error("bookmark store unsupported schema_version: {0}")]
    UnsupportedSchemaVersion(u32),
    #[error("bookmark name already exists: {0}")]
    NameAlreadyExists(String),
    #[error("bookmark not found: {0}")]
    NotFound(String),
    #[error("normalize path failed: {0:?}")]
    NormalizePath(NormalizePathError),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreFile {
    schema_version: Option<u32>,
    #[serde(default)]
    bookmarks: Vec<Bookmark>,
}

#[derive(Debug, Serialize)]
struct StoreFileWrite {
    schema_version: Option<u32>,
    bookmarks: Vec<PersistedBookmark>,
}

#[derive(Debug, Serialize)]
struct PersistedBookmark {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<BookmarkSource>,
    #[serde(skip_serializing_if = "is_false")]
    pinned: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    desc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace: Option<String>,
    #[serde(skip_serializing_if = "is_zero_u64")]
    created_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_visited: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    visit_count: Option<u32>,
    #[serde(skip_serializing_if = "is_zero_f64")]
    frecency_score: f64,
}

impl Store {
    #[doc(hidden)]
    pub fn new() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            bookmarks: Vec::new(),
            dirty_count: 0,
            last_save_at: 0,
            storage_path: None,
            index: OnceLock::new(),
        }
    }

    #[doc(hidden)]
    pub fn load(path: &Path) -> Result<Self, StoreError> {
        let fingerprint = SourceFingerprint::from_path(path)?;
        let mut timing = BookmarkLoadTiming::new(path, fingerprint.len as usize);
        timing.mark("stat_file");
        let cache_disabled = std::env::var_os("XUN_BM_DISABLE_BINARY_CACHE").is_some();
        if let Some(payload) = load_cache_store_data_mmap(
            &store_cache_path(path),
            CURRENT_SCHEMA_VERSION,
            &fingerprint,
            if cache_disabled { None } else { Some(&mut timing) },
        )? {
            let bookmark_count = payload.bookmarks.len();
            timing.finish(
                bookmark_count,
                true,
                &[(
                    "cache",
                    if cache_disabled {
                        "disabled".to_string()
                    } else {
                        "hit".to_string()
                    },
                )],
            );
            let store = Self {
                schema_version: CURRENT_SCHEMA_VERSION,
                bookmarks: payload.bookmarks,
                dirty_count: 0,
                last_save_at: 0,
                storage_path: Some(path.to_path_buf()),
                index: OnceLock::new(),
            };
            if let Some(index) = payload.index {
                let _ = store.index.set(index);
            }
            return Ok(store);
        }

        let raw = fs::read(path)?;
        timing.mark("read_file");
        let (file, fast_path) = match serde_json::from_slice::<StoreFile>(&raw) {
            Ok(file) => (file, true),
            Err(_) => {
                timing.mark("parse_store_file_miss");
                let value: serde_json::Value = serde_json::from_slice(&raw)?;
                timing.mark("parse_value");
                (migrate_store_value(value)?, false)
            }
        };
        if fast_path {
            timing.mark("parse_store_file");
        } else {
            timing.mark("migrate_value");
        }
        let version = file.schema_version.ok_or(StoreError::MissingSchemaVersion)?;
        if version != CURRENT_SCHEMA_VERSION {
            return Err(StoreError::UnsupportedSchemaVersion(version));
        }
        let mut bookmarks = file.bookmarks;
        normalize_loaded_bookmarks(&mut bookmarks);
        timing.mark("normalize");
        let bookmark_count = bookmarks.len();
        let payload = CachePayload {
            bookmarks: bookmarks.iter().map(CachedBookmark::from_bookmark).collect(),
            index: Some(BookmarkIndex::to_persisted(&bookmarks)),
        };
        let _ = write_cache_payload_atomic(
            &store_cache_path(path),
            CURRENT_SCHEMA_VERSION,
            &fingerprint,
            &payload,
        );
        timing.mark("write_cache");
        timing.finish(
            bookmark_count,
            fast_path,
            &[(
                "cache",
                if cache_disabled {
                    "disabled".to_string()
                } else {
                    "miss".to_string()
                },
            )],
        );
        Ok(Self {
            schema_version: version,
            bookmarks,
            dirty_count: 0,
            last_save_at: 0,
            storage_path: Some(path.to_path_buf()),
            index: OnceLock::new(),
        })
    }

    #[doc(hidden)]
    pub fn load_or_default(path: &Path) -> Result<Self, StoreError> {
        match Self::load(path) {
            Ok(store) => Ok(store),
            Err(StoreError::Io(err)) if err.kind() == io::ErrorKind::NotFound => {
                let mut store = Self::new();
                store.storage_path = Some(path.to_path_buf());
                Ok(store)
            }
            Err(err) => Err(err),
        }
    }

    #[doc(hidden)]
    pub fn save(&mut self, path: &Path, now: u64) -> Result<(), StoreError> {
        self.write_store(path, now, true)
    }

    pub(crate) fn save_exact(&mut self, path: &Path, now: u64) -> Result<(), StoreError> {
        self.write_store(path, now, false)
    }

    fn write_store(&mut self, path: &Path, now: u64, with_runtime_aging: bool) -> Result<(), StoreError> {
        if with_runtime_aging {
            self.apply_runtime_aging();
        }
        let file = StoreFileWrite {
            schema_version: Some(self.schema_version),
            bookmarks: self
                .bookmarks
                .iter()
                .map(PersistedBookmark::from_bookmark)
                .collect(),
        };
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, serde_json::to_vec(&file)?)?;
        fs::rename(&tmp, path)?;
        let fingerprint = SourceFingerprint::from_path(path)?;
        self.dirty_count = 0;
        self.last_save_at = now;
        self.storage_path = Some(path.to_path_buf());
        let payload = CachePayload {
            bookmarks: self
                .bookmarks
                .iter()
                .map(CachedBookmark::from_bookmark)
                .collect(),
            index: Some(BookmarkIndex::to_persisted(&self.bookmarks)),
        };
        let _ = write_cache_payload_atomic(
            &store_cache_path(path),
            self.schema_version,
            &fingerprint,
            &payload,
        );
        let _ = BookmarkIndex::sync_persisted(path, &self.bookmarks);
        Ok(())
    }

    pub(crate) fn apply_undo_batch_forward(
        &mut self,
        batch: &BookmarkUndoBatch,
    ) -> Result<(), StoreError> {
        self.apply_undo_batch(batch, false)
    }

    pub(crate) fn apply_undo_batch_inverse(
        &mut self,
        batch: &BookmarkUndoBatch,
    ) -> Result<(), StoreError> {
        self.apply_undo_batch(batch, true)
    }

    fn apply_undo_batch(&mut self, batch: &BookmarkUndoBatch, inverse: bool) -> Result<(), StoreError> {
        let target_version = if inverse {
            batch.before_schema_version
        } else {
            batch.after_schema_version
        };
        if target_version > CURRENT_SCHEMA_VERSION {
            return Err(StoreError::UnsupportedSchemaVersion(target_version));
        }

        if inverse {
            let mut creates: Vec<_> = batch
                .ops
                .iter()
                .filter_map(|op| match op {
                    BookmarkUndoOp::Create { after, after_index } => Some((*after_index, after)),
                    _ => None,
                })
                .collect();
            creates.sort_by_key(|(index, _)| *index);
            for (_index, after) in creates.into_iter().rev() {
                self.remove_bookmark_by_id(&after.id)?;
            }

            for op in &batch.ops {
                if let BookmarkUndoOp::Update { before, .. } = op {
                    self.replace_bookmark_by_id(&before.id, before.clone())?;
                }
            }

            let mut deletes: Vec<_> = batch
                .ops
                .iter()
                .filter_map(|op| match op {
                    BookmarkUndoOp::Delete {
                        before,
                        before_index,
                    } => Some((*before_index, before)),
                    _ => None,
                })
                .collect();
            deletes.sort_by_key(|(index, _)| *index);
            for (index, before) in deletes {
                self.insert_bookmark_at(index, before.clone());
            }
        } else {
            let mut deletes: Vec<_> = batch
                .ops
                .iter()
                .filter_map(|op| match op {
                    BookmarkUndoOp::Delete {
                        before,
                        before_index,
                    } => Some((*before_index, before)),
                    _ => None,
                })
                .collect();
            deletes.sort_by_key(|(index, _)| *index);
            for (_index, before) in deletes.into_iter().rev() {
                self.remove_bookmark_by_id(&before.id)?;
            }

            for op in &batch.ops {
                if let BookmarkUndoOp::Update { after, .. } = op {
                    self.replace_bookmark_by_id(&after.id, after.clone())?;
                }
            }

            let mut creates: Vec<_> = batch
                .ops
                .iter()
                .filter_map(|op| match op {
                    BookmarkUndoOp::Create { after, after_index } => Some((*after_index, after)),
                    _ => None,
                })
                .collect();
            creates.sort_by_key(|(index, _)| *index);
            for (index, after) in creates {
                self.insert_bookmark_at(index, after.clone());
            }
        }

        self.schema_version = target_version;
        if !batch.ops.is_empty() {
            self.dirty_count += 1;
            self.invalidate_index();
        }
        Ok(())
    }

    #[doc(hidden)]
    pub fn set(
        &mut self,
        name: &str,
        raw_path: &str,
        cwd: &Path,
        home: Option<&Path>,
        now: u64,
    ) -> Result<(), StoreError> {
        let normalized = normalize_path_runtime(raw_path, cwd, home).map_err(StoreError::NormalizePath)?;
        let name_norm = normalize_name(name);
        if let Some(existing) = self
            .bookmarks
            .iter_mut()
            .find(|b| b.source == BookmarkSource::Explicit && b.name_norm.as_deref() == Some(&name_norm))
        {
            existing.name = Some(name.to_string());
            existing.path = normalized.display().to_string();
            existing.path_norm = normalized.key().to_string();
            self.dirty_count += 1;
            self.invalidate_index();
            return Ok(());
        }

        self.bookmarks.push(Bookmark {
            id: Uuid::new_v4().to_string(),
            name: Some(name.to_string()),
            name_norm: Some(name_norm),
            path: normalized.display().to_string(),
            path_norm: normalized.key().to_string(),
            source: BookmarkSource::Explicit,
            pinned: false,
            tags: Vec::new(),
            desc: String::new(),
            workspace: None,
            created_at: now,
            last_visited: None,
            visit_count: Some(0),
            frecency_score: 0.0,
        });
        self.dirty_count += 1;
        self.invalidate_index();
        Ok(())
    }

    pub(crate) fn rename(&mut self, old: &str, new: &str) -> Result<(), StoreError> {
        let old_norm = normalize_name(old);
        let new_norm = normalize_name(new);
        if self
            .bookmarks
            .iter()
            .any(|b| b.source == BookmarkSource::Explicit && b.name_norm.as_deref() == Some(&new_norm))
        {
            return Err(StoreError::NameAlreadyExists(new.to_string()));
        }
        let bookmark = self
            .bookmarks
            .iter_mut()
            .find(|b| b.source == BookmarkSource::Explicit && b.name_norm.as_deref() == Some(&old_norm))
            .ok_or_else(|| StoreError::NotFound(old.to_string()))?;
        bookmark.name = Some(new.to_string());
        bookmark.name_norm = Some(new_norm);
        self.dirty_count += 1;
        self.invalidate_index();
        Ok(())
    }

    pub(crate) fn pin(&mut self, name: &str) -> Result<(), StoreError> {
        let name_norm = normalize_name(name);
        let bookmark = self
            .bookmarks
            .iter_mut()
            .find(|b| b.source == BookmarkSource::Explicit && b.name_norm.as_deref() == Some(&name_norm))
            .ok_or_else(|| StoreError::NotFound(name.to_string()))?;
        bookmark.pinned = true;
        self.dirty_count += 1;
        self.invalidate_index();
        Ok(())
    }

    pub(crate) fn unpin(&mut self, name: &str) -> Result<(), StoreError> {
        let name_norm = normalize_name(name);
        let bookmark = self
            .bookmarks
            .iter_mut()
            .find(|b| b.source == BookmarkSource::Explicit && b.name_norm.as_deref() == Some(&name_norm))
            .ok_or_else(|| StoreError::NotFound(name.to_string()))?;
        bookmark.pinned = false;
        self.dirty_count += 1;
        self.invalidate_index();
        Ok(())
    }

    pub(crate) fn set_explicit_metadata(
        &mut self,
        name: &str,
        tags: Vec<String>,
        desc: String,
    ) -> Result<(), StoreError> {
        let bookmark = self.find_explicit_mut(name)?;
        bookmark.tags = tags;
        bookmark.desc = desc;
        self.dirty_count += 1;
        self.invalidate_index();
        Ok(())
    }

    pub(crate) fn set_explicit_workspace(
        &mut self,
        name: &str,
        workspace: Option<String>,
    ) -> Result<(), StoreError> {
        let bookmark = self.find_explicit_mut(name)?;
        if bookmark.workspace == workspace {
            return Ok(());
        }
        bookmark.workspace = workspace;
        self.dirty_count += 1;
        self.invalidate_index();
        Ok(())
    }

    pub(crate) fn touch_explicit(&mut self, name: &str, now: u64) -> Result<(), StoreError> {
        let bookmark = self.find_explicit_mut(name)?;
        let current = bookmark.visit_count.unwrap_or(0);
        bookmark.visit_count = Some(current.saturating_add(1));
        bookmark.last_visited = Some(now);
        self.dirty_count += 1;
        Ok(())
    }

    pub(crate) fn delete_explicit(&mut self, name: &str) -> Result<(), StoreError> {
        let name_norm = normalize_name(name);
        let before = self.bookmarks.len();
        self.bookmarks.retain(|bookmark| {
            !(bookmark.source == BookmarkSource::Explicit
                && bookmark.name_norm.as_deref() == Some(&name_norm))
        });
        if self.bookmarks.len() == before {
            return Err(StoreError::NotFound(name.to_string()));
        }
        self.dirty_count += 1;
        self.invalidate_index();
        Ok(())
    }

    pub(crate) fn add_tags(&mut self, name: &str, tags: &[String]) -> Result<usize, StoreError> {
        let bookmark = self.find_explicit_mut(name)?;
        let mut added = 0usize;
        for tag in tags {
            if !bookmark.tags.iter().any(|existing| existing.eq_ignore_ascii_case(tag)) {
                bookmark.tags.push(tag.clone());
                added += 1;
            }
        }
        if added > 0 {
            self.dirty_count += 1;
            self.invalidate_index();
        }
        Ok(added)
    }

    pub(crate) fn remove_tags(&mut self, name: &str, tags: &[String]) -> Result<usize, StoreError> {
        let bookmark = self.find_explicit_mut(name)?;
        let before = bookmark.tags.len();
        bookmark
            .tags
            .retain(|tag| !tags.iter().any(|remove| remove.eq_ignore_ascii_case(tag)));
        let removed = before.saturating_sub(bookmark.tags.len());
        if removed > 0 {
            self.dirty_count += 1;
            self.invalidate_index();
        }
        Ok(removed)
    }

    pub(crate) fn rename_tag_globally(&mut self, old: &str, new: &str) -> usize {
        let mut changed = 0usize;
        for bookmark in &mut self.bookmarks {
            let mut updated = false;
            for tag in &mut bookmark.tags {
                if tag.eq_ignore_ascii_case(old) {
                    *tag = new.to_string();
                    updated = true;
                    changed += 1;
                }
            }
            if updated {
                let mut deduped = Vec::new();
                for tag in &bookmark.tags {
                    if !deduped.iter().any(|existing: &String| existing.eq_ignore_ascii_case(tag)) {
                        deduped.push(tag.clone());
                    }
                }
                bookmark.tags = deduped;
            }
        }
        if changed > 0 {
            self.dirty_count += 1;
            self.invalidate_index();
        }
        changed
    }

    pub(crate) fn learn(
        &mut self,
        raw_path: &str,
        cwd: &Path,
        home: Option<&Path>,
        now: u64,
    ) -> Result<(), StoreError> {
        let normalized = normalize_path_runtime(raw_path, cwd, home).map_err(StoreError::NormalizePath)?;
        if let Some(existing) = self
            .bookmarks
            .iter_mut()
            .find(|b| b.path_norm == normalized.key())
        {
            let current = existing.visit_count.unwrap_or(0);
            existing.visit_count = Some(current.saturating_add(1));
            existing.last_visited = Some(now);
            self.dirty_count += 1;
            self.invalidate_index();
            return Ok(());
        }

        self.bookmarks.push(Bookmark {
            id: Uuid::new_v4().to_string(),
            name: None,
            name_norm: None,
            path: normalized.display().to_string(),
            path_norm: normalized.key().to_string(),
            source: BookmarkSource::Learned,
            pinned: false,
            tags: Vec::new(),
            desc: String::new(),
            workspace: None,
            created_at: now,
            last_visited: Some(now),
            visit_count: Some(1),
            frecency_score: 1.0,
        });
        self.dirty_count += 1;
        self.invalidate_index();
        Ok(())
    }

    pub(crate) fn import_entry(
        &mut self,
        raw_path: &str,
        cwd: &Path,
        home: Option<&Path>,
        score: f64,
        now: u64,
    ) -> Result<(), StoreError> {
        let normalized = normalize_path_runtime(raw_path, cwd, home).map_err(StoreError::NormalizePath)?;
        if self
            .bookmarks
            .iter()
            .any(|b| b.source == BookmarkSource::Explicit && b.path_norm == normalized.key())
        {
            return Ok(());
        }
        self.bookmarks.push(Bookmark {
            id: Uuid::new_v4().to_string(),
            name: None,
            name_norm: None,
            path: normalized.display().to_string(),
            path_norm: normalized.key().to_string(),
            source: BookmarkSource::Imported,
            pinned: false,
            tags: Vec::new(),
            desc: String::new(),
            workspace: None,
            created_at: now,
            last_visited: None,
            visit_count: None,
            frecency_score: score,
        });
        self.dirty_count += 1;
        self.invalidate_index();
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn apply_aging(&mut self, max_age: f64) {
        let total: f64 = self
            .bookmarks
            .iter()
            .filter(|b| !is_aging_exempt(b))
            .map(|b| b.frecency_score)
            .sum();
        if total <= max_age || max_age <= 0.0 {
            return;
        }
        let factor = total / (0.9 * max_age);
        self.bookmarks.retain_mut(|bookmark| {
            if is_aging_exempt(bookmark) {
                return true;
            }
            bookmark.frecency_score /= factor;
            bookmark.frecency_score >= 1.0
        });
    }

    #[cfg(test)]
    pub(crate) fn record_visit(&mut self) {
        self.dirty_count += 1;
    }

    pub(crate) fn record_visit_by_id(&mut self, id: &str, now: u64) -> Result<(), StoreError> {
        let bookmark = self
            .bookmarks
            .iter_mut()
            .find(|bookmark| bookmark.id == id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        let current = bookmark.visit_count.unwrap_or(0);
        bookmark.visit_count = Some(current.saturating_add(1));
        bookmark.last_visited = Some(now);
        self.dirty_count += 1;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn should_flush_by_count(&self, threshold: u32) -> bool {
        self.dirty_count >= threshold
    }

    #[cfg(test)]
    pub(crate) fn should_flush_by_time(&self, now: u64, threshold_secs: u64) -> bool {
        self.last_save_at > 0 && now.saturating_sub(self.last_save_at) >= threshold_secs
    }

    fn find_explicit_mut(&mut self, name: &str) -> Result<&mut Bookmark, StoreError> {
        let name_norm = normalize_name(name);
        self.bookmarks
            .iter_mut()
            .find(|bookmark| {
                bookmark.source == BookmarkSource::Explicit
                    && bookmark.name_norm.as_deref() == Some(&name_norm)
            })
            .ok_or_else(|| StoreError::NotFound(name.to_string()))
    }

    pub(crate) fn index(&self) -> &BookmarkIndex {
        self.index.get_or_init(|| {
            if let Some(path) = self.storage_path.as_deref() {
                if let Some(index) = BookmarkIndex::maybe_load_persisted(path, &self.bookmarks) {
                    return index;
                }
                let index = BookmarkIndex::build(&self.bookmarks);
                let _ = BookmarkIndex::sync_persisted(path, &self.bookmarks);
                return index;
            }
            BookmarkIndex::build(&self.bookmarks)
        })
    }

    fn apply_runtime_aging(&mut self) {
        let max_age = crate::config::bookmark_max_age();
        if max_age == 0 {
            return;
        }
        let total: f64 = self
            .bookmarks
            .iter()
            .filter(|bookmark| !is_aging_exempt(bookmark))
            .map(|bookmark| bookmark.frecency_score)
            .sum();
        if total <= max_age as f64 {
            return;
        }
        let factor = total / (0.9 * max_age as f64);
        self.bookmarks.retain_mut(|bookmark| {
            if is_aging_exempt(bookmark) {
                return true;
            }
            bookmark.frecency_score /= factor;
            bookmark.frecency_score >= 1.0
        });
        self.invalidate_index();
    }

    fn invalidate_index(&mut self) {
        self.index = OnceLock::new();
    }

    fn remove_bookmark_by_id(&mut self, id: &str) -> Result<Bookmark, StoreError> {
        let index = self
            .bookmarks
            .iter()
            .position(|bookmark| bookmark.id == id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        Ok(self.bookmarks.remove(index))
    }

    fn replace_bookmark_by_id(&mut self, id: &str, bookmark: Bookmark) -> Result<(), StoreError> {
        let index = self
            .bookmarks
            .iter()
            .position(|entry| entry.id == id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        let mut bookmark = bookmark;
        normalize_loaded_bookmark(&mut bookmark);
        self.bookmarks[index] = bookmark;
        Ok(())
    }

    fn insert_bookmark_at(&mut self, index: usize, bookmark: Bookmark) {
        let mut bookmark = bookmark;
        normalize_loaded_bookmark(&mut bookmark);
        if let Some(existing) = self
            .bookmarks
            .iter()
            .position(|entry| entry.id == bookmark.id)
        {
            self.bookmarks.remove(existing);
        }
        let index = index.min(self.bookmarks.len());
        self.bookmarks.insert(index, bookmark);
    }
}

fn is_aging_exempt(bookmark: &Bookmark) -> bool {
    bookmark.source == BookmarkSource::Explicit || bookmark.pinned
}

fn default_source() -> BookmarkSource {
    BookmarkSource::Explicit
}

impl PersistedBookmark {
    fn from_bookmark(bookmark: &Bookmark) -> Self {
        Self {
            id: bookmark.id.clone(),
            name: bookmark.name.clone(),
            path: bookmark.path.clone(),
            source: (bookmark.source != BookmarkSource::Explicit).then_some(bookmark.source),
            pinned: bookmark.pinned,
            tags: bookmark.tags.clone(),
            desc: bookmark.desc.clone(),
            workspace: bookmark.workspace.clone(),
            created_at: bookmark.created_at,
            last_visited: bookmark.last_visited,
            visit_count: bookmark.visit_count,
            frecency_score: bookmark.frecency_score,
        }
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

fn is_zero_f64(value: &f64) -> bool {
    *value == 0.0
}

fn migrate_store_value(value: serde_json::Value) -> Result<StoreFile, StoreError> {
    match detect_schema_version(&value) {
        Some(version) if version > CURRENT_SCHEMA_VERSION => Err(StoreError::UnsupportedSchemaVersion(version)),
        Some(version) if version == CURRENT_SCHEMA_VERSION => {
            serde_json::from_value(value).map_err(StoreError::Json)
        }
        Some(version) => Err(StoreError::UnsupportedSchemaVersion(version)),
        None => migrate_legacy_value(value),
    }
}

fn migrate_legacy_value(value: serde_json::Value) -> Result<StoreFile, StoreError> {
    let entries = parse_legacy_entries(value).ok_or(StoreError::MissingSchemaVersion)?;
    Ok(StoreFile {
        schema_version: Some(CURRENT_SCHEMA_VERSION),
        bookmarks: entries
            .into_iter()
            .map(|(name, entry)| migrate_legacy_entry(name, entry))
            .collect(),
    })
}

fn migrate_legacy_entry(name: String, entry: LegacyEntry) -> Bookmark {
    Bookmark {
        id: Uuid::new_v4().to_string(),
        name: Some(name.clone()),
        name_norm: Some(normalize_name(&name)),
        path_norm: entry.path.to_ascii_lowercase(),
        path: entry.path,
        source: BookmarkSource::Explicit,
        pinned: false,
        tags: entry.tags,
        desc: String::new(),
        workspace: None,
        created_at: entry.last_visited,
        last_visited: Some(entry.last_visited).filter(|value| *value > 0),
        visit_count: Some(entry.visit_count),
        frecency_score: entry.visit_count.max(1) as f64,
    }
}

fn normalize_path_runtime(
    raw_path: &str,
    cwd: &Path,
    home: Option<&Path>,
) -> Result<crate::bookmark_core::NormalizedPath, NormalizePathError> {
    let normalized = normalize_path(raw_path, cwd, home)?;
    if !crate::config::bookmark_resolve_symlinks() {
        return Ok(normalized);
    }

    let canonical = match std::fs::canonicalize(normalized.display()) {
        Ok(path) => path,
        Err(_) => return Ok(normalized),
    };
    normalize_path(&canonical.to_string_lossy(), cwd, home)
}

fn normalize_loaded_bookmarks(bookmarks: &mut [Bookmark]) {
    for bookmark in bookmarks {
        normalize_loaded_bookmark(bookmark);
    }
}

fn normalize_loaded_bookmark(bookmark: &mut Bookmark) {
    if bookmark.id.trim().is_empty() {
        bookmark.id = Uuid::new_v4().to_string();
    }
    if bookmark.name_norm.is_none() {
        bookmark.name_norm = bookmark.name.as_deref().map(normalize_name);
    }
    if bookmark.path_norm.trim().is_empty() {
        bookmark.path_norm = bookmark.path.to_ascii_lowercase();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmark::cache::{CachePayload, CachedBookmark, SourceFingerprint, store_cache_path, write_cache_payload_atomic};
    use crate::bookmark::index::BookmarkIndex;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    fn cwd() -> PathBuf {
        PathBuf::from("C:/work")
    }

    fn cache_env_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn bookmark_roundtrip_with_new_fields() {
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        store.pin("foo").unwrap();
        let dir = tempdir().unwrap();
        let path = dir.path().join("bookmark.json");
        store.save(&path, 20).unwrap();

        let loaded = Store::load(&path).unwrap();
        assert_eq!(loaded.schema_version, 1);
        assert_eq!(loaded.bookmarks.len(), 1);
        assert_eq!(loaded.bookmarks[0].name.as_deref(), Some("foo"));
        assert!(loaded.bookmarks[0].pinned);
    }

    #[test]
    fn store_load_uses_binary_cache_when_metadata_matches() {
        let _guard = cache_env_guard();
        let dir = tempdir().unwrap();
        let path = dir.path().join("bookmark.json");
        fs::write(&path, br#"{"schema_version":1,"bookmarks":[]}"#).unwrap();

        let mut source_store = Store::new();
        source_store
            .set("foo", "C:/work/foo", &cwd(), None, 10)
            .unwrap();
        let fingerprint = SourceFingerprint::from_path(&path).unwrap();
        let payload = CachePayload {
            bookmarks: source_store
                .bookmarks
                .iter()
                .map(CachedBookmark::from_bookmark)
                .collect(),
            index: Some(BookmarkIndex::to_persisted(&source_store.bookmarks)),
        };
        write_cache_payload_atomic(
            &store_cache_path(&path),
            CURRENT_SCHEMA_VERSION,
            &fingerprint,
            &payload,
        )
        .unwrap();

        let loaded = Store::load(&path).unwrap();
        assert_eq!(loaded.bookmarks.len(), 1);
        assert_eq!(loaded.bookmarks[0].name.as_deref(), Some("foo"));
        assert_eq!(loaded.index().lookup_prefix("fo"), vec![0]);
    }

    #[test]
    fn store_load_falls_back_to_json_when_cache_missing() {
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        let dir = tempdir().unwrap();
        let path = dir.path().join("bookmark.json");
        store.save(&path, 20).unwrap();
        let _ = fs::remove_file(store_cache_path(&path));

        let loaded = Store::load(&path).unwrap();
        assert_eq!(loaded.bookmarks.len(), 1);
        assert_eq!(loaded.bookmarks[0].name.as_deref(), Some("foo"));
    }

    #[test]
    fn store_load_rebuilds_cache_after_json_fallback() {
        let _guard = cache_env_guard();
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        let dir = tempdir().unwrap();
        let path = dir.path().join("bookmark.json");
        store.save(&path, 20).unwrap();
        let cache = store_cache_path(&path);
        let _ = fs::remove_file(&cache);

        let _loaded = Store::load(&path).unwrap();
        assert!(cache.exists());
    }

    #[test]
    fn store_save_writes_json_before_cache() {
        let _guard = cache_env_guard();
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        let dir = tempdir().unwrap();
        let path = dir.path().join("bookmark.json");
        let cache = store_cache_path(&path);

        store.save(&path, 20).unwrap();
        assert!(path.exists());
        assert!(cache.exists());

        let json_meta = fs::metadata(&path).unwrap();
        let cache_meta = fs::metadata(&cache).unwrap();
        assert!(cache_meta.modified().unwrap() >= json_meta.modified().unwrap());
    }

    #[test]
    fn store_save_embeds_persisted_index_into_cache_payload() {
        let _guard = cache_env_guard();
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        let dir = tempdir().unwrap();
        let path = dir.path().join("bookmark.json");
        store.save(&path, 20).unwrap();

        let fingerprint = SourceFingerprint::from_path(&path).unwrap();
        let payload = crate::bookmark::cache::load_cache_payload_checked(
            &store_cache_path(&path),
            CURRENT_SCHEMA_VERSION,
            &fingerprint,
            None,
        )
        .unwrap()
        .unwrap();
        assert!(payload.index.is_some());
    }

    #[test]
    fn store_load_restores_index_from_cache_payload() {
        let _guard = cache_env_guard();
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        let dir = tempdir().unwrap();
        let path = dir.path().join("bookmark.json");
        store.save(&path, 20).unwrap();

        let loaded = Store::load(&path).unwrap();
        assert_eq!(loaded.index().lookup_prefix("fo"), vec![0]);
    }

    #[test]
    fn store_index_cache_hit_skips_rebuild() {
        let _guard = cache_env_guard();
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        let dir = tempdir().unwrap();
        let path = dir.path().join("bookmark.json");
        store.save(&path, 20).unwrap();

        let loaded = Store::load(&path).unwrap();
        let cache = store_cache_path(&path);
        let before = fs::metadata(&cache).unwrap().modified().unwrap();
        let _ = loaded.index().lookup_prefix("fo");
        let after = fs::metadata(&cache).unwrap().modified().unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn store_load_skips_binary_cache_when_env_disabled() {
        let _guard = cache_env_guard();
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        let dir = tempdir().unwrap();
        let path = dir.path().join("bookmark.json");
        store.save(&path, 20).unwrap();
        let cache = store_cache_path(&path);
        assert!(cache.exists());

        unsafe {
            std::env::set_var("XUN_BM_DISABLE_BINARY_CACHE", "1");
        }
        let loaded = Store::load(&path).unwrap();
        unsafe {
            std::env::remove_var("XUN_BM_DISABLE_BINARY_CACHE");
        }
        assert_eq!(loaded.bookmarks[0].name.as_deref(), Some("foo"));
    }

    #[test]
    fn cache_write_failure_does_not_fail_command() {
        let _guard = cache_env_guard();
        let env_dir = tempdir().unwrap();
        let path = env_dir.path().join("bookmark.json");
        let cache_parent = path.parent().unwrap().join(".xun.bookmark.cache");

        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        store.save(&path, 20).unwrap();

        let _ = fs::remove_file(&cache_parent);
        fs::create_dir_all(&cache_parent).unwrap();

        store.set("bar", "C:/work/bar", &cwd(), None, 30).unwrap();
        let result = store.save(&path, 40);
        assert!(result.is_ok());
        let loaded = Store::load(&path).unwrap();
        assert!(loaded.bookmarks.iter().any(|bookmark| bookmark.name.as_deref() == Some("bar")));
    }

    #[test]
    fn set_duplicate_name_updates_existing() {
        let mut store = Store::new();
        store.set("my-project", "C:/work/a", &cwd(), None, 10).unwrap();
        store.set("My-Project", "C:/work/b", &cwd(), None, 20).unwrap();
        assert_eq!(store.bookmarks.len(), 1);
        assert_eq!(store.bookmarks[0].path, "C:/work/b");
        assert_eq!(store.bookmarks[0].name_norm.as_deref(), Some("my-project"));
    }

    #[test]
    fn rename_to_existing_name_fails() {
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        store.set("bar", "C:/work/bar", &cwd(), None, 10).unwrap();
        let err = store.rename("foo", "bar").unwrap_err();
        assert!(matches!(err, StoreError::NameAlreadyExists(_)));
    }

    #[test]
    fn pin_sets_pinned_true() {
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        store.pin("foo").unwrap();
        assert!(store.bookmarks[0].pinned);
    }

    #[test]
    fn unpin_sets_pinned_false() {
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        store.pin("foo").unwrap();
        store.unpin("foo").unwrap();
        assert!(!store.bookmarks[0].pinned);
    }

    #[test]
    fn pin_nonexistent_bookmark_fails() {
        let mut store = Store::new();
        let err = store.pin("none").unwrap_err();
        assert!(matches!(err, StoreError::NotFound(_)));
    }

    #[test]
    fn import_creates_imported_bookmark() {
        let mut store = Store::new();
        store
            .import_entry("C:/work/imported", &cwd(), None, 50.0, 10)
            .unwrap();
        assert_eq!(store.bookmarks[0].source, BookmarkSource::Imported);
        assert_eq!(store.bookmarks[0].visit_count, None);
        assert_eq!(store.bookmarks[0].last_visited, None);
    }

    #[test]
    fn learn_creates_learned_bookmark() {
        let mut store = Store::new();
        store.learn("C:/work/learned", &cwd(), None, 10).unwrap();
        assert_eq!(store.bookmarks[0].source, BookmarkSource::Learned);
        assert_eq!(store.bookmarks[0].visit_count, Some(1));
    }

    #[test]
    fn learn_same_path_increments_visit_count() {
        let mut store = Store::new();
        store.learn("C:/work/learned", &cwd(), None, 10).unwrap();
        store.learn("C:/work/learned", &cwd(), None, 20).unwrap();
        assert_eq!(store.bookmarks.len(), 1);
        assert_eq!(store.bookmarks[0].visit_count, Some(2));
        assert_eq!(store.bookmarks[0].last_visited, Some(20));
    }

    #[test]
    fn learn_does_not_override_explicit() {
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        store.learn("C:/work/foo", &cwd(), None, 20).unwrap();
        assert_eq!(store.bookmarks.len(), 1);
        assert_eq!(store.bookmarks[0].source, BookmarkSource::Explicit);
        assert_eq!(store.bookmarks[0].visit_count, Some(1));
    }

    #[test]
    fn set_explicit_workspace_updates_and_clears_workspace() {
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();

        store
            .set_explicit_workspace("foo", Some("xunyu".to_string()))
            .unwrap();
        assert_eq!(store.bookmarks[0].workspace.as_deref(), Some("xunyu"));

        store.set_explicit_workspace("foo", None).unwrap();
        assert_eq!(store.bookmarks[0].workspace, None);
    }

    #[test]
    fn aging_keeps_explicit_and_pinned() {
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        store.learn("C:/work/learned", &cwd(), None, 10).unwrap();
        store.bookmarks[1].frecency_score = 20_000.0;
        store.pin("foo").unwrap();
        store.apply_aging(10_000.0);
        assert!(store
            .bookmarks
            .iter()
            .any(|bookmark| bookmark.source == BookmarkSource::Explicit));
    }

    #[test]
    fn dirty_save_triggered_after_n_accesses() {
        let mut store = Store::new();
        for _ in 0..50 {
            store.record_visit();
        }
        assert!(store.should_flush_by_count(50));
        assert!(!store.should_flush_by_count(51));
    }

    #[test]
    fn dirty_save_triggered_after_t_seconds() {
        let mut store = Store::new();
        store.last_save_at = 100;
        assert!(store.should_flush_by_time(700, 600));
        assert!(!store.should_flush_by_time(650, 600));
    }

    #[test]
    fn save_is_atomic() {
        let mut store = Store::new();
        store.set("foo", "C:/work/foo", &cwd(), None, 10).unwrap();
        let dir = tempdir().unwrap();
        let path = dir.path().join("bookmark.json");
        store.save(&path, 20).unwrap();
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn load_legacy_map_migrates_to_schema_v1() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        fs::write(
            &path,
            r#"{
  "home": {
    "path": "C:/work/home",
    "tags": ["work"],
    "visit_count": 2,
    "last_visited": 100
  }
}"#,
        )
        .unwrap();

        let loaded = Store::load(&path).unwrap();
        assert_eq!(loaded.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(loaded.bookmarks.len(), 1);
        assert_eq!(loaded.bookmarks[0].name.as_deref(), Some("home"));
        assert_eq!(loaded.bookmarks[0].source, BookmarkSource::Explicit);
        assert_eq!(loaded.bookmarks[0].visit_count, Some(2));
        assert_eq!(loaded.bookmarks[0].last_visited, Some(100));
    }

    #[test]
    fn load_unknown_missing_schema_shape_still_errors() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("invalid.json");
        fs::write(&path, r#"{"bookmarks":[]}"#).unwrap();

        let err = Store::load(&path).unwrap_err();
        assert!(matches!(err, StoreError::MissingSchemaVersion));
    }
}
