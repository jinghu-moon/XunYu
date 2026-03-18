use super::common::*;
use crate::common::*;

#[test]
fn acl_orphans_empty_reports_clean() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_orphans");

    let out = run_ok(acl_cmd(&env).args([
        "acl", "orphans", "-p", &str_path(&dir), "--action", "none",
    ]));
    let err = stderr_str(&out);
    assert!(
        err.contains("No orphan SIDs found.") || err.contains("Found"),
        "unexpected orphans output: {err}"
    );
    let actions = read_audit_actions(&env);
    assert!(actions.iter().any(|a| a == "ScanOrphans"), "missing ScanOrphans audit entry");
}

#[test]
fn acl_orphans_export_delete_both() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_orphans_actions");
    let orphan_sid = "S-1-5-21-123456789-123456789-123456789-1234";

    run_ok(acl_cmd(&env).args([
        "acl", "add", "-p", &str_path(&dir),
        "--principal", orphan_sid,
        "--rights", "Read",
        "--ace-type", "Allow",
        "--inherit", "None", "-y",
    ]));

    let export = env.root.join("acl_orphans_export.csv");
    run_ok(acl_cmd(&env).args([
        "acl", "orphans", "-p", &str_path(&dir),
        "--action", "export",
        "--output", &str_path(&export),
    ]));
    assert!(export.exists(), "orphans export not created");
    let rows = read_csv_rows(&export);
    assert!(csv_rows_contain(&rows, orphan_sid), "orphans export missing orphan sid");

    run_ok(acl_cmd(&env).args([
        "acl", "orphans", "-p", &str_path(&dir), "--action", "delete", "-y",
    ]));
    let actions = read_audit_actions(&env);
    assert!(actions.iter().any(|a| a == "PurgeOrphans"), "missing PurgeOrphans audit entry");

    let rows = export_acl_rows(&env, &dir, "acl_orphans_after_delete");
    assert!(
        !has_acl_row(&rows, "Allow", "显式", orphan_sid, "Read", "None", "None", "是"),
        "expected orphan ACE to be removed"
    );

    // re-add and test "both" action
    run_ok(acl_cmd(&env).args([
        "acl", "add", "-p", &str_path(&dir),
        "--principal", orphan_sid,
        "--rights", "Read",
        "--ace-type", "Allow",
        "--inherit", "None", "-y",
    ]));

    let export_both = env.root.join("acl_orphans_both.csv");
    run_ok(acl_cmd(&env).args([
        "acl", "orphans", "-p", &str_path(&dir),
        "--action", "both",
        "--output", &str_path(&export_both), "-y",
    ]));
    assert!(export_both.exists(), "orphans both export not created");
    let rows = read_csv_rows(&export_both);
    assert!(csv_rows_contain(&rows, orphan_sid), "orphans both export missing orphan sid");
}

#[test]
fn acl_batch_orphans_inherit_reset_and_error_csv() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_batch_orphans");
    let export_dir = env.root.join("acl_batch_exports");
    std::fs::create_dir_all(&export_dir).unwrap();

    // batch orphans（正常路径）
    run_ok(acl_cmd(&env).args([
        "acl", "batch", "--paths", &str_path(&dir),
        "--action", "orphans",
        "--output", &str_path(&export_dir), "-y",
    ]));

    // batch inherit-reset，含一个不存在路径 → 生成 error CSV
    let missing = env.root.join("missing_path");
    let paths = format!("{},{}", str_path(&dir), str_path(&missing));
    run_ok(acl_cmd(&env).args([
        "acl", "batch", "--paths", &paths,
        "--action", "inherit-reset",
        "--output", &str_path(&export_dir), "-y",
    ]));

    let err_csv = find_csv_with_prefix(&export_dir, "ACLErrors_inherit-reset_");
    assert!(err_csv.is_some(), "missing inherit-reset error csv");
    let rows = read_csv_rows(err_csv.as_ref().unwrap());
    assert!(
        csv_rows_contain(&rows, "missing_path"),
        "inherit-reset error csv missing path"
    );
}
