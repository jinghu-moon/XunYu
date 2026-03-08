use crate::config::ProtectRule;
use crate::config::load_config;
use std::path::Path;

pub(crate) fn is_protected<'a>(
    rules: &'a [ProtectRule],
    path: &Path,
    action: &str, // "delete", "move", "rename"
) -> Option<&'a ProtectRule> {
    let path_str = path.to_string_lossy().replace('\\', "/").to_lowercase();
    for r in rules {
        if r.deny.is_empty() || r.deny.iter().any(|d| d.eq_ignore_ascii_case(action)) {
            let r_path = r.path.replace('\\', "/").to_lowercase();
            // Prefix match if directory, explicit match if file (or simple prefix fallback)
            // Just use starts_with for now assuming recursive protection
            if path_str == r_path || path_str.starts_with(&format!("{r_path}/")) {
                return Some(r);
            }
        }
    }
    None
}

pub(crate) fn check_protection(
    path: &Path,
    action: &str,
    force: bool,
    reason: Option<&str>,
) -> Result<(), &'static str> {
    let cfg = load_config();
    let rules = cfg.protect.rules;

    if let Some(rule) = is_protected(&rules, path, action) {
        let req_force = rule.require.iter().any(|q| q.eq_ignore_ascii_case("force"));
        let req_reason = rule
            .require
            .iter()
            .any(|q| q.eq_ignore_ascii_case("reason"));

        if (!req_force || force) && (!req_reason || reason.is_some()) {
            return Ok(());
        }

        if req_reason && reason.is_none() {
            return Err("Protection rule requires a --reason to bypass.");
        }
        if req_force && !force {
            return Err("Protection rule requires --force to bypass.");
        }

        return Err("Path is protected against this action.");
    }
    Ok(())
}
