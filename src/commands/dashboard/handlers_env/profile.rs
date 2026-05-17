use super::*;

pub(in crate::commands::dashboard) async fn list_profiles() -> Response {
    let manager = manager();
    match manager.profile_list() {
        Ok(profiles) => ok(ProfilesPayload { profiles }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(in crate::commands::dashboard) async fn capture_profile(
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

pub(in crate::commands::dashboard) async fn apply_profile(
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

pub(in crate::commands::dashboard) async fn profile_diff(
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

pub(in crate::commands::dashboard) async fn delete_profile(Path(name): Path<String>) -> Response {
    let manager = manager();
    match manager.profile_delete(&name) {
        Ok(deleted) => ok(json!({ "name": name, "deleted": deleted })).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

