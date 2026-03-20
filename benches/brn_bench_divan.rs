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
use xun::batch_rename::types::{CaseStyle, RenameOp};
use xun::batch_rename::testing as brn;

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
                RenameOp { from: a.clone(), to: b.clone() },
                RenameOp { from: b.clone(), to: a.clone() },
            ]
        })
        .collect()
}

// ── 基准：单步 compute_ops ────────────────────────────────────────────────────

#[divan::bench(args = [100, 1_000, 10_000])]
fn compute_single(bencher: Bencher, n: usize) {
    let files = make_files(n);
    let mode = RenameMode::Case(CaseStyle::Kebab);
    bencher.bench(|| {
        black_box(brn::compute_ops(black_box(&files), black_box(&mode)).ok())
    });
}

// ── 基准：3 步 chain ──────────────────────────────────────────────────────────

#[divan::bench(args = [100, 1_000, 10_000])]
fn compute_chain_3(bencher: Bencher, n: usize) {
    let files = make_files(n);
    let steps = vec![
        RenameMode::Replace(vec![ReplacePair { from: " ".into(), to: "_".into() }]),
        RenameMode::Prefix("pre_".into()),
        RenameMode::Case(CaseStyle::Kebab),
    ];
    bencher.bench(|| {
        black_box(brn::compute_ops_chain(black_box(&files), black_box(&steps)).ok())
    });
}

// ── 基准：break_cycles（环形依赖中转）────────────────────────────────────────

#[divan::bench(args = [50, 500])]
fn break_cycles_swaps(bencher: Bencher, n: usize) {
    // n 对互换，产生 n 个两节点环
    let ops = make_swap_ops(n);
    let existing: Vec<PathBuf> = vec![];
    bencher.bench(|| {
        black_box(break_cycles(black_box(ops.clone()), black_box(&existing)))
    });
}
