use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;

use super::io;
use super::registry;
use super::types::{
    EnvResult, EnvScope, LiveExportFormat, ShellExportFormat, TemplateExpandResult,
    TemplateValidationReport,
};

const MAX_EXPANSION_DEPTH: usize = 64;

static TEMPLATE_TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"%([A-Za-z0-9_]+)%").expect("template token regex must be valid"));

pub fn template_expand(scope: EnvScope, input: &str) -> EnvResult<TemplateExpandResult> {
    let vars = scope_vars_map(scope)?;
    Ok(expand_with_map(input, &vars))
}

pub fn template_validate(scope: EnvScope, input: &str) -> EnvResult<TemplateValidationReport> {
    let vars = scope_vars_map(scope)?;
    Ok(expand_with_map(input, &vars).report)
}

pub fn build_runtime_env(
    scope: EnvScope,
    env_files: &[PathBuf],
    set_pairs: &[(String, String)],
) -> EnvResult<BTreeMap<String, String>> {
    let mut merged = BTreeMap::<String, String>::new();
    for (name, value) in std::env::vars() {
        upsert_env_value(&mut merged, &name, &value);
    }

    for var in registry::list_vars(scope)? {
        upsert_env_value(&mut merged, &var.name, &var.raw_value);
    }

    for file in env_files {
        let parsed = io::parse_import_file(file)?;
        for item in parsed.vars {
            upsert_env_value(&mut merged, &item.name, &item.value);
        }
    }

    for (name, value) in set_pairs {
        upsert_env_value(&mut merged, name, value);
    }

    Ok(expand_env_values(&merged))
}

pub fn render_shell_exports(
    env_map: &BTreeMap<String, String>,
    shell: ShellExportFormat,
) -> String {
    let mut lines = Vec::with_capacity(env_map.len());
    for (name, value) in env_map {
        let line = match shell {
            ShellExportFormat::Bash => {
                format!("export {}='{}'", name, escape_single_quoted(value))
            }
            ShellExportFormat::PowerShell => {
                format!("$env:{} = '{}'", name, value.replace('\'', "''"))
            }
            ShellExportFormat::Cmd => {
                format!("set \"{}={}\"", name, value.replace('"', "\\\""))
            }
        };
        lines.push(line);
    }
    lines.join("\n")
}

pub fn render_live_export(
    scope: EnvScope,
    env_map: &BTreeMap<String, String>,
    format: LiveExportFormat,
) -> EnvResult<String> {
    match format {
        LiveExportFormat::Dotenv => Ok(render_dotenv(env_map)),
        LiveExportFormat::Sh => Ok(render_shell_exports(env_map, ShellExportFormat::Bash)),
        LiveExportFormat::Json => serde_json::to_string_pretty(env_map).map_err(Into::into),
        LiveExportFormat::Reg => Ok(render_reg(scope, env_map)),
    }
}

fn scope_vars_map(scope: EnvScope) -> EnvResult<HashMap<String, String>> {
    let mut out = HashMap::<String, String>::new();
    for (name, value) in std::env::vars() {
        out.insert(normalize_key(&name), value);
    }
    let vars = registry::list_vars(scope)?;
    for var in vars {
        out.insert(normalize_key(&var.name), var.raw_value);
    }
    Ok(out)
}

fn expand_with_map(input: &str, vars: &HashMap<String, String>) -> TemplateExpandResult {
    let references = collect_references(input);
    let mut missing = BTreeSet::<String>::new();
    let mut cycles = BTreeSet::<String>::new();
    let mut stack = Vec::<String>::new();
    let mut memo = HashMap::<String, String>::new();
    let expanded = expand_text(
        input,
        vars,
        &mut memo,
        &mut stack,
        &mut missing,
        &mut cycles,
    );
    let valid = missing.is_empty() && cycles.is_empty();

    let report = TemplateValidationReport {
        input: input.to_string(),
        references,
        missing: missing.into_iter().collect(),
        cycles: cycles_to_paths(&cycles),
        valid,
    };

    TemplateExpandResult {
        input: input.to_string(),
        expanded,
        report,
    }
}

fn collect_references(input: &str) -> Vec<String> {
    let mut seen = BTreeSet::<String>::new();
    let mut out = Vec::new();
    for caps in TEMPLATE_TOKEN_RE.captures_iter(input) {
        let Some(name) = caps.get(1).map(|m| m.as_str()) else {
            continue;
        };
        let key = normalize_key(name);
        if seen.insert(key) {
            out.push(name.to_string());
        }
    }
    out
}

fn expand_text(
    text: &str,
    vars: &HashMap<String, String>,
    memo: &mut HashMap<String, String>,
    stack: &mut Vec<String>,
    missing: &mut BTreeSet<String>,
    cycles: &mut BTreeSet<String>,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0usize;
    for caps in TEMPLATE_TOKEN_RE.captures_iter(text) {
        let Some(full) = caps.get(0) else {
            continue;
        };
        let Some(token) = caps.get(1).map(|m| m.as_str()) else {
            continue;
        };
        out.push_str(&text[last..full.start()]);
        out.push_str(&expand_token(token, vars, memo, stack, missing, cycles));
        last = full.end();
    }
    out.push_str(&text[last..]);
    out
}

fn expand_token(
    token: &str,
    vars: &HashMap<String, String>,
    memo: &mut HashMap<String, String>,
    stack: &mut Vec<String>,
    missing: &mut BTreeSet<String>,
    cycles: &mut BTreeSet<String>,
) -> String {
    let normalized = normalize_key(token);

    if let Some(expanded) = memo.get(&normalized) {
        return expanded.clone();
    }

    if let Some(pos) = stack.iter().position(|v| *v == normalized) {
        let mut path = stack[pos..].to_vec();
        path.push(normalized.clone());
        cycles.insert(path.join("->"));
        return format!("%{}%", token);
    }

    let Some(raw) = vars.get(&normalized) else {
        missing.insert(token.to_string());
        return format!("%{}%", token);
    };

    if stack.len() >= MAX_EXPANSION_DEPTH {
        cycles.insert(format!("depth_limit->{}", normalized));
        return format!("%{}%", token);
    }

    stack.push(normalized.clone());
    let expanded = expand_text(raw, vars, memo, stack, missing, cycles);
    let _ = stack.pop();

    memo.insert(normalized, expanded.clone());
    expanded
}

fn cycles_to_paths(cycles: &BTreeSet<String>) -> Vec<Vec<String>> {
    cycles
        .iter()
        .map(|raw| raw.split("->").map(|s| s.to_string()).collect::<Vec<_>>())
        .collect()
}

fn normalize_key(name: &str) -> String {
    name.to_ascii_uppercase()
}

fn upsert_env_value(map: &mut BTreeMap<String, String>, name: &str, value: &str) {
    let existed = map.keys().find(|k| k.eq_ignore_ascii_case(name)).cloned();
    if let Some(existing_key) = existed {
        map.insert(existing_key, value.to_string());
    } else {
        map.insert(name.to_string(), value.to_string());
    }
}

fn expand_env_values(input: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut source = HashMap::<String, String>::new();
    for (name, value) in input {
        source.insert(normalize_key(name), value.clone());
    }

    let mut out = BTreeMap::<String, String>::new();
    let mut missing = BTreeSet::<String>::new();
    let mut cycles = BTreeSet::<String>::new();
    let mut stack = Vec::<String>::new();
    let mut memo = HashMap::<String, String>::new();
    for (name, value) in input {
        let expanded = expand_text(
            value,
            &source,
            &mut memo,
            &mut stack,
            &mut missing,
            &mut cycles,
        );
        out.insert(name.clone(), expanded);
    }
    out
}

fn render_dotenv(map: &BTreeMap<String, String>) -> String {
    let mut lines = Vec::with_capacity(map.len());
    for (name, value) in map {
        if value.contains(' ') || value.contains('"') || value.contains('#') {
            lines.push(format!("{}=\"{}\"", name, value.replace('"', "\\\"")));
        } else {
            lines.push(format!("{}={}", name, value));
        }
    }
    lines.join("\n")
}

fn render_reg(scope: EnvScope, map: &BTreeMap<String, String>) -> String {
    let hive = match scope {
        EnvScope::System => {
            "HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment"
        }
        _ => "HKEY_CURRENT_USER\\Environment",
    };
    let mut lines = vec![
        "Windows Registry Editor Version 5.00".to_string(),
        String::new(),
        format!("[{}]", hive),
    ];

    for (name, value) in map {
        if name.eq_ignore_ascii_case("PATH") || value.contains('%') {
            let hex: Vec<String> = value
                .encode_utf16()
                .chain(std::iter::once(0))
                .flat_map(|u| u.to_le_bytes())
                .map(|b| format!("{:02x}", b))
                .collect();
            lines.push(format!("\"{}\"=hex(2):{}", name, hex.join(",")));
        } else {
            lines.push(format!(
                "\"{}\"=\"{}\"",
                name,
                value.replace('\\', "\\\\").replace('"', "\\\"")
            ));
        }
    }

    lines.join("\n")
}

fn escape_single_quoted(raw: &str) -> String {
    raw.replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{expand_with_map, normalize_key};

    #[test]
    fn expand_nested_template() {
        let mut vars = HashMap::new();
        vars.insert(normalize_key("A"), "%B%".to_string());
        vars.insert(normalize_key("B"), "ok".to_string());
        let result = expand_with_map("value=%A%", &vars);
        assert_eq!(result.expanded, "value=ok");
        assert!(result.report.valid);
    }

    #[test]
    fn detect_var_cycle() {
        let mut vars = HashMap::new();
        vars.insert(normalize_key("A"), "%B%".to_string());
        vars.insert(normalize_key("B"), "%A%".to_string());
        let result = expand_with_map("%A%", &vars);
        assert!(!result.report.valid);
        assert!(!result.report.cycles.is_empty());
    }
}
