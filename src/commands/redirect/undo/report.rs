#[derive(Debug, Clone)]
pub(super) struct AuditEntry {
    pub(super) action: String,
    pub(super) target: String,
    pub(super) params: serde_json::Value,
    pub(super) result: String,
    pub(super) reason: String,
}

#[derive(Clone)]
pub(crate) struct UndoResult {
    pub(crate) action: String,
    pub(crate) src: String,
    pub(crate) dst: String,
    pub(crate) result: String,
    pub(crate) reason: String,
}
