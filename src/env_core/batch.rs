use std::collections::HashSet;
use std::path::Path;

use super::registry;
use super::types::{BatchResult, EnvError, EnvResult, EnvScope};

pub fn batch_set(
    scope: EnvScope,
    items: &[(String, String)],
    dry_run: bool,
) -> EnvResult<BatchResult> {
    if !scope.is_writable() {
        return Err(EnvError::ScopeNotWritable(scope));
    }
    let mut result = BatchResult {
        dry_run,
        scope,
        added: 0,
        updated: 0,
        deleted: 0,
        renamed: 0,
        skipped: 0,
        details: Vec::new(),
    };
    for (name_raw, value) in items {
        let name = name_raw.trim();
        if name.is_empty() {
            result.skipped += 1;
            result.details.push("skip empty name".to_string());
            continue;
        }
        let old = registry::get_var(scope, name)?;
        match old {
            Some(old_var) => {
                if old_var.raw_value == *value {
                    result.skipped += 1;
                    result.details.push(format!("skip {} (unchanged)", name));
                    continue;
                }
                result.updated += 1;
                result.details.push(format!(
                    "update {} ({} -> {})",
                    name, old_var.raw_value, value
                ));
            }
            None => {
                result.added += 1;
                result.details.push(format!("add {}={}", name, value));
            }
        }
        if !dry_run {
            registry::set_var(scope, name, value)?;
        }
    }
    Ok(result)
}

pub fn batch_delete(scope: EnvScope, names: &[String], dry_run: bool) -> EnvResult<BatchResult> {
    if !scope.is_writable() {
        return Err(EnvError::ScopeNotWritable(scope));
    }
    let mut result = BatchResult {
        dry_run,
        scope,
        added: 0,
        updated: 0,
        deleted: 0,
        renamed: 0,
        skipped: 0,
        details: Vec::new(),
    };
    for name_raw in names {
        let name = name_raw.trim();
        if name.is_empty() {
            result.skipped += 1;
            result.details.push("skip empty name".to_string());
            continue;
        }
        let old = registry::get_var(scope, name)?;
        if old.is_none() {
            result.skipped += 1;
            result.details.push(format!("skip {} (not found)", name));
            continue;
        }
        result.deleted += 1;
        result.details.push(format!("delete {}", name));
        if !dry_run {
            let _ = registry::delete_var(scope, name)?;
        }
    }
    Ok(result)
}

pub fn batch_rename(
    scope: EnvScope,
    old_name: &str,
    new_name: &str,
    dry_run: bool,
) -> EnvResult<BatchResult> {
    if !scope.is_writable() {
        return Err(EnvError::ScopeNotWritable(scope));
    }
    let old_name = old_name.trim();
    let new_name = new_name.trim();
    if old_name.is_empty() || new_name.is_empty() {
        return Err(EnvError::InvalidInput(
            "rename requires non-empty OLD and NEW".to_string(),
        ));
    }
    if old_name.eq_ignore_ascii_case(new_name) {
        return Err(EnvError::InvalidInput(
            "OLD and NEW cannot be the same".to_string(),
        ));
    }

    let mut result = BatchResult {
        dry_run,
        scope,
        added: 0,
        updated: 0,
        deleted: 0,
        renamed: 0,
        skipped: 0,
        details: Vec::new(),
    };

    let old = registry::get_var(scope, old_name)?;
    let Some(old_var) = old else {
        result.skipped = 1;
        result
            .details
            .push(format!("skip {} (not found)", old_name));
        return Ok(result);
    };
    let new_exists = registry::get_var(scope, new_name)?.is_some();
    if new_exists {
        return Err(EnvError::InvalidInput(format!(
            "target variable '{}' already exists",
            new_name
        )));
    }

    result.renamed = 1;
    result
        .details
        .push(format!("rename {} -> {}", old_name, new_name));
    if !dry_run {
        registry::set_var(scope, new_name, &old_var.raw_value)?;
        let _ = registry::delete_var(scope, old_name)?;
    }
    Ok(result)
}

pub fn path_dedup(scope: EnvScope, remove_missing: bool, dry_run: bool) -> EnvResult<BatchResult> {
    if !scope.is_writable() {
        return Err(EnvError::ScopeNotWritable(scope));
    }

    let mut result = BatchResult {
        dry_run,
        scope,
        added: 0,
        updated: 0,
        deleted: 0,
        renamed: 0,
        skipped: 0,
        details: Vec::new(),
    };

    let entries = registry::get_path_entries(scope)?;
    let mut seen = HashSet::new();
    let mut compact = Vec::new();
    for item in entries {
        let key = item.to_ascii_lowercase();
        if !seen.insert(key) {
            result.deleted += 1;
            result.details.push(format!("remove duplicate {}", item));
            continue;
        }
        if remove_missing && !Path::new(&item).exists() {
            result.deleted += 1;
            result.details.push(format!("remove missing {}", item));
            continue;
        }
        compact.push(item);
    }

    if result.deleted == 0 {
        result.skipped = 1;
        result.details.push("PATH unchanged".to_string());
        return Ok(result);
    }
    result.updated = 1;
    if !dry_run {
        registry::set_path_entries(scope, &compact)?;
    }
    Ok(result)
}
