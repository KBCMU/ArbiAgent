use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use tracing::{info, warn};

use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/ws/arb-feed", get(ws_handler))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to the broadcast channel
    let mut rx = state.arb_tx.subscribe();

    info!("📱 WebSocket client connected");

    // Send initial state: all current events with odds
    let events = state.cache.get_all_events_with_odds();
    let initial_msg = serde_json::json!({
        "type": "initial_state",
        "data": {
            "event_count": events.len(),
            "arb_count": state.cache.get_all_active_arbs().len(),
            "message": "Connected to ArbiAgent real-time feed",
        }
    });
    if let Err(e) = sender
        .send(Message::Text(initial_msg.to_string().into()))
        .await
    {
        warn!("Failed to send initial state: {}", e);
        return;
    }

    // Spawn a task to forward broadcast messages to this WebSocket client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(e) => {
                    warn!("Failed to serialize WS message: {}", e);
                    continue;
                }
            };

            if sender.send(Message::Text(json.into())).await.is_err() {
                // Client disconnected
                break;
            }
        }
    });

    // Listen for incoming messages (e.g., ping/pong, close)
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Ping(_data) => {
                    // Pong is handled automatically by axum
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete (client disconnect or error)
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    info!("📱 WebSocket client disconnected");
}
