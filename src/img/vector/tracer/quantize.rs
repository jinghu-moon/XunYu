type Rgb = (u8, u8, u8);

pub(super) fn median_cut(pixels: &[(u8, u8, u8, u8)], num_colors: usize) -> Vec<Rgb> {
    if num_colors == 0 {
        return vec![];
    }
    let step = (pixels.len() / 50_000).max(1);
    let colors: Vec<Rgb> = pixels
        .iter()
        .step_by(step)
        .map(|&(r, g, b, _)| (r, g, b))
        .collect();
    if colors.is_empty() {
        return vec![(0, 0, 0)];
    }

    let mut boxes: Vec<Vec<Rgb>> = vec![colors];
    while boxes.len() < num_colors {
        let best = boxes
            .iter()
            .enumerate()
            .max_by_key(|(_, b)| box_range(b))
            .map(|(i, _)| i);
        let Some(bi) = best else { break };
        if boxes[bi].len() < 2 {
            break;
        }
        let to_split = boxes.remove(bi);
        let (a, b) = split(to_split);
        if !a.is_empty() {
            boxes.push(a);
        }
        if !b.is_empty() {
            boxes.push(b);
        }
    }
    boxes.iter().map(|b| box_avg(b)).collect()
}

fn box_range(c: &[Rgb]) -> u16 {
    let (mut rn, mut rx, mut gn, mut gx, mut bn, mut bx) = (255, 0u8, 255, 0u8, 255, 0u8);
    for &(r, g, b) in c {
        rn = rn.min(r);
        rx = rx.max(r);
        gn = gn.min(g);
        gx = gx.max(g);
        bn = bn.min(b);
        bx = bx.max(b);
    }
    ((rx - rn) as u16)
        .max((gx - gn) as u16)
        .max((bx - bn) as u16)
}

fn split(mut c: Vec<Rgb>) -> (Vec<Rgb>, Vec<Rgb>) {
    let (mut rn, mut rx, mut gn, mut gx, mut bn, mut bx) = (255, 0u8, 255, 0u8, 255, 0u8);
    for &(r, g, b) in &c {
        rn = rn.min(r);
        rx = rx.max(r);
        gn = gn.min(g);
        gx = gx.max(g);
        bn = bn.min(b);
        bx = bx.max(b);
    }
    let rr = (rx - rn) as u16;
    let gr = (gx - gn) as u16;
    let br = (bx - bn) as u16;
    if rr >= gr && rr >= br {
        c.sort_by_key(|c| c.0);
    } else if gr >= br {
        c.sort_by_key(|c| c.1);
    } else {
        c.sort_by_key(|c| c.2);
    }
    let mid = c.len() / 2;
    let r = c.split_off(mid);
    (c, r)
}

fn box_avg(c: &[Rgb]) -> Rgb {
    if c.is_empty() {
        return (0, 0, 0);
    }
    let (sr, sg, sb) = c
        .iter()
        .fold((0u64, 0u64, 0u64), |(a, b, cc), &(r, g, bl)| {
            (a + r as u64, b + g as u64, cc + bl as u64)
        });
    let n = c.len() as u64;
    ((sr / n) as u8, (sg / n) as u8, (sb / n) as u8)
}

pub(super) fn quantize(
    pixels: &[(u8, u8, u8, u8)],
    palette: &[(u8, u8, u8)],
) -> Vec<(u8, u8, u8, u8)> {
    pixels
        .iter()
        .map(|&(r, g, b, a)| {
            let (pr, pg, pb) = palette
                .iter()
                .min_by_key(|&&(pr, pg, pb)| {
                    let dr = pr as i32 - r as i32;
                    let dg = pg as i32 - g as i32;
                    let db = pb as i32 - b as i32;
                    dr * dr + dg * dg + db * db
                })
                .copied()
                .unwrap_or((r, g, b));
            (pr, pg, pb, a)
        })
        .collect()
}
