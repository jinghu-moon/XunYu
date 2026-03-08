use super::*;

impl EnvManager {
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

    #[cfg(feature = "dashboard")]
    pub fn annotate_get(&self, name: &str) -> EnvResult<Option<AnnotationEntry>> {
        annotations::get_annotation(&self.cfg, name)
    }

    #[cfg(feature = "dashboard")]
    pub fn annotate_delete(&self, name: &str) -> EnvResult<bool> {
        annotations::delete_annotation(&self.cfg, name)
    }

    pub fn audit_list(&self, limit: usize) -> EnvResult<Vec<EnvAuditEntry>> {
        audit::list_audit(&self.cfg, limit)
    }
}
