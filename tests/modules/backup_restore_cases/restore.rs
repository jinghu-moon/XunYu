use crate::common::*;
use serde_json::Value;
use std::fs;
use std::io::{Cursor, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::MetadataExt;

const CFG_NO_COMPRESS: &str = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["a.txt","b.txt"],"exclude":[]}"#;
const CFG_COMPRESS: &str = r#"{"storage":{"backupsDir":"A_backups","compress":true},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["a.txt","b.txt"],"exclude":[]}"#;

struct BakProject {
    env: TestEnv,
    root: std::path::PathBuf,
}

impl BakProject {
    fn new(name: &str, compress: bool) -> Self {
        let env = TestEnv::new();
        let root = env.root.join(name);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a.txt"), "aaa").unwrap();
        fs::write(root.join("b.txt"), "bbb").unwrap();

        let cfg = if compress {
            CFG_COMPRESS
        } else {
            CFG_NO_COMPRESS
        };
        fs::write(root.join(".xun-bak.json"), cfg).unwrap();

        run_ok(
            env.cmd()
                .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
        );
        Self { env, root }
    }

    fn backup_name(&self, prefix: &str, ext: Option<&str>) -> String {
        let backups = self.root.join("A_backups");
        fs::read_dir(&backups)
            .unwrap()
            .flatten()
            .find(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                n.starts_with(prefix) && ext.map(|ex| n.ends_with(ex)).unwrap_or(true)
            })
            .expect("backup not found")
            .file_name()
            .to_string_lossy()
            .into_owned()
    }
}

fn write_test_zip(path: &std::path::Path, entries: &[(&str, &[u8])]) {
    let cursor = Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in entries {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(path, bytes).unwrap();
}

fn fixture_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("xunbak_sample")
}

fn set_windows_attributes(path: &std::path::Path, attrs: u32) {
    use windows_sys::Win32::Storage::FileSystem::SetFileAttributesW;

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let ok = unsafe { SetFileAttributesW(wide.as_ptr(), attrs) };
    assert_ne!(ok, 0, "failed to set attributes for {}", path.display());
}

fn copy_tree(src: &std::path::Path, dst: &std::path::Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&src_path, &dst_path);
            continue;
        }
        fs::copy(&src_path, &dst_path).unwrap();
        let attrs = fs::metadata(&src_path).unwrap().file_attributes();
        set_windows_attributes(&dst_path, attrs);
    }
}

#[test]
fn top_level_restore_command_is_removed() {
    let env = TestEnv::new();
    let out = run_err(env.cmd().args(["restore", "some-backup"]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Run xun --help for more information.")
            || stderr.contains("Unrecognized argument: restore")
            || stderr.contains("Unrecognized argument"),
        "top-level restore should be removed, got: {stderr}"
    );
}

#[test]
fn backup_restore_subcommand_by_name() {
    let proj = BakProject::new("proj_backup_restore_subcommand", false);
    let name = proj.backup_name("v1-", None);

    fs::remove_file(proj.root.join("a.txt")).unwrap();
    fs::remove_file(proj.root.join("b.txt")).unwrap();

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        proj.root.to_str().unwrap(),
        "-y",
    ]));

    assert!(proj.root.join("a.txt").exists());
    assert!(proj.root.join("b.txt").exists());
    assert_eq!(fs::read_to_string(proj.root.join("a.txt")).unwrap(), "aaa");
    assert_eq!(fs::read_to_string(proj.root.join("b.txt")).unwrap(), "bbb");
}

#[test]
fn backup_restore_subcommand_directory_artifact_by_path() {
    let proj = BakProject::new("proj_backup_restore_dir_artifact", false);
    let name = proj.backup_name("v1-", None);
    let backup_path = proj.root.join("A_backups").join(&name);

    fs::remove_file(proj.root.join("a.txt")).unwrap();
    fs::remove_file(proj.root.join("b.txt")).unwrap();

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        backup_path.to_str().unwrap(),
        "-C",
        proj.root.to_str().unwrap(),
        "-y",
    ]));

    assert_eq!(fs::read_to_string(proj.root.join("a.txt")).unwrap(), "aaa");
    assert_eq!(fs::read_to_string(proj.root.join("b.txt")).unwrap(), "bbb");
}

#[test]
fn backup_restore_subcommand_file_from_directory_artifact_by_path() {
    let proj = BakProject::new("proj_backup_restore_dir_file", false);
    let name = proj.backup_name("v1-", None);
    let backup_path = proj.root.join("A_backups").join(&name);

    fs::write(proj.root.join("a.txt"), "changed").unwrap();

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        backup_path.to_str().unwrap(),
        "-C",
        proj.root.to_str().unwrap(),
        "--file",
        "a.txt",
        "-y",
    ]));

    assert_eq!(fs::read_to_string(proj.root.join("a.txt")).unwrap(), "aaa");
    assert_eq!(fs::read_to_string(proj.root.join("b.txt")).unwrap(), "bbb");
}

#[test]
fn backup_restore_subcommand_glob_from_directory_artifact_by_path() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_restore_dir_glob");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.rs"), "fn main(){}").unwrap();

    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["a.txt","b.rs"],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let backup_path = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();

    fs::remove_file(root.join("a.txt")).unwrap();
    fs::remove_file(root.join("b.rs")).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "restore",
        backup_path.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "--glob",
        "*.txt",
        "-y",
    ]));

    assert!(root.join("a.txt").exists());
    assert!(!root.join("b.rs").exists());
}

#[test]
fn backup_restore_subcommand_zip_by_name() {
    let proj = BakProject::new("proj_backup_restore_zip_subcommand", true);
    let zip_name = proj.backup_name("v1-", Some(".zip"));
    let name = zip_name.trim_end_matches(".zip");

    fs::remove_file(proj.root.join("a.txt")).unwrap();
    fs::remove_file(proj.root.join("b.txt")).unwrap();

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        name,
        "-C",
        proj.root.to_str().unwrap(),
        "-y",
    ]));

    assert!(proj.root.join("a.txt").exists());
    assert!(proj.root.join("b.txt").exists());
    assert_eq!(fs::read_to_string(proj.root.join("a.txt")).unwrap(), "aaa");
    assert_eq!(fs::read_to_string(proj.root.join("b.txt")).unwrap(), "bbb");
}

#[test]
fn backup_restore_subcommand_file_from_zip_backup() {
    let proj = BakProject::new("proj_backup_restore_zip_file", true);
    let zip_name = proj.backup_name("v1-", Some(".zip"));
    let name = zip_name.trim_end_matches(".zip");

    fs::remove_file(proj.root.join("a.txt")).unwrap();

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        name,
        "-C",
        proj.root.to_str().unwrap(),
        "--file",
        "a.txt",
        "-y",
    ]));

    assert_eq!(fs::read_to_string(proj.root.join("a.txt")).unwrap(), "aaa");
    assert_eq!(fs::read_to_string(proj.root.join("b.txt")).unwrap(), "bbb");
}

#[test]
fn backup_restore_subcommand_glob_from_zip_backup() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_restore_zip_glob");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.rs"), "fn main(){}").unwrap();

    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":true},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["a.txt","b.rs"],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let zip_name = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.starts_with("v1-") && name.ends_with(".zip")
        })
        .unwrap()
        .file_name()
        .to_string_lossy()
        .into_owned();
    let name = zip_name.trim_end_matches(".zip").to_string();

    fs::remove_file(root.join("a.txt")).unwrap();
    fs::remove_file(root.join("b.rs")).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        root.to_str().unwrap(),
        "--glob",
        "*.txt",
        "-y",
    ]));

    assert!(root.join("a.txt").exists());
    assert!(!root.join("b.rs").exists());
}

#[test]
fn backup_restore_by_path() {
    let proj = BakProject::new("proj_restore_by_path", false);
    let name = proj.backup_name("v1-", None);
    let backup_path = proj.root.join("A_backups").join(&name);

    fs::remove_file(proj.root.join("a.txt")).unwrap();

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        backup_path.to_str().unwrap(),
        "-C",
        proj.root.to_str().unwrap(),
        "-y",
    ]));

    assert!(proj.root.join("a.txt").exists());
    assert_eq!(fs::read_to_string(proj.root.join("a.txt")).unwrap(), "aaa");
}

#[test]
fn restore_cmd_to_dir() {
    let proj = BakProject::new("proj_restore_to_dir", false);
    let name = proj.backup_name("v1-", None);
    let dest = proj.env.root.join("restored_output");

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        proj.root.to_str().unwrap(),
        "--to",
        dest.to_str().unwrap(),
        "-y",
    ]));

    assert!(
        dest.join("a.txt").exists(),
        "a.txt should exist in --to dir"
    );
    assert!(
        dest.join("b.txt").exists(),
        "b.txt should exist in --to dir"
    );
    assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "aaa");
}

#[test]
fn restore_cmd_glob_txt_only() {
    let env = TestEnv::new();
    let root = env.root.join("proj_restore_glob");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.rs"), "fn main(){}").unwrap();

    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["a.txt","b.rs"],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let name = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .file_name()
        .to_string_lossy()
        .into_owned();

    fs::remove_file(root.join("a.txt")).unwrap();
    fs::remove_file(root.join("b.rs")).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        root.to_str().unwrap(),
        "--glob",
        "*.txt",
        "-y",
    ]));

    assert!(
        root.join("a.txt").exists(),
        "a.txt should be restored by glob"
    );
    assert!(!root.join("b.rs").exists(), "b.rs should NOT be restored");
}

#[test]
fn restore_cmd_snapshot_creates_pre_restore() {
    let proj = BakProject::new("proj_restore_snapshot", false);
    let name = proj.backup_name("v1-", None);

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        proj.root.to_str().unwrap(),
        "--snapshot",
        "-y",
    ]));

    let backups = proj.root.join("A_backups");
    let has_pre_restore = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().contains("pre_restore"));
    assert!(
        has_pre_restore,
        "pre_restore snapshot backup should be created"
    );
}

#[test]
fn restore_cmd_report_counts() {
    let proj = BakProject::new("proj_restore_report", false);
    let name = proj.backup_name("v1-", None);

    fs::remove_file(proj.root.join("a.txt")).unwrap();
    fs::remove_file(proj.root.join("b.txt")).unwrap();

    let out = run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        proj.root.to_str().unwrap(),
        "-y",
    ]));

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Restored:"),
        "stderr should contain 'Restored:' count, got: {stderr}"
    );
}

#[test]
fn restore_cmd_zip_by_name() {
    let proj = BakProject::new("proj_restore_zip", true);
    let zip_name = proj.backup_name("v1-", Some(".zip"));
    let name = zip_name.trim_end_matches(".zip");

    fs::remove_file(proj.root.join("a.txt")).unwrap();
    fs::remove_file(proj.root.join("b.txt")).unwrap();

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        name,
        "-C",
        proj.root.to_str().unwrap(),
        "-y",
    ]));

    assert!(proj.root.join("a.txt").exists());
    assert!(proj.root.join("b.txt").exists());
    assert_eq!(fs::read_to_string(proj.root.join("a.txt")).unwrap(), "aaa");
    assert_eq!(fs::read_to_string(proj.root.join("b.txt")).unwrap(), "bbb");
}

#[test]
fn restore_cmd_7z_by_path() {
    let env = TestEnv::new();
    let root = env.root.join("proj_restore_7z");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.txt"), "bbb").unwrap();
    fs::write(root.join(".xun-bak.json"), CFG_NO_COMPRESS).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
    ]));

    fs::remove_file(root.join("a.txt")).unwrap();
    fs::remove_file(root.join("b.txt")).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("artifact.7z").to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "aaa");
    assert_eq!(fs::read_to_string(root.join("b.txt")).unwrap(), "bbb");
}

#[test]
fn backup_create_dir_then_restore_by_path() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_dir_restore");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.txt"), "bbb").unwrap();
    fs::write(root.join(".xun-bak.json"), CFG_NO_COMPRESS).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        "artifact_dir",
    ]));

    fs::remove_file(root.join("a.txt")).unwrap();
    fs::remove_file(root.join("b.txt")).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("artifact_dir").to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "aaa");
    assert_eq!(fs::read_to_string(root.join("b.txt")).unwrap(), "bbb");
}

#[test]
fn backup_create_zip_then_restore_by_path() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_zip_restore");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.txt"), "bbb").unwrap();
    fs::write(root.join(".xun-bak.json"), CFG_NO_COMPRESS).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        "artifact.zip",
    ]));

    fs::remove_file(root.join("a.txt")).unwrap();
    fs::remove_file(root.join("b.txt")).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("artifact.zip").to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "aaa");
    assert_eq!(fs::read_to_string(root.join("b.txt")).unwrap(), "bbb");
}

#[test]
fn backup_create_split_7z_then_restore_by_base_path() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_split_7z_restore");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "a".repeat(1200)).unwrap();
    fs::write(root.join("b.txt"), "b".repeat(1200)).unwrap();
    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["a.txt","b.txt"],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
        "--no-compress",
        "--split-size",
        "1400",
    ]));

    fs::remove_file(root.join("a.txt")).unwrap();
    fs::remove_file(root.join("b.txt")).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("artifact.7z").to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert_eq!(
        fs::read_to_string(root.join("a.txt")).unwrap(),
        "a".repeat(1200)
    );
    assert_eq!(
        fs::read_to_string(root.join("b.txt")).unwrap(),
        "b".repeat(1200)
    );
}

#[test]
fn backup_create_split_7z_then_restore_by_first_volume_path() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_split_7z_restore_first_volume");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "a".repeat(1200)).unwrap();
    fs::write(root.join("b.txt"), "b".repeat(1200)).unwrap();
    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["a.txt","b.txt"],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
        "--no-compress",
        "--split-size",
        "1400",
    ]));

    fs::remove_file(root.join("a.txt")).unwrap();
    fs::remove_file(root.join("b.txt")).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("artifact.7z.001").to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert_eq!(
        fs::read_to_string(root.join("a.txt")).unwrap(),
        "a".repeat(1200)
    );
    assert_eq!(
        fs::read_to_string(root.join("b.txt")).unwrap(),
        "b".repeat(1200)
    );
}

#[test]
fn backup_create_zip_fixture_roundtrip_preserves_unicode_spaces_empty_and_deep_paths() {
    let env = TestEnv::new();
    let root = env.root.join("proj_fixture_zip");
    copy_tree(&fixture_root(), &root);
    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["README.md","empty.txt","中文目录","path with spaces","deep","config","docs","src","assets"],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        "fixture.zip",
    ]));

    let restore = env.root.join("fixture_zip_restore");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("fixture.zip").to_str().unwrap(),
        "--to",
        restore.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert_eq!(
        fs::read_to_string(restore.join("中文目录").join("说明.txt")).unwrap(),
        fs::read_to_string(root.join("中文目录").join("说明.txt")).unwrap()
    );
    assert_eq!(
        fs::read_to_string(
            restore
                .join("path with spaces")
                .join("nested folder")
                .join("notes file.txt")
        )
        .unwrap(),
        fs::read_to_string(
            root.join("path with spaces")
                .join("nested folder")
                .join("notes file.txt")
        )
        .unwrap()
    );
    assert_eq!(
        fs::metadata(restore.join("empty.txt")).unwrap().len(),
        0,
        "empty file should remain empty"
    );
    assert_eq!(
        fs::read_to_string(
            restore
                .join("deep")
                .join("level1")
                .join("level2")
                .join("level3")
                .join("level4")
                .join("leaf.txt")
        )
        .unwrap(),
        fs::read_to_string(
            root.join("deep")
                .join("level1")
                .join("level2")
                .join("level3")
                .join("level4")
                .join("leaf.txt")
        )
        .unwrap()
    );
}

#[test]
fn backup_create_7z_fixture_roundtrip_preserves_unicode_spaces_empty_and_deep_paths() {
    let env = TestEnv::new();
    let root = env.root.join("proj_fixture_7z");
    copy_tree(&fixture_root(), &root);
    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["README.md","empty.txt","中文目录","path with spaces","deep","config","docs","src","assets"],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "fixture.7z",
    ]));

    let restore = env.root.join("fixture_7z_restore");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("fixture.7z").to_str().unwrap(),
        "--to",
        restore.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert_eq!(
        fs::read_to_string(restore.join("中文目录").join("说明.txt")).unwrap(),
        fs::read_to_string(root.join("中文目录").join("说明.txt")).unwrap()
    );
    assert_eq!(
        fs::read_to_string(
            restore
                .join("path with spaces")
                .join("nested folder")
                .join("notes file.txt")
        )
        .unwrap(),
        fs::read_to_string(
            root.join("path with spaces")
                .join("nested folder")
                .join("notes file.txt")
        )
        .unwrap()
    );
    assert_eq!(fs::metadata(restore.join("empty.txt")).unwrap().len(), 0);
    assert_eq!(
        fs::read_to_string(
            restore
                .join("deep")
                .join("level1")
                .join("level2")
                .join("level3")
                .join("level4")
                .join("leaf.txt")
        )
        .unwrap(),
        fs::read_to_string(
            root.join("deep")
                .join("level1")
                .join("level2")
                .join("level3")
                .join("level4")
                .join("leaf.txt")
        )
        .unwrap()
    );
    assert_eq!(
        fs::metadata(root.join("config").join("readonly_file.txt"))
            .unwrap()
            .file_attributes()
            & 0x0000_0001,
        fs::metadata(restore.join("config").join("readonly_file.txt"))
            .unwrap()
            .file_attributes()
            & 0x0000_0001
    );
}

#[test]
fn backup_create_split_7z_fixture_roundtrip_preserves_unicode_spaces_empty_and_deep_paths() {
    let env = TestEnv::new();
    let root = env.root.join("proj_fixture_split_7z");
    copy_tree(&fixture_root(), &root);
    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["README.md","empty.txt","中文目录","path with spaces","deep","config","docs","src","assets"],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "fixture_split.7z",
        "--split-size",
        "40000",
    ]));

    let restore = env.root.join("fixture_split_7z_restore");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("fixture_split.7z").to_str().unwrap(),
        "--to",
        restore.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert_eq!(
        fs::read_to_string(restore.join("中文目录").join("说明.txt")).unwrap(),
        fs::read_to_string(root.join("中文目录").join("说明.txt")).unwrap()
    );
    assert_eq!(
        fs::read_to_string(
            restore
                .join("path with spaces")
                .join("nested folder")
                .join("notes file.txt")
        )
        .unwrap(),
        fs::read_to_string(
            root.join("path with spaces")
                .join("nested folder")
                .join("notes file.txt")
        )
        .unwrap()
    );
    assert_eq!(fs::metadata(restore.join("empty.txt")).unwrap().len(), 0);
    assert_eq!(
        fs::read_to_string(
            restore
                .join("deep")
                .join("level1")
                .join("level2")
                .join("level3")
                .join("level4")
                .join("leaf.txt")
        )
        .unwrap(),
        fs::read_to_string(
            root.join("deep")
                .join("level1")
                .join("level2")
                .join("level3")
                .join("level4")
                .join("leaf.txt")
        )
        .unwrap()
    );
}

#[test]
fn restore_cmd_file_from_7z_backup() {
    let env = TestEnv::new();
    let root = env.root.join("proj_restore_file_7z");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.txt"), "bbb").unwrap();
    fs::write(root.join(".xun-bak.json"), CFG_NO_COMPRESS).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
    ]));

    fs::remove_file(root.join("a.txt")).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("artifact.7z").to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "--file",
        "a.txt",
        "-y",
    ]));

    assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "aaa");
    assert_eq!(fs::read_to_string(root.join("b.txt")).unwrap(), "bbb");
}

#[test]
fn restore_cmd_glob_from_7z_backup() {
    let env = TestEnv::new();
    let root = env.root.join("proj_restore_glob_7z");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.rs"), "fn main(){}").unwrap();
    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["a.txt","b.rs"],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
    ]));

    fs::remove_file(root.join("a.txt")).unwrap();
    fs::remove_file(root.join("b.rs")).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("artifact.7z").to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "--glob",
        "*.txt",
        "-y",
    ]));

    assert!(root.join("a.txt").exists());
    assert!(!root.join("b.rs").exists());
}

#[test]
fn restore_cmd_dry_run() {
    let proj = BakProject::new("proj_restore_dry", false);
    let name = proj.backup_name("v1-", None);

    fs::remove_file(proj.root.join("a.txt")).unwrap();

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        proj.root.to_str().unwrap(),
        "--dry-run",
        "-y",
    ]));

    assert!(
        !proj.root.join("a.txt").exists(),
        "dry-run should not restore files"
    );
}

// ─── 边界：备份名不存在 ────────────────────────────────────────────────────────

#[test]
fn restore_cmd_nonexistent_backup_fails() {
    let env = TestEnv::new();
    let root = env.root.join("proj_nonexistent");
    fs::create_dir_all(&root).unwrap();
    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":[],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    let out = env
        .cmd()
        .args([
            "backup",
            "restore",
            "nonexistent-backup-xyz",
            "-C",
            root.to_str().unwrap(),
            "-y",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "should fail for nonexistent backup");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("Backup"),
        "stderr should mention backup not found, got: {stderr}"
    );
}

// ─── 边界：--file 单文件从目录备份还原，内容覆盖验证 ─────────────────────────

#[test]
fn restore_cmd_file_from_dir_backup() {
    let proj = BakProject::new("proj_file_from_dir", false);
    let name = proj.backup_name("v1-", None);

    // 修改 a.txt 内容（覆盖场景）
    fs::write(proj.root.join("a.txt"), "modified").unwrap();

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        proj.root.to_str().unwrap(),
        "--file",
        "a.txt",
        "-y",
    ]));

    // a.txt 应被还原为备份时的内容
    assert_eq!(
        fs::read_to_string(proj.root.join("a.txt")).unwrap(),
        "aaa",
        "a.txt should be overwritten with backup content"
    );
    // b.txt 不应受影响
    assert_eq!(
        fs::read_to_string(proj.root.join("b.txt")).unwrap(),
        "bbb",
        "b.txt should remain unchanged"
    );
}

// ─── 边界：--file 单文件从 zip 备份还原 ───────────────────────────────────────

#[test]
fn restore_cmd_file_from_zip_backup() {
    let proj = BakProject::new("proj_file_from_zip", true);
    let zip_name = proj.backup_name("v1-", Some(".zip"));
    let name = zip_name.trim_end_matches(".zip");

    // 只删除 a.txt
    fs::remove_file(proj.root.join("a.txt")).unwrap();

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        name,
        "-C",
        proj.root.to_str().unwrap(),
        "--file",
        "a.txt",
        "-y",
    ]));

    assert!(
        proj.root.join("a.txt").exists(),
        "a.txt should be restored from zip"
    );
    assert_eq!(fs::read_to_string(proj.root.join("a.txt")).unwrap(), "aaa");
    // b.txt 应未受影响
    assert!(proj.root.join("b.txt").exists());
    assert_eq!(fs::read_to_string(proj.root.join("b.txt")).unwrap(), "bbb");
}

#[test]
fn restore_cmd_file_missing_in_zip_backup_fails() {
    let proj = BakProject::new("proj_file_missing_in_zip", true);
    let zip_name = proj.backup_name("v1-", Some(".zip"));
    let name = zip_name.trim_end_matches(".zip");

    let out = run_err(proj.env.cmd().args([
        "backup",
        "restore",
        name,
        "-C",
        proj.root.to_str().unwrap(),
        "--file",
        "missing.txt",
        "-y",
    ]));

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("Restore failed"),
        "stderr should indicate missing file in zip backup, got: {stderr}"
    );
}

// ─── 边界：--file 传含 .. 的不安全路径 → 非零退出 ────────────────────────────

#[test]
fn restore_cmd_file_unsafe_path_fails() {
    let proj = BakProject::new("proj_unsafe_path", false);
    let name = proj.backup_name("v1-", None);

    let out = proj
        .env
        .cmd()
        .args([
            "backup",
            "restore",
            &name,
            "-C",
            proj.root.to_str().unwrap(),
            "--file",
            "../../../etc/passwd",
            "-y",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "unsafe path should be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Unsafe") || stderr.contains("unsafe") || stderr.contains("Invalid"),
        "stderr should indicate unsafe path, got: {stderr}"
    );
}

// ─── 边界：--glob **/*.ts 跨子目录匹配 ────────────────────────────────────────

#[test]
fn restore_cmd_glob_double_star() {
    let env = TestEnv::new();
    let root = env.root.join("proj_glob_dstar");
    let src_dir = root.join("src").join("components");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("button.ts"), "export const Button = () => {}").unwrap();
    fs::write(src_dir.join("icon.ts"), "export const Icon = () => {}").unwrap();
    fs::write(root.join("src").join("index.rs"), "fn main(){}").unwrap();

    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["src"],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let name = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .file_name()
        .to_string_lossy()
        .into_owned();

    // 删除所有 src 文件
    fs::remove_file(src_dir.join("button.ts")).unwrap();
    fs::remove_file(src_dir.join("icon.ts")).unwrap();
    fs::remove_file(root.join("src").join("index.rs")).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        root.to_str().unwrap(),
        "--glob",
        "**/*.ts",
        "-y",
    ]));

    // .ts 文件应被还原
    assert!(
        src_dir.join("button.ts").exists(),
        "button.ts should be restored"
    );
    assert!(
        src_dir.join("icon.ts").exists(),
        "icon.ts should be restored"
    );
    // .rs 文件不应被还原（不匹配 glob）
    assert!(
        !root.join("src").join("index.rs").exists(),
        "index.rs should NOT be restored (not matched by **/*.ts)"
    );
}

// ─── 边界：--glob 无匹配文件 → 正常退出，Restored: 0 ─────────────────────────

#[test]
fn restore_cmd_glob_no_match_exits_ok() {
    let proj = BakProject::new("proj_glob_no_match", false);
    let name = proj.backup_name("v1-", None);

    let out = run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        proj.root.to_str().unwrap(),
        "--glob",
        "**/*.xyz_nonexistent",
        "-y",
    ]));

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Restored: 0"),
        "should report Restored: 0 for non-matching glob, got: {stderr}"
    );
}

// ─── 边界：--glob + zip 备份 ───────────────────────────────────────────────────

#[test]
fn restore_cmd_glob_from_zip() {
    let proj = BakProject::new("proj_glob_zip", true);
    let zip_name = proj.backup_name("v1-", Some(".zip"));
    let name = zip_name.trim_end_matches(".zip");

    fs::remove_file(proj.root.join("a.txt")).unwrap();
    fs::remove_file(proj.root.join("b.txt")).unwrap();

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        name,
        "-C",
        proj.root.to_str().unwrap(),
        "--glob",
        "*.txt",
        "-y",
    ]));

    assert!(
        proj.root.join("a.txt").exists(),
        "a.txt should be restored from zip via glob"
    );
    assert!(
        proj.root.join("b.txt").exists(),
        "b.txt should be restored from zip via glob"
    );
}

#[test]
fn restore_cmd_glob_from_zip_rejects_unsafe_entries() {
    let env = TestEnv::new();
    let root = env.root.join("proj_glob_zip_unsafe");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join(".xun-bak.json"),
        r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":[],"exclude":[]}"#,
    )
    .unwrap();

    let zip_path = env.root.join("unsafe_restore_source.zip");
    write_test_zip(&zip_path, &[("safe.txt", b"ok"), ("../evil.txt", b"bad")]);

    let out = run_err(env.cmd().args([
        "backup",
        "restore",
        zip_path.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "--glob",
        "**/*.txt",
        "-y",
    ]));

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unsafe zip entry") || stderr.contains("Unsafe"),
        "stderr should mention unsafe zip entry, got: {stderr}"
    );
    assert!(
        root.join("safe.txt").exists(),
        "safe.txt should still be restored"
    );
    assert!(
        !env.root.join("evil.txt").exists(),
        "unsafe zip entry must not write outside restore root"
    );
}

// ─── 边界：嵌套子目录结构保留 ─────────────────────────────────────────────────

#[test]
fn restore_cmd_preserves_nested_directory_structure() {
    let env = TestEnv::new();
    let root = env.root.join("proj_nested_restore");
    let deep = root.join("src").join("lib").join("utils");
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("helper.txt"), "helper content").unwrap();
    fs::write(root.join("src").join("main.txt"), "main content").unwrap();

    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["src"],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let name = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .file_name()
        .to_string_lossy()
        .into_owned();

    // 完全删除 src 目录
    fs::remove_dir_all(root.join("src")).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    // 嵌套结构应完整重建
    assert!(
        deep.join("helper.txt").exists(),
        "nested helper.txt should be restored"
    );
    assert!(
        root.join("src").join("main.txt").exists(),
        "main.txt should be restored"
    );
    assert_eq!(
        fs::read_to_string(deep.join("helper.txt")).unwrap(),
        "helper content"
    );
    assert_eq!(
        fs::read_to_string(root.join("src").join("main.txt")).unwrap(),
        "main content"
    );
}

#[test]
fn restore_cmd_directory_backup_skips_internal_meta_files() {
    let proj = BakProject::new("proj_restore_skip_internal_meta", false);
    let name = proj.backup_name("v1-", None);

    fs::remove_file(proj.root.join("a.txt")).unwrap();
    fs::remove_file(proj.root.join("b.txt")).unwrap();
    assert!(
        !proj.root.join(".bak-meta.json").exists(),
        "project root should not start with backup metadata"
    );

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        proj.root.to_str().unwrap(),
        "-y",
    ]));

    assert!(proj.root.join("a.txt").exists());
    assert!(proj.root.join("b.txt").exists());
    assert!(
        !proj.root.join(".bak-meta.json").exists(),
        "backup internal metadata must not be restored into project root"
    );
}

// ─── 边界：--dry-run stderr 含 DRY RUN 提示 ───────────────────────────────────

#[test]
fn restore_cmd_dry_run_stderr_mentions_files() {
    let proj = BakProject::new("proj_dry_stderr", false);
    let name = proj.backup_name("v1-", None);

    let out = run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        proj.root.to_str().unwrap(),
        "--dry-run",
        "-y",
    ]));

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("DRY RUN"),
        "dry-run should mention DRY RUN in stderr, got: {stderr}"
    );
}

#[test]
fn restore_cmd_dry_run_lists_directory_entries_in_sorted_order() {
    let env = TestEnv::new();
    let root = env.root.join("proj_dry_order");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("z.txt"), "zzz").unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("nested").join("b.txt"), "bbb").unwrap();
    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["a.txt","z.txt","nested"],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let name = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|entry| entry.file_type().unwrap().is_dir())
        .unwrap()
        .file_name()
        .to_string_lossy()
        .into_owned();

    let out = run_ok(env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        root.to_str().unwrap(),
        "--dry-run",
        "-y",
    ]));

    let stderr = String::from_utf8_lossy(&out.stderr);
    let lines = stderr
        .lines()
        .filter(|line| line.contains("DRY RUN: would restore "))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            "DRY RUN: would restore a.txt",
            "DRY RUN: would restore nested\\b.txt",
            "DRY RUN: would restore z.txt",
        ]
    );
}

// ─── 边界：--snapshot + --dry-run 不创建 snapshot ────────────────────────────

#[test]
fn restore_cmd_snapshot_skipped_on_dry_run() {
    let proj = BakProject::new("proj_snapshot_dry", false);
    let name = proj.backup_name("v1-", None);

    run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        proj.root.to_str().unwrap(),
        "--snapshot",
        "--dry-run",
        "-y",
    ]));

    // dry-run 时不应创建 pre_restore snapshot（只应有 v1 备份）
    let backups = proj.root.join("A_backups");
    let backup_count = fs::read_dir(&backups).unwrap().flatten().count();
    assert_eq!(
        backup_count, 1,
        "dry-run with --snapshot should not create a new backup, got {backup_count}"
    );
}

#[test]
fn restore_cmd_json_outputs_summary() {
    let proj = BakProject::new("proj_restore_json", false);
    let name = proj.backup_name("v1-", None);

    fs::remove_file(proj.root.join("a.txt")).unwrap();
    fs::remove_file(proj.root.join("b.txt")).unwrap();

    let out = run_ok(proj.env.cmd().args([
        "backup",
        "restore",
        &name,
        "-C",
        proj.root.to_str().unwrap(),
        "-y",
        "--json",
    ]));

    let value: Value = serde_json::from_slice(&out.stdout).expect("restore json should be valid");
    assert_eq!(value["action"], "restore");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["mode"], "all");
    assert_eq!(value["restored"], 2);
    assert_eq!(value["skipped_unchanged"], 0);
    assert_eq!(value["failed"], 0);
    assert_eq!(value["dry_run"], false);
}

#[test]
fn restore_cmd_reports_skipped_unchanged_for_xunbak() {
    let env = TestEnv::new();
    let root = env.root.join("proj_restore_skip_unchanged");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    let cfg = r#"{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"backup"},"retention":{"maxBackups":5,"deleteCount":1},"include":["a.txt"],"exclude":[]}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "artifact.xunbak",
    ]));

    let out_dir = root.join("restore-target");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("artifact.xunbak").to_str().unwrap(),
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    let out = run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("artifact.xunbak").to_str().unwrap(),
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Restored: 0"));
    assert!(stderr.contains("Skipped: 1"));
}
