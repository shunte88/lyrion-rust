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
async fn handle_socket(mut socket: WebSocket, _state: AppState) {
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

    // In a real implementation, this would:
    // 1. Subscribe to a global broadcast channel
    // 2. Forward player status updates to this client
    // 3. Handle client messages (subscriptions, commands, etc.)

    // For now, just handle incoming messages and keep connection alive
    let mut ping_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

    loop {
        tokio::select! {
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
            _ = ping_interval.tick() => {
                // Send periodic ping to keep connection alive
                if socket.send(Message::Ping(vec![])).await.is_err() {
                    tracing::warn!("Failed to send ping, closing connection");
                    break;
                }
            }
        }
    }
}

// TODO: Add global broadcast channel to AppState for sending updates to all connected clients
// For now, this is a stub that demonstrates the structure
