use super::super::common::ensure_parent_dir;
use super::*;

pub(super) fn cmd_use(args: CtxUseCmd) -> CliResult {
    let store_path = ctx_store_path();
    let store = load_store(&store_path);
    let profile = store.profiles.get(&args.name).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Profile not found: {}", args.name),
            &["Hint: Run `xun ctx list` to see existing profiles."],
        )
    })?;

    if !Path::new(&profile.path).exists() {
        return Err(CliError::with_details(
            2,
            format!("Path not found: {}", profile.path),
            &[
                "Hint: Run `xun ctx show <name>` to check profile details.",
                "Fix: Update with `xun ctx set <name> --path <dir>`.",
            ],
        ));
    }

    let session_path = session_path_from_env().ok_or_else(|| {
        CliError::with_details(
            2,
            "XUN_CTX_STATE is not set.".to_string(),
            &[
                "Hint: Load shell integration with `xun init <shell>`.",
                "Fix: Re-open your shell after running `xun init`.",
            ],
        )
    })?;

    let previous_dir = env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| ".".to_string());

    let mut previous_env: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut env_keys: HashSet<String> = HashSet::new();
    env_keys.insert("XUN_DEFAULT_TAG".to_string());
    env_keys.insert("XUN_CTX".to_string());
    for k in profile.env.keys() {
        env_keys.insert(k.clone());
    }
    for key in env_keys {
        previous_env.insert(key.clone(), env::var(&key).ok());
    }

    let env_proxy = env::var("HTTP_PROXY")
        .or_else(|_| env::var("http_proxy"))
        .ok()
        .filter(|v| !v.trim().is_empty());
    let env_noproxy = env::var("NO_PROXY")
        .or_else(|_| env::var("no_proxy"))
        .ok()
        .filter(|v| !v.trim().is_empty());

    let previous_proxy = if let Some(url) = env_proxy {
        Some(CtxProxyState {
            url,
            noproxy: env_noproxy,
        })
    } else {
        load_proxy_state().map(|p| CtxProxyState {
            url: p.url,
            noproxy: p.noproxy,
        })
    };

    let proxy_changed = matches!(profile.proxy.mode, CtxProxyMode::Set | CtxProxyMode::Off);
    let session = CtxSession {
        active: args.name.clone(),
        previous_dir,
        previous_env,
        previous_proxy,
        proxy_changed,
    };
    ensure_parent_dir(&session_path);
    save_session(&session_path, &session)
        .map_err(|e| CliError::new(1, format!("Failed to write ctx session: {e}")))?;

    out_println!("__CD__:{}", profile.path);

    match profile.proxy.mode {
        CtxProxyMode::Set => {
            let url = profile.proxy.url.as_ref().ok_or_else(|| {
                CliError::with_details(
                    2,
                    format!("Profile '{}' has proxy mode set but no url.", args.name),
                    &["Fix: Update with `xun ctx set <name> --proxy <url>`."],
                )
            })?;
            let url = normalize_proxy_url(url);
            let noproxy = profile.proxy.noproxy.as_deref().unwrap_or(DEFAULT_NOPROXY);
            emit_proxy_set(&url, noproxy);
            set_proxy(&url, noproxy, None, None);
        }
        CtxProxyMode::Off => {
            emit_proxy_off();
            del_proxy(None, None);
        }
        CtxProxyMode::Keep => {}
    }

    if profile.tags.is_empty() {
        out_println!("__ENV_UNSET__:XUN_DEFAULT_TAG");
    } else {
        out_println!("__ENV_SET__:XUN_DEFAULT_TAG={}", profile.tags.join(","));
    }
    out_println!("__ENV_SET__:XUN_CTX={}", args.name);

    for (k, v) in &profile.env {
        out_println!("__ENV_SET__:{k}={v}");
    }

    Ok(())
}
