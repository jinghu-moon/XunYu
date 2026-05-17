use std::ffi::c_void;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};

use crate::runtime;

const OBJECT_NAME_INFORMATION_CLASS: u32 = 1;
const NTQUERYOBJECT_TIMEOUT: Duration = Duration::from_millis(200);
pub(super) const MAX_NTQUERYOBJECT_TIMEOUTS: usize = 8;
const MAX_ABANDONED_WORKERS_WARN: usize = 16;

#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *const u16,
}

#[repr(C)]
struct ObjectNameInformation {
    name: UnicodeString,
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQueryObject(
        handle: HANDLE,
        object_information_class: u32,
        object_information: *mut c_void,
        object_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

enum PathQueryCommand {
    Resolve {
        handle: usize,
        reply_tx: Sender<Option<String>>,
    },
    Shutdown,
}

pub(super) enum PathQueryResult {
    Resolved(Option<String>),
    TimedOut,
    WorkerFailed,
}

pub(super) struct NtPathResolver {
    tx: Sender<PathQueryCommand>,
    worker: Option<JoinHandle<()>>,
    abandoned_workers: usize,
}

impl NtPathResolver {
    pub(super) fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let worker = Some(Self::spawn_worker(rx));
        Self {
            tx,
            worker,
            abandoned_workers: 0,
        }
    }

    fn spawn_worker(rx: Receiver<PathQueryCommand>) -> JoinHandle<()> {
        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                match msg {
                    PathQueryCommand::Resolve { handle, reply_tx } => {
                        let handle = handle as HANDLE;
                        let result = query_handle_nt_path_blocking(handle);
                        unsafe {
                            CloseHandle(handle);
                        }
                        let _ = reply_tx.send(result);
                    }
                    PathQueryCommand::Shutdown => break,
                }
            }
        })
    }

    fn restart_worker(&mut self, timed_out: bool) {
        let (new_tx, new_rx) = mpsc::channel();
        let new_worker = Self::spawn_worker(new_rx);
        let old_tx = std::mem::replace(&mut self.tx, new_tx);
        let old_worker = self.worker.replace(new_worker);
        drop(old_tx);
        if let Some(worker) = old_worker {
            if timed_out {
                self.abandoned_workers = self.abandoned_workers.saturating_add(1);
                if self.abandoned_workers == MAX_ABANDONED_WORKERS_WARN && runtime::is_verbose() {
                    eprintln!(
                        "[WARN] lock query: NtQueryObject timed out repeatedly; abandoned {} worker threads",
                        self.abandoned_workers
                    );
                }
                drop(worker);
            } else {
                let _ = worker.join();
            }
        }
    }

    pub(super) fn query(&mut self, handle: HANDLE) -> PathQueryResult {
        let (reply_tx, reply_rx) = mpsc::channel();
        if self
            .tx
            .send(PathQueryCommand::Resolve {
                handle: handle as usize,
                reply_tx,
            })
            .is_err()
        {
            unsafe {
                CloseHandle(handle);
            }
            self.restart_worker(false);
            return PathQueryResult::WorkerFailed;
        }
        match reply_rx.recv_timeout(NTQUERYOBJECT_TIMEOUT) {
            Ok(path) => PathQueryResult::Resolved(path),
            Err(RecvTimeoutError::Timeout) => {
                self.restart_worker(true);
                PathQueryResult::TimedOut
            }
            Err(RecvTimeoutError::Disconnected) => {
                self.restart_worker(false);
                PathQueryResult::WorkerFailed
            }
        }
    }
}

impl Drop for NtPathResolver {
    fn drop(&mut self) {
        let _ = self.tx.send(PathQueryCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            // Avoid hanging drop if worker is blocked in NtQueryObject.
            if worker.is_finished() {
                let _ = worker.join();
            }
        }
    }
}

fn query_handle_nt_path_blocking(handle: HANDLE) -> Option<String> {
    let mut needed: u32 = 0;
    let _ = unsafe {
        NtQueryObject(
            handle,
            OBJECT_NAME_INFORMATION_CLASS,
            std::ptr::null_mut(),
            0,
            &mut needed,
        )
    };

    let mut cap = (needed as usize).max(4096);
    if cap > (1usize << 20) {
        cap = 1usize << 20;
    }
    let mut buf = vec![0u8; cap];
    let status = unsafe {
        NtQueryObject(
            handle,
            OBJECT_NAME_INFORMATION_CLASS,
            buf.as_mut_ptr() as *mut c_void,
            buf.len() as u32,
            &mut needed,
        )
    };
    if status < 0 {
        return None;
    }

    let info = unsafe { &*(buf.as_ptr() as *const ObjectNameInformation) };
    if info.name.buffer.is_null() || info.name.length == 0 {
        return None;
    }
    let chars = (info.name.length / 2) as usize;
    let slice = unsafe { std::slice::from_raw_parts(info.name.buffer, chars) };
    Some(super::device_map::normalize_path_like(
        &String::from_utf16_lossy(slice),
    ))
}
