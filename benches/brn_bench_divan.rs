//! brn 模块 Divan 基准测试套件
//!
//! 覆盖维度：
//!   1. compute_single   - 单步 compute_ops，N 文件基线
//!   2. compute_chain_3  - 3 步 compute_ops_chain，N 文件
//!   3. break_cycles_100 - 100 个两节点环的 break_cycles
//!   4. break_cycles_1k  - 1000 个两节点环的 break_cycles

#![cfg(feature = "batch_rename")]

use std::hint::black_box;
use std::path::PathBuf;

use divan::{AllocProfiler, Bencher};

use xun::batch_rename::compute::{RenameMode, ReplacePair};
use xun::batch_rename::cycle_break::break_cycles;
use xun::batch_rename::testing as brn;
use xun::batch_rename::types::{CaseStyle, RenameOp};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

// ── 夹具 ──────────────────────────────────────────────────────────────────────

fn make_files(n: usize) -> Vec<PathBuf> {
    (0..n)
        .map(|i| PathBuf::from(format!("/bench/My File {:04}.txt", i)))
        .collect()
}

/// 生成 n 个两节点环：file_0 ↔ file_1, file_2 ↔ file_3, ...
fn make_swap_ops(n: usize) -> Vec<RenameOp> {
    (0..n)
        .flat_map(|i| {
            let a = PathBuf::from(format!("/bench/file_{:04}a.txt", i));
            let b = PathBuf::from(format!("/bench/file_{:04}b.txt", i));
            vec![
                RenameOp {
                    from: a.clone(),
                    to: b.clone(),
                },
                RenameOp {
                    from: b.clone(),
                    to: a.clone(),
                },
            ]
        })
        .collect()
}

// ── 基准：单步 compute_ops ────────────────────────────────────────────────────

#[divan::bench(args = [100, 1_000, 10_000])]
fn compute_single(bencher: Bencher, n: usize) {
    let files = make_files(n);
    let mode = RenameMode::Case(CaseStyle::Kebab);
    bencher.bench(|| black_box(brn::compute_ops(black_box(&files), black_box(&mode)).ok()));
}

// ── 基准：3 步 chain ──────────────────────────────────────────────────────────

#[divan::bench(args = [100, 1_000, 10_000])]
fn compute_chain_3(bencher: Bencher, n: usize) {
    let files = make_files(n);
    let steps = vec![
        RenameMode::Replace(vec![ReplacePair {
            from: " ".into(),
            to: "_".into(),
        }]),
        RenameMode::Prefix("pre_".into()),
        RenameMode::Case(CaseStyle::Kebab),
    ];
    bencher.bench(|| black_box(brn::compute_ops_chain(black_box(&files), black_box(&steps)).ok()));
}

// ── 基准：break_cycles（环形依赖中转）────────────────────────────────────────

#[divan::bench(args = [50, 500])]
fn break_cycles_swaps(bencher: Bencher, n: usize) {
    // n 对互换，产生 n 个两节点环
    let ops = make_swap_ops(n);
    let existing: Vec<PathBuf> = vec![];
    bencher.bench(|| black_box(break_cycles(black_box(ops.clone()), black_box(&existing))));
}

// ── 基准：undo/redo 100 次 apply（单文件累积历史）────────────────────────────
//
// 场景：对同一个文件做 N 次 apply，每次 push_undo 追加一个 batch，
// 然后测量：
//   undo_push_100     - 100 次 push_undo 的累计耗时（含每次读写 JSON）
//   undo_steps_100    - 一次性 run_undo_steps(100) 的耗时
//   redo_steps_100    - 一次性 run_redo_steps(100) 的耗时

use xun::batch_rename::undo::UndoRecord;

/// 在临时目录写入 n 个 batch 的历史文件，返回目录路径（调用方负责清理）
fn setup_undo_history(n: usize) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    // 创建一个真实文件供 rename 使用（bench 只测 push，不做实际 rename）
    for i in 0..n {
        let records = vec![UndoRecord {
            from: format!("file_{:04}.txt", i + 1),
            to: format!("file_{:04}.txt", i),
        }];
        brn::push_undo(dir.path(), &records).unwrap();
    }
    dir
}

/// 测量：100 次 push_undo 累计耗时（O(n²) I/O 热点）
#[divan::bench]
fn undo_push_100(bencher: Bencher) {
    bencher.bench(|| {
        let dir = tempfile::TempDir::new().unwrap();
        for i in 0..100usize {
            let records = vec![UndoRecord {
                from: format!("file_{:04}.txt", i + 1),
                to: format!("file_{:04}.txt", i),
            }];
            brn::push_undo(black_box(dir.path()), black_box(&records)).unwrap();
        }
        black_box(dir)
    });
}

/// 测量：读取 100 batch 历史文件的耗时
#[divan::bench]
fn undo_read_history_100(bencher: Bencher) {
    let dir = setup_undo_history(100);
    bencher.bench(|| black_box(brn::read_undo_history(black_box(dir.path())).unwrap()));
}

/// 测量：run_undo_steps(100) 在无真实文件时的耗时（JSON 解析 + 历史更新）
/// 注意：rename 会失败（文件不存在），但历史更新仍会执行
#[divan::bench]
fn undo_steps_100(bencher: Bencher) {
    bencher.bench(|| {
        let dir = setup_undo_history(100);
        // 忽略 rename 错误，只测框架耗时
        let _ = brn::run_undo_steps(black_box(dir.path().to_str().unwrap()), black_box(100));
        black_box(dir)
    });
}

/// 测量：run_redo_steps(100) 耗时（先 undo 填满 redo 栈）
#[divan::bench]
fn redo_steps_100(bencher: Bencher) {
    bencher.bench(|| {
        let dir = setup_undo_history(100);
        let _ = brn::run_undo_steps(dir.path().to_str().unwrap(), 100);
        let _ = brn::run_redo_steps(black_box(dir.path().to_str().unwrap()), black_box(100));
        black_box(dir)
    });
}
