use super::super::registry;
use super::super::types::{
    EnvError, EnvResult, EnvScope, ImportApplyResult, ImportStrategy, ParsedImport,
};

pub(super) fn apply_import(
    scope: EnvScope,
    parsed: &ParsedImport,
    strategy: ImportStrategy,
    dry_run: bool,
) -> EnvResult<ImportApplyResult> {
    if !scope.is_writable() {
        return Err(EnvError::ScopeNotWritable(scope));
    }

    let mut added = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut changed_names = Vec::new();

    for var in &parsed.vars {
        if var.name.eq_ignore_ascii_case("PATH") {
            let mut next_entries = split_path_entries(&var.value);
            if matches!(strategy, ImportStrategy::Merge) {
                let mut current = registry::get_path_entries(scope)?;
                for entry in next_entries {
                    if !current.iter().any(|v| v.eq_ignore_ascii_case(&entry)) {
                        current.push(entry);
                    }
                }
                next_entries = current;
            }
            if dry_run {
                changed_names.push("PATH".to_string());
                continue;
            }
            registry::set_path_entries(scope, &next_entries)?;
            updated += 1;
            changed_names.push("PATH".to_string());
            continue;
        }

        let existed = registry::get_var(scope, &var.name)?.is_some();
        if existed && matches!(strategy, ImportStrategy::Merge) {
            skipped += 1;
            continue;
        }

        if dry_run {
            changed_names.push(var.name.clone());
            continue;
        }

        registry::set_var(scope, &var.name, &var.value)?;
        if existed {
            updated += 1;
        } else {
            added += 1;
        }
        changed_names.push(var.name.clone());
    }

    Ok(ImportApplyResult {
        dry_run,
        added,
        updated,
        skipped,
        changed_names,
    })
}

fn split_path_entries(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
