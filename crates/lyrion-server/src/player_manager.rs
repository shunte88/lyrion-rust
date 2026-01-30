//! Player manager for coordinating player state and commands

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use lyrion_core::{StreamingController, StreamingEvent, Song};

/// Manager for all player streaming controllers
pub struct PlayerManager {
    controllers: Arc<RwLock<HashMap<Uuid, StreamingController>>>,
}

impl PlayerManager {
    pub fn new() -> Self {
        Self {
            controllers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create streaming controller for a player
    pub async fn get_controller(&self, player_id: Uuid) -> Option<StreamingController> {
        let controllers = self.controllers.read().await;
        controllers.get(&player_id).cloned()
    }

    /// Register a new player
    pub async fn register_player(&self, player_id: Uuid) {
        let mut controllers = self.controllers.write().await;
        controllers.insert(player_id, StreamingController::new(player_id));
        tracing::info!("Registered streaming controller for player {}", player_id);
    }

    /// Remove a player
    pub async fn unregister_player(&self, player_id: &Uuid) {
        let mut controllers = self.controllers.write().await;
        controllers.remove(player_id);
        tracing::info!("Unregistered streaming controller for player {}", player_id);
    }

    /// Send event to player
    pub async fn send_event(&self, player_id: Uuid, event: StreamingEvent) -> Result<(), String> {
        let mut controllers = self.controllers.write().await;

        if let Some(controller) = controllers.get_mut(&player_id) {
            controller.handle_event(event)
        } else {
            Err(format!("Player {} not found", player_id))
        }
    }

    /// Play track on player
    pub async fn play_track(&self, player_id: Uuid, track_id: i64, url: String) -> Result<(), String> {
        let mut controllers = self.controllers.write().await;

        if let Some(controller) = controllers.get_mut(&player_id) {
            let song = Song {
                track_id,
                url,
                duration: None, // TODO: Get from database
            };

            controller.enqueue(song);
            controller.handle_event(StreamingEvent::Play)
        } else {
            Err(format!("Player {} not found", player_id))
        }
    }

    /// Get all player IDs
    pub async fn get_player_ids(&self) -> Vec<Uuid> {
        let controllers = self.controllers.read().await;
        controllers.keys().copied().collect()
    }
}

impl Default for PlayerManager {
    fn default() -> Self {
        Self::new()
    }
}
