use crate::output::can_interact;
use crate::windows::ctrlc::{install_ctrlc_handler_once, is_cancelled, reset_cancelled};

use super::super::fs_utils::wide;

use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use windows_sys::Win32::Foundation::{ERROR_NOT_SAME_DEVICE, GetLastError};
use windows_sys::Win32::Storage::FileSystem::{
    CopyFileExW, MOVEFILE_COPY_ALLOWED, MOVEFILE_WRITE_THROUGH, MoveFileWithProgressW,
};

const COPY_FILE_FAIL_IF_EXISTS: u32 = 1;
const COPY_FILE_NO_BUFFERING: u32 = 0x0000_1000;
const BIG_FILE_NO_BUFFERING_THRESHOLD: u64 = 8 * 1024 * 1024;
const PROGRESS_CONTINUE: u32 = 0;
const PROGRESS_CANCEL: u32 = 1;

type CopyProgressRoutine = unsafe extern "system" fn(
    i64,
    i64,
    i64,
    i64,
    u32,
    u32,
    *mut core::ffi::c_void,
    *mut core::ffi::c_void,
    *const core::ffi::c_void,
) -> u32;

pub(crate) fn copy_file(src: &Path, dst: &Path) -> Result<Option<String>, String> {
    let src_w = wide(src);
    let dst_w = wide(dst);
    let mut flags = COPY_FILE_FAIL_IF_EXISTS;
    let size = std::fs::metadata(src).map(|m| m.len()).unwrap_or(0);
    if size >= BIG_FILE_NO_BUFFERING_THRESHOLD {
        flags |= COPY_FILE_NO_BUFFERING;
    }

    install_ctrlc_handler_once();
    reset_cancelled();
    let progress = make_progress_ctx(size, "copy");
    let routine = if progress.is_some() {
        Some(copy_progress_routine as CopyProgressRoutine)
    } else {
        None
    };
    let progress_ptr: *const core::ffi::c_void = progress
        .as_ref()
        .map(|b| (&**b as *const ProgressCtx).cast::<core::ffi::c_void>())
        .unwrap_or(std::ptr::null());
    let ok = unsafe {
        CopyFileExW(
            src_w.as_ptr(),
            dst_w.as_ptr(),
            routine,
            progress_ptr,
            std::ptr::null_mut(),
            flags,
        )
    };
    if let Some(ctx) = &progress {
        ctx.pb.finish_and_clear();
    }
    if ok == 0 {
        let err = unsafe { GetLastError() };
        if is_cancelled() {
            return Err("cancelled".to_string());
        }
        return Err(format!("copy_failed:os={err}"));
    }
    Ok(None)
}

pub(crate) fn move_file(src: &Path, dst: &Path) -> Result<Option<String>, String> {
    match std::fs::rename(src, dst) {
        Ok(_) => Ok(None),
        Err(e) => {
            if is_probably_long_path(src) || is_probably_long_path(dst) {
                return move_file_cross_volume(src, dst);
            }
            if e.raw_os_error() == Some(ERROR_NOT_SAME_DEVICE as i32) {
                move_file_cross_volume(src, dst)
            } else {
                Err(format!("rename_failed:{e}"))
            }
        }
    }
}

fn move_file_cross_volume(src: &Path, dst: &Path) -> Result<Option<String>, String> {
    let src_w = wide(src);
    let dst_w = wide(dst);
    let size = std::fs::metadata(src).map(|m| m.len()).unwrap_or(0);
    install_ctrlc_handler_once();
    reset_cancelled();
    let progress = make_progress_ctx(size, "move");
    let routine = if progress.is_some() {
        Some(copy_progress_routine as CopyProgressRoutine)
    } else {
        None
    };
    let progress_ptr: *const core::ffi::c_void = progress
        .as_ref()
        .map(|b| (&**b as *const ProgressCtx).cast::<core::ffi::c_void>())
        .unwrap_or(std::ptr::null());
    let ok = unsafe {
        MoveFileWithProgressW(
            src_w.as_ptr(),
            dst_w.as_ptr(),
            routine,
            progress_ptr,
            MOVEFILE_COPY_ALLOWED | MOVEFILE_WRITE_THROUGH,
        )
    };
    if let Some(ctx) = &progress {
        ctx.pb.finish_and_clear();
    }
    if ok == 0 {
        let err = unsafe { GetLastError() };
        if is_cancelled() {
            return Err("cancelled".to_string());
        }
        if dst.exists() && src.exists() {
            return Ok(Some(format!("copy_ok_delete_failed:os={err}")));
        }
        return Err(format!("move_failed:os={err}"));
    }
    Ok(None)
}

fn is_probably_long_path(path: &Path) -> bool {
    path.to_string_lossy().len() >= 240
}

struct ProgressCtx {
    pb: ProgressBar,
    last: AtomicU64,
}

fn make_progress_ctx(total: u64, label: &str) -> Option<Box<ProgressCtx>> {
    if !can_interact() || total < BIG_FILE_NO_BUFFERING_THRESHOLD {
        return None;
    }
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:30}] {bytes}/{total_bytes} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("=>-"),
    );
    pb.set_message(label.to_string());
    Some(Box::new(ProgressCtx {
        pb,
        last: AtomicU64::new(0),
    }))
}

unsafe extern "system" fn copy_progress_routine(
    total_file_size: i64,
    total_bytes_transferred: i64,
    _stream_size: i64,
    _stream_bytes_transferred: i64,
    _stream_number: u32,
    _callback_reason: u32,
    _source_file: *mut core::ffi::c_void,
    _destination_file: *mut core::ffi::c_void,
    lp_data: *const core::ffi::c_void,
) -> u32 {
    if is_cancelled() {
        return PROGRESS_CANCEL;
    }
    if lp_data.is_null() {
        return PROGRESS_CONTINUE;
    }
    let ctx = unsafe { &*(lp_data as *const ProgressCtx) };
    let total = total_file_size.max(0) as u64;
    if total > 0 {
        ctx.pb.set_length(total);
    }
    let pos = total_bytes_transferred.max(0) as u64;
    let prev = ctx.last.swap(pos, Ordering::Relaxed);
    if pos != prev {
        ctx.pb.set_position(pos);
    }
    PROGRESS_CONTINUE
}
