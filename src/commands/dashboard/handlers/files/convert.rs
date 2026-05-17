use super::*;

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

// SECURITY: /api/validate 鍏佽璇诲彇浠绘剰鏈湴閰嶇疆鏂囦欢骞惰繑鍥炶娉曢敊璇€傚綋鍓?Dashboard 浠呯粦瀹?127.0.0.1锛?
// 椋庨櫓鍙帶銆傝嫢鏈潵闇€瑕佸紑鏀剧綉缁滆闂紝蹇呴』澧炲姞璺緞鐧藉悕鍗?/ 娌欑鏈哄埗銆?
#[cfg(feature = "diff")]
pub(in crate::commands::dashboard) async fn validate_file(
    Json(req): Json<ValidateFileRequest>,
) -> Response {
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
                .ok_or("missing content")
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

// SECURITY: /api/convert 鍏佽璇诲彇骞跺啓鍏ユ湰鍦伴厤缃枃浠躲€傚綋鍓?Dashboard 浠呯粦瀹?127.0.0.1锛?
// 椋庨櫓鍙帶銆傝嫢鏈潵闇€瑕佸紑鏀剧綉缁滆闂紝蹇呴』澧炲姞璺緞鐧藉悕鍗?/ 娌欑鏈哄埗銆?
#[cfg(feature = "diff")]
pub(in crate::commands::dashboard) async fn convert_file(
    State(state): State<super::super::super::DashboardState>,
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

