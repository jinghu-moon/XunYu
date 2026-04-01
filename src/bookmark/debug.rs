use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

pub(crate) struct BookmarkTiming {
    enabled: bool,
    command: &'static str,
    start: Instant,
    last: Instant,
    events: Vec<(&'static str, u128)>,
    file: Option<PathBuf>,
}

impl BookmarkTiming {
    pub(crate) fn new(command: &'static str) -> Self {
        let now = Instant::now();
        Self {
            enabled: timing_enabled(),
            command,
            start: now,
            last: now,
            events: Vec::new(),
            file: env::var("XUN_BM_DEBUG_FILE")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(PathBuf::from),
        }
    }

    pub(crate) fn mark(&mut self, label: &'static str) {
        if !self.enabled && self.file.is_none() {
            return;
        }
        let now = Instant::now();
        self.events.push((label, now.duration_since(self.last).as_millis()));
        self.last = now;
    }

    pub(crate) fn finish(&mut self, extras: &[(&str, String)]) {
        if !self.enabled && self.file.is_none() {
            return;
        }
        let total_ms = self.start.elapsed().as_millis();
        self.emit(total_ms, extras);
    }

    fn emit(&self, total_ms: u128, extras: &[(&str, String)]) {
        let mut line = format!("bookmark timing [{}]", self.command);
        for (label, elapsed) in &self.events {
            line.push_str(&format!(" {label}={elapsed}ms"));
        }
        line.push_str(&format!(" total={total_ms}ms"));
        for (key, value) in extras {
            line.push_str(&format!(" {key}={value}"));
        }

        if self.enabled {
            eprintln!("{line}");
        }
        if let Some(path) = &self.file
            && let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path)
        {
            let _ = writeln!(&mut file, "{line}");
        }
    }
}

pub(crate) struct BookmarkLoadTiming {
    enabled: bool,
    start: Instant,
    last: Instant,
    events: Vec<(&'static str, u128)>,
    file: Option<PathBuf>,
    path: PathBuf,
    bytes: usize,
    start_ws: u64,
}

impl BookmarkLoadTiming {
    pub(crate) fn new(path: &std::path::Path, bytes: usize) -> Self {
        let now = Instant::now();
        Self {
            enabled: timing_enabled() || env::var_os("XUN_BM_LOAD_TIMING").is_some(),
            start: now,
            last: now,
            events: Vec::new(),
            file: env::var("XUN_BM_DEBUG_FILE")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(PathBuf::from),
            path: path.to_path_buf(),
            bytes,
            start_ws: current_working_set_bytes(),
        }
    }

    pub(crate) fn mark(&mut self, label: &'static str) {
        if !self.enabled && self.file.is_none() {
            return;
        }
        let now = Instant::now();
        self.events.push((label, now.duration_since(self.last).as_millis()));
        self.last = now;
    }

    pub(crate) fn finish(&self, bookmarks: usize, fast_path: bool, extras: &[(&str, String)]) {
        if !self.enabled && self.file.is_none() {
            return;
        }
        let total_ms = self.start.elapsed().as_millis();
        let end_ws = current_working_set_bytes();
        let delta_ws = end_ws.saturating_sub(self.start_ws);
        let mut line = format!(
            "bookmark load path={} bytes={} bookmarks={} fast_path={}",
            self.path.display(),
            self.bytes,
            bookmarks,
            fast_path
        );
        for (label, elapsed) in &self.events {
            line.push_str(&format!(" {label}={elapsed}ms"));
        }
        line.push_str(&format!(
            " total={}ms ws_delta_kib={} ws_end_kib={}",
            total_ms,
            delta_ws / 1024,
            end_ws / 1024
        ));
        for (key, value) in extras {
            line.push_str(&format!(" {key}={value}"));
        }
        if self.enabled {
            eprintln!("{line}");
        }
        if let Some(path) = &self.file
            && let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path)
        {
            let _ = writeln!(&mut file, "{line}");
        }
    }
}

fn timing_enabled() -> bool {
    ["XUN_BM_TIMING", "XUN_BOOKMARK_TIMING", "XUN_CMD_TIMING"]
        .into_iter()
        .any(|name| env::var_os(name).is_some())
}

#[cfg(windows)]
fn current_working_set_bytes() -> u64 {
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

#[cfg(not(windows))]
fn current_working_set_bytes() -> u64 {
    0
}

#[cfg(test)]
mod tests {
    #[test]
    fn bookmark_timing_enabled_accepts_bookmark_specific_names() {
        let env = std::collections::HashMap::from([
            ("XUN_BM_TIMING", "1"),
            ("XUN_BOOKMARK_TIMING", "1"),
        ]);
        assert!(["XUN_BM_TIMING", "XUN_BOOKMARK_TIMING"]
            .into_iter()
            .all(|name| env.contains_key(name)));
        assert!(timing_enabled_with(|name| env.get(name).copied()));
    }

    #[test]
    fn bookmark_timing_enabled_accepts_global_command_timing() {
        let env = std::collections::HashMap::from([("XUN_CMD_TIMING", "1")]);
        assert!(timing_enabled_with(|name| env.get(name).copied()));
    }

    fn timing_enabled_with<F>(mut get_env: F) -> bool
    where
        F: FnMut(&str) -> Option<&'static str>,
    {
        ["XUN_BM_TIMING", "XUN_BOOKMARK_TIMING", "XUN_CMD_TIMING"]
            .into_iter()
            .any(|name| get_env(name).is_some())
    }
}
