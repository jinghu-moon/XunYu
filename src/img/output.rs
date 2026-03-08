use std::path::{Path, PathBuf};

use super::types::OutputFormat;

pub fn derive_output_path(
    input_file: &Path,
    input_root: &Path,
    output_dir: &Path,
    format: OutputFormat,
) -> PathBuf {
    let rel = input_file
        .strip_prefix(input_root)
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            input_file
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| input_file.to_path_buf())
        });

    let mut out = output_dir.join(rel);
    out.set_extension(format.extension());
    out
}

pub fn should_skip(output_path: &Path, overwrite: bool) -> bool {
    !overwrite && output_path.exists()
}
