use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::common::{
    CreateFileW, DeviceIoControl, FILE_ATTRIBUTE_DIRECTORY, FILE_SHARE_READ, FILE_SHARE_WRITE,
    FSCTL_ENUM_USN_DATA, FSCTL_QUERY_USN_JOURNAL, GENERIC_READ, HandleGuard, INVALID_HANDLE_VALUE,
    MftEnumData, OPEN_EXISTING, PVoid, UsnJournalData, UsnRecordV2, frn_index, to_wide,
};
use super::path::{path_excluded, resolve_path};
use crate::commands::delete::progress::Progress;

pub(super) fn scan_volume(
    volume_root: &str,
    target_names: &HashSet<String>,
    match_all: bool,
    exclude_dirs: &HashSet<String>,
    progress: &Progress,
) -> Vec<PathBuf> {
    if !match_all && target_names.is_empty() {
        return Vec::new();
    }

    let drive_letter = volume_root.trim_end_matches(['\\', '/']);
    let device_path = format!("\\\\.\\{}", drive_letter);
    let wide = to_wide(&device_path);

    let h = unsafe {
        CreateFileW(
            wide.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            0,
        )
    };
    if h == INVALID_HANDLE_VALUE {
        ui_println!("USN scan: cannot open volume {}", device_path);
        return vec![];
    }
    let _hg = HandleGuard(h);

    let mut journal_data = UsnJournalData {
        usn_journal_id: 0,
        first_usn: 0,
        next_usn: i64::MAX,
        lowest_valid_usn: 0,
        max_usn: 0,
        maximum_size: 0,
        allocation_delta: 0,
    };
    let mut returned = 0;
    unsafe {
        DeviceIoControl(
            h,
            FSCTL_QUERY_USN_JOURNAL,
            std::ptr::null_mut(),
            0,
            &mut journal_data as *mut _ as PVoid,
            std::mem::size_of::<UsnJournalData>() as u32,
            &mut returned,
            std::ptr::null_mut(),
        );
    }
    let high_usn = if journal_data.next_usn > 0 {
        journal_data.next_usn
    } else {
        i64::MAX
    };

    let mut frn_map: HashMap<u64, (String, u64, bool)> = HashMap::with_capacity(200_000);

    let buf_size = 64 * 1024usize;
    let mut buf: Vec<u8> = vec![0u8; buf_size];

    let mut med = MftEnumData {
        start_file_reference_number: 0,
        low_usn: 0,
        high_usn,
    };

    loop {
        if crate::windows::ctrlc::is_cancelled() {
            break;
        }

        let mut bytes_returned = 0;
        let ok = unsafe {
            DeviceIoControl(
                h,
                FSCTL_ENUM_USN_DATA,
                &med as *const _ as PVoid,
                std::mem::size_of::<MftEnumData>() as u32,
                buf.as_mut_ptr() as PVoid,
                buf_size as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if ok == 0 {
            break;
        }

        if bytes_returned < 8 {
            break;
        }
        let next_start = unsafe { *(buf.as_ptr() as *const u64) };
        med.start_file_reference_number = next_start;

        let mut offset = 8usize;
        while offset + std::mem::size_of::<UsnRecordV2>() <= bytes_returned as usize {
            let rec = unsafe { &*(buf.as_ptr().add(offset) as *const UsnRecordV2) };
            let rec_len = rec.record_length as usize;
            if rec_len < std::mem::size_of::<UsnRecordV2>() {
                break;
            }

            let name_offset = offset + rec.file_name_offset as usize;
            let name_len = rec.file_name_length as usize;
            if name_offset + name_len <= bytes_returned as usize {
                let name_slice = unsafe {
                    std::slice::from_raw_parts(
                        buf.as_ptr().add(name_offset) as *const u16,
                        name_len / 2,
                    )
                };
                let name = String::from_utf16_lossy(name_slice);
                let is_dir = (rec.file_attributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
                let frn = frn_index(rec.file_reference_number);
                let parent = frn_index(rec.parent_file_ref_number);
                frn_map.insert(frn, (name, parent, is_dir));
                progress.inc_scanned();
            }

            offset += rec_len;
            if offset >= bytes_returned as usize {
                break;
            }
        }
    }

    let root_prefix = drive_letter.trim_end_matches(':');
    let mut results = Vec::new();

    for (&frn, (name, _parent_frn, is_dir)) in &frn_map {
        if *is_dir {
            continue;
        }
        if !match_all && !target_names.contains(&name.to_lowercase()) {
            continue;
        }

        if let Some(full_path) = resolve_path(frn, &frn_map, root_prefix)
            && !path_excluded(&full_path, exclude_dirs)
        {
            results.push(PathBuf::from(&full_path));
        }
    }

    results
}
