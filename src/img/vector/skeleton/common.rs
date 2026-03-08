/// 按论文图1定义返回8邻域 [P2,P3,P4,P5,P6,P7,P8,P9]
#[inline]
pub(super) fn nb(bin: &[u8], w: usize, x: usize, y: usize) -> [u8; 8] {
    [
        bin[(y - 1) * w + x],     // P2: 北
        bin[(y - 1) * w + x + 1], // P3: 东北
        bin[y * w + x + 1],       // P4: 东
        bin[(y + 1) * w + x + 1], // P5: 东南
        bin[(y + 1) * w + x],     // P6: 南
        bin[(y + 1) * w + x - 1], // P7: 西南
        bin[y * w + x - 1],       // P8: 西
        bin[(y - 1) * w + x - 1], // P9: 西北
    ]
}

/// A(P1): 顺时针序列中 0→1 跳变次数（论文定义）
#[inline]
pub(super) fn trans(n: [u8; 9]) -> u8 {
    // n[0]=P2, n[1]=P3, ..., n[7]=P9, n[8]=P2 (论文的循环)
    let mut c = 0u8;
    for i in 0..8 {
        if n[i] == 0 && n[i + 1] == 1 {
            c += 1;
        }
    }
    c
}

pub(super) const DIRS8: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];
