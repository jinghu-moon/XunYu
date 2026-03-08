//! 语言配置 + tree-sitter Query 常量
//!
//! 支持 5 种 AST 语言 + 行级回退覆盖。
//! Query 常量复用 cfx 参考实现（已验证可用）。

use std::sync::OnceLock;

use tree_sitter::{Language, Query};

// ── Tree-sitter Query 常量 ───────────────────────────────────────────────────

/// JavaScript：函数、类、变量声明、import/export
const JS_QUERY: &str = r#"
(function_declaration name: (identifier) @name) @symbol
(class_declaration) @symbol
(lexical_declaration
  (variable_declarator name: (identifier) @name)) @symbol
(variable_declaration
  (variable_declarator name: (identifier) @name)) @symbol
(export_statement declaration: (_) @inner) @symbol
"#;

/// TypeScript：函数、类、接口、类型别名、变量、export
const TS_QUERY: &str = r#"
(function_declaration name: (identifier) @name) @symbol
(class_declaration) @symbol
(interface_declaration name: (type_identifier) @name) @symbol
(type_alias_declaration name: (type_identifier) @name) @symbol
(lexical_declaration (variable_declarator name: (identifier) @name)) @symbol
(export_statement declaration: (_) @inner) @symbol
"#;

/// CSS：规则集、at 规则
const CSS_QUERY: &str = r#"
(rule_set) @symbol
(at_rule) @symbol
"#;

/// Rust：函数、结构体、枚举、trait、impl、const、type
const RUST_QUERY: &str = r#"
(function_item name: (identifier) @name) @symbol
(struct_item name: (type_identifier) @name) @symbol
(enum_item name: (type_identifier) @name) @symbol
(trait_item name: (type_identifier) @name) @symbol
(impl_item) @symbol
(const_item name: (identifier) @name) @symbol
(type_item name: (type_identifier) @name) @symbol
"#;

/// HTML：顶层元素
const HTML_QUERY: &str = r#"
(element) @symbol
"#;

// ── 语言配置 ──────────────────────────────────────────────────────────────────

/// 语言信息：tree-sitter Language + Query 源码
struct LangConfig {
    language: Language,
    query_src: &'static str,
}

/// 根据扩展名获取语言配置（返回 None 表示不支持 AST diff）
fn get_lang_config(ext: &str) -> Option<LangConfig> {
    match ext {
        "js" | "mjs" | "cjs" => Some(LangConfig {
            language: tree_sitter_javascript::LANGUAGE.into(),
            query_src: JS_QUERY,
        }),
        "ts" | "mts" | "cts" => Some(LangConfig {
            language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            query_src: TS_QUERY,
        }),
        "css" => Some(LangConfig {
            language: tree_sitter_css::LANGUAGE.into(),
            query_src: CSS_QUERY,
        }),
        "rs" => Some(LangConfig {
            language: tree_sitter_rust::LANGUAGE.into(),
            query_src: RUST_QUERY,
        }),
        "html" => Some(LangConfig {
            language: tree_sitter_html::LANGUAGE.into(),
            query_src: HTML_QUERY,
        }),
        _ => None,
    }
}

// ── 公开 API ──────────────────────────────────────────────────────────────────

/// 检查扩展名是否支持 AST diff
pub fn has_ast_support(ext: &str) -> bool {
    matches!(
        ext,
        "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "css" | "rs" | "html"
    )
}

/// 获取 tree-sitter Language（用于 Parser::set_language）
pub fn get_language(ext: &str) -> Option<Language> {
    get_lang_config(ext).map(|c| c.language)
}

// ── Query 编译缓存（OnceLock，每种语言只编译一次） ────────────────────────────

macro_rules! cached_query {
    ($name:ident, $ext:expr) => {
        static $name: OnceLock<Query> = OnceLock::new();
    };
}

cached_query!(Q_JS, "js");
cached_query!(Q_TS, "ts");
cached_query!(Q_CSS, "css");
cached_query!(Q_RUST, "rs");
cached_query!(Q_HTML, "html");

/// 获取编译后的 Query（缓存，首次编译后复用）
pub fn get_query(ext: &str) -> Option<&'static Query> {
    let config = get_lang_config(ext)?;

    let cache = match ext {
        "js" | "mjs" | "cjs" => &Q_JS,
        "ts" | "mts" | "cts" => &Q_TS,
        "css" => &Q_CSS,
        "rs" => &Q_RUST,
        "html" => &Q_HTML,
        _ => return None,
    };

    Some(cache.get_or_init(|| {
        Query::new(&config.language, config.query_src).expect("built-in query should compile")
    }))
}
