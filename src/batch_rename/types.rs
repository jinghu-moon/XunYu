// batch_rename/types.rs

use std::path::PathBuf;

/// One rename operation: original path → new path.
pub(crate) struct RenameOp {
    pub from: PathBuf,
    pub to: PathBuf,
}

/// Naming convention styles.
#[derive(Clone, Debug)]
pub(crate) enum CaseStyle {
    Kebab,
    Snake,
    Pascal,
    Upper,
    Lower,
}
