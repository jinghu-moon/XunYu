use super::*;

pub(in crate::commands::dashboard) async fn audit_list(Query(q): Query<AuditQuery>) -> Response {
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

pub(in crate::commands::dashboard) async fn var_history(
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

pub(in crate::commands::dashboard) async fn annotations_list() -> Response {
    let manager = manager();
    match manager.annotate_list() {
        Ok(entries) => ok(AnnotationsPayload { entries }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(in crate::commands::dashboard) async fn annotation_get(Path(name): Path<String>) -> Response {
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

pub(in crate::commands::dashboard) async fn annotation_set(
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

pub(in crate::commands::dashboard) async fn annotation_delete(
    Path(name): Path<String>,
) -> Response {
    let manager = manager();
    match manager.annotate_delete(&name) {
        Ok(deleted) => ok(json!({ "name": name, "deleted": deleted })).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

