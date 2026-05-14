//! Context 业务逻辑服务
//!
//! 封装上下文 profile 管理操作，供 CommandSpec 实现调用。

use crate::xun_core::error::XunError;
use crate::xun_core::value::Value;

/// 列出所有 profile。
pub fn list_profiles() -> Result<Value, XunError> {
    let path = crate::ctx_store::ctx_store_path();
    let store = crate::ctx_store::load_store(&path);

    // 读取当前激活的 profile 名称
    let session_path = crate::ctx_store::session_path_from_env()
        .unwrap_or_else(|| path.with_extension("session.json"));
    let active_name = crate::ctx_store::load_session(&session_path)
        .map(|s| s.active)
        .unwrap_or_default();

    let items: Vec<Value> = store
        .profiles
        .iter()
        .map(|(name, profile)| {
            let mut rec = crate::xun_core::value::Record::new();
            rec.insert("name".into(), Value::String(name.clone()));
            rec.insert("path".into(), Value::String(profile.path.clone()));
            rec.insert(
                "proxy_url".into(),
                Value::String(profile.proxy.url.clone().unwrap_or_default()),
            );
            rec.insert(
                "active".into(),
                Value::Bool(name == &active_name),
            );
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(items))
}

/// 切换到指定 profile。
pub fn use_profile(name: &str) -> Result<(), XunError> {
    let path = crate::ctx_store::ctx_store_path();
    let store = crate::ctx_store::load_store(&path);
    if !store.profiles.contains_key(name) {
        return Err(XunError::NotFound(format!("profile '{}' not found", name)));
    }

    // 更新 session 中的 active 字段
    let session_path = crate::ctx_store::session_path_from_env()
        .unwrap_or_else(|| path.with_extension("session.json"));
    let mut session = crate::ctx_store::load_session(&session_path)
        .unwrap_or_default();
    session.active = name.to_string();
    crate::ctx_store::save_session(&session_path, &session)
        .map_err(|e| XunError::Internal(anyhow::anyhow!("failed to save session: {e}")))?;
    Ok(())
}

/// 停用当前 profile。
pub fn off_profile() -> Result<(), XunError> {
    let path = crate::ctx_store::ctx_store_path();
    let session_path = crate::ctx_store::session_path_from_env()
        .unwrap_or_else(|| path.with_extension("session.json"));
    let mut session = crate::ctx_store::load_session(&session_path)
        .unwrap_or_default();
    session.active.clear();
    crate::ctx_store::save_session(&session_path, &session)
        .map_err(|e| XunError::Internal(anyhow::anyhow!("failed to save session: {e}")))?;
    Ok(())
}

/// 删除 profile。
pub fn rm_profile(name: &str) -> Result<(), XunError> {
    let path = crate::ctx_store::ctx_store_path();
    let mut store = crate::ctx_store::load_store(&path);
    if store.profiles.remove(name).is_none() {
        return Err(XunError::NotFound(format!("profile '{}' not found", name)));
    }
    crate::ctx_store::save_store(&path, &store)
        .map_err(|e| XunError::Internal(anyhow::anyhow!("failed to save ctx store: {e}")))?;

    // 如果删除的是当前激活的 profile，清除激活状态
    let session_path = crate::ctx_store::session_path_from_env()
        .unwrap_or_else(|| path.with_extension("session.json"));
    if let Some(mut session) = crate::ctx_store::load_session(&session_path) {
        if session.active == name {
            session.active.clear();
            let _ = crate::ctx_store::save_session(&session_path, &session);
        }
    }
    Ok(())
}

/// 重命名 profile。
pub fn rename_profile(old: &str, new: &str) -> Result<(), XunError> {
    let path = crate::ctx_store::ctx_store_path();
    let mut store = crate::ctx_store::load_store(&path);
    let profile = store
        .profiles
        .remove(old)
        .ok_or_else(|| XunError::NotFound(format!("profile '{}' not found", old)))?;
    store.profiles.insert(new.to_string(), profile);
    crate::ctx_store::save_store(&path, &store)
        .map_err(|e| XunError::Internal(anyhow::anyhow!("failed to save ctx store: {e}")))?;

    // 更新 session 中的 active 字段
    let session_path = crate::ctx_store::session_path_from_env()
        .unwrap_or_else(|| path.with_extension("session.json"));
    if let Some(mut session) = crate::ctx_store::load_session(&session_path) {
        if session.active == old {
            session.active = new.to_string();
            let _ = crate::ctx_store::save_session(&session_path, &session);
        }
    }
    Ok(())
}
