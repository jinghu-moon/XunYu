use std::collections::HashMap;
use std::mem::offset_of;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_ACTION_ADDED, FILE_ACTION_MODIFIED, FILE_ACTION_REMOVED, FILE_ACTION_RENAMED_NEW_NAME,
    FILE_ACTION_RENAMED_OLD_NAME, FILE_NOTIFY_EXTENDED_INFORMATION,
};

pub fn parse_basic_events(buf: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    let mut pending_rename: Option<String> = None;

    // C layout: NextEntryOffset (4) + Action (4) + FileNameLength (4) + FileName (UTF-16 bytes...)
    const BASIC_HEADER: usize = 12;

    while offset.saturating_add(BASIC_HEADER) <= buf.len() {
        let next = match read_u32_le(buf, offset) {
            Some(v) => v,
            None => break,
        };
        let action = match read_u32_le(buf, offset + 4) {
            Some(v) => v,
            None => break,
        };
        let name_len = match read_u32_le(buf, offset + 8) {
            Some(v) => v as usize,
            None => break,
        };
        let name_start = offset + BASIC_HEADER;
        let name_end = name_start.saturating_add(name_len);
        if name_end > buf.len() {
            break;
        }

        let name = read_utf16_le_bytes(&buf[name_start..name_end]);

        match action {
            FILE_ACTION_RENAMED_OLD_NAME => pending_rename = Some(name),
            FILE_ACTION_RENAMED_NEW_NAME => {
                if let Some(old) = pending_rename.take() {
                    out.push(old);
                }
                out.push(name);
            }
            FILE_ACTION_ADDED | FILE_ACTION_MODIFIED | FILE_ACTION_REMOVED => out.push(name),
            _ => {}
        }

        if next == 0 {
            break;
        }
        if next < BASIC_HEADER as u32 {
            break;
        }
        offset = offset.saturating_add(next as usize);
    }

    out
}

pub fn parse_extended_events(buf: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    let mut pending_rename: Option<String> = None;

    // Use struct offsets instead of dereferencing unaligned pointers.
    const EXT_NAME_LEN_OFF: usize = offset_of!(FILE_NOTIFY_EXTENDED_INFORMATION, FileNameLength);
    const EXT_NAME_OFF: usize = offset_of!(FILE_NOTIFY_EXTENDED_INFORMATION, FileName);
    const EXT_MIN_HEADER: usize = EXT_NAME_OFF;

    while offset.saturating_add(EXT_MIN_HEADER) <= buf.len() {
        let next = match read_u32_le(buf, offset) {
            Some(v) => v,
            None => break,
        };
        let action = match read_u32_le(buf, offset + 4) {
            Some(v) => v,
            None => break,
        };
        let name_len = match read_u32_le(buf, offset + EXT_NAME_LEN_OFF) {
            Some(v) => v as usize,
            None => break,
        };
        let name_start = offset + EXT_NAME_OFF;
        let name_end = name_start.saturating_add(name_len);
        if name_end > buf.len() {
            break;
        }
        let name = read_utf16_le_bytes(&buf[name_start..name_end]);

        match action {
            FILE_ACTION_RENAMED_OLD_NAME => pending_rename = Some(name),
            FILE_ACTION_RENAMED_NEW_NAME => {
                if let Some(old) = pending_rename.take() {
                    out.push(old);
                }
                out.push(name);
            }
            FILE_ACTION_ADDED | FILE_ACTION_MODIFIED | FILE_ACTION_REMOVED => out.push(name),
            _ => {}
        }

        if next == 0 {
            break;
        }
        if next < EXT_MIN_HEADER as u32 {
            break;
        }
        offset = offset.saturating_add(next as usize);
    }

    out
}

pub fn normalize_path_key(raw: &str) -> String {
    let mut s = raw.trim().replace('\\', "/");
    while s.ends_with('/') {
        s.pop();
    }
    s.to_ascii_lowercase()
}

fn read_u32_le(buf: &[u8], offset: usize) -> Option<u32> {
    let bytes = buf.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_utf16_le_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let mut v = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        v.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    String::from_utf16_lossy(&v)
}

pub struct Debouncer {
    window_ms: u64,
    last_seen_ms: HashMap<String, u64>,
    path_by_key: HashMap<String, String>,
}

impl Debouncer {
    pub fn new(window_ms: u64) -> Self {
        Self {
            window_ms,
            last_seen_ms: HashMap::new(),
            path_by_key: HashMap::new(),
        }
    }

    pub fn push(&mut self, now_ms: u64, path: &str) {
        let key = normalize_path_key(path);
        self.last_seen_ms.insert(key.clone(), now_ms);
        self.path_by_key.insert(key, path.to_string());
    }

    pub fn flush_due(&mut self, now_ms: u64, limit: usize) -> Vec<String> {
        let mut due_keys: Vec<String> = self
            .last_seen_ms
            .iter()
            .filter_map(|(k, &t)| {
                if now_ms.saturating_sub(t) >= self.window_ms {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect();
        due_keys.sort();
        if limit > 0 && due_keys.len() > limit {
            due_keys.truncate(limit);
        }

        let mut out = Vec::new();
        for k in due_keys {
            self.last_seen_ms.remove(&k);
            if let Some(p) = self.path_by_key.remove(&k) {
                out.push(p);
            }
        }
        out
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.last_seen_ms.is_empty()
    }
}
