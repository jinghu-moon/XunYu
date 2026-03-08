use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};

use axum::Json;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::json;
use tokio::sync::broadcast;

use crate::commands::env::web_dto::{
    AnnotationBody, AnnotationsPayload, ApiError, ApiSuccess, AuditPayload, AuditQuery,
    DiffPayload, DiffQuery, DoctorBody, DoctorFixPayload, DoctorPayload, ExportLiveQuery,
    ExportQuery, GraphPayload, GraphQuery, ImportBody, ImportPayload, PathUpdateBody, ProfileBody,
    ProfileCaptureBody, ProfilesPayload, RunBody, RunPayload, SchemaAddEnumBody,
    SchemaAddRegexBody, SchemaAddRequiredBody, SchemaPayload, SchemaRemoveBody, ScopeQuery,
    SetVarBody, SnapshotCreateBody, SnapshotPayload, SnapshotRestoreBody, StatusPayload,
    SnapshotPrunePayload, SnapshotPruneQuery, TemplateExpandBody, TemplatePayload, ValidateBody,
    ValidatePayload, VarHistoryPayload, VarHistoryQuery, VarsPayload,
};
use crate::env_core::EnvManager;
use crate::env_core::types::{
    EnvError, EnvEvent, EnvScope, ExportFormat, ImportStrategy, LiveExportFormat,
};

fn manager() -> EnvManager {
    let tx = env_event_sender().clone();
    EnvManager::new().with_event_callback(Arc::new(move |event: EnvEvent| {
        if let Ok(payload) = serde_json::to_string(&event) {
            let _ = tx.send(payload);
        }
    }))
}

fn env_event_sender() -> &'static broadcast::Sender<String> {
    static TX: OnceLock<broadcast::Sender<String>> = OnceLock::new();
    TX.get_or_init(|| {
        let (tx, _) = broadcast::channel(256);
        tx
    })
}

fn ok<T: Serialize>(data: T) -> Json<ApiSuccess<T>> {
    Json(ApiSuccess { ok: true, data })
}

fn resolve_scope(value: Option<String>, default: EnvScope) -> Result<EnvScope, Response> {
    match value {
        Some(v) => EnvScope::from_str(&v).map_err(|e| map_env_error(e).into_response()),
        None => Ok(default),
    }
}

fn parse_set_pairs(items: &[String]) -> Result<Vec<(String, String)>, EnvError> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some((name, value)) = item.split_once('=') else {
            return Err(EnvError::InvalidInput(format!(
                "invalid set item '{}', expected KEY=VALUE",
                item
            )));
        };
        let key = name.trim();
        if key.is_empty() {
            return Err(EnvError::InvalidInput(format!(
                "invalid set item '{}': empty key",
                item
            )));
        }
        out.push((key.to_string(), value.to_string()));
    }
    Ok(out)
}

fn run_api_enabled(cfg_allow: bool) -> bool {
    if cfg_allow {
        return true;
    }
    matches!(
        std::env::var("ENVMGR_ALLOW_RUN"),
        Ok(v) if v == "1" || v.eq_ignore_ascii_case("true")
    )
}

fn map_env_error(err: EnvError) -> (StatusCode, Json<ApiError>) {
    let status = match &err {
        EnvError::InvalidInput(_) | EnvError::ScopeNotWritable(_) => StatusCode::BAD_REQUEST,
        EnvError::PermissionDenied(_) => StatusCode::FORBIDDEN,
        EnvError::NotFound(_) => StatusCode::NOT_FOUND,
        EnvError::UnsupportedPlatform => StatusCode::NOT_IMPLEMENTED,
        EnvError::Other(msg) if msg.starts_with("schema-check failed:") => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let code = match &err {
        EnvError::InvalidInput(_) => "env.invalid_input",
        EnvError::ScopeNotWritable(_) => "env.invalid_scope",
        EnvError::PermissionDenied(_) => "env.permission_denied",
        EnvError::NotFound(_) => "env.not_found",
        EnvError::UnsupportedPlatform => "env.unsupported_platform",
        EnvError::LockFailed(_) => "env.lock_failed",
        EnvError::Other(msg) if msg.starts_with("schema-check failed:") => "env.invalid_input",
        _ => "env.internal_error",
    };
    (
        status,
        Json(ApiError {
            ok: false,
            code: code.to_string(),
            message: err.to_string(),
            details: None,
        }),
    )
}

pub(super) async fn env_ping() -> Json<ApiSuccess<serde_json::Value>> {
    ok(json!({
        "service": "env",
        "status": "ok"
    }))
}

pub(super) async fn env_status(Query(q): Query<ScopeQuery>) -> Response {
    let scope = match resolve_scope(q.scope, EnvScope::All) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    match manager.status_overview(scope) {
        Ok(status) => ok(StatusPayload { status }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn list_vars(Query(q): Query<ScopeQuery>) -> Response {
    let scope = match resolve_scope(q.scope, EnvScope::User) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    match manager.list_vars(scope) {
        Ok(vars) => ok(VarsPayload { scope, vars }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn get_var(Path(name): Path<String>, Query(q): Query<ScopeQuery>) -> Response {
    let scope = match resolve_scope(q.scope, EnvScope::User) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    match manager.get_var(scope, &name) {
        Ok(Some(var)) => ok(var).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                ok: false,
                code: "env.not_found".to_string(),
                message: format!("variable '{}' not found", name),
                details: None,
            }),
        )
            .into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn set_var(
    Path(name): Path<String>,
    Query(q): Query<ScopeQuery>,
    Json(body): Json<SetVarBody>,
) -> Response {
    let scope = match resolve_scope(q.scope, EnvScope::User) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    match manager.set_var(scope, &name, &body.value, body.no_snapshot) {
        Ok(_) => ok(json!({ "scope": scope, "name": name })).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn delete_var(Path(name): Path<String>, Query(q): Query<ScopeQuery>) -> Response {
    let scope = match resolve_scope(q.scope, EnvScope::User) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    match manager.delete_var(scope, &name) {
        Ok(deleted) => {
            ok(json!({ "scope": scope, "name": name, "deleted": deleted })).into_response()
        }
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn path_add(Json(body): Json<PathUpdateBody>) -> Response {
    let scope = match resolve_scope(body.scope, EnvScope::User) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    match manager.path_add(scope, &body.entry, body.head) {
        Ok(changed) => {
            ok(json!({ "scope": scope, "entry": body.entry, "changed": changed })).into_response()
        }
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn path_remove(Json(body): Json<PathUpdateBody>) -> Response {
    let scope = match resolve_scope(body.scope, EnvScope::User) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    match manager.path_remove(scope, &body.entry) {
        Ok(changed) => {
            ok(json!({ "scope": scope, "entry": body.entry, "changed": changed })).into_response()
        }
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn list_snapshots() -> Response {
    let manager = manager();
    match manager.snapshot_list() {
        Ok(snapshots) => ok(SnapshotPayload { snapshots }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn create_snapshot(Json(body): Json<SnapshotCreateBody>) -> Response {
    let manager = manager();
    match manager.snapshot_create(body.desc.as_deref()) {
        Ok(meta) => ok(meta).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn prune_snapshots(Query(q): Query<SnapshotPruneQuery>) -> Response {
    let keep = q.keep.unwrap_or(50).min(10_000);
    let manager = manager();
    match manager.snapshot_prune(keep) {
        Ok(removed) => match manager.snapshot_list() {
            Ok(items) => ok(SnapshotPrunePayload {
                removed,
                remaining: items.len(),
            })
            .into_response(),
            Err(e) => map_env_error(e).into_response(),
        },
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn restore_snapshot(Json(body): Json<SnapshotRestoreBody>) -> Response {
    let scope = match resolve_scope(body.scope, EnvScope::All) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    match manager.snapshot_restore(scope, body.id.as_deref(), body.latest) {
        Ok(meta) => ok(meta).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn doctor_run(Json(body): Json<DoctorBody>) -> Response {
    let scope = match resolve_scope(body.scope, EnvScope::All) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    match manager.doctor_run(scope) {
        Ok(report) => ok(DoctorPayload { report }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn doctor_fix(Json(body): Json<DoctorBody>) -> Response {
    let scope = match resolve_scope(body.scope, EnvScope::All) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    match manager.doctor_fix(scope) {
        Ok(result) => ok(DoctorFixPayload { result }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn import_vars(Json(body): Json<ImportBody>) -> Response {
    let scope = match resolve_scope(body.scope, EnvScope::User) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let mode = match body.mode.as_deref() {
        Some(raw) => match ImportStrategy::from_str(raw) {
            Ok(v) => v,
            Err(e) => return map_env_error(e).into_response(),
        },
        None => ImportStrategy::Merge,
    };
    let manager = manager();
    match manager.import_content(scope, &body.content, mode, body.dry_run) {
        Ok(result) => ok(ImportPayload { result }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn export_vars(Query(q): Query<ExportQuery>) -> Response {
    let scope = match resolve_scope(q.scope, EnvScope::User) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let format = match q.format {
        Some(raw) => match ExportFormat::from_str(&raw) {
            Ok(v) => v,
            Err(e) => return map_env_error(e).into_response(),
        },
        None => ExportFormat::Json,
    };
    let manager = manager();
    match manager.export_vars(scope, format) {
        Ok(data) => {
            let content_type = match format {
                ExportFormat::Json => "application/json; charset=utf-8",
                ExportFormat::Csv => "text/csv; charset=utf-8",
                _ => "text/plain; charset=utf-8",
            };
            ([(header::CONTENT_TYPE, content_type)], data).into_response()
        }
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn export_all(Query(q): Query<ScopeQuery>) -> Response {
    let scope = match resolve_scope(q.scope, EnvScope::All) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    match manager.export_bundle(scope) {
        Ok(bytes) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/zip"),
            );
            let cd = format!("attachment; filename=\"xun-env-{}.zip\"", scope);
            match HeaderValue::from_str(&cd) {
                Ok(v) => {
                    headers.insert(header::CONTENT_DISPOSITION, v);
                    (headers, bytes).into_response()
                }
                Err(_) => map_env_error(EnvError::Other("invalid header value".to_string()))
                    .into_response(),
            }
        }
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn export_live(Query(q): Query<ExportLiveQuery>) -> Response {
    let scope = match resolve_scope(q.scope, EnvScope::All) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let format = match q.format {
        Some(raw) => match LiveExportFormat::from_str(&raw) {
            Ok(v) => v,
            Err(e) => return map_env_error(e).into_response(),
        },
        None => LiveExportFormat::Dotenv,
    };
    let manager = manager();
    match manager.export_live(scope, format, &[], &[]) {
        Ok(data) => {
            let content_type = match format {
                LiveExportFormat::Json => "application/json; charset=utf-8",
                LiveExportFormat::Sh => "text/x-shellscript; charset=utf-8",
                LiveExportFormat::Dotenv | LiveExportFormat::Reg => "text/plain; charset=utf-8",
            };
            ([(header::CONTENT_TYPE, content_type)], data).into_response()
        }
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn diff_live(Query(q): Query<DiffQuery>) -> Response {
    let scope = match resolve_scope(q.scope, EnvScope::User) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    if q.snapshot.is_some() && q.since.is_some() {
        return map_env_error(EnvError::InvalidInput(
            "diff-live does not allow using snapshot and since together".to_string(),
        ))
        .into_response();
    }
    let manager = manager();
    let result = if let Some(since) = q.since.as_deref() {
        manager.diff_since(scope, since)
    } else {
        manager.diff_live(scope, q.snapshot.as_deref())
    };
    match result {
        Ok(diff) => {
            if q.color {
                let text = crate::env_core::diff::format_diff(&diff, true);
                return ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], text)
                    .into_response();
            }
            ok(DiffPayload { diff }).into_response()
        }
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn dependency_graph(Query(q): Query<GraphQuery>) -> Response {
    let scope = match resolve_scope(q.scope, EnvScope::All) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let max_depth = q.max_depth.unwrap_or(8).clamp(1, 64);
    let manager = manager();
    match manager.dependency_tree(scope, &q.name, max_depth) {
        Ok(tree) => ok(GraphPayload { tree }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn audit_list(Query(q): Query<AuditQuery>) -> Response {
    let limit = q.limit.unwrap_or(100).min(5000);
    let manager = manager();
    match manager.audit_list(limit) {
        Ok(mut entries) => {
            entries.reverse();
            ok(AuditPayload { entries }).into_response()
        }
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn var_history(
    Path(name): Path<String>,
    Query(q): Query<VarHistoryQuery>,
) -> Response {
    let limit = q.limit.unwrap_or(50).min(5000);
    let manager = manager();
    match manager.audit_list(0) {
        Ok(entries) => {
            let mut filtered = entries
                .into_iter()
                .filter(|item| {
                    item.name
                        .as_ref()
                        .map(|n| n.eq_ignore_ascii_case(&name))
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>();
            if limit > 0 && filtered.len() > limit {
                let keep_from = filtered.len() - limit;
                filtered = filtered.split_off(keep_from);
            }
            filtered.reverse();
            ok(VarHistoryPayload {
                name,
                entries: filtered,
            })
            .into_response()
        }
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn annotations_list() -> Response {
    let manager = manager();
    match manager.annotate_list() {
        Ok(entries) => ok(AnnotationsPayload { entries }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn annotation_get(Path(name): Path<String>) -> Response {
    let manager = manager();
    match manager.annotate_get(&name) {
        Ok(Some(entry)) => ok(entry).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                ok: false,
                code: "env.not_found".to_string(),
                message: format!("annotation '{}' not found", name),
                details: None,
            }),
        )
            .into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn annotation_set(
    Path(name): Path<String>,
    Json(body): Json<AnnotationBody>,
) -> Response {
    let note = body.note.trim();
    if note.is_empty() {
        return map_env_error(EnvError::InvalidInput(
            "annotation note cannot be empty".to_string(),
        ))
        .into_response();
    }
    let manager = manager();
    match manager.annotate_set(&name, note) {
        Ok(entry) => ok(entry).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn annotation_delete(Path(name): Path<String>) -> Response {
    let manager = manager();
    match manager.annotate_delete(&name) {
        Ok(deleted) => ok(json!({ "name": name, "deleted": deleted })).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn list_profiles() -> Response {
    let manager = manager();
    match manager.profile_list() {
        Ok(profiles) => ok(ProfilesPayload { profiles }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn capture_profile(
    Path(name): Path<String>,
    Json(body): Json<ProfileCaptureBody>,
) -> Response {
    let scope = match resolve_scope(body.scope, EnvScope::User) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    match manager.profile_capture(&name, scope) {
        Ok(meta) => ok(meta).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn apply_profile(
    Path(name): Path<String>,
    Json(body): Json<ProfileBody>,
) -> Response {
    let scope = match body.scope {
        Some(raw) => match EnvScope::from_str(&raw) {
            Ok(v) => Some(v),
            Err(e) => return map_env_error(e).into_response(),
        },
        None => None,
    };
    let manager = manager();
    match manager.profile_apply(&name, scope) {
        Ok(meta) => ok(meta).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn profile_diff(
    Path(name): Path<String>,
    Query(q): Query<ScopeQuery>,
) -> Response {
    let scope = match q.scope {
        Some(raw) => match EnvScope::from_str(&raw) {
            Ok(v) => Some(v),
            Err(e) => return map_env_error(e).into_response(),
        },
        None => None,
    };
    let manager = manager();
    match manager.profile_diff(&name, scope) {
        Ok(diff) => ok(DiffPayload { diff }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn delete_profile(Path(name): Path<String>) -> Response {
    let manager = manager();
    match manager.profile_delete(&name) {
        Ok(deleted) => ok(json!({ "name": name, "deleted": deleted })).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn schema_show() -> Response {
    let manager = manager();
    match manager.schema_show() {
        Ok(schema) => ok(SchemaPayload { schema }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn schema_add_required(Json(body): Json<SchemaAddRequiredBody>) -> Response {
    let manager = manager();
    match manager.schema_add_required(&body.pattern, body.warn_only) {
        Ok(schema) => ok(SchemaPayload { schema }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn schema_add_regex(Json(body): Json<SchemaAddRegexBody>) -> Response {
    let manager = manager();
    match manager.schema_add_regex(&body.pattern, &body.regex, body.warn_only) {
        Ok(schema) => ok(SchemaPayload { schema }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn schema_add_enum(Json(body): Json<SchemaAddEnumBody>) -> Response {
    let manager = manager();
    match manager.schema_add_enum(&body.pattern, &body.values, body.warn_only) {
        Ok(schema) => ok(SchemaPayload { schema }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn schema_remove(Json(body): Json<SchemaRemoveBody>) -> Response {
    let manager = manager();
    match manager.schema_remove(&body.pattern) {
        Ok(schema) => ok(SchemaPayload { schema }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn schema_reset() -> Response {
    let manager = manager();
    match manager.schema_reset() {
        Ok(schema) => ok(SchemaPayload { schema }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn validate(Json(body): Json<ValidateBody>) -> Response {
    let scope = match resolve_scope(body.scope, EnvScope::All) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    match manager.validate_schema(scope, body.strict) {
        Ok(report) => ok(ValidatePayload { report }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn template_expand(Json(body): Json<TemplateExpandBody>) -> Response {
    let scope = match resolve_scope(body.scope, EnvScope::All) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let manager = manager();
    if body.validate_only {
        return match manager.template_validate(scope, &body.template) {
            Ok(report) => ok(TemplatePayload {
                output: None,
                report,
            })
            .into_response(),
            Err(e) => map_env_error(e).into_response(),
        };
    }
    match manager.template_expand(scope, &body.template) {
        Ok(result) => ok(TemplatePayload {
            output: Some(result.expanded),
            report: result.report,
        })
        .into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn run_command(Json(body): Json<RunBody>) -> Response {
    let manager = manager();
    if !run_api_enabled(manager.config().allow_run) {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiError {
                ok: false,
                code: "env.run_disabled".to_string(),
                message:
                    "run endpoint disabled; set env config allow_run=true or ENVMGR_ALLOW_RUN=1"
                        .to_string(),
                details: None,
            }),
        )
            .into_response();
    }

    let RunBody {
        cmd,
        scope: scope_raw,
        env_files,
        set,
        schema_check,
        notify,
        cwd: cwd_raw,
        max_output,
    } = body;

    let scope = match resolve_scope(scope_raw, EnvScope::All) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let set_pairs = match parse_set_pairs(&set) {
        Ok(v) => v,
        Err(e) => return map_env_error(e).into_response(),
    };
    let env_files = env_files
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<PathBuf>>();
    let cwd = match cwd_raw {
        Some(raw) => {
            let path = PathBuf::from(raw);
            if !path.is_dir() {
                return map_env_error(EnvError::InvalidInput(
                    "cwd must be an existing directory".to_string(),
                ))
                .into_response();
            }
            Some(path)
        }
        None => None,
    };
    let max_output = max_output.unwrap_or(64 * 1024).clamp(1024, 1024 * 1024);

    match manager.run_command(
        scope,
        &env_files,
        &set_pairs,
        &cmd,
        cwd.as_deref(),
        schema_check,
        notify,
        true,
        max_output,
    ) {
        Ok(result) => ok(RunPayload { result }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(super) async fn env_ws(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| async move {
        handle_env_ws(socket).await;
    })
}

async fn handle_env_ws(mut socket: WebSocket) {
    let mut rx = env_event_sender().subscribe();
    let _ = socket
        .send(Message::Text(
            r#"{"type":"connected","channel":"env"}"#.into(),
        ))
        .await;

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(msg) => {
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        if socket.send(Message::Text(r#"{"type":"env.refresh"}"#.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
}
