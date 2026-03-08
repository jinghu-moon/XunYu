#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::c_void;
use core::mem::{size_of, zeroed};
use core::ptr::{null, null_mut};
use core::str;

use windows_sys::Win32::Foundation::{
    BOOL, CloseHandle, ERROR_ELEVATION_REQUIRED, GENERIC_READ, GetLastError, HANDLE,
    INVALID_HANDLE_VALUE, LocalFree,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_BEGIN, FILE_SHARE_READ, OPEN_EXISTING, ReadFile,
    SetFilePointerEx, WriteFile,
};
use windows_sys::Win32::System::Console::{
    CTRL_C_EVENT, FreeConsole, GetConsoleProcessList, GetStdHandle, STD_ERROR_HANDLE,
    SetConsoleCtrlHandler,
};
use windows_sys::Win32::System::Diagnostics::Debug::{
    IMAGE_SUBSYSTEM_WINDOWS_CUI, IMAGE_SUBSYSTEM_WINDOWS_GUI,
};
use windows_sys::Win32::System::Environment::GetCommandLineW;
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::System::SystemServices::{IMAGE_DOS_SIGNATURE, IMAGE_NT_SIGNATURE};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, ExitProcess, GetExitCodeProcess, INFINITE, PROCESS_INFORMATION, STARTUPINFOW,
    WaitForSingleObject,
};
use windows_sys::Win32::UI::Shell::{
    CommandLineToArgvW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
};

const MAX_PATH_W: usize = 32_768;
const MAX_SHIM_BYTES: usize = 16_384;
const MAX_CMDLINE_W: usize = 32_768;
const TRUE: BOOL = 1;
const FALSE: BOOL = 0;

const RUNAS_W: [u16; 6] = [
    b'r' as u16,
    b'u' as u16,
    b'n' as u16,
    b'a' as u16,
    b's' as u16,
    0,
];

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    unsafe { ExitProcess(1) }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    unsafe { shim_entry() }
}

unsafe fn shim_entry() -> ! {
    let mut module_path = [0u16; MAX_PATH_W];
    let module_len = GetModuleFileNameW(
        null_mut(),
        module_path.as_mut_ptr(),
        module_path.len() as u32,
    );
    if module_len == 0 || module_len as usize >= module_path.len() {
        fatal(1, b"GetModuleFileNameW failed", Some(GetLastError()));
    }

    let mut shim_path = WBuf::<MAX_PATH_W>::new();
    if !build_shim_path(&module_path, module_len as usize, &mut shim_path) {
        fatal(1, b"shim path overflow", None);
    }

    let mut shim_bytes = [0u8; MAX_SHIM_BYTES];
    let shim_size = match read_file_all(shim_path.as_ptr(), &mut shim_bytes) {
        Ok(n) => n,
        Err(err) => fatal(1, b"read .shim failed", Some(err)),
    };

    let shim_text = match str::from_utf8(&shim_bytes[..shim_size]) {
        Ok(s) => s,
        Err(_) => fatal(1, b".shim utf8 invalid", None),
    };

    let desc = match parse_shim(shim_text) {
        Some(v) => v,
        None => fatal(1, b".shim parse failed", None),
    };

    // 仅忽略本进程 Ctrl-C，避免可继承的 ConsoleFlags 方案。
    let _ = SetConsoleCtrlHandler(Some(ctrl_handler), TRUE);

    let mut argc: i32 = 0;
    let argv = CommandLineToArgvW(GetCommandLineW(), &mut argc);
    if argv.is_null() || argc <= 0 {
        fatal(1, b"CommandLineToArgvW failed", Some(GetLastError()));
    }

    let code = match desc.kind {
        ShimKind::Exe => run_exe(&desc, argv, argc),
        ShimKind::Cmd => run_cmd(&desc, argv, argc),
    };

    let _ = LocalFree(argv.cast());
    ExitProcess(code);
}

unsafe extern "system" fn ctrl_handler(ctrl_type: u32) -> BOOL {
    if ctrl_type == CTRL_C_EVENT {
        TRUE
    } else {
        FALSE
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShimKind {
    Exe,
    Cmd,
}

#[derive(Clone, Copy)]
struct ShimDesc<'a> {
    kind: ShimKind,
    path: &'a str,
    cmd: &'a str,
    args: Option<&'a str>,
    wait: Option<bool>,
    debug: bool,
}

fn parse_shim(text: &str) -> Option<ShimDesc<'_>> {
    let mut kind: Option<ShimKind> = None;
    let mut path = "";
    let mut cmd = "";
    let mut args: Option<&str> = None;
    let mut wait: Option<bool> = None;
    let mut debug = false;

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (k, v) = match line.split_once('=') {
            Some((k, v)) => (k.trim(), trim_optional_quotes(v.trim())),
            None => continue,
        };
        match k {
            "type" if eq_ignore_ascii_case(v, "exe") => kind = Some(ShimKind::Exe),
            "type" if eq_ignore_ascii_case(v, "cmd") => kind = Some(ShimKind::Cmd),
            "path" => path = v,
            "cmd" => cmd = v,
            "args" => {
                if !v.is_empty() {
                    args = Some(v);
                }
            }
            "wait" => wait = parse_bool(v),
            "debug" => debug = parse_bool(v).unwrap_or(false),
            _ => {}
        }
    }

    let kind = kind?;
    match kind {
        ShimKind::Exe if path.is_empty() => None,
        ShimKind::Cmd if cmd.is_empty() => None,
        _ => Some(ShimDesc {
            kind,
            path,
            cmd,
            args,
            wait,
            debug,
        }),
    }
}

fn trim_optional_quotes(v: &str) -> &str {
    if v.len() >= 2 {
        let b = v.as_bytes();
        if (b[0] == b'"' && b[v.len() - 1] == b'"') || (b[0] == b'\'' && b[v.len() - 1] == b'\'') {
            return &v[1..v.len() - 1];
        }
    }
    v
}

fn parse_bool(v: &str) -> Option<bool> {
    if eq_ignore_ascii_case(v, "true")
        || eq_ignore_ascii_case(v, "1")
        || eq_ignore_ascii_case(v, "yes")
    {
        Some(true)
    } else if eq_ignore_ascii_case(v, "false")
        || eq_ignore_ascii_case(v, "0")
        || eq_ignore_ascii_case(v, "no")
    {
        Some(false)
    } else {
        None
    }
}

fn eq_ignore_ascii_case(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .all(|(x, y)| x.eq_ignore_ascii_case(&y))
}

unsafe fn run_exe(desc: &ShimDesc<'_>, argv: *mut *mut u16, argc: i32) -> u32 {
    let mut app_path = WBuf::<MAX_PATH_W>::new();
    if !app_path.push_utf16(desc.path) || !app_path.terminate() {
        return 1;
    }

    let is_gui = read_pe_is_gui(app_path.as_ptr()).unwrap_or(false);
    let wait = desc.wait.unwrap_or(!is_gui);
    if is_gui && !wait {
        // If this shim owns a transient console by itself, detach it to avoid black-window flash
        // when launching GUI targets from non-console contexts.
        maybe_detach_own_console();
    }

    let mut params = WBuf::<MAX_CMDLINE_W>::new();
    if let Some(fixed) = desc.args {
        if !params.push_utf16(fixed) {
            return 1;
        }
    }
    append_user_args(&mut params, argv, argc, desc.args.is_some());
    let _ = params.terminate();

    let mut cmdline = WBuf::<MAX_CMDLINE_W>::new();
    if !append_quoted_wide_z(&mut cmdline, app_path.as_ptr()) {
        return 1;
    }
    if params.len > 0 {
        if !cmdline.push_u16(b' ' as u16) || !cmdline.push_wide_slice(params.as_slice_no_nul()) {
            return 1;
        }
    }
    if !cmdline.terminate() {
        return 1;
    }

    create_process_dispatch(
        Some(app_path.as_ptr()),
        cmdline.as_mut_ptr(),
        wait,
        desc.debug,
        Some(app_path.as_ptr()),
        if params.len > 0 {
            Some(params.as_ptr())
        } else {
            None
        },
    )
}

unsafe fn maybe_detach_own_console() {
    let mut pids = [0u32; 2];
    let n = GetConsoleProcessList(pids.as_mut_ptr(), pids.len() as u32);
    if n == 1 {
        let _ = FreeConsole();
    }
}

unsafe fn run_cmd(desc: &ShimDesc<'_>, argv: *mut *mut u16, argc: i32) -> u32 {
    let wait = desc.wait.unwrap_or(true);
    let mut cmdline = WBuf::<MAX_CMDLINE_W>::new();

    if !cmdline.push_ascii("cmd.exe /d /s /c ") || !cmdline.push_utf16(desc.cmd) {
        return 1;
    }

    if let Some(fixed) = desc.args {
        if !cmdline.push_u16(b' ' as u16) || !cmdline.push_utf16(fixed) {
            return 1;
        }
    }
    append_user_args(&mut cmdline, argv, argc, true);

    if !cmdline.terminate() {
        return 1;
    }

    create_process_dispatch(None, cmdline.as_mut_ptr(), wait, desc.debug, None, None)
}

unsafe fn create_process_dispatch(
    app_name: Option<*const u16>,
    cmdline: *mut u16,
    wait: bool,
    debug: bool,
    runas_file: Option<*const u16>,
    runas_params: Option<*const u16>,
) -> u32 {
    let mut si: STARTUPINFOW = zeroed();
    si.cb = size_of::<STARTUPINFOW>() as u32;
    let mut pi: PROCESS_INFORMATION = zeroed();

    let ok = CreateProcessW(
        app_name.unwrap_or(null()),
        cmdline,
        null(),
        null(),
        FALSE,
        0,
        null(),
        null(),
        &si,
        &mut pi,
    );

    if ok == 0 {
        let err = GetLastError();
        if err == ERROR_ELEVATION_REQUIRED {
            if let Some(file) = runas_file {
                return shell_execute_runas(file, runas_params, wait, debug);
            }
        }
        return 1;
    }

    if !pi.hThread.is_null() {
        let _ = CloseHandle(pi.hThread);
    }

    let job = if wait {
        setup_job_object(pi.hProcess, debug)
    } else {
        null_mut()
    };
    let code = if wait { wait_exit_code(pi.hProcess) } else { 0 };

    if !pi.hProcess.is_null() {
        let _ = CloseHandle(pi.hProcess);
    }
    if !job.is_null() {
        let _ = CloseHandle(job);
    }
    code
}

unsafe fn shell_execute_runas(
    file: *const u16,
    params: Option<*const u16>,
    wait: bool,
    _debug: bool,
) -> u32 {
    let mut sei: SHELLEXECUTEINFOW = zeroed();
    sei.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
    sei.fMask = SEE_MASK_NOCLOSEPROCESS;
    sei.lpVerb = RUNAS_W.as_ptr();
    sei.lpFile = file;
    sei.lpParameters = params.unwrap_or(null());
    sei.nShow = 1;

    if ShellExecuteExW(&mut sei) == 0 {
        return 1;
    }

    if !wait {
        return 0;
    }
    if sei.hProcess.is_null() || sei.hProcess == INVALID_HANDLE_VALUE {
        return 1;
    }
    let code = wait_exit_code(sei.hProcess);
    let _ = CloseHandle(sei.hProcess);
    code
}

unsafe fn setup_job_object(process: HANDLE, debug: bool) -> HANDLE {
    let job = CreateJobObjectW(null(), null());
    if job.is_null() || job == INVALID_HANDLE_VALUE {
        return null_mut();
    }

    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    let ok = SetInformationJobObject(
        job,
        JobObjectExtendedLimitInformation,
        (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast::<c_void>(),
        size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
    );
    if ok == 0 {
        let _ = CloseHandle(job);
        return null_mut();
    }

    let ok = AssignProcessToJobObject(job, process);
    if ok == 0 {
        let err = GetLastError();
        debug_log_code(debug, b"Job Object bind skipped", err);
        let _ = CloseHandle(job);
        return null_mut();
    }

    job
}

unsafe fn wait_exit_code(process: HANDLE) -> u32 {
    let _ = WaitForSingleObject(process, INFINITE);
    let mut code: u32 = 1;
    if GetExitCodeProcess(process, &mut code) == 0 {
        1
    } else {
        code
    }
}

unsafe fn read_pe_is_gui(path: *const u16) -> Result<bool, u32> {
    let file = CreateFileW(
        path,
        GENERIC_READ,
        FILE_SHARE_READ,
        null(),
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        null_mut(),
    );
    if file == INVALID_HANDLE_VALUE {
        return Err(GetLastError());
    }

    let mz = read_u16_at(file, 0)?;
    if mz != IMAGE_DOS_SIGNATURE {
        let _ = CloseHandle(file);
        return Err(1);
    }
    let e_lfanew = read_u32_at(file, 0x3c)? as i64;
    let sig = read_u32_at(file, e_lfanew)?;
    if sig != IMAGE_NT_SIGNATURE {
        let _ = CloseHandle(file);
        return Err(1);
    }
    let opt = e_lfanew + 4 + 20;
    let magic = read_u16_at(file, opt)?;
    if magic != 0x10b && magic != 0x20b {
        let _ = CloseHandle(file);
        return Err(1);
    }
    let subsystem = read_u16_at(file, opt + 68)?;
    let _ = CloseHandle(file);

    if subsystem == IMAGE_SUBSYSTEM_WINDOWS_GUI {
        Ok(true)
    } else if subsystem == IMAGE_SUBSYSTEM_WINDOWS_CUI {
        Ok(false)
    } else {
        Ok(false)
    }
}

unsafe fn read_u16_at(file: HANDLE, offset: i64) -> Result<u16, u32> {
    let mut b = [0u8; 2];
    read_exact_at(file, offset, &mut b)?;
    Ok(u16::from_le_bytes(b))
}

unsafe fn read_u32_at(file: HANDLE, offset: i64) -> Result<u32, u32> {
    let mut b = [0u8; 4];
    read_exact_at(file, offset, &mut b)?;
    Ok(u32::from_le_bytes(b))
}

unsafe fn read_exact_at(file: HANDLE, offset: i64, out: &mut [u8]) -> Result<(), u32> {
    if SetFilePointerEx(file, offset, null_mut(), FILE_BEGIN) == 0 {
        return Err(GetLastError());
    }
    let mut total = 0usize;
    while total < out.len() {
        let mut read = 0u32;
        let ok = ReadFile(
            file,
            out[total..].as_mut_ptr(),
            (out.len() - total) as u32,
            &mut read,
            null_mut(),
        );
        if ok == 0 {
            return Err(GetLastError());
        }
        if read == 0 {
            return Err(1);
        }
        total += read as usize;
    }
    Ok(())
}

unsafe fn read_file_all(path: *const u16, out: &mut [u8]) -> Result<usize, u32> {
    let file = CreateFileW(
        path,
        GENERIC_READ,
        FILE_SHARE_READ,
        null(),
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        null_mut(),
    );
    if file == INVALID_HANDLE_VALUE {
        return Err(GetLastError());
    }

    let mut total = 0usize;
    loop {
        if total == out.len() {
            let _ = CloseHandle(file);
            return Err(1);
        }
        let mut read = 0u32;
        let ok = ReadFile(
            file,
            out[total..].as_mut_ptr(),
            (out.len() - total) as u32,
            &mut read,
            null_mut(),
        );
        if ok == 0 {
            let err = GetLastError();
            let _ = CloseHandle(file);
            return Err(err);
        }
        if read == 0 {
            break;
        }
        total += read as usize;
    }

    let _ = CloseHandle(file);
    Ok(total)
}

fn build_shim_path(module: &[u16], module_len: usize, out: &mut WBuf<MAX_PATH_W>) -> bool {
    out.clear();
    if module_len == 0 || module_len > module.len() {
        return false;
    }
    if !out.push_wide_slice(&module[..module_len]) {
        return false;
    }

    let mut sep = 0usize;
    for i in 0..out.len {
        let ch = out.buf[i];
        if ch == b'\\' as u16 || ch == b'/' as u16 {
            sep = i + 1;
        }
    }
    let mut dot = None;
    for i in sep..out.len {
        if out.buf[i] == b'.' as u16 {
            dot = Some(i);
        }
    }
    if let Some(idx) = dot {
        out.len = idx;
    }
    out.push_ascii(".shim") && out.terminate()
}

unsafe fn append_user_args(
    buf: &mut WBuf<MAX_CMDLINE_W>,
    argv: *mut *mut u16,
    argc: i32,
    had_prefix: bool,
) {
    let mut need_space = had_prefix;
    let mut i = 1;
    while i < argc {
        let arg = *argv.add(i as usize);
        if !arg.is_null() {
            if need_space && !buf.push_u16(b' ' as u16) {
                return;
            }
            if !append_quoted_wide_z(buf, arg) {
                return;
            }
            need_space = true;
        }
        i += 1;
    }
}

unsafe fn append_quoted_wide_z(buf: &mut WBuf<MAX_CMDLINE_W>, ptr: *const u16) -> bool {
    let len = wcslen(ptr);
    if len == 0 {
        return buf.push_ascii("\"\"");
    }
    let mut needs_quote = false;
    let mut i = 0usize;
    while i < len {
        let c = *ptr.add(i);
        if c == b' ' as u16 || c == b'\t' as u16 || c == b'"' as u16 {
            needs_quote = true;
            break;
        }
        i += 1;
    }
    if !needs_quote {
        return buf.push_wide_slice(core::slice::from_raw_parts(ptr, len));
    }

    if !buf.push_u16(b'"' as u16) {
        return false;
    }
    let mut slash_count = 0usize;
    i = 0;
    while i < len {
        let c = *ptr.add(i);
        if c == b'\\' as u16 {
            slash_count += 1;
        } else if c == b'"' as u16 {
            for _ in 0..(slash_count * 2 + 1) {
                if !buf.push_u16(b'\\' as u16) {
                    return false;
                }
            }
            slash_count = 0;
            if !buf.push_u16(b'"' as u16) {
                return false;
            }
        } else {
            for _ in 0..slash_count {
                if !buf.push_u16(b'\\' as u16) {
                    return false;
                }
            }
            slash_count = 0;
            if !buf.push_u16(c) {
                return false;
            }
        }
        i += 1;
    }
    for _ in 0..(slash_count * 2) {
        if !buf.push_u16(b'\\' as u16) {
            return false;
        }
    }
    buf.push_u16(b'"' as u16)
}

unsafe fn wcslen(mut p: *const u16) -> usize {
    let mut len = 0usize;
    while *p != 0 {
        p = p.add(1);
        len += 1;
    }
    len
}

fn debug_log_code(enabled: bool, msg: &[u8], code: u32) {
    if !enabled {
        return;
    }
    write_stderr(b"[xun-alias-shim] ");
    write_stderr(msg);
    write_stderr(b": ");
    write_u32(code);
    write_stderr(b"\n");
}

fn fatal(exit_code: u32, msg: &[u8], code: Option<u32>) -> ! {
    write_stderr(b"[xun-alias-shim] ");
    write_stderr(msg);
    if let Some(v) = code {
        write_stderr(b": ");
        write_u32(v);
    }
    write_stderr(b"\n");
    unsafe { ExitProcess(exit_code) }
}

fn write_u32(mut n: u32) {
    let mut buf = [0u8; 10];
    let mut i = 0usize;
    if n == 0 {
        write_stderr(b"0");
        return;
    }
    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        i += 1;
        n /= 10;
    }
    while i > 0 {
        i -= 1;
        write_stderr(&buf[i..i + 1]);
    }
}

fn write_stderr(bytes: &[u8]) {
    unsafe {
        let h = GetStdHandle(STD_ERROR_HANDLE);
        if h.is_null() || h == INVALID_HANDLE_VALUE {
            return;
        }
        let mut written = 0u32;
        let _ = WriteFile(
            h,
            bytes.as_ptr().cast_mut(),
            bytes.len() as u32,
            &mut written,
            null_mut(),
        );
    }
}

struct WBuf<const N: usize> {
    buf: [u16; N],
    len: usize,
}

impl<const N: usize> WBuf<N> {
    const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }

    fn clear(&mut self) {
        self.len = 0;
    }

    fn as_ptr(&self) -> *const u16 {
        self.buf.as_ptr()
    }

    fn as_mut_ptr(&mut self) -> *mut u16 {
        self.buf.as_mut_ptr()
    }

    fn as_slice_no_nul(&self) -> &[u16] {
        &self.buf[..self.len]
    }

    fn push_u16(&mut self, v: u16) -> bool {
        if self.len + 1 >= N {
            return false;
        }
        self.buf[self.len] = v;
        self.len += 1;
        true
    }

    fn push_ascii(&mut self, s: &str) -> bool {
        for b in s.bytes() {
            if !self.push_u16(b as u16) {
                return false;
            }
        }
        true
    }

    fn push_utf16(&mut self, s: &str) -> bool {
        for c in s.encode_utf16() {
            if !self.push_u16(c) {
                return false;
            }
        }
        true
    }

    fn push_wide_slice(&mut self, s: &[u16]) -> bool {
        for c in s {
            if !self.push_u16(*c) {
                return false;
            }
        }
        true
    }

    fn terminate(&mut self) -> bool {
        if self.len >= N {
            return false;
        }
        self.buf[self.len] = 0;
        true
    }
}
