use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub(crate) struct Progress {
    pub(crate) scanned: AtomicU64,
    pub(crate) processed: AtomicU64,
    pub(crate) succeeded: AtomicU64,
    pub(crate) failed: AtomicU64,
}

impl Progress {
    pub(crate) fn scanned(&self) -> u64 {
        self.scanned.load(Ordering::Relaxed)
    }

    pub(crate) fn processed(&self) -> u64 {
        self.processed.load(Ordering::Relaxed)
    }

    pub(crate) fn succeeded(&self) -> u64 {
        self.succeeded.load(Ordering::Relaxed)
    }

    pub(crate) fn failed(&self) -> u64 {
        self.failed.load(Ordering::Relaxed)
    }

    pub(crate) fn inc_scanned(&self) {
        self.scanned.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn inc_processed(&self) {
        self.processed.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn inc_succeeded(&self) {
        self.succeeded.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn inc_failed(&self) {
        self.failed.fetch_add(1, Ordering::Relaxed);
    }
}
