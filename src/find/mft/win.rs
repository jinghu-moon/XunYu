use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Component, Path, Prefix};

use windows_sys::Win32::Foundation::{GENERIC_READ, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Ioctl::{FSCTL_ENUM_USN_DATA, MFT_ENUM_DATA_V0, USN_RECORD_V2};

use super::types::{MftRecord, WcharPool};

pub(super) const NTFS_ROOT_FILE_REF: u64 = 5;
pub(super) const MFT_ENUM_BUFFER_SIZE: usize = 1024 * 1024;

pub(super) fn open_volume_handle(letter: u8) -> Option<windows_sys::Win32::Foundation::HANDLE> {
    let letter = (letter as char).to_ascii_uppercase();
    let vol = format!("\\\\.\\{letter}:");
    let wide = to_wide_null(OsStr::new(&vol));
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        None
    } else {
        Some(handle)
    }
}

pub(super) fn enumerate_mft(
    handle: windows_sys::Win32::Foundation::HANDLE,
) -> Option<(Vec<MftRecord>, WcharPool)> {
    let mut records: Vec<MftRecord> = Vec::with_capacity(800_000);
    let mut pool = WcharPool::new();
    pool.reserve(800_000 * 12);

    let mut med = MFT_ENUM_DATA_V0 {
        StartFileReferenceNumber: 0,
        LowUsn: 0,
        HighUsn: i64::MAX,
    };
    let mut buffer = vec![0u8; MFT_ENUM_BUFFER_SIZE];
    let mut bytes_returned = 0u32;

    loop {
        let ok = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_ENUM_USN_DATA,
                &mut med as *mut _ as *mut _,
                std::mem::size_of::<MFT_ENUM_DATA_V0>() as u32,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            break;
        }
        if bytes_returned <= 8 {
            break;
        }
        let next_ref = unsafe { *(buffer.as_ptr() as *const u64) };
        med.StartFileReferenceNumber = next_ref;

        let mut offset = std::mem::size_of::<u64>();
        let buffer_end = bytes_returned as usize;
        while offset < buffer_end {
            let record = unsafe { &*(buffer.as_ptr().add(offset) as *const USN_RECORD_V2) };
            if record.RecordLength == 0 {
                break;
            }
            if record.MajorVersion != 2 {
                return None;
            }
            let name_len = (record.FileNameLength / 2) as usize;
            let name_ptr = unsafe {
                (buffer.as_ptr().add(offset + record.FileNameOffset as usize)) as *const u16
            };
            let name_bytes = record.FileNameLength as usize;
            let name_end = offset
                .saturating_add(record.FileNameOffset as usize)
                .saturating_add(name_bytes);
            if name_end > buffer_end {
                return None;
            }
            let name_slice = unsafe { std::slice::from_raw_parts(name_ptr, name_len) };

            let file_ref = mask_file_ref(record.FileReferenceNumber);
            let parent_ref = mask_file_ref(record.ParentFileReferenceNumber);
            if file_ref == 0 || parent_ref == 0 {
                offset = offset.saturating_add(record.RecordLength as usize);
                continue;
            }

            let name_offset = pool.append(name_slice);
            records.push(MftRecord {
                file_ref,
                parent_ref,
                name_offset,
                name_len: name_len as u16,
                attrs: record.FileAttributes,
            });

            offset = offset.saturating_add(record.RecordLength as usize);
        }
    }

    Some((records, pool))
}

pub(super) fn mask_file_ref(value: u64) -> u64 {
    value & 0x0000_FFFF_FFFF_FFFF
}

pub(super) fn extract_drive_letter(path: &Path) -> Option<u8> {
    let mut components = path.components();
    match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => Some(letter),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn is_volume_root(path: &Path) -> bool {
    let mut components = path.components();
    matches!(
        (components.next(), components.next(), components.next()),
        (Some(Component::Prefix(_)), Some(Component::RootDir), None)
    )
}

pub(super) fn wide_to_string(input: &[u16]) -> String {
    OsString::from_wide(input).to_string_lossy().into_owned()
}

fn to_wide_null(s: &OsStr) -> Vec<u16> {
    let mut wide: Vec<u16> = s.encode_wide().collect();
    wide.push(0);
    wide
}
