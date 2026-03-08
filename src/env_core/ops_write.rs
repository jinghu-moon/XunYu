use super::*;

impl EnvManager {
    pub fn env_config_set(&self, key: &str, value: &str) -> EnvResult<EnvCoreConfig> {
        let mut cfg = load_env_config();
        set_config_value(&mut cfg, key, value)?;
        save_env_config(&cfg)?;
        Ok(cfg)
    }

    pub fn env_config_reset(&self) -> EnvResult<EnvCoreConfig> {
        reset_env_config()
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
}
