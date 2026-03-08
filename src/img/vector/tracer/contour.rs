use std::collections::HashMap;

use super::Pt;

pub(super) fn marching_squares(mask: &[bool], width: usize, height: usize) -> Vec<Vec<Pt>> {
    let gw = width + 2;
    let gh = height + 2;
    let inside = |gx: usize, gy: usize| -> bool {
        gx > 0 && gy > 0 && gx <= width && gy <= height && mask[(gy - 1) * width + (gx - 1)]
    };
    let cell_case = |cx: usize, cy: usize| -> u8 {
        (inside(cx, cy) as u8) << 3
            | (inside(cx + 1, cy) as u8) << 2
            | (inside(cx + 1, cy + 1) as u8) << 1
            | (inside(cx, cy + 1) as u8)
    };
    let fw = width as f64;
    let fh = height as f64;
    let edge_pt = move |cx: usize, cy: usize, side: u8| -> Pt {
        let (x, y) = match side {
            0 => (cx as f64 + 0.5, cy as f64),
            1 => ((cx + 1) as f64, cy as f64 + 0.5),
            2 => (cx as f64 + 0.5, (cy + 1) as f64),
            3 => (cx as f64, cy as f64 + 0.5),
            _ => unreachable!(),
        };
        Pt {
            x: (x - 0.5).clamp(0.0, fw),
            y: (y - 0.5).clamp(0.0, fh),
        }
    };
    let case_edges = |case: u8| -> Vec<(u8, u8)> {
        match case {
            0 | 15 => vec![],
            1 => vec![(2, 3)],
            2 => vec![(1, 2)],
            3 => vec![(1, 3)],
            4 => vec![(0, 1)],
            5 => vec![(0, 1), (2, 3)],
            6 => vec![(0, 2)],
            7 => vec![(0, 3)],
            8 => vec![(3, 0)],
            9 => vec![(2, 0)],
            10 => vec![(3, 0), (1, 2)],
            11 => vec![(1, 0)],
            12 => vec![(3, 1)],
            13 => vec![(2, 1)],
            14 => vec![(3, 2)],
            _ => vec![],
        }
    };
    let opp = |s: u8| -> u8 {
        match s {
            0 => 2,
            1 => 3,
            2 => 0,
            3 => 1,
            _ => unreachable!(),
        }
    };
    let neighbor = move |cx: usize, cy: usize, side: u8| -> Option<(usize, usize)> {
        match side {
            0 if cy > 0 => Some((cx, cy - 1)),
            1 if cx + 1 < gw => Some((cx + 1, cy)),
            2 if cy + 1 < gh => Some((cx, cy + 1)),
            3 if cx > 0 => Some((cx - 1, cy)),
            _ => None,
        }
    };

    let mut visited: HashMap<(usize, usize, u8), bool> = HashMap::new();
    let mut contours = Vec::new();

    for cy in 0..gh {
        for cx in 0..gw {
            let case = cell_case(cx, cy);
            for &(entry, exit) in &case_edges(case) {
                if visited.contains_key(&(cx, cy, entry)) {
                    continue;
                }
                let mut contour = Vec::new();
                let (mut ccx, mut ccy, mut cen, mut cex) = (cx, cy, entry, exit);
                let start = (cx, cy, entry);
                loop {
                    visited.insert((ccx, ccy, cen), true);
                    visited.insert((ccx, ccy, cex), true);
                    contour.push(edge_pt(ccx, ccy, cex));
                    let next_entry = opp(cex);
                    if let Some((nx, ny)) = neighbor(ccx, ccy, cex) {
                        let nc = cell_case(nx, ny);
                        if let Some(&(ne, nx_exit)) =
                            case_edges(nc).iter().find(|&&(e, _)| e == next_entry)
                        {
                            if (nx, ny, ne) == start {
                                break;
                            }
                            ccx = nx;
                            ccy = ny;
                            cen = ne;
                            cex = nx_exit;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if contour.len() >= 3 {
                    contours.push(contour);
                }
            }
        }
    }
    contours
}
