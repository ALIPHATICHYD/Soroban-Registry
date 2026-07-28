use crate::state::AppState;
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    extract::State,
};
use futures_util::{SinkExt, StreamExt};

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(|socket| handle_connection(socket, state))
}

async fn handle_connection(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut broadcaster = state.event_broadcaster.subscribe();

    // Use a channel to coordinate outgoing messages to the sender
    let (tx, mut rx) = tokio::sync::mpsc::channel::<axum::extract::ws::Message>(100);

    // Spawn a task to forward messages from the channel to the WebSocket sender
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Spawn a task to forward broadcast messages to the channel
    let tx_broadcast = tx.clone();
    let broadcast_task = tokio::spawn(async move {
        while let Ok(event) = broadcaster.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                let msg = axum::extract::ws::Message::Text(json);
                if tx_broadcast.send(msg).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                if text == "ping" {
                    let _ = tx
                        .send(axum::extract::ws::Message::Text("pong".to_string()))
                        .await;
                }
            }
            axum::extract::ws::Message::Close(_) => break,
            _ => {}
        }
    }

    broadcast_task.abort();
    send_task.abort();
}
