use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy)]
pub(crate) enum SizeOp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

type ParsedExpr = Result<(SizeOp, u64), String>;
type ExprCache = OnceLock<Mutex<HashMap<String, ParsedExpr>>>;

static SIZE_EXPR_CACHE: ExprCache = OnceLock::new();
static AGE_EXPR_CACHE: ExprCache = OnceLock::new();

fn parse_size_expr_uncached(s: &str) -> Result<(SizeOp, u64), String> {
    let (op, rest) = if let Some(r) = s.strip_prefix(">=") {
        (SizeOp::Ge, r)
    } else if let Some(r) = s.strip_prefix("<=") {
        (SizeOp::Le, r)
    } else if let Some(r) = s.strip_prefix('>') {
        (SizeOp::Gt, r)
    } else if let Some(r) = s.strip_prefix('<') {
        (SizeOp::Lt, r)
    } else if let Some(r) = s.strip_prefix('=') {
        (SizeOp::Eq, r)
    } else {
        return Err("need one of >,>=,<,<=,=".to_string());
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return Err("missing size value".to_string());
    }

    let mut num = String::new();
    let mut unit = String::new();
    for c in rest.chars() {
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

    let value: f64 = num.parse().map_err(|_| "invalid number".to_string())?;
    if !value.is_finite() || value < 0.0 {
        return Err("invalid number".to_string());
    }

    let unit = unit.to_ascii_lowercase();
    let mul: f64 = match unit.as_str() {
        "" | "b" => 1.0,
        "kb" | "k" => 1024.0,
        "mb" | "m" => 1024.0 * 1024.0,
        "gb" | "g" => 1024.0 * 1024.0 * 1024.0,
        _ => return Err(format!("unknown unit: {unit}")),
    };

    let bytes = (value * mul).round();
    if bytes > (u64::MAX as f64) {
        return Err("size too large".to_string());
    }
    Ok((op, bytes as u64))
}

pub(super) fn parse_size_expr(raw: &str) -> Result<(SizeOp, u64), String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty".to_string());
    }

    let cache = SIZE_EXPR_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    {
        let guard = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cached) = guard.get(s) {
            return cached.clone();
        }
    }

    let parsed = parse_size_expr_uncached(s);
    let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(s.to_string(), parsed.clone());
    parsed
}

fn parse_age_expr_uncached(s: &str) -> Result<(SizeOp, u64), String> {
    let (op, rest) = if let Some(r) = s.strip_prefix(">=") {
        (SizeOp::Ge, r)
    } else if let Some(r) = s.strip_prefix("<=") {
        (SizeOp::Le, r)
    } else if let Some(r) = s.strip_prefix('>') {
        (SizeOp::Gt, r)
    } else if let Some(r) = s.strip_prefix('<') {
        (SizeOp::Lt, r)
    } else if let Some(r) = s.strip_prefix('=') {
        (SizeOp::Eq, r)
    } else {
        return Err("need one of >,>=,<,<=,=".to_string());
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return Err("missing age value".to_string());
    }

    let mut num = String::new();
    let mut unit = String::new();
    for c in rest.chars() {
        if c.is_ascii_digit() || c == '.' {
            if unit.is_empty() {
                num.push(c);
            } else {
                return Err("invalid age format".to_string());
            }
        } else if !c.is_whitespace() {
            unit.push(c);
        }
    }
    let value: f64 = num.parse().map_err(|_| "invalid number".to_string())?;
    if !value.is_finite() || value < 0.0 {
        return Err("invalid number".to_string());
    }

    let unit = unit.to_ascii_lowercase();
    let mul: f64 = match unit.as_str() {
        "s" | "sec" | "secs" => 1.0,
        "m" | "min" | "mins" => 60.0,
        "h" | "hr" | "hrs" => 3600.0,
        "d" | "day" | "days" => 86400.0,
        "w" | "week" | "weeks" => 7.0 * 86400.0,
        _ => return Err(format!("unknown unit: {unit}")),
    };

    let secs = (value * mul).round();
    if secs > (u64::MAX as f64) {
        return Err("age too large".to_string());
    }
    Ok((op, secs as u64))
}

pub(super) fn parse_age_expr(raw: &str) -> Result<(SizeOp, u64), String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty".to_string());
    }

    let cache = AGE_EXPR_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    {
        let guard = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cached) = guard.get(s) {
            return cached.clone();
        }
    }

    let parsed = parse_age_expr_uncached(s);
    let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(s.to_string(), parsed.clone());
    parsed
}

pub(super) fn size_matches(size_bytes: u64, op: SizeOp, rhs: u64) -> bool {
    match op {
        SizeOp::Lt => size_bytes < rhs,
        SizeOp::Le => size_bytes <= rhs,
        SizeOp::Gt => size_bytes > rhs,
        SizeOp::Ge => size_bytes >= rhs,
        SizeOp::Eq => size_bytes == rhs,
    }
}

pub(super) fn age_secs(modified: SystemTime) -> Option<u64> {
    let now = SystemTime::now();
    let now_s = now.duration_since(UNIX_EPOCH).ok()?.as_secs();
    let m_s = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(now_s.saturating_sub(m_s))
}
