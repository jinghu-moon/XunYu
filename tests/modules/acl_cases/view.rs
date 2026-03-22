use super::common::*;
use crate::common::*;

#[test]
fn acl_view_detail_and_export_csv() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_view");
    let export = env.root.join("acl_view.csv");

    let out = run_ok(acl_cmd(&env).args(["acl", "view", "-p", &str_path(&dir)]));
    let err = stderr_str(&out);
    assert!(err.contains("Owner:"), "missing owner header: {err}");
    assert!(err.contains("Total:"), "missing summary line: {err}");

    let out = run_ok(acl_cmd(&env).args(["acl", "view", "-p", &str_path(&dir), "--detail"]));
    let err = stderr_str(&out);
    assert!(err.contains("Rights:"), "missing detail line: {err}");

    run_ok(acl_cmd(&env).args([
        "acl",
        "view",
        "-p",
        &str_path(&dir),
        "--export",
        &str_path(&export),
    ]));
    assert!(export.exists(), "export file not created");
    let csv = std::fs::read_to_string(&export).unwrap();
    assert!(csv.contains("访问类型"), "unexpected export header");
}

#[test]
fn acl_view_missing_path_errors() {
    let env = TestEnv::new();
    let missing = env.root.join("acl_missing_path");

    let out = run_err(acl_cmd(&env).args(["acl", "view", "-p", &str_path(&missing)]));
    let err = stderr_str(&out);
    let err_lower = err.to_lowercase();
    assert!(
        err_lower.contains("not found")
            || err_lower.contains("cannot find")
            || err.contains("找不到"),
        "expected missing path error: {err}"
    );
}

#[test]
fn acl_view_reserved_name_rejected() {
    let env = TestEnv::new();
    let out = run_err(acl_cmd(&env).args(["acl", "view", "-p", "NUL"]));
    let err = stderr_str(&out);
    assert!(
        err.contains("Invalid") || err.contains("reserved") || err.contains("Reserved"),
        "expected reserved name rejection: {err}"
    );
}

#[test]
fn acl_view_invalid_char_rejected() {
    let env = TestEnv::new();
    let out = run_err(acl_cmd(&env).args(["acl", "view", "-p", "C:\\foo<bar"]));
    let err = stderr_str(&out);
    assert!(
        err.contains("Invalid") || err.contains("invalid"),
        "expected invalid char rejection: {err}"
    );
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
    let diff_body = std::fs::read_to_string(&diff_csv).unwrap();
    assert!(diff_body.contains("差异方向"), "unexpected diff header");

    let out = run_ok(acl_cmd(&env).args(["acl", "audit", "--tail", "1"]));
    let err = stderr_str(&out);
    assert!(
        err.contains("Diff"),
        "audit should include Diff entry: {err}"
    );

    run_ok(acl_cmd(&env).args(["acl", "audit", "--export", &str_path(&audit_csv)]));
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
    if stderr_str(&out).contains("Owner unchanged.") {
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
fn acl_diff_missing_target_rejected() {
    let env = TestEnv::new();
    let existing = setup_acl_dir(&env, "acl_diff_missing_target");
    let missing = env.root.join("acl_diff_no_such");

    let out = run_err(acl_cmd(&env).args([
        "acl",
        "diff",
        "-p",
        &str_path(&missing),
        "-r",
        &str_path(&existing),
    ]));
    let err = stderr_str(&out);
    let err_lower = err.to_lowercase();
    assert!(
        err_lower.contains("not found")
            || err_lower.contains("cannot find")
            || err_lower.contains("invalid")
            || err.contains("找不到"),
        "expected missing path error for target: {err}"
    );
}

#[test]
fn acl_diff_missing_reference_rejected() {
    let env = TestEnv::new();
    let existing = setup_acl_dir(&env, "acl_diff_missing_ref");
    let missing = env.root.join("acl_diff_no_ref");

    let out = run_err(acl_cmd(&env).args([
        "acl",
        "diff",
        "-p",
        &str_path(&existing),
        "-r",
        &str_path(&missing),
    ]));
    let err = stderr_str(&out);
    let err_lower = err.to_lowercase();
    assert!(
        err_lower.contains("not found")
            || err_lower.contains("cannot find")
            || err_lower.contains("invalid")
            || err.contains("找不到"),
        "expected missing path error for reference: {err}"
    );
}

#[test]
fn acl_diff_same_path_reports_zero_diff() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_diff_same");

    let out =
        run_ok(acl_cmd(&env).args(["acl", "diff", "-p", &str_path(&dir), "-r", &str_path(&dir)]));
    let err = stderr_str(&out);
    assert!(
        err.contains("Only in A: 0"),
        "expected zero only-in-A: {err}"
    );
    assert!(
        err.contains("Only in B: 0"),
        "expected zero only-in-B: {err}"
    );
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
    assert!(
        err.contains("User: BUILTIN\\Users"),
        "missing user header: {err}"
    );
    assert!(
        err.contains("specified user only"),
        "missing user-only note: {err}"
    );
}

#[test]
fn acl_effective_deny_overrides_allow_cli() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_effective_deny");
    let principal = "BUILTIN\\Users";

    run_ok(acl_cmd(&env).args([
        "acl",
        "add",
        "-p",
        &str_path(&dir),
        "--principal",
        principal,
        "--rights",
        "FullControl",
        "--ace-type",
        "Allow",
        "--inherit",
        "None",
        "-y",
    ]));
    run_ok(acl_cmd(&env).args([
        "acl",
        "add",
        "-p",
        &str_path(&dir),
        "--principal",
        principal,
        "--rights",
        "0x00010000",
        "--ace-type",
        "Deny",
        "--inherit",
        "None",
        "-y",
    ]));

    let out =
        run_ok(acl_cmd(&env).args(["acl", "effective", "-p", &str_path(&dir), "-u", principal]));
    let err = stderr_str(&out);
    assert!(
        err.lines()
            .any(|line| line.contains("Delete") && line.contains("Deny")),
        "expected Delete to be Deny in effective output: {err}"
    );
}

#[test]
fn acl_effective_missing_path_rejected() {
    let env = TestEnv::new();
    let missing = env.root.join("acl_effective_missing");

    let out = run_err(acl_cmd(&env).args(["acl", "effective", "-p", &str_path(&missing)]));
    let err = stderr_str(&out);
    let err_lower = err.to_lowercase();
    assert!(
        err_lower.contains("not found")
            || err_lower.contains("cannot find")
            || err_lower.contains("invalid")
            || err.contains("找不到"),
        "expected missing path error for effective: {err}"
    );
}
