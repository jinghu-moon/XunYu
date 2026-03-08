use super::*;

impl EnvManager {
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
}
