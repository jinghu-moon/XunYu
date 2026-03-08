use std::fs;
use std::path::PathBuf;

use regex::Regex;

use super::config::{EnvCoreConfig, config_file_path};
use super::registry;
use super::types::{
    EnvError, EnvResult, EnvSchema, EnvScope, SchemaRule, SchemaViolation, ValidationReport,
};

pub fn schema_file_path(_cfg: &EnvCoreConfig) -> PathBuf {
    config_file_path().with_file_name(".xun.env.schema.json")
}

pub fn load_schema(cfg: &EnvCoreConfig) -> EnvResult<EnvSchema> {
    let path = schema_file_path(cfg);
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str::<EnvSchema>(&content).map_err(Into::into),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(EnvSchema::default()),
        Err(e) => Err(EnvError::Io(e)),
    }
}

pub fn save_schema(cfg: &EnvCoreConfig, schema: &EnvSchema) -> EnvResult<()> {
    let path = schema_file_path(cfg);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(schema)?)?;
    Ok(())
}

pub fn reset_schema(cfg: &EnvCoreConfig) -> EnvResult<EnvSchema> {
    let schema = EnvSchema::default();
    save_schema(cfg, &schema)?;
    Ok(schema)
}

pub fn add_or_replace_rule(cfg: &EnvCoreConfig, rule: SchemaRule) -> EnvResult<EnvSchema> {
    let mut schema = load_schema(cfg)?;
    if let Some(existing) = schema
        .rules
        .iter_mut()
        .find(|it| it.pattern.eq_ignore_ascii_case(&rule.pattern))
    {
        *existing = rule;
    } else {
        schema.rules.push(rule);
    }
    save_schema(cfg, &schema)?;
    Ok(schema)
}

pub fn remove_rule(cfg: &EnvCoreConfig, pattern: &str) -> EnvResult<EnvSchema> {
    let mut schema = load_schema(cfg)?;
    schema
        .rules
        .retain(|r| !r.pattern.eq_ignore_ascii_case(pattern));
    save_schema(cfg, &schema)?;
    Ok(schema)
}

pub fn validate_schema(
    cfg: &EnvCoreConfig,
    scope: EnvScope,
    strict: bool,
) -> EnvResult<ValidationReport> {
    let schema = load_schema(cfg)?;
    let vars = registry::list_vars(scope)?;

    let mut report = ValidationReport {
        scope,
        total_vars: vars.len(),
        violations: Vec::new(),
        errors: 0,
        warnings: 0,
        passed: true,
    };

    for rule in &schema.rules {
        let matcher = wildcard_to_matcher(&rule.pattern)?;
        let matched: Vec<_> = vars.iter().filter(|v| matcher.is_match(&v.name)).collect();
        if rule.required && matched.is_empty() {
            push_violation(
                &mut report,
                SchemaViolation {
                    name: None,
                    pattern: rule.pattern.clone(),
                    kind: "required_missing".to_string(),
                    message: format!("required pattern '{}' has no match", rule.pattern),
                    severity: severity_of(rule, strict).to_string(),
                },
            );
            continue;
        }

        if let Some(pattern) = &rule.regex {
            let regex = match Regex::new(pattern) {
                Ok(v) => v,
                Err(e) => {
                    push_violation(
                        &mut report,
                        SchemaViolation {
                            name: None,
                            pattern: rule.pattern.clone(),
                            kind: "invalid_rule_regex".to_string(),
                            message: format!("invalid regex '{}': {}", pattern, e),
                            severity: "error".to_string(),
                        },
                    );
                    continue;
                }
            };
            for var in &matched {
                if !regex.is_match(&var.raw_value) {
                    push_violation(
                        &mut report,
                        SchemaViolation {
                            name: Some(var.name.clone()),
                            pattern: rule.pattern.clone(),
                            kind: "regex_mismatch".to_string(),
                            message: format!(
                                "value does not match regex '{}': {}",
                                pattern, var.raw_value
                            ),
                            severity: severity_of(rule, strict).to_string(),
                        },
                    );
                }
            }
        }

        if !rule.enum_values.is_empty() {
            for var in &matched {
                if !rule.enum_values.iter().any(|v| v == &var.raw_value) {
                    push_violation(
                        &mut report,
                        SchemaViolation {
                            name: Some(var.name.clone()),
                            pattern: rule.pattern.clone(),
                            kind: "enum_mismatch".to_string(),
                            message: format!(
                                "value '{}' not in enum [{}]",
                                var.raw_value,
                                rule.enum_values.join(", ")
                            ),
                            severity: severity_of(rule, strict).to_string(),
                        },
                    );
                }
            }
        }
    }

    report.passed = report.errors == 0 && report.warnings == 0;
    Ok(report)
}

fn push_violation(report: &mut ValidationReport, violation: SchemaViolation) {
    if violation.severity == "warning" {
        report.warnings += 1;
    } else {
        report.errors += 1;
    }
    report.violations.push(violation);
}

fn severity_of(rule: &SchemaRule, strict: bool) -> &'static str {
    if rule.warn_only && !strict {
        "warning"
    } else {
        "error"
    }
}

fn wildcard_to_matcher(pattern: &str) -> EnvResult<Regex> {
    let mut out = String::with_capacity(pattern.len() * 2 + 6);
    out.push('^');
    for ch in pattern.chars() {
        match ch {
            '*' => out.push_str(".*"),
            '?' => out.push('.'),
            _ => out.push_str(&regex::escape(&ch.to_string())),
        }
    }
    out.push('$');
    Regex::new(&format!("(?i:{})", out))
        .map_err(|e| EnvError::InvalidInput(format!("invalid schema pattern '{}': {}", pattern, e)))
}
