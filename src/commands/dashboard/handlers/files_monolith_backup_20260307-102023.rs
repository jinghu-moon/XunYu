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
    State(state): State<super::super::DashboardState>,
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
    State(state): State<super::super::DashboardState>,
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

#[cfg(feature = "diff")]
const DIFF_PREVIEW_MAX_SIZE: u64 = 4 * 1024 * 1024; // 4MB

#[cfg(feature = "diff")]
const DIFF_PREVIEW_DEFAULT_LIMIT: usize = 120;

#[cfg(feature = "diff")]
const DIFF_PREVIEW_MAX_LIMIT: usize = 500;

#[cfg(feature = "diff")]
const DIFF_INFO_LINECOUNT_MAX_SIZE: u64 = 2 * 1024 * 1024; // 2MB

#[cfg(feature = "diff")]
#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct FileInfoQuery {
    path: String,
}

#[cfg(feature = "diff")]
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub(in crate::commands::dashboard) enum FileClass {
    Config,
    Code,
    Unknown,
}

#[cfg(feature = "diff")]
#[derive(Serialize)]
pub(in crate::commands::dashboard) struct FileInfoResponse {
    path: String,
    name: String,
    size: u64,
    line_count: Option<usize>,
    language: String,
    file_class: FileClass,
    modified: Option<u64>,
}

#[cfg(feature = "diff")]
#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct FileContentQuery {
    path: String,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
}

#[cfg(feature = "diff")]
#[derive(Serialize)]
pub(in crate::commands::dashboard) struct FileContentResponse {
    path: String,
    offset: usize,
    limit: usize,
    total_lines: usize,
    truncated: bool,
    is_binary: bool,
    lines: Vec<String>,
}

#[cfg(feature = "diff")]
fn diff_detect_language(ext: &str) -> String {
    match ext {
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" | "json5" => "json",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "vue" => "vue",
        "html" => "html",
        "css" => "css",
        "scss" => "scss",
        "sass" => "sass",
        "less" => "less",
        "rs" => "rust",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "md" => "markdown",
        _ => "plaintext",
    }
    .to_string()
}

#[cfg(feature = "diff")]
fn diff_classify(ext: &str) -> FileClass {
    match ext {
        "toml" | "yaml" | "yml" | "json" | "json5" | "env" => FileClass::Config,
        "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "vue" | "html" | "css" | "scss" | "sass"
        | "less" | "rs" | "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" => FileClass::Code,
        _ => FileClass::Unknown,
    }
}

#[cfg(feature = "diff")]
fn looks_binary_prefix(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|b| *b == 0)
}

#[cfg(feature = "diff")]
fn estimate_line_count_from_bytes(bytes: &[u8]) -> usize {
    if bytes.is_empty() {
        return 0;
    }
    let mut n = bytes.iter().filter(|b| **b == b'\n').count();
    if bytes.last().copied() != Some(b'\n') {
        n += 1;
    }
    n
}

#[cfg(feature = "diff")]
fn preview_text_lines(text: &str, offset: usize, limit: usize) -> (Vec<String>, usize) {
    let mut total = 0usize;
    let mut out: Vec<String> = Vec::with_capacity(limit);
    for (idx, line) in text.lines().enumerate() {
        total = idx + 1;
        if idx >= offset && out.len() < limit {
            out.push(line.to_string());
        }
    }
    (out, total)
}

// SECURITY: /api/info 允许读取任意本地文件元信息。当前 Dashboard 仅绑定 127.0.0.1，
// 风险可控。若未来需要开放网络访问，必须增加路径白名单 / 沙箱机制。
#[cfg(feature = "diff")]
pub(in crate::commands::dashboard) async fn get_file_info(
    State(state): State<super::super::DashboardState>,
    Query(q): Query<FileInfoQuery>,
) -> Response {
    let path = std::path::Path::new(&q.path);
    if !path.is_file() {
        return (StatusCode::NOT_FOUND, format!("not a file: {}", q.path)).into_response();
    }
    state.request_watch_path(path);

    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("metadata: {e}")).into_response();
        }
    };
    let size = meta.len();
    let modified = meta
        .modified()
        .ok()
        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let line_count = if size <= DIFF_INFO_LINECOUNT_MAX_SIZE {
        match std::fs::read(path) {
            Ok(bytes) if !looks_binary_prefix(&bytes) => {
                Some(estimate_line_count_from_bytes(&bytes))
            }
            Ok(_) => None,
            Err(_) => None,
        }
    } else {
        None
    };

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    Json(FileInfoResponse {
        path: q.path,
        name,
        size,
        line_count,
        language: diff_detect_language(&ext),
        file_class: diff_classify(&ext),
        modified,
    })
    .into_response()
}

// SECURITY: /api/content 允许读取任意本地文件内容。当前 Dashboard 仅绑定 127.0.0.1，
// 风险可控。若未来需要开放网络访问，必须增加路径白名单 / 沙箱机制。
#[cfg(feature = "diff")]
pub(in crate::commands::dashboard) async fn get_file_content(
    State(state): State<super::super::DashboardState>,
    Query(q): Query<FileContentQuery>,
) -> Response {
    let path = std::path::Path::new(&q.path);
    if !path.is_file() {
        return (StatusCode::NOT_FOUND, format!("not a file: {}", q.path)).into_response();
    }
    state.request_watch_path(path);

    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("metadata: {e}")).into_response();
        }
    };

    if meta.len() > DIFF_PREVIEW_MAX_SIZE {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            format!(
                "file exceeds {}MB preview limit",
                DIFF_PREVIEW_MAX_SIZE / 1024 / 1024
            ),
        )
            .into_response();
    }

    let offset = q.offset.unwrap_or(0);
    let limit = q
        .limit
        .unwrap_or(DIFF_PREVIEW_DEFAULT_LIMIT)
        .clamp(1, DIFF_PREVIEW_MAX_LIMIT);

    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("read: {e}")).into_response();
        }
    };

    if looks_binary_prefix(&bytes) {
        return Json(FileContentResponse {
            path: q.path,
            offset,
            limit,
            total_lines: 0,
            truncated: false,
            is_binary: true,
            lines: Vec::new(),
        })
        .into_response();
    }

    let text = String::from_utf8_lossy(&bytes);
    let (lines, total_lines) = preview_text_lines(&text, offset, limit);
    let truncated = offset.saturating_add(lines.len()) < total_lines;

    Json(FileContentResponse {
        path: q.path,
        offset,
        limit,
        total_lines,
        truncated,
        is_binary: false,
        lines,
    })
    .into_response()
}

#[cfg(feature = "diff")]
#[derive(Clone, Copy)]
enum ConfigFormat {
    Toml,
    Yaml,
    Json,
    Json5,
}

#[cfg(feature = "diff")]
impl ConfigFormat {
    fn as_str(self) -> &'static str {
        match self {
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Json5 => "json5",
        }
    }

    fn ext(self) -> &'static str {
        match self {
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Json5 => "json5",
        }
    }

    fn from_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "toml" => Some(Self::Toml),
            "yaml" | "yml" => Some(Self::Yaml),
            "json" => Some(Self::Json),
            "json5" => Some(Self::Json5),
            _ => None,
        }
    }

    fn from_path(path: &std::path::Path) -> Option<Self> {
        let ext = path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        Self::from_str(&ext)
    }
}

#[cfg(feature = "diff")]
#[derive(Serialize)]
pub(in crate::commands::dashboard) struct ValidateErrorItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    col: Option<usize>,
    message: String,
}

#[cfg(feature = "diff")]
#[derive(Serialize)]
pub(in crate::commands::dashboard) struct ValidateFileResponse {
    valid: bool,
    errors: Vec<ValidateErrorItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
}

#[cfg(feature = "diff")]
#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct ValidateFileRequest {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    format: Option<String>,
}

#[cfg(feature = "diff")]
#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct ConvertFileRequest {
    path: String,
    to_format: String,
    #[serde(default = "default_convert_preview")]
    preview: bool,
}

#[cfg(feature = "diff")]
fn default_convert_preview() -> bool {
    true
}

#[cfg(feature = "diff")]
#[derive(Serialize)]
pub(in crate::commands::dashboard) struct ConvertFileResponse {
    from_format: String,
    to_format: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    written_path: Option<String>,
}

#[cfg(feature = "diff")]
struct ParseIssue {
    line: Option<usize>,
    col: Option<usize>,
    message: String,
}

#[cfg(feature = "diff")]
fn line_col_from_offset(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    let cap = offset.min(text.len());
    for ch in text[..cap].chars() {
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(feature = "diff")]
fn parse_config_value(format: ConfigFormat, text: &str) -> Result<serde_json::Value, ParseIssue> {
    match format {
        ConfigFormat::Json => {
            serde_json::from_str::<serde_json::Value>(text).map_err(|e| ParseIssue {
                line: Some(e.line()),
                col: Some(e.column()),
                message: e.to_string(),
            })
        }
        ConfigFormat::Json5 => json5::from_str::<serde_json::Value>(text).map_err(|e| ParseIssue {
            line: None,
            col: None,
            message: e.to_string(),
        }),
        ConfigFormat::Toml => {
            let parsed: toml::Value = toml::from_str(text).map_err(|e| {
                let (line, col) = e
                    .span()
                    .map(|span| line_col_from_offset(text, span.start))
                    .map(|(l, c)| (Some(l), Some(c)))
                    .unwrap_or((None, None));
                ParseIssue {
                    line,
                    col,
                    message: e.to_string(),
                }
            })?;
            serde_json::to_value(parsed).map_err(|e| ParseIssue {
                line: None,
                col: None,
                message: e.to_string(),
            })
        }
        ConfigFormat::Yaml => {
            let parsed: serde_yaml::Value = serde_yaml::from_str(text).map_err(|e| {
                let loc = e.location();
                ParseIssue {
                    line: loc.as_ref().map(|v| v.line()),
                    col: loc.as_ref().map(|v| v.column()),
                    message: e.to_string(),
                }
            })?;
            serde_json::to_value(parsed).map_err(|e| ParseIssue {
                line: None,
                col: None,
                message: e.to_string(),
            })
        }
    }
}

#[cfg(feature = "diff")]
fn render_config_value(format: ConfigFormat, value: &serde_json::Value) -> Result<String, String> {
    match format {
        ConfigFormat::Json | ConfigFormat::Json5 => {
            serde_json::to_string_pretty(value).map_err(|e| e.to_string())
        }
        ConfigFormat::Yaml => serde_yaml::to_string(value).map_err(|e| e.to_string()),
        ConfigFormat::Toml => {
            let toml_value = toml::Value::try_from(value.clone()).map_err(|e| e.to_string())?;
            toml::to_string_pretty(&toml_value).map_err(|e| e.to_string())
        }
    }
}

#[cfg(feature = "diff")]
fn convert_output_path(src: &std::path::Path, target: ConfigFormat) -> std::path::PathBuf {
    let mut out = src.to_path_buf();
    let same_ext = src
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| v.eq_ignore_ascii_case(target.ext()))
        .unwrap_or(false);

    if same_ext {
        let stem = src
            .file_stem()
            .and_then(|v| v.to_str())
            .filter(|v| !v.is_empty())
            .unwrap_or("converted");
        out.set_file_name(format!("{stem}.converted.{}", target.ext()));
    } else {
        out.set_extension(target.ext());
    }
    out
}

// SECURITY: /api/validate 允许读取任意本地配置文件并返回语法错误。当前 Dashboard 仅绑定 127.0.0.1，
// 风险可控。若未来需要开放网络访问，必须增加路径白名单 / 沙箱机制。
#[cfg(feature = "diff")]
pub(in crate::commands::dashboard) async fn validate_file(Json(req): Json<ValidateFileRequest>) -> Response {
    let (content, format) =
        if let Some(path) = req.path.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
            let p = std::path::Path::new(path);
            if !p.is_file() {
                return (StatusCode::NOT_FOUND, format!("not a file: {path}")).into_response();
            }
            let format = if let Some(raw) = req.format.as_deref() {
                match ConfigFormat::from_str(raw) {
                    Some(v) => v,
                    None => {
                        return (
                            StatusCode::BAD_REQUEST,
                            format!("unsupported format: {raw}"),
                        )
                            .into_response();
                    }
                }
            } else {
                match ConfigFormat::from_path(p) {
                    Some(v) => v,
                    None => {
                        return (
                            StatusCode::BAD_REQUEST,
                            format!("unsupported file format: {path}"),
                        )
                            .into_response();
                    }
                }
            };
            let content = match std::fs::read_to_string(p) {
                Ok(v) => v,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("read file: {e}"))
                        .into_response();
                }
            };
            (content, format)
        } else {
            let raw = req
                .content
                .as_deref()
                .map(str::to_string)
                .ok_or_else(|| "missing content")
                .and_then(|c| {
                    let format_raw = req
                        .format
                        .as_deref()
                        .ok_or("missing format")
                        .map(str::to_string)?;
                    Ok((c, format_raw))
                });
            let (content, format_raw) = match raw {
                Ok(v) => v,
                Err(msg) => return (StatusCode::BAD_REQUEST, msg).into_response(),
            };
            let format = match ConfigFormat::from_str(&format_raw) {
                Some(v) => v,
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        format!("unsupported format: {format_raw}"),
                    )
                        .into_response();
                }
            };
            (content, format)
        };

    let response = match parse_config_value(format, &content) {
        Ok(_) => ValidateFileResponse {
            valid: true,
            errors: Vec::new(),
            format: Some(format.as_str().to_string()),
        },
        Err(err) => ValidateFileResponse {
            valid: false,
            errors: vec![ValidateErrorItem {
                line: err.line,
                col: err.col,
                message: err.message,
            }],
            format: Some(format.as_str().to_string()),
        },
    };

    Json(response).into_response()
}

// SECURITY: /api/convert 允许读取并写入本地配置文件。当前 Dashboard 仅绑定 127.0.0.1，
// 风险可控。若未来需要开放网络访问，必须增加路径白名单 / 沙箱机制。
#[cfg(feature = "diff")]
pub(in crate::commands::dashboard) async fn convert_file(
    State(state): State<super::super::DashboardState>,
    Json(req): Json<ConvertFileRequest>,
) -> Response {
    let src = std::path::Path::new(&req.path);
    if !src.is_file() {
        return (StatusCode::NOT_FOUND, format!("not a file: {}", req.path)).into_response();
    }

    let from_format = match ConfigFormat::from_path(src) {
        Some(v) => v,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                format!("unsupported source format: {}", req.path),
            )
                .into_response();
        }
    };
    let to_format = match ConfigFormat::from_str(&req.to_format) {
        Some(v) => v,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                format!("unsupported target format: {}", req.to_format),
            )
                .into_response();
        }
    };

    let source_text = match std::fs::read_to_string(src) {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("read file: {e}")).into_response();
        }
    };
    let parsed = match parse_config_value(from_format, &source_text) {
        Ok(v) => v,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("parse failed: {}", err.message),
            )
                .into_response();
        }
    };
    let rendered = match render_config_value(to_format, &parsed) {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("convert failed: {e}")).into_response(),
    };

    let mut written_path: Option<String> = None;
    if !req.preview {
        let out_path = convert_output_path(src, to_format);
        if out_path.exists() {
            return (
                StatusCode::CONFLICT,
                format!("output already exists: {}", out_path.to_string_lossy()),
            )
                .into_response();
        }
        if let Err(e) = std::fs::write(&out_path, rendered.as_bytes()) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("write output failed: {e}"),
            )
                .into_response();
        }
        state.request_watch_path(&out_path);
        state.emit_file_changed(&out_path);
        written_path = Some(out_path.to_string_lossy().to_string());
    }

    Json(ConvertFileResponse {
        from_format: from_format.as_str().to_string(),
        to_format: to_format.as_str().to_string(),
        content: rendered,
        written_path,
    })
    .into_response()
}

// --- Diff ---

#[cfg(feature = "diff")]
const DIFF_MAX_SIZE: u64 = 512 * 1024; // 512KB

#[cfg(feature = "diff")]
#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct DiffApiRequest {
    pub old_path: String,
    pub new_path: String,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub algorithm: Option<String>,
    #[serde(default)]
    pub context: Option<usize>,
    #[serde(default)]
    pub ignore_space_change: Option<bool>,
    #[serde(default)]
    pub ignore_all_space: Option<bool>,
    #[serde(default)]
    pub ignore_blank_lines: Option<bool>,
    #[serde(default)]
    pub strip_trailing_cr: Option<bool>,
    #[serde(default)]
    pub force_text: Option<bool>,
}

// SECURITY: /api/diff 允许读取任意本地文件进行 diff。当前 Dashboard 仅绑定 127.0.0.1，
// 风险可控。若未来需要开放网络访问，必须增加路径白名单 / 沙箱机制。
#[cfg(feature = "diff")]
pub(in crate::commands::dashboard) async fn diff_handler(Json(req): Json<DiffApiRequest>) -> Response {
    use crate::diff;
    use crate::diff::types::*;

    // 1. 路径验证
    let old_path = std::path::PathBuf::from(&req.old_path);
    let new_path = std::path::PathBuf::from(&req.new_path);

    if !old_path.is_file() {
        return (
            StatusCode::NOT_FOUND,
            format!("old_path not found: {}", req.old_path),
        )
            .into_response();
    }
    if !new_path.is_file() {
        return (
            StatusCode::NOT_FOUND,
            format!("new_path not found: {}", req.new_path),
        )
            .into_response();
    }

    // 2. 大小检查
    for (p, label) in [(&old_path, "old_path"), (&new_path, "new_path")] {
        match std::fs::metadata(p) {
            Ok(m) if m.len() > DIFF_MAX_SIZE => {
                return (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    format!("{label} exceeds 512KB limit"),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("cannot stat {label}: {e}"),
                )
                    .into_response();
            }
            _ => {}
        }
    }

    // 3. 读取文件
    let old_bytes = match std::fs::read(&old_path) {
        Ok(b) => b,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("read old: {e}")).into_response();
        }
    };
    let new_bytes = match std::fs::read(&new_path) {
        Ok(b) => b,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("read new: {e}")).into_response();
        }
    };

    // 3.5 提前解析算法（UTF-8 早退需要用到）
    let algorithm = match req.algorithm.as_deref().unwrap_or("histogram") {
        "histogram" => DiffAlgorithm::Histogram,
        "myers" => DiffAlgorithm::Myers,
        "minimal" => DiffAlgorithm::Minimal,
        "patience" => DiffAlgorithm::Patience,
        other => {
            return (
                StatusCode::BAD_REQUEST,
                format!("invalid algorithm: {other}"),
            )
                .into_response();
        }
    };

    // 3.6 UTF-8 验证：非 force_text 模式下，非 UTF-8 文件视为 Binary
    let force_text = req.force_text.unwrap_or(false);
    if !force_text {
        if std::str::from_utf8(&old_bytes).is_err() || std::str::from_utf8(&new_bytes).is_err() {
            let (actual_algorithm, _) = crate::diff::line::map_algorithm(algorithm);
            let binary_result = crate::diff::types::DiffResult {
                kind: crate::diff::types::DiffResultKind::Binary,
                stats: crate::diff::types::DiffStats::zero(crate::diff::types::StatsUnit::Line),
                hunks: vec![],
                actual_algorithm,
                identical_with_filters: false,
            };
            return Json(binary_result).into_response();
        }
    }

    // 4. 解析剩余参数
    let mode = match req.mode.as_deref().unwrap_or("auto") {
        "auto" => DiffMode::Auto,
        "line" => DiffMode::Line,
        "ast" => DiffMode::Ast,
        other => {
            return (StatusCode::BAD_REQUEST, format!("invalid mode: {other}")).into_response();
        }
    };

    // 扩展名推断：优先 new_path，fallback old_path
    let ext = new_path
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.is_empty())
        .or_else(|| old_path.extension().and_then(|e| e.to_str()))
        .unwrap_or("")
        .to_lowercase();
    let context = req.context.unwrap_or(3);

    // 5. spawn_blocking 执行 diff（CPU 密集型）
    let result = tokio::task::spawn_blocking(move || {
        let old_text = String::from_utf8_lossy(&old_bytes);
        let new_text = String::from_utf8_lossy(&new_bytes);

        diff::diff(DiffRequest {
            old: &old_text,
            new: &new_text,
            ext: &ext,
            mode,
            algorithm,
            context,
            whitespace: WhitespaceOpt {
                ignore_space_change: req.ignore_space_change.unwrap_or(false),
                ignore_all_space: req.ignore_all_space.unwrap_or(false),
                ignore_blank_lines: req.ignore_blank_lines.unwrap_or(false),
                strip_trailing_cr: req.strip_trailing_cr.unwrap_or(false),
            },
            force_text: req.force_text.unwrap_or(false),
        })
    })
    .await;

    match result {
        Ok(diff_result) => Json(diff_result).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("diff task failed: {e}"),
        )
            .into_response(),
    }
}


