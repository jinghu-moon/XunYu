use ratatui::layout::Rect;

pub(super) fn calc_scroll(cursor: usize, height: usize, total: usize) -> usize {
    if total <= height || height == 0 {
        return 0;
    }
    let margin = 3usize;
    if cursor < margin {
        return 0;
    }
    if cursor + margin >= total {
        return total.saturating_sub(height);
    }

    cursor.saturating_sub(height.saturating_sub(margin + 1))
}

pub(super) fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(r.width), height.min(r.height))
}

pub(super) fn fmt_size(bytes: u64) -> String {
    match bytes {
        0 => "0 B".into(),
        1..=1023 => format!("{} B", bytes),
        1024..=1048575 => format!("{:.1} KB", bytes as f64 / 1024.0),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
    }
}
