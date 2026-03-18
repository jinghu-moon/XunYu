pub fn fuzzy_score(name: &str, command: &str, desc: &str, keyword: &str) -> i32 {
    let kw = keyword.trim().to_ascii_lowercase();
    if kw.is_empty() {
        return 0;
    }
    let name_l = name.to_ascii_lowercase();
    let cmd_l = command.to_ascii_lowercase();
    let desc_l = desc.to_ascii_lowercase();

    let mut score = 0i32;
    if name_l == kw {
        score += 180;
    } else if name_l.starts_with(&kw) {
        score += 130;
    } else if name_l.contains(&kw) {
        score += 100;
    }
    if cmd_l.contains(&kw) {
        score += 50;
    }
    if !desc_l.is_empty() && desc_l.contains(&kw) {
        score += 10;
    }
    score
}

pub fn parse_selection(input: &str, max: usize) -> Vec<usize> {
    let trimmed = input.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed == "a" || trimmed == "all" {
        return (0..max).collect();
    }

    let mut indices = Vec::new();
    for part in trimmed.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((l, r)) = part.split_once('-') {
            if let (Ok(a), Ok(b)) = (l.trim().parse::<usize>(), r.trim().parse::<usize>()) {
                if a == 0 || b == 0 {
                    continue;
                }
                let (s, e) = if a <= b {
                    (a - 1, b - 1)
                } else {
                    (b - 1, a - 1)
                };
                for i in s..=e {
                    if i < max {
                        indices.push(i);
                    }
                }
            }
            continue;
        }
        if let Ok(v) = part.parse::<usize>()
            && (1..=max).contains(&v)
        {
            indices.push(v - 1);
        }
    }
    indices.sort_unstable();
    indices.dedup();
    indices
}
