// WebSocket handler for real-time updates to web clients

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::AppState;

// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    #[serde(rename = "player_status")]
    PlayerStatus(PlayerStatusUpdate),

    #[serde(rename = "player_connected")]
    PlayerConnected(PlayerInfo),

    #[serde(rename = "player_disconnected")]
    PlayerDisconnected { player_id: String },

    #[serde(rename = "track_started")]
    TrackStarted(TrackStartedEvent),

    #[serde(rename = "progress_update")]
    ProgressUpdate(ProgressUpdateEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStatusUpdate {
    pub player_id: String,
    pub playing: bool,
    pub position: Option<f64>,
    pub volume: Option<i32>,
    pub current_track_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: String,
    pub name: String,
    pub model: String,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackStartedEvent {
    pub player_id: String,
    pub track_id: i64,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdateEvent {
    pub player_id: String,
    pub position: f64,
    pub duration: f64,
}

/// WebSocket upgrade handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(mut socket: WebSocket, state: AppState) {
    // Send initial connection message
    let welcome = WsMessage::PlayerStatus(PlayerStatusUpdate {
        player_id: "system".to_string(),
        playing: false,
        position: None,
        volume: None,
        current_track_id: None,
    });

    if let Ok(msg) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(msg)).await.is_err() {
            return;
        }
    }

    tracing::info!("WebSocket client connected");

    // Subscribe to broadcast channel for player status updates
    let mut ws_rx = state.ws_broadcast.subscribe();
    let mut ping_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

    loop {
        tokio::select! {
            // Receive messages from broadcast channel and forward to client
            msg = ws_rx.recv() => {
                match msg {
                    Ok(ws_msg) => {
                        if let Ok(json) = serde_json::to_string(&ws_msg) {
                            if socket.send(Message::Text(json)).await.is_err() {
                                tracing::warn!("Failed to send message to WebSocket client");
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!("WebSocket client lagged, skipped {} messages", skipped);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::error!("Broadcast channel closed");
                        break;
                    }
                }
            }
            // Handle messages from client
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        tracing::debug!("WebSocket received: {}", text);
                        // Handle client messages if needed
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!("WebSocket client disconnected");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Client responded to our ping
                    }
                    _ => {}
                }
            }
            // Send periodic ping to keep connection alive
            _ = ping_interval.tick() => {
                if socket.send(Message::Ping(vec![])).await.is_err() {
                    tracing::warn!("Failed to send ping, closing connection");
                    break;
                }
            }
        }
    }
}
