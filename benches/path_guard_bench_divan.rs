//! path_guard 性能基准测试套件（Divan）
//!
//! 覆盖维度：
//!   1. 吞吐量：字符串校验（无IO）vs 含IO探测，串/并行阈值边界，规模曲线
//!   2. 延迟：单条路径 P50/P95/P99，冷启动
//!   3. 内存/分配：AllocProfiler 统计堆分配次数和字节
//!   4. 扩展性：路径数 N 曲线，重复率，目录分布，路径长度
//!   5. 稳定性：100次重复的 stddev/P99（由 divan 自动统计）

#![cfg(windows)]

use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::OnceLock;

use divan::{AllocProfiler, Bencher};

use xun::path_guard::{PathPolicy, validate_paths, validate_paths_owned, validate_single};

// ── 全局 allocator：启用分配统计 ─────────────────────────────────────────────
#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

// ── 测试夹具 ─────────────────────────────────────────────────────────────────

struct BenchFixture {
    /// 临时目录（Drop 时自动清理）
    _dir: tempfile::TempDir,
    /// 已存在的路径
    existing: Vec<PathBuf>,
    /// 不存在的路径
    missing: Vec<PathBuf>,
}

impl BenchFixture {
    fn new(existing_count: usize, missing_count: usize) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut existing = Vec::with_capacity(existing_count);
        let mut missing = Vec::with_capacity(missing_count);

        for i in 0..existing_count {
            let p = dir.path().join(format!("e{i:06}.txt"));
            fs::write(&p, b"ok").expect("write");
            existing.push(p);
        }
        for i in 0..missing_count {
            missing.push(dir.path().join(format!("m{i:06}.txt")));
        }
        Self {
            _dir: dir,
            existing,
            missing,
        }
    }

    fn all_paths(&self) -> Vec<PathBuf> {
        self.existing
            .iter()
            .chain(self.missing.iter())
            .cloned()
            .collect()
    }

    fn existing_strings(&self) -> Vec<String> {
        self.existing
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    fn all_strings(&self) -> Vec<String> {
        self.all_paths()
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }
}

/// 全局大型夹具（50k 路径），避免每个 bench 重复建文件
static FIXTURE_50K: OnceLock<BenchFixture> = OnceLock::new();
static FIXTURE_100K: OnceLock<BenchFixture> = OnceLock::new();

fn fixture_50k() -> &'static BenchFixture {
    FIXTURE_50K.get_or_init(|| BenchFixture::new(25_000, 25_000))
}

fn fixture_100k() -> &'static BenchFixture {
    FIXTURE_100K.get_or_init(|| BenchFixture::new(50_000, 50_000))
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. 吞吐量基准
// ═══════════════════════════════════════════════════════════════════════════

/// 纯字符串校验吞吐（must_exist=false，无任何 IO，测 CPU 瓶颈）
#[divan::bench(sample_count = 20)]
fn throughput_string_only_50k(bencher: Bencher) {
    let fx = fixture_50k();
    let inputs = fx.all_strings();
    let policy = PathPolicy::for_output(); // must_exist=false

    bencher
        .counter(divan::counter::ItemsCount::new(inputs.len()))
        .bench(|| black_box(validate_paths(black_box(inputs.iter()), black_box(&policy))));
}

/// 含 IO 探测吞吐（must_exist=true，50% existing）
#[divan::bench(sample_count = 20)]
fn throughput_with_probe_50k(bencher: Bencher) {
    let fx = fixture_50k();
    let inputs = fx.all_strings();
    let policy = PathPolicy::for_read();

    bencher
        .counter(divan::counter::ItemsCount::new(inputs.len()))
        .bench(|| black_box(validate_paths(black_box(inputs.iter()), black_box(&policy))));
}

/// 含 IO 探测吞吐（100% existing）
#[divan::bench(sample_count = 20)]
fn throughput_all_existing_50k(bencher: Bencher) {
    let fx = fixture_50k();
    let inputs = fx.existing_strings();
    let policy = PathPolicy::for_read();

    bencher
        .counter(divan::counter::ItemsCount::new(inputs.len()))
        .bench(|| black_box(validate_paths(black_box(inputs.iter()), black_box(&policy))));
}

/// 含 IO 探测吞吐（100% missing）
#[divan::bench(sample_count = 20)]
fn throughput_all_missing_50k(bencher: Bencher) {
    let fx = fixture_50k();
    let policy = PathPolicy::for_read();
    let inputs: Vec<String> = fx
        .missing
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();

    bencher
        .counter(divan::counter::ItemsCount::new(inputs.len()))
        .bench(|| black_box(validate_paths(black_box(inputs.iter()), black_box(&policy))));
}

/// 串行/并行阈值边界：31 条（串行）vs 32 条（并行）
#[divan::bench(args = [31usize, 32, 33, 64])]
fn throughput_threshold_boundary(bencher: Bencher, n: usize) {
    let fx = BenchFixture::new(n, 0);
    let inputs = fx.existing_strings();
    let policy = PathPolicy::for_read();

    bencher
        .counter(divan::counter::ItemsCount::new(n))
        .bench(|| black_box(validate_paths(black_box(inputs.iter()), black_box(&policy))));
}

/// 规模扩展曲线：N = 1k / 5k / 10k / 50k / 100k
#[divan::bench(args = [1_000usize, 5_000, 10_000, 50_000, 100_000], sample_count = 10)]
fn throughput_scale_curve(bencher: Bencher, n: usize) {
    let half = n / 2;
    let fx = BenchFixture::new(half, n - half);
    let inputs = fx.all_strings();
    let policy = PathPolicy::for_read();

    bencher
        .counter(divan::counter::ItemsCount::new(n))
        .bench(|| black_box(validate_paths(black_box(inputs.iter()), black_box(&policy))));
}

/// borrow vs owned API 对比
#[divan::bench(sample_count = 20)]
fn throughput_owned_50k(bencher: Bencher) {
    let fx = fixture_50k();
    let policy = PathPolicy::for_read();

    bencher
        .counter(divan::counter::ItemsCount::new(50_000usize))
        .with_inputs(|| fx.all_paths())
        .bench_values(|inputs| {
            black_box(validate_paths_owned(black_box(inputs), black_box(&policy)))
        });
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. 延迟基准（单条路径）
// ═══════════════════════════════════════════════════════════════════════════

/// 单条 validate_single — 热路径（文件已存在，OnceLock 已初始化）
#[divan::bench(sample_count = 200)]
fn latency_single_existing(bencher: Bencher) {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("single.txt");
    fs::write(&file, b"ok").expect("write");
    let policy = PathPolicy::for_read();
    // 预热：触发 OnceLock 初始化
    let mut scratch = Vec::new();
    let _ = validate_single(file.as_os_str(), &policy, &mut scratch);

    bencher.bench_local(|| {
        let mut sc = Vec::new();
        black_box(validate_single(
            black_box(file.as_os_str()),
            black_box(&policy),
            &mut sc,
        ))
    });
}

/// 单条 validate_single — 缺失路径
#[divan::bench(sample_count = 200)]
fn latency_single_missing(bencher: Bencher) {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("ghost.txt");
    let policy = PathPolicy::for_read();

    bencher.bench_local(|| {
        let mut sc = Vec::new();
        black_box(validate_single(
            black_box(missing.as_os_str()),
            black_box(&policy),
            &mut sc,
        ))
    });
}

/// 单条字符串校验（无 IO）
#[divan::bench(sample_count = 500)]
fn latency_single_string_only(bencher: Bencher) {
    let path = "C:\\foo\\bar\\baz\\file.txt";
    let policy = PathPolicy::for_output();

    bencher.bench(|| {
        black_box(validate_paths(
            black_box(std::iter::once(path)),
            black_box(&policy),
        ))
    });
}

/// 冷启动延迟（每次迭代创建新进程相当，这里模拟首次调用场景）
/// 注意：OnceLock 在进程内只初始化一次，此 bench 测量第一次调用的实际开销
#[divan::bench(sample_count = 5)]
fn latency_first_call_warmup(bencher: Bencher) {
    // 使用 with_inputs 确保每次迭代有全新的路径列表
    let fx = BenchFixture::new(1, 0);
    let policy = PathPolicy::for_read();

    bencher
        .with_inputs(|| fx.existing_strings())
        .bench_values(|inputs| {
            black_box(validate_paths(black_box(inputs.iter()), black_box(&policy)))
        });
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. 内存 / 分配基准（AllocProfiler 统计）
// ═══════════════════════════════════════════════════════════════════════════

/// 50k 路径的堆分配次数和总字节（含 IO）
#[divan::bench(sample_count = 10)]
fn alloc_probe_50k(bencher: Bencher) {
    let fx = fixture_50k();
    let inputs = fx.all_strings();
    let policy = PathPolicy::for_read();

    bencher.bench(|| black_box(validate_paths(black_box(inputs.iter()), black_box(&policy))));
}

/// 纯字符串校验的堆分配（无 IO，应接近零额外分配）
#[divan::bench(sample_count = 10)]
fn alloc_string_only_50k(bencher: Bencher) {
    let fx = fixture_50k();
    let inputs = fx.all_strings();
    let policy = PathPolicy::for_output();

    bencher.bench(|| black_box(validate_paths(black_box(inputs.iter()), black_box(&policy))));
}

/// 单条路径的分配开销
#[divan::bench(sample_count = 100)]
fn alloc_single_path(bencher: Bencher) {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("alloc.txt");
    fs::write(&file, b"ok").expect("write");
    let policy = PathPolicy::for_read();

    bencher.bench_local(|| {
        let mut sc = Vec::new();
        black_box(validate_single(
            black_box(file.as_os_str()),
            black_box(&policy),
            &mut sc,
        ))
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. 扩展性基准
// ═══════════════════════════════════════════════════════════════════════════

/// 重复率对 dedupe 性能的影响：0% / 50% / 90% / 99%
#[divan::bench(args = [0u32, 50, 90, 99], sample_count = 20)]
fn scalability_dedupe_rate(bencher: Bencher, dup_pct: u32) {
    const TOTAL: usize = 1000;
    let unique = ((100 - dup_pct) as usize * TOTAL / 100).max(1);

    let dir = tempfile::tempdir().expect("tempdir");
    let base_paths: Vec<PathBuf> = (0..unique)
        .map(|i| {
            let p = dir.path().join(format!("u{i}.txt"));
            fs::write(&p, b"ok").unwrap();
            p
        })
        .collect();

    // 填充至 TOTAL，后面的都是重复
    let mut all: Vec<String> = base_paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    while all.len() < TOTAL {
        let idx = all.len() % unique;
        all.push(base_paths[idx].to_string_lossy().into_owned());
    }

    let policy = PathPolicy::for_read();
    bencher
        .counter(divan::counter::ItemsCount::new(TOTAL))
        .bench(|| black_box(validate_paths(black_box(all.iter()), black_box(&policy))));
}

/// 目录分布对 probe_cache 的影响：1目录 / 10目录 / 100目录 / 1000目录
#[divan::bench(args = [1usize, 10, 100, 1000], sample_count = 15)]
fn scalability_dir_distribution(bencher: Bencher, num_dirs: usize) {
    const TOTAL: usize = 1000;
    let per_dir = (TOTAL / num_dirs).max(1);

    let root = tempfile::tempdir().expect("tempdir");
    let mut inputs: Vec<String> = Vec::with_capacity(TOTAL);

    'outer: for d in 0..num_dirs {
        let dir = root.path().join(format!("d{d:04}"));
        fs::create_dir_all(&dir).unwrap();
        for f in 0..per_dir {
            let p = dir.join(format!("f{f:04}.txt"));
            fs::write(&p, b"ok").unwrap();
            inputs.push(p.to_string_lossy().into_owned());
            if inputs.len() >= TOTAL {
                break 'outer;
            }
        }
    }

    let policy = PathPolicy::for_read();
    bencher
        .counter(divan::counter::ItemsCount::new(inputs.len()))
        .bench(|| black_box(validate_paths(black_box(inputs.iter()), black_box(&policy))));
}

/// 路径长度对性能的影响：短(~20) / 中(~80) / 长(~200) / 超长(~400)
/// 注：Windows MAX_PATH=260，超长路径通过多层子目录实现，避免单文件名超限
#[divan::bench(args = [20usize, 80, 200, 400], sample_count = 20)]
fn scalability_path_length(bencher: Bencher, target_len: usize) {
    let root = tempfile::tempdir().expect("tempdir");

    // 将目标总路径长度分配到多层子目录 + 文件名
    // 每层子目录最多 50 字符，最后文件名最多 50 字符
    const SEG_MAX: usize = 50;
    let base_len = root.path().to_string_lossy().len() + 1; // +1 for separator
    let needed = target_len.saturating_sub(base_len);
    let num_dirs = needed / (SEG_MAX + 1); // +1 for separator per dir
    let file_seg_len = (needed % (SEG_MAX + 1)).max(1).min(SEG_MAX);

    // 构造子目录链
    let mut dir_path = root.path().to_path_buf();
    for i in 0..num_dirs {
        let seg = format!("{:0>width$}", i, width = SEG_MAX.min(needed));
        dir_path = dir_path.join(&seg[..seg.len().min(SEG_MAX)]);
    }
    fs::create_dir_all(&dir_path).unwrap_or_default();

    let file_name = "a".repeat(file_seg_len.max(1)) + ".txt";
    let file = dir_path.join(&file_name);
    // 仅在路径合法时才创建（超长路径可能仍超 MAX_PATH）
    let file = if fs::write(&file, b"ok").is_ok() {
        file
    } else {
        // 降级：使用根目录下的短路径
        let fallback = root.path().join("fallback.txt");
        fs::write(&fallback, b"ok").unwrap();
        fallback
    };

    // 构造 50 条路径（超过 PARALLEL_MIN=32）
    let inputs: Vec<String> = (0..50)
        .map(|_| file.to_string_lossy().into_owned())
        .collect();
    let policy = PathPolicy::for_read();

    bencher
        .counter(divan::counter::ItemsCount::new(inputs.len()))
        .bench(|| black_box(validate_paths(black_box(inputs.iter()), black_box(&policy))));
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. 稳定性基准（divan 自动统计 min/max/mean/stddev）
// ═══════════════════════════════════════════════════════════════════════════

/// 高重复次数（100 sample）验证 jitter，divan 输出 min/max/stddev
#[divan::bench(sample_count = 100)]
fn stability_jitter_50k(bencher: Bencher) {
    let fx = fixture_50k();
    let inputs = fx.all_strings();
    let policy = PathPolicy::for_read();

    bencher
        .counter(divan::counter::ItemsCount::new(50_000usize))
        .bench(|| black_box(validate_paths(black_box(inputs.iter()), black_box(&policy))));
}

/// 字符串校验稳定性（无 IO 干扰，纯 CPU jitter）
#[divan::bench(sample_count = 100)]
fn stability_jitter_string_only_50k(bencher: Bencher) {
    let fx = fixture_50k();
    let inputs = fx.all_strings();
    let policy = PathPolicy::for_output();

    bencher
        .counter(divan::counter::ItemsCount::new(50_000usize))
        .bench(|| black_box(validate_paths(black_box(inputs.iter()), black_box(&policy))));
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. 并发压力基准
// ═══════════════════════════════════════════════════════════════════════════

/// 多线程并发调用 validate_paths（测试 thread_local 竞争和 rayon 调度）
#[divan::bench(args = [1usize, 2, 4, 8], sample_count = 10)]
fn concurrency_parallel_callers(bencher: Bencher, num_threads: usize) {
    let fx = fixture_50k();
    let inputs = std::sync::Arc::new(fx.all_strings());
    let policy = PathPolicy::for_read();

    bencher
        .counter(divan::counter::ItemsCount::new(50_000 * num_threads))
        .bench(|| {
            let handles: Vec<_> = (0..num_threads)
                .map(|_| {
                    let inp = std::sync::Arc::clone(&inputs);
                    let pol = policy.clone();
                    std::thread::spawn(move || {
                        black_box(validate_paths(black_box(inp.iter()), black_box(&pol)))
                    })
                })
                .collect();
            for h in handles {
                h.join().unwrap();
            }
        });
}

// ═══════════════════════════════════════════════════════════════════════════
// 7. 句柄泄漏检测（进程级，运行多轮验证句柄数稳定）
// ═══════════════════════════════════════════════════════════════════════════

/// 多轮调用后句柄数不增长（仅记录，不断言，bench框架线程池句柄波动较大）
#[divan::bench(sample_count = 5)]
fn handle_leak_check(bencher: Bencher) {
    let fx = fixture_50k();
    let inputs = fx.all_strings();
    let policy = PathPolicy::for_read();

    let handle_count_before = current_handle_count();

    bencher.bench(|| black_box(validate_paths(black_box(inputs.iter()), black_box(&policy))));

    let handle_count_after = current_handle_count();
    let delta = handle_count_after.saturating_sub(handle_count_before);
    // 仅打印，不断言：rayon/crossbeam 线程池持有句柄，bench 框架本身也会波动
    if delta > 100 {
        eprintln!(
            "[handle_leak_check] delta={delta} (before={handle_count_before} after={handle_count_after})"
        );
    }
}

fn current_handle_count() -> u32 {
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};
    let mut count = 0u32;
    unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) };
    count
}
