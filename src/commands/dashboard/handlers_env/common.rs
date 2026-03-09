use super::*;

pub(super) fn manager() -> EnvManager {
    let tx = env_event_sender().clone();
    EnvManager::new().with_event_callback(Arc::new(move |event: EnvEvent| {
        if let Ok(payload) = serde_json::to_string(&event) {
            let _ = tx.send(payload);
        }
    }))
}

pub(super) fn env_event_sender() -> &'static broadcast::Sender<String> {
    static TX: OnceLock<broadcast::Sender<String>> = OnceLock::new();
    TX.get_or_init(|| {
        let (tx, _) = broadcast::channel(256);
        tx
    })
}

pub(super) fn ok<T: Serialize>(data: T) -> Json<ApiSuccess<T>> {
    Json(ApiSuccess { ok: true, data })
}

#[allow(clippy::result_large_err)]
pub(super) fn resolve_scope(
    value: Option<String>,
    default: EnvScope,
) -> Result<EnvScope, Response> {
    match value {
        Some(v) => EnvScope::from_str(&v).map_err(|e| map_env_error(e).into_response()),
        None => Ok(default),
    }
}

pub(super) fn parse_set_pairs(items: &[String]) -> Result<Vec<(String, String)>, EnvError> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some((name, value)) = item.split_once('=') else {
            return Err(EnvError::InvalidInput(format!(
                "invalid set item '{}', expected KEY=VALUE",
                item
            )));
        };
        let key = name.trim();
        if key.is_empty() {
            return Err(EnvError::InvalidInput(format!(
                "invalid set item '{}': empty key",
                item
            )));
        }
        out.push((key.to_string(), value.to_string()));
    }
    Ok(out)
}

pub(super) fn run_api_enabled(cfg_allow: bool) -> bool {
    if cfg_allow {
        return true;
    }
    matches!(
        std::env::var("ENVMGR_ALLOW_RUN"),
        Ok(v) if v == "1" || v.eq_ignore_ascii_case("true")
    )
}

pub(super) fn map_env_error(err: EnvError) -> (StatusCode, Json<ApiError>) {
    let status = match &err {
        EnvError::InvalidInput(_) | EnvError::ScopeNotWritable(_) => StatusCode::BAD_REQUEST,
        EnvError::PermissionDenied(_) => StatusCode::FORBIDDEN,
        EnvError::NotFound(_) => StatusCode::NOT_FOUND,
        EnvError::UnsupportedPlatform => StatusCode::NOT_IMPLEMENTED,
        EnvError::Other(msg) if msg.starts_with("schema-check failed:") => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let code = match &err {
        EnvError::InvalidInput(_) => "env.invalid_input",
        EnvError::ScopeNotWritable(_) => "env.invalid_scope",
        EnvError::PermissionDenied(_) => "env.permission_denied",
        EnvError::NotFound(_) => "env.not_found",
        EnvError::UnsupportedPlatform => "env.unsupported_platform",
        EnvError::LockFailed(_) => "env.lock_failed",
        EnvError::Other(msg) if msg.starts_with("schema-check failed:") => "env.invalid_input",
        _ => "env.internal_error",
    };
    (
        status,
        Json(ApiError {
            ok: false,
            code: code.to_string(),
            message: err.to_string(),
            details: None,
        }),
    )
}
