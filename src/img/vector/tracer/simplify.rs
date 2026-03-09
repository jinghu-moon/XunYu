use super::Pt;

pub(super) fn gaussian_smooth(pts: &[Pt], level: u8) -> Vec<Pt> {
    if level == 0 || pts.len() < 3 {
        return pts.to_vec();
    }
    let mut cur = pts.to_vec();
    for _ in 0..(level as usize).min(3) {
        let n = cur.len();
        let mut next = Vec::with_capacity(n);
        for i in 0..n {
            let prev = &cur[(i + n - 1) % n];
            let c = &cur[i];
            let nxt = &cur[(i + 1) % n];
            next.push(Pt {
                x: 0.25 * prev.x + 0.5 * c.x + 0.25 * nxt.x,
                y: 0.25 * prev.y + 0.5 * c.y + 0.25 * nxt.y,
            });
        }
        cur = next;
    }
    cur
}

pub(super) fn rdp_simplify(pts: &[Pt], eps: f64) -> Vec<Pt> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    let (first, last) = (&pts[0], &pts[pts.len() - 1]);
    let (mut max_d, mut max_i) = (0.0f64, 0);
    for (i, point) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
        let d = pt_line_dist(point, first, last);
        if d > max_d {
            max_d = d;
            max_i = i;
        }
    }
    if max_d > eps {
        let mut l = rdp_simplify(&pts[..=max_i], eps);
        let r = rdp_simplify(&pts[max_i..], eps);
        l.pop();
        l.extend(r);
        l
    } else {
        vec![first.clone(), last.clone()]
    }
}

fn pt_line_dist(p: &Pt, a: &Pt, b: &Pt) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let ls = dx * dx + dy * dy;
    if ls < 1e-10 {
        return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
    }
    let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / ls;
    let t = t.clamp(0.0, 1.0);
    ((p.x - (a.x + t * dx)).powi(2) + (p.y - (a.y + t * dy)).powi(2)).sqrt()
}
