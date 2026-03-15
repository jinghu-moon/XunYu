use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use crossbeam_channel::{unbounded, Receiver, Sender};
use rayon::prelude::*;

use super::{
    build_issue, current_dir_safe, dedupe_inputs, string_check, winapi, PathIssue,
    PathIssueKind, PathKind, PathPolicy, PathValidationResult,
};

const PARALLEL_MIN: usize = 64;
const UNC_THRESHOLD: usize = 500;

pub(crate) fn validate_paths(raw_inputs: Vec<String>, policy: &PathPolicy) -> PathValidationResult {
    let total = raw_inputs.len();
    if total < PARALLEL_MIN {
        return super::validate_paths_serial(raw_inputs, policy);
    }

    let has_unc = raw_inputs
        .iter()
        .any(|raw| has_unc_prefix(raw.as_bytes()));

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
    raw_inputs: Vec<String>,
    policy: &PathPolicy,
    limit_unc: bool,
) -> PathValidationResult {
    let (inputs, deduped) = dedupe_inputs(raw_inputs);
    let total = inputs.len();
    let inputs = Arc::new(inputs);

    let mut out = PathValidationResult::default();
    out.deduped = deduped;
    if total == 0 {
        return out;
    }

    let (work_tx, work_rx) = unbounded::<WorkItem>();
    let (result_tx, result_rx) = unbounded::<Outcome>();

    let io_threads = io_threads(limit_unc);
    let mut workers = Vec::with_capacity(io_threads);
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

    let stage_result_tx = result_tx.clone();
    let stage_work_tx = work_tx.clone();
    let policy_stage = policy.clone();
    let cwd_snapshot = if policy.allow_relative {
        policy_stage
            .cwd_snapshot
            .clone()
            .or_else(current_dir_safe)
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
    raw: &str,
    policy: &PathPolicy,
    cwd_snapshot: Option<&PathBuf>,
    work_tx: &Sender<WorkItem>,
    result_tx: &Sender<Outcome>,
) {
    let mut check_policy = policy.clone();
    if policy.expand_env && raw.contains('%') {
        check_policy.allow_relative = true;
    }
    if let Some(kind) = string_check::check_string(raw, &check_policy) {
        let _ = result_tx.send(Outcome::Issue {
            idx,
            issue: build_issue(raw, kind),
        });
        return;
    }

    if !policy.expand_env && raw.contains('%') {
        let _ = result_tx.send(Outcome::Issue {
            idx,
            issue: build_issue(raw, PathIssueKind::EnvVarNotAllowed),
        });
        return;
    }

    let mut current = raw.to_string();
    if policy.expand_env && raw.contains('%') {
        match winapi::expand_env(raw) {
            Ok(expanded) => {
                current = expanded;
                if let Some(kind) = string_check::check_string(&current, policy) {
                    let _ = result_tx.send(Outcome::Issue {
                        idx,
                        issue: build_issue(raw, kind),
                    });
                    return;
                }
            }
            Err(kind) => {
                let _ = result_tx.send(Outcome::Issue {
                    idx,
                    issue: build_issue(raw, kind),
                });
                return;
            }
        }
    }

    let mut kind = string_check::detect_kind(&current);
    if matches!(kind, PathKind::Relative) {
        if !policy.allow_relative {
            let _ = result_tx.send(Outcome::Issue {
                idx,
                issue: build_issue(raw, PathIssueKind::RelativeNotAllowed),
            });
            return;
        }
        let base = match cwd_snapshot {
            Some(value) => value,
            None => {
                let _ = result_tx.send(Outcome::Issue {
                    idx,
                    issue: build_issue(raw, PathIssueKind::IoError),
                });
                return;
            }
        };
        let joined = base.join(&current);
        let full = match winapi::get_full_path(&joined) {
            Ok(path) => path,
            Err(kind) => {
                let _ = result_tx.send(Outcome::Issue {
                    idx,
                    issue: build_issue(raw, kind),
                });
                return;
            }
        };
        current = full.to_string_lossy().to_string();
        kind = string_check::detect_kind(&current);
        if matches!(kind, PathKind::Relative) {
            let _ = result_tx.send(Outcome::Issue {
                idx,
                issue: build_issue(raw, PathIssueKind::RelativeNotAllowed),
            });
            return;
        }
        if matches!(kind, PathKind::DriveRelative) {
            let _ = result_tx.send(Outcome::Issue {
                idx,
                issue: build_issue(raw, PathIssueKind::DriveRelativeNotAllowed),
            });
            return;
        }
    }

    let path = PathBuf::from(&current);
    if policy.must_exist || policy.safety_check {
        let _ = work_tx.send(WorkItem { idx, path });
        return;
    }

    let _ = result_tx.send(Outcome::Ok { idx, path });
}

fn worker_loop(
    work_rx: Receiver<WorkItem>,
    result_tx: Sender<Outcome>,
    inputs: Arc<Vec<String>>,
    policy: PathPolicy,
) {
    for item in work_rx.iter() {
        let raw = inputs.get(item.idx).map(|s| s.as_str()).unwrap_or("");
        if policy.must_exist {
            match winapi::probe(&item.path) {
                Ok(attr) => {
                    if !policy.allow_reparse && winapi::is_reparse_point(attr) {
                        let _ = result_tx.send(Outcome::Issue {
                            idx: item.idx,
                            issue: build_issue(raw, PathIssueKind::ReparsePoint),
                        });
                        continue;
                    }
                }
                Err(kind) => {
                    let _ = result_tx.send(Outcome::Issue {
                        idx: item.idx,
                        issue: build_issue(raw, kind),
                    });
                    continue;
                }
            }
        }

        if policy.safety_check {
            if crate::windows::safety::ensure_safe_target(&item.path).is_err() {
                let _ = result_tx.send(Outcome::Issue {
                    idx: item.idx,
                    issue: build_issue(raw, PathIssueKind::AccessDenied),
                });
                continue;
            }
        }

        let _ = result_tx.send(Outcome::Ok {
            idx: item.idx,
            path: item.path,
        });
    }
}

fn has_unc_prefix(bytes: &[u8]) -> bool {
    matches!(bytes, [b'\\', b'\\', b'?', b'\\', b'U', b'N', b'C', b'\\', ..])
        || matches!(bytes, [b'\\', b'\\', ..])
}

fn io_threads(limit_unc: bool) -> usize {
    let avail = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let max = if limit_unc { 4 } else { 8 };
    let threads = std::cmp::min(max, avail);
    threads.max(1)
}
