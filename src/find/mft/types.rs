#[derive(Clone)]
pub(super) struct DfsEntry {
    pub(super) dir_ref: u64,
    pub(super) rel_prefix: String,
    pub(super) inherited: super::super::rules::RuleKind,
    pub(super) depth: i32,
}

#[derive(Clone, Copy)]
pub(super) struct MftRecord {
    pub(super) file_ref: u64,
    pub(super) parent_ref: u64,
    pub(super) name_offset: u32,
    pub(super) name_len: u16,
    pub(super) attrs: u32,
}

pub(super) struct WcharPool {
    data: Vec<u16>,
}

impl WcharPool {
    pub(super) fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub(super) fn reserve(&mut self, chars: usize) {
        self.data.reserve(chars);
    }

    pub(super) fn append(&mut self, src: &[u16]) -> u32 {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(src);
        offset
    }

    pub(super) fn slice(&self, offset: u32, len: u16) -> &[u16] {
        let start = offset as usize;
        let end = start + len as usize;
        &self.data[start..end]
    }
}

#[derive(Clone, Copy)]
struct ChildEntry {
    parent_ref: u64,
    record_idx: u32,
}

pub(super) struct ChildrenIndex {
    entries: Vec<ChildEntry>,
}

impl ChildrenIndex {
    pub(super) fn build(records: &[MftRecord]) -> Self {
        let mut entries = Vec::with_capacity(records.len());
        for (idx, rec) in records.iter().enumerate() {
            entries.push(ChildEntry {
                parent_ref: rec.parent_ref,
                record_idx: idx as u32,
            });
        }
        entries.sort_by(|a, b| a.parent_ref.cmp(&b.parent_ref));
        Self { entries }
    }

    pub(super) fn find_range(&self, parent_ref: u64) -> (usize, usize) {
        let key = ChildEntry {
            parent_ref,
            record_idx: 0,
        };
        let lo = self
            .entries
            .partition_point(|e| e.parent_ref < key.parent_ref);
        let hi = self
            .entries
            .partition_point(|e| e.parent_ref <= key.parent_ref);
        (lo, hi)
    }

    pub(super) fn is_empty(&self, parent_ref: u64) -> bool {
        let (lo, hi) = self.find_range(parent_ref);
        lo == hi
    }

    pub(super) fn record_index_at(&self, idx: usize) -> usize {
        self.entries[idx].record_idx as usize
    }
}
