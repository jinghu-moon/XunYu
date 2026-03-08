// cstat/lang.rs
//
// Hardcoded comment-syntax rules for supported languages.

/// Comment syntax rules for a language.
#[derive(Clone, Debug)]
pub(crate) struct LangRules {
    pub name: &'static str,
    /// Single-line comment prefix, e.g. "//"
    pub line_comment: Option<&'static str>,
    /// Block comment start, e.g. "/*"
    pub block_start: Option<&'static str>,
    /// Block comment end, e.g. "*/"
    pub block_end: Option<&'static str>,
    /// Alternative block comment start (HTML/Vue template uses <!-- -->)
    pub block_start2: Option<&'static str>,
    pub block_end2: Option<&'static str>,
}

impl LangRules {
    const fn new(name: &'static str) -> Self {
        LangRules {
            name,
            line_comment: None,
            block_start: None,
            block_end: None,
            block_start2: None,
            block_end2: None,
        }
    }
}

/// Determine language rules from a file extension.
/// Returns `None` for unknown/unsupported extensions.
pub(crate) fn rules_for_ext(ext: &str) -> Option<LangRules> {
    match ext.to_ascii_lowercase().as_str() {
        // TypeScript / JavaScript
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => Some(LangRules {
            name: if ext.starts_with('t') {
                "TypeScript"
            } else {
                "JavaScript"
            },
            line_comment: Some("//"),
            block_start: Some("/*"),
            block_end: Some("*/"),
            ..LangRules::new("")
        }),

        // Vue SFC
        "vue" => Some(LangRules {
            name: "Vue",
            line_comment: Some("//"),
            block_start: Some("/*"),
            block_end: Some("*/"),
            block_start2: Some("<!--"),
            block_end2: Some("-->"),
        }),

        // HTML
        "html" | "htm" => Some(LangRules {
            name: "HTML",
            line_comment: None,
            block_start: Some("<!--"),
            block_end: Some("-->"),
            ..LangRules::new("")
        }),

        // CSS / SCSS / Less
        "css" | "scss" | "less" => Some(LangRules {
            name: "CSS",
            line_comment: None,
            block_start: Some("/*"),
            block_end: Some("*/"),
            ..LangRules::new("")
        }),

        // Rust
        "rs" => Some(LangRules {
            name: "Rust",
            line_comment: Some("//"),
            block_start: Some("/*"),
            block_end: Some("*/"),
            ..LangRules::new("")
        }),

        // C / C++
        "c" | "h" => Some(LangRules {
            name: "C",
            line_comment: Some("//"),
            block_start: Some("/*"),
            block_end: Some("*/"),
            ..LangRules::new("")
        }),
        "cpp" | "cxx" | "cc" | "hpp" | "hxx" => Some(LangRules {
            name: "C++",
            line_comment: Some("//"),
            block_start: Some("/*"),
            block_end: Some("*/"),
            ..LangRules::new("")
        }),

        // TOML
        "toml" => Some(LangRules {
            name: "TOML",
            line_comment: Some("#"),
            ..LangRules::new("")
        }),

        // YAML
        "yaml" | "yml" => Some(LangRules {
            name: "YAML",
            line_comment: Some("#"),
            ..LangRules::new("")
        }),

        // JSON — no comments
        "json" | "jsonc" => Some(LangRules {
            name: "JSON",
            ..LangRules::new("")
        }),

        _ => None,
    }
}

/// Temporary file extensions.
pub(crate) const TMP_EXTENSIONS: &[&str] = &["bak", "tmp", "orig", "swp", "old"];

/// Glob-style name prefixes marking temp files.
pub(crate) const TMP_PREFIXES: &[&str] = &["~$", ".#"];
