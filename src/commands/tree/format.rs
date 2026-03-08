pub(super) fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{bytes}B")
    } else if b < MB {
        format!("{:.1}KB", b / KB)
    } else if b < GB {
        format!("{:.1}MB", b / MB)
    } else {
        format!("{:.2}GB", b / GB)
    }
}
