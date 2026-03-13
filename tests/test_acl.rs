#![cfg(windows)]

mod common;

use common::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn acl_cmd(env: &TestEnv) -> Command {
    let mut cmd = env.cmd();
    let local = env.root.join("LocalAppData");
    let desktop = env.root.join("Desktop");
    let _ = fs::create_dir_all(&local);
    let _ = fs::create_dir_all(&desktop);
    cmd.env("LOCALAPPDATA", &local);
    cmd
}

fn setup_acl_dir(env: &TestEnv, name: &str) -> PathBuf {
    let dir = env.root.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("sample.txt"), b"data").unwrap();
    dir
}

fn stderr_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn str_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn acl_audit_path(env: &TestEnv) -> PathBuf {
    env.root
        .join("LocalAppData")
        .join("xun")
        .join("acl_audit.jsonl")
}

fn read_audit_actions(env: &TestEnv) -> Vec<String> {
    let path = acl_audit_path(env);
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
        .collect()
}

fn count_acl_backups(dir: &Path) -> usize {
    let entries = match fs::read_dir(dir) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.extension().map(|v| v == "json").unwrap_or(false)
                && e.file_name().to_string_lossy().starts_with("ACL_")
        })
        .count()
}

fn find_csv_with_prefix(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(prefix) && name.ends_with(".csv") {
            return Some(entry.path());
        }
    }
    None
}

#[allow(deprecated)]
fn is_admin() -> bool {
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}

#[test]
fn acl_view_detail_and_export_csv() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_view");
    let export = env.root.join("acl_view.csv");

    let out = run_ok(acl_cmd(&env).args(["acl", "view", "-p", &str_path(&dir)]));
    let err = stderr_str(&out);
    assert!(err.contains("Owner:"), "missing owner header: {err}");
    assert!(err.contains("Total:"), "missing summary line: {err}");

    let out = run_ok(
        acl_cmd(&env).args(["acl", "view", "-p", &str_path(&dir), "--detail"]),
    );
    let err = stderr_str(&out);
    assert!(err.contains("Rights:"), "missing detail line: {err}");

    run_ok(
        acl_cmd(&env).args([
            "acl",
            "view",
            "-p",
            &str_path(&dir),
            "--export",
            &str_path(&export),
        ]),
    );
    assert!(export.exists(), "export file not created");
    let csv = fs::read_to_string(&export).unwrap();
    assert!(csv.contains("访问类型"), "unexpected export header");
}

#[test]
fn acl_diff_audit_and_export() {
    let env = TestEnv::new();
    let dir_a = setup_acl_dir(&env, "acl_diff_a");
    let dir_b = setup_acl_dir(&env, "acl_diff_b");
    let diff_csv = env.root.join("acl_diff.csv");
    let audit_csv = env.root.join("acl_audit.csv");

    let out = run_ok(acl_cmd(&env).args([
        "acl",
        "diff",
        "-p",
        &str_path(&dir_a),
        "-r",
        &str_path(&dir_b),
        "-o",
        &str_path(&diff_csv),
    ]));
    let err = stderr_str(&out);
    assert!(err.contains("Only in A:"), "missing diff summary: {err}");
    assert!(diff_csv.exists(), "diff csv not created");
    let diff_body = fs::read_to_string(&diff_csv).unwrap();
    assert!(diff_body.contains("差异方向"), "unexpected diff header");

    let out = run_ok(acl_cmd(&env).args(["acl", "audit", "--tail", "1"]));
    let err = stderr_str(&out);
    assert!(err.contains("Diff"), "audit should include Diff entry: {err}");

    run_ok(
        acl_cmd(&env).args(["acl", "audit", "--export", &str_path(&audit_csv)]),
    );
    assert!(audit_csv.exists(), "audit export not created");
}

#[test]
fn acl_diff_reports_inheritance_diff() {
    let env = TestEnv::new();
    let dir_a = setup_acl_dir(&env, "acl_diff_inherit_a");
    let dir_b = setup_acl_dir(&env, "acl_diff_inherit_b");

    run_ok(acl_cmd(&env).args([
        "acl",
        "inherit",
        "-p",
        &str_path(&dir_a),
        "--disable",
        "--preserve",
        "false",
    ]));

    let out = run_ok(acl_cmd(&env).args([
        "acl",
        "diff",
        "-p",
        &str_path(&dir_a),
        "-r",
        &str_path(&dir_b),
    ]));
    let err = stderr_str(&out);
    assert!(
        err.contains("Inheritance differs"),
        "missing inheritance diff: {err}"
    );
}

#[test]
fn acl_diff_reports_owner_diff_when_admin() {
    if !is_admin() {
        return;
    }
    let env = TestEnv::new();
    let dir_a = setup_acl_dir(&env, "acl_diff_owner_a");
    let dir_b = setup_acl_dir(&env, "acl_diff_owner_b");

    let out = run_ok(acl_cmd(&env).args([
        "acl",
        "owner",
        "-p",
        &str_path(&dir_a),
        "--set",
        "BUILTIN\\Administrators",
        "-y",
    ]));
    let err = stderr_str(&out);
    if err.contains("Owner unchanged.") {
        return;
    }

    let out = run_ok(acl_cmd(&env).args([
        "acl",
        "diff",
        "-p",
        &str_path(&dir_a),
        "-r",
        &str_path(&dir_b),
    ]));
    let err = stderr_str(&out);
    assert!(err.contains("Owner differs"), "missing owner diff: {err}");
}

#[test]
fn acl_effective_outputs_masks() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_effective");

    let out = run_ok(acl_cmd(&env).args(["acl", "effective", "-p", &str_path(&dir)]));
    let err = stderr_str(&out);
    assert!(err.contains("User:"), "missing user header: {err}");
    assert!(err.contains("Effective:"), "missing effective masks: {err}");
}

#[test]
fn acl_effective_outputs_masks_for_user() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_effective_user");

    let out = run_ok(acl_cmd(&env).args([
        "acl",
        "effective",
        "-p",
        &str_path(&dir),
        "-u",
        "BUILTIN\\Users",
    ]));
    let err = stderr_str(&out);
    assert!(err.contains("User: BUILTIN\\Users"), "missing user header: {err}");
    assert!(
        err.contains("specified user only"),
        "missing user-only note: {err}"
    );
}

#[test]
fn acl_config_set_persists_and_invalid_rejected() {
    let env = TestEnv::new();

    run_ok(acl_cmd(&env).args([
        "acl",
        "config",
        "--set",
        "throttle_limit",
        "8",
    ]));
    let cfg_path = env.root.join(".xun.config.json");
    let raw = fs::read_to_string(&cfg_path).unwrap();
    let v: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["acl"]["throttle_limit"].as_u64(), Some(8));

    let out = run_err(acl_cmd(&env).args(["acl", "config", "--set", "throttle_limit"]));
    let err = stderr_str(&out);
    assert!(err.contains("requires KEY VALUE"), "unexpected error: {err}");
}

#[test]
fn acl_config_sets_all_keys_and_rejects_unknown() {
    let env = TestEnv::new();
    let audit_path = env.root.join("audit_path.jsonl");
    let export_path = env.root.join("exports");
    let audit_path_s = str_path(&audit_path);
    let export_path_s = str_path(&export_path);
    fs::create_dir_all(&export_path).unwrap();

    run_ok(acl_cmd(&env).args([
        "acl",
        "config",
        "--set",
        "chunk_size",
        "64",
    ]));
    run_ok(acl_cmd(&env).args([
        "acl",
        "config",
        "--set",
        "audit_log_path",
        &audit_path_s,
    ]));
    run_ok(acl_cmd(&env).args([
        "acl",
        "config",
        "--set",
        "export_path",
        &export_path_s,
    ]));
    run_ok(acl_cmd(&env).args([
        "acl",
        "config",
        "--set",
        "default_owner",
        "BUILTIN\\Users",
    ]));
    run_ok(acl_cmd(&env).args([
        "acl",
        "config",
        "--set",
        "max_audit_lines",
        "1234",
    ]));

    let cfg_path = env.root.join(".xun.config.json");
    let raw = fs::read_to_string(&cfg_path).unwrap();
    let v: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["acl"]["chunk_size"].as_u64(), Some(64));
    assert_eq!(
        v["acl"]["audit_log_path"].as_str(),
        Some(audit_path_s.as_str())
    );
    assert_eq!(
        v["acl"]["export_path"].as_str(),
        Some(export_path_s.as_str())
    );
    assert_eq!(
        v["acl"]["default_owner"].as_str(),
        Some("BUILTIN\\Users")
    );
    assert_eq!(v["acl"]["max_audit_lines"].as_u64(), Some(1234));

    let out = run_err(acl_cmd(&env).args([
        "acl",
        "config",
        "--set",
        "unknown_key",
        "1",
    ]));
    let err = stderr_str(&out);
    assert!(err.contains("Unknown key"), "unexpected error: {err}");
}

#[test]
fn acl_orphans_empty_reports_clean() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_orphans");

    let out = run_ok(acl_cmd(&env).args([
        "acl",
        "orphans",
        "-p",
        &str_path(&dir),
        "--action",
        "none",
    ]));
    let err = stderr_str(&out);
    assert!(
        err.contains("No orphan SIDs found.") || err.contains("Found"),
        "unexpected orphans output: {err}"
    );
    let actions = read_audit_actions(&env);
    assert!(
        actions.iter().any(|a| a == "ScanOrphans"),
        "missing ScanOrphans audit entry"
    );
}

#[test]
fn acl_orphans_export_delete_both() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_orphans_actions");
    let orphan_sid = "S-1-5-21-123456789-123456789-123456789-1234";

    run_ok(acl_cmd(&env).args([
        "acl",
        "add",
        "-p",
        &str_path(&dir),
        "--principal",
        orphan_sid,
        "--rights",
        "Read",
        "--ace-type",
        "Allow",
        "--inherit",
        "None",
        "-y",
    ]));

    let export = env.root.join("acl_orphans_export.csv");
    run_ok(acl_cmd(&env).args([
        "acl",
        "orphans",
        "-p",
        &str_path(&dir),
        "--action",
        "export",
        "--output",
        &str_path(&export),
    ]));
    assert!(export.exists(), "orphans export not created");

    run_ok(acl_cmd(&env).args([
        "acl",
        "orphans",
        "-p",
        &str_path(&dir),
        "--action",
        "delete",
        "-y",
    ]));
    let actions = read_audit_actions(&env);
    assert!(
        actions.iter().any(|a| a == "PurgeOrphans"),
        "missing PurgeOrphans audit entry"
    );

    run_ok(acl_cmd(&env).args([
        "acl",
        "add",
        "-p",
        &str_path(&dir),
        "--principal",
        orphan_sid,
        "--rights",
        "Read",
        "--ace-type",
        "Allow",
        "--inherit",
        "None",
        "-y",
    ]));

    let export_both = env.root.join("acl_orphans_both.csv");
    run_ok(acl_cmd(&env).args([
        "acl",
        "orphans",
        "-p",
        &str_path(&dir),
        "--action",
        "both",
        "--output",
        &str_path(&export_both),
        "-y",
    ]));
    assert!(export_both.exists(), "orphans both export not created");
}

#[test]
fn acl_add_and_purge_write_audit() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_write");
    let principal = "BUILTIN\\Users";

    run_ok(acl_cmd(&env).args([
        "acl",
        "add",
        "-p",
        &str_path(&dir),
        "--principal",
        principal,
        "--rights",
        "Read",
        "--ace-type",
        "Allow",
        "--inherit",
        "None",
        "-y",
    ]));

    run_ok(acl_cmd(&env).args([
        "acl",
        "purge",
        "-p",
        &str_path(&dir),
        "--principal",
        principal,
        "-y",
    ]));

    let actions = read_audit_actions(&env);
    assert!(
        actions.iter().any(|a| a == "AddPermission"),
        "missing AddPermission audit entry"
    );
    assert!(
        actions.iter().any(|a| a == "PurgePrincipal"),
        "missing PurgePrincipal audit entry"
    );
}

#[test]
fn acl_add_deny_with_inherit() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_add_deny");
    let principal = "S-1-5-21-222222222-333333333-444444444-5555";

    run_ok(acl_cmd(&env).args([
        "acl",
        "add",
        "-p",
        &str_path(&dir),
        "--principal",
        principal,
        "--rights",
        "Write",
        "--ace-type",
        "Deny",
        "--inherit",
        "ContainerOnly",
        "-y",
    ]));

    let out = run_ok(
        acl_cmd(&env).args(["acl", "view", "-p", &str_path(&dir), "--detail"]),
    );
    let err = stderr_str(&out);
    assert!(err.contains(principal), "missing principal in view: {err}");
    assert!(err.contains("Deny"), "missing deny entry: {err}");
    assert!(
        err.contains("ContainerInherit"),
        "missing inheritance flag: {err}"
    );
}

#[test]
fn acl_remove_non_interactive_by_principal() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_remove_non_interactive");
    let principal = "BUILTIN\\Users";

    run_ok(acl_cmd(&env).args([
        "acl",
        "add",
        "-p",
        &str_path(&dir),
        "--principal",
        principal,
        "--rights",
        "Read",
        "--ace-type",
        "Allow",
        "--inherit",
        "None",
        "-y",
    ]));

    run_ok(acl_cmd(&env).args([
        "acl",
        "remove",
        "-p",
        &str_path(&dir),
        "--principal",
        principal,
        "--rights",
        "Read",
        "--ace-type",
        "Allow",
        "-y",
    ]));

    let actions = read_audit_actions(&env);
    assert!(
        actions.iter().any(|a| a == "RemovePermission"),
        "missing RemovePermission audit entry"
    );
}

#[test]
fn acl_inherit_copy_backup_restore_and_batch() {
    let env = TestEnv::new();
    let dir_a = setup_acl_dir(&env, "acl_inherit_a");
    let dir_b = setup_acl_dir(&env, "acl_inherit_b");
    let backup = env.root.join("acl_backup.json");
    let export_dir = env.root.join("acl_exports");
    fs::create_dir_all(&export_dir).unwrap();

    run_ok(acl_cmd(&env).args([
        "acl",
        "inherit",
        "-p",
        &str_path(&dir_a),
        "--disable",
    ]));

    run_ok(acl_cmd(&env).args([
        "acl",
        "copy",
        "-p",
        &str_path(&dir_b),
        "-r",
        &str_path(&dir_a),
        "-y",
    ]));

    run_ok(acl_cmd(&env).args([
        "acl",
        "backup",
        "-p",
        &str_path(&dir_a),
        "-o",
        &str_path(&backup),
    ]));
    assert!(backup.exists(), "backup file not created");

    run_ok(acl_cmd(&env).args([
        "acl",
        "restore",
        "-p",
        &str_path(&dir_b),
        "--from",
        &str_path(&backup),
        "-y",
    ]));

    let paths = format!("{},{}", str_path(&dir_a), str_path(&dir_b));
    run_ok(acl_cmd(&env).args([
        "acl",
        "batch",
        "--paths",
        &paths,
        "--action",
        "backup",
        "--output",
        &str_path(&export_dir),
        "-y",
    ]));
    assert!(
        count_acl_backups(&export_dir) >= 2,
        "expected backups in export dir"
    );

    let actions = read_audit_actions(&env);
    for action in [
        "SetInheritance",
        "CopyAcl",
        "BackupAcl",
        "RestoreAcl",
        "Batch",
    ] {
        assert!(
            actions.iter().any(|a| a == action),
            "missing {action} audit entry"
        );
    }
}

#[test]
fn acl_inherit_enable_and_preserve_false() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_inherit_enable");

    run_ok(acl_cmd(&env).args([
        "acl",
        "inherit",
        "-p",
        &str_path(&dir),
        "--disable",
        "--preserve",
        "false",
    ]));

    run_ok(acl_cmd(&env).args([
        "acl",
        "inherit",
        "-p",
        &str_path(&dir),
        "--enable",
    ]));

    let actions = read_audit_actions(&env);
    let count = actions.iter().filter(|a| *a == "SetInheritance").count();
    assert!(count >= 2, "expected multiple SetInheritance entries");
}

#[test]
fn acl_batch_orphans_inherit_reset_and_error_csv() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_batch_orphans");
    let export_dir = env.root.join("acl_batch_exports");
    fs::create_dir_all(&export_dir).unwrap();

    run_ok(acl_cmd(&env).args([
        "acl",
        "batch",
        "--paths",
        &str_path(&dir),
        "--action",
        "orphans",
        "--output",
        &str_path(&export_dir),
        "-y",
    ]));

    let missing = env.root.join("missing_path");
    let paths = format!("{},{}", str_path(&dir), str_path(&missing));

    run_ok(acl_cmd(&env).args([
        "acl",
        "batch",
        "--paths",
        &paths,
        "--action",
        "inherit-reset",
        "--output",
        &str_path(&export_dir),
        "-y",
    ]));

    let err_csv = find_csv_with_prefix(&export_dir, "ACLErrors_inherit-reset_");
    assert!(err_csv.is_some(), "missing inherit-reset error csv");
}

#[test]
fn acl_remove_requires_interactive() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_remove");

    run_ok(acl_cmd(&env).args([
        "acl",
        "add",
        "-p",
        &str_path(&dir),
        "--principal",
        "BUILTIN\\Users",
        "--rights",
        "Read",
        "--ace-type",
        "Allow",
        "--inherit",
        "None",
        "-y",
    ]));

    let out = run_err(acl_cmd(&env).args(["acl", "remove", "-p", &str_path(&dir)]));
    let err = stderr_str(&out);
    assert!(
        err.contains("requires interactive mode"),
        "unexpected error: {err}"
    );
}

#[test]
fn acl_repair_requires_confirmation_non_interactive() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_repair_confirm");

    let out = run_err(acl_cmd(&env).args(["acl", "repair", "-p", &str_path(&dir)]));
    let err = stderr_str(&out);
    assert!(
        err.contains("Interactive confirmation required."),
        "unexpected error: {err}"
    );
}

#[test]
fn acl_audit_tail_table_contains_headers() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_audit_tail");

    run_ok(acl_cmd(&env).args([
        "acl",
        "diff",
        "-p",
        &str_path(&dir),
        "-r",
        &str_path(&dir),
    ]));

    let out = run_ok(acl_cmd(&env).args(["acl", "audit", "--tail", "1"]));
    let err = stderr_str(&out);
    assert!(err.contains("Action"), "missing Action header: {err}");
    assert!(err.contains("Status"), "missing Status header: {err}");
    assert!(err.contains("Path"), "missing Path header: {err}");
}

#[test]
fn acl_repair_export_errors_on_failure() {
    let env = TestEnv::new();
    let missing = env.root.join("acl_repair_missing");
    let desktop = env.root.join("Desktop");
    fs::create_dir_all(&desktop).unwrap();

    run_ok(acl_cmd(&env).args([
        "acl",
        "repair",
        "-p",
        &str_path(&missing),
        "--export-errors",
        "-y",
    ]));

    let err_csv = find_csv_with_prefix(&desktop, "ACLErrors_repair_");
    assert!(err_csv.is_some(), "missing repair error csv");
}

#[test]
fn acl_owner_success_when_admin() {
    if !is_admin() {
        return;
    }
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_owner");

    let out = run_ok(acl_cmd(&env).args([
        "acl",
        "owner",
        "-p",
        &str_path(&dir),
        "--set",
        "BUILTIN\\Administrators",
        "-y",
    ]));

    let err = stderr_str(&out);
    let actions = read_audit_actions(&env);
    if err.contains("Owner unchanged.") {
        return;
    }
    assert!(
        actions.iter().any(|a| a == "SetOwner"),
        "missing SetOwner audit entry"
    );
}

#[test]
fn acl_batch_repair_when_admin() {
    if !is_admin() {
        return;
    }
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_batch_repair");

    run_ok(acl_cmd(&env).args([
        "acl",
        "batch",
        "--paths",
        &str_path(&dir),
        "--action",
        "repair",
        "-y",
    ]));
}

#[test]
fn acl_repair_success_when_admin() {
    if !is_admin() {
        return;
    }
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_repair");

    run_ok(acl_cmd(&env).args([
        "acl",
        "repair",
        "-p",
        &str_path(&dir),
        "-y",
    ]));

    let actions = read_audit_actions(&env);
    assert!(
        actions.iter().any(|a| a == "ForceRepair"),
        "missing ForceRepair audit entry"
    );
}
