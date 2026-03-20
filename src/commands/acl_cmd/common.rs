use super::*;
use crate::path_guard::{PathIssueKind, PathPolicy};

pub(super) struct AclRuntimeConfig {
    pub(super) cfg: AclConfig,
    pub(super) audit_log_path: PathBuf,
    pub(super) export_path: PathBuf,
}

pub(super) fn normalize_acl_config(cfg: AclConfig) -> AclConfig {
    let defaults = AclConfig::default();
    let mut out = cfg;
    if out.throttle_limit == 0 {
        out.throttle_limit = defaults.throttle_limit;
    }
    if out.chunk_size == 0 {
        out.chunk_size = defaults.chunk_size;
    }
    if out.audit_log_path.trim().is_empty() {
        out.audit_log_path = defaults.audit_log_path;
    }
    if out.export_path.trim().is_empty() {
        out.export_path = defaults.export_path;
    }
    if out.default_owner.trim().is_empty() {
        out.default_owner = defaults.default_owner;
    }
    if out.max_audit_lines == 0 {
        out.max_audit_lines = defaults.max_audit_lines;
    }
    out
}

pub(super) fn load_acl_runtime_config() -> AclRuntimeConfig {
    let cfg = load_config();
    let acl_cfg = normalize_acl_config(cfg.acl);
    AclRuntimeConfig {
        audit_log_path: PathBuf::from(&acl_cfg.audit_log_path),
        export_path: PathBuf::from(&acl_cfg.export_path),
        cfg: acl_cfg,
    }
}

pub(super) fn audit_log(cfg: &AclRuntimeConfig) -> AuditLog {
    AuditLog::new(cfg.audit_log_path.clone(), cfg.cfg.max_audit_lines)
}

pub(super) fn map_acl_err(err: anyhow::Error) -> CliError {
    let mut details: Vec<String> = Vec::new();
    if let Some(acl_err) = err.downcast_ref::<AclError>()
        && acl_err.is_access_denied()
    {
        details.push("Hint: Run as Administrator for ACL write/repair operations.".to_string());
    }
    CliError {
        code: 1,
        message: format!("{err:#}"),
        details,
    }
}

pub(super) fn prompt_confirm(prompt: &str, default: bool, yes: bool) -> CliResult<bool> {
    if yes {
        return Ok(true);
    }
    if !can_interact() {
        return Err(CliError::with_details(
            2,
            "Interactive confirmation required.".to_string(),
            &["Fix: Run in an interactive terminal or pass -y to skip confirmation."],
        ));
    }
    Confirm::new()
        .with_prompt(prompt)
        .default(default)
        .interact()
        .map_err(|e| CliError::new(1, format!("Failed to read confirmation: {e}")))
}

pub(super) fn ensure_interactive(label: &str) -> CliResult {
    if can_interact() {
        Ok(())
    } else {
        Err(CliError::with_details(
            2,
            format!("{label} requires interactive mode."),
            &["Fix: Run in an interactive terminal."],
        ))
    }
}

pub(super) fn print_path_header(path: &Path) {
    ui_println!("\nPath: {}", path.display());
}

pub(super) fn print_acl_summary(snapshot: &acl::types::AclSnapshot) {
    let allow = snapshot.allow_count();
    let deny = snapshot.deny_count();
    let orphan = snapshot.orphan_count();
    let explicit = snapshot.explicit_count();
    let inherited = snapshot.inherited_count();
    ui_println!(
        "Owner: {} | Inherit: {}",
        snapshot.owner,
        if snapshot.is_protected {
            "disabled"
        } else {
            "enabled"
        }
    );
    ui_println!(
        "Total: {} (Allow {} / Deny {})  Explicit {}  Inherited {}  Orphan {}",
        snapshot.entries.len(),
        allow,
        deny,
        explicit,
        inherited,
        orphan
    );
}

pub(super) fn validate_acl_path(
    raw: &str,
    policy: &PathPolicy,
    label: &str,
    open: bool,
) -> CliResult<PathBuf> {
    let mut policy = policy.clone();
    policy.allow_relative = true;
    let result = crate::path_guard::validate_paths(vec![raw.to_string()], &policy);
    if !result.issues.is_empty() {
        let mut details: Vec<String> = result
            .issues
            .iter()
            .map(|issue| format!("Invalid {label} path: {} ({})", issue.raw, issue.detail))
            .collect();
        details.push("Fix: Provide an existing path and avoid reserved device names.".to_string());
        return Err(CliError::with_details(2, "Invalid path input.".to_string(), &details));
    }
    let Some(path) = result.ok.into_iter().next() else {
        return Err(CliError::new(2, "No valid path provided."));
    };

    if open
        && let Err(kind) = crate::path_guard::winapi::open_path_with_policy(&path, &policy) {
            let detail = match kind {
                PathIssueKind::NotFound => "Path not found.",
                PathIssueKind::AccessDenied => "Access denied.",
                PathIssueKind::SharingViolation => "Sharing violation.",
                PathIssueKind::NetworkPathNotFound => "Network path not found.",
                PathIssueKind::TooLong => "Path too long.",
                PathIssueKind::SymlinkLoop => "Symlink loop detected.",
                _ => "I/O error.",
            };
            return Err(CliError::with_details(
                2,
                "Invalid path input.".to_string(),
                &[format!("Invalid {label} path: {detail}")],
            ));
        }

    Ok(path)
}
