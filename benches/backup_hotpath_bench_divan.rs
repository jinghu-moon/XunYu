//! backup 热路径 micro-benchmark
//!
//! 运行：
//!   cargo bench --bench backup_hotpath_bench_divan

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use divan::{AllocProfiler, Bencher};
use xun::bench_support::backup::{self, CopyBackend};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

struct Fixture {
    root: PathBuf,
    baseline: PathBuf,
    current: PathBuf,
    changed: PathBuf,
    unchanged: PathBuf,
}

static FIXTURE: OnceLock<Fixture> = OnceLock::new();

fn fixture() -> &'static Fixture {
    FIXTURE.get_or_init(|| {
        let root = std::env::temp_dir().join(format!(
            "xun_backup_hotpath_bench_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let seed = root.join("seed");
        let baseline = root.join("baseline");
        let current = root.join("current");
        let changed = root.join("changed");
        let unchanged = root.join("unchanged");

        populate_files(&seed, 500);
        copy_tree_plain(&seed, &baseline);
        copy_tree_plain(&seed, &current);

        let dirs = [
            "src/components",
            "src/utils",
            "src/hooks",
            "src/pages",
            "public",
        ];
        for i in 0..50usize {
            let dir = dirs[i % dirs.len()];
            fs::write(
                current.join(dir).join(format!("file_{i:04}.ts")),
                format!("modified-{i}-{}", "y".repeat(512)),
            )
            .unwrap();
        }

        split_tree(&current, &changed, &unchanged, 50);

        Fixture {
            root,
            baseline,
            current,
            changed,
            unchanged,
        }
    })
}

fn populate_files(root: &PathBuf, n: usize) {
    let dirs = [
        "src/components",
        "src/utils",
        "src/hooks",
        "src/pages",
        "public",
    ];
    for d in &dirs {
        fs::create_dir_all(root.join(d)).unwrap();
    }
    for i in 0..n {
        let d = dirs[i % dirs.len()];
        let size = 500 + (i * 53) % 2500;
        let content = "x".repeat(size);
        fs::write(root.join(d).join(format!("file_{i:04}.ts")), content).unwrap();
    }
}

fn copy_tree_plain(src_root: &PathBuf, dst_root: &PathBuf) {
    let _ = fs::remove_dir_all(dst_root);
    for entry in walk_files(src_root) {
        let rel = entry.strip_prefix(src_root).unwrap();
        let dst = dst_root.join(rel);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::copy(&entry, &dst).unwrap();
    }
}

fn split_tree(
    current_root: &PathBuf,
    changed_root: &PathBuf,
    unchanged_root: &PathBuf,
    changed_count: usize,
) {
    let _ = fs::remove_dir_all(changed_root);
    let _ = fs::remove_dir_all(unchanged_root);
    for entry in walk_files(current_root) {
        let rel = entry.strip_prefix(current_root).unwrap();
        let file_name = rel.file_name().and_then(|s| s.to_str()).unwrap_or_default();
        let index = file_name
            .trim_start_matches("file_")
            .trim_end_matches(".ts")
            .parse::<usize>()
            .unwrap_or(usize::MAX);
        let dst_root = if index < changed_count {
            changed_root
        } else {
            unchanged_root
        };
        let dst = dst_root.join(rel);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::copy(&entry, &dst).unwrap();
    }
}

fn walk_files(root: &PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_files_inner(root, &mut out);
    out
}

fn walk_files_inner(root: &PathBuf, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(root) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_files_inner(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

fn bench_includes() -> Vec<String> {
    vec!["src".to_string(), "public".to_string()]
}

#[divan::bench]
fn read_baseline_500_dir(bencher: Bencher) {
    let fx = fixture();
    bencher.bench(|| backup::read_baseline_len(&fx.baseline));
}

#[divan::bench]
fn scan_and_diff_500_with_50_changed(bencher: Bencher) {
    let fx = fixture();
    let includes = bench_includes();
    bencher.bench(|| backup::scan_and_metadata_diff_count(&fx.current, &fx.baseline, &includes));
}

#[divan::bench]
fn copy_tree_std_50_changed(bencher: Bencher) {
    let fx = fixture();
    let dest = fx.root.join("bench-copy-std");
    bencher.bench(|| {
        let _ = fs::remove_dir_all(&dest);
        backup::copy_tree_with_backend(&fx.changed, &dest, CopyBackend::Std)
    });
}

#[divan::bench]
fn copy_tree_copyfile2_50_changed(bencher: Bencher) {
    let fx = fixture();
    let dest = fx.root.join("bench-copy-copyfile2");
    bencher.bench(|| {
        let _ = fs::remove_dir_all(&dest);
        backup::copy_tree_with_backend(&fx.changed, &dest, CopyBackend::CopyFile2)
    });
}

#[divan::bench]
fn copy_tree_std_450_unchanged(bencher: Bencher) {
    let fx = fixture();
    let dest = fx.root.join("bench-copy-std-unchanged");
    bencher.bench(|| {
        let _ = fs::remove_dir_all(&dest);
        backup::copy_tree_with_backend(&fx.unchanged, &dest, CopyBackend::Std)
    });
}

#[divan::bench]
fn copy_tree_copyfile2_450_unchanged(bencher: Bencher) {
    let fx = fixture();
    let dest = fx.root.join("bench-copy-copyfile2-unchanged");
    bencher.bench(|| {
        let _ = fs::remove_dir_all(&dest);
        backup::copy_tree_with_backend(&fx.unchanged, &dest, CopyBackend::CopyFile2)
    });
}

#[divan::bench]
fn hardlink_tree_450_unchanged(bencher: Bencher) {
    let fx = fixture();
    let dest = fx.root.join("bench-hardlink");
    bencher.bench(|| {
        let _ = fs::remove_dir_all(&dest);
        backup::hardlink_tree(&fx.unchanged, &dest)
    });
}
