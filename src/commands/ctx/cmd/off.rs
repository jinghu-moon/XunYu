use super::*;

pub(super) fn cmd_off(_args: CtxOffCmd) -> CliResult {
    let Some(session_path) = session_path_from_env() else {
        ui_println!("No active profile.");
        return Ok(());
    };
    let Some(session) = load_session(&session_path) else {
        ui_println!("No active profile.");
        return Ok(());
    };

    if session.proxy_changed {
        if let Some(prev) = &session.previous_proxy {
            let url = normalize_proxy_url(&prev.url);
            let noproxy = prev.noproxy.as_deref().unwrap_or(DEFAULT_NOPROXY);
            emit_proxy_set(&url, noproxy);
            set_proxy(&url, noproxy, None, None);
        } else {
            emit_proxy_off();
            del_proxy(None, None);
        }
    }

    for (k, v) in &session.previous_env {
        match v {
            Some(val) => out_println!("__ENV_SET__:{k}={val}"),
            None => out_println!("__ENV_UNSET__:{k}"),
        }
    }

    if !session.previous_dir.trim().is_empty() {
        if Path::new(&session.previous_dir).exists() {
            out_println!("__CD__:{}", session.previous_dir);
        } else {
            emit_warning(
                format!("Previous directory not found: {}", session.previous_dir),
                &["Hint: The directory may have been moved or deleted."],
            );
        }
    }

    out_println!("__ENV_UNSET__:{}", crate::ctx_store::CTX_STATE_ENV);
    let _ = fs::remove_file(&session_path);
    Ok(())
}
