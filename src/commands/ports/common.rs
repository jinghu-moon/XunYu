use super::*;

pub(super) fn is_dev_port(port: u16) -> bool {
    (3000..=3999).contains(&port)
        || (5000..=5999).contains(&port)
        || (8000..=8999).contains(&port)
        || port == 4173
        || port == 5173
}

pub(super) fn trunc(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    let start = chars.len().saturating_sub(max - 3);
    format!("...{}", chars[start..].iter().collect::<String>())
}

pub(super) fn proto_rank(p: Protocol) -> u8 {
    match p {
        Protocol::Tcp => 0,
        Protocol::Udp => 1,
    }
}

pub(super) fn parse_range(raw: &str) -> Option<(u16, u16)> {
    let mut parts = raw.split('-').map(str::trim).filter(|s| !s.is_empty());
    let start = parts.next()?.parse::<u16>().ok()?;
    let end = parts.next()?.parse::<u16>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    if start <= end {
        Some((start, end))
    } else {
        Some((end, start))
    }
}
