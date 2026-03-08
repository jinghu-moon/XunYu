mod bundle;
mod export_render;
mod import_apply;
mod import_parse;

use std::path::Path;

use super::types::{
    EnvResult, EnvScope, EnvVar, ExportFormat, ImportApplyResult, ImportStrategy, ParsedImport,
};

pub fn export_vars(vars: &[EnvVar], scope: EnvScope, format: ExportFormat) -> EnvResult<String> {
    export_render::export_vars(vars, scope, format)
}

pub fn export_bundle(scope: EnvScope) -> EnvResult<Vec<u8>> {
    bundle::export_bundle(scope)
}

pub fn parse_import_file(path: &Path) -> EnvResult<ParsedImport> {
    import_parse::parse_import_file(path)
}

pub fn parse_import_content(content: &str) -> EnvResult<ParsedImport> {
    import_parse::parse_import_content(content)
}

pub fn apply_import(
    scope: EnvScope,
    parsed: &ParsedImport,
    strategy: ImportStrategy,
    dry_run: bool,
) -> EnvResult<ImportApplyResult> {
    import_apply::apply_import(scope, parsed, strategy, dry_run)
}
