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
        let v = wb * wf * ((sum_b / wb) - (sum_t - sum_b) / wf).powi(2);
        if v > best {
            best = v;
            thresh = t as u8;
        }
    }
    thresh
}
