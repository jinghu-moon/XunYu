use serde::{Deserialize, Serialize};

use super::EnvScope;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateValidationReport {
    pub input: String,
    #[serde(default)]
    pub references: Vec<String>,
    #[serde(default)]
    pub missing: Vec<String>,
    #[serde(default)]
    pub cycles: Vec<Vec<String>>,
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateExpandResult {
    pub input: String,
    pub expanded: String,
    pub report: TemplateValidationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCommandResult {
    pub command_line: String,
    pub exit_code: Option<i32>,
    pub success: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stdout: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stderr: String,
    #[serde(default)]
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvEventType {
    Changed,
    Snapshot,
    Doctor,
    Import,
    Export,
    Diff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvEvent {
    #[serde(rename = "type")]
    pub event_type: EnvEventType,
    pub scope: EnvScope,
    pub at: String,
    pub name: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvAuditEntry {
    pub at: String,
    pub action: String,
    pub scope: EnvScope,
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvWatchEvent {
    pub at: String,
    pub op: String,
    pub scope: EnvScope,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_value: Option<String>,
}
