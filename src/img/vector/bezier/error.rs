use super::geom::{Bezier, Pt, dist};
use super::sample::{chord_param, eval};

pub(super) fn max_error(b: &Bezier, pts: &[Pt]) -> (f64, usize) {
    let t = chord_param(pts);
    let (mut max_e, mut max_i) = (0.0, 0);
    for i in 1..pts.len() - 1 {
        let q = eval(b, t[i]);
        let e = dist(&pts[i], &q);
        if e > max_e {
            max_e = e;
            max_i = i;
        }
    }
    (max_e, max_i)
}

pub(super) fn is_linear_points(pts: &[Pt], tolerance: f64) -> bool {
    let p0 = &pts[0];
    let pn = &pts[pts.len() - 1];
    let dx = pn.x - p0.x;
    let dy = pn.y - p0.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return true;
    }
    let thresh = (tolerance * 0.5).max(len * 0.01);
    pts[1..pts.len() - 1]
        .iter()
        .all(|p| ((p.y - p0.y) * dx - (p.x - p0.x) * dy).abs() / len <= thresh)
}
