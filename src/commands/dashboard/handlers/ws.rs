use super::*;
use crate::xun_core::value::{ColumnDef, Record, Table, Value, ValueKind};
use crate::xun_core::ws_protocol::{WsCommand, WsErrorCode, WsResponse};

// SECURITY: /ws 支持双向命令分发 + 事件推送。当前 Dashboard 仅绑定 127.0.0.1，
// 风险可控。若未来需要开放网络访问，必须增加鉴权与访问控制。
pub(in crate::commands::dashboard) async fn ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<super::super::DashboardState>,
) -> Response {
    ws.on_upgrade(move |socket| async move {
        handle_ws(socket, state).await;
    })
}

async fn handle_ws(
    mut socket: axum::extract::ws::WebSocket,
    state: super::super::DashboardState,
) {
    use axum::extract::ws::Message;

    let _ = socket
        .send(Message::Text(
            serde_json::to_string(&WsResponse::connected())
                .unwrap_or_default()
                .into(),
        ))
        .await;

    #[cfg(feature = "diff")]
    {
        let mut event_rx = state.subscribe_events();
        loop {
            tokio::select! {
                event = event_rx.recv() => {
                    match event {
                        Ok(msg) => {
                            if socket.send(Message::Text(msg.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            let refresh = r#"{"type":"refresh"}"#.to_string();
                            if socket.send(Message::Text(refresh.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                incoming = socket.recv() => {
                    if !handle_incoming(&mut socket, &incoming, &state).await {
                        break;
                    }
                }
            }
        }
    }

    #[cfg(not(feature = "diff"))]
    {
        loop {
            let incoming = socket.recv().await;
            if !handle_incoming(&mut socket, &incoming, &state).await {
                break;
            }
        }
    }
}

/// 处理 incoming 消息，返回 false 表示应断开连接。
async fn handle_incoming(
    socket: &mut axum::extract::ws::WebSocket,
    incoming: &Option<Result<axum::extract::ws::Message, axum::Error>>,
    state: &super::super::DashboardState,
) -> bool {
    use axum::extract::ws::Message;

    match incoming {
        Some(Ok(Message::Close(_))) | None => false,
        Some(Ok(Message::Ping(payload))) => {
            socket.send(Message::Pong(payload.clone())).await.is_ok()
        }
        Some(Ok(Message::Text(text))) => {
            let response = dispatch_command(text, state).await;
            let json = serde_json::to_string(&response).unwrap_or_default();
            socket.send(Message::Text(json.into())).await.is_ok()
        }
        Some(Ok(_)) => true,
        Some(Err(_)) => false,
    }
}

async fn dispatch_command(text: &str, state: &super::super::DashboardState) -> WsResponse {
    let cmd: WsCommand = match serde_json::from_str(text) {
        Ok(cmd) => cmd,
        Err(e) => {
            return WsResponse::error(
                format!("invalid command: {e}"),
                WsErrorCode::InvalidArgs,
            );
        }
    };

    match cmd {
        WsCommand::Query { command, args } => dispatch_query(&command, &args).await,
        WsCommand::PreviewOp { operation, args } => {
            dispatch_preview(&operation, &args, state).await
        }
        WsCommand::ConfirmOp { operation, args } => {
            dispatch_confirm(&operation, &args, state).await
        }
        WsCommand::CancelOp => WsResponse::op_result(
            crate::xun_core::operation::OperationResult::new().with_changes_applied(0),
        ),
    }
}

async fn dispatch_query(command: &str, args: &[String]) -> WsResponse {
    match command {
        "bookmark.list" => query_bookmark_list(args).await,
        "port.list" => query_port_list().await,
        "proxy.show" => query_proxy_show().await,
        "config.show" => query_config_show().await,
        _ => WsResponse::error(
            format!("unknown query command: {command}"),
            WsErrorCode::NotFound,
        ),
    }
}

async fn dispatch_preview(
    operation: &str,
    _args: &[String],
    _state: &super::super::DashboardState,
) -> WsResponse {
    // TODO: wire to GuardedTaskService when preview-confirm flow is unified
    WsResponse::error(
        format!("preview for '{operation}' not yet supported via WS; use HTTP API"),
        WsErrorCode::NotFound,
    )
}

async fn dispatch_confirm(
    operation: &str,
    _args: &[String],
    _state: &super::super::DashboardState,
) -> WsResponse {
    // TODO: wire to GuardedTaskService when preview-confirm flow is unified
    WsResponse::error(
        format!("confirm for '{operation}' not yet supported via WS; use HTTP API"),
        WsErrorCode::NotFound,
    )
}

// --- Query implementations ---

async fn query_bookmark_list(_args: &[String]) -> WsResponse {
    let db = match crate::bookmark::storage::load_strict(&crate::bookmark::storage::db_path()) {
        Ok(db) => db,
        Err(e) => return WsResponse::error(format!("{e}"), WsErrorCode::ExecutionFailed),
    };

    let columns = vec![
        ColumnDef::new("name", ValueKind::String),
        ColumnDef::new("path", ValueKind::String),
        ColumnDef::new("tags", ValueKind::String),
        ColumnDef::new("visits", ValueKind::Int),
        ColumnDef::new("last_visited", ValueKind::String),
    ];

    let mut table = Table::new(columns);
    for (name, entry) in db {
        let mut row = Record::new();
        row.insert("name".into(), Value::String(name));
        row.insert("path".into(), Value::String(entry.path));
        row.insert("tags".into(), Value::String(entry.tags.join(",")));
        row.insert("visits".into(), Value::Int(entry.visit_count as i64));
        row.insert(
            "last_visited".into(),
            Value::String(format_timestamp(entry.last_visited)),
        );
        table.push_row(row);
    }

    WsResponse::query_result(table)
}

async fn query_port_list() -> WsResponse {
    let tcp = ports::list_tcp_listeners();
    let udp = ports::list_udp_endpoints();
    let items: Vec<_> = tcp.into_iter().chain(udp).collect();

    let columns = vec![
        ColumnDef::new("port", ValueKind::Int),
        ColumnDef::new("pid", ValueKind::Int),
        ColumnDef::new("process", ValueKind::String),
        ColumnDef::new("protocol", ValueKind::String),
        ColumnDef::new("exe_path", ValueKind::String),
    ];

    let mut table = Table::new(columns);
    for item in items {
        let mut row = Record::new();
        row.insert("port".into(), Value::Int(item.port as i64));
        row.insert("pid".into(), Value::Int(item.pid as i64));
        row.insert("process".into(), Value::String(item.name));
        row.insert(
            "protocol".into(),
            Value::String(format!("{:?}", item.protocol)),
        );
        row.insert("exe_path".into(), Value::String(item.exe_path));
        table.push_row(row);
    }

    WsResponse::query_result(table)
}

async fn query_proxy_show() -> WsResponse {
    let proxy_cfg = config::load_config().proxy;

    let columns = vec![
        ColumnDef::new("key", ValueKind::String),
        ColumnDef::new("value", ValueKind::String),
    ];

    let mut table = Table::new(columns);
    if let Some(url) = proxy_cfg.default_url {
        let mut row = Record::new();
        row.insert("key".into(), Value::String("default_url".into()));
        row.insert("value".into(), Value::String(url));
        table.push_row(row);
    }
    if let Some(no) = proxy_cfg.noproxy {
        let mut row = Record::new();
        row.insert("key".into(), Value::String("noproxy".into()));
        row.insert("value".into(), Value::String(no));
        table.push_row(row);
    }

    WsResponse::query_result(table)
}

async fn query_config_show() -> WsResponse {
    let cfg = config::load_config();

    let columns = vec![
        ColumnDef::new("key", ValueKind::String),
        ColumnDef::new("value", ValueKind::String),
    ];

    let mut table = Table::new(columns);
    let json_val = serde_json::to_value(&cfg).unwrap_or_default();
    if let serde_json::Value::Object(map) = json_val {
        for (k, v) in map {
            let mut row = Record::new();
            row.insert("key".into(), Value::String(k));
            row.insert(
                "value".into(),
                Value::String(match v {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                }),
            );
            table.push_row(row);
        }
    }

    WsResponse::query_result(table)
}

fn format_timestamp(ts: u64) -> String {
    if ts == 0 {
        return String::new();
    }
    match chrono::DateTime::from_timestamp(ts as i64, 0) {
        Some(dt) => {
            let local = dt.with_timezone(&chrono::Local);
            local.format("%Y-%m-%d %H:%M:%S").to_string()
        }
        None => ts.to_string(),
    }
}
