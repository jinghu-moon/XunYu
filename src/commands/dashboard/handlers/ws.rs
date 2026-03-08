use super::*;

// SECURITY: /ws 会推送本地文件系统变化事件。当前 Dashboard 仅绑定 127.0.0.1，
// 风险可控。若未来需要开放网络访问，必须增加鉴权与访问控制。
#[cfg(feature = "diff")]
pub(in crate::commands::dashboard) async fn ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<super::super::DashboardState>,
) -> Response {
    ws.on_upgrade(move |socket| async move {
        handle_ws(socket, state.subscribe_events()).await;
    })
}

#[cfg(feature = "diff")]
async fn handle_ws(
    mut socket: axum::extract::ws::WebSocket,
    mut event_rx: tokio::sync::broadcast::Receiver<String>,
) {
    use axum::extract::ws::Message;

    let _ = socket
        .send(Message::Text(r#"{"type":"connected"}"#.to_string().into()))
        .await;

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
                        if socket.send(Message::Text(r#"{"type":"refresh"}"#.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
}
