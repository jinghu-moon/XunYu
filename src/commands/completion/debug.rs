use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

pub(super) struct DebugContext {
    start: Instant,
    file: Option<PathBuf>,
}

impl DebugContext {
    pub(super) fn new() -> Self {
        let file = env::var("XUN_COMP_DEBUG_FILE")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(PathBuf::from);
        Self {
            start: Instant::now(),
            file,
        }
    }

    pub(super) fn log(&self, msg: impl AsRef<str>) {
        let Some(path) = &self.file else { return };
        let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(path) else {
            return;
        };
        let _ = writeln!(
            &mut f,
            "elapsed_ms={}\t{}",
            self.start.elapsed().as_millis(),
            msg.as_ref()
        );
    }
}
