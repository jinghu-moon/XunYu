use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use crate::output::{CliError, CliResult};

use super::super::filters::FindFilters;
use super::super::ignore::IgnoreSet;
use super::super::rules::{CompiledRules, RuleKind};
use super::common::{ScanItem, ScanOutput, resolve_base_root};
use super::single::scan_single_file;

#[cfg(not(windows))]
use super::dir_std::scan_dir_std;
#[cfg(windows)]
use super::dir_windows::scan_dir_windows;

struct Task {
    dir: PathBuf,
    depth: i32,
    inherited: RuleKind,
    base_root: Arc<PathBuf>,
    base_display: Arc<String>,
    ignore: Arc<IgnoreSet>,
}

const TASK_BATCH_SIZE: usize = 32;

pub(super) fn scan_parallel(
    base_dirs: &[String],
    rules: &CompiledRules,
    filters: &FindFilters,
    force_meta: bool,
    count_only: bool,
    threads: usize,
) -> CliResult<ScanOutput> {
    let rules = Arc::new(rules.clone());
    let filters = Arc::new(filters.clone());

    let (tx, rx) = mpsc::channel::<Task>();
    let rx = Arc::new(Mutex::new(rx));
    let pending = Arc::new(AtomicUsize::new(0));
    let results: Option<Arc<Mutex<Vec<ScanItem>>>> = if count_only {
        None
    } else {
        Some(Arc::new(Mutex::new(Vec::new())))
    };
    let total_count = Arc::new(AtomicUsize::new(0));

    let max_depth = filters.depth.as_ref().and_then(|d| d.max).unwrap_or(-1);

    let mut file_items = Vec::new();
    let mut file_count = 0usize;

    for base in base_dirs {
        let base_path = PathBuf::from(base);
        if !base_path.exists() {
            return Err(CliError::new(2, format!("Path not found: {base}")));
        }
        let (base_root, base_display) = resolve_base_root(base, &base_path);
        let ignore = Arc::new(IgnoreSet::new(base_root.as_path()));
        let inherited = if rules.default_include {
            RuleKind::Include
        } else {
            RuleKind::Exclude
        };

        if base_path.is_file() {
            scan_single_file(
                &base_path,
                &base_root,
                &base_display,
                &rules,
                &filters,
                force_meta,
                count_only,
                inherited,
                &ignore,
                &mut file_items,
                &mut file_count,
            );
            continue;
        }

        enqueue_task(
            &tx,
            &pending,
            Task {
                dir: base_path,
                depth: 0,
                inherited,
                base_root: Arc::new(base_root),
                base_display: Arc::new(base_display),
                ignore,
            },
        );
    }

    let mut handles = Vec::new();
    for _ in 0..threads {
        let rx = Arc::clone(&rx);
        let tx = tx.clone();
        let pending = Arc::clone(&pending);
        let rules = Arc::clone(&rules);
        let filters = Arc::clone(&filters);
        let results = results.clone();
        let total_count = Arc::clone(&total_count);

        let handle = thread::spawn(move || {
            let mut local_items = Vec::new();
            let mut local_count = 0usize;
            loop {
                let task = {
                    let guard = rx.lock().unwrap();
                    guard.recv_timeout(Duration::from_millis(50))
                };
                match task {
                    Ok(task) => {
                        process_task(
                            task,
                            &rules,
                            &filters,
                            force_meta,
                            count_only,
                            max_depth,
                            &tx,
                            &pending,
                            &mut local_items,
                            &mut local_count,
                        );
                        pending.fetch_sub(1, Ordering::SeqCst);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if pending.load(Ordering::SeqCst) == 0 {
                            break;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }

            if count_only {
                if local_count > 0 {
                    total_count.fetch_add(local_count, Ordering::SeqCst);
                }
            } else if let Some(results) = results {
                if !local_items.is_empty() {
                    let mut guard = results.lock().unwrap();
                    guard.extend(local_items);
                }
            }
        });
        handles.push(handle);
    }

    while pending.load(Ordering::SeqCst) > 0 {
        thread::sleep(Duration::from_millis(20));
    }
    drop(tx);

    for handle in handles {
        let _ = handle.join();
    }

    if count_only {
        let total = total_count.load(Ordering::SeqCst) + file_count;
        return Ok(ScanOutput {
            items: Vec::new(),
            count: total,
        });
    }

    let results = results.expect("results must exist");
    let mut out = results.lock().unwrap_or_else(|e| e.into_inner());
    if !file_items.is_empty() {
        out.extend(file_items);
    }
    let items = std::mem::take(&mut *out);
    let count = items.len();
    Ok(ScanOutput { items, count })
}

fn process_task(
    task: Task,
    rules: &CompiledRules,
    filters: &FindFilters,
    force_meta: bool,
    count_only: bool,
    max_depth: i32,
    tx: &mpsc::Sender<Task>,
    pending: &AtomicUsize,
    local_items: &mut Vec<ScanItem>,
    local_count: &mut usize,
) {
    if max_depth >= 0 && task.depth >= max_depth {
        return;
    }
    let mut pending_tasks: Vec<Task> = Vec::new();
    let mut push_dir = |child: PathBuf, state: RuleKind| {
        pending_tasks.push(Task {
            dir: child,
            depth: task.depth + 1,
            inherited: state,
            base_root: Arc::clone(&task.base_root),
            base_display: Arc::clone(&task.base_display),
            ignore: Arc::clone(&task.ignore),
        });
        if pending_tasks.len() >= TASK_BATCH_SIZE {
            enqueue_tasks(tx, pending, &mut pending_tasks);
        }
    };
    let mut push_item = |item: ScanItem| {
        local_items.push(item);
    };

    #[cfg(windows)]
    {
        scan_dir_windows(
            &task.dir,
            &task.base_root,
            &task.base_display,
            rules,
            filters,
            &task.ignore,
            task.inherited,
            task.depth,
            force_meta,
            count_only,
            local_count,
            &mut push_dir,
            &mut push_item,
        );
    }
    #[cfg(not(windows))]
    {
        scan_dir_std(
            &task.dir,
            &task.base_root,
            &task.base_display,
            rules,
            filters,
            &task.ignore,
            task.inherited,
            task.depth,
            force_meta,
            count_only,
            local_count,
            &mut push_dir,
            &mut push_item,
        );
    }
    if !pending_tasks.is_empty() {
        enqueue_tasks(tx, pending, &mut pending_tasks);
    }
}

fn enqueue_task(tx: &mpsc::Sender<Task>, pending: &AtomicUsize, task: Task) {
    pending.fetch_add(1, Ordering::SeqCst);
    if tx.send(task).is_err() {
        pending.fetch_sub(1, Ordering::SeqCst);
    }
}

fn enqueue_tasks(tx: &mpsc::Sender<Task>, pending: &AtomicUsize, tasks: &mut Vec<Task>) {
    if tasks.is_empty() {
        return;
    }
    pending.fetch_add(tasks.len(), Ordering::SeqCst);
    for task in tasks.drain(..) {
        if tx.send(task).is_err() {
            pending.fetch_sub(1, Ordering::SeqCst);
        }
    }
}
