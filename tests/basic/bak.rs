use crate::common::*;
use std::fs;
use std::thread;
use std::time::Duration;

#[test]
fn bak_creates_backup_folder() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let backups = root.join("A_backups");
    let entry = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .expect("backup v1 not found");
    let first = entry.path();
    assert!(first.is_dir());
    assert!(first.join("a.txt").exists());
}

#[test]
fn bak_dry_run_creates_no_version() {
    let env = TestEnv::new();
    let root = env.root.join("proj_dry");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t", "--dry-run"]),
    );

    let backups = root.join("A_backups");
    let entries: Vec<_> = fs::read_dir(&backups).unwrap().flatten().collect();
    assert!(entries.is_empty());
}

#[test]
fn bak_gitignore_excludes_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj_gitignore");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("keep.txt"), "ok").unwrap();
    fs::write(root.join("skip.txt"), "no").unwrap();
    fs::write(root.join(".gitignore"), "skip.txt\n").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "." ],
  "exclude": [],
  "useGitignore": true
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let backups = root.join("A_backups");
    let entry = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .expect("backup v1 not found");
    let first = entry.path();
    assert!(first.join("keep.txt").exists());
    assert!(!first.join("skip.txt").exists());
}

#[test]
fn bak_incremental_reports_new_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj_inc");
    let data = root.join("data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("a.txt"), "one").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "data" ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "first"]),
    );

    fs::write(data.join("b.txt"), "two").unwrap();
    let out = run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "second"]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("+ "));
    assert!(err.contains("data\\b.txt") || err.contains("data/b.txt"));
}

#[test]
fn bak_retention_removes_old_versions() {
    let env = TestEnv::new();
    let root = env.root.join("proj_ret");
    let data = root.join("data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("a.txt"), "one").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 2, "deleteCount": 1 },
  "include": [ "data" ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );
    fs::write(data.join("a.txt"), "two").unwrap();
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v2"]),
    );
    fs::write(data.join("a.txt"), "three").unwrap();
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v3"]),
    );

    let backups = root.join("A_backups");
    let mut versions: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("v") && n.contains('-'))
        .collect();
    versions.sort();

    assert_eq!(versions.len(), 2);
    assert!(!versions.iter().any(|n| n.starts_with("v1-")));
}

#[test]
fn bak_missing_config_auto_creates_default_config() {
    let env = TestEnv::new();
    let root = env.root.join("proj_no_cfg");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let out = run_ok(env.cmd().args([
        "bak",
        "-C",
        root.to_str().unwrap(),
        "--include",
        "a.txt",
        "--no-compress",
        "-m",
        "t",
    ]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(root.join(".svconfig.json").exists());
    assert!(
        err.contains("Auto-created default config"),
        "unexpected stderr:\n{err}"
    );
}

#[test]
fn bak_compress_true_creates_zip() {
    let env = TestEnv::new();
    let root = env.root.join("proj_zip");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": true },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let backups = root.join("A_backups");
    let zip = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("v1-") && n.ends_with(".zip"))
        })
        .expect("backup zip not found");
    assert!(zip.is_file());
}

#[test]
fn bak_list_and_restore_single_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj_restore");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let backups = root.join("A_backups");
    let entry = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .expect("backup v1 not found");
    let name = entry.file_name().to_string_lossy().into_owned();

    let list_out = run_ok(
        env.cmd()
            .args(["bak", "list", "-C", root.to_str().unwrap()]),
    );
    let list_err = String::from_utf8_lossy(&list_out.stderr);
    assert!(
        list_err.contains(&name),
        "bak list should include {name}, got: {list_err}"
    );

    fs::remove_file(root.join("a.txt")).unwrap();
    run_ok(env.cmd().args([
        "bak",
        "restore",
        &name,
        "-C",
        root.to_str().unwrap(),
        "--file",
        "a.txt",
    ]));
    assert!(root.join("a.txt").exists());
}

#[test]
fn bak_incremental_reports_modified_file_with_tilde() {
    let env = TestEnv::new();
    let root = env.root.join("proj_mod");
    fs::create_dir_all(&root).unwrap();
    let file = root.join("a.txt");
    fs::write(&file, "one").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    // bak's incremental change detection treats mtime deltas <= 2s as unchanged (filesystem timestamp slop).
    thread::sleep(Duration::from_secs(3));
    fs::write(&file, "two").unwrap();
    let out = run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v2"]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains('~') && err.contains("a.txt"),
        "expected modified marker in stderr:\n{err}"
    );
}

#[test]
fn bak_incremental_reports_deleted_file_with_minus() {
    let env = TestEnv::new();
    let root = env.root.join("proj_del");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "one").unwrap();
    fs::write(root.join("b.txt"), "two").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "." ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );
    fs::remove_file(root.join("b.txt")).unwrap();

    let out = run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v2"]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("\n- ") && err.contains("b.txt"),
        "expected deleted marker in stderr:\n{err}"
    );
}

#[test]
fn bak_version_increments_v1_v2() {
    let env = TestEnv::new();
    let root = env.root.join("proj_ver");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "first"]),
    );
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "second"]),
    );

    let backups = root.join("A_backups");
    let mut names: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("v") && n.contains('-'))
        .collect();
    names.sort();

    assert!(names.iter().any(|n| n.starts_with("v1-")));
    assert!(names.iter().any(|n| n.starts_with("v2-")));
}
