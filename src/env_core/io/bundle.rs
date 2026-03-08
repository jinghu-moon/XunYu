use std::io::Write;

use super::super::registry;
use super::super::types::{EnvError, EnvResult, EnvScope, ExportFormat};
use super::export_render::{export_ext, export_vars};

pub(super) fn export_bundle(scope: EnvScope) -> EnvResult<Vec<u8>> {
    let scopes = match scope {
        EnvScope::All => vec![EnvScope::User, EnvScope::System],
        other => vec![other],
    };

    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::<u8>::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for target_scope in scopes {
        let vars = registry::list_vars(target_scope)?;
        for format in [
            ExportFormat::Json,
            ExportFormat::Env,
            ExportFormat::Reg,
            ExportFormat::Csv,
        ] {
            let content = export_vars(&vars, target_scope, format)?;
            let entry_name = format!("xun-env-{}.{}", target_scope, export_ext(format));
            writer
                .start_file(entry_name, options)
                .map_err(zip_to_env_error)?;
            writer.write_all(content.as_bytes())?;
        }
    }

    let cursor = writer.finish().map_err(zip_to_env_error)?;
    Ok(cursor.into_inner())
}

fn zip_to_env_error(err: zip::result::ZipError) -> EnvError {
    EnvError::Other(format!("zip error: {}", err))
}
