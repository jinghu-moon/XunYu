pub mod annotations;
pub mod audit;
pub mod batch;
pub mod config;
pub mod dep_graph;
pub mod diff;
pub mod doctor;
pub mod events;
pub mod io;
pub mod lock;
pub mod notifier;
pub mod profile;
pub mod registry;
pub mod schema;
pub mod snapshot;
pub mod template;
pub mod types;
pub mod uac;
pub mod var_type;
pub mod watch;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

use config::{
    EnvCoreConfig, config_file_path, get_config_value, load_env_config, reset_env_config,
    save_env_config, set_config_value,
};
use types::{
    AnnotationEntry, BatchResult, DoctorFixResult, DoctorReport, EnvAuditEntry, EnvDiff, EnvError,
    EnvEvent, EnvEventType, EnvProfileMeta, EnvResult, EnvSchema, EnvScope, EnvStatusSummary,
    EnvVar, EnvWatchEvent, ExportFormat, ImportApplyResult, ImportStrategy, LiveExportFormat,
    RunCommandResult, SchemaRule, ShellExportFormat, SnapshotMeta, TemplateExpandResult,
    TemplateValidationReport, ValidationReport,
};

pub use types::*;

pub type EventCallback = Arc<dyn Fn(EnvEvent) + Send + Sync>;

#[derive(Clone)]
pub struct EnvManager {
    cfg: EnvCoreConfig,
    event_cb: Option<EventCallback>,
}

impl Default for EnvManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvManager {
    pub fn new() -> Self {
        Self {
            cfg: load_env_config(),
            event_cb: None,
        }
    }

    pub fn with_event_callback(mut self, event_cb: EventCallback) -> Self {
        self.event_cb = Some(event_cb);
        self
    }

    pub fn config(&self) -> &EnvCoreConfig {
        &self.cfg
    }

    pub fn list_vars(&self, scope: EnvScope) -> EnvResult<Vec<EnvVar>> {
        registry::list_vars(scope)
    }

    pub fn search_vars(&self, scope: EnvScope, query: &str) -> EnvResult<Vec<EnvVar>> {
        let q = query.trim().to_lowercase();
        let vars = self.list_vars(scope)?;
        if q.is_empty() {
            return Ok(vars);
        }
        Ok(vars
            .into_iter()
            .filter(|v| {
                v.name.to_lowercase().contains(&q) || v.raw_value.to_lowercase().contains(&q)
            })
            .collect())
    }

    pub fn get_var(&self, scope: EnvScope, name: &str) -> EnvResult<Option<EnvVar>> {
        if scope == EnvScope::All {
            return Err(EnvError::InvalidInput(
                "get does not support --scope all".to_string(),
            ));
        }
        registry::get_var(scope, name)
    }

    pub fn status_overview(&self, scope: EnvScope) -> EnvResult<EnvStatusSummary> {
        let mut notes = Vec::new();

        let user_vars = match registry::list_scope(EnvScope::User) {
            Ok(vars) => Some(vars.len()),
            Err(e) => {
                notes.push(format!("user vars unavailable: {}", e));
                None
            }
        };
        let system_vars = match registry::list_scope(EnvScope::System) {
            Ok(vars) => Some(vars.len()),
            Err(e) => {
                notes.push(format!("system vars unavailable: {}", e));
                None
            }
        };
        let total_vars = match scope {
            EnvScope::User => user_vars,
            EnvScope::System => system_vars,
            EnvScope::All => match (user_vars, system_vars) {
                (Some(u), Some(s)) => Some(u + s),
                _ => None,
            },
        };

        let snapshots = snapshot::list_snapshots(&self.cfg)?;
        let latest_snapshot_id = snapshots.first().map(|s| s.id.clone());
        let latest_snapshot_at = snapshots.first().map(|s| s.created_at.clone());

        let profiles = profile::list_profiles(&self.cfg)?;
        let schema = schema::load_schema(&self.cfg)?;
        let annotations = annotations::list_annotations(&self.cfg)?;
        let audit_entries = audit::list_audit(&self.cfg, 0)?;
        let audit_count = audit_entries.len();
        let last_audit_at = audit_entries.last().map(|e| e.at.clone());

        Ok(EnvStatusSummary {
            scope,
            user_vars,
            system_vars,
            total_vars,
            snapshots: snapshots.len(),
            latest_snapshot_id,
            latest_snapshot_at,
            profiles: profiles.len(),
            schema_rules: schema.rules.len(),
            annotations: annotations.len(),
            audit_entries: audit_count,
            last_audit_at,
            notes,
        })
    }

    pub fn template_expand(&self, scope: EnvScope, input: &str) -> EnvResult<TemplateExpandResult> {
        template::template_expand(scope, input)
    }

    pub fn template_validate(
        &self,
        scope: EnvScope,
        input: &str,
    ) -> EnvResult<TemplateValidationReport> {
        template::template_validate(scope, input)
    }

    pub fn runtime_env(
        &self,
        scope: EnvScope,
        env_files: &[PathBuf],
        set_pairs: &[(String, String)],
    ) -> EnvResult<std::collections::BTreeMap<String, String>> {
        template::build_runtime_env(scope, env_files, set_pairs)
    }

    pub fn render_shell_exports(
        &self,
        scope: EnvScope,
        env_files: &[PathBuf],
        set_pairs: &[(String, String)],
        shell: ShellExportFormat,
    ) -> EnvResult<String> {
        let env_map = self.runtime_env(scope, env_files, set_pairs)?;
        Ok(template::render_shell_exports(&env_map, shell))
    }

    pub fn export_live(
        &self,
        scope: EnvScope,
        format: LiveExportFormat,
        env_files: &[PathBuf],
        set_pairs: &[(String, String)],
    ) -> EnvResult<String> {
        let env_map = self.runtime_env(scope, env_files, set_pairs)?;
        template::render_live_export(scope, &env_map, format)
    }

    pub fn merged_env_pairs(
        &self,
        scope: EnvScope,
        env_files: &[PathBuf],
        set_pairs: &[(String, String)],
    ) -> EnvResult<Vec<(String, String)>> {
        let env_map = self.runtime_env(scope, env_files, set_pairs)?;
        Ok(env_map.into_iter().collect())
    }

    pub fn env_config_path(&self) -> PathBuf {
        config_file_path()
    }

    pub fn env_config_show(&self) -> EnvCoreConfig {
        load_env_config()
    }

    pub fn env_config_get(&self, key: &str) -> EnvResult<String> {
        let cfg = load_env_config();
        get_config_value(&cfg, key).ok_or_else(|| {
            EnvError::InvalidInput(format!(
                "unknown env config key '{}', expected snapshot_dir|profile_dir|max_snapshots|general.max_snapshots|lock_timeout_ms|stale_lock_secs|notify_enabled|allow_run|snapshot_every_secs|general.snapshot_every_secs",
                key
            ))
        })
    }

    pub fn env_config_set(&self, key: &str, value: &str) -> EnvResult<EnvCoreConfig> {
        let mut cfg = load_env_config();
        set_config_value(&mut cfg, key, value)?;
        save_env_config(&cfg)?;
        Ok(cfg)
    }

    pub fn env_config_reset(&self) -> EnvResult<EnvCoreConfig> {
        reset_env_config()
    }

    pub fn validate_schema(&self, scope: EnvScope, strict: bool) -> EnvResult<ValidationReport> {
        schema::validate_schema(&self.cfg, scope, strict)
    }

    pub fn schema_show(&self) -> EnvResult<EnvSchema> {
        schema::load_schema(&self.cfg)
    }

    pub fn schema_add_required(&self, pattern: &str, warn_only: bool) -> EnvResult<EnvSchema> {
        schema::add_or_replace_rule(
            &self.cfg,
            SchemaRule {
                pattern: pattern.to_string(),
                required: true,
                warn_only,
                regex: None,
                enum_values: Vec::new(),
            },
        )
    }

    pub fn schema_add_regex(
        &self,
        pattern: &str,
        regex: &str,
        warn_only: bool,
    ) -> EnvResult<EnvSchema> {
        schema::add_or_replace_rule(
            &self.cfg,
            SchemaRule {
                pattern: pattern.to_string(),
                required: false,
                warn_only,
                regex: Some(regex.to_string()),
                enum_values: Vec::new(),
            },
        )
    }

    pub fn schema_add_enum(
        &self,
        pattern: &str,
        enum_values: &[String],
        warn_only: bool,
    ) -> EnvResult<EnvSchema> {
        schema::add_or_replace_rule(
            &self.cfg,
            SchemaRule {
                pattern: pattern.to_string(),
                required: false,
                warn_only,
                regex: None,
                enum_values: enum_values.to_vec(),
            },
        )
    }

    pub fn schema_remove(&self, pattern: &str) -> EnvResult<EnvSchema> {
        schema::remove_rule(&self.cfg, pattern)
    }

    pub fn schema_reset(&self) -> EnvResult<EnvSchema> {
        schema::reset_schema(&self.cfg)
    }

    pub fn annotate_set(&self, name: &str, note: &str) -> EnvResult<AnnotationEntry> {
        annotations::set_annotation(&self.cfg, name, note)
    }

    pub fn annotate_list(&self) -> EnvResult<Vec<AnnotationEntry>> {
        annotations::list_annotations(&self.cfg)
    }

    pub fn annotate_get(&self, name: &str) -> EnvResult<Option<AnnotationEntry>> {
        annotations::get_annotation(&self.cfg, name)
    }

    pub fn annotate_delete(&self, name: &str) -> EnvResult<bool> {
        annotations::delete_annotation(&self.cfg, name)
    }

    pub fn audit_list(&self, limit: usize) -> EnvResult<Vec<EnvAuditEntry>> {
        audit::list_audit(&self.cfg, limit)
    }

    pub fn watch_diff(
        &self,
        scope: EnvScope,
        prev: &[EnvVar],
        next: &[EnvVar],
    ) -> Vec<EnvWatchEvent> {
        watch::diff_vars(scope, prev, next)
    }

    pub fn notify_run_result(
        &self,
        command_line: &str,
        exit_code: Option<i32>,
        success: bool,
    ) -> EnvResult<bool> {
        notifier::notify_run_result(&self.cfg, command_line, exit_code, success)
    }

    pub fn dependency_tree(
        &self,
        scope: EnvScope,
        root: &str,
        max_depth: usize,
    ) -> EnvResult<EnvDepTree> {
        let vars = self.list_vars(scope)?;
        dep_graph::build_tree(scope, &vars, root, max_depth)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_command(
        &self,
        scope: EnvScope,
        env_files: &[PathBuf],
        set_pairs: &[(String, String)],
        command: &[String],
        cwd: Option<&Path>,
        schema_check: bool,
        notify: bool,
        capture_output: bool,
        max_output: usize,
    ) -> EnvResult<RunCommandResult> {
        if command.is_empty() {
            return Err(EnvError::InvalidInput(
                "run requires command tokens".to_string(),
            ));
        }

        if schema_check {
            let report = self.validate_schema(scope, false)?;
            if report.errors > 0 {
                return Err(EnvError::Other(format!(
                    "schema-check failed: errors={}, warnings={}",
                    report.errors, report.warnings
                )));
            }
            if report.warnings > 0 {
                return Err(EnvError::InvalidInput(format!(
                    "schema-check failed: errors={}, warnings={}",
                    report.errors, report.warnings
                )));
            }
        }

        let env_map = self.runtime_env(scope, env_files, set_pairs)?;
        let mut cmd = Command::new(&command[0]);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.env_clear();
        cmd.envs(&env_map);

        let command_line = command.join(" ");
        let result = if capture_output {
            let output = cmd.output()?;
            let cap = max_output.clamp(1024, 1024 * 1024 * 8);
            let (stdout, stdout_truncated) = truncate_output(&output.stdout, cap);
            let (stderr, stderr_truncated) = truncate_output(&output.stderr, cap);
            RunCommandResult {
                command_line,
                exit_code: output.status.code(),
                success: output.status.success(),
                stdout,
                stderr,
                truncated: stdout_truncated || stderr_truncated,
            }
        } else {
            let status = cmd.status()?;
            RunCommandResult {
                command_line,
                exit_code: status.code(),
                success: status.success(),
                stdout: String::new(),
                stderr: String::new(),
                truncated: false,
            }
        };

        if notify {
            let _ = self.notify_run_result(&result.command_line, result.exit_code, result.success);
        }

        Ok(result)
    }

    pub fn set_var(
        &self,
        scope: EnvScope,
        name: &str,
        value: &str,
        no_snapshot: bool,
    ) -> EnvResult<()> {
        self.with_write_guard(scope, "set", !no_snapshot, || {
            registry::set_var(scope, name, value)?;
            Ok(Some(events::build_event(
                EnvEventType::Changed,
                scope,
                Some(name.to_string()),
                Some("set variable".to_string()),
            )))
        })
    }

    pub fn delete_var(&self, scope: EnvScope, name: &str) -> EnvResult<bool> {
        let mut deleted = false;
        self.with_write_guard(scope, "delete", true, || {
            deleted = registry::delete_var(scope, name)?;
            Ok(Some(events::build_event(
                EnvEventType::Changed,
                scope,
                Some(name.to_string()),
                Some("delete variable".to_string()),
            )))
        })?;
        Ok(deleted)
    }

    pub fn path_add(&self, scope: EnvScope, entry: &str, head: bool) -> EnvResult<bool> {
        let mut changed = false;
        self.with_write_guard(scope, "path.add", true, || {
            changed = registry::add_path_entry(scope, entry, head)?;
            Ok(Some(events::build_event(
                EnvEventType::Changed,
                scope,
                Some("PATH".to_string()),
                Some(if head {
                    "path add head".to_string()
                } else {
                    "path add tail".to_string()
                }),
            )))
        })?;
        Ok(changed)
    }

    pub fn path_remove(&self, scope: EnvScope, entry: &str) -> EnvResult<bool> {
        let mut changed = false;
        self.with_write_guard(scope, "path.remove", true, || {
            changed = registry::remove_path_entry(scope, entry)?;
            Ok(Some(events::build_event(
                EnvEventType::Changed,
                scope,
                Some("PATH".to_string()),
                Some("path remove".to_string()),
            )))
        })?;
        Ok(changed)
    }

    pub fn path_entries(&self, scope: EnvScope) -> EnvResult<Vec<String>> {
        registry::get_path_entries(scope)
    }

    pub fn snapshot_create(&self, desc: Option<&str>) -> EnvResult<SnapshotMeta> {
        let description = desc.unwrap_or("manual snapshot");
        let meta = snapshot::create_snapshot(&self.cfg, description)?;
        self.emit_event(events::build_event(
            EnvEventType::Snapshot,
            EnvScope::All,
            None,
            Some(format!("snapshot created: {}", meta.id)),
        ));
        Ok(meta)
    }

    pub fn snapshot_list(&self) -> EnvResult<Vec<SnapshotMeta>> {
        snapshot::list_snapshots(&self.cfg)
    }

    pub fn snapshot_prune(&self, keep: usize) -> EnvResult<usize> {
        let keep = keep.min(10_000);
        let removed = lock::try_with_lock(&self.cfg, "snapshot.prune", || {
            snapshot::prune_snapshots(&self.cfg, keep)
        })?;
        let message = format!("snapshot prune keep={}, removed={}", keep, removed);
        let _ = audit::append_audit(
            &self.cfg,
            &EnvAuditEntry {
                at: events::now_iso(),
                action: "snapshot.prune".to_string(),
                scope: EnvScope::All,
                result: "ok".to_string(),
                name: None,
                message: Some(message.clone()),
            },
        );
        self.emit_event(events::build_event(
            EnvEventType::Snapshot,
            EnvScope::All,
            None,
            Some(message),
        ));
        Ok(removed)
    }

    pub fn snapshot_restore(
        &self,
        scope: EnvScope,
        snapshot_id: Option<&str>,
        latest: bool,
    ) -> EnvResult<SnapshotMeta> {
        let target = if let Some(id) = snapshot_id {
            let snap = snapshot::load_snapshot_by_id(&self.cfg, id)?;
            SnapshotMeta {
                id: snap.id,
                description: snap.description,
                created_at: snap.created_at,
                path: self.cfg.snapshot_dir().join(format!("{id}.json")),
            }
        } else if latest {
            let snap = snapshot::load_latest_snapshot(&self.cfg)?;
            SnapshotMeta {
                id: snap.id,
                description: snap.description,
                created_at: snap.created_at,
                path: self.cfg.snapshot_dir().join("latest.json"),
            }
        } else {
            return Err(EnvError::InvalidInput(
                "restore requires --id <id> or --latest".to_string(),
            ));
        };

        self.with_write_guard(scope, "snapshot.restore", true, || {
            let snapshot = snapshot::load_snapshot_by_id(&self.cfg, &target.id)
                .or_else(|_| snapshot::load_latest_snapshot(&self.cfg))?;
            snapshot::restore_snapshot(&snapshot, scope)?;
            Ok(Some(events::build_event(
                EnvEventType::Snapshot,
                scope,
                None,
                Some(format!("snapshot restored: {}", target.id)),
            )))
        })?;

        Ok(target)
    }

    pub fn doctor_run(&self, scope: EnvScope) -> EnvResult<DoctorReport> {
        doctor::run_doctor(scope)
    }

    pub fn check_run(&self, scope: EnvScope) -> EnvResult<DoctorReport> {
        self.doctor_run(scope)
    }

    pub fn doctor_fix(&self, scope: EnvScope) -> EnvResult<DoctorFixResult> {
        let mut result = DoctorFixResult {
            scope,
            fixed: 0,
            details: Vec::new(),
        };
        self.with_write_guard(scope, "doctor.fix", true, || {
            result = doctor::fix_doctor(scope)?;
            Ok(Some(events::build_event(
                EnvEventType::Doctor,
                scope,
                None,
                Some(format!("doctor fixed {} items", result.fixed)),
            )))
        })?;
        Ok(result)
    }

    pub fn export_vars(&self, scope: EnvScope, format: ExportFormat) -> EnvResult<String> {
        let vars = self.list_vars(scope)?;
        io::export_vars(&vars, scope, format)
    }

    pub fn export_bundle(&self, scope: EnvScope) -> EnvResult<Vec<u8>> {
        io::export_bundle(scope)
    }

    pub fn import_file(
        &self,
        scope: EnvScope,
        path: &std::path::Path,
        strategy: ImportStrategy,
        dry_run: bool,
    ) -> EnvResult<ImportApplyResult> {
        let parsed = io::parse_import_file(path)?;
        self.import_parsed(scope, &parsed, strategy, dry_run)
    }

    pub fn import_content(
        &self,
        scope: EnvScope,
        content: &str,
        strategy: ImportStrategy,
        dry_run: bool,
    ) -> EnvResult<ImportApplyResult> {
        let parsed = io::parse_import_content(content)?;
        self.import_parsed(scope, &parsed, strategy, dry_run)
    }

    pub fn diff_live(&self, scope: EnvScope, snapshot_id: Option<&str>) -> EnvResult<EnvDiff> {
        let snapshot = match snapshot_id {
            Some(id) => snapshot::load_snapshot_by_id(&self.cfg, id)?,
            None => snapshot::load_latest_snapshot(&self.cfg)?,
        };
        let reference = snapshot_to_env_vars(&snapshot, scope);
        let live = registry::list_vars(scope)?;
        Ok(diff::diff_var_lists(&reference, &live))
    }

    pub fn diff_since(&self, scope: EnvScope, since_raw: &str) -> EnvResult<EnvDiff> {
        let since = parse_since_datetime(since_raw)?;
        let baseline = select_snapshot_id_by_since(&self.cfg, since)?
            .ok_or_else(|| EnvError::NotFound("no snapshots found".to_string()))?;
        self.diff_live(scope, Some(&baseline))
    }

    pub fn profile_list(&self) -> EnvResult<Vec<EnvProfileMeta>> {
        profile::list_profiles(&self.cfg)
    }

    pub fn profile_capture(&self, name: &str, scope: EnvScope) -> EnvResult<EnvProfileMeta> {
        profile::capture_profile(&self.cfg, name, scope)
    }

    pub fn profile_delete(&self, name: &str) -> EnvResult<bool> {
        profile::delete_profile(&self.cfg, name)
    }

    pub fn profile_apply(
        &self,
        name: &str,
        scope_override: Option<EnvScope>,
    ) -> EnvResult<EnvProfileMeta> {
        let loaded = profile::load_profile(&self.cfg, name)?;
        let target_scope = scope_override.unwrap_or(loaded.scope);
        if !target_scope.is_writable() {
            return Err(EnvError::ScopeNotWritable(target_scope));
        }
        let mut applied = 0usize;
        let profile_name = loaded.name.clone();
        let vars = loaded.vars.clone();
        self.with_write_guard(target_scope, "profile.apply", true, || {
            for item in &vars {
                registry::set_var(target_scope, &item.name, &item.raw_value)?;
                applied += 1;
            }
            Ok(Some(events::build_event(
                EnvEventType::Changed,
                target_scope,
                None,
                Some(format!(
                    "profile '{}' applied ({} vars)",
                    profile_name, applied
                )),
            )))
        })?;
        Ok(EnvProfileMeta {
            name: loaded.name,
            scope: target_scope,
            created_at: loaded.created_at,
            var_count: vars.len(),
            path: profile::profile_file_path(&self.cfg, name)?,
        })
    }

    pub fn profile_diff(&self, name: &str, scope_override: Option<EnvScope>) -> EnvResult<EnvDiff> {
        let loaded = profile::load_profile(&self.cfg, name)?;
        let target_scope = scope_override.unwrap_or(loaded.scope);
        let baseline = profile::profile_vars_as_env(&loaded, Some(target_scope))?;
        let live = registry::list_vars(target_scope)?;
        Ok(diff::diff_var_lists(&baseline, &live))
    }

    pub fn batch_set(
        &self,
        scope: EnvScope,
        items: &[(String, String)],
        dry_run: bool,
    ) -> EnvResult<BatchResult> {
        if dry_run {
            return batch::batch_set(scope, items, true);
        }
        let mut result = BatchResult {
            dry_run: false,
            scope,
            added: 0,
            updated: 0,
            deleted: 0,
            renamed: 0,
            skipped: 0,
            details: Vec::new(),
        };
        self.with_write_guard(scope, "batch.set", true, || {
            result = batch::batch_set(scope, items, false)?;
            Ok(Some(events::build_event(
                EnvEventType::Changed,
                scope,
                None,
                Some(format!(
                    "batch set added={}, updated={}, skipped={}",
                    result.added, result.updated, result.skipped
                )),
            )))
        })?;
        Ok(result)
    }

    pub fn batch_delete(
        &self,
        scope: EnvScope,
        names: &[String],
        dry_run: bool,
    ) -> EnvResult<BatchResult> {
        if dry_run {
            return batch::batch_delete(scope, names, true);
        }
        let mut result = BatchResult {
            dry_run: false,
            scope,
            added: 0,
            updated: 0,
            deleted: 0,
            renamed: 0,
            skipped: 0,
            details: Vec::new(),
        };
        self.with_write_guard(scope, "batch.delete", true, || {
            result = batch::batch_delete(scope, names, false)?;
            Ok(Some(events::build_event(
                EnvEventType::Changed,
                scope,
                None,
                Some(format!(
                    "batch delete deleted={}, skipped={}",
                    result.deleted, result.skipped
                )),
            )))
        })?;
        Ok(result)
    }

    pub fn batch_rename(
        &self,
        scope: EnvScope,
        old_name: &str,
        new_name: &str,
        dry_run: bool,
    ) -> EnvResult<BatchResult> {
        if dry_run {
            return batch::batch_rename(scope, old_name, new_name, true);
        }
        let mut result = BatchResult {
            dry_run: false,
            scope,
            added: 0,
            updated: 0,
            deleted: 0,
            renamed: 0,
            skipped: 0,
            details: Vec::new(),
        };
        self.with_write_guard(scope, "batch.rename", true, || {
            result = batch::batch_rename(scope, old_name, new_name, false)?;
            Ok(Some(events::build_event(
                EnvEventType::Changed,
                scope,
                Some(new_name.to_string()),
                Some(format!("batch rename {} -> {}", old_name, new_name)),
            )))
        })?;
        Ok(result)
    }

    pub fn path_dedup(
        &self,
        scope: EnvScope,
        remove_missing: bool,
        dry_run: bool,
    ) -> EnvResult<BatchResult> {
        if dry_run {
            return batch::path_dedup(scope, remove_missing, true);
        }
        let mut result = BatchResult {
            dry_run: false,
            scope,
            added: 0,
            updated: 0,
            deleted: 0,
            renamed: 0,
            skipped: 0,
            details: Vec::new(),
        };
        self.with_write_guard(scope, "path.dedup", true, || {
            result = batch::path_dedup(scope, remove_missing, false)?;
            Ok(Some(events::build_event(
                EnvEventType::Changed,
                scope,
                Some("PATH".to_string()),
                Some(format!(
                    "path dedup removed={}, skipped={}",
                    result.deleted, result.skipped
                )),
            )))
        })?;
        Ok(result)
    }

    fn import_parsed(
        &self,
        scope: EnvScope,
        parsed: &types::ParsedImport,
        strategy: ImportStrategy,
        dry_run: bool,
    ) -> EnvResult<ImportApplyResult> {
        if dry_run {
            return io::apply_import(scope, parsed, strategy, true);
        }
        let mut result = ImportApplyResult {
            dry_run: false,
            added: 0,
            updated: 0,
            skipped: 0,
            changed_names: Vec::new(),
        };
        self.with_write_guard(scope, "import", true, || {
            result = io::apply_import(scope, parsed, strategy, false)?;
            Ok(Some(events::build_event(
                EnvEventType::Import,
                scope,
                None,
                Some(format!(
                    "import added={}, updated={}, skipped={}",
                    result.added, result.updated, result.skipped
                )),
            )))
        })?;
        Ok(result)
    }

    fn with_write_guard<F>(
        &self,
        scope: EnvScope,
        action: &str,
        snapshot_before: bool,
        op: F,
    ) -> EnvResult<()>
    where
        F: FnOnce() -> EnvResult<Option<EnvEvent>>,
    {
        if !matches!(scope, EnvScope::User | EnvScope::System | EnvScope::All) {
            return Err(EnvError::ScopeNotWritable(scope));
        }
        if uac::requires_elevation(scope) {
            return Err(EnvError::PermissionDenied(uac::elevation_hint(scope)));
        }

        lock::try_with_lock(&self.cfg, action, || {
            if snapshot_before {
                let _ = snapshot::create_snapshot(&self.cfg, &format!("pre-{action}"));
            }
            match op() {
                Ok(evt_opt) => {
                    let (name, message) = if let Some(evt) = evt_opt.as_ref() {
                        (evt.name.clone(), evt.message.clone())
                    } else {
                        (None, Some("write operation completed".to_string()))
                    };
                    let _ = audit::append_audit(
                        &self.cfg,
                        &EnvAuditEntry {
                            at: events::now_iso(),
                            action: action.to_string(),
                            scope,
                            result: "ok".to_string(),
                            name,
                            message,
                        },
                    );
                    if let Some(evt) = evt_opt {
                        self.emit_event(evt);
                    }
                    Ok(())
                }
                Err(err) => {
                    let _ = audit::append_audit(
                        &self.cfg,
                        &EnvAuditEntry {
                            at: events::now_iso(),
                            action: action.to_string(),
                            scope,
                            result: "error".to_string(),
                            name: None,
                            message: Some(err.to_string()),
                        },
                    );
                    Err(err)
                }
            }
        })
    }

    fn emit_event(&self, event: EnvEvent) {
        if let Some(cb) = &self.event_cb {
            cb(event);
        }
    }
}

fn snapshot_to_env_vars(snapshot: &types::Snapshot, scope: EnvScope) -> Vec<EnvVar> {
    let mut out = Vec::new();
    match scope {
        EnvScope::User => {
            out.extend(snapshot.user_vars.iter().cloned().map(|v| EnvVar {
                scope: EnvScope::User,
                inferred_kind: var_type::infer_var_kind(&v.name, &v.raw_value),
                name: v.name,
                raw_value: v.raw_value,
                reg_type: v.reg_type,
            }));
        }
        EnvScope::System => {
            out.extend(snapshot.system_vars.iter().cloned().map(|v| EnvVar {
                scope: EnvScope::System,
                inferred_kind: var_type::infer_var_kind(&v.name, &v.raw_value),
                name: v.name,
                raw_value: v.raw_value,
                reg_type: v.reg_type,
            }));
        }
        EnvScope::All => {
            out.extend(snapshot.user_vars.iter().cloned().map(|v| EnvVar {
                scope: EnvScope::User,
                inferred_kind: var_type::infer_var_kind(&v.name, &v.raw_value),
                name: v.name,
                raw_value: v.raw_value,
                reg_type: v.reg_type,
            }));
            out.extend(snapshot.system_vars.iter().cloned().map(|v| EnvVar {
                scope: EnvScope::System,
                inferred_kind: var_type::infer_var_kind(&v.name, &v.raw_value),
                name: v.name,
                raw_value: v.raw_value,
                reg_type: v.reg_type,
            }));
        }
    }
    out
}

fn truncate_output(raw: &[u8], max_output: usize) -> (String, bool) {
    if raw.len() <= max_output {
        return (String::from_utf8_lossy(raw).into_owned(), false);
    }
    (
        String::from_utf8_lossy(&raw[..max_output]).into_owned(),
        true,
    )
}

fn parse_since_datetime(raw: &str) -> EnvResult<DateTime<Utc>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(EnvError::InvalidInput(
            "since cannot be empty".to_string(),
        ));
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S") {
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        let dt = date.and_hms_opt(0, 0, 0).ok_or_else(|| {
            EnvError::InvalidInput(format!("invalid since date '{}'", raw))
        })?;
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    Err(EnvError::InvalidInput(format!(
        "invalid since '{}', expected RFC3339 or YYYY-MM-DD or YYYY-MM-DD HH:MM:SS",
        raw
    )))
}

fn parse_snapshot_time(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn select_snapshot_id_by_since(cfg: &EnvCoreConfig, since: DateTime<Utc>) -> EnvResult<Option<String>> {
    let metas = snapshot::list_snapshots(cfg)?;
    if metas.is_empty() {
        return Ok(None);
    }
    let first_id = metas.first().map(|m| m.id.clone());

    let mut best: Option<(DateTime<Utc>, String)> = None;
    let mut oldest: Option<(DateTime<Utc>, String)> = None;

    for meta in metas {
        let ts = match parse_snapshot_time(&meta.created_at) {
            Some(v) => v,
            None => continue,
        };
        if oldest.as_ref().map(|(d, _)| ts < *d).unwrap_or(true) {
            oldest = Some((ts, meta.id.clone()));
        }
        if ts <= since && best.as_ref().map(|(d, _)| ts > *d).unwrap_or(true) {
            best = Some((ts, meta.id.clone()));
        }
    }

    if let Some((_, id)) = best {
        return Ok(Some(id));
    }
    if let Some((_, id)) = oldest {
        return Ok(Some(id));
    }

    Ok(first_id)
}
