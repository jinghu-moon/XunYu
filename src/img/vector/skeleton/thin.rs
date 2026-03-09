use super::common::{nb, trans};

// ── Zhang-Suen 并行细化 ───────────────────────────────────────────────────
// 论文: 两步并行删除，迭代至收敛
pub(super) fn zhang_suen(bin: &mut [u8], w: usize, h: usize) {
    loop {
        let mut changed = false;

        // 子迭代 1：删除东南边界 + 西北角点
        let mut del: Vec<usize> = Vec::new();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let idx = y * w + x;
                if bin[idx] == 0 {
                    continue;
                }
                // 按论文图1顺序读取8邻域: P2(北),P3(东北),P4(东),P5(东南),P6(南),P7(西南),P8(西),P9(西北)
                let [p2, p3, p4, p5, p6, p7, p8, p9] = nb(bin, w, x, y);
                let b = p2 + p3 + p4 + p5 + p6 + p7 + p8 + p9;
                let a = trans([p2, p3, p4, p5, p6, p7, p8, p9, p2]);
                if (2..=6).contains(&b) && a == 1 && p2 * p4 * p6 == 0 && p4 * p6 * p8 == 0 {
                    del.push(idx);
                }
            }
        }
        for idx in &del {
            bin[*idx] = 0;
        }
        changed |= !del.is_empty();

        // 子迭代 2：删除西北边界 + 东南角点
        let mut del2: Vec<usize> = Vec::new();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let idx = y * w + x;
                if bin[idx] == 0 {
                    continue;
                }
                let [p2, p3, p4, p5, p6, p7, p8, p9] = nb(bin, w, x, y);
                let b = p2 + p3 + p4 + p5 + p6 + p7 + p8 + p9;
                let a = trans([p2, p3, p4, p5, p6, p7, p8, p9, p2]);
                if (2..=6).contains(&b) && a == 1 && p2 * p4 * p8 == 0 && p2 * p6 * p8 == 0 {
                    del2.push(idx);
                }
            }
        }
        for idx in &del2 {
            bin[*idx] = 0;
        }
        changed |= !del2.is_empty();

        if !changed {
            break;
        }
    }
}
