use std::ffi::c_void;

pub(super) type Handle = isize;
pub(super) type Dword = u32;
pub(super) type Bool = i32;
pub(super) type PVoid = *mut c_void;

pub(super) const GENERIC_READ: Dword = 0x80000000;
pub(super) const FILE_SHARE_READ: Dword = 0x00000001;
pub(super) const FILE_SHARE_WRITE: Dword = 0x00000002;
pub(super) const OPEN_EXISTING: Dword = 3;
pub(super) const INVALID_HANDLE_VALUE: Handle = -1;

pub(super) const FSCTL_QUERY_USN_JOURNAL: Dword = 0x000900F4;
pub(super) const FSCTL_ENUM_USN_DATA: Dword = 0x000900B3;
pub(super) const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;

#[repr(C)]
pub(super) struct MftEnumData {
    pub(super) start_file_reference_number: u64,
    pub(super) low_usn: i64,
    pub(super) high_usn: i64,
}

#[repr(C)]
pub(super) struct UsnJournalData {
    pub(super) usn_journal_id: u64,
    pub(super) first_usn: i64,
    pub(super) next_usn: i64,
    pub(super) lowest_valid_usn: i64,
    pub(super) max_usn: i64,
    pub(super) maximum_size: u64,
    pub(super) allocation_delta: u64,
}

#[repr(C)]
pub(super) struct UsnRecordV2 {
    pub(super) record_length: u32,
    pub(super) major_version: u16,
    pub(super) minor_version: u16,
    pub(super) file_reference_number: u64,
    pub(super) parent_file_ref_number: u64,
    pub(super) usn: i64,
    pub(super) time_stamp: i64,
    pub(super) reason: u32,
    pub(super) source_info: u32,
    pub(super) security_id: u32,
    pub(super) file_attributes: u32,
    pub(super) file_name_length: u16,
    pub(super) file_name_offset: u16,
}

pub(super) fn frn_index(frn: u64) -> u64 {
    frn & 0x0000_FFFF_FFFF_FFFF
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub(super) fn CreateFileW(
        lp: *const u16,
        access: Dword,
        share: Dword,
        sec: PVoid,
        disp: Dword,
        flags: Dword,
        tmpl: Handle,
    ) -> Handle;
    pub(super) fn CloseHandle(h: Handle) -> Bool;
    pub(super) fn DeviceIoControl(
        device: Handle,
        code: Dword,
        in_buf: PVoid,
        in_len: Dword,
        out_buf: PVoid,
        out_len: Dword,
        returned: *mut Dword,
        overlapped: PVoid,
    ) -> Bool;
}

pub(super) fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(super) struct HandleGuard(pub(super) Handle);

impl Drop for HandleGuard {
    fn drop(&mut self) {
        if self.0 != 0 && self.0 != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.0) };
        }
    }
}
