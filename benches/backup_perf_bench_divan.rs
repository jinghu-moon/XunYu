use divan::{AllocProfiler, Bencher};
use tempfile::tempdir;
use xun::bench_support::backup_perf::{
    prepare_hash_fixture, prepare_restore_fixture, prepare_sidecar_fixture, prepare_verify_fixture,
};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

#[divan::bench]
fn sidecar_build_missing_hash_1000_files(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let fixture = prepare_sidecar_fixture(tmp.path(), 1_000, 16 * 1024, false);
    bencher.bench_local(|| {
        let _ = fixture.build_sidecar_bytes();
    });
}

#[divan::bench]
fn sidecar_build_prehash_1000_files(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let fixture = prepare_sidecar_fixture(tmp.path(), 1_000, 16 * 1024, true);
    bencher.bench_local(|| {
        let _ = fixture.build_sidecar_bytes();
    });
}

#[divan::bench]
fn hash_file_content_64mb(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let fixture = prepare_hash_fixture(tmp.path(), 64 * 1024 * 1024);
    bencher.bench_local(|| {
        let _ = fixture.compute_hash();
    });
}

#[divan::bench]
fn xunbak_restore_all_1000_files(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let fixture = prepare_restore_fixture(tmp.path(), 1_000, 4 * 1024);
    bencher.bench_local(|| {
        fixture.restore_all(&tmp.path().join("restore-target"));
    });
}

#[divan::bench]
fn xunbak_restore_incremental_1000_files(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let fixture = prepare_restore_fixture(tmp.path(), 1_000, 4 * 1024);
    let target = tmp.path().join("restore-target");
    fixture.restore_all(&target);
    bencher.bench_local(|| {
        fixture.restore_all_incremental(&target);
    });
}

#[divan::bench]
fn verify_entries_content_dir_1000_files(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let fixture = prepare_verify_fixture(tmp.path(), 1_000, 4 * 1024);
    bencher.bench_local(|| {
        fixture.verify_dir_entries_content();
    });
}

#[divan::bench]
fn verify_full_xunbak_1000_files(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let fixture = prepare_verify_fixture(tmp.path(), 1_000, 4 * 1024);
    bencher.bench_local(|| {
        fixture.verify_xunbak_full();
    });
}

#[divan::bench]
fn verify_entries_content_xunbak_1000_files(bencher: Bencher) {
    let tmp = tempdir().unwrap();
    let fixture = prepare_verify_fixture(tmp.path(), 1_000, 4 * 1024);
    bencher.bench_local(|| {
        fixture.verify_xunbak_entries_content();
    });
}
