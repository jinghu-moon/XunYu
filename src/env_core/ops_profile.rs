use super::*;

impl EnvManager {
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
}
