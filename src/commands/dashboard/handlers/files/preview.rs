use super::*;

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

// SECURITY: /api/info 鍏佽璇诲彇浠绘剰鏈湴鏂囦欢鍏冧俊鎭€傚綋鍓?Dashboard 浠呯粦瀹?127.0.0.1锛?
// 椋庨櫓鍙帶銆傝嫢鏈潵闇€瑕佸紑鏀剧綉缁滆闂紝蹇呴』澧炲姞璺緞鐧藉悕鍗?/ 娌欑鏈哄埗銆?
#[cfg(feature = "diff")]
pub(in crate::commands::dashboard) async fn get_file_info(
    State(state): State<super::super::super::DashboardState>,
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

// SECURITY: /api/content 鍏佽璇诲彇浠绘剰鏈湴鏂囦欢鍐呭銆傚綋鍓?Dashboard 浠呯粦瀹?127.0.0.1锛?
// 椋庨櫓鍙帶銆傝嫢鏈潵闇€瑕佸紑鏀剧綉缁滆闂紝蹇呴』澧炲姞璺緞鐧藉悕鍗?/ 娌欑鏈哄埗銆?
#[cfg(feature = "diff")]
pub(in crate::commands::dashboard) async fn get_file_content(
    State(state): State<super::super::super::DashboardState>,
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

