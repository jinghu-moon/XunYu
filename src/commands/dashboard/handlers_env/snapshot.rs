use super::*;

pub(in crate::commands::dashboard) async fn list_snapshots() -> Response {
    let manager = manager();
    match manager.snapshot_list() {
        Ok(snapshots) => ok(SnapshotPayload { snapshots }).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(in crate::commands::dashboard) async fn create_snapshot(
    Json(body): Json<SnapshotCreateBody>,
) -> Response {
    let manager = manager();
    match manager.snapshot_create(body.desc.as_deref()) {
        Ok(meta) => ok(meta).into_response(),
        Err(e) => map_env_error(e).into_response(),
    }
}

pub(in crate::commands::dashboard) async fn prune_snapshots(
    Query(q): Query<SnapshotPruneQuery>,
) -> Response {
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

pub(in crate::commands::dashboard) async fn restore_snapshot(
    Json(body): Json<SnapshotRestoreBody>,
) -> Response {
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

pub(in crate::commands::dashboard) async fn diff_live(Query(q): Query<DiffQuery>) -> Response {
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

pub(in crate::commands::dashboard) async fn dependency_graph(
    Query(q): Query<GraphQuery>,
) -> Response {
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

