use super::types::{RangeBound, SizeCompare, SizeFilter};

pub(super) fn parse_size_filter(expr: &str, is_fuzzy: bool) -> Result<SizeFilter, String> {
    let s = expr.trim().to_string();
    if s.is_empty() {
        return Err("empty".to_string());
    }
    if is_fuzzy {
        if s.contains('>')
            || s.contains('<')
            || s.contains('=')
            || s.contains('[')
            || s.contains(']')
            || s.contains('(')
            || s.contains(')')
            || s.contains(',')
        {
            return Err("fuzzy size only accepts simple values or ranges".to_string());
        }
        if let Some(dash_pos) = s.find('-') {
            let (left, right) = s.split_at(dash_pos);
            let right = &right[1..];
            if left.trim().is_empty() || right.trim().is_empty() {
                return Err("invalid fuzzy range".to_string());
            }
            let left_bytes = parse_size_value(left)?;
            let left_mult = parse_size_multiplier(left);
            let min = if left_bytes < left_mult {
                1
            } else {
                left_bytes - left_mult + 1
            };
            let max = parse_size_value(right)?;
            if min > max {
                return Err("invalid fuzzy range: min > max".to_string());
            }
            return Ok(SizeFilter::Range {
                min,
                max,
                left: RangeBound::Closed,
                right: RangeBound::Closed,
            });
        }
        let bytes = parse_size_value(&s)?;
        let mult = parse_size_multiplier(&s);
        let min = if bytes < mult { 1 } else { bytes - mult + 1 };
        let max = bytes;
        return Ok(SizeFilter::Range {
            min,
            max,
            left: RangeBound::Closed,
            right: RangeBound::Closed,
        });
    }

    if (s.starts_with('[') || s.starts_with('(')) && (s.ends_with(']') || s.ends_with(')')) {
        let left = if s.starts_with('[') {
            RangeBound::Closed
        } else {
            RangeBound::Open
        };
        let right = if s.ends_with(']') {
            RangeBound::Closed
        } else {
            RangeBound::Open
        };
        let inner = &s[1..s.len() - 1];
        let (min, max) = parse_range_values(inner)?;
        return Ok(SizeFilter::Range {
            min,
            max,
            left,
            right,
        });
    }

    if let Some(dash_pos) = s.find('-') {
        if dash_pos > 0 {
            let prev = s.as_bytes()[dash_pos - 1] as char;
            if prev != '>' && prev != '<' && prev != '=' {
                let (min, max) = parse_range_values(&s)?;
                return Ok(SizeFilter::Range {
                    min,
                    max,
                    left: RangeBound::Closed,
                    right: RangeBound::Closed,
                });
            }
        }
    }

    let mut op = SizeCompare::Eq;
    let mut offset = 0usize;
    for (sym, cmp) in [
        (">=", SizeCompare::Ge),
        ("<=", SizeCompare::Le),
        (">", SizeCompare::Gt),
        ("<", SizeCompare::Lt),
        ("=", SizeCompare::Eq),
    ] {
        if s.starts_with(sym) {
            op = cmp;
            offset = sym.len();
            break;
        }
    }
    let value = parse_size_value(&s[offset..])?;
    Ok(SizeFilter::Compare { op, value })
}

fn parse_range_values(content: &str) -> Result<(u64, u64), String> {
    let dash_pos = content
        .find('-')
        .ok_or_else(|| "invalid range".to_string())?;
    if dash_pos == 0 || dash_pos + 1 >= content.len() {
        return Err("invalid range".to_string());
    }
    let min = parse_size_value(&content[..dash_pos])?;
    let max = parse_size_value(&content[dash_pos + 1..])?;
    if min > max {
        return Err("invalid range: min > max".to_string());
    }
    Ok((min, max))
}

fn parse_size_value(raw: &str) -> Result<u64, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty size value".to_string());
    }
    let mut num = String::new();
    let mut unit = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' {
            if unit.is_empty() {
                num.push(c);
            } else {
                return Err("invalid size format".to_string());
            }
        } else if !c.is_whitespace() {
            unit.push(c);
        }
    }
    if num.is_empty() {
        return Err("invalid size format".to_string());
    }
    let value: f64 = num.parse().map_err(|_| "invalid number".to_string())?;
    if !value.is_finite() || value < 0.0 {
        return Err("invalid number".to_string());
    }
    let unit = unit.to_ascii_uppercase();
    let mul = match unit.chars().next() {
        Some('B') | None => 1.0,
        Some('K') => 1024.0,
        Some('M') => 1024.0 * 1024.0,
        Some('G') => 1024.0 * 1024.0 * 1024.0,
        Some(c) => return Err(format!("unknown size unit: {c}")),
    };
    let bytes = (value * mul).round();
    if bytes > (u64::MAX as f64) {
        return Err("size too large".to_string());
    }
    Ok(bytes as u64)
}

fn parse_size_multiplier(raw: &str) -> u64 {
    let s = raw.trim();
    let mut idx = 0usize;
    for (i, c) in s.char_indices() {
        if c.is_ascii_digit() || c == '.' {
            idx = i + c.len_utf8();
            continue;
        }
        if c.is_whitespace() {
            idx = i + c.len_utf8();
            continue;
        }
        idx = i;
        break;
    }
    let unit = s[idx..].trim();
    match unit.chars().next().map(|c| c.to_ascii_uppercase()) {
        Some('K') => 1024,
        Some('M') => 1024 * 1024,
        Some('G') => 1024 * 1024 * 1024,
        _ => 1,
    }
}
