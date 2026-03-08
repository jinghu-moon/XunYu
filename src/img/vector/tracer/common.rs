use super::Pt;

pub(super) fn polygon_area(pts: &[Pt]) -> f64 {
    if pts.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..pts.len() {
        let j = (i + 1) % pts.len();
        area += pts[i].x * pts[j].y - pts[j].x * pts[i].y;
    }
    area.abs() / 2.0
}

pub(super) fn hex(r: u8, g: u8, b: u8) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}
