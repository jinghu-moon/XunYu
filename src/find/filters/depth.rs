use super::types::{DepthFilter, RangeBound};

pub(super) fn parse_depth_filter(expr: &str) -> Result<DepthFilter, String> {
    let mut sv = expr.trim();
    if sv.is_empty() {
        return Err("empty depth expression".to_string());
    }
    let parse_val = |s: &str| -> Result<i32, String> {
        let v: i32 = s
            .trim()
            .parse()
            .map_err(|_| "invalid depth value".to_string())?;
        if v < 0 {
            return Err("depth must be >= 0".to_string());
        }
        Ok(v)
    };

    let is_range = sv.starts_with('[')
        || sv.starts_with('(')
        || (sv.contains('-') && sv.find('-').unwrap_or(0) > 0);
    if is_range {
        let left_bound = if sv.starts_with('(') {
            RangeBound::Open
        } else {
            RangeBound::Closed
        };
        let right_bound = if sv.ends_with(')') {
            RangeBound::Open
        } else {
            RangeBound::Closed
        };
        if sv.starts_with('[') || sv.starts_with('(') {
            sv = &sv[1..];
        }
        if sv.ends_with(']') || sv.ends_with(')') {
            sv = &sv[..sv.len() - 1];
        }
        let dash_pos = sv
            .find('-')
            .ok_or_else(|| "invalid depth range".to_string())?;
        let n1 = parse_val(&sv[..dash_pos])?;
        let n2 = parse_val(&sv[dash_pos + 1..])?;
        if n1 > n2 {
            return Err("invalid depth range: start > end".to_string());
        }
        let min = if left_bound == RangeBound::Closed {
            n1
        } else {
            n1 + 1
        };
        let max = if right_bound == RangeBound::Closed {
            n2
        } else {
            n2 - 1
        };
        if min > max {
            return Err("depth range is empty".to_string());
        }
        return Ok(DepthFilter {
            min: Some(min),
            max: Some(max),
        });
    }

    let ops = [">=", "<=", ">", "<", "="];
    let mut op = "";
    for sym in ops {
        if sv.starts_with(sym) {
            op = sym;
            sv = &sv[sym.len()..];
            break;
        }
    }
    let depth = parse_val(sv)?;
    let (min, max) = match op {
        ">=" => (Some(depth), None),
        ">" => (Some(depth + 1), None),
        "<=" => (None, Some(depth)),
        "<" => (None, Some(depth - 1)),
        _ => (Some(depth), Some(depth)),
    };
    Ok(DepthFilter { min, max })
}
