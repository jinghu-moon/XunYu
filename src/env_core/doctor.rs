use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::registry;
use super::types::{
    DoctorFixResult, DoctorIssue, DoctorIssueKind, DoctorReport, EnvResult, EnvScope,
};

pub fn run_doctor(scope: EnvScope) -> EnvResult<DoctorReport> {
    let mut issues = Vec::new();
    for sc in scope.expand_scopes() {
        run_scope_checks(*sc, &mut issues)?;
    }
    if scope == EnvScope::All {
        run_shadow_check(&mut issues)?;
    }

    let errors = issues.iter().filter(|i| i.severity == "error").count();
    let warnings = issues.iter().filter(|i| i.severity == "warning").count();
    let fixable = issues.iter().filter(|i| i.fixable).count();

    Ok(DoctorReport {
        scope,
        issues,
        errors,
        warnings,
        fixable,
    })
}

pub fn fix_doctor(scope: EnvScope) -> EnvResult<DoctorFixResult> {
    let mut fixed = 0usize;
    let mut details = Vec::new();

    for sc in scope.expand_scopes() {
        let original = registry::get_path_entries(*sc)?;
        let mut seen = HashSet::new();
        let mut compact = Vec::new();
        for item in original {
            let normalized = item.to_lowercase();
            if !seen.insert(normalized) {
                fixed += 1;
                details.push(format!("{}: remove duplicate {}", sc, item));
                continue;
            }
            if !Path::new(&item).exists() {
                fixed += 1;
                details.push(format!("{}: remove missing {}", sc, item));
                continue;
            }
            compact.push(item);
        }
        registry::set_path_entries(*sc, &compact)?;
    }

    Ok(DoctorFixResult {
        scope,
        fixed,
        details,
    })
}

fn run_scope_checks(scope: EnvScope, issues: &mut Vec<DoctorIssue>) -> EnvResult<()> {
    let path_entries = registry::get_path_entries(scope)?;
    let mut seen = HashSet::new();
    for item in &path_entries {
        let key = item.to_lowercase();
        if !seen.insert(key) {
            issues.push(DoctorIssue {
                kind: DoctorIssueKind::PathDuplicate,
                severity: "warning".to_string(),
                scope,
                name: "PATH".to_string(),
                message: format!("duplicate PATH entry: {}", item),
                fixable: true,
            });
            continue;
        }
        if !Path::new(item).exists() {
            issues.push(DoctorIssue {
                kind: DoctorIssueKind::PathMissing,
                severity: "warning".to_string(),
                scope,
                name: "PATH".to_string(),
                message: format!("missing PATH entry: {}", item),
                fixable: true,
            });
        }
    }
    let path_len = path_entries.join(";").len();
    if path_len > 2048 {
        issues.push(DoctorIssue {
            kind: DoctorIssueKind::PathTooLong,
            severity: "warning".to_string(),
            scope,
            name: "PATH".to_string(),
            message: format!("PATH length {} exceeds recommended 2048", path_len),
            fixable: false,
        });
    }

    let vars = registry::list_scope(scope)?;
    let graph: HashMap<String, String> = vars
        .into_iter()
        .map(|v| (v.name.to_uppercase(), v.raw_value))
        .collect();
    let cycles = detect_cycles(&graph);
    for cycle in cycles {
        issues.push(DoctorIssue {
            kind: DoctorIssueKind::VarCycle,
            severity: "error".to_string(),
            scope,
            name: cycle
                .first()
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            message: format!("variable reference cycle: {}", cycle.join(" -> ")),
            fixable: false,
        });
    }
    Ok(())
}

fn run_shadow_check(issues: &mut Vec<DoctorIssue>) -> EnvResult<()> {
    let user_vars = registry::list_scope(EnvScope::User)?;
    let system_vars = registry::list_scope(EnvScope::System)?;
    let sys_map: HashMap<String, String> = system_vars
        .into_iter()
        .map(|v| (v.name.to_lowercase(), v.raw_value))
        .collect();
    for user_var in user_vars {
        if let Some(sys_value) = sys_map.get(&user_var.name.to_lowercase()) {
            issues.push(DoctorIssue {
                kind: DoctorIssueKind::UserShadowsSystem,
                severity: "warning".to_string(),
                scope: EnvScope::All,
                name: user_var.name.clone(),
                message: format!(
                    "user '{}' shadows system value (system='{}', user='{}')",
                    user_var.name, sys_value, user_var.raw_value
                ),
                fixable: false,
            });
        }
    }
    Ok(())
}

pub fn report_text(report: &DoctorReport) -> String {
    if report.issues.is_empty() {
        return "OK: no issues".to_string();
    }
    let mut out = Vec::new();
    out.push(format!(
        "issues: {} (errors={}, warnings={}, fixable={})",
        report.issues.len(),
        report.errors,
        report.warnings,
        report.fixable
    ));
    for issue in &report.issues {
        out.push(format!(
            "- [{}] {}: {}",
            issue.severity, issue.kind as u8, issue.message
        ));
    }
    out.join("\n")
}

pub fn doctor_exit_code(report: &DoctorReport) -> i32 {
    if report.errors > 0 {
        2
    } else if report.warnings > 0 {
        1
    } else {
        0
    }
}

fn detect_cycles(vars: &HashMap<String, String>) -> Vec<Vec<String>> {
    let mut status: HashMap<String, u8> = HashMap::new();
    let mut stack: Vec<String> = Vec::new();
    let mut out = Vec::new();
    let mut seen_cycle = HashSet::new();

    fn dfs(
        node: &str,
        vars: &HashMap<String, String>,
        status: &mut HashMap<String, u8>,
        stack: &mut Vec<String>,
        out: &mut Vec<Vec<String>>,
        seen_cycle: &mut HashSet<String>,
    ) {
        let state = status.get(node).copied().unwrap_or(0);
        if state == 2 {
            return;
        }
        if state == 1 {
            if let Some(pos) = stack.iter().position(|n| n == node) {
                let mut cycle: Vec<String> = stack[pos..].to_vec();
                cycle.push(node.to_string());
                let key = cycle.join("|");
                if seen_cycle.insert(key) {
                    out.push(cycle);
                }
            }
            return;
        }

        status.insert(node.to_string(), 1);
        stack.push(node.to_string());
        if let Some(value) = vars.get(node) {
            for dep in extract_refs(value) {
                if vars.contains_key(&dep) {
                    dfs(&dep, vars, status, stack, out, seen_cycle);
                }
            }
        }
        stack.pop();
        status.insert(node.to_string(), 2);
    }

    for name in vars.keys() {
        if status.get(name).copied().unwrap_or(0) == 0 {
            dfs(
                name,
                vars,
                &mut status,
                &mut stack,
                &mut out,
                &mut seen_cycle,
            );
        }
    }
    out
}

fn extract_refs(value: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] != '%' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < chars.len() && chars[j] != '%' {
            j += 1;
        }
        if j < chars.len() && j > i + 1 {
            let name: String = chars[i + 1..j].iter().collect();
            if !name.trim().is_empty() {
                refs.push(name.trim().to_uppercase());
            }
            i = j + 1;
            continue;
        }
        i += 1;
    }
    refs
}
