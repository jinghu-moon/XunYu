#[derive(Clone, Debug)]
pub(super) struct Pt {
    pub(super) x: f64,
    pub(super) y: f64,
}

impl Pt {
    pub(super) fn new(x: f64, y: f64) -> Self {
        Pt { x, y }
    }
    pub(super) fn sub(&self, o: &Pt) -> Pt {
        Pt::new(self.x - o.x, self.y - o.y)
    }
    pub(super) fn add(&self, o: &Pt) -> Pt {
        Pt::new(self.x + o.x, self.y + o.y)
    }
    pub(super) fn scale(&self, s: f64) -> Pt {
        Pt::new(self.x * s, self.y * s)
    }
    pub(super) fn dot(&self, o: &Pt) -> f64 {
        self.x * o.x + self.y * o.y
    }
    pub(super) fn len(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    pub(super) fn dist(&self, o: &Pt) -> f64 {
        self.sub(o).len()
    }
    pub(super) fn maxdist(&self, o: &Pt) -> f64 {
        (self.x - o.x).abs().max((self.y - o.y).abs())
    }
}

#[derive(Clone, Debug)]
pub(super) enum Seg {
    Curve(Pt, Pt, Pt), // c1, c2, end
    Corner(Pt, Pt),    // vertex, end
}

impl Seg {
    pub(super) fn end(&self) -> &Pt {
        match self {
            Seg::Curve(_, _, e) | Seg::Corner(_, e) => e,
        }
    }
}

pub(super) fn trace_path(bm: &[bool], w: usize, h: usize, sx: usize, sy: usize) -> Vec<Pt> {
    let pix = |x: i32, y: i32| -> bool {
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && bm[y as usize * w + x as usize]
    };
    let mut cx = sx as i32;
    let mut cy = sy as i32;
    let mut dir: u8 = if pix(cx - 1, cy) { 1 } else { 3 }; // 1=下, 3=上
    let (start_x, start_y, start_dir) = (cx, cy, dir);
    let mut pts = Vec::new();

    loop {
        // 记录当前角点
        let (dx, dy) = dd(dir);
        let pt = match dir {
            0 => Pt::new((cx + 1) as f64, cy as f64),
            1 => Pt::new((cx + 1) as f64, (cy + 1) as f64),
            2 => Pt::new(cx as f64, (cy + 1) as f64),
            _ => Pt::new(cx as f64, cy as f64),
        };
        pts.push(pt);

        // minority turn policy (论文§2.1.2)
        // 左侧像素: 相对于前进方向的左边
        let right_black = pix(cx + dx - dy, cy + dy + dx);
        let left_black = pix(cx - dy, cy + dx);
        let next_dir = if right_black {
            (dir + 1) % 4
        } else if !left_black {
            (dir + 3) % 4
        } else {
            dir
        };
        let (ndx, ndy) = dd(next_dir);
        cx += ndx;
        cy += ndy;
        dir = next_dir;

        if cx == start_x && cy == start_y && dir == start_dir {
            break;
        }
        if pts.len() > (w + h) * 8 {
            break;
        }
    }
    pts
}

fn dd(dir: u8) -> (i32, i32) {
    match dir {
        0 => (1, 0),
        1 => (0, 1),
        2 => (-1, 0),
        _ => (0, -1),
    }
}

pub(super) fn find_first_black(bm: &[bool], w: usize, h: usize) -> Option<(usize, usize)> {
    for y in 0..h {
        for x in 0..w {
            if bm[y * w + x] {
                return Some((x, y));
            }
        }
    }
    None
}

pub(super) fn shoelace(pts: &[Pt]) -> f64 {
    let mut a = 0.0;
    let n = pts.len();
    for i in 0..n {
        let j = (i + 1) % n;
        a += pts[i].x * pts[j].y - pts[j].x * pts[i].y;
    }
    a.abs() / 2.0
}

pub(super) fn erase_path(bm: &mut [bool], w: usize, h: usize, pts: &[Pt]) {
    for y in 0..h {
        let fy = y as f64 + 0.5;
        let mut inside = false;
        for x in 0..w {
            let fx = x as f64 + 0.5;
            for i in 0..pts.len() {
                let j = (i + 1) % pts.len();
                let p1 = &pts[i];
                let p2 = &pts[j];
                if (p1.y <= fy && p2.y > fy) || (p2.y <= fy && p1.y > fy) {
                    let t = (fy - p1.y) / (p2.y - p1.y);
                    if p1.x + t * (p2.x - p1.x) > fx {
                        inside = !inside;
                    }
                }
            }
            if inside {
                bm[y * w + x] = false;
            }
        }
    }
}

pub(super) fn otsu(gray: &[u8]) -> u8 {
    let mut hist = [0u32; 256];
    for &v in gray {
        hist[v as usize] += 1;
    }
    let total = gray.len() as f64;
    let sum_t: f64 = (0..256).map(|i| i as f64 * hist[i] as f64).sum();
    let (mut wb, mut sum_b, mut best, mut thresh) = (0.0f64, 0.0f64, 0.0f64, 128u8);
    for (t, count) in hist.iter().enumerate() {
        wb += *count as f64;
        if wb == 0.0 {
            continue;
        }
        let wf = total - wb;
        if wf == 0.0 {
            break;
        }
        sum_b += t as f64 * *count as f64;
        let between = wb * wf * ((sum_b / wb) - (sum_t - sum_b) / wf).powi(2);
        if between > best {
            best = between;
            thresh = t as u8;
        }
    }
    thresh
}

pub(super) fn f(v: f64) -> String {
    if (v - v.round()).abs() < 1e-4 {
        format!("{}", v.round() as i64)
    } else {
        format!("{:.2}", v)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}
