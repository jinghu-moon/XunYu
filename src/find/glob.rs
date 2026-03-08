fn char_equals(a: u8, b: u8, case_sensitive: bool) -> bool {
    if case_sensitive {
        return a == b;
    }
    a.to_ascii_lowercase() == b.to_ascii_lowercase()
}

fn is_char_in_range(c: u8, start: u8, end: u8, case_sensitive: bool) -> bool {
    if case_sensitive {
        return c >= start && c <= end;
    }
    let c = c.to_ascii_lowercase();
    let start = start.to_ascii_lowercase();
    let end = end.to_ascii_lowercase();
    c >= start && c <= end
}

pub(crate) fn glob_match_component(text: &str, pattern: &str, case_sensitive: bool) -> bool {
    let text = text.as_bytes();
    let pattern = pattern.as_bytes();
    let mut text_it = 0usize;
    let mut pattern_it = 0usize;
    let mut text_star: Option<usize> = None;
    let mut pattern_star: Option<usize> = None;

    while text_it < text.len() {
        if pattern_it < pattern.len() {
            let pc = pattern[pattern_it];
            if pc == b'\\' && pattern_it + 1 < pattern.len() {
                if char_equals(pattern[pattern_it + 1], text[text_it], case_sensitive) {
                    pattern_it += 2;
                    text_it += 1;
                    continue;
                }
            } else if pc == b'?' {
                text_it += 1;
                pattern_it += 1;
                continue;
            } else if pc == b'*' {
                pattern_star = Some(pattern_it);
                text_star = Some(text_it);
                pattern_it += 1;
                continue;
            } else if pc == b'[' {
                let mut set_start = pattern_it + 1;
                let mut set_end = set_start;
                if set_end < pattern.len() && pattern[set_end] == b']' {
                    set_end += 1;
                }
                while set_end < pattern.len() {
                    if pattern[set_end] == b'\\' && set_end + 1 < pattern.len() {
                        set_end += 2;
                    } else if pattern[set_end] == b']' {
                        break;
                    } else {
                        set_end += 1;
                    }
                }
                if set_end >= pattern.len() || set_start == set_end {
                    if char_equals(pc, text[text_it], case_sensitive) {
                        text_it += 1;
                        pattern_it += 1;
                        continue;
                    }
                } else {
                    let is_negated = pattern[set_start] == b'!' || pattern[set_start] == b'^';
                    if is_negated {
                        set_start += 1;
                    }
                    let mut matched = false;
                    let mut p = set_start;
                    while p < set_end {
                        if pattern[p] == b'\\' && p + 1 < set_end {
                            if char_equals(pattern[p + 1], text[text_it], case_sensitive) {
                                matched = true;
                            }
                            p += 2;
                        } else if p + 2 < set_end && pattern[p + 1] == b'-' {
                            if is_char_in_range(
                                text[text_it],
                                pattern[p],
                                pattern[p + 2],
                                case_sensitive,
                            ) {
                                matched = true;
                            }
                            p += 3;
                        } else {
                            if char_equals(pattern[p], text[text_it], case_sensitive) {
                                matched = true;
                            }
                            p += 1;
                        }
                        if matched {
                            break;
                        }
                    }
                    if (is_negated && !matched) || (!is_negated && matched) {
                        text_it += 1;
                        pattern_it = set_end + 1;
                        continue;
                    }
                }
            } else if char_equals(pc, text[text_it], case_sensitive) {
                text_it += 1;
                pattern_it += 1;
                continue;
            }
        }
        if let (Some(ps), Some(ts)) = (pattern_star, text_star) {
            pattern_it = ps + 1;
            let next = ts + 1;
            text_star = Some(next);
            text_it = next;
            continue;
        }
        return false;
    }
    while pattern_it < pattern.len() && pattern[pattern_it] == b'*' {
        pattern_it += 1;
    }
    pattern_it == pattern.len()
}

fn match_path_parts_inner(
    path_parts: &[&str],
    pattern_parts: &[String],
    mut path_idx: usize,
    mut pattern_idx: usize,
    case_sensitive: bool,
) -> bool {
    while pattern_idx < pattern_parts.len() {
        let p_part = &pattern_parts[pattern_idx];
        if p_part == "**" {
            pattern_idx += 1;
            if pattern_idx == pattern_parts.len() {
                return true;
            }
            for i in path_idx..=path_parts.len() {
                if match_path_parts_inner(path_parts, pattern_parts, i, pattern_idx, case_sensitive)
                {
                    return true;
                }
            }
            return false;
        }
        if path_idx >= path_parts.len() {
            return false;
        }
        if !glob_match_component(path_parts[path_idx], p_part, case_sensitive) {
            return false;
        }
        path_idx += 1;
        pattern_idx += 1;
    }
    path_idx == path_parts.len()
}

pub(crate) fn match_path_parts(
    path_parts: &[&str],
    pattern_parts: &[String],
    case_sensitive: bool,
) -> bool {
    match_path_parts_inner(path_parts, pattern_parts, 0, 0, case_sensitive)
}

pub(crate) fn split_path_parts(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

pub(crate) fn split_pattern_parts(pattern: &str) -> Vec<String> {
    pattern
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

pub(crate) fn has_glob_wildcard(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            if i + 1 < bytes.len() {
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'*' || bytes[i] == b'?' || bytes[i] == b'[' {
            return true;
        }
        i += 1;
    }
    false
}

pub(crate) fn unescape_glob_literal(pattern: &str) -> String {
    let bytes = pattern.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            out.push(bytes[i + 1]);
            i += 2;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_component_basic() {
        assert!(glob_match_component("abc", "a?c", true));
        assert!(glob_match_component("abc", "a*c", true));
        assert!(!glob_match_component("abc", "a*d", true));
        assert!(glob_match_component("a*c", r"a\*c", true));
        assert!(glob_match_component("b", "[a-c]", true));
        assert!(!glob_match_component("d", "[a-c]", true));
        assert!(glob_match_component("d", "[!a-c]", true));
    }

    #[test]
    fn glob_component_case_insensitive() {
        assert!(glob_match_component("AbC", "a?c", false));
        assert!(glob_match_component("ABC", "a*", false));
    }

    #[test]
    fn match_path_parts_with_double_star() {
        let path = split_path_parts("a/b/c.txt");
        let pattern = vec!["a".to_string(), "**".to_string(), "c.txt".to_string()];
        assert!(match_path_parts(&path, &pattern, true));

        let pattern = vec!["a".to_string(), "**".to_string()];
        assert!(match_path_parts(&path, &pattern, true));
    }

    #[test]
    fn double_star_in_component_behaves_like_star() {
        let single = glob_match_component("abc", "*", true);
        let double = glob_match_component("abc", "**", true);
        assert_eq!(single, double);
    }

    #[test]
    fn wildcard_detection_respects_escape() {
        assert!(!has_glob_wildcard(r"a\*b"));
        assert!(has_glob_wildcard("a*b"));
        assert!(has_glob_wildcard("a?b"));
        assert!(has_glob_wildcard("a[b]"));
    }
}
