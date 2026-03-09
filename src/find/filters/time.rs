use std::time::{SystemTime, UNIX_EPOCH};

use crate::output::{CliError, CliResult};
use crate::store::now_secs;

use super::types::{TimeFilter, TimeType};

#[derive(Clone, Copy)]
struct TimePoint {
    prefix: Option<char>,
    is_relative: bool,
    value: i64,
    unit: char,
    absolute_secs: i64,
}

pub(super) fn parse_time_filter(raw: &str, kind: TimeType) -> CliResult<TimeFilter> {
    let expr = raw.trim();
    if expr.is_empty() {
        return Err(CliError::new(2, "Empty time filter."));
    }
    let now = now_secs() as i64;
    let unit_secs = |unit: char| -> Result<i64, CliError> {
        Ok(match unit {
            's' => 1,
            'm' => 60,
            'h' => 3600,
            'd' => 86_400,
            'w' => 7 * 86_400,
            _ => return Err(CliError::new(2, format!("Invalid time unit: {unit}"))),
        })
    };

    let (start, end) = if starts_with_prefix(expr) {
        let p = parse_time_point(expr)?;
        resolve_single_time(p, now, unit_secs)?
    } else if let Some((left, right)) = split_time_range(expr) {
        let p1 = parse_time_point(left)?;
        let p2 = parse_time_point(right)?;
        resolve_time_range(p1, p2, now, unit_secs)?
    } else {
        let p = parse_time_point(expr)?;
        resolve_single_time(p, now, unit_secs)?
    };

    if end != -1 && start > end {
        return Err(CliError::new(2, "Invalid time range: start > end."));
    }
    Ok(TimeFilter { kind, start, end })
}

fn starts_with_prefix(expr: &str) -> bool {
    matches!(expr.chars().next(), Some('+') | Some('-') | Some('~'))
}

fn split_time_range(expr: &str) -> Option<(&str, &str)> {
    let dash_pos = expr.find('-')?;
    if dash_pos == 0 || dash_pos + 1 >= expr.len() {
        return None;
    }
    let right = &expr[dash_pos + 1..];
    if right.chars().all(|c| c == '-') {
        return None;
    }
    Some((&expr[..dash_pos], right))
}

fn resolve_single_time(
    p: TimePoint,
    now: i64,
    unit_secs: impl Fn(char) -> Result<i64, CliError>,
) -> CliResult<(i64, i64)> {
    if p.is_relative {
        let prefix = p
            .prefix
            .ok_or_else(|| CliError::new(2, "Relative time requires a prefix."))?;
        let duration = p.value * unit_secs(p.unit)?;
        match prefix {
            '+' => Ok((0, now - duration)),
            '-' => Ok((now - duration, -1)),
            '~' => {
                let unit = unit_secs(p.unit)?;
                Ok((now - duration - unit, now - duration))
            }
            _ => Err(CliError::new(2, "Invalid time prefix.")),
        }
    } else {
        let prefix = p
            .prefix
            .ok_or_else(|| CliError::new(2, "Absolute time requires a prefix."))?;
        match prefix {
            '+' => Ok((0, p.absolute_secs)),
            '-' => Ok((p.absolute_secs, -1)),
            '~' => Ok((p.absolute_secs, p.absolute_secs + 86_400)),
            _ => Err(CliError::new(2, "Invalid time prefix.")),
        }
    }
}

fn resolve_time_range(
    p1: TimePoint,
    p2: TimePoint,
    now: i64,
    unit_secs: impl Fn(char) -> Result<i64, CliError>,
) -> CliResult<(i64, i64)> {
    if p1.is_relative && p2.is_relative {
        if p1.prefix.is_some() || p2.prefix.is_some() {
            return Err(CliError::new(2, "Relative range must not have prefixes."));
        }
        let d1 = p1.value * unit_secs(p1.unit)?;
        let d2 = p2.value * unit_secs(p2.unit)?;
        let unit = unit_secs(p1.unit)?;
        return Ok((now - d1 - unit, now - d2));
    }
    let resolve = |p: TimePoint| -> CliResult<i64> {
        if p.is_relative {
            if p.prefix.is_none() {
                return Err(CliError::new(
                    2,
                    "Mixed range requires a prefix on the relative part.",
                ));
            }
            let d = p.value * unit_secs(p.unit)?;
            Ok(now - d)
        } else {
            Ok(p.absolute_secs)
        }
    };
    let start = resolve(p1)?;
    let end = resolve(p2)? + if p2.is_relative { 0 } else { 86_400 };
    Ok((start, end))
}

fn parse_time_point(expr: &str) -> CliResult<TimePoint> {
    let mut s = expr.trim();
    if s.is_empty() {
        return Err(CliError::new(2, "Empty time expression."));
    }
    let mut prefix = None;
    if let Some(ch) = s.chars().next()
        && (ch == '+' || ch == '-' || ch == '~')
    {
        prefix = Some(ch);
        s = &s[ch.len_utf8()..];
    }
    if is_absolute_date(s) {
        let (secs, _) = parse_absolute_datetime(s)?;
        return Ok(TimePoint {
            prefix,
            is_relative: false,
            value: 0,
            unit: 'd',
            absolute_secs: secs,
        });
    }
    let (value, unit) = parse_relative_time(s)?;
    Ok(TimePoint {
        prefix,
        is_relative: true,
        value,
        unit,
        absolute_secs: 0,
    })
}

fn is_absolute_date(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 10 && bytes.get(4) == Some(&b'.') && bytes.get(7) == Some(&b'.')
}

fn parse_relative_time(s: &str) -> CliResult<(i64, char)> {
    let mut idx = 0usize;
    let bytes = s.as_bytes();
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    if idx == 0 {
        return Err(CliError::new(2, "Invalid relative time value."));
    }
    let value: i64 = s[..idx]
        .parse()
        .map_err(|_| CliError::new(2, "Invalid time value."))?;
    let rest = s[idx..].trim();
    let mut chars = rest.chars();
    let unit = chars
        .next()
        .ok_or_else(|| CliError::new(2, "Missing time unit."))?
        .to_ascii_lowercase();
    if !chars.as_str().trim().is_empty() {
        return Err(CliError::new(2, "Invalid relative time value."));
    }
    if !"smhdw".contains(unit) {
        return Err(CliError::new(2, format!("Invalid time unit: {unit}")));
    }
    Ok((value, unit))
}

fn parse_absolute_datetime(s: &str) -> CliResult<(i64, usize)> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 && parts.len() != 6 {
        return Err(CliError::new(2, "Invalid date format."));
    }
    let year: i64 = parts[0]
        .parse()
        .map_err(|_| CliError::new(2, "Invalid year."))?;
    let month: i64 = parts[1]
        .parse()
        .map_err(|_| CliError::new(2, "Invalid month."))?;
    let day: i64 = parts[2]
        .parse()
        .map_err(|_| CliError::new(2, "Invalid day."))?;
    let (hour, minute, second) = if parts.len() == 6 {
        let h: i64 = parts[3]
            .parse()
            .map_err(|_| CliError::new(2, "Invalid hour."))?;
        let m: i64 = parts[4]
            .parse()
            .map_err(|_| CliError::new(2, "Invalid minute."))?;
        let sec: i64 = parts[5]
            .parse()
            .map_err(|_| CliError::new(2, "Invalid second."))?;
        (h, m, sec)
    } else {
        (0, 0, 0)
    };
    if !(1..=12).contains(&month) {
        return Err(CliError::new(2, "Invalid date value."));
    }
    let dim = days_in_month(year, month);
    if day < 1 || day > dim {
        return Err(CliError::new(2, "Invalid date value."));
    }
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) || !(0..=59).contains(&second) {
        return Err(CliError::new(2, "Invalid time value."));
    }
    let days = days_from_civil(year, month, day);
    let secs = days.saturating_mul(86_400) + hour * 3_600 + minute * 60 + second;
    Ok((secs, parts.len()))
}

pub(super) fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub(super) fn days_in_month(year: i64, month: i64) -> i64 {
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

pub(crate) fn system_time_to_secs(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}
