use std::path::{Path, PathBuf};

use walkdir::WalkDir;

const SUPPORTED_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "avif"];

fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

fn output_dir_for_compare(output_dir: &Path) -> PathBuf {
    dunce::canonicalize(output_dir).unwrap_or_else(|_| absolutize(output_dir))
}

fn is_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|s| ext.eq_ignore_ascii_case(s))
        })
        .unwrap_or(false)
}

pub fn collect_files(input_path: &Path, output_dir: &Path) -> Vec<PathBuf> {
    let output_dir_cmp = output_dir_for_compare(output_dir);

    let mut files: Vec<PathBuf> = if input_path.is_file() {
        if is_supported_file(input_path) {
            vec![input_path.to_path_buf()]
        } else {
            Vec::new()
        }
    } else {
        WalkDir::new(input_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !e.path().starts_with(&output_dir_cmp))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| is_supported_file(e.path()))
            .map(|e| e.into_path())
            .collect()
    };

    files.sort();
    files
}
