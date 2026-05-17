use super::*;

pub(in crate::commands::dashboard) async fn doctor_run(Json(body): Json<DoctorBody>) -> Response {
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

pub(in crate::commands::dashboard) async fn doctor_fix(Json(body): Json<DoctorBody>) -> Response {
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

