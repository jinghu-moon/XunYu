/// 2D 点
#[derive(Debug, Clone, PartialEq)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

/// 三次贝塞尔曲线段 (P0, P1, P2, P3)
#[derive(Debug, Clone)]
pub struct Bezier {
    pub p0: Pt, // start
    pub p1: Pt, // control 1
    pub p2: Pt, // control 2
    pub p3: Pt, // end
}

pub(super) fn dist(a: &Pt, b: &Pt) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

/// 将贝塞尔列表转为 SVG path `d` 属性字符串
/// 近直线段用 L，真曲线用 C，连续共线 L 合并
pub fn to_svg_path(curves: &[Bezier], closed: bool) -> String {
    if curves.is_empty() {
        return String::new();
    }
    let mut s = format!("M{},{}", fmt(curves[0].p0.x), fmt(curves[0].p0.y));
    let mut i = 0;
    while i < curves.len() {
        let c = &curves[i];
        if is_linear_curve(c) {
            // 合并连续共线 L 段
            let start = &curves[i].p0;
            let mut end = &c.p3;
            let mut j = i + 1;
            while j < curves.len() {
                let nc = &curves[j];
                if !is_linear_curve(nc) {
                    break;
                }
                let ce = &nc.p3;
                let dx = ce.x - start.x;
                let dy = ce.y - start.y;
                let ll = (dx * dx + dy * dy).sqrt();
                if ll < 0.5 {
                    end = ce;
                    j += 1;
                    continue;
                }
                let d = ((end.y - start.y) * dx - (end.x - start.x) * dy).abs() / ll;
                if d < 1.5 {
                    end = ce;
                    j += 1;
                } else {
                    break;
                }
            }
            s.push_str(&format!("L{},{}", fmt(end.x), fmt(end.y)));
            i = j;
        } else {
            s.push_str(&format!(
                "C{},{} {},{} {},{}",
                fmt(c.p1.x),
                fmt(c.p1.y),
                fmt(c.p2.x),
                fmt(c.p2.y),
                fmt(c.p3.x),
                fmt(c.p3.y)
            ));
            i += 1;
        }
    }
    if closed {
        s.push('Z');
    }
    s
}

fn is_linear_curve(b: &Bezier) -> bool {
    let dx = b.p3.x - b.p0.x;
    let dy = b.p3.y - b.p0.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.5 {
        return true;
    }
    let d1 = ((b.p1.y - b.p0.y) * dx - (b.p1.x - b.p0.x) * dy).abs() / len;
    let d2 = ((b.p2.y - b.p0.y) * dx - (b.p2.x - b.p0.x) * dy).abs() / len;
    d1 < 1.0 && d2 < 1.0
}

fn fmt(v: f64) -> String {
    if (v - v.round()).abs() < 1e-4 {
        format!("{}", v.round() as i64)
    } else {
        let s = format!("{:.2}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}
