use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type EnvResult<T> = Result<T, EnvError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvScope {
    User,
    System,
    All,
}

impl EnvScope {
    pub fn is_writable(self) -> bool {
        matches!(self, Self::User | Self::System)
    }

    pub fn expand_scopes(self) -> &'static [EnvScope] {
        match self {
            Self::User => &[Self::User],
            Self::System => &[Self::System],
            Self::All => &[Self::User, Self::System],
        }
    }
}

impl std::fmt::Display for EnvScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::System => write!(f, "system"),
            Self::All => write!(f, "all"),
        }
    }
}

impl FromStr for EnvScope {
    type Err = EnvError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Self::User),
            "system" => Ok(Self::System),
            "all" => Ok(Self::All),
            _ => Err(EnvError::InvalidInput(format!(
                "invalid scope '{}', expected user|system|all",
                s
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub scope: EnvScope,
    pub name: String,
    pub raw_value: String,
    pub reg_type: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inferred_kind: Option<EnvVarKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvVarKind {
    Url,
    Path,
    PathList,
    Boolean,
    Secret,
    Json,
    Email,
    Version,
    Integer,
    Float,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub name: String,
    pub raw_value: String,
    pub reg_type: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub description: String,
    pub created_at: String,
    #[serde(default)]
    pub user_vars: Vec<SnapshotEntry>,
    #[serde(default)]
    pub system_vars: Vec<SnapshotEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub id: String,
    pub description: String,
    pub created_at: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorIssueKind {
    PathMissing,
    PathDuplicate,
    PathTooLong,
    VarCycle,
    UserShadowsSystem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorIssue {
    pub kind: DoctorIssueKind,
    pub severity: String,
    pub scope: EnvScope,
    pub name: String,
    pub message: String,
    #[serde(default)]
    pub fixable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub scope: EnvScope,
    #[serde(default)]
    pub issues: Vec<DoctorIssue>,
    pub errors: usize,
    pub warnings: usize,
    pub fixable: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffChangeKind {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSegmentDiff {
    pub segment: String,
    pub kind: DiffChangeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub name: String,
    pub kind: DiffChangeKind,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path_diff: Vec<PathSegmentDiff>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvDiff {
    #[serde(default)]
    pub added: Vec<DiffEntry>,
    #[serde(default)]
    pub removed: Vec<DiffEntry>,
    #[serde(default)]
    pub changed: Vec<DiffEntry>,
}

impl EnvDiff {
    pub fn total_changes(&self) -> usize {
        self.added.len() + self.removed.len() + self.changed.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Env,
    Reg,
    Csv,
}

impl FromStr for ExportFormat {
    type Err = EnvError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "env" | "dotenv" => Ok(Self::Env),
            "reg" => Ok(Self::Reg),
            "csv" => Ok(Self::Csv),
            _ => Err(EnvError::InvalidInput(format!(
                "invalid export format '{}', expected json|env|reg|csv",
                s
            ))),
        }
    }
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Env => write!(f, "env"),
            Self::Reg => write!(f, "reg"),
            Self::Csv => write!(f, "csv"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LiveExportFormat {
    Dotenv,
    Sh,
    Json,
    Reg,
}

impl FromStr for LiveExportFormat {
    type Err = EnvError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dotenv" | "env" => Ok(Self::Dotenv),
            "sh" | "bash" => Ok(Self::Sh),
            "json" => Ok(Self::Json),
            "reg" => Ok(Self::Reg),
            _ => Err(EnvError::InvalidInput(format!(
                "invalid live export format '{}', expected dotenv|sh|json|reg",
                s
            ))),
        }
    }
}

impl std::fmt::Display for LiveExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dotenv => write!(f, "dotenv"),
            Self::Sh => write!(f, "sh"),
            Self::Json => write!(f, "json"),
            Self::Reg => write!(f, "reg"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellExportFormat {
    Bash,
    PowerShell,
    Cmd,
}

impl FromStr for ShellExportFormat {
    type Err = EnvError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bash" | "sh" => Ok(Self::Bash),
            "powershell" | "pwsh" | "ps1" => Ok(Self::PowerShell),
            "cmd" | "cmd.exe" => Ok(Self::Cmd),
            _ => Err(EnvError::InvalidInput(format!(
                "invalid shell '{}', expected bash|powershell|cmd",
                s
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportStrategy {
    Merge,
    Overwrite,
}

impl Default for ImportStrategy {
    fn default() -> Self {
        Self::Merge
    }
}

impl FromStr for ImportStrategy {
    type Err = EnvError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "merge" => Ok(Self::Merge),
            "overwrite" => Ok(Self::Overwrite),
            _ => Err(EnvError::InvalidInput(format!(
                "invalid import strategy '{}', expected merge|overwrite",
                s
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedImportVar {
    pub name: String,
    pub value: String,
    pub reg_type: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedImport {
    pub format: String,
    pub scope_hint: Option<EnvScope>,
    #[serde(default)]
    pub vars: Vec<ParsedImportVar>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportApplyResult {
    pub dry_run: bool,
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
    #[serde(default)]
    pub changed_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorFixResult {
    pub scope: EnvScope,
    pub fixed: usize,
    #[serde(default)]
    pub details: Vec<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub dry_run: bool,
    pub scope: EnvScope,
    pub added: usize,
    pub updated: usize,
    pub deleted: usize,
    pub renamed: usize,
    pub skipped: usize,
    #[serde(default)]
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvProfile {
    pub name: String,
    pub scope: EnvScope,
    pub created_at: String,
    #[serde(default)]
    pub vars: Vec<SnapshotEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvProfileMeta {
    pub name: String,
    pub scope: EnvScope,
    pub created_at: String,
    pub path: PathBuf,
    pub var_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvStatusSummary {
    pub scope: EnvScope,
    pub user_vars: Option<usize>,
    pub system_vars: Option<usize>,
    pub total_vars: Option<usize>,
    pub snapshots: usize,
    pub latest_snapshot_id: Option<String>,
    pub latest_snapshot_at: Option<String>,
    pub profiles: usize,
    pub schema_rules: usize,
    pub annotations: usize,
    pub audit_entries: usize,
    pub last_audit_at: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvDepTree {
    pub scope: EnvScope,
    pub root: String,
    #[serde(default)]
    pub lines: Vec<String>,
    #[serde(default)]
    pub missing: Vec<String>,
    #[serde(default)]
    pub cycles: Vec<String>,
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

#[derive(Debug, Error)]
pub enum EnvError {
    #[error("unsupported platform: EnvMgr requires Windows registry APIs")]
    UnsupportedPlatform,

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("scope '{0}' is not writable")]
    ScopeNotWritable(EnvScope),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("lock acquisition failed: {0}")]
    LockFailed(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),

    #[error("{0}")]
    Other(String),
}

impl EnvError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::InvalidInput(_) | Self::ScopeNotWritable(_) => 2,
            Self::PermissionDenied(_) => 5,
            Self::NotFound(_) => 4,
            Self::UnsupportedPlatform => 6,
            _ => 1,
        }
    }
}
