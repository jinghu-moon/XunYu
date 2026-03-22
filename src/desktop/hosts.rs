use crate::output::CliError;

const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";

#[derive(Debug, Clone)]
pub(crate) struct HostEntry {
    pub(crate) ip: String,
    pub(crate) host: String,
    pub(crate) enabled: bool,
    pub(crate) comment: Option<String>,
}

impl HostEntry {
    fn to_line(&self) -> String {
        let prefix = if self.enabled { "" } else { "# " };
        let comment_part = self
            .comment
            .as_deref()
            .map(|c| format!("  # {c}"))
            .unwrap_or_default();
        format!("{}{}\t{}{}", prefix, self.ip, self.host, comment_part)
    }
}

pub(crate) fn list_entries() -> Result<Vec<HostEntry>, CliError> {
    #[cfg(windows)]
    {
        let entries = parse_hosts()?;
        Ok(entries.into_iter().filter(|e| e.enabled).collect())
    }
    #[cfg(not(windows))]
    {
        Err(CliError::new(2, "desktop hosts is Windows-only."))
    }
}

pub(crate) fn add_entry(ip: &str, host: &str) -> Result<(), CliError> {
    #[cfg(windows)]
    {
        let mut entries = parse_hosts()?;
        if entries.iter().any(|e| e.host.eq_ignore_ascii_case(host)) {
            return Err(CliError::with_details(
                2,
                format!("Host already exists: {host}"),
                &["Fix: remove the existing host entry first."],
            ));
        }
        entries.push(HostEntry {
            ip: ip.to_string(),
            host: host.to_string(),
            enabled: true,
            comment: None,
        });
        write_hosts(&entries)
    }
    #[cfg(not(windows))]
    {
        let _ = (ip, host);
        Err(CliError::new(2, "desktop hosts is Windows-only."))
    }
}

pub(crate) fn preview_add_entry(ip: &str, host: &str) -> Result<HostEntry, CliError> {
    #[cfg(windows)]
    {
        let entries = parse_hosts()?;
        if entries.iter().any(|e| e.host.eq_ignore_ascii_case(host)) {
            return Err(CliError::with_details(
                2,
                format!("Host already exists: {host}"),
                &["Fix: remove the existing host entry first."],
            ));
        }
        Ok(HostEntry {
            ip: ip.to_string(),
            host: host.to_string(),
            enabled: true,
            comment: None,
        })
    }
    #[cfg(not(windows))]
    {
        let _ = (ip, host);
        Err(CliError::new(2, "desktop hosts is Windows-only."))
    }
}

pub(crate) fn remove_entry(host: &str) -> Result<bool, CliError> {
    #[cfg(windows)]
    {
        let mut entries = parse_hosts()?;
        let before = entries.len();
        entries.retain(|e| !e.host.eq_ignore_ascii_case(host));
        if entries.len() == before {
            return Ok(false);
        }
        write_hosts(&entries)?;
        Ok(true)
    }
    #[cfg(not(windows))]
    {
        let _ = host;
        Err(CliError::new(2, "desktop hosts is Windows-only."))
    }
}

pub(crate) fn preview_remove_entry(host: &str) -> Result<bool, CliError> {
    #[cfg(windows)]
    {
        let entries = parse_hosts()?;
        Ok(entries.iter().any(|e| e.host.eq_ignore_ascii_case(host)))
    }
    #[cfg(not(windows))]
    {
        let _ = host;
        Err(CliError::new(2, "desktop hosts is Windows-only."))
    }
}

#[cfg(windows)]
fn parse_hosts() -> Result<Vec<HostEntry>, CliError> {
    let content = std::fs::read_to_string(HOSTS_PATH).map_err(map_hosts_io_error)?;

    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || (trimmed.starts_with('#') && !looks_like_disabled_entry(trimmed)) {
            continue;
        }

        let (enabled, content_part) = if trimmed.starts_with('#') {
            (false, trimmed.trim_start_matches('#').trim())
        } else {
            (true, trimmed)
        };

        let (entry_part, comment) = if let Some(idx) = content_part.find(" # ") {
            (
                &content_part[..idx],
                Some(content_part[idx + 3..].trim().to_string()),
            )
        } else {
            (content_part, None)
        };

        let mut parts = entry_part.split_whitespace();
        let Some(ip) = parts.next() else {
            continue;
        };
        let Some(host) = parts.next() else {
            continue;
        };

        entries.push(HostEntry {
            ip: ip.to_string(),
            host: host.to_string(),
            enabled,
            comment,
        });
    }

    Ok(entries)
}

#[cfg(windows)]
fn write_hosts(entries: &[HostEntry]) -> Result<(), CliError> {
    let original = std::fs::read_to_string(HOSTS_PATH).map_err(map_hosts_io_error)?;
    let mut header_lines: Vec<&str> = original
        .lines()
        .take_while(|line| {
            let trimmed = line.trim();
            trimmed.is_empty() || (trimmed.starts_with('#') && !looks_like_disabled_entry(trimmed))
        })
        .collect();

    if header_lines.is_empty() {
        header_lines.push("# Windows hosts file managed by xun");
    }

    let mut content = header_lines.join("\n");
    content.push('\n');
    for entry in entries {
        content.push_str(&entry.to_line());
        content.push('\n');
    }

    std::fs::write(HOSTS_PATH, content).map_err(map_hosts_io_error)
}

#[cfg(windows)]
fn looks_like_disabled_entry(line: &str) -> bool {
    let content = line.trim_start_matches('#').trim();
    let mut parts = content.split_whitespace();
    if let (Some(ip), Some(_)) = (parts.next(), parts.next()) {
        return ip.chars().all(|c| c.is_ascii_digit() || c == '.');
    }
    false
}

#[cfg(windows)]
fn map_hosts_io_error(err: std::io::Error) -> CliError {
    if err.kind() == std::io::ErrorKind::PermissionDenied {
        CliError::with_details(
            2,
            "Hosts file access denied.".to_string(),
            &["Fix: run terminal as Administrator."],
        )
    } else {
        CliError::new(2, format!("Hosts file error: {err}"))
    }
}
