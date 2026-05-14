//! Env 业务逻辑服务
//!
//! 封装环境变量管理操作，支持 CommandSpec 和 Operation 实现。

use crate::env_core::EnvManager;
use crate::env_core::types::EnvScope;
use crate::xun_core::error::XunError;
use crate::xun_core::operation::{Change, Operation, OperationResult, Preview, RiskLevel};
use crate::xun_core::value::Value;

// ============================================================
// EnvSetOp — Operation trait 实现
// ============================================================

/// 环境变量设置操作（实现 Operation trait）。
pub struct EnvSetOp {
    name: String,
    value: String,
    scope: EnvScope,
    preview: Preview,
}

impl EnvSetOp {
    pub fn new(
        name: impl Into<String>,
        value: impl Into<String>,
        scope: EnvScope,
    ) -> Self {
        let name = name.into();
        let value = value.into();
        let scope_str = format!("{:?}", scope);
        let preview = Preview::new(format!("Set environment variable '{}' = '{}'", name, value))
            .add_change(Change::new("set", format!("{name} ({scope_str})")))
            .with_risk_level(RiskLevel::Medium);
        Self {
            name,
            value,
            scope,
            preview,
        }
    }
}

impl Operation for EnvSetOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        let mgr = EnvManager::new();
        mgr.set_var(self.scope, &self.name, &self.value, false)
            .map_err(|e| XunError::user(format!("failed to set env var: {e}")))?;
        Ok(OperationResult::new().with_changes_applied(1))
    }
}

// ============================================================
// EnvDelOp — Operation trait 实现
// ============================================================

/// 环境变量删除操作（实现 Operation trait）。
pub struct EnvDelOp {
    name: String,
    scope: EnvScope,
    preview: Preview,
}

impl EnvDelOp {
    pub fn new(name: impl Into<String>, scope: EnvScope) -> Self {
        let name = name.into();
        let scope_str = format!("{:?}", scope);
        let preview = Preview::new(format!("Delete environment variable '{}'", name))
            .add_change(Change::new("delete", format!("{name} ({scope_str})")))
            .with_risk_level(RiskLevel::Medium);
        Self {
            name,
            scope,
            preview,
        }
    }
}

impl Operation for EnvDelOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        let mgr = EnvManager::new();
        mgr.delete_var(self.scope, &self.name)
            .map_err(|e| XunError::user(format!("failed to delete env var: {e}")))?;
        Ok(OperationResult::new().with_changes_applied(1))
    }
}

// ============================================================
// 环境变量查询服务
// ============================================================

/// 列出环境变量。
pub fn list_env_vars(scope: EnvScope) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let vars = mgr
        .list_vars(scope)
        .map_err(|e| XunError::user(format!("failed to list env vars: {e}")))?;
    let items: Vec<Value> = vars
        .into_iter()
        .map(|v| {
            let mut rec = crate::xun_core::value::Record::new();
            rec.insert("name".into(), Value::String(v.name));
            rec.insert("value".into(), Value::String(v.raw_value));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(items))
}

/// 获取环境变量值。
pub fn get_env_var(name: &str, scope: EnvScope) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    match mgr
        .get_var(scope, name)
        .map_err(|e| XunError::user(format!("failed to get env var: {e}")))?
    {
        Some(v) => Ok(Value::String(v.raw_value)),
        None => Ok(Value::Null),
    }
}

/// 搜索环境变量。
pub fn search_env_vars(scope: EnvScope, query: &str) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let vars = mgr
        .search_vars(scope, query)
        .map_err(|e| XunError::user(format!("failed to search env vars: {e}")))?;
    let items: Vec<Value> = vars
        .into_iter()
        .map(|v| {
            let mut rec = crate::xun_core::value::Record::new();
            rec.insert("name".into(), Value::String(v.name));
            rec.insert("value".into(), Value::String(v.raw_value));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(items))
}

/// 获取状态概览。
pub fn status_env(scope: EnvScope) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let summary = mgr
        .status_overview(scope)
        .map_err(|e| XunError::user(format!("failed to get status: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("user_vars".into(), Value::Int(summary.user_vars.unwrap_or(0) as i64));
    rec.insert("system_vars".into(), Value::Int(summary.system_vars.unwrap_or(0) as i64));
    rec.insert("total_vars".into(), Value::Int(summary.total_vars.unwrap_or(0) as i64));
    rec.insert("snapshots".into(), Value::Int(summary.snapshots as i64));
    rec.insert("profiles".into(), Value::Int(summary.profiles as i64));
    Ok(Value::Record(rec))
}

/// 运行 doctor 检查。
pub fn doctor_env(scope: EnvScope, fix: bool) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    if fix {
        let result = mgr
            .doctor_fix(scope)
            .map_err(|e| XunError::user(format!("doctor fix failed: {e}")))?;
        let mut rec = crate::xun_core::value::Record::new();
        rec.insert("fixed".into(), Value::Int(result.fixed as i64));
        Ok(Value::Record(rec))
    } else {
        let report = mgr
            .doctor_run(scope)
            .map_err(|e| XunError::user(format!("doctor run failed: {e}")))?;
        let mut rec = crate::xun_core::value::Record::new();
        rec.insert("issues".into(), Value::Int(report.issues.len() as i64));
        rec.insert("errors".into(), Value::Int(report.errors as i64));
        rec.insert("warnings".into(), Value::Int(report.warnings as i64));
        Ok(Value::Record(rec))
    }
}

/// PATH 去重。
pub fn path_dedup_env(scope: EnvScope, remove_missing: bool, dry_run: bool) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let result = mgr
        .path_dedup(scope, remove_missing, dry_run)
        .map_err(|e| XunError::user(format!("path dedup failed: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("deleted".into(), Value::Int(result.deleted as i64));
    rec.insert("skipped".into(), Value::Int(result.skipped as i64));
    Ok(Value::Record(rec))
}

/// 添加 PATH 条目。
pub fn path_add_env(scope: EnvScope, entry: &str, head: bool) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    mgr.path_add(scope, entry, head)
        .map_err(|e| XunError::user(format!("path add failed: {e}")))?;
    Ok(Value::Null)
}

/// 删除 PATH 条目。
pub fn path_rm_env(scope: EnvScope, entry: &str) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    mgr.path_remove(scope, entry)
        .map_err(|e| XunError::user(format!("path remove failed: {e}")))?;
    Ok(Value::Null)
}

/// 创建快照。
pub fn snapshot_create_env(desc: Option<&str>) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let meta = mgr
        .snapshot_create(desc)
        .map_err(|e| XunError::user(format!("snapshot create failed: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("id".into(), Value::String(meta.id));
    rec.insert("created_at".into(), Value::String(meta.created_at));
    rec.insert("description".into(), Value::String(meta.description));
    Ok(Value::Record(rec))
}

/// 列出快照。
pub fn snapshot_list_env() -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let snapshots = mgr
        .snapshot_list()
        .map_err(|e| XunError::user(format!("snapshot list failed: {e}")))?;
    let items: Vec<Value> = snapshots
        .into_iter()
        .map(|s| {
            let mut rec = crate::xun_core::value::Record::new();
            rec.insert("id".into(), Value::String(s.id));
            rec.insert("created_at".into(), Value::String(s.created_at));
            rec.insert("description".into(), Value::String(s.description));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(items))
}

/// 恢复快照。
pub fn snapshot_restore_env(id: Option<&str>, latest: bool, scope: EnvScope) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let meta = mgr
        .snapshot_restore(scope, id, latest)
        .map_err(|e| XunError::user(format!("snapshot restore failed: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("id".into(), Value::String(meta.id));
    rec.insert("description".into(), Value::String(meta.description));
    Ok(Value::Record(rec))
}

/// 清理旧快照。
pub fn snapshot_prune_env(keep: usize) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let pruned = mgr
        .snapshot_prune(keep)
        .map_err(|e| XunError::user(format!("snapshot prune failed: {e}")))?;
    Ok(Value::Int(pruned as i64))
}

/// 列出 profiles。
pub fn profile_list_env() -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let profiles = mgr
        .profile_list()
        .map_err(|e| XunError::user(format!("profile list failed: {e}")))?;
    let items: Vec<Value> = profiles
        .into_iter()
        .map(|p| {
            let mut rec = crate::xun_core::value::Record::new();
            rec.insert("name".into(), Value::String(p.name));
            rec.insert("var_count".into(), Value::Int(p.var_count as i64));
            rec.insert("created_at".into(), Value::String(p.created_at));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(items))
}

/// 捕获当前环境到 profile。
pub fn profile_capture_env(name: &str, scope: EnvScope) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let meta = mgr
        .profile_capture(name, scope)
        .map_err(|e| XunError::user(format!("profile capture failed: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("name".into(), Value::String(meta.name));
    rec.insert("var_count".into(), Value::Int(meta.var_count as i64));
    Ok(Value::Record(rec))
}

/// 应用 profile。
pub fn profile_apply_env(name: &str, scope: Option<EnvScope>) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    mgr.profile_apply(name, scope)
        .map_err(|e| XunError::user(format!("profile apply failed: {e}")))?;
    Ok(Value::Null)
}

/// Profile diff。
pub fn profile_diff_env(name: &str, scope: Option<EnvScope>) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let diff = mgr
        .profile_diff(name, scope)
        .map_err(|e| XunError::user(format!("profile diff failed: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("added".into(), Value::Int(diff.added.len() as i64));
    rec.insert("removed".into(), Value::Int(diff.removed.len() as i64));
    rec.insert("changed".into(), Value::Int(diff.changed.len() as i64));
    Ok(Value::Record(rec))
}

/// 删除 profile。
pub fn profile_delete_env(name: &str) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    mgr.profile_delete(name)
        .map_err(|e| XunError::user(format!("profile delete failed: {e}")))?;
    Ok(Value::Null)
}

/// 批量设置。
pub fn batch_set_env(scope: EnvScope, items: &[String], dry_run: bool) -> Result<Value, XunError> {
    let pairs: Vec<(String, String)> = items
        .iter()
        .filter_map(|s| {
            let mut parts = s.splitn(2, '=');
            Some((parts.next()?.to_string(), parts.next().unwrap_or("").to_string()))
        })
        .collect();
    let mgr = EnvManager::new();
    let result = mgr
        .batch_set(scope, &pairs, dry_run)
        .map_err(|e| XunError::user(format!("batch set failed: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("added".into(), Value::Int(result.added as i64));
    rec.insert("updated".into(), Value::Int(result.updated as i64));
    rec.insert("skipped".into(), Value::Int(result.skipped as i64));
    Ok(Value::Record(rec))
}

/// 批量删除。
pub fn batch_delete_env(scope: EnvScope, names: &[String], dry_run: bool) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let result = mgr
        .batch_delete(scope, names, dry_run)
        .map_err(|e| XunError::user(format!("batch delete failed: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("deleted".into(), Value::Int(result.deleted as i64));
    rec.insert("skipped".into(), Value::Int(result.skipped as i64));
    Ok(Value::Record(rec))
}

/// 批量重命名。
pub fn batch_rename_env(scope: EnvScope, old: &str, new: &str, dry_run: bool) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let result = mgr
        .batch_rename(scope, old, new, dry_run)
        .map_err(|e| XunError::user(format!("batch rename failed: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("renamed".into(), Value::Int(result.renamed as i64));
    Ok(Value::Record(rec))
}

/// 应用 profile（直接）。
pub fn apply_env(name: &str, scope: Option<EnvScope>) -> Result<Value, XunError> {
    profile_apply_env(name, scope)
}

/// 导出环境变量。
pub fn export_env(scope: EnvScope, format: &str, out: Option<&str>) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let fmt = match format {
        "json" => crate::env_core::types::ExportFormat::Json,
        "env" => crate::env_core::types::ExportFormat::Env,
        "reg" => crate::env_core::types::ExportFormat::Reg,
        "csv" => crate::env_core::types::ExportFormat::Csv,
        _ => return Err(XunError::user(format!("unsupported export format: {format}"))),
    };
    let content = mgr
        .export_vars(scope, fmt)
        .map_err(|e| XunError::user(format!("export failed: {e}")))?;
    if let Some(path) = out {
        std::fs::write(path, &content)
            .map_err(|e| XunError::user(format!("failed to write file: {e}")))?;
    }
    Ok(Value::String(content))
}

/// 导出环境 bundle（zip）。
pub fn export_all_env(scope: EnvScope, out: Option<&str>) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let data = mgr
        .export_bundle(scope)
        .map_err(|e| XunError::user(format!("export bundle failed: {e}")))?;
    if let Some(path) = out {
        std::fs::write(path, &data)
            .map_err(|e| XunError::user(format!("failed to write file: {e}")))?;
    }
    Ok(Value::Int(data.len() as i64))
}

/// 导出 live 环境。
pub fn export_live_env(
    scope: EnvScope,
    format: &str,
    env_files: &[String],
    set: &[String],
    out: Option<&str>,
) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let fmt = match format {
        "dotenv" => crate::env_core::types::LiveExportFormat::Dotenv,
        "sh" => crate::env_core::types::LiveExportFormat::Sh,
        "json" => crate::env_core::types::LiveExportFormat::Json,
        "reg" => crate::env_core::types::LiveExportFormat::Reg,
        _ => return Err(XunError::user(format!("unsupported live export format: {format}"))),
    };
    let env_paths: Vec<std::path::PathBuf> = env_files.iter().map(std::path::PathBuf::from).collect();
    let set_pairs: Vec<(String, String)> = set
        .iter()
        .filter_map(|s| {
            let mut parts = s.splitn(2, '=');
            Some((parts.next()?.to_string(), parts.next().unwrap_or("").to_string()))
        })
        .collect();
    let content = mgr
        .export_live(scope, fmt, &env_paths, &set_pairs)
        .map_err(|e| XunError::user(format!("export live failed: {e}")))?;
    if let Some(path) = out {
        std::fs::write(path, &content)
            .map_err(|e| XunError::user(format!("failed to write file: {e}")))?;
    }
    Ok(Value::String(content))
}

/// 合并环境变量列表。
pub fn merged_env(
    scope: EnvScope,
    env_files: &[String],
    set: &[String],
) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let env_paths: Vec<std::path::PathBuf> = env_files.iter().map(std::path::PathBuf::from).collect();
    let set_pairs: Vec<(String, String)> = set
        .iter()
        .filter_map(|s| {
            let mut parts = s.splitn(2, '=');
            Some((parts.next()?.to_string(), parts.next().unwrap_or("").to_string()))
        })
        .collect();
    let pairs = mgr
        .merged_env_pairs(scope, &env_paths, &set_pairs)
        .map_err(|e| XunError::user(format!("merged env failed: {e}")))?;
    let items: Vec<Value> = pairs
        .into_iter()
        .map(|(k, v)| {
            let mut rec = crate::xun_core::value::Record::new();
            rec.insert("key".into(), Value::String(k));
            rec.insert("value".into(), Value::String(v));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(items))
}

/// 导入环境变量。
pub fn import_env(
    file: Option<&str>,
    stdin: bool,
    scope: EnvScope,
    mode: &str,
    dry_run: bool,
) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let import_mode = match mode {
        "overwrite" => crate::env_core::types::ImportStrategy::Overwrite,
        _ => crate::env_core::types::ImportStrategy::Merge,
    };
    let result = if stdin {
        let mut content = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut content)
            .map_err(|e| XunError::user(format!("failed to read stdin: {e}")))?;
        mgr.import_content(scope, &content, import_mode, dry_run)
            .map_err(|e| XunError::user(format!("import failed: {e}")))?
    } else {
        let path = file.ok_or_else(|| XunError::user("file path or --stdin is required"))?;
        mgr.import_file(scope, std::path::Path::new(path), import_mode, dry_run)
            .map_err(|e| XunError::user(format!("import failed: {e}")))?
    };
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("added".into(), Value::Int(result.added as i64));
    rec.insert("updated".into(), Value::Int(result.updated as i64));
    rec.insert("skipped".into(), Value::Int(result.skipped as i64));
    Ok(Value::Record(rec))
}

/// Diff live 环境。
pub fn diff_live_env(
    scope: EnvScope,
    snapshot: Option<&str>,
) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let diff = mgr
        .diff_live(scope, snapshot)
        .map_err(|e| XunError::user(format!("diff failed: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("added".into(), Value::Int(diff.added.len() as i64));
    rec.insert("removed".into(), Value::Int(diff.removed.len() as i64));
    rec.insert("changed".into(), Value::Int(diff.changed.len() as i64));
    Ok(Value::Record(rec))
}

/// 依赖图。
pub fn graph_env(name: &str, scope: EnvScope, max_depth: usize) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let tree = mgr
        .dependency_tree(scope, name, max_depth)
        .map_err(|e| XunError::user(format!("dependency tree failed: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("root".into(), Value::String(tree.root));
    rec.insert("lines".into(), Value::List(tree.lines.into_iter().map(Value::String).collect()));
    rec.insert("missing".into(), Value::List(tree.missing.into_iter().map(Value::String).collect()));
    rec.insert("cycles".into(), Value::List(tree.cycles.into_iter().map(Value::String).collect()));
    Ok(Value::Record(rec))
}

/// 验证 schema。
pub fn validate_env(scope: EnvScope, strict: bool) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let report = mgr
        .validate_schema(scope, strict)
        .map_err(|e| XunError::user(format!("validation failed: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("passed".into(), Value::Bool(report.passed));
    rec.insert("errors".into(), Value::Int(report.errors as i64));
    rec.insert("warnings".into(), Value::Int(report.warnings as i64));
    rec.insert("total_vars".into(), Value::Int(report.total_vars as i64));
    Ok(Value::Record(rec))
}

/// 显示 schema。
pub fn schema_show_env() -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let schema = mgr
        .schema_show()
        .map_err(|e| XunError::user(format!("schema show failed: {e}")))?;
    let items: Vec<Value> = schema
        .rules
        .into_iter()
        .map(|r| {
            let mut rec = crate::xun_core::value::Record::new();
            rec.insert("pattern".into(), Value::String(r.pattern));
            rec.insert("required".into(), Value::Bool(r.required));
            rec.insert("warn_only".into(), Value::Bool(r.warn_only));
            if let Some(regex) = r.regex {
                rec.insert("regex".into(), Value::String(regex));
            }
            if !r.enum_values.is_empty() {
                rec.insert("enum_values".into(), Value::List(r.enum_values.into_iter().map(Value::String).collect()));
            }
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(items))
}

/// 添加 required 规则。
pub fn schema_add_required_env(pattern: &str, warn_only: bool) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    mgr.schema_add_required(pattern, warn_only)
        .map_err(|e| XunError::user(format!("schema add required failed: {e}")))?;
    Ok(Value::Null)
}

/// 添加 regex 规则。
pub fn schema_add_regex_env(pattern: &str, regex: &str, warn_only: bool) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    mgr.schema_add_regex(pattern, regex, warn_only)
        .map_err(|e| XunError::user(format!("schema add regex failed: {e}")))?;
    Ok(Value::Null)
}

/// 添加 enum 规则。
pub fn schema_add_enum_env(pattern: &str, values: &[String], warn_only: bool) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    mgr.schema_add_enum(pattern, values, warn_only)
        .map_err(|e| XunError::user(format!("schema add enum failed: {e}")))?;
    Ok(Value::Null)
}

/// 删除 schema 规则。
pub fn schema_remove_env(pattern: &str) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    mgr.schema_remove(pattern)
        .map_err(|e| XunError::user(format!("schema remove failed: {e}")))?;
    Ok(Value::Null)
}

/// 重置 schema。
pub fn schema_reset_env() -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    mgr.schema_reset()
        .map_err(|e| XunError::user(format!("schema reset failed: {e}")))?;
    Ok(Value::Null)
}

/// 设置注解。
pub fn annotate_set_env(name: &str, note: &str) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let entry = mgr
        .annotate_set(name, note)
        .map_err(|e| XunError::user(format!("annotate set failed: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("name".into(), Value::String(entry.name));
    rec.insert("note".into(), Value::String(entry.note));
    Ok(Value::Record(rec))
}

/// 列出注解。
pub fn annotate_list_env() -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let entries = mgr
        .annotate_list()
        .map_err(|e| XunError::user(format!("annotate list failed: {e}")))?;
    let items: Vec<Value> = entries
        .into_iter()
        .map(|e| {
            let mut rec = crate::xun_core::value::Record::new();
            rec.insert("name".into(), Value::String(e.name));
            rec.insert("note".into(), Value::String(e.note));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(items))
}

/// 显示 config。
pub fn config_show_env() -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let cfg = mgr.env_config_show();
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("snapshot_dir".into(), Value::String(cfg.snapshot_dir().to_string_lossy().to_string()));
    rec.insert("profile_dir".into(), Value::String(cfg.profile_dir().to_string_lossy().to_string()));
    Ok(Value::Record(rec))
}

/// Config path。
pub fn config_path_env() -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let path = mgr.env_config_path();
    Ok(Value::String(path.to_string_lossy().to_string()))
}

/// 重置 config。
pub fn config_reset_env() -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    mgr.env_config_reset()
        .map_err(|e| XunError::user(format!("config reset failed: {e}")))?;
    Ok(Value::Null)
}

/// 获取 config 值。
pub fn config_get_env(key: &str) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let value = mgr
        .env_config_get(key)
        .map_err(|e| XunError::user(format!("config get failed: {e}")))?;
    Ok(Value::String(value))
}

/// 设置 config 值。
pub fn config_set_env(key: &str, value: &str) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    mgr.env_config_set(key, value)
        .map_err(|e| XunError::user(format!("config set failed: {e}")))?;
    Ok(Value::Null)
}

/// 审计日志。
pub fn audit_env(limit: usize) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let entries = mgr
        .audit_list(limit)
        .map_err(|e| XunError::user(format!("audit list failed: {e}")))?;
    let items: Vec<Value> = entries
        .into_iter()
        .map(|e| {
            let mut rec = crate::xun_core::value::Record::new();
            rec.insert("at".into(), Value::String(e.at));
            rec.insert("action".into(), Value::String(e.action));
            rec.insert("scope".into(), Value::String(format!("{:?}", e.scope)));
            rec.insert("result".into(), Value::String(e.result));
            if let Some(name) = e.name {
                rec.insert("name".into(), Value::String(name));
            }
            if let Some(msg) = e.message {
                rec.insert("message".into(), Value::String(msg));
            }
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(items))
}

/// 模板展开。
pub fn template_env(input: &str, scope: EnvScope, validate_only: bool) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    if validate_only {
        let report = mgr
            .template_validate(scope, input)
            .map_err(|e| XunError::user(format!("template validate failed: {e}")))?;
        let mut rec = crate::xun_core::value::Record::new();
        rec.insert("valid".into(), Value::Bool(report.valid));
        rec.insert("missing".into(), Value::Int(report.missing.len() as i64));
        rec.insert("cycles".into(), Value::Int(report.cycles.len() as i64));
        Ok(Value::Record(rec))
    } else {
        let result = mgr
            .template_expand(scope, input)
            .map_err(|e| XunError::user(format!("template expand failed: {e}")))?;
        Ok(Value::String(result.expanded))
    }
}

/// 运行命令。
pub fn run_env(
    env_files: &[String],
    set: &[String],
    scope: EnvScope,
    schema_check: bool,
    notify: bool,
    command: &[String],
) -> Result<Value, XunError> {
    let mgr = EnvManager::new();
    let env_paths: Vec<std::path::PathBuf> = env_files.iter().map(std::path::PathBuf::from).collect();
    let set_pairs: Vec<(String, String)> = set
        .iter()
        .filter_map(|s| {
            let mut parts = s.splitn(2, '=');
            Some((parts.next()?.to_string(), parts.next().unwrap_or("").to_string()))
        })
        .collect();
    let result = mgr
        .run_command(scope, &env_paths, &set_pairs, command, None, schema_check, notify, false, 0)
        .map_err(|e| XunError::user(format!("run failed: {e}")))?;
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("success".into(), Value::Bool(result.success));
    if let Some(code) = result.exit_code {
        rec.insert("exit_code".into(), Value::Int(code as i64));
    }
    if !result.stdout.is_empty() {
        rec.insert("stdout".into(), Value::String(result.stdout));
    }
    if !result.stderr.is_empty() {
        rec.insert("stderr".into(), Value::String(result.stderr));
    }
    Ok(Value::Record(rec))
}
