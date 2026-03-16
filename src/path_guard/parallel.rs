use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::thread;

use crossbeam_channel::{unbounded, Receiver, Sender};
use rayon::prelude::*;

use super::{
    build_issue, dedupe_inputs, string_check, winapi, PathIssue, PathIssueKind, PathKind,
    PathPolicy, PathValidationResult,
};

const PARALLEL_MIN: usize = 32;
const UNC_THRESHOLD: usize = 500;

pub(crate) fn validate_paths(raw_inputs: Vec<OsString>, policy: &PathPolicy) -> PathValidationResult {
    let total = raw_inputs.len();
    if total < PARALLEL_MIN {
        return super::validate_paths_serial(raw_inputs, policy);
    }

    let has_unc = raw_inputs.iter().any(|raw| is_unc_path(raw));

    let limit_unc = total > UNC_THRESHOLD && has_unc;
    validate_paths_parallel(raw_inputs, policy, limit_unc)
}

struct WorkItem {
    idx: usize,
    path: PathBuf,
}

enum Outcome {
    Ok { idx: usize, path: PathBuf },
    Issue { idx: usize, issue: PathIssue },
}

fn validate_paths_parallel(
    raw_inputs: Vec<OsString>,
    policy: &PathPolicy,
    limit_unc: bool,
) -> PathValidationResult {
    let (inputs, deduped) = dedupe_inputs(raw_inputs);
    let total = inputs.len();
    let inputs = Arc::new(inputs);
    let use_workers = policy.must_exist || policy.safety_check;

    let mut out = PathValidationResult::default();
    out.deduped = deduped;
    if total == 0 {
        return out;
    }

    let (work_tx, work_rx) = unbounded::<WorkItem>();
    let (result_tx, result_rx) = unbounded::<Outcome>();

    let mut workers: Vec<thread::JoinHandle<()>> = Vec::new();
    if use_workers {
        let io_threads = io_threads(limit_unc);
        workers.reserve(io_threads);
        for _ in 0..io_threads {
            let rx = work_rx.clone();
            let tx = result_tx.clone();
            let policy = policy.clone();
            let inputs = Arc::clone(&inputs);
            workers.push(thread::spawn(move || {
                worker_loop(rx, tx, inputs, policy);
            }));
        }
        drop(work_rx);
    }

    let stage_result_tx = result_tx.clone();
    let stage_work_tx = work_tx.clone();
    let policy_stage = policy.clone();
    let cwd_snapshot = if policy.allow_relative {
        policy_stage
            .cwd_snapshot
            .clone()
            .or_else(super::current_dir_safe)
            .map(Arc::new)
    } else {
        None
    };

    inputs
        .par_iter()
        .enumerate()
        .for_each(|(idx, raw)| {
            process_stage_a(
                idx,
                raw,
                &policy_stage,
                cwd_snapshot.as_deref(),
                &stage_work_tx,
                &stage_result_tx,
            );
        });

    drop(stage_work_tx);
    drop(stage_result_tx);
    drop(work_tx);
    drop(result_tx);

    let mut ok_slots: Vec<Option<PathBuf>> = vec![None; total];
    let mut issue_slots: Vec<Option<PathIssue>> = vec![None; total];

    for _ in 0..total {
        let outcome = match result_rx.recv() {
            Ok(value) => value,
            Err(_) => break,
        };
        match outcome {
            Outcome::Ok { idx, path } => ok_slots[idx] = Some(path),
            Outcome::Issue { idx, issue } => issue_slots[idx] = Some(issue),
        }
    }

    for handle in workers {
        let _ = handle.join();
    }

    out.ok = ok_slots.into_iter().flatten().collect();
    out.issues = issue_slots.into_iter().flatten().collect();
    out
}

fn process_stage_a(
    idx: usize,
    raw: &OsString,
    policy: &PathPolicy,
    cwd_snapshot: Option<&PathBuf>,
    work_tx: &Sender<WorkItem>,
    result_tx: &Sender<Outcome>,
) {
    let result = super::with_check_buf(|scratch| {
        super::validate_string_stage(raw.as_os_str(), policy, cwd_snapshot, scratch)
    });

    let (path, _) = match result {
        Ok(value) => value,
        Err(kind) => {
            let _ = result_tx.send(Outcome::Issue {
                idx,
                issue: build_issue(raw.as_os_str(), kind),
            });
            return;
        }
    };

    if policy.must_exist || policy.safety_check {
        let _ = work_tx.send(WorkItem { idx, path });
        return;
    }

    let _ = result_tx.send(Outcome::Ok { idx, path });
}

fn worker_loop(
    work_rx: Receiver<WorkItem>,
    result_tx: Sender<Outcome>,
    inputs: Arc<Vec<OsString>>,
    policy: PathPolicy,
) {
    for item in work_rx.iter() {
        let raw = inputs
            .get(item.idx)
            .map(|s| s.as_os_str())
            .unwrap_or_else(|| OsStr::new(""));
        if policy.must_exist {
            let probe_timer = super::trace_stage_start(super::TraceStage::Probe);
            match winapi::probe(&item.path) {
                Ok(attr) => {
                    if !policy.allow_reparse && winapi::is_reparse_point(attr) {
                        super::trace_stage_end(super::TraceStage::Probe, probe_timer);
                        let _ = result_tx.send(Outcome::Issue {
                            idx: item.idx,
                            issue: build_issue(raw, PathIssueKind::ReparsePoint),
                        });
                        continue;
                    }
                }
                Err(kind) => {
                    super::trace_stage_end(super::TraceStage::Probe, probe_timer);
                    let _ = result_tx.send(Outcome::Issue {
                        idx: item.idx,
                        issue: build_issue(raw, kind),
                    });
                    continue;
                }
            }
            super::trace_stage_end(super::TraceStage::Probe, probe_timer);
        }

        if policy.safety_check {
            let safety_timer = super::trace_stage_start(super::TraceStage::Safety);
            if crate::windows::safety::ensure_safe_target(&item.path).is_err() {
                super::trace_stage_end(super::TraceStage::Safety, safety_timer);
                let _ = result_tx.send(Outcome::Issue {
                    idx: item.idx,
                    issue: build_issue(raw, PathIssueKind::AccessDenied),
                });
                continue;
            }
            super::trace_stage_end(super::TraceStage::Safety, safety_timer);
        }

        let _ = result_tx.send(Outcome::Ok {
            idx: item.idx,
            path: item.path,
        });
    }
}

fn is_unc_path(raw: &OsStr) -> bool {
    super::with_check_buf(|scratch| {
        super::fill_wide(scratch, raw);
        matches!(
            string_check::detect_kind(scratch),
            PathKind::UNC | PathKind::ExtendedUNC
        )
    })
}

fn io_threads(limit_unc: bool) -> usize {
    let avail = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let mut max = if limit_unc {
        4
    } else {
        std::cmp::min((avail / 2).max(1), 8)
    };
    if let Some(override_threads) = env_io_threads() {
        max = override_threads;
    }
    if limit_unc && max > 4 {
        max = 4;
    }
    let threads = std::cmp::min(max, avail);
    threads.max(1)
}

fn env_io_threads() -> Option<usize> {
    static OVERRIDE: OnceLock<Option<usize>> = OnceLock::new();
    *OVERRIDE.get_or_init(|| {
        std::env::var("XUN_PG_IO_THREADS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
    })
}
