use serde::{Deserialize, Serialize};

use crate::env_core::types::{
    AnnotationEntry, DoctorFixResult, DoctorReport, EnvAuditEntry, EnvDepTree, EnvDiff,
    EnvProfileMeta, EnvSchema, EnvScope, EnvStatusSummary, EnvVar, ImportApplyResult,
    RunCommandResult, SnapshotMeta, TemplateValidationReport, ValidationReport,
};

#[derive(Debug, Serialize)]
pub struct ApiSuccess<T> {
    pub ok: bool,
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub ok: bool,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScopeQuery {
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetVarBody {
    pub value: String,
    #[serde(default)]
    pub no_snapshot: bool,
}

#[derive(Debug, Deserialize)]
pub struct PathUpdateBody {
    pub entry: String,
    pub scope: Option<String>,
    #[serde(default)]
    pub head: bool,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotCreateBody {
    pub desc: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotPruneQuery {
    pub keep: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotRestoreBody {
    pub id: Option<String>,
    #[serde(default)]
    pub latest: bool,
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DoctorBody {
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportBody {
    pub content: String,
    pub scope: Option<String>,
    pub mode: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub scope: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExportLiveQuery {
    pub scope: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    pub scope: Option<String>,
    pub snapshot: Option<String>,
    pub since: Option<String>,
    #[serde(default)]
    pub color: bool,
}

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct VarHistoryQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQuery {
    pub scope: Option<String>,
    pub name: String,
    pub max_depth: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ProfileBody {
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProfileCaptureBody {
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ValidateBody {
    pub scope: Option<String>,
    #[serde(default)]
    pub strict: bool,
}

#[derive(Debug, Deserialize)]
pub struct AnnotationBody {
    pub note: String,
}

#[derive(Debug, Deserialize)]
pub struct TemplateExpandBody {
    pub template: String,
    pub scope: Option<String>,
    #[serde(default)]
    pub validate_only: bool,
}

#[derive(Debug, Deserialize)]
pub struct RunBody {
    #[serde(default)]
    pub cmd: Vec<String>,
    pub scope: Option<String>,
    #[serde(default)]
    pub env_files: Vec<String>,
    #[serde(default)]
    pub set: Vec<String>,
    #[serde(default)]
    pub schema_check: bool,
    #[serde(default)]
    pub notify: bool,
    pub cwd: Option<String>,
    pub max_output: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SchemaAddRequiredBody {
    pub pattern: String,
    #[serde(default)]
    pub warn_only: bool,
}

#[derive(Debug, Deserialize)]
pub struct SchemaAddRegexBody {
    pub pattern: String,
    pub regex: String,
    #[serde(default)]
    pub warn_only: bool,
}

#[derive(Debug, Deserialize)]
pub struct SchemaAddEnumBody {
    pub pattern: String,
    pub values: Vec<String>,
    #[serde(default)]
    pub warn_only: bool,
}

#[derive(Debug, Deserialize)]
pub struct SchemaRemoveBody {
    pub pattern: String,
}

#[derive(Debug, Serialize)]
pub struct VarsPayload {
    pub scope: EnvScope,
    pub vars: Vec<EnvVar>,
}

#[derive(Debug, Serialize)]
pub struct SnapshotPayload {
    pub snapshots: Vec<SnapshotMeta>,
}

#[derive(Debug, Serialize)]
pub struct SnapshotPrunePayload {
    pub removed: usize,
    pub remaining: usize,
}

#[derive(Debug, Serialize)]
pub struct DoctorPayload {
    pub report: DoctorReport,
}

#[derive(Debug, Serialize)]
pub struct DoctorFixPayload {
    pub result: DoctorFixResult,
}

#[derive(Debug, Serialize)]
pub struct ImportPayload {
    pub result: ImportApplyResult,
}

#[derive(Debug, Serialize)]
pub struct DiffPayload {
    pub diff: EnvDiff,
}

#[derive(Debug, Serialize)]
pub struct AuditPayload {
    pub entries: Vec<EnvAuditEntry>,
}

#[derive(Debug, Serialize)]
pub struct AnnotationsPayload {
    pub entries: Vec<AnnotationEntry>,
}

#[derive(Debug, Serialize)]
pub struct VarHistoryPayload {
    pub name: String,
    pub entries: Vec<EnvAuditEntry>,
}

#[derive(Debug, Serialize)]
pub struct GraphPayload {
    pub tree: EnvDepTree,
}

#[derive(Debug, Serialize)]
pub struct ProfilesPayload {
    pub profiles: Vec<EnvProfileMeta>,
}

#[derive(Debug, Serialize)]
pub struct SchemaPayload {
    pub schema: EnvSchema,
}

#[derive(Debug, Serialize)]
pub struct ValidatePayload {
    pub report: ValidationReport,
}

#[derive(Debug, Serialize)]
pub struct TemplatePayload {
    pub output: Option<String>,
    pub report: TemplateValidationReport,
}

#[derive(Debug, Serialize)]
pub struct RunPayload {
    pub result: RunCommandResult,
}

#[derive(Debug, Serialize)]
pub struct StatusPayload {
    pub status: EnvStatusSummary,
}
