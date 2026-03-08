use super::*;

// --- Files (for diff file browser) ---

#[cfg(feature = "diff")]
#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct FilesQuery {
    path: String,
}

#[cfg(feature = "diff")]
#[derive(Serialize)]
pub(in crate::commands::dashboard) struct FileEntry {
    name: String,
    is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
}

// SECURITY: /api/files 允许读取任意本地目录。当前 Dashboard 仅绑定 127.0.0.1，
// 风险可控。若未来需要开放网络访问，必须增加路径白名单 / 沙箱机制。
#[cfg(feature = "diff")]
pub(in crate::commands::dashboard) async fn list_files(
    State(state): State<super::super::super::DashboardState>,
    Query(q): Query<FilesQuery>,
) -> Response {
    let dir = std::path::Path::new(&q.path);
    if !dir.is_dir() {
        return (
            StatusCode::NOT_FOUND,
            format!("not a directory: {}", q.path),
        )
            .into_response();
    }

    state.request_watch_path(dir);

    let entries = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("read_dir: {e}")).into_response();
        }
    };

    let mut dirs: Vec<FileEntry> = Vec::new();
    let mut files: Vec<FileEntry> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let meta = entry.metadata().ok();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = if is_dir {
            None
        } else {
            meta.as_ref().map(|m| m.len())
        };
        let fe = FileEntry { name, is_dir, size };
        if is_dir {
            dirs.push(fe);
        } else {
            files.push(fe);
        }
    }

    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    dirs.append(&mut files);

    Json(dirs).into_response()
}

#[cfg(feature = "diff")]
const FILE_SEARCH_DEFAULT_LIMIT: usize = 200;

#[cfg(feature = "diff")]
const FILE_SEARCH_MAX_LIMIT: usize = 1000;

#[cfg(feature = "diff")]
const FILE_SEARCH_MAX_SCANNED: usize = 50000;

#[cfg(feature = "diff")]
#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct FileSearchQuery {
    root: String,
    query: String,
    #[serde(default)]
    limit: Option<usize>,
}

#[cfg(feature = "diff")]
#[derive(Serialize)]
pub(in crate::commands::dashboard) struct FileSearchEntry {
    path: String,
    name: String,
    is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
}

#[cfg(feature = "diff")]
#[derive(Clone, Copy)]
enum FileSearchMatchMode {
    NameOnly,
    NameOrPath,
}

#[cfg(feature = "diff")]
fn resolve_file_search_mode(needle: &str) -> FileSearchMatchMode {
    if needle.contains('/') || needle.contains('\\') || needle.contains(':') {
        FileSearchMatchMode::NameOrPath
    } else {
        FileSearchMatchMode::NameOnly
    }
}

#[cfg(feature = "diff")]
struct FileSearchRunStats {
    list: Vec<FileSearchEntry>,
    scanned: usize,
    matched: usize,
    scan_ms: u128,
    sort_ms: u128,
}

#[cfg(feature = "diff")]
fn search_dir_parallel(
    dir: std::path::PathBuf,
    needle: &str,
    path_needle_alt: Option<&str>,
    mode: FileSearchMatchMode,
    limit: usize,
    scanned: &AtomicUsize,
    matched: &AtomicUsize,
    stop: &AtomicBool,
    out: &Mutex<Vec<FileSearchEntry>>,
) {
    if stop.load(Ordering::Relaxed) {
        return;
    }

    let entries = match std::fs::read_dir(&dir) {
        Ok(v) => v,
        Err(_) => return,
    };

    let mut subdirs: Vec<std::path::PathBuf> = Vec::new();

    for entry in entries.flatten() {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        let prev = scanned.fetch_add(1, Ordering::Relaxed);
        if prev >= FILE_SEARCH_MAX_SCANNED {
            stop.store(true, Ordering::Relaxed);
            break;
        }

        let file_type = match entry.file_type() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let is_dir = file_type.is_dir();
        let path = entry.path();
        if is_dir {
            subdirs.push(path.clone());
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let name_lc = name.to_ascii_lowercase();
        let is_matched = if name_lc.contains(needle) {
            true
        } else {
            match mode {
                FileSearchMatchMode::NameOnly => false,
                FileSearchMatchMode::NameOrPath => {
                    let path_lc = path.to_string_lossy().to_ascii_lowercase();
                    path_lc.contains(needle)
                        || path_needle_alt
                            .map(|alt| path_lc.contains(alt))
                            .unwrap_or(false)
                }
            }
        };
        if !is_matched {
            continue;
        }
        matched.fetch_add(1, Ordering::Relaxed);

        let size = if is_dir {
            None
        } else {
            entry.metadata().ok().map(|m| m.len())
        };
        let path_str = path.to_string_lossy().to_string();

        let mut guard = out.lock().unwrap_or_else(|e| e.into_inner());
        if guard.len() < limit {
            guard.push(FileSearchEntry {
                path: path_str,
                name,
                is_dir,
                size,
            });
            if guard.len() >= limit {
                stop.store(true, Ordering::Relaxed);
            }
        } else {
            stop.store(true, Ordering::Relaxed);
        }
    }

    if stop.load(Ordering::Relaxed) {
        return;
    }

    use rayon::prelude::*;
    subdirs.into_par_iter().for_each(|child| {
        search_dir_parallel(
            child,
            needle,
            path_needle_alt,
            mode,
            limit,
            scanned,
            matched,
            stop,
            out,
        );
    });
}

#[cfg(feature = "diff")]
fn run_parallel_file_search(
    root: std::path::PathBuf,
    needle: String,
    limit: usize,
    mode: FileSearchMatchMode,
) -> FileSearchRunStats {
    let scan_started = Instant::now();
    let scanned = AtomicUsize::new(0);
    let matched = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let out = Mutex::new(Vec::<FileSearchEntry>::with_capacity(limit.min(128)));
    let path_needle_alt = match mode {
        FileSearchMatchMode::NameOnly => None,
        FileSearchMatchMode::NameOrPath => {
            if needle.contains('/') {
                Some(needle.replace('/', "\\"))
            } else if needle.contains('\\') {
                Some(needle.replace('\\', "/"))
            } else {
                None
            }
        }
    };

    search_dir_parallel(
        root,
        &needle,
        path_needle_alt.as_deref(),
        mode,
        limit,
        &scanned,
        &matched,
        &stop,
        &out,
    );
    let scan_ms = scan_started.elapsed().as_millis();

    let sort_started = Instant::now();
    let mut list = out.into_inner().unwrap_or_else(|e| e.into_inner());
    list.sort_by_cached_key(|entry| entry.path.to_ascii_lowercase());
    let sort_ms = sort_started.elapsed().as_millis();

    FileSearchRunStats {
        list,
        scanned: scanned.load(Ordering::Relaxed),
        matched: matched.load(Ordering::Relaxed),
        scan_ms,
        sort_ms,
    }
}

#[cfg(feature = "diff")]
fn insert_response_header(headers: &mut axum::http::HeaderMap, key: &'static str, value: String) {
    if let Ok(v) = HeaderValue::from_str(&value) {
        headers.insert(key, v);
    }
}

// SECURITY: /api/files/search 允许递归扫描任意本地目录。当前 Dashboard 仅绑定 127.0.0.1，
// 风险可控。若未来需要开放网络访问，必须增加路径白名单 / 沙箱机制。
#[cfg(feature = "diff")]
pub(in crate::commands::dashboard) async fn search_files(
    State(state): State<super::super::super::DashboardState>,
    Query(q): Query<FileSearchQuery>,
) -> Response {
    let total_started = Instant::now();
    let root = std::path::PathBuf::from(&q.root);
    if !root.is_dir() {
        return (
            StatusCode::NOT_FOUND,
            format!("not a directory: {}", q.root),
        )
            .into_response();
    }
    state.request_watch_path(&root);

    let needle = q.query.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return Json(Vec::<FileSearchEntry>::new()).into_response();
    }

    let limit = q
        .limit
        .unwrap_or(FILE_SEARCH_DEFAULT_LIMIT)
        .clamp(1, FILE_SEARCH_MAX_LIMIT);
    let mode = resolve_file_search_mode(&needle);
    let task =
        tokio::task::spawn_blocking(move || run_parallel_file_search(root, needle, limit, mode));
    match task.await {
        Ok(out) => {
            let total_ms = total_started.elapsed().as_millis();
            let mut resp = Json(out.list).into_response();
            let headers = resp.headers_mut();
            insert_response_header(headers, "x-xun-search-total-ms", total_ms.to_string());
            insert_response_header(headers, "x-xun-search-scan-ms", out.scan_ms.to_string());
            insert_response_header(headers, "x-xun-search-sort-ms", out.sort_ms.to_string());
            insert_response_header(headers, "x-xun-search-scanned", out.scanned.to_string());
            insert_response_header(headers, "x-xun-search-matched", out.matched.to_string());
            insert_response_header(
                headers,
                "server-timing",
                format!(
                    "scan;dur={}, sort;dur={}, total;dur={}",
                    out.scan_ms, out.sort_ms, total_ms
                ),
            );
            resp
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("search task failed: {e}"),
        )
            .into_response(),
    }
}
