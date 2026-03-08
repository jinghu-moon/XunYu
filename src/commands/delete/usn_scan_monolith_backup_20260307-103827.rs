use std::collections::HashMap;
use std::ffi::c_void;
use std::path::PathBuf;

use super::progress::Progress;

type HANDLE = isize;
type DWORD = u32;
type BOOL = i32;
type PVOID = *mut c_void;

const GENERIC_READ: DWORD = 0x80000000;
const FILE_SHARE_READ: DWORD = 0x00000001;
const FILE_SHARE_WRITE: DWORD = 0x00000002;
const OPEN_EXISTING: DWORD = 3;
const INVALID_HANDLE_VALUE: HANDLE = -1;

const FSCTL_QUERY_USN_JOURNAL: DWORD = 0x000900F4;
const FSCTL_ENUM_USN_DATA: DWORD = 0x000900B3;

#[repr(C)]
struct MftEnumData {
    start_file_reference_number: u64,
    low_usn: i64,
    high_usn: i64,
}

#[repr(C)]
struct UsnJournalData {
    usn_journal_id: u64,
    first_usn: i64,
    next_usn: i64,
    lowest_valid_usn: i64,
    max_usn: i64,
    maximum_size: u64,
    allocation_delta: u64,
}

#[repr(C)]
struct UsnRecordV2 {
    record_length: u32,
    major_version: u16,
    minor_version: u16,
    file_reference_number: u64,
    parent_file_ref_number: u64,
    usn: i64,
    time_stamp: i64,
    reason: u32,
    source_info: u32,
    security_id: u32,
    file_attributes: u32,
    file_name_length: u16,
    file_name_offset: u16,
}

const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;

fn frn_index(frn: u64) -> u64 {
    frn & 0x0000_FFFF_FFFF_FFFF
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateFileW(
        lp: *const u16,
        access: DWORD,
        share: DWORD,
        sec: PVOID,
        disp: DWORD,
        flags: DWORD,
        tmpl: HANDLE,
    ) -> HANDLE;
    fn CloseHandle(h: HANDLE) -> BOOL;
    fn DeviceIoControl(
        device: HANDLE,
        code: DWORD,
        in_buf: PVOID,
        in_len: DWORD,
        out_buf: PVOID,
        out_len: DWORD,
        returned: *mut DWORD,
        overlapped: PVOID,
    ) -> BOOL;
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

struct HandleGuard(HANDLE);
impl Drop for HandleGuard {
    fn drop(&mut self) {
        if self.0 != 0 && self.0 != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.0) };
        }
    }
}

pub(crate) fn scan_volume(
    volume_root: &str,
    target_names: &std::collections::HashSet<String>,
    match_all: bool,
    exclude_dirs: &std::collections::HashSet<String>,
    progress: &Progress,
) -> Vec<PathBuf> {
    if !match_all && target_names.is_empty() {
        return Vec::new();
    }

    let drive_letter = volume_root.trim_end_matches(|c| c == '\\' || c == '/');
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
    let mut returned: DWORD = 0;
    unsafe {
        DeviceIoControl(
            h,
            FSCTL_QUERY_USN_JOURNAL,
            std::ptr::null_mut(),
            0,
            &mut journal_data as *mut _ as PVOID,
            std::mem::size_of::<UsnJournalData>() as DWORD,
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

        let mut bytes_returned: DWORD = 0;
        let ok = unsafe {
            DeviceIoControl(
                h,
                FSCTL_ENUM_USN_DATA,
                &med as *const _ as PVOID,
                std::mem::size_of::<MftEnumData>() as DWORD,
                buf.as_mut_ptr() as PVOID,
                buf_size as DWORD,
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

        if let Some(full_path) = resolve_path(frn, &frn_map, root_prefix) {
            if !path_excluded(&full_path, exclude_dirs) {
                results.push(PathBuf::from(&full_path));
            }
        }
    }

    results
}

fn resolve_path(frn: u64, map: &HashMap<u64, (String, u64, bool)>, drive: &str) -> Option<String> {
    let mut parts = Vec::new();
    let mut current = frn;
    let mut visited = std::collections::HashSet::new();

    loop {
        if visited.contains(&current) {
            return None;
        }
        visited.insert(current);

        match map.get(&current) {
            None => break,
            Some((name, parent, _)) => {
                parts.push(name.clone());
                if frn_index(*parent) == current {
                    break;
                }
                current = frn_index(*parent);
            }
        }

        if parts.len() > 64 {
            return None;
        }
    }

    parts.reverse();
    let mut path = format!("{}:\\", drive);
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            path.push('\\');
        }
        path.push_str(part);
    }
    Some(path)
}

fn path_excluded(path: &str, exclude_dirs: &std::collections::HashSet<String>) -> bool {
    let segments = path.split(|c| c == '\\' || c == '/').skip(1);
    for seg in segments {
        if exclude_dirs.contains(&seg.to_lowercase()) {
            return true;
        }
    }
    false
}

pub(crate) fn is_ntfs(root: &str) -> bool {
    let drive = root.trim_end_matches(|c| c == '\\' || c == '/');
    let root_path = format!("{}\\", drive);
    let wide = to_wide(&root_path);

    let mut fs_name = [0u16; 32];

    unsafe extern "system" {
        fn GetVolumeInformationW(
            root: *const u16,
            vol_name: *mut u16,
            vol_len: DWORD,
            serial: *mut DWORD,
            max_comp: *mut DWORD,
            flags: *mut DWORD,
            fs_name: *mut u16,
            fs_len: DWORD,
        ) -> BOOL;
    }

    let ok = unsafe {
        GetVolumeInformationW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            fs_name.as_mut_ptr(),
            fs_name.len() as DWORD,
        )
    };
    if ok == 0 {
        return false;
    }

    let name = String::from_utf16_lossy(&fs_name);
    name.starts_with("NTFS")
}
