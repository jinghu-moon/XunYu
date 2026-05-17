use super::*;

pub(in crate::commands::dashboard) async fn import_vars(Json(body): Json<ImportBody>) -> Response {
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

pub(in crate::commands::dashboard) async fn export_vars(Query(q): Query<ExportQuery>) -> Response {
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

pub(in crate::commands::dashboard) async fn export_all(Query(q): Query<ScopeQuery>) -> Response {
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

pub(in crate::commands::dashboard) async fn export_live(
    Query(q): Query<ExportLiveQuery>,
) -> Response {
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

pub(in crate::commands::dashboard) async fn template_expand(
    Json(body): Json<TemplateExpandBody>,
) -> Response {
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

pub(in crate::commands::dashboard) async fn run_command(Json(body): Json<RunBody>) -> Response {
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

