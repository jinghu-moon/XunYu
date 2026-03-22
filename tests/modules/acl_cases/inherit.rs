use super::common::*;
use crate::common::*;
use std::fs;

#[test]
fn acl_inherit_copy_backup_restore_and_batch() {
    let env = TestEnv::new();
    let dir_a = setup_acl_dir(&env, "acl_inherit_a");
    let dir_b = setup_acl_dir(&env, "acl_inherit_b");
    let backup = env.root.join("acl_backup.json");
    let export_dir = env.root.join("acl_exports");
    fs::create_dir_all(&export_dir).unwrap();

    run_ok(acl_cmd(&env).args(["acl", "inherit", "-p", &str_path(&dir_a), "--disable"]));

    run_ok(acl_cmd(&env).args([
        "acl",
        "copy",
        "-p",
        &str_path(&dir_b),
        "-r",
        &str_path(&dir_a),
        "-y",
    ]));

    backup_acl_to(&env, &dir_a, &backup);
    assert!(backup.exists(), "backup file not created");

    restore_acl(&env, &dir_b, &backup);

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

    let out = run_ok(acl_cmd(&env).args(["acl", "view", "-p", &str_path(&dir)]));
    let err = stderr_str(&out);
    assert!(
        err.contains("Inherit: disabled"),
        "expected inheritance disabled: {err}"
    );

    run_ok(acl_cmd(&env).args(["acl", "inherit", "-p", &str_path(&dir), "--enable"]));

    let out = run_ok(acl_cmd(&env).args(["acl", "view", "-p", &str_path(&dir)]));
    let err = stderr_str(&out);
    assert!(
        err.contains("Inherit: enabled"),
        "expected inheritance enabled: {err}"
    );
}
