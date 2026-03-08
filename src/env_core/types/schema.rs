use serde::{Deserialize, Serialize};

use super::EnvScope;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvSchema {
    #[serde(default)]
    pub rules: Vec<SchemaRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRule {
    pub pattern: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub warn_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaViolation {
    pub name: Option<String>,
    pub pattern: String,
    pub kind: String,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub scope: EnvScope,
    pub total_vars: usize,
    #[serde(default)]
    pub violations: Vec<SchemaViolation>,
    pub errors: usize,
    pub warnings: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationEntry {
    pub name: String,
    pub note: String,
}
