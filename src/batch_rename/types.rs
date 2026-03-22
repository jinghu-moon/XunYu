// batch_rename/types.rs

use serde::Serialize;
use std::path::PathBuf;

/// One rename operation: original path → new path.
#[derive(Clone, Serialize)]
pub struct RenameOp {
    pub from: PathBuf,
    pub to: PathBuf,
}

/// Naming convention styles.
#[derive(Clone, Debug)]
pub enum CaseStyle {
    Kebab,
    Snake,
    Pascal,
    Upper,
    Lower,
    Title,
}
