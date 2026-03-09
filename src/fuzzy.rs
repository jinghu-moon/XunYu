use std::collections::BTreeMap;

use crate::model::Entry;
use crate::store::now_secs;

pub(crate) const FRECENCY_WEIGHT: f64 = 0.15;

#[derive(Clone, Copy, Debug)]
pub(crate) struct CharMask {
    ascii: u128,
    has_non_ascii: bool,
}

#[derive(Clone)]
struct PairMask {
    map: [u128; 128],
}

struct FuzzyEntry {
    name: String,
    name_lower: String,
    mask: CharMask,
    pair_mask: PairMask,
    entry: Entry,
}

pub(crate) struct FuzzyIndex {
    entries: Vec<FuzzyEntry>,
    ascii_index: Vec<Vec<usize>>,
    pair_index: Vec<Vec<usize>>,
}

impl FuzzyIndex {
    pub(crate) fn from_db(db: &BTreeMap<String, Entry>) -> Self {
        let mut entries = Vec::with_capacity(db.len());
        let mut ascii_index: Vec<Vec<usize>> = vec![Vec::new(); 128];
        let mut pair_index: Vec<Vec<usize>> = vec![Vec::new(); 128 * 128];

        for (name, entry) in db {
            let name_lower = name.to_lowercase();
            let mask = build_char_mask(&name_lower);
            let pair_mask = build_pair_mask(&name_lower);
            let idx = entries.len();
            entries.push(FuzzyEntry {
                name: name.clone(),
                name_lower,
                mask,
                pair_mask: pair_mask.clone(),
                entry: entry.clone(),
            });

            let mut bits = mask.ascii;
            while bits != 0 {
                let bit = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                if bit < 128 {
                    ascii_index[bit].push(idx);
                }
            }

            for first in 0..128 {
                let mut pair_bits = pair_mask.map[first];
                while pair_bits != 0 {
                    let bit = pair_bits.trailing_zeros() as usize;
                    pair_bits &= pair_bits - 1;
                    let key = first * 128 + bit;
                    pair_index[key].push(idx);
                }
            }
        }

        Self {
            entries,
            ascii_index,
            pair_index,
        }
    }

    pub(crate) fn search(
        &self,
        pattern: &str,
        tag: Option<&str>,
        cwd: Option<&str>,
    ) -> Vec<(f64, String, Entry)> {
        let pattern_chars: Vec<char> = pattern.to_lowercase().chars().collect();
        let pattern_mask = build_pattern_mask(&pattern_chars);

        let mut best_list: Option<&Vec<usize>> = None;
        let mut pattern_pairs: Vec<(usize, usize)> = Vec::new();

        if pattern_chars.len() >= 2 && !pattern_mask.has_non_ascii {
            let mut best_len = usize::MAX;
            for w in pattern_chars.windows(2) {
                let a = w[0] as u32;
                let b = w[1] as u32;
                if a >= 128 || b >= 128 {
                    pattern_pairs.clear();
                    best_list = None;
                    break;
                }
                let key = (a as usize) * 128 + (b as usize);
                let list = &self.pair_index[key];
                if list.is_empty() {
                    return Vec::new();
                }
                if list.len() < best_len {
                    best_len = list.len();
                    best_list = Some(list);
                }
                pattern_pairs.push((a as usize, b as usize));
            }
        }

        if best_list.is_none() && pattern_mask.ascii != 0 {
            let mut bits = pattern_mask.ascii;
            let mut best_len = usize::MAX;
            while bits != 0 {
                let bit = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                let list = &self.ascii_index[bit];
                if list.is_empty() {
                    return Vec::new();
                }
                if list.len() < best_len {
                    best_len = list.len();
                    best_list = Some(list);
                }
            }
        }

        let mut scored = Vec::new();
        let iter: Box<dyn Iterator<Item = usize>> = if let Some(list) = best_list {
            Box::new(list.iter().copied())
        } else {
            Box::new(0..self.entries.len())
        };

        for idx in iter {
            let item = &self.entries[idx];
            if !matches_tag(&item.entry, tag) {
                continue;
            }
            if !pattern_pairs.is_empty() {
                let pairs = &item.pair_mask.map;
                let mut ok = true;
                for (a, b) in &pattern_pairs {
                    if (pairs[*a] & (1u128 << b)) == 0 {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    continue;
                }
            }
            if !mask_may_match(item.mask, pattern_mask) {
                continue;
            }
            if let Some(fs) = fuzzy_score(&pattern_chars, &item.name_lower) {
                let boost = cwd.map(|c| cwd_boost(c, &item.entry.path)).unwrap_or(1.0);
                let combined = fs * (1.0 + frecency(&item.entry) * FRECENCY_WEIGHT) * boost;
                scored.push((combined, item.name.clone(), item.entry.clone()));
            }
        }

        scored
    }
}

pub(crate) fn build_char_mask(text_lower: &str) -> CharMask {
    let mut mask = 0u128;
    let mut non_ascii = false;
    for c in text_lower.chars() {
        if c.is_ascii() {
            let bit = c as u32;
            if bit < 128 {
                mask |= 1u128 << bit;
            }
        } else {
            non_ascii = true;
        }
    }
    CharMask {
        ascii: mask,
        has_non_ascii: non_ascii,
    }
}

pub(crate) fn build_pattern_mask(pattern_chars: &[char]) -> CharMask {
    let mut mask = 0u128;
    let mut non_ascii = false;
    for &c in pattern_chars {
        if c.is_ascii() {
            let bit = c as u32;
            if bit < 128 {
                mask |= 1u128 << bit;
            }
        } else {
            non_ascii = true;
        }
    }
    CharMask {
        ascii: mask,
        has_non_ascii: non_ascii,
    }
}

fn build_pair_mask(text_lower: &str) -> PairMask {
    let mut map = [0u128; 128];
    let mut seen = 0u128;
    for c in text_lower.chars().rev() {
        if c.is_ascii() {
            let idx = c as u32;
            if idx < 128 {
                let i = idx as usize;
                map[i] |= seen;
                seen |= 1u128 << i;
            }
        }
    }
    PairMask { map }
}

pub(crate) fn mask_may_match(text_mask: CharMask, pattern_mask: CharMask) -> bool {
    if pattern_mask.has_non_ascii && pattern_mask.ascii == 0 {
        return true;
    }
    (text_mask.ascii & pattern_mask.ascii) == pattern_mask.ascii
}

/// Frecency score (zoxide-style 4-bucket time decay).
/// Score = visit_count × time_multiplier.
pub(crate) fn frecency(e: &Entry) -> f64 {
    let elapsed = now_secs().saturating_sub(e.last_visited);
    let mult = if elapsed < 3_600 {
        4.0
    } else if elapsed < 86_400 {
        2.0
    } else if elapsed < 604_800 {
        0.5
    } else {
        0.25
    };
    mult * e.visit_count.max(1) as f64
}

/// CWD context boost: paths related to current directory score higher.
pub(crate) fn cwd_boost(cwd: &str, bookmark_path: &str) -> f64 {
    let cwd_n = normalize_for_cmp(cwd);
    let bm_n = normalize_for_cmp(bookmark_path);
    if cwd_n == bm_n {
        2.0 // exact match: user is in this bookmark's directory
    } else if cwd_n.starts_with(&bm_n) && cwd_n.as_bytes().get(bm_n.len()) == Some(&b'\\') {
        1.5 // bookmark is parent of cwd
    } else if bm_n.starts_with(&cwd_n) && bm_n.as_bytes().get(cwd_n.len()) == Some(&b'\\') {
        1.3 // bookmark is child of cwd
    } else {
        1.0 // unrelated
    }
}

fn normalize_for_cmp(p: &str) -> String {
    p.trim_end_matches(['\\', '/'])
        .replace('/', "\\")
        .to_ascii_lowercase()
}

pub(crate) fn fuzzy_score(pattern_chars: &[char], text_lower: &str) -> Option<f64> {
    if pattern_chars.is_empty() {
        return Some(1.0);
    }

    let mut score = 0.0;
    let mut last_match = None;
    let mut pi = 0usize;
    for (i, c) in text_lower.chars().enumerate() {
        if c == pattern_chars[pi] {
            score += 1.0;
            if let Some(prev) = last_match
                && i == prev + 1
            {
                score += 1.0;
            }
            last_match = Some(i);
            pi += 1;
            if pi == pattern_chars.len() {
                break;
            }
        }
    }

    if pi == pattern_chars.len() {
        Some(score)
    } else {
        None
    }
}

pub(crate) fn matches_tag(e: &Entry, tag: Option<&str>) -> bool {
    match tag {
        Some(t) => !t.is_empty() && e.tags.iter().any(|et| et.eq_ignore_ascii_case(t)),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(visits: u32, last_visited: u64, tags: &[&str]) -> Entry {
        Entry {
            path: "C:\\tmp".to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            visit_count: visits,
            last_visited,
        }
    }

    #[test]
    fn fuzzy_score_empty_pattern_matches() {
        assert_eq!(fuzzy_score(&[], "anything"), Some(1.0));
    }

    #[test]
    fn fuzzy_score_exact_and_consecutive_get_bonus() {
        let pattern: Vec<char> = "ab".chars().collect();

        let exact = fuzzy_score(&pattern, "ab").expect("match");
        let non_consecutive = fuzzy_score(&pattern, "a__b").expect("match");

        assert!(exact > non_consecutive, "expected consecutive bonus");
        assert_eq!(exact, 3.0);
        assert_eq!(non_consecutive, 2.0);
    }

    #[test]
    fn fuzzy_score_no_match_is_none() {
        let pattern: Vec<char> = "z".chars().collect();
        assert_eq!(fuzzy_score(&pattern, "abc"), None);
    }

    #[test]
    fn fuzzy_score_case_insensitive_when_lowercased() {
        let pattern_chars: Vec<char> = "AB".to_lowercase().chars().collect();
        let text_lower = "aBcd".to_lowercase();
        assert!(fuzzy_score(&pattern_chars, &text_lower).is_some());
    }

    #[test]
    fn frecency_prefers_recent_and_counts_visits() {
        let now = now_secs();

        let recent = entry(10, now, &[]);
        let old = entry(10, now.saturating_sub(60 * 60 * 24 * 40), &[]);

        let recent_score = frecency(&recent);
        let old_score = frecency(&old);

        assert_eq!(recent_score, 40.0);
        assert_eq!(old_score, 2.5); // 10 × 0.25 (>1week bucket)
        assert!(recent_score > old_score);
    }

    #[test]
    fn frecency_treats_zero_visits_as_one() {
        let now = now_secs();
        let e = entry(0, now, &[]);
        assert_eq!(frecency(&e), 4.0);
    }

    #[test]
    fn matches_tag_is_case_insensitive_and_rejects_empty_tag() {
        let e = entry(0, 0, &["Work", "Personal"]);
        assert!(matches_tag(&e, None));
        assert!(!matches_tag(&e, Some("")));
        assert!(matches_tag(&e, Some("work")));
        assert!(!matches_tag(&e, Some("nope")));
    }
}
