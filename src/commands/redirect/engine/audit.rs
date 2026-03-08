use crate::security::audit::{AuditParams, audit_log};

use super::types::RedirectOptions;

use std::collections::BTreeMap;

pub(super) fn audit_if(
    opts: &RedirectOptions,
    action: &str,
    target: &str,
    user: &str,
    params: AuditParams,
    result: &str,
    reason: &str,
) {
    if !opts.audit {
        return;
    }
    audit_log(action, target, user, params, result, reason);
}

pub(super) fn audit_params_redirect(tx: &str, dst: &str, copy: bool) -> AuditParams {
    let mut m: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    m.insert("tx".to_string(), serde_json::Value::String(tx.to_string()));
    m.insert(
        "dst".to_string(),
        serde_json::Value::String(dst.to_string()),
    );
    m.insert("copy".to_string(), serde_json::Value::Bool(copy));
    AuditParams::Map(m)
}
