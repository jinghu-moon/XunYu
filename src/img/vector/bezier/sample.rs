use super::geom::{Bezier, Pt};

pub(super) fn chord_param(pts: &[Pt]) -> Vec<f64> {
    let mut t = vec![0.0; pts.len()];
    for i in 1..pts.len() {
        t[i] = t[i - 1] + super::geom::dist(&pts[i], &pts[i - 1]);
    }
    let total = t[pts.len() - 1];
    if total > 0.0 {
        for ti in t.iter_mut() {
            *ti /= total;
        }
    }
    *t.last_mut().unwrap() = 1.0;
    t
}

pub(super) fn eval(b: &Bezier, t: f64) -> Pt {
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    let t2 = t * t;
    let t3 = t2 * t;
    Pt {
        x: mt3 * b.p0.x + 3.0 * mt2 * t * b.p1.x + 3.0 * mt * t2 * b.p2.x + t3 * b.p3.x,
        y: mt3 * b.p0.y + 3.0 * mt2 * t * b.p1.y + 3.0 * mt * t2 * b.p2.y + t3 * b.p3.y,
    }
}

pub(super) fn eval_d1(b: &Bezier, t: f64) -> Pt {
    let mt = 1.0 - t;
    let ax = b.p1.x - b.p0.x;
    let ay = b.p1.y - b.p0.y;
    let bx = b.p2.x - b.p1.x;
    let by_ = b.p2.y - b.p1.y;
    let cx = b.p3.x - b.p2.x;
    let cy = b.p3.y - b.p2.y;
    Pt {
        x: 3.0 * (mt * mt * ax + 2.0 * mt * t * bx + t * t * cx),
        y: 3.0 * (mt * mt * ay + 2.0 * mt * t * by_ + t * t * cy),
    }
}

pub(super) fn eval_d2(b: &Bezier, t: f64) -> Pt {
    let mt = 1.0 - t;
    let ax = b.p2.x - 2.0 * b.p1.x + b.p0.x;
    let ay = b.p2.y - 2.0 * b.p1.y + b.p0.y;
    let bx = b.p3.x - 2.0 * b.p2.x + b.p1.x;
    let by_ = b.p3.y - 2.0 * b.p2.y + b.p1.y;
    Pt {
        x: 6.0 * (mt * ax + t * bx),
        y: 6.0 * (mt * ay + t * by_),
    }
}
