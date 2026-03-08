use super::*;

pub(in crate::commands::dashboard) async fn env_ping() -> Json<ApiSuccess<serde_json::Value>> {
    ok(json!({
        "service": "env",
        "status": "ok"
    }))
}

pub(in crate::commands::dashboard) async fn env_status(Query(q): Query<ScopeQuery>) -> Response {
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

pub(in crate::commands::dashboard) async fn list_vars(Query(q): Query<ScopeQuery>) -> Response {
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

pub(in crate::commands::dashboard) async fn get_var(
    Path(name): Path<String>,
    Query(q): Query<ScopeQuery>,
) -> Response {
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

pub(in crate::commands::dashboard) async fn set_var(
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

pub(in crate::commands::dashboard) async fn delete_var(
    Path(name): Path<String>,
    Query(q): Query<ScopeQuery>,
) -> Response {
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

pub(in crate::commands::dashboard) async fn path_add(Json(body): Json<PathUpdateBody>) -> Response {
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

pub(in crate::commands::dashboard) async fn path_remove(
    Json(body): Json<PathUpdateBody>,
) -> Response {
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
