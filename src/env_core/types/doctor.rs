use serde::{Deserialize, Serialize};

use super::EnvScope;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorFixResult {
    pub scope: EnvScope,
    pub fixed: usize,
    #[serde(default)]
    pub details: Vec<String>,
}
