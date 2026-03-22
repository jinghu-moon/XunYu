use super::common::*;
use crate::common::*;
use serde_json::Value;

#[test]
fn acl_config_set_persists_and_invalid_rejected() {
    let env = TestEnv::new();

    run_ok(acl_cmd(&env).args(["acl", "config", "--set", "throttle_limit", "8"]));
    let cfg_path = env.root.join(".xun.config.json");
    let raw = std::fs::read_to_string(&cfg_path).unwrap();
    let v: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["acl"]["throttle_limit"].as_u64(), Some(8));

    let out = run_err(acl_cmd(&env).args(["acl", "config", "--set", "throttle_limit"]));
    let err = stderr_str(&out);
    assert!(
        err.contains("requires KEY VALUE"),
        "unexpected error: {err}"
    );
}

#[test]
fn acl_config_sets_all_keys_and_rejects_unknown() {
    let env = TestEnv::new();
    let audit_path = env.root.join("audit_path.jsonl");
    let export_path = env.root.join("exports");
    let audit_path_s = str_path(&audit_path);
    let export_path_s = str_path(&export_path);
    std::fs::create_dir_all(&export_path).unwrap();

    run_ok(acl_cmd(&env).args(["acl", "config", "--set", "chunk_size", "64"]));
    run_ok(acl_cmd(&env).args(["acl", "config", "--set", "audit_log_path", &audit_path_s]));
    run_ok(acl_cmd(&env).args(["acl", "config", "--set", "export_path", &export_path_s]));
    run_ok(acl_cmd(&env).args(["acl", "config", "--set", "default_owner", "BUILTIN\\Users"]));
    run_ok(acl_cmd(&env).args(["acl", "config", "--set", "max_audit_lines", "1234"]));

    let cfg_path = env.root.join(".xun.config.json");
    let raw = std::fs::read_to_string(&cfg_path).unwrap();
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
    assert_eq!(v["acl"]["default_owner"].as_str(), Some("BUILTIN\\Users"));
    assert_eq!(v["acl"]["max_audit_lines"].as_u64(), Some(1234));

    let out = run_err(acl_cmd(&env).args(["acl", "config", "--set", "unknown_key", "1"]));
    let err = stderr_str(&out);
    assert!(err.contains("Unknown key"), "unexpected error: {err}");
}
