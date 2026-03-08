use super::Pt;
use super::common::{DIRS8, nb};

// ── 骨架路径提取 ──────────────────────────────────────────────────────────
// 找端点（1邻居）和交叉点（≥3邻居）作为起点，沿连通骨架追踪
pub(super) fn extract_paths(bin: &[u8], w: usize, h: usize, min_len: usize) -> Vec<Vec<Pt>> {
    let mut visited = vec![false; w * h];
    let mut paths = Vec::new();

    // 收集所有骨架点，优先端点（1邻居）
    let mut endpoints = Vec::new();
    let mut regular = Vec::new();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let idx = y * w + x;
            if bin[idx] == 0 {
                continue;
            }
            let cnt = nb(bin, w, x, y).iter().map(|&v| v as usize).sum::<usize>();
            if cnt == 1 {
                endpoints.push(idx);
            } else if cnt >= 2 {
                regular.push(idx);
            }
        }
    }
    // 端点优先遍历
    let all: Vec<usize> = endpoints.into_iter().chain(regular).collect();

    for start_idx in all {
        if visited[start_idx] || bin[start_idx] == 0 {
            continue;
        }
        let sx = start_idx % w;
        let sy = start_idx / w;
        let path_pts = follow(bin, w, h, &mut visited, sx, sy);
        if path_pts.len() >= min_len {
            paths.push(path_pts);
        }
    }
    paths
}

/// 从 (sx,sy) 开始沿骨架追踪，返回路径坐标列表
fn follow(bin: &[u8], w: usize, h: usize, visited: &mut [bool], sx: usize, sy: usize) -> Vec<Pt> {
    let mut pts = Vec::new();
    // 用栈式 DFS，但优先选择"直行"方向保持路径连续
    let mut stack = vec![(sx, sy, 0usize, 0usize)]; // x, y, prev_x, prev_y

    while let Some((cx, cy, px, py)) = stack.pop() {
        if visited[cy * w + cx] {
            continue;
        }
        visited[cy * w + cx] = true;
        pts.push(Pt {
            x: cx as f64,
            y: cy as f64,
        });

        // 找8邻接中未访问的骨架像素
        let mut nexts: Vec<(usize, usize)> = DIRS8
            .iter()
            .filter_map(|&(dx, dy)| {
                let nx = cx as i32 + dx;
                let ny = cy as i32 + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    return None;
                }
                let (nx, ny) = (nx as usize, ny as usize);
                if bin[ny * w + nx] == 1 && !visited[ny * w + nx] {
                    Some((nx, ny))
                } else {
                    None
                }
            })
            .collect();

        if nexts.is_empty() {
            continue;
        }

        // 优先选择与前进方向相同的像素（减少折点）
        nexts.sort_by_key(|&(nx, ny)| {
            let ddx = cx as i32 - px as i32;
            let ddy = cy as i32 - py as i32;
            let tx = nx as i32 - cx as i32;
            let ty = ny as i32 - cy as i32;
            // 用负点积：点积越大（方向越一致）优先级越高
            -(ddx * tx + ddy * ty)
        });
        // 只追踪最优方向（避免分叉导致重复）
        let (nx, ny) = nexts[0];
        stack.push((nx, ny, cx, cy));
    }
    pts
}
