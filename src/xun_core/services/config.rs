//! Config 业务逻辑服务
//!
//! 封装配置读写操作，供 CommandSpec 实现调用。

use crate::xun_core::error::XunError;
use crate::xun_core::value::Value;

/// 读取配置值（按点路径，如 "proxy.defaultUrl"）。
pub fn get_config(key: &str) -> Result<Value, XunError> {
    let cfg = crate::config::load_config();
    let json = serde_json::to_value(&cfg)
        .map_err(|e| XunError::Internal(anyhow::anyhow!("config serialize failed: {e}")))?;
    let val = json.pointer(&format!("/{}", key.replace('.', "/")));
    Ok(match val {
        Some(v) if v.is_string() => Value::String(v.as_str().unwrap_or("").to_string()),
        Some(v) if v.is_i64() => Value::Int(v.as_i64().unwrap_or(0)),
        Some(v) if v.is_f64() => Value::Float(v.as_f64().unwrap_or(0.0)),
        Some(v) if v.is_boolean() => Value::Bool(v.as_bool().unwrap_or(false)),
        Some(v) => Value::String(v.to_string()),
        None => Value::Null,
    })
}

/// 设置配置值。
pub fn set_config(key: &str, value: &str) -> Result<(), XunError> {
    let mut cfg = crate::config::load_config();
    let mut json = serde_json::to_value(&cfg)
        .map_err(|e| XunError::Internal(anyhow::anyhow!("config serialize failed: {e}")))?;
    let pointer = format!("/{}", key.replace('.', "/"));
    if let Some(slot) = json.pointer_mut(&pointer) {
        if slot.is_i64() {
            if let Ok(n) = value.parse::<i64>() {
                *slot = serde_json::Value::Number(n.into());
            }
        } else if slot.is_f64() {
            if let Ok(f) = value.parse::<f64>() {
                *slot = serde_json::json!(f);
            }
        } else if slot.is_boolean() {
            if let Ok(b) = value.parse::<bool>() {
                *slot = serde_json::Value::Bool(b);
            }
        } else {
            *slot = serde_json::Value::String(value.to_string());
        }
    } else {
        return Err(XunError::NotFound(format!("config key '{}' not found", key)));
    }
    cfg = serde_json::from_value(json)
        .map_err(|e| XunError::Internal(anyhow::anyhow!("config deserialize failed: {e}")))?;
    crate::config::save_config(&cfg)
        .map_err(|e| XunError::Internal(anyhow::anyhow!("config save failed: {e}")))?;
    Ok(())
}
