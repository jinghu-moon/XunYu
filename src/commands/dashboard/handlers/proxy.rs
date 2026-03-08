use super::*;

// --- Proxy ---

#[derive(Serialize)]
pub(in crate::commands::dashboard) struct ProxyItem {
    tool: &'static str,
    status: &'static str,
    address: String,
}

fn cmd_output(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd).args(args).output().ok().and_then(|o| {
        let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if v.is_empty() || v == "null" {
            None
        } else {
            Some(v)
        }
    })
}

pub(in crate::commands::dashboard) async fn proxy_status() -> Json<Vec<ProxyItem>> {
    let env_proxy = std::env::var("HTTP_PROXY")
        .or_else(|_| std::env::var("http_proxy"))
        .ok();
    let git_proxy = if has_cmd("git") {
        cmd_output("git", &["config", "--global", "--get", "http.proxy"])
    } else {
        None
    };
    let npm_proxy = if has_cmd("npm") {
        cmd_output("npm", &["config", "get", "proxy"])
    } else {
        None
    };

    let cargo_proxy = proxy::config::read_cargo_proxy();

    let row = |tool: &'static str, val: &Option<String>| ProxyItem {
        tool,
        status: if val.is_some() { "ON" } else { "OFF" },
        address: val.clone().unwrap_or_default(),
    };

    Json(vec![
        row("Env", &env_proxy),
        row("Git", &git_proxy),
        row("npm", &npm_proxy),
        row("Cargo", &cargo_proxy),
    ])
}

// --- Proxy Config / Apply / Test ---

pub(in crate::commands::dashboard) async fn get_proxy_config() -> Json<config::ProxyConfig> {
    Json(config::load_config().proxy)
}

pub(in crate::commands::dashboard) async fn set_proxy_config(
    Json(body): Json<config::ProxyConfig>,
) -> Response {
    let cfg_path = config::config_path();
    let Some(_lock) = common::try_acquire_lock(&cfg_path) else {
        return StatusCode::CONFLICT.into_response();
    };

    let mut cfg = config::load_config();
    cfg.proxy = body;
    match config::save_config(&cfg) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct ProxySetBody {
    url: String,
    #[serde(default)]
    noproxy: String,
    #[serde(default)]
    only: Option<String>,
    #[serde(default)]
    msys2: Option<String>,
}

pub(in crate::commands::dashboard) async fn proxy_set(Json(body): Json<ProxySetBody>) -> Response {
    let only = match proxy::config::parse_proxy_only(body.only.as_deref()) {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    proxy::config::set_proxy(
        &body.url,
        &body.noproxy,
        body.msys2.as_deref(),
        only.as_ref(),
    );
    proxy::config::save_proxy_state(&body.url, &body.noproxy);
    StatusCode::OK.into_response()
}

#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct ProxyDelBody {
    #[serde(default)]
    only: Option<String>,
    #[serde(default)]
    msys2: Option<String>,
}

pub(in crate::commands::dashboard) async fn proxy_del(Json(body): Json<ProxyDelBody>) -> Response {
    let only = match proxy::config::parse_proxy_only(body.only.as_deref()) {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    proxy::config::del_proxy(body.msys2.as_deref(), only.as_ref());
    StatusCode::OK.into_response()
}

#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct ProxyTestQuery {
    url: String,
    targets: Option<String>,
    timeout_ms: Option<u64>,
    jobs: Option<usize>,
}

#[derive(Serialize)]
pub(in crate::commands::dashboard) struct ProxyTestItem {
    label: String,
    ok: bool,
    ms: i64,
    error: String,
}

pub(in crate::commands::dashboard) async fn proxy_test(
    Query(q): Query<ProxyTestQuery>,
) -> Json<Vec<ProxyTestItem>> {
    let targets = proxy::test::parse_proxy_targets(q.targets.as_deref());
    let timeout = Duration::from_millis(q.timeout_ms.unwrap_or(2000).clamp(100, 30_000));
    let jobs = q.jobs.unwrap_or(3).clamp(1, 16);
    let out = proxy::test::run_proxy_tests_with(&q.url, targets, timeout, jobs);
    Json(
        out.into_iter()
            .map(|(label, res)| match res {
                Ok(ms) => ProxyTestItem {
                    label,
                    ok: true,
                    ms: ms as i64,
                    error: String::new(),
                },
                Err(e) => ProxyTestItem {
                    label,
                    ok: false,
                    ms: -1,
                    error: e,
                },
            })
            .collect(),
    )
}
