//! ACL 模块性能基准测试套件（Divan）
//!
//! 覆盖维度：
//!   1. diff_acl：快照对比吞吐量，按条目数和差异率分组
//!   2. compute_effective_access：有效权限计算吞吐
//!   3. diff_key：AceEntry.diff_key() 字符串格式化吞吐
//!   4. rights_short：权限掩码表查找延迟
//!   5. orphan filter：孤儿 SID 过滤吞吐

#![cfg(windows)]

use std::hint::black_box;
use std::path::PathBuf;

use divan::{AllocProfiler, Bencher};

use xun::acl::diff::diff_acl;
use xun::acl::effective::compute_effective_access;
use xun::acl::types::{AceEntry, AceType, AclSnapshot, InheritanceFlags, PropagationFlags};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

// ── 测试夹具 ─────────────────────────────────────────────────────────────────

fn make_ace(principal: &str, rights: u32, ace_type: AceType, inherited: bool) -> AceEntry {
    AceEntry {
        principal: principal.to_string(),
        raw_sid: format!("S-1-5-{}", principal.len()),
        rights_mask: rights,
        ace_type,
        inheritance: InheritanceFlags::BOTH,
        propagation: PropagationFlags::NONE,
        is_inherited: inherited,
        is_orphan: false,
    }
}

fn make_snapshot(n: usize) -> AclSnapshot {
    let entries = (0..n)
        .map(|i| {
            let ace_type = if i % 3 == 0 { AceType::Deny } else { AceType::Allow };
            let inherited = i % 2 == 0;
            make_ace(&format!("S-1-5-21-bench-{i}"), 0x1F01FF, ace_type, inherited)
        })
        .collect();
    AclSnapshot {
        path: PathBuf::from(r"C:\bench\acl"),
        owner: r"BUILTIN\Administrators".to_string(),
        is_protected: false,
        entries,
    }
}

fn make_snapshot_pair(n: usize, diff_pct: u32) -> (AclSnapshot, AclSnapshot) {
    let diff_count = n * diff_pct as usize / 100;
    let common_count = n - diff_count;
    let common: Vec<AceEntry> = (0..common_count)
        .map(|i| make_ace(&format!("S-1-5-21-common-{i}"), 0x1F01FF, AceType::Allow, false))
        .collect();
    let only_a: Vec<AceEntry> = (0..diff_count)
        .map(|i| make_ace(&format!("S-1-5-21-onlya-{i}"), 0x1F01FF, AceType::Allow, false))
        .collect();
    let mut a_entries = common.clone();
    a_entries.extend(only_a);
    let snap_a = AclSnapshot {
        path: PathBuf::from(r"C:\bench\a"),
        owner: r"BUILTIN\Administrators".to_string(),
        is_protected: false,
        entries: a_entries,
    };
    let snap_b = AclSnapshot {
        path: PathBuf::from(r"C:\bench\b"),
        owner: r"BUILTIN\Administrators".to_string(),
        is_protected: false,
        entries: common,
    };
    (snap_a, snap_b)
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. diff_acl 吞吐量
// ═══════════════════════════════════════════════════════════════════════════

/// 相同快照对比（HashMap 建表 + 全部命中，最优路径）
#[divan::bench(args = [10usize, 100, 500, 1000], sample_count = 200)]
fn diff_identical(bencher: Bencher, n: usize) {
    let snap = make_snapshot(n);
    bencher
        .counter(divan::counter::ItemsCount::new(n * 2))
        .bench(|| black_box(diff_acl(black_box(&snap), black_box(&snap))));
}

/// 按差异率分组：0%（完全相同）/ 10% / 50% / 100%（完全不同）
#[divan::bench(args = [0u32, 10, 50, 100], sample_count = 200)]
fn diff_by_diff_rate(bencher: Bencher, diff_pct: u32) {
    const N: usize = 500;
    let (a, b) = make_snapshot_pair(N, diff_pct);
    bencher
        .counter(divan::counter::ItemsCount::new(N * 2))
        .bench(|| black_box(diff_acl(black_box(&a), black_box(&b))));
}

/// 完全不同快照（所有 ACE 都在 only_in_a/only_in_b，无 common）
#[divan::bench(args = [10usize, 100, 500, 1000], sample_count = 100)]
fn diff_completely_different(bencher: Bencher, n: usize) {
    let snap_a = make_snapshot(n);
    let snap_b = AclSnapshot {
        path: PathBuf::from(r"C:\bench\b2"),
        owner: r"NT AUTHORITY\SYSTEM".to_string(),
        is_protected: true,
        entries: (0..n)
            .map(|i| make_ace(&format!("S-1-5-21-uniq-{i}"), 0x1200A9, AceType::Allow, false))
            .collect(),
    };
    bencher
        .counter(divan::counter::ItemsCount::new(n * 2))
        .bench(|| black_box(diff_acl(black_box(&snap_a), black_box(&snap_b))));
}

/// 空快照对比（基线开销）
#[divan::bench(sample_count = 2000)]
fn diff_empty(bencher: Bencher) {
    let a = AclSnapshot {
        path: PathBuf::from(r"C:\bench\empty"),
        owner: r"BUILTIN\Administrators".to_string(),
        is_protected: false,
        entries: vec![],
    };
    bencher.bench(|| black_box(diff_acl(black_box(&a), black_box(&a))));
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. compute_effective_access 吞吐量
// ═══════════════════════════════════════════════════════════════════════════

/// 单次调用延迟 — 单个 SID，少量 ACE（热路径）
#[divan::bench(sample_count = 2000)]
fn effective_single_sid_small(bencher: Bencher) {
    let snap = make_snapshot(10);
    let sids = vec!["S-1-5-21-bench-0".to_string()];
    bencher.bench(|| black_box(compute_effective_access(black_box(&snap), black_box(&sids))));
}

/// SID 数量对匹配吞吐的影响：1 / 5 / 20 个 SID
#[divan::bench(args = [1usize, 5, 20], sample_count = 500)]
fn effective_sid_count(bencher: Bencher, sid_count: usize) {
    let snap = make_snapshot(100);
    let sids: Vec<String> = (0..sid_count)
        .map(|i| format!("S-1-5-21-bench-{i}"))
        .collect();
    bencher.bench(|| black_box(compute_effective_access(black_box(&snap), black_box(&sids))));
}

/// ACE 数量扩展曲线：10 / 50 / 200 / 500 条
#[divan::bench(args = [10usize, 50, 200, 500], sample_count = 200)]
fn effective_ace_count(bencher: Bencher, n: usize) {
    let snap = make_snapshot(n);
    let sids = vec!["S-1-5-21-bench-0".to_string(), "S-1-5-21-bench-1".to_string()];
    bencher
        .counter(divan::counter::ItemsCount::new(n))
        .bench(|| black_box(compute_effective_access(black_box(&snap), black_box(&sids))));
}

/// deny 覆盖 allow 场景（deny_mask 有位，extra 路径）
#[divan::bench(sample_count = 1000)]
fn effective_deny_overrides_allow(bencher: Bencher) {
    use xun::acl::effective::{RIGHT_DELETE, RIGHT_READ_DATA};
    let sid = "S-1-5-21-target";
    let snap = AclSnapshot {
        path: PathBuf::from(r"C:\bench\deny"),
        owner: r"BUILTIN\Administrators".to_string(),
        is_protected: false,
        entries: vec![
            make_ace(sid, RIGHT_READ_DATA | RIGHT_DELETE, AceType::Allow, false),
            make_ace(sid, RIGHT_DELETE, AceType::Deny, false),
        ],
    };
    let sids = vec![sid.to_string()];
    bencher.bench(|| black_box(compute_effective_access(black_box(&snap), black_box(&sids))));
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. diff_key 吞吐量
// ═══════════════════════════════════════════════════════════════════════════

/// diff_key 生成吞吐（纯字符串格式化，无 IO）
#[divan::bench(args = [100usize, 1000, 10_000], sample_count = 100)]
fn diff_key_throughput(bencher: Bencher, n: usize) {
    let snap = make_snapshot(n);
    bencher
        .counter(divan::counter::ItemsCount::new(n))
        .bench(|| {
            let _keys: Vec<String> =
                snap.entries.iter().map(|e| black_box(e.diff_key())).collect();
        });
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. rights_short 查找
// ═══════════════════════════════════════════════════════════════════════════

/// rights_short 表查找 — 已知掩码命中（最快路径）
#[divan::bench(sample_count = 2000)]
fn rights_short_known(bencher: Bencher) {
    use xun::acl::types::rights_short;
    let masks = [2_032_127u32, 1_245_631, 1_179_817, 1_179_785, 278];
    bencher.bench(|| {
        for &m in &masks {
            black_box(rights_short(black_box(m)));
        }
    });
}

/// rights_short 表查找 — 未知掩码（回退 hex 格式化，最慢路径）
#[divan::bench(sample_count = 2000)]
fn rights_short_unknown(bencher: Bencher) {
    use xun::acl::types::rights_short;
    let masks = [0x0000_0001u32, 0xDEAD_BEEF, 0x0042_0000, 0x1234_5678];
    bencher.bench(|| {
        for &m in &masks {
            black_box(rights_short(black_box(m)));
        }
    });
}

/// rights_short 含 Synchronize 位剥离（0x00100000 strip）
#[divan::bench(sample_count = 2000)]
fn rights_short_strip_synchronize(bencher: Bencher) {
    use xun::acl::types::rights_short;
    let mask = 2_032_127u32 | 0x0010_0000;
    bencher.bench(|| black_box(rights_short(black_box(mask))));
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. 孤儿过滤（orphan filter）
// ═══════════════════════════════════════════════════════════════════════════

/// orphan 过滤 — 全部非孤儿（常见场景）
#[divan::bench(args = [100usize, 1000], sample_count = 200)]
fn orphan_filter_none(bencher: Bencher, n: usize) {
    let snap = make_snapshot(n);
    bencher
        .counter(divan::counter::ItemsCount::new(n))
        .bench(|| {
            let count = snap.entries.iter().filter(|e| black_box(e.is_orphan)).count();
            black_box(count);
        });
}

/// orphan 过滤 — 50% 孤儿
#[divan::bench(args = [100usize, 1000], sample_count = 200)]
fn orphan_filter_half(bencher: Bencher, n: usize) {
    let entries: Vec<AceEntry> = (0..n)
        .map(|i| {
            let mut e = make_ace(&format!("S-1-5-21-o-{i}"), 0x1F01FF, AceType::Allow, false);
            e.is_orphan = i % 2 == 0;
            e
        })
        .collect();
    let snap = AclSnapshot {
        path: PathBuf::from(r"C:\bench\orphan"),
        owner: r"BUILTIN\Administrators".to_string(),
        is_protected: false,
        entries,
    };
    bencher
        .counter(divan::counter::ItemsCount::new(n))
        .bench(|| {
            let count = snap.entries.iter().filter(|e| black_box(e.is_orphan)).count();
            black_box(count);
        });
}
