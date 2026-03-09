use super::*;

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
pub(in crate::commands::dashboard) async fn diff_handler(
    Json(req): Json<DiffApiRequest>,
) -> Response {
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
    if !force_text
        && (std::str::from_utf8(&old_bytes).is_err() || std::str::from_utf8(&new_bytes).is_err())
    {
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
