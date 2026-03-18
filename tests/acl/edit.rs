use super::common::*;
use crate::common::*;

#[test]
fn acl_add_path_with_spaces() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl space dir");

    run_ok(acl_cmd(&env).args([
        "acl", "add", "-p", &str_path(&dir),
        "--principal", "BUILTIN\\Users",
        "--rights", "Read",
        "--ace-type", "Allow",
        "--inherit", "None", "-y",
    ]));

    let rows = export_acl_rows(&env, &dir, "acl_space_export");
    assert!(
        has_acl_row(&rows, "Allow", "显式", "BUILTIN\\Users", "Read", "None", "None", "否"),
        "missing allow ACE for path with spaces"
    );
}

#[test]
fn acl_add_and_purge_write_audit() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_write");
    let principal = "BUILTIN\\Users";

    run_ok(acl_cmd(&env).args([
        "acl", "add", "-p", &str_path(&dir),
        "--principal", principal,
        "--rights", "Read",
        "--ace-type", "Allow",
        "--inherit", "None", "-y",
    ]));

    let rows = export_acl_rows(&env, &dir, "acl_write_after_add");
    assert!(
        has_acl_row(&rows, "Allow", "显式", principal, "Read", "None", "None", "否"),
        "missing added ACE in export rows"
    );

    run_ok(acl_cmd(&env).args([
        "acl", "purge", "-p", &str_path(&dir),
        "--principal", principal, "-y",
    ]));

    let rows = export_acl_rows(&env, &dir, "acl_write_after_purge");
    assert!(
        !has_acl_row(&rows, "Allow", "显式", principal, "Read", "None", "None", "否"),
        "expected purged ACE to be absent"
    );

    let actions = read_audit_actions(&env);
    assert!(actions.iter().any(|a| a == "AddPermission"), "missing AddPermission audit entry");
    assert!(actions.iter().any(|a| a == "PurgePrincipal"), "missing PurgePrincipal audit entry");
}

#[test]
fn acl_add_invalid_principal_rejected() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_add_invalid_principal");

    let out = run_err(acl_cmd(&env).args([
        "acl", "add", "-p", &str_path(&dir),
        "--principal", "S-1-5-XYZ",
        "--rights", "Read",
        "--ace-type", "Allow",
        "--inherit", "None", "-y",
    ]));
    let err = stderr_str(&out);
    assert!(
        err.contains("invalid principal") || err.contains("cannot resolve"),
        "unexpected error: {err}"
    );
    let actions = read_audit_actions(&env);
    assert!(
        !actions.iter().any(|a| a == "AddPermission"),
        "unexpected audit entry for failed add"
    );
}

#[test]
fn acl_add_overwrites_existing_allow_for_same_principal() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_add_overwrite");
    let principal = "S-1-5-21-222222222-333333333-444444444-6666";

    run_ok(acl_cmd(&env).args([
        "acl", "add", "-p", &str_path(&dir),
        "--principal", principal,
        "--rights", "Write",
        "--ace-type", "Allow",
        "--inherit", "None", "-y",
    ]));
    run_ok(acl_cmd(&env).args([
        "acl", "add", "-p", &str_path(&dir),
        "--principal", principal,
        "--rights", "Modify",
        "--ace-type", "Allow",
        "--inherit", "None", "-y",
    ]));

    let rows = export_acl_rows(&env, &dir, "acl_add_overwrite_export");
    assert!(
        has_acl_row(&rows, "Allow", "显式", principal, "Modify", "None", "None", "是"),
        "missing overwritten Modify ACE in export rows"
    );
    assert!(
        !has_acl_row(&rows, "Allow", "显式", principal, "Write", "None", "None", "是"),
        "expected previous Write ACE to be overwritten"
    );
}

#[test]
fn acl_add_deny_with_inherit() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_add_deny");
    let principal = "S-1-5-21-222222222-333333333-444444444-5555";

    run_ok(acl_cmd(&env).args([
        "acl", "add", "-p", &str_path(&dir),
        "--principal", principal,
        "--rights", "Write",
        "--ace-type", "Deny",
        "--inherit", "ContainerOnly", "-y",
    ]));

    let out = run_ok(acl_cmd(&env).args(["acl", "view", "-p", &str_path(&dir), "--detail"]));
    let err = stderr_str(&out);
    assert!(err.contains(principal), "missing principal in view: {err}");
    assert!(err.contains("Deny"), "missing deny entry: {err}");
    assert!(err.contains("ContainerInherit"), "missing inheritance flag: {err}");

    let rows = export_acl_rows(&env, &dir, "acl_add_deny_export");
    assert!(
        has_acl_row(&rows, "Deny", "显式", principal, "Write", "ContainerInherit", "None", "是"),
        "missing deny ACE in export rows"
    );
}

#[test]
fn acl_add_allow_with_object_inherit() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_add_object_inherit");
    let principal = "S-1-5-21-222222222-333333333-444444444-7777";

    run_ok(acl_cmd(&env).args([
        "acl", "add", "-p", &str_path(&dir),
        "--principal", principal,
        "--rights", "Read",
        "--ace-type", "Allow",
        "--inherit", "ObjectOnly", "-y",
    ]));

    let rows = export_acl_rows(&env, &dir, "acl_add_object_inherit_export");
    assert!(
        has_acl_row(&rows, "Allow", "显式", principal, "Read", "ObjectInherit", "None", "是"),
        "missing object inherit ACE in export rows"
    );
}

#[test]
fn acl_add_allow_with_both_inherit() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_add_both_inherit");
    let principal = "S-1-5-21-222222222-333333333-444444444-8888";

    run_ok(acl_cmd(&env).args([
        "acl", "add", "-p", &str_path(&dir),
        "--principal", principal,
        "--rights", "Read",
        "--ace-type", "Allow",
        "--inherit", "BothInherit", "-y",
    ]));

    let rows = export_acl_rows(&env, &dir, "acl_add_both_inherit_export");
    assert!(
        has_acl_row(
            &rows, "Allow", "显式", principal, "Read",
            "ContainerInherit|ObjectInherit", "None", "是"
        ),
        "missing both-inherit ACE in export rows"
    );
}

#[test]
fn acl_remove_non_interactive_by_principal() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_remove");
    let principal = "S-1-5-21-222222222-333333333-444444444-9999";

    run_ok(acl_cmd(&env).args([
        "acl", "add", "-p", &str_path(&dir),
        "--principal", principal,
        "--rights", "Read",
        "--ace-type", "Allow",
        "--inherit", "None", "-y",
    ]));

    let rows = export_acl_rows(&env, &dir, "acl_remove_before");
    assert!(
        has_acl_row(&rows, "Allow", "显式", principal, "Read", "None", "None", "是"),
        "missing ACE before removal"
    );

    run_ok(acl_cmd(&env).args([
        "acl", "remove", "-p", &str_path(&dir),
        "--principal", principal, "-y",
    ]));

    let rows = export_acl_rows(&env, &dir, "acl_remove_after");
    assert!(
        !has_acl_row(&rows, "Allow", "显式", principal, "Read", "None", "None", "是"),
        "expected ACE to be removed"
    );
}

#[test]
fn acl_remove_requires_interactive() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_remove_interactive");

    // 先添加一条 ACE，确保有条目可删
    run_ok(acl_cmd(&env).args([
        "acl", "add", "-p", &str_path(&dir),
        "--principal", "BUILTIN\\Users",
        "--rights", "Read",
        "--ace-type", "Allow",
        "--inherit", "None", "-y",
    ]));

    // 不带 -y，非交互环境应报错
    let out = run_err(
        acl_cmd(&env)
            .args(["acl", "remove", "-p", &str_path(&dir)])
            .stdin(std::process::Stdio::null()),
    );
    let err = stderr_str(&out);
    assert!(
        err.contains("interactive") || err.contains("confirmation"),
        "expected interactive mode error: {err}"
    );
}

#[test]
fn acl_write_operations_fail_without_admin() {
    if is_admin() { return; }
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_non_admin_write");

    let rows_before = export_acl_rows(&env, &dir, "acl_non_admin_before");

    let out = run_err(acl_cmd(&env).args([
        "acl", "owner", "-p", &str_path(&dir),
        "--set", "BUILTIN\\Administrators", "-y",
    ]));
    let err = stderr_str(&out);
    assert!(
        err.contains("access denied") || err.contains("Access"),
        "expected access denied error: {err}"
    );

    let rows_after = export_acl_rows(&env, &dir, "acl_non_admin_after");
    assert_eq!(
        rows_before.len(), rows_after.len(),
        "ACL rows changed after non-admin write attempt"
    );
}
