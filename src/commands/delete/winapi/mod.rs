#![allow(non_snake_case)]

mod delete;
mod elevation;
mod guards;
mod handles;
mod ownership;
mod path;
mod privilege;
mod snapshot;
mod utils;

use std::ffi::c_void;

pub(crate) use delete::mark_delete_on_close;
pub(crate) use elevation::{is_elevated, relaunch_elevated};
pub(crate) use handles::force_close_external_handles;
pub(crate) use ownership::take_ownership_and_grant;
pub(crate) use path::{delete_file, get_attrs, get_last_error, is_dir_attr, set_normal_attrs};
pub(crate) use privilege::enable_delete_privileges;
pub(crate) use snapshot::handle_snapshot;

pub(super) type HANDLE = isize;
pub(super) type BOOL = i32;
pub(super) type DWORD = u32;
pub(super) type PCWSTR = *const u16;
pub(super) type PVOID = *mut c_void;
pub(super) type PSID = *mut c_void;
pub(super) type PACL = *mut c_void;

pub(super) const INVALID_HANDLE_VALUE: HANDLE = -1isize;

pub(super) const FILE_ATTRIBUTE_NORMAL: DWORD = 0x80;
pub(super) const FILE_ATTRIBUTE_DIRECTORY: DWORD = 0x10;

pub(super) const GENERIC_READ: DWORD = 0x80000000;
pub(super) const GENERIC_WRITE: DWORD = 0x40000000;
pub(super) const FILE_SHARE_READ: DWORD = 0x01;
pub(super) const FILE_SHARE_WRITE: DWORD = 0x02;
pub(super) const FILE_SHARE_DELETE: DWORD = 0x04;
pub(super) const OPEN_EXISTING: DWORD = 3;
pub(super) const FILE_FLAG_DELETE_ON_CLOSE: DWORD = 0x04000000;
pub(super) const FILE_FLAG_BACKUP_SEMANTICS: DWORD = 0x02000000;

pub(super) const TOKEN_QUERY: DWORD = 0x0008;
pub(super) const TOKEN_ADJUST_PRIVILEGES: DWORD = 0x0020;
pub(super) const SE_PRIVILEGE_ENABLED: DWORD = 0x00000002;
pub(super) const TOKEN_OWNER_CLASS: i32 = 4;
pub(super) const TOKEN_ELEVATION_CLASS: i32 = 20;

pub(super) const PROCESS_DUP_HANDLE: DWORD = 0x0040;
pub(super) const DUPLICATE_CLOSE_SOURCE: DWORD = 0x00000001;
pub(super) const DUPLICATE_SAME_ACCESS: DWORD = 0x00000002;

pub(super) const OWNER_SECURITY_INFORMATION: DWORD = 0x00000001;
pub(super) const DACL_SECURITY_INFORMATION: DWORD = 0x00000004;
pub(super) const SE_FILE_OBJECT: DWORD = 1;
pub(super) const ACL_REVISION: DWORD = 2;
pub(super) const GENERIC_ALL: DWORD = 0x10000000;

pub(super) const STATUS_INFO_LENGTH_MISMATCH: i32 = -1073741820; // 0xC0000004
pub(super) const FILE_DISPOSITION_INFORMATION_CLASS: i32 = 13;

#[repr(C)]
pub(super) struct IoStatusBlock {
    pub(super) status: u32,
    pub(super) information: usize,
}

#[repr(C)]
pub(super) struct FileDispositionInfo {
    pub(super) delete_file: i32,
}

#[repr(C)]
pub(super) struct Luid {
    pub(super) low_part: u32,
    pub(super) high_part: i32,
}

#[repr(C)]
pub(super) struct LuidAndAttributes {
    pub(super) luid: Luid,
    pub(super) attributes: DWORD,
}

#[repr(C)]
pub(super) struct TokenPrivileges {
    pub(super) count: DWORD,
    pub(super) priv0: LuidAndAttributes,
}

#[repr(C, packed(1))]
pub(crate) struct SysHandleEntry {
    pub(crate) owner_pid: u16,
    pub(crate) back_trace_idx: u16,
    pub(crate) object_type: u8,
    pub(crate) handle_flags: u8,
    pub(crate) handle_value: u16,
    pub(crate) object: usize,
    pub(crate) granted_access: u32,
}

#[repr(C)]
pub(super) struct SidIdentifierAuthority {
    pub(super) value: [u8; 6],
}

#[repr(C)]
pub(super) struct TokenOwner {
    pub(super) owner: PSID,
}

#[repr(C)]
pub(super) struct TokenElevation {
    pub(super) is_elevated: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub(super) fn DeleteFileW(lp: PCWSTR) -> BOOL;
    pub(super) fn SetFileAttributesW(lp: PCWSTR, attr: DWORD) -> BOOL;
    pub(super) fn GetFileAttributesW(lp: PCWSTR) -> DWORD;
    pub(super) fn CreateFileW(
        lp: PCWSTR,
        access: DWORD,
        share: DWORD,
        sec: PVOID,
        disp: DWORD,
        flags: DWORD,
        tmpl: HANDLE,
    ) -> HANDLE;
    pub(super) fn CloseHandle(h: HANDLE) -> BOOL;
    pub(super) fn OpenProcess(access: DWORD, inherit: BOOL, pid: DWORD) -> HANDLE;
    pub(super) fn DuplicateHandle(
        hsrc_proc: HANDLE,
        hsrc: HANDLE,
        htgt_proc: HANDLE,
        out: *mut HANDLE,
        access: DWORD,
        inherit: BOOL,
        opts: DWORD,
    ) -> BOOL;
    pub(super) fn GetFinalPathNameByHandleW(
        h: HANDLE,
        buf: *mut u16,
        len: DWORD,
        flags: DWORD,
    ) -> DWORD;
    pub(super) fn GetCurrentProcess() -> HANDLE;
    pub(super) fn GetCurrentProcessId() -> DWORD;
}

#[link(name = "ntdll")]
unsafe extern "system" {
    pub(super) fn NtSetInformationFile(
        h: HANDLE,
        iosb: *mut IoStatusBlock,
        info: PVOID,
        len: DWORD,
        class: i32,
    ) -> i32;
    pub(super) fn NtQuerySystemInformation(
        class: i32,
        info: PVOID,
        len: DWORD,
        ret: *mut DWORD,
    ) -> i32;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    pub(super) fn OpenProcessToken(proc: HANDLE, access: DWORD, out: *mut HANDLE) -> BOOL;
    pub(super) fn LookupPrivilegeValueW(sys: PCWSTR, name: PCWSTR, luid: *mut Luid) -> BOOL;
    pub(super) fn AdjustTokenPrivileges(
        tok: HANDLE,
        disable: BOOL,
        new: *const TokenPrivileges,
        len: DWORD,
        prev: PVOID,
        ret: *mut DWORD,
    ) -> BOOL;
    pub(super) fn GetTokenInformation(
        tok: HANDLE,
        class: i32,
        info: PVOID,
        len: DWORD,
        ret: *mut DWORD,
    ) -> BOOL;
    pub(super) fn AllocateAndInitializeSid(
        auth: *const SidIdentifierAuthority,
        n: u8,
        s0: DWORD,
        s1: DWORD,
        s2: DWORD,
        s3: DWORD,
        s4: DWORD,
        s5: DWORD,
        s6: DWORD,
        s7: DWORD,
        out: *mut PSID,
    ) -> BOOL;
    pub(super) fn FreeSid(sid: PSID) -> PVOID;
    pub(super) fn InitializeAcl(acl: PACL, len: DWORD, rev: DWORD) -> BOOL;
    pub(super) fn AddAccessAllowedAce(acl: PACL, rev: DWORD, mask: DWORD, sid: PSID) -> BOOL;
    pub(super) fn SetNamedSecurityInfoW(
        name: PCWSTR,
        obj_type: DWORD,
        info: DWORD,
        owner: PSID,
        group: PSID,
        dacl: PACL,
        sacl: PACL,
    ) -> DWORD;
}

#[link(name = "shell32")]
unsafe extern "system" {
    pub(super) fn ShellExecuteW(
        hwnd: HANDLE,
        op: PCWSTR,
        file: PCWSTR,
        params: PCWSTR,
        dir: PCWSTR,
        show_cmd: i32,
    ) -> HANDLE;
}
