use crate::common::*;
use serde_json::Value;
use std::fs;
use std::io::{Cursor, Write};

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

#[test]
fn restore_cmd_by_name() {
    let proj = BakProject::new("proj_restore_by_name", false);
    let name = proj.backup_name("v1-", None);

    fs::remove_file(proj.root.join("a.txt")).unwrap();
    fs::remove_file(proj.root.join("b.txt")).unwrap();

    run_ok(
        proj.env
            .cmd()
            .args(["restore", &name, "-C", proj.root.to_str().unwrap(), "-y"]),
    );

    assert!(proj.root.join("a.txt").exists());
    assert!(proj.root.join("b.txt").exists());
    assert_eq!(fs::read_to_string(proj.root.join("a.txt")).unwrap(), "aaa");
    assert_eq!(fs::read_to_string(proj.root.join("b.txt")).unwrap(), "bbb");
}

#[test]
fn rst_cmd_by_name() {
    let proj = BakProject::new("proj_rst_by_name", false);
    let name = proj.backup_name("v1-", None);

    fs::remove_file(proj.root.join("a.txt")).unwrap();
    fs::remove_file(proj.root.join("b.txt")).unwrap();

    run_ok(
        proj.env
            .cmd()
            .args(["rst", &name, "-C", proj.root.to_str().unwrap(), "-y"]),
    );

    assert!(proj.root.join("a.txt").exists());
    assert!(proj.root.join("b.txt").exists());
}

#[test]
fn restore_cmd_by_path() {
    let proj = BakProject::new("proj_restore_by_path", false);
    let name = proj.backup_name("v1-", None);
    let backup_path = proj.root.join("A_backups").join(&name);

    fs::remove_file(proj.root.join("a.txt")).unwrap();

    run_ok(proj.env.cmd().args([
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

    let out =
        run_ok(
            proj.env
                .cmd()
                .args(["restore", &name, "-C", proj.root.to_str().unwrap(), "-y"]),
        );

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

    run_ok(
        proj.env
            .cmd()
            .args(["restore", name, "-C", proj.root.to_str().unwrap(), "-y"]),
    );

    assert!(proj.root.join("a.txt").exists());
    assert!(proj.root.join("b.txt").exists());
    assert_eq!(fs::read_to_string(proj.root.join("a.txt")).unwrap(), "aaa");
    assert_eq!(fs::read_to_string(proj.root.join("b.txt")).unwrap(), "bbb");
}

#[test]
fn restore_cmd_dry_run() {
    let proj = BakProject::new("proj_restore_dry", false);
    let name = proj.backup_name("v1-", None);

    fs::remove_file(proj.root.join("a.txt")).unwrap();

    run_ok(proj.env.cmd().args([
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

    run_ok(
        env.cmd()
            .args(["restore", &name, "-C", root.to_str().unwrap(), "-y"]),
    );

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

    run_ok(
        proj.env
            .cmd()
            .args(["restore", &name, "-C", proj.root.to_str().unwrap(), "-y"]),
    );

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

// ─── 边界：--snapshot + --dry-run 不创建 snapshot ────────────────────────────

#[test]
fn restore_cmd_snapshot_skipped_on_dry_run() {
    let proj = BakProject::new("proj_snapshot_dry", false);
    let name = proj.backup_name("v1-", None);

    run_ok(proj.env.cmd().args([
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
        "restore",
        &name,
        "-C",
        proj.root.to_str().unwrap(),
        "-y",
        "--json",
    ]));

    let value: Value = serde_json::from_slice(&out.stdout).expect("restore json should be valid");
    assert_eq!(value["action"], "restore");
    assert_eq!(value["mode"], "all");
    assert_eq!(value["restored"], 2);
    assert_eq!(value["failed"], 0);
    assert_eq!(value["dry_run"], false);
}
