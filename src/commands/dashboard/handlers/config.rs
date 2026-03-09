use super::*;

// --- Config ---

fn parse_optional_usize(value: &Value, field: &str) -> Result<Option<usize>, String> {
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_u64()
        .map(|v| Some(v as usize))
        .ok_or_else(|| format!("{field} must be number or null"))
}

fn parse_optional_string(value: &Value, field: &str) -> Result<Option<String>, String> {
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_str()
        .map(|v| Some(v.to_string()))
        .ok_or_else(|| format!("{field} must be string or null"))
}

fn parse_string_vec(value: &Value, field: &str) -> Result<Vec<String>, String> {
    let arr = value
        .as_array()
        .ok_or_else(|| format!("{field} must be array of strings"))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let s = item
            .as_str()
            .ok_or_else(|| format!("{field} must be array of strings"))?;
        out.push(s.to_string());
    }
    Ok(out)
}

pub(in crate::commands::dashboard) async fn post_config_patch(Json(body): Json<Value>) -> Response {
    let cfg_path = config::config_path();
    let Some(_lock) = common::try_acquire_lock(&cfg_path) else {
        return StatusCode::CONFLICT.into_response();
    };

    let mut cfg = config::load_config();
    if let Some(tree) = body.get("tree") {
        let tree = match tree.as_object() {
            Some(v) => v,
            None => return (StatusCode::BAD_REQUEST, "tree must be object").into_response(),
        };
        if let Some(value) = tree.get("defaultDepth") {
            match parse_optional_usize(value, "tree.defaultDepth") {
                Ok(v) => cfg.tree.default_depth = v,
                Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
            }
        }
        if let Some(value) = tree.get("excludeNames") {
            match parse_string_vec(value, "tree.excludeNames") {
                Ok(v) => cfg.tree.exclude_names = v,
                Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
            }
        }
    }

    if let Some(proxy) = body.get("proxy") {
        let proxy = match proxy.as_object() {
            Some(v) => v,
            None => return (StatusCode::BAD_REQUEST, "proxy must be object").into_response(),
        };
        let default_url = proxy.get("defaultUrl").or_else(|| proxy.get("default_url"));
        if let Some(value) = default_url {
            match parse_optional_string(value, "proxy.defaultUrl") {
                Ok(v) => cfg.proxy.default_url = v,
                Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
            }
        }
        if let Some(value) = proxy.get("noproxy") {
            match parse_optional_string(value, "proxy.noproxy") {
                Ok(v) => cfg.proxy.noproxy = v,
                Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
            }
        }
    }

    #[cfg(feature = "protect")]
    if let Some(value) = body.get("protect") {
        if value.is_null() {
            cfg.protect = config::ProtectConfig::default();
        } else {
            match serde_json::from_value::<config::ProtectConfig>(value.clone()) {
                Ok(v) => cfg.protect = v,
                Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
            }
        }
    }

    #[cfg(feature = "redirect")]
    if let Some(value) = body.get("redirect") {
        if value.is_null() {
            cfg.redirect = config::RedirectConfig::default();
        } else {
            match serde_json::from_value::<config::RedirectConfig>(value.clone()) {
                Ok(v) => cfg.redirect = v,
                Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
            }
        }
    }

    match config::save_config(&cfg) {
        Ok(_) => Json(cfg).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub(in crate::commands::dashboard) async fn put_config_replace(
    Json(body): Json<config::GlobalConfig>,
) -> Response {
    let cfg_path = config::config_path();
    let Some(_lock) = common::try_acquire_lock(&cfg_path) else {
        return StatusCode::CONFLICT.into_response();
    };

    let cfg = body;
    match config::save_config(&cfg) {
        Ok(_) => Json(cfg).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub(in crate::commands::dashboard) async fn get_config() -> Json<config::GlobalConfig> {
    Json(config::load_config())
}

// --- Redirect Profiles ---

#[cfg(feature = "redirect")]
fn validate_redirect_profile(p: &config::RedirectProfile) -> Result<(), String> {
    if p.rules.is_empty() {
        return Err("rules is empty".to_string());
    }

    if p.recursive && p.max_depth == 0 {
        return Err("recursive=true requires max_depth >= 1".to_string());
    }

    for (idx, r) in p.rules.iter().enumerate() {
        if r.dest.trim().is_empty() {
            return Err(format!("rules[{idx}].dest is empty"));
        }
        let has_ext = !r.match_cond.ext.is_empty();
        let has_glob = r
            .match_cond
            .glob
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_regex = r
            .match_cond
            .regex
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_size = r
            .match_cond
            .size
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_age = r
            .match_cond
            .age
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if !has_ext && !has_glob && !has_regex && !has_size && !has_age {
            return Err(format!("rules[{idx}].match is empty"));
        }

        if let Some(re) = r.match_cond.regex.as_deref()
            && !re.trim().is_empty()
        {
            regex::Regex::new(re).map_err(|e| format!("rules[{idx}].regex invalid: {e}"))?;
        }
        if let Some(sz) = r.match_cond.size.as_deref()
            && !sz.trim().is_empty()
        {
            crate::commands::redirect::parse_size_expr(sz)
                .map_err(|e| format!("rules[{idx}].size invalid: {e}"))?;
        }
        if let Some(age) = r.match_cond.age.as_deref()
            && !age.trim().is_empty()
        {
            crate::commands::redirect::parse_age_expr(age)
                .map_err(|e| format!("rules[{idx}].age invalid: {e}"))?;
        }
    }
    Ok(())
}

#[cfg(feature = "redirect")]
#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct RedirectDryRunRequest {
    source: String,
    profile: config::RedirectProfile,
    #[serde(default)]
    copy: bool,
}

#[cfg(feature = "redirect")]
#[derive(Serialize)]
pub(in crate::commands::dashboard) struct RedirectDryRunItem {
    action: String,
    src: String,
    dst: String,
    rule: String,
    result: String,
    reason: String,
}

#[cfg(feature = "redirect")]
#[derive(Serialize)]
pub(in crate::commands::dashboard) struct RedirectDryRunStats {
    total: usize,
    dry_run: usize,
    skipped: usize,
    failed: usize,
}

#[cfg(feature = "redirect")]
#[derive(Serialize)]
pub(in crate::commands::dashboard) struct RedirectDryRunResponse {
    results: Vec<RedirectDryRunItem>,
    stats: RedirectDryRunStats,
}

#[cfg(feature = "redirect")]
pub(in crate::commands::dashboard) async fn redirect_dry_run(
    Json(body): Json<RedirectDryRunRequest>,
) -> Response {
    if let Err(msg) = validate_redirect_profile(&body.profile) {
        return (StatusCode::BAD_REQUEST, msg).into_response();
    }
    let source = body.source.trim();
    if source.is_empty() {
        return (StatusCode::BAD_REQUEST, "source is empty").into_response();
    }
    let source_path = std::path::Path::new(source);
    if !source_path.exists() {
        return (StatusCode::NOT_FOUND, "source not found").into_response();
    }

    let out = crate::commands::redirect::plan_redirect(source_path, &body.profile, body.copy);
    let mut dry_run = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    let results: Vec<RedirectDryRunItem> = out
        .results
        .into_iter()
        .map(|r| {
            match r.result.as_str() {
                "dry_run" => dry_run += 1,
                "skipped" => skipped += 1,
                "failed" => failed += 1,
                _ => {}
            }
            RedirectDryRunItem {
                action: r.action,
                src: r.src,
                dst: r.dst,
                rule: r.rule,
                result: r.result,
                reason: r.reason,
            }
        })
        .collect();
    let total = results.len();

    Json(RedirectDryRunResponse {
        results,
        stats: RedirectDryRunStats {
            total,
            dry_run,
            skipped,
            failed,
        },
    })
    .into_response()
}

#[cfg(feature = "redirect")]
pub(in crate::commands::dashboard) async fn list_redirect_profiles() -> Json<config::RedirectConfig>
{
    Json(config::load_config().redirect)
}

#[cfg(feature = "redirect")]
pub(in crate::commands::dashboard) async fn upsert_redirect_profile(
    Path(name): Path<String>,
    Json(body): Json<config::RedirectProfile>,
) -> Response {
    if let Err(msg) = validate_redirect_profile(&body) {
        return (StatusCode::BAD_REQUEST, msg).into_response();
    }

    let cfg_path = config::config_path();
    let Some(_lock) = common::try_acquire_lock(&cfg_path) else {
        return StatusCode::CONFLICT.into_response();
    };

    let mut cfg = config::load_config();
    cfg.redirect.profiles.insert(name, body);
    match config::save_config(&cfg) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[cfg(feature = "redirect")]
pub(in crate::commands::dashboard) async fn delete_redirect_profile(
    Path(name): Path<String>,
) -> StatusCode {
    let cfg_path = config::config_path();
    let Some(_lock) = common::try_acquire_lock(&cfg_path) else {
        return StatusCode::CONFLICT;
    };

    let mut cfg = config::load_config();
    if cfg.redirect.profiles.remove(&name).is_none() {
        return StatusCode::NOT_FOUND;
    }
    match config::save_config(&cfg) {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
