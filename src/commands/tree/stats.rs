use std::path::Path;

pub(super) fn print_stats(root: &Path, lines: usize, elapsed: std::time::Duration, depth: usize) {
    ui_println!("  - Path: {}", root.display());
    ui_println!("  - Lines: {lines}");
    ui_println!("  - Elapsed: {} ms", elapsed.as_millis());
    if depth > 0 {
        ui_println!("  - Max depth: {depth}");
    }
}
