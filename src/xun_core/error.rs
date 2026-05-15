//! XunError — 分层错误类型
//!
//! 统一的错误模型，替代当前 CliError 的扁平结构。
//! 每个变体携带语义信息，exit code 由类型自动推导。

use std::fmt;

/// 分层错误类型，覆盖 CLI 所有错误场景。
#[derive(Debug)]
pub enum XunError {
    /// 用户输入/操作错误（exit 1）
    User {
        message: String,
        hints: Vec<String>,
    },

    /// 用户取消操作（exit 130，对应 SIGINT）
    Cancelled,

    /// 需要管理员权限（exit 77，对应 EPERM）
    ElevationRequired(String),

    /// 资源未找到（exit 2）
    NotFound(String),

    /// 内部/不可预期错误（exit 1）
    Internal(anyhow::Error),
}

impl XunError {
    /// 构造用户错误。
    pub fn user(message: impl Into<String>) -> Self {
        Self::User {
            message: message.into(),
            hints: Vec::new(),
        }
    }

    /// 附加提示信息（仅 User 变体有效）。
    pub fn with_hints(mut self, hints: &[&str]) -> Self {
        if let Self::User {
            hints: ref mut h, ..
        } = self
        {
            h.extend(hints.iter().map(|s| s.to_string()));
        }
        self
    }

    /// 获取提示列表。
    pub fn hints(&self) -> &[String] {
        match self {
            Self::User { hints, .. } => hints,
            _ => &[],
        }
    }

    /// 进程退出码。
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::User { .. } | Self::Internal(_) => 1,
            Self::Cancelled => 130,
            Self::ElevationRequired(_) => 77,
            Self::NotFound(_) => 2,
        }
    }
}

impl fmt::Display for XunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User { message, .. } => write!(f, "{message}"),
            Self::Cancelled => write!(f, "operation cancelled"),
            Self::ElevationRequired(msg) => write!(f, "{msg}"),
            Self::NotFound(msg) => write!(f, "{msg}"),
            Self::Internal(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for XunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Internal(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for XunError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err)
    }
}

impl From<std::io::Error> for XunError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(anyhow::Error::from(err))
    }
}

impl From<crate::output::CliError> for XunError {
    fn from(e: crate::output::CliError) -> Self {
        Self::User {
            message: e.message,
            hints: e.details,
        }
    }
}

// 编译期保证 Send + Sync
#[cfg(test)]
mod _send_sync_check {
    use super::XunError;
    fn _assert<T: Send + Sync>() {}
    fn _check() {
        _assert::<XunError>();
    }
}
