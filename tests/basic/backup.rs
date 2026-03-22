use crate::common::*;
use std::fs;
use std::thread;
use std::time::Duration;

#[test]
fn backup_creates_backup_folder() {
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
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
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
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t", "--dry-run"]),
    );

    let backups = root.join("A_backups");
    // dry-run 时目录可能不存在，或存在但为空
    let entries: Vec<_> = fs::read_dir(&backups)
        .map(|rd| rd.flatten().collect())
        .unwrap_or_default();
    assert!(
        entries.is_empty(),
        "dry-run should create no backup entries"
    );
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
    assert!(root.join(".xun-bak.json").exists());
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
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

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

// ─── 新增：配置迁移 ───────────────────────────────────────────────────────────

#[test]
fn bak_config_migration_svconfig_to_xun_bak() {
    let env = TestEnv::new();
    let root = env.root.join("proj_migrate");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hello").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    // 写旧名
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "migrate"]),
    );

    // 旧名应已被 rename
    assert!(
        !root.join(".svconfig.json").exists(),
        ".svconfig.json should be migrated"
    );
    // 新名应存在
    assert!(
        root.join(".xun-bak.json").exists(),
        ".xun-bak.json should exist after migration"
    );
    // 备份仍然正常创建
    let backups = root.join("A_backups");
    let found = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().starts_with("v1-"));
    assert!(found, "backup v1 should be created after migration");
}

// ─── 新增：增量备份 ──────────────────────────────────────────────────────────

#[test]
fn bak_incremental_only_copies_changed_files() {
    let env = TestEnv::new();
    let root = env.root.join("proj_incr");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.txt"), "bbb").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt", "b.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    // 全量备份 v1
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "full"]),
    );

    // 只修改 a.txt（等待 mtime 变化，避免同秒误判为 Unchanged）
    thread::sleep(Duration::from_secs(2));
    fs::write(root.join("a.txt"), "aaa-modified").unwrap();

    // 增量备份 v2
    run_ok(env.cmd().args([
        "bak",
        "-C",
        root.to_str().unwrap(),
        "-m",
        "incr",
        "--incremental",
    ]));

    let backups = root.join("A_backups");
    let v2 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.starts_with("v2-") && n.ends_with("-incr")
        })
        .expect("v2 incremental backup not found");

    let v2_path = v2.path();
    // 增量备份应包含 a.txt（修改过）
    assert!(
        v2_path.join("a.txt").exists(),
        "a.txt should be in incremental backup"
    );
    // b.txt 未修改，增量备份不含它
    assert!(
        !v2_path.join("b.txt").exists(),
        "b.txt should NOT be in incremental backup"
    );
}

// ─── 新增：list mtime 格式化 ─────────────────────────────────────────────────

#[test]
fn bak_list_shows_human_readable_mtime() {
    let env = TestEnv::new();
    let root = env.root.join("proj_list_mtime");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let out = env
        .cmd()
        .args(["bak", "-C", root.to_str().unwrap(), "list"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    // mtime 应为 "20xx-" 格式，而非纯数字时间戳
    assert!(
        stderr.contains("20") && stderr.contains("-"),
        "list mtime should be human-readable date (got: {stderr})"
    );
    // 不应含纯数字大时间戳（Unix epoch > 1700000000）
    assert!(
        !stderr.contains("17000000") && !stderr.contains("16000000"),
        "list should not show raw unix timestamp (got: {stderr})"
    );
}

#[test]
fn backup_list_empty_reports_no_backups() {
    let env = TestEnv::new();
    let root = env.root.join("proj_list_empty");
    fs::create_dir_all(&root).unwrap();

    let out = run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "list"]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("No backups found"),
        "empty list should explain no backups found, got: {stderr}"
    );
}

// ─── 新增：verify 命令 ────────────────────────────────────────────────────────

#[test]
fn bak_verify_no_manifest_returns_error() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    // 找到备份名
    let backups = root.join("A_backups");
    let v1_name = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .file_name()
        .to_string_lossy()
        .into_owned();

    // 未启用 bak feature，无 manifest 文件 → verify 应报错（NoManifest）
    let out = env
        .cmd()
        .args(["bak", "-C", root.to_str().unwrap(), "verify", &v1_name])
        .output()
        .unwrap();
    // 无 manifest 时应以非零退出码退出
    assert!(
        !out.status.success(),
        "verify should fail when no manifest exists"
    );
}

#[test]
fn bak_verify_zip_backup_reports_not_supported() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify_zip");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": true },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "ziponly"]),
    );

    let zip_name = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".zip").map(str::to_string)
        })
        .expect("zip backup should exist");

    let out = run_err(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "verify", &zip_name]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Verify is only supported for directory backups"),
        "zip verify should explain unsupported mode, got: {stderr}"
    );
}

// ─── 新增：find 命令 ──────────────────────────────────────────────────────────

#[test]
fn bak_find_lists_backups_with_meta() {
    let env = TestEnv::new();
    let root = env.root.join("proj_find");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "myfind"]),
    );

    // find 无过滤条件 → 列出有 meta 的备份
    let out = env
        .cmd()
        .args(["bak", "-C", root.to_str().unwrap(), "find"])
        .output()
        .unwrap();
    assert!(out.status.success(), "find should succeed");
    let stderr = String::from_utf8_lossy(&out.stderr);
    // 应包含备份名（v1-...）
    assert!(
        stderr.contains("v1") || stderr.contains("myfind"),
        "find output should list backup (got: {stderr})"
    );
}

#[test]
fn bak_find_filters_backups_by_tag() {
    let env = TestEnv::new();
    let root = env.root.join("proj_find_tag");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "tagged"]),
    );
    fs::write(root.join("a.txt"), "changed").unwrap();
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "plain"]),
    );

    let backups_root = root.join("A_backups");
    let mut entries: Vec<_> = fs::read_dir(&backups_root)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let tagged_backup = entries
        .iter()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .expect("v1 backup should exist")
        .path();
    let plain_backup = entries
        .iter()
        .find(|e| e.file_name().to_string_lossy().starts_with("v2-"))
        .expect("v2 backup should exist")
        .path();

    fs::write(
        tagged_backup.join(".bak-meta.json"),
        serde_json::json!({
            "version": 1,
            "ts": 1_700_000_000u64,
            "desc": "tagged",
            "tags": ["demo"],
            "stats": { "new": 1, "modified": 0, "deleted": 0 },
            "incremental": false
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        plain_backup.join(".bak-meta.json"),
        serde_json::json!({
            "version": 1,
            "ts": 1_700_000_100u64,
            "desc": "plain",
            "tags": [],
            "stats": { "new": 1, "modified": 0, "deleted": 0 },
            "incremental": false
        })
        .to_string(),
    )
    .unwrap();

    let out = run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "find", "demo"]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("tagged"), "filtered find should keep tagged backup");
    assert!(
        !stderr.contains("plain"),
        "filtered find should exclude untagged backup, got: {stderr}"
    );
}

// ─── 新增：retention 时间窗口 ─────────────────────────────────────────────────

#[test]
fn bak_retention_keep_daily_preserves_one_per_day() {
    let env = TestEnv::new();
    let root = env.root.join("proj_keep_daily");
    let data = root.join("data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("a.txt"), "v1").unwrap();

    // maxBackups=2 但 keepDaily=3：即使超出 maxBackups，keepDaily 标记的备份不删
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 2, "deleteCount": 1, "keepDaily": 3 },
  "include": [ "data" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    // 创建 3 次备份（同一天内，所有备份属同一 day bucket）
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "d1"]),
    );
    fs::write(data.join("a.txt"), "v2").unwrap();
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "d2"]),
    );
    fs::write(data.join("a.txt"), "v3").unwrap();
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "d3"]),
    );

    let backups = root.join("A_backups");
    let versions: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with('v') && n.contains('-'))
        .collect();

    // keepDaily=3 保护今天的 1 个代表（最新），maxBackups=2 限制总数，
    // 但至少保留 keepDaily 保护的条目，所以不应全删到 0
    // 实际结果：3 条备份，overflow=1，to_delete=1，但 keep[最新]=true，
    // 只删最旧未标记的 → 剩余 2 条
    assert!(
        versions.len() >= 2,
        "keepDaily should protect recent backups, got: {versions:?}"
    );
    // 最新备份（v3）必须被保留
    assert!(
        versions.iter().any(|n| n.starts_with("v3-")),
        "latest backup v3 should be kept by keepDaily, got: {versions:?}"
    );
}

#[test]
fn bak_retention_max_backups_without_time_window() {
    // 无时间窗口时，超出 maxBackups 严格按最旧优先删除
    let env = TestEnv::new();
    let root = env.root.join("proj_max_only");
    let data = root.join("data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("a.txt"), "v1").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 3, "deleteCount": 1 },
  "include": [ "data" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    for i in 1..=5 {
        fs::write(data.join("a.txt"), format!("v{i}")).unwrap();
        run_ok(
            env.cmd()
                .args(["bak", "-C", root.to_str().unwrap(), "-m", &format!("v{i}")]),
        );
    }

    let backups = root.join("A_backups");
    let mut versions: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with('v') && n.contains('-'))
        .collect();
    versions.sort();

    // maxBackups=3，每次备份后超出 1 个就删 1 个最旧的
    // v1 在 v4 后删，v2 在 v5 后删 → 剩余 v3/v4/v5
    assert_eq!(
        versions.len(),
        3,
        "should retain exactly 3 backups, got: {versions:?}"
    );
    assert!(
        !versions.iter().any(|n| n.starts_with("v1-")),
        "v1 should be deleted"
    );
    assert!(
        !versions.iter().any(|n| n.starts_with("v2-")),
        "v2 should be deleted"
    );
}

// ─── 新增：多级子目录备份 ─────────────────────────────────────────────────────

#[test]
fn bak_nested_directory_scan() {
    let env = TestEnv::new();
    let root = env.root.join("proj_nested");
    let deep = root.join("src").join("components").join("ui");
    fs::create_dir_all(&deep).unwrap();
    fs::write(
        deep.join("button.tsx"),
        "export const Button = () => <button/>",
    )
    .unwrap();
    fs::write(
        root.join("src").join("index.ts"),
        "export * from './components/ui/button'",
    )
    .unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let backups = root.join("A_backups");
    let v1 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();

    // 深层文件应被完整备份
    assert!(
        v1.join("src")
            .join("components")
            .join("ui")
            .join("button.tsx")
            .exists(),
        "nested file should be backed up"
    );
    assert!(
        v1.join("src").join("index.ts").exists(),
        "top-level src file should be backed up"
    );
}
