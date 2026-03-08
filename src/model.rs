use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct Entry {
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) visit_count: u32,
    #[serde(default)]
    pub(crate) last_visited: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ListItem {
    pub(crate) name: String,
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) visits: u32,
    #[serde(default)]
    pub(crate) last_visited: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListFormat {
    Auto,
    Table,
    Tsv,
    Json,
}

pub(crate) fn parse_list_format(raw: &str) -> Option<ListFormat> {
    match raw.to_lowercase().as_str() {
        "auto" => Some(ListFormat::Auto),
        "table" => Some(ListFormat::Table),
        "tsv" => Some(ListFormat::Tsv),
        "json" => Some(ListFormat::Json),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DedupMode {
    Path,
    Name,
}

pub(crate) fn parse_dedup_mode(raw: &str) -> Option<DedupMode> {
    match raw.to_lowercase().as_str() {
        "path" => Some(DedupMode::Path),
        "name" => Some(DedupMode::Name),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum IoFormat {
    Json,
    Tsv,
}

pub(crate) fn parse_io_format(raw: &str) -> Option<IoFormat> {
    match raw.to_lowercase().as_str() {
        "json" => Some(IoFormat::Json),
        "tsv" => Some(IoFormat::Tsv),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImportMode {
    Merge,
    Overwrite,
}

pub(crate) fn parse_import_mode(raw: &str) -> Option<ImportMode> {
    match raw.to_lowercase().as_str() {
        "merge" => Some(ImportMode::Merge),
        "overwrite" => Some(ImportMode::Overwrite),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_serde_roundtrip() {
        let e = Entry {
            path: "C:\\tmp\\a".to_string(),
            tags: vec!["TagA".to_string(), "tagb".to_string()],
            visit_count: 42,
            last_visited: 123,
        };
        let s = serde_json::to_string(&e).expect("serialize");
        let d: Entry = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(d.path, e.path);
        assert_eq!(d.tags, e.tags);
        assert_eq!(d.visit_count, e.visit_count);
        assert_eq!(d.last_visited, e.last_visited);
    }

    #[test]
    fn entry_defaults_are_zero() {
        let e = Entry::default();
        assert_eq!(e.visit_count, 0);
        assert_eq!(e.last_visited, 0);
    }

    #[test]
    fn parse_list_format_accepts_known_and_rejects_unknown() {
        assert!(matches!(parse_list_format("auto"), Some(ListFormat::Auto)));
        assert!(matches!(
            parse_list_format("table"),
            Some(ListFormat::Table)
        ));
        assert!(matches!(parse_list_format("tsv"), Some(ListFormat::Tsv)));
        assert!(matches!(parse_list_format("json"), Some(ListFormat::Json)));

        // Case-insensitive.
        assert!(matches!(parse_list_format("TSV"), Some(ListFormat::Tsv)));

        assert!(parse_list_format("nope").is_none());
    }

    #[test]
    fn parse_dedup_mode_accepts_known_and_rejects_unknown() {
        assert!(matches!(parse_dedup_mode("path"), Some(DedupMode::Path)));
        assert!(matches!(parse_dedup_mode("name"), Some(DedupMode::Name)));
        assert!(matches!(parse_dedup_mode("NAME"), Some(DedupMode::Name)));
        assert!(parse_dedup_mode("nope").is_none());
    }

    #[test]
    fn parse_io_format_accepts_known_and_rejects_unknown() {
        assert!(matches!(parse_io_format("json"), Some(IoFormat::Json)));
        assert!(matches!(parse_io_format("tsv"), Some(IoFormat::Tsv)));
        assert!(matches!(parse_io_format("TSV"), Some(IoFormat::Tsv)));
        assert!(parse_io_format("nope").is_none());
    }

    #[test]
    fn parse_import_mode_accepts_known_and_rejects_unknown() {
        assert!(matches!(
            parse_import_mode("merge"),
            Some(ImportMode::Merge)
        ));
        assert!(matches!(
            parse_import_mode("overwrite"),
            Some(ImportMode::Overwrite)
        ));
        assert!(matches!(
            parse_import_mode("OVERWRITE"),
            Some(ImportMode::Overwrite)
        ));
        assert!(parse_import_mode("nope").is_none());
    }
}
