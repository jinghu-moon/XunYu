use super::*;

impl EnvManager {
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

    pub fn watch_diff(
        &self,
        scope: EnvScope,
        prev: &[EnvVar],
        next: &[EnvVar],
    ) -> Vec<EnvWatchEvent> {
        watch::diff_vars(scope, prev, next)
    }

    #[cfg(feature = "tui")]
    pub fn path_entries(&self, scope: EnvScope) -> EnvResult<Vec<String>> {
        registry::get_path_entries(scope)
    }

    pub fn doctor_run(&self, scope: EnvScope) -> EnvResult<DoctorReport> {
        doctor::run_doctor(scope)
    }

    pub fn check_run(&self, scope: EnvScope) -> EnvResult<DoctorReport> {
        self.doctor_run(scope)
    }
}
