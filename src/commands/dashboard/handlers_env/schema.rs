use super::*;

pub(in crate::commands::dashboard) async fn schema_show() -> Response {
    let manager = manager();
    match manager.schema_show() {
        Ok(schema) => ok(SchemaPayload { schema }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(in crate::commands::dashboard) async fn schema_add_required(
    Json(body): Json<SchemaAddRequiredBody>,
) -> Response {
    let manager = manager();
    match manager.schema_add_required(&body.pattern, body.warn_only) {
        Ok(schema) => ok(SchemaPayload { schema }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(in crate::commands::dashboard) async fn schema_add_regex(
    Json(body): Json<SchemaAddRegexBody>,
) -> Response {
    let manager = manager();
    match manager.schema_add_regex(&body.pattern, &body.regex, body.warn_only) {
        Ok(schema) => ok(SchemaPayload { schema }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(in crate::commands::dashboard) async fn schema_add_enum(
    Json(body): Json<SchemaAddEnumBody>,
) -> Response {
    let manager = manager();
    match manager.schema_add_enum(&body.pattern, &body.values, body.warn_only) {
        Ok(schema) => ok(SchemaPayload { schema }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(in crate::commands::dashboard) async fn schema_remove(
    Json(body): Json<SchemaRemoveBody>,
) -> Response {
    let manager = manager();
    match manager.schema_remove(&body.pattern) {
        Ok(schema) => ok(SchemaPayload { schema }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(in crate::commands::dashboard) async fn schema_reset() -> Response {
    let manager = manager();
    match manager.schema_reset() {
        Ok(schema) => ok(SchemaPayload { schema }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(in crate::commands::dashboard) async fn validate(Json(body): Json<ValidateBody>) -> Response {
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

