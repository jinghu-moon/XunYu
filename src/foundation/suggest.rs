#[allow(dead_code)]
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];

    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur[j + 1] = prev[j + 1].min(cur[j] + 1).min(prev[j] + cost);
        }
        prev.clone_from_slice(&cur);
    }
    prev[b.len()]
}

#[allow(dead_code)]
pub(crate) fn did_you_mean<'a>(input: &str, candidates: &'a [&str]) -> Option<&'a str> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    let mut best: Option<(&str, usize)> = None;
    for &c in candidates {
        let d = levenshtein(&input.to_ascii_lowercase(), &c.to_ascii_lowercase());
        best = match best {
            None => Some((c, d)),
            Some((_, bd)) if d < bd => Some((c, d)),
            Some(v) => Some(v),
        };
    }
    match best {
        Some((c, d)) if d <= 2 => Some(c),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_basic() {
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("abc", "abd"), 1);
        assert_eq!(levenshtein("", "abc"), 3);
    }

    #[test]
    fn did_you_mean_finds_close_match() {
        assert_eq!(did_you_mean("skiip", &["skip", "overwrite"]), Some("skip"));
        assert_eq!(did_you_mean("xyz", &["skip", "overwrite"]), None);
        assert_eq!(did_you_mean("", &["skip"]), None);
    }
}
