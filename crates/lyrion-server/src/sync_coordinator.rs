//! Sync coordinator
//! Manages the 950ms sync loop and sends adjustment commands to players

use lyrion_core::{SyncManager, SyncAdjustment};
use lyrion_protocol::StreamCommand;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use uuid::Uuid;

/// Player connection for sending commands
pub type PlayerConnection = Arc<RwLock<TcpStream>>;

/// Sync coordinator
/// Runs background task that checks sync every 950ms and sends adjustment commands
pub struct SyncCoordinator {
    sync_manager: Arc<SyncManager>,
    player_connections: Arc<RwLock<HashMap<Uuid, PlayerConnection>>>,
}

impl SyncCoordinator {
    /// Create new sync coordinator
    pub fn new(sync_manager: Arc<SyncManager>) -> Self {
        Self {
            sync_manager,
            player_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a player connection for sending commands
    pub async fn register_player(&self, player_id: Uuid, connection: PlayerConnection) {
        let mut connections = self.player_connections.write().await;
        connections.insert(player_id, connection);
        tracing::debug!("Registered player {} for sync commands", player_id);
    }

    /// Unregister a player connection
    pub async fn unregister_player(&self, player_id: &Uuid) {
        let mut connections = self.player_connections.write().await;
        connections.remove(player_id);
        tracing::debug!("Unregistered player {} from sync commands", player_id);
    }

    /// Start the sync loop
    /// Runs every 950ms and checks sync for all groups
    pub async fn start_sync_loop(self: Arc<Self>) {
        let mut sync_interval = interval(Duration::from_millis(950));

        tracing::info!("Starting sync loop (950ms interval)");

        loop {
            sync_interval.tick().await;

            // Get all sync groups
            let groups = self.sync_manager.get_all_groups().await;

            for group in groups {
                // Check sync for this group
                let adjustments = self.sync_manager.check_sync(group.id).await;

                if !adjustments.is_empty() {
                    tracing::debug!(
                        "Group {} needs {} adjustments",
                        group.id,
                        adjustments.len()
                    );
                }

                // Apply adjustments
                for adjustment in adjustments {
                    if let Err(e) = self.send_adjustment(&adjustment).await {
                        tracing::error!("Failed to send sync adjustment: {}", e);
                    }
                }
            }
        }
    }

    /// Send sync adjustment command to player
    async fn send_adjustment(&self, adjustment: &SyncAdjustment) -> Result<(), String> {
        let (player_id, command) = match adjustment {
            SyncAdjustment::SkipAhead { player, delta } => {
                let interval_ms = delta.as_millis() as u32;
                tracing::info!(
                    "Sync adjustment: player {} skip ahead {}ms",
                    player,
                    interval_ms
                );
                (*player, StreamCommand::SkipAhead { interval_ms })
            }
            SyncAdjustment::PauseFor { player, delta } => {
                let interval_ms = delta.as_millis() as u32;
                tracing::info!(
                    "Sync adjustment: player {} pause for {}ms",
                    player,
                    interval_ms
                );
                (*player, StreamCommand::PauseFor { interval_ms })
            }
        };

        // Get player connection
        let connections = self.player_connections.read().await;
        let connection = connections
            .get(&player_id)
            .ok_or_else(|| format!("Player {} not connected", player_id))?;

        // Encode and send command
        let command_bytes = command.encode();
        let mut stream = connection.write().await;

        stream
            .write_all(&command_bytes)
            .await
            .map_err(|e| format!("Failed to write command: {}", e))?;

        stream
            .flush()
            .await
            .map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(())
    }

    /// Get sync manager reference
    pub fn sync_manager(&self) -> &Arc<SyncManager> {
        &self.sync_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_coordinator_creation() {
        let sync_manager = Arc::new(SyncManager::new());
        let _coordinator = SyncCoordinator::new(sync_manager);
        // Just verify it compiles and constructs
    }
}
