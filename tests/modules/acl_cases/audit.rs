use super::common::*;
use crate::common::*;

#[test]
fn acl_audit_tail_table_contains_headers() {
    let env = TestEnv::new();
    let dir = setup_acl_dir(&env, "acl_audit_tail");

    run_ok(acl_cmd(&env).args(["acl", "diff", "-p", &str_path(&dir), "-r", &str_path(&dir)]));

    let out = run_ok(acl_cmd(&env).args(["acl", "audit", "--tail", "1"]));
    let err = stderr_str(&out);
    assert!(err.contains("Action"), "missing Action header: {err}");
    assert!(err.contains("Status"), "missing Status header: {err}");
    assert!(err.contains("Path"), "missing Path header: {err}");
}
