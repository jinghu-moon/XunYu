//! WebSocket 命令协议
//!
//! 定义前端 → 后端的命令格式和后端 → 前端的响应格式。
//! 所有消息均为 JSON，通过 `serde` 序列化/反序列化。

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::xun_core::operation::{OperationResult, Preview};
use crate::xun_core::value::Table;

/// 前端发送的 WebSocket 命令。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type", content = "payload")]
pub enum WsCommand {
    /// 查询类命令：返回表格数据。
    Query {
        /// 命令名称（如 "bookmark.list", "proxy.show"）
        command: String,
        /// 命令参数
        #[serde(default)]
        args: Vec<String>,
    },
    /// 操作预览：返回 Preview 描述。
    PreviewOp {
        /// 操作名称
        operation: String,
        /// 操作参数
        #[serde(default)]
        args: Vec<String>,
    },
    /// 确认执行操作。
    ConfirmOp {
        /// 操作名称（必须与上次 PreviewOp 一致）
        operation: String,
        /// 操作参数（必须与上次 PreviewOp 一致）
        #[serde(default)]
        args: Vec<String>,
    },
    /// 取消当前操作。
    CancelOp,
}

/// 后端返回的 WebSocket 响应。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type", content = "payload")]
pub enum WsResponse {
    /// 查询结果。
    QueryResult { table: Table },
    /// 操作预览。
    PreviewResult { preview: Preview },
    /// 操作执行结果。
    OpResult { result: OperationResult },
    /// 错误。
    Error { message: String, code: String },
    /// 连接确认。
    Connected,
}

/// 错误码。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub enum WsErrorCode {
    /// 命令未找到。
    NotFound,
    /// 参数错误。
    InvalidArgs,
    /// 操作执行失败。
    ExecutionFailed,
    /// 需要先预览。
    PreviewRequired,
    /// 未知错误。
    Unknown,
}

impl WsErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotFound => "NOT_FOUND",
            Self::InvalidArgs => "INVALID_ARGS",
            Self::ExecutionFailed => "EXECUTION_FAILED",
            Self::PreviewRequired => "PREVIEW_REQUIRED",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl WsResponse {
    /// 创建查询结果响应。
    pub fn query_result(table: Table) -> Self {
        Self::QueryResult { table }
    }

    /// 创建预览结果响应。
    pub fn preview_result(preview: Preview) -> Self {
        Self::PreviewResult { preview }
    }

    /// 创建操作结果响应。
    pub fn op_result(result: OperationResult) -> Self {
        Self::OpResult { result }
    }

    /// 创建错误响应。
    pub fn error(message: impl Into<String>, code: WsErrorCode) -> Self {
        Self::Error {
            message: message.into(),
            code: code.as_str().to_string(),
        }
    }

    /// 创建连接确认响应。
    pub fn connected() -> Self {
        Self::Connected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_command_deserializes_query() {
        let json = r#"{"type":"Query","payload":{"command":"bookmark.list","args":["--tag","shell"]}}"#;
        let cmd: WsCommand = serde_json::from_str(json).unwrap();
        match cmd {
            WsCommand::Query { command, args } => {
                assert_eq!(command, "bookmark.list");
                assert_eq!(args, vec!["--tag", "shell"]);
            }
            _ => panic!("expected Query"),
        }
    }

    #[test]
    fn ws_command_deserializes_preview_op() {
        let json = r#"{"type":"PreviewOp","payload":{"operation":"backup.create","args":["--path","C:\\temp"]}}"#;
        let cmd: WsCommand = serde_json::from_str(json).unwrap();
        match cmd {
            WsCommand::PreviewOp { operation, args } => {
                assert_eq!(operation, "backup.create");
                assert_eq!(args, vec!["--path", "C:\\temp"]);
            }
            _ => panic!("expected PreviewOp"),
        }
    }

    #[test]
    fn ws_command_deserializes_confirm_op() {
        let json = r#"{"type":"ConfirmOp","payload":{"operation":"backup.create","args":[]}}"#;
        let cmd: WsCommand = serde_json::from_str(json).unwrap();
        match cmd {
            WsCommand::ConfirmOp { operation, args } => {
                assert_eq!(operation, "backup.create");
                assert!(args.is_empty());
            }
            _ => panic!("expected ConfirmOp"),
        }
    }

    #[test]
    fn ws_command_deserializes_cancel_op() {
        let json = r#"{"type":"CancelOp"}"#;
        let cmd: WsCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(cmd, WsCommand::CancelOp));
    }

    #[test]
    fn ws_command_deserializes_query_no_args() {
        let json = r#"{"type":"Query","payload":{"command":"proxy.show"}}"#;
        let cmd: WsCommand = serde_json::from_str(json).unwrap();
        match cmd {
            WsCommand::Query { command, args } => {
                assert_eq!(command, "proxy.show");
                assert!(args.is_empty());
            }
            _ => panic!("expected Query"),
        }
    }

    #[test]
    fn ws_response_serializes_query_result() {
        use crate::xun_core::value::{ColumnDef, ValueKind};

        let table = Table::new(vec![
            ColumnDef::new("name", ValueKind::String),
            ColumnDef::new("count", ValueKind::Int),
        ]);
        let resp = WsResponse::query_result(table);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("QueryResult"));
        assert!(json.contains("name"));
    }

    #[test]
    fn ws_response_serializes_preview_result() {
        use crate::xun_core::operation::{Change, RiskLevel};

        let preview = Preview::new("Delete files")
            .add_change(Change::new("delete", "/tmp/test.txt"))
            .with_risk_level(RiskLevel::High);
        let resp = WsResponse::preview_result(preview);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("PreviewResult"));
        assert!(json.contains("Delete files"));
    }

    #[test]
    fn ws_response_serializes_op_result() {
        let result = OperationResult::new()
            .with_changes_applied(5)
            .with_duration_ms(120);
        let resp = WsResponse::op_result(result);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("OpResult"));
        assert!(json.contains("5"));
    }

    #[test]
    fn ws_response_serializes_error() {
        let resp = WsResponse::error("command not found", WsErrorCode::NotFound);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Error"));
        assert!(json.contains("NOT_FOUND"));
        assert!(json.contains("command not found"));
    }

    #[test]
    fn ws_response_serializes_connected() {
        let resp = WsResponse::connected();
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Connected"));
    }

    #[test]
    fn ws_response_roundtrip_query() {
        use crate::xun_core::value::{ColumnDef, ValueKind};

        let table = Table::new(vec![ColumnDef::new("id", ValueKind::Int)]);
        let resp = WsResponse::query_result(table);
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: WsResponse = serde_json::from_str(&json).unwrap();
        match parsed {
            WsResponse::QueryResult { table: t } => {
                assert_eq!(t.columns.len(), 1);
                assert_eq!(t.columns[0].name, "id");
            }
            _ => panic!("expected QueryResult"),
        }
    }

    #[test]
    fn ws_error_code_as_str() {
        assert_eq!(WsErrorCode::NotFound.as_str(), "NOT_FOUND");
        assert_eq!(WsErrorCode::InvalidArgs.as_str(), "INVALID_ARGS");
        assert_eq!(WsErrorCode::ExecutionFailed.as_str(), "EXECUTION_FAILED");
        assert_eq!(WsErrorCode::PreviewRequired.as_str(), "PREVIEW_REQUIRED");
        assert_eq!(WsErrorCode::Unknown.as_str(), "UNKNOWN");
    }
}
