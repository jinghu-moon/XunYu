// batch_rename/compute.rs
//
// Pure computation for 6 rename modes.

use std::path::{Path, PathBuf};

use heck::{ToKebabCase, ToPascalCase, ToSnakeCase};
use regex::Regex;

use crate::batch_rename::types::{CaseStyle, RenameOp};
use crate::output::{CliError, CliResult};

/// Which rename mode is active.
pub(crate) enum RenameMode {
    Regex { pattern: String, replace: String },
    Case(CaseStyle),
    Prefix(String),
    Suffix(String),
    StripPrefix(String),
    Seq { start: usize, pad: usize },
}

/// Dispatch to the correct rename mode.
pub(crate) fn compute_ops(files: &[PathBuf], mode: &RenameMode) -> CliResult<Vec<RenameOp>> {
    match mode {
        RenameMode::Regex { pattern, replace } => mode_regex(files, pattern, replace),
        RenameMode::Case(style) => mode_case(files, style),
        RenameMode::Prefix(p) => mode_prefix(files, p),
        RenameMode::Suffix(s) => mode_suffix(files, s),
        RenameMode::StripPrefix(s) => mode_strip_prefix(files, s),
        RenameMode::Seq { start, pad } => mode_seq(files, *start, *pad),
    }
}

// ─── Rename modes ────────────────────────────────────────────────────────────

fn mode_regex(files: &[PathBuf], pattern: &str, replacement: &str) -> CliResult<Vec<RenameOp>> {
    let re = Regex::new(pattern)
        .map_err(|e| CliError::new(1, format!("Invalid regex '{}': {}", pattern, e)))?;
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let new_stem = re.replace_all(&stem, replacement).into_owned();
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

fn mode_case(files: &[PathBuf], style: &CaseStyle) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let new_stem = match style {
                CaseStyle::Kebab => stem.to_kebab_case(),
                CaseStyle::Snake => stem.to_snake_case(),
                CaseStyle::Pascal => stem.to_pascal_case(),
                CaseStyle::Upper => stem.to_uppercase(),
                CaseStyle::Lower => stem.to_lowercase(),
            };
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

fn mode_prefix(files: &[PathBuf], prefix: &str) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}{}", prefix, stem, ext)),
            }
        })
        .collect())
}

fn mode_suffix(files: &[PathBuf], suffix: &str) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}{}", stem, suffix, ext)),
            }
        })
        .collect())
}

fn mode_strip_prefix(files: &[PathBuf], strip: &str) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .map(|f| {
            let (stem, ext) = split_stem_ext(f);
            let new_stem = stem.strip_prefix(strip).unwrap_or(&stem).to_string();
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}{}", new_stem, ext)),
            }
        })
        .collect())
}

fn mode_seq(files: &[PathBuf], start: usize, pad: usize) -> CliResult<Vec<RenameOp>> {
    Ok(files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let (stem, ext) = split_stem_ext(f);
            let n = start + i;
            RenameOp {
                from: f.clone(),
                to: sibling(f, &format!("{}_{:0>w$}{}", stem, n, ext, w = pad)),
            }
        })
        .collect())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub(crate) fn split_stem_ext(path: &Path) -> (String, String) {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_owned();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();
    (stem, ext)
}

fn sibling(original: &Path, new_name: &str) -> PathBuf {
    original
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(new_name)
}
