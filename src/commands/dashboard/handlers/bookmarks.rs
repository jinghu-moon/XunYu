use super::*;

// --- Bookmarks ---

pub(in crate::commands::dashboard) async fn list_bookmarks() -> Json<Vec<ListItem>> {
    let db = store::load(&store::db_path());
    let items: Vec<ListItem> = db
        .into_iter()
        .map(|(name, e)| ListItem {
            name,
            path: e.path,
            tags: e.tags,
            visits: e.visit_count,
            last_visited: e.last_visited,
        })
        .collect();
    Json(items)
}

#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct BookmarksExportQuery {
    format: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct BookmarksImportQuery {
    format: Option<String>,
    mode: Option<String>,
}

#[derive(Serialize)]
pub(in crate::commands::dashboard) struct BookmarksImportResult {
    added: usize,
    updated: usize,
    total: usize,
}

pub(in crate::commands::dashboard) async fn export_bookmarks(
    Query(q): Query<BookmarksExportQuery>,
) -> Response {
    let raw_format = q.format.as_deref().unwrap_or("json");
    let Some(format) = parse_io_format(raw_format) else {
        return (StatusCode::BAD_REQUEST, "invalid format").into_response();
    };

    let db = store::load(&store::db_path());
    let mut items: Vec<ListItem> = db
        .iter()
        .map(|(name, entry)| ListItem {
            name: name.clone(),
            path: entry.path.clone(),
            tags: entry.tags.clone(),
            visits: entry.visit_count,
            last_visited: entry.last_visited,
        })
        .collect();
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    match format {
        IoFormat::Json => Json(items).into_response(),
        IoFormat::Tsv => {
            let mut out = String::new();
            for item in items {
                out.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\n",
                    item.name,
                    item.path,
                    item.tags.join(","),
                    item.visits,
                    item.last_visited
                ));
            }
            ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], out).into_response()
        }
    }
}

pub(in crate::commands::dashboard) async fn import_bookmarks(
    Query(q): Query<BookmarksImportQuery>,
    body: String,
) -> Response {
    let raw_format = q.format.as_deref().unwrap_or("json");
    let Some(format) = parse_io_format(raw_format) else {
        return (StatusCode::BAD_REQUEST, "invalid format").into_response();
    };
    let raw_mode = q.mode.as_deref().unwrap_or("merge");
    let Some(mode) = parse_import_mode(raw_mode) else {
        return (StatusCode::BAD_REQUEST, "invalid mode").into_response();
    };

    let mut items: Vec<ListItem> = Vec::new();
    match format {
        IoFormat::Json => {
            let parsed: Vec<ListItem> = match serde_json::from_str(&body) {
                Ok(v) => v,
                Err(e) => {
                    return (StatusCode::BAD_REQUEST, format!("import json error: {e}"))
                        .into_response();
                }
            };
            items.extend(parsed);
        }
        IoFormat::Tsv => {
            for line in body.lines() {
                let cols: Vec<&str> = line.split('\t').collect();
                if cols.len() < 2 {
                    continue;
                }
                let name = cols[0].trim().to_string();
                let path = cols[1].trim().to_string();
                if name.is_empty() || path.is_empty() {
                    continue;
                }
                let tags = if cols.len() > 2 {
                    parse_tags(cols[2])
                } else {
                    Vec::new()
                };
                let visits = if cols.len() > 3 {
                    cols[3].trim().parse::<u32>().unwrap_or(0)
                } else {
                    0
                };
                let last_visited = if cols.len() > 4 {
                    cols[4].trim().parse::<u64>().unwrap_or(0)
                } else {
                    0
                };
                items.push(ListItem {
                    name,
                    path,
                    tags,
                    visits,
                    last_visited,
                });
            }
        }
    }

    if items.is_empty() {
        return Json(BookmarksImportResult {
            added: 0,
            updated: 0,
            total: 0,
        })
        .into_response();
    }

    let db_path = store::db_path();
    let Some(_lock) = common::try_acquire_lock(&db_path) else {
        return StatusCode::CONFLICT.into_response();
    };
    let mut db = store::load(&db_path);

    let mut added = 0usize;
    let mut updated = 0usize;
    for item in items {
        let entry = Entry {
            path: item.path.clone(),
            tags: item.tags.clone(),
            visit_count: item.visits,
            last_visited: item.last_visited,
        };
        if let Some(existing) = db.get_mut(&item.name) {
            match mode {
                ImportMode::Merge => {
                    if !item.path.is_empty() {
                        existing.path = item.path.clone();
                    }
                    let mut seen: HashSet<String> =
                        existing.tags.iter().map(|t| t.to_lowercase()).collect();
                    for t in item.tags {
                        if seen.insert(t.to_lowercase()) {
                            existing.tags.push(t);
                        }
                    }
                    existing.visit_count = existing.visit_count.max(item.visits);
                    existing.last_visited = existing.last_visited.max(item.last_visited);
                }
                ImportMode::Overwrite => {
                    *existing = entry;
                }
            }
            updated += 1;
        } else {
            db.insert(item.name.clone(), entry);
            added += 1;
        }
    }

    match store::save_db(&db_path, &db) {
        Ok(_) => Json(BookmarksImportResult {
            added,
            updated,
            total: added + updated,
        })
        .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct BookmarkBody {
    path: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct RenameBody {
    #[serde(rename = "newName")]
    new_name: String,
}

pub(in crate::commands::dashboard) async fn upsert_bookmark(
    Path(name): Path<String>,
    Json(body): Json<BookmarkBody>,
) -> StatusCode {
    let db_path = store::db_path();
    let Some(_lock) = common::try_acquire_lock(&db_path) else {
        return StatusCode::CONFLICT;
    };
    let mut db = store::load(&db_path);
    let entry = db.entry(name).or_default();
    entry.path = body.path;
    entry.tags = body.tags;
    match store::save_db(&db_path, &db) {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub(in crate::commands::dashboard) async fn delete_bookmark(
    Path(name): Path<String>,
) -> StatusCode {
    let db_path = store::db_path();
    let Some(_lock) = common::try_acquire_lock(&db_path) else {
        return StatusCode::CONFLICT;
    };
    let mut db = store::load(&db_path);
    if db.remove(&name).is_none() {
        return StatusCode::NOT_FOUND;
    }
    match store::save_db(&db_path, &db) {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub(in crate::commands::dashboard) async fn rename_bookmark(
    Path(name): Path<String>,
    Json(body): Json<RenameBody>,
) -> Response {
    let new_name = body.new_name.trim();
    if new_name.is_empty() {
        return (StatusCode::BAD_REQUEST, "newName is empty").into_response();
    }
    if new_name == name {
        return (StatusCode::BAD_REQUEST, "newName equals old name").into_response();
    }

    let db_path = store::db_path();
    let Some(_lock) = common::try_acquire_lock(&db_path) else {
        return StatusCode::CONFLICT.into_response();
    };
    let mut db = store::load(&db_path);

    if !db.contains_key(&name) {
        return (StatusCode::NOT_FOUND, "bookmark not found").into_response();
    }
    if db.contains_key(new_name) {
        return (StatusCode::CONFLICT, "bookmark already exists").into_response();
    }

    let entry = match db.remove(&name) {
        Some(v) => v,
        None => return (StatusCode::NOT_FOUND, "bookmark not found").into_response(),
    };
    let item = ListItem {
        name: new_name.to_string(),
        path: entry.path.clone(),
        tags: entry.tags.clone(),
        visits: entry.visit_count,
        last_visited: entry.last_visited,
    };
    db.insert(new_name.to_string(), entry);

    match store::save_db(&db_path, &db) {
        Ok(_) => Json(item).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct BookmarksBatchRequest {
    op: String,
    #[serde(default)]
    names: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
}

pub(in crate::commands::dashboard) async fn bookmarks_batch(
    Json(req): Json<BookmarksBatchRequest>,
) -> Response {
    let db_path = store::db_path();
    let Some(_lock) = common::try_acquire_lock(&db_path) else {
        return StatusCode::CONFLICT.into_response();
    };

    let mut db = store::load(&db_path);

    if req.op.eq_ignore_ascii_case("delete") {
        let mut deleted = 0usize;
        for n in req.names {
            if db.remove(&n).is_some() {
                deleted += 1;
            }
        }
        if let Err(e) = store::save_db(&db_path, &db) {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
        return Json(serde_json::json!({ "deleted": deleted })).into_response();
    }

    if req.op.eq_ignore_ascii_case("add_tags") || req.op.eq_ignore_ascii_case("remove_tags") {
        let add = req.op.eq_ignore_ascii_case("add_tags");
        let tags: Vec<String> = req
            .tags
            .into_iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        if tags.is_empty() || req.names.is_empty() {
            return (StatusCode::BAD_REQUEST, "names/tags is empty").into_response();
        }

        let mut updated = 0usize;
        for n in req.names {
            let Some(e) = db.get_mut(&n) else { continue };
            if add {
                for t in &tags {
                    if !e.tags.iter().any(|x| x == t) {
                        e.tags.push(t.clone());
                    }
                }
            } else {
                e.tags.retain(|x| !tags.iter().any(|t| t == x));
            }
            updated += 1;
        }

        if let Err(e) = store::save_db(&db_path, &db) {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
        return Json(serde_json::json!({ "updated": updated })).into_response();
    }

    (StatusCode::BAD_REQUEST, "unknown op").into_response()
}
