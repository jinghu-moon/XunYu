#![cfg(all(windows, feature = "redirect"))]

#[path = "../src/commands/redirect/watch_core.rs"]
mod watch_core;

use std::mem::offset_of;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_ACTION_ADDED, FILE_ACTION_MODIFIED, FILE_ACTION_RENAMED_NEW_NAME,
    FILE_ACTION_RENAMED_OLD_NAME, FILE_NOTIFY_EXTENDED_INFORMATION,
};

fn utf16_le_bytes(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for u in s.encode_utf16() {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out
}

fn push_basic_record(buf: &mut Vec<u8>, action: u32, name: &str, has_next: bool) {
    let name_bytes = utf16_le_bytes(name);
    let mut len = 12 + name_bytes.len();
    while len % 4 != 0 {
        len += 1;
    }
    let next = if has_next { len as u32 } else { 0u32 };

    buf.extend_from_slice(&next.to_le_bytes());
    buf.extend_from_slice(&action.to_le_bytes());
    buf.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(&name_bytes);
    buf.resize(buf.len() + (len - 12 - name_bytes.len()), 0);
}

fn push_extended_record(buf: &mut Vec<u8>, action: u32, name: &str, has_next: bool) {
    let name_len_off = offset_of!(FILE_NOTIFY_EXTENDED_INFORMATION, FileNameLength);
    let name_off = offset_of!(FILE_NOTIFY_EXTENDED_INFORMATION, FileName);
    let name_bytes = utf16_le_bytes(name);

    let mut len = name_off + name_bytes.len();
    while len % 4 != 0 {
        len += 1;
    }
    let next = if has_next { len as u32 } else { 0u32 };

    let base = buf.len();
    buf.resize(base + len, 0);

    buf[base..base + 4].copy_from_slice(&next.to_le_bytes());
    buf[base + 4..base + 8].copy_from_slice(&action.to_le_bytes());
    buf[base + name_len_off..base + name_len_off + 4]
        .copy_from_slice(&(name_bytes.len() as u32).to_le_bytes());
    buf[base + name_off..base + name_off + name_bytes.len()].copy_from_slice(&name_bytes);
}

#[test]
fn parse_basic_events_traverses_and_pairs_rename() {
    let mut buf = Vec::new();
    push_basic_record(&mut buf, FILE_ACTION_RENAMED_OLD_NAME, "old.txt", true);
    push_basic_record(&mut buf, FILE_ACTION_RENAMED_NEW_NAME, "new.txt", true);
    push_basic_record(&mut buf, FILE_ACTION_ADDED, "a.jpg", true);
    push_basic_record(&mut buf, FILE_ACTION_MODIFIED, "b.md", false);

    let got = watch_core::parse_basic_events(&buf);
    assert_eq!(got, vec!["old.txt", "new.txt", "a.jpg", "b.md"]);
}

#[test]
fn parse_extended_events_reads_filename_by_offset() {
    let mut buf = Vec::new();
    push_extended_record(&mut buf, FILE_ACTION_ADDED, "a.jpg", true);
    push_extended_record(&mut buf, FILE_ACTION_MODIFIED, "b.md", false);

    let got = watch_core::parse_extended_events(&buf);
    assert_eq!(got, vec!["a.jpg", "b.md"]);
}

#[test]
fn debouncer_merges_and_flushes_with_limit() {
    let mut d = watch_core::Debouncer::new(100);

    d.push(0, r"C:\X\a.jpg");
    d.push(50, r"C:\X\a.jpg");
    assert!(d.flush_due(120, 10).is_empty());

    let got = d.flush_due(200, 10);
    assert_eq!(got, vec![r"C:\X\a.jpg".to_string()]);

    d.push(0, r"C:\X\c.jpg");
    d.push(0, r"C:\X\b.jpg");
    d.push(0, r"C:\X\a.jpg");

    let got = d.flush_due(200, 2);
    assert_eq!(got.len(), 2);
    assert!(!d.is_empty());
}
