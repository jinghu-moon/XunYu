use super::*;

pub(in crate::commands::dashboard) async fn env_ws(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| async move {
        handle_env_ws(socket).await;
    })
}

async fn handle_env_ws(mut socket: WebSocket) {
    let mut rx = env_event_sender().subscribe();
    let _ = socket
        .send(Message::Text(
            r#"{"type":"connected","channel":"env"}"#.into(),
        ))
        .await;

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(msg) => {
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        if socket.send(Message::Text(r#"{"type":"env.refresh"}"#.to_string().into())).await.is_err() {
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
