#![cfg(windows)]
#![allow(dead_code)]

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct TestEnv {
    pub root: PathBuf,
}

static TEST_ENV_SEQ: AtomicU64 = AtomicU64::new(0);

impl TestEnv {
    pub fn new() -> Self {
        for _ in 0..32 {
            let mut root = std::env::temp_dir();
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let seq = TEST_ENV_SEQ.fetch_add(1, Ordering::Relaxed);
            root.push(format!("xun-test-{}-{nanos}-{seq}", std::process::id()));
            if fs::create_dir(&root).is_ok() {
                return Self { root };
            }
        }
        panic!("failed to create unique test env directory");
    }

    pub fn cmd(&self) -> Command {
        let exe = env!("CARGO_BIN_EXE_xun");
        let mut c = Command::new(exe);
        let _ = fs::create_dir_all(&self.root);
        c.env("_BM_DATA_FILE", self.root.join(".xun.bookmark.json"));
        c.env("USERPROFILE", &self.root);
        c.env("HOME", &self.root);
        c.env("XUN_NON_INTERACTIVE", "1");
        // Prevent lock query from hanging tests on some machines.
        c.env("XUN_LOCK_QUERY_TIMEOUT_MS", "5000");
        c.env_remove("XUN_UI");
        c
    }

    pub fn audit_path(&self) -> PathBuf {
        self.root.join("audit.jsonl")
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn run_ok(cmd: &mut Command) -> Output {
    let out = cmd.output().unwrap();
    if !out.status.success() {
        panic!(
            "command failed: {}\nstderr: {}\nstdout: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
    }
    out
}

pub fn run_raw(cmd: &mut Command) -> Output {
    cmd.output().unwrap()
}

pub fn run_err(cmd: &mut Command) -> Output {
    let out = cmd.output().unwrap();
    if out.status.success() {
        panic!(
            "command unexpectedly succeeded:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    out
}

pub fn run_ok_status(cmd: &mut Command) {
    let status = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    if !status.success() {
        panic!("command failed: {}", status);
    }
}

pub fn env_u64(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|v| v.parse::<u64>().ok())
}

pub fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

pub fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(default)
}

pub fn assert_under_ms(label: &str, elapsed: Duration, key: &str) {
    if let Some(max_ms) = env_u64(key) {
        let ms = elapsed.as_millis() as u64;
        assert!(ms <= max_ms, "{label} took {ms}ms > {max_ms}ms");
    }
}

pub fn write_mixed_files(root: &Path, total: usize, bytes_per_file: usize, per_dir: usize) {
    let payload = vec![b'x'; bytes_per_file.max(1)];
    let short_root = root.join("short");
    let long_root = root
        .join("long_path")
        .join("segment01_abcdefghijklmnopqrstuvwxyz")
        .join("segment02_ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .join("segment03_unicode")
        .join("segment04_1234567890abcdef");

    fs::create_dir_all(&short_root).unwrap();
    fs::create_dir_all(&long_root).unwrap();

    let per_dir = per_dir.max(1);
    let mut current_bucket = usize::MAX;
    let mut short_dir = short_root.clone();
    let mut long_dir = long_root.clone();

    for i in 0..total {
        let bucket = i / per_dir;
        if bucket != current_bucket {
            current_bucket = bucket;
            short_dir = short_root.join(format!("s{:04}", bucket));
            long_dir = long_root.join(format!("l{:04}", bucket));
            fs::create_dir_all(&short_dir).unwrap();
            fs::create_dir_all(&long_dir).unwrap();
        }

        let base = match i % 3 {
            0 => format!("a{:06}", i),
            1 => format!("u{:06}", i),
            _ => format!("mix{:06}_u", i),
        };
        let name = if i % 2 == 0 {
            format!(
                "{}_abcdefghijklmnopqrstuvwxyz_ABCDEFGHIJKLMNOPQRSTUVWXYZ_mixed.txt",
                base
            )
        } else {
            format!("{}.txt", base)
        };

        let dir = if i % 2 == 0 { &long_dir } else { &short_dir };
        let path = dir.join(name);
        fs::write(path, &payload).unwrap();
    }
}

#[derive(Clone)]
pub struct HeavyFsInfo {
    pub root: PathBuf,
    pub files: usize,
    pub bytes: usize,
    pub per_dir: usize,
}

const HEAVY_TESTS_TOTAL: usize = 2;

static HEAVY_FS: OnceLock<HeavyFsInfo> = OnceLock::new();
static HEAVY_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static HEAVY_DONE: OnceLock<Mutex<usize>> = OnceLock::new();

pub fn heavy_lock() -> std::sync::MutexGuard<'static, ()> {
    HEAVY_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

pub fn heavy_fs_info() -> HeavyFsInfo {
    HEAVY_FS
        .get_or_init(|| {
            let files = env_usize("XUN_TEST_HEAVY_FILES", 1_000_000);
            let bytes = env_usize("XUN_TEST_HEAVY_FILE_BYTES", 64);
            let per_dir = env_usize("XUN_TEST_HEAVY_FILES_PER_DIR", 1000);

            let mut root = std::env::temp_dir();
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            root.push(format!("xun-heavy-{}-{}", std::process::id(), nanos));
            fs::create_dir_all(&root).unwrap();

            eprintln!(
                "perf: prepare heavy fs root={} files={} bytes={} per_dir={}",
                root.display(),
                files,
                bytes,
                per_dir
            );
            let start = Instant::now();
            write_mixed_files(&root, files, bytes, per_dir);
            eprintln!(
                "perf: heavy fs prepared in {} ms",
                start.elapsed().as_millis()
            );

            HeavyFsInfo {
                root,
                files,
                bytes,
                per_dir,
            }
        })
        .clone()
}

pub fn heavy_mark_done(info: &HeavyFsInfo) {
    if env_bool("XUN_TEST_HEAVY_KEEP", false) {
        return;
    }
    let mut done = HEAVY_DONE.get_or_init(|| Mutex::new(0)).lock().unwrap();
    *done += 1;
    let should_clean = *done >= HEAVY_TESTS_TOTAL;
    drop(done);
    if should_clean {
        let _ = fs::remove_dir_all(&info.root);
        eprintln!("perf: heavy fs cleaned root={}", info.root.display());
    }
}

pub struct HeavyTestGuard {
    pub info: HeavyFsInfo,
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl HeavyTestGuard {
    pub fn new() -> Self {
        let lock = heavy_lock();
        let info = heavy_fs_info();
        Self { info, _lock: lock }
    }
}

impl Drop for HeavyTestGuard {
    fn drop(&mut self) {
        heavy_mark_done(&self.info);
    }
}

pub fn handle_count() -> u32 {
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};
    let mut count = 0u32;
    let ok = unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) };
    if ok == 0 { 0 } else { count }
}

pub fn working_set_bytes() -> u64 {
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let ok = GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters as *mut _ as *mut _,
            counters.cb,
        );
        if ok == 0 {
            0
        } else {
            counters.WorkingSetSize as u64
        }
    }
}

fn filetime_to_u64(ft: windows_sys::Win32::Foundation::FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

fn process_cpu_time_100ns(handle: windows_sys::Win32::Foundation::HANDLE) -> u64 {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::GetProcessTimes;
    unsafe {
        let mut creation: FILETIME = std::mem::zeroed();
        let mut exit: FILETIME = std::mem::zeroed();
        let mut kernel: FILETIME = std::mem::zeroed();
        let mut user: FILETIME = std::mem::zeroed();
        let ok = GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user);
        if ok == 0 {
            0
        } else {
            filetime_to_u64(kernel) + filetime_to_u64(user)
        }
    }
}

fn cpu_count() -> u32 {
    use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
    unsafe {
        let mut info: SYSTEM_INFO = std::mem::zeroed();
        GetSystemInfo(&mut info);
        info.dwNumberOfProcessors.max(1)
    }
}

pub fn measure_cpu_peak_percent(mut child: Child, sample_ms: u64) -> f64 {
    use windows_sys::Win32::Foundation::HANDLE;

    let handle = child.as_raw_handle() as HANDLE;
    let cpus = cpu_count() as f64;
    let mut peak = 0.0;
    let mut last_cpu = process_cpu_time_100ns(handle);
    let mut last_wall = Instant::now();

    loop {
        if let Ok(Some(_)) = child.try_wait() {
            let now = Instant::now();
            let cur_cpu = process_cpu_time_100ns(handle);
            let dt_cpu = cur_cpu.saturating_sub(last_cpu) as f64;
            let dt_wall_100ns = now.duration_since(last_wall).as_secs_f64() * 10_000_000.0;
            if dt_wall_100ns > 0.0 {
                let usage = (dt_cpu / dt_wall_100ns) * 100.0 / cpus;
                if usage > peak {
                    peak = usage;
                }
            }
            break;
        }

        thread::sleep(Duration::from_millis(sample_ms));
        let now = Instant::now();
        let cur_cpu = process_cpu_time_100ns(handle);
        let dt_cpu = cur_cpu.saturating_sub(last_cpu) as f64;
        let dt_wall_100ns = now.duration_since(last_wall).as_secs_f64() * 10_000_000.0;
        if dt_wall_100ns > 0.0 {
            let usage = (dt_cpu / dt_wall_100ns) * 100.0 / cpus;
            if usage > peak {
                peak = usage;
            }
        }
        last_cpu = cur_cpu;
        last_wall = now;
    }

    let _ = child.wait();
    peak
}

pub fn measure_working_set_peak_bytes(mut child: Child, sample_ms: u64) -> u64 {
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };

    let handle = child.as_raw_handle() as HANDLE;
    let mut peak = 0u64;

    let sample = || -> u64 {
        unsafe {
            let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
            counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            let ok = GetProcessMemoryInfo(handle, &mut counters as *mut _ as *mut _, counters.cb);
            if ok == 0 {
                0
            } else {
                counters.WorkingSetSize as u64
            }
        }
    };

    loop {
        let current = sample();
        if current > peak {
            peak = current;
        }
        if let Ok(Some(_)) = child.try_wait() {
            break;
        }
        thread::sleep(Duration::from_millis(sample_ms));
    }

    let current = sample();
    if current > peak {
        peak = current;
    }
    let _ = child.wait();
    peak
}

pub fn measure_working_set_peak_with_baseline_bytes(
    mut child: Child,
    sample_ms: u64,
    warmup_ms: u64,
) -> (u64, u64) {
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };

    let handle = child.as_raw_handle() as HANDLE;
    let mut baseline_peak = 0u64;
    let mut peak = 0u64;
    let warmup_deadline = Instant::now() + Duration::from_millis(warmup_ms);

    let sample = || -> u64 {
        unsafe {
            let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
            counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            let ok = GetProcessMemoryInfo(handle, &mut counters as *mut _ as *mut _, counters.cb);
            if ok == 0 {
                0
            } else {
                counters.WorkingSetSize as u64
            }
        }
    };

    loop {
        let current = sample();
        if current > peak {
            peak = current;
        }
        if Instant::now() <= warmup_deadline && current > baseline_peak {
            baseline_peak = current;
        }
        if let Ok(Some(_)) = child.try_wait() {
            break;
        }
        thread::sleep(Duration::from_millis(sample_ms));
    }

    let current = sample();
    if current > peak {
        peak = current;
    }
    if Instant::now() <= warmup_deadline && current > baseline_peak {
        baseline_peak = current;
    }
    let _ = child.wait();
    (baseline_peak, peak)
}

pub fn measure_handle_peak_count(mut child: Child, sample_ms: u64) -> u32 {
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::Threading::GetProcessHandleCount;

    let handle = child.as_raw_handle() as HANDLE;
    let mut peak = 0u32;

    let sample = || -> u32 {
        let mut count = 0u32;
        let ok = unsafe { GetProcessHandleCount(handle, &mut count) };
        if ok == 0 { 0 } else { count }
    };

    loop {
        let current = sample();
        if current > peak {
            peak = current;
        }
        if let Ok(Some(_)) = child.try_wait() {
            break;
        }
        thread::sleep(Duration::from_millis(sample_ms));
    }

    let current = sample();
    if current > peak {
        peak = current;
    }
    let _ = child.wait();
    peak
}

pub fn start_fake_proxy(expected: usize) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut count = 0usize;
        listener.set_nonblocking(true).ok();
        while count < expected && Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    count += 1;
                    thread::spawn(move || {
                        let mut buf = [0u8; 512];
                        stream.set_read_timeout(Some(Duration::from_secs(1))).ok();
                        let n = stream.read(&mut buf).unwrap_or(0);
                        if n > 0 {
                            let _ =
                                stream.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n");
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });
    (format!("http://{}", addr), handle)
}

pub struct LockHolder {
    child: Child,
}

impl LockHolder {
    pub fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for LockHolder {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub fn start_lock_holder(path: &Path) -> LockHolder {
    let p = path.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference='Stop'; \
         $p='{p}'; \
         if (-not (Test-Path $p)) {{ New-Item -Path $p -ItemType File -Force | Out-Null }}; \
         $f=[System.IO.File]::Open($p,[System.IO.FileMode]::Open,[System.IO.FileAccess]::ReadWrite,[System.IO.FileShare]::None); \
         try {{ Start-Sleep -Seconds 300 }} finally {{ $f.Dispose() }}"
    );

    let child = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    thread::sleep(Duration::from_millis(300));
    LockHolder { child }
}

pub fn make_safe_work_dir(prefix: &str) -> PathBuf {
    let mut base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    base.push("target");
    base.push("xun-test-safe");
    fs::create_dir_all(&base).unwrap();

    for _ in 0..64 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = TEST_ENV_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = base.join(format!("{prefix}-{}-{nanos}-{seq}", std::process::id()));
        if fs::create_dir(&dir).is_ok() {
            return dir;
        }
    }
    panic!("failed to create safe work dir for test");
}

pub fn cleanup_dir(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

pub fn wait_until_locked(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if is_locked(path) {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

fn is_locked(path: &Path) -> bool {
    let mut opts = fs::OpenOptions::new();
    opts.read(true).write(true).share_mode(0);
    opts.open(path).is_err()
}

pub fn is_lock_query_env_unavailable(out: &Output) -> bool {
    let stderr = String::from_utf8_lossy(&out.stderr);
    stderr.contains("OS Error 29")
        || stderr.contains("OS Error 121")
        || stderr.contains("Lock query timed out.")
}
