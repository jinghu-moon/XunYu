use super::common::*;
use crate::common::*;

#[test]
fn acl_repair_requires_confirmation_non_interactive() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_repair_confirm");

    let out = run_err(
        acl_cmd(&env)
            .args(["acl", "repair", "-p", &str_path(&dir)])
            .stdin(std::process::Stdio::null()),
    );
    let err = stderr_str(&out);
    assert!(
        err.contains("interactive") || err.contains("confirmation"),
        "expected interactive mode error: {err}"
    );
}

#[test]
fn acl_repair_missing_path_rejected_by_path_guard() {
    // path_guard 应在进程入口处拒绝不存在的路径，返回明确错误信息
    // 不会到达 repair 逻辑，因此不会产生 error CSV
    let env = TestEnv::new();
    let missing = env.root.join("acl_repair_missing_path");

    let out = run_err(acl_cmd(&env).args([
        "acl", "repair", "-p", &str_path(&missing), "-y",
    ]));
    let err = stderr_str(&out);
    let err_lower = err.to_lowercase();
    assert!(
        err.contains("Invalid path input.")
            || err.contains("Invalid target path")
            || err_lower.contains("not found")
            || err_lower.contains("cannot find")
            || err.contains("找不到"),
        "expected path validation error: {err}"
    );

    // 路径被 path_guard 拒绝 → 不写审计条目
    let actions = read_audit_actions(&env);
    assert!(
        !actions.iter().any(|a| a == "ForceRepair"),
        "ForceRepair should not be audited when path is invalid"
    );
}

#[test]
fn acl_repair_success_when_admin() {
    if !is_admin() { return; }
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_repair");

    run_ok(acl_cmd(&env).args(["acl", "repair", "-p", &str_path(&dir), "-y"]));

    let actions = read_audit_actions(&env);
    assert!(actions.iter().any(|a| a == "ForceRepair"), "missing ForceRepair audit entry");
}

#[test]
fn acl_repair_sets_owner_and_full_control() {
    if !is_admin() { return; }
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_repair_state");

    run_ok(acl_cmd(&env).args(["acl", "repair", "-p", &str_path(&dir), "-y"]));

    let out = run_ok(acl_cmd(&env).args(["acl", "view", "-p", &str_path(&dir)]));
    let err = stderr_str(&out);
    let owner = owner_from_summary(&err);
    assert_eq!(
        owner.unwrap_or_default(), "BUILTIN\\Administrators",
        "owner not set to Administrators after repair"
    );
    assert!(err.contains("Inherit: enabled"), "expected inheritance enabled after repair: {err}");

    let rows = export_acl_rows(&env, &dir, "acl_repair_state_export");
    assert!(
        has_acl_row(
            &rows, "Allow", "显式", "BUILTIN\\Administrators",
            "FullControl", "ContainerInherit|ObjectInherit", "None", "否"
        ),
        "missing FullControl ACE for Administrators after repair"
    );
}

#[test]
fn acl_batch_repair_when_admin() {
    if !is_admin() { return; }
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_batch_repair");

    run_ok(acl_cmd(&env).args([
        "acl", "batch", "--paths", &str_path(&dir),
        "--action", "repair", "-y",
    ]));
}

#[test]
fn acl_owner_success_when_admin() {
    if !is_admin() { return; }
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_owner");

    let before = run_ok(acl_cmd(&env).args(["acl", "view", "-p", &str_path(&dir)]));
    let before_owner = owner_from_summary(&stderr_str(&before));

    let out = run_ok(acl_cmd(&env).args([
        "acl", "owner", "-p", &str_path(&dir),
        "--set", "BUILTIN\\Administrators", "-y",
    ]));
    if stderr_str(&out).contains("Owner unchanged.") { return; }

    let actions = read_audit_actions(&env);
    assert!(actions.iter().any(|a| a == "SetOwner"), "missing SetOwner audit entry");

    let after = run_ok(acl_cmd(&env).args(["acl", "view", "-p", &str_path(&dir)]));
    let after_owner = owner_from_summary(&stderr_str(&after));
    assert!(
        before_owner.is_some() && after_owner.is_some(),
        "missing owner in view output"
    );
    assert_eq!(
        after_owner.unwrap(), "BUILTIN\\Administrators",
        "owner not updated as expected"
    );
}
