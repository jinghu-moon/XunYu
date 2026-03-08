pub(crate) mod cache;
pub(crate) mod path_env;
pub(crate) mod registry;
pub(crate) mod startmenu;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct AppEntry {
    pub(crate) name: String,
    pub(crate) display_name: String,
    pub(crate) exe_path: String,
    pub(crate) source: Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Source {
    Registry,
    StartMenu,
    PathEnv,
}

impl Source {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Registry => "registry",
            Self::StartMenu => "startmenu",
            Self::PathEnv => "path",
        }
    }
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanSource {
    Registry,
    StartMenu,
    PathEnv,
    All,
}

impl ScanSource {
    pub(crate) fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "reg" | "registry" => Some(Self::Registry),
            "start" | "startmenu" => Some(Self::StartMenu),
            "path" | "path_env" => Some(Self::PathEnv),
            "all" | "" => Some(Self::All),
            _ => None,
        }
    }
}

pub(crate) fn scan(source: ScanSource, filter: Option<&str>, no_cache: bool) -> Vec<AppEntry> {
    let mut entries = match source {
        ScanSource::Registry => registry::scan_registry(no_cache),
        ScanSource::StartMenu => startmenu::scan_startmenu(no_cache),
        ScanSource::PathEnv => path_env::scan_path_env(no_cache),
        ScanSource::All => {
            let mut list = registry::scan_registry(no_cache);
            list.extend(startmenu::scan_startmenu(no_cache));
            list.extend(path_env::scan_path_env(no_cache));
            list
        }
    };

    dedup_by_exe(&mut entries);

    if let Some(keyword) = filter {
        let kw = keyword.trim().to_ascii_lowercase();
        if !kw.is_empty() {
            entries.retain(|entry| {
                entry.name.to_ascii_lowercase().contains(&kw)
                    || entry.display_name.to_ascii_lowercase().contains(&kw)
                    || entry.exe_path.to_ascii_lowercase().contains(&kw)
            });
        }
    }

    entries.sort_by(|a, b| {
        a.display_name
            .to_ascii_lowercase()
            .cmp(&b.display_name.to_ascii_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });
    entries
}

pub(crate) fn auto_alias(display_name: &str) -> String {
    let first = display_name
        .split_whitespace()
        .next()
        .unwrap_or(display_name);
    let mut out = String::with_capacity(first.len());
    for ch in first.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        }
    }
    if out.is_empty() {
        "app".to_string()
    } else {
        out
    }
}

pub(crate) fn is_utility_exe(name: &str) -> bool {
    let v = name.to_ascii_lowercase();
    const KEYWORDS: &[&str] = &[
        "uninstall",
        "uninst",
        "setup",
        "installer",
        "updater",
        "update",
        "repair",
        "maintenance",
        "crash",
        "helper",
    ];
    KEYWORDS.iter().any(|k| v.contains(k))
}

fn source_priority(source: Source) -> u8 {
    match source {
        Source::Registry => 0,
        Source::StartMenu => 1,
        Source::PathEnv => 2,
    }
}

fn dedup_by_exe(entries: &mut Vec<AppEntry>) {
    entries.sort_by(|a, b| {
        source_priority(a.source)
            .cmp(&source_priority(b.source))
            .then_with(|| {
                a.exe_path
                    .to_ascii_lowercase()
                    .cmp(&b.exe_path.to_ascii_lowercase())
            })
    });
    let mut seen = std::collections::HashSet::new();
    entries.retain(|entry| {
        let key = entry.exe_path.to_ascii_lowercase();
        seen.insert(key)
    });
}
