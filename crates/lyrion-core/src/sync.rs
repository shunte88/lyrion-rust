//! Multi-room synchronization for Lyrion Music Server
//!
//! Implements precise audio synchronization across multiple Squeezebox players
//! with < 10ms accuracy, ported from Slim::Player::Sync

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::sync::Arc;

/// Sync check interval (950ms)
pub const CHECK_SYNC_INTERVAL: Duration = Duration::from_millis(950);

/// Minimum deviation to trigger adjustment (10ms)
pub const MIN_DEVIATION_ADJUST: Duration = Duration::from_millis(10);

/// Maximum deviation to trigger adjustment (10 seconds)
pub const MAX_DEVIATION_ADJUST: Duration = Duration::from_secs(10);

/// Threshold for considering a play point "recent" (3 seconds)
pub const PLAYPOINT_RECENT_THRESHOLD: Duration = Duration::from_secs(3);

/// A unique identifier for a player
pub type PlayerId = uuid::Uuid;

/// A unique identifier for a sync group
pub type SyncGroupId = uuid::Uuid;

/// Play point - timestamp and position in stream
#[derive(Debug, Clone)]
pub struct PlayPoint {
    /// When this play point was measured
    pub timestamp: Instant,
    /// Current playback position in seconds
    pub position: f64,
}

/// Sync group containing multiple players playing in sync
#[derive(Debug, Clone)]
pub struct SyncGroup {
    /// Unique identifier for this sync group
    pub id: SyncGroupId,
    /// Master player (controls playback for the group)
    pub master: PlayerId,
    /// Slave players (synchronized to master)
    pub slaves: Vec<PlayerId>,
    /// Last time we checked sync
    pub next_check_sync_time: Instant,
}

impl SyncGroup {
    /// Create a new sync group with a master player
    pub fn new(master: PlayerId) -> Self {
        Self {
            id: SyncGroupId::new_v4(),
            master,
            slaves: Vec::new(),
            next_check_sync_time: Instant::now(),
        }
    }

    /// Add a slave player to the group
    pub fn add_slave(&mut self, player: PlayerId) {
        if !self.slaves.contains(&player) {
            self.slaves.push(player);
        }
    }

    /// Remove a slave player from the group
    pub fn remove_slave(&mut self, player: &PlayerId) {
        self.slaves.retain(|p| p != player);
    }

    /// Get all players in the group (master + slaves)
    pub fn all_players(&self) -> Vec<PlayerId> {
        let mut players = vec![self.master];
        players.extend(&self.slaves);
        players
    }

    /// Check if this player is the master
    pub fn is_master(&self, player: &PlayerId) -> bool {
        self.master == *player
    }

    /// Check if this player is a slave
    pub fn is_slave(&self, player: &PlayerId) -> bool {
        self.slaves.contains(player)
    }

    /// Get the number of players in this group
    pub fn player_count(&self) -> usize {
        1 + self.slaves.len() // master + slaves
    }

    /// Check if the group is empty (only has master)
    pub fn is_empty(&self) -> bool {
        self.slaves.is_empty()
    }
}

/// Sync adjustment command
#[derive(Debug, Clone)]
pub enum SyncAdjustment {
    /// Skip ahead by this duration (for players ahead of reference)
    SkipAhead { player: PlayerId, delta: Duration },
    /// Pause for this duration (for players behind reference)
    PauseFor { player: PlayerId, delta: Duration },
}

/// Player play point with player ID
#[derive(Debug, Clone)]
struct PlayerPlayPoint {
    player: PlayerId,
    position: f64, // in seconds, adjusted for playDelay
}

/// Manages sync groups and synchronization timing
pub struct SyncManager {
    /// Map of player ID to sync group ID
    player_groups: Arc<RwLock<HashMap<PlayerId, SyncGroupId>>>,
    /// Map of sync group ID to sync group
    sync_groups: Arc<RwLock<HashMap<SyncGroupId, SyncGroup>>>,
    /// Map of player ID to current play point
    play_points: Arc<RwLock<HashMap<PlayerId, PlayPoint>>>,
    /// Map of player ID to play delay (in milliseconds)
    play_delays: Arc<RwLock<HashMap<PlayerId, i32>>>,
}

impl SyncManager {
    /// Create a new sync manager
    pub fn new() -> Self {
        Self {
            player_groups: Arc::new(RwLock::new(HashMap::new())),
            sync_groups: Arc::new(RwLock::new(HashMap::new())),
            play_points: Arc::new(RwLock::new(HashMap::new())),
            play_delays: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new sync group with a master player
    pub async fn create_group(&self, master: PlayerId) -> SyncGroupId {
        let group = SyncGroup::new(master);
        let group_id = group.id;

        let mut groups = self.sync_groups.write().await;
        let mut player_groups = self.player_groups.write().await;

        groups.insert(group_id, group);
        player_groups.insert(master, group_id);

        group_id
    }

    /// Add a player to an existing sync group
    pub async fn add_to_group(&self, group_id: SyncGroupId, player: PlayerId) -> Result<(), String> {
        let mut groups = self.sync_groups.write().await;
        let mut player_groups = self.player_groups.write().await;

        let group = groups
            .get_mut(&group_id)
            .ok_or_else(|| format!("Sync group {} not found", group_id))?;

        group.add_slave(player);
        player_groups.insert(player, group_id);

        Ok(())
    }

    /// Remove a player from their sync group
    pub async fn remove_from_group(&self, player: PlayerId) -> Result<(), String> {
        let mut player_groups = self.player_groups.write().await;
        let mut groups = self.sync_groups.write().await;

        let group_id = player_groups
            .remove(&player)
            .ok_or_else(|| format!("Player {} not in any sync group", player))?;

        let group = groups
            .get_mut(&group_id)
            .ok_or_else(|| format!("Sync group {} not found", group_id))?;

        if group.is_master(&player) {
            // Master is leaving - dissolve the group
            for slave in &group.slaves {
                player_groups.remove(slave);
            }
            groups.remove(&group_id);
        } else {
            // Slave is leaving - just remove from group
            group.remove_slave(&player);

            // If no more slaves, remove the group
            if group.slaves.is_empty() {
                player_groups.remove(&group.master);
                groups.remove(&group_id);
            }
        }

        Ok(())
    }

    /// Get the sync group for a player
    pub async fn get_group(&self, player: PlayerId) -> Option<SyncGroup> {
        let player_groups = self.player_groups.read().await;
        let groups = self.sync_groups.read().await;

        player_groups
            .get(&player)
            .and_then(|group_id| groups.get(group_id))
            .cloned()
    }

    /// Check if a player is in a sync group
    pub async fn is_synced(&self, player: PlayerId) -> bool {
        let player_groups = self.player_groups.read().await;
        player_groups.contains_key(&player)
    }

    /// Update a player's play point
    pub async fn update_play_point(&self, player: PlayerId, position: f64) {
        let mut play_points = self.play_points.write().await;
        play_points.insert(
            player,
            PlayPoint {
                timestamp: Instant::now(),
                position,
            },
        );
    }

    /// Set a player's play delay (in milliseconds)
    pub async fn set_play_delay(&self, player: PlayerId, delay_ms: i32) {
        let mut play_delays = self.play_delays.write().await;
        play_delays.insert(player, delay_ms);
    }

    /// Check sync for a group and return adjustments
    /// Returns a list of sync adjustments to apply
    ///
    /// Algorithm (ported from Slim::Player::StreamingController::_CheckSync):
    /// 1. Collect recent play points from all players
    /// 2. Sort by decreasing apparent start time (most ahead first)
    /// 3. Find reference player (most behind that doesn't support skipAhead)
    /// 4. Calculate delta for each player vs reference
    /// 5. Apply adjustments if delta is between MIN and MAX thresholds
    pub async fn check_sync(&self, group_id: SyncGroupId) -> Vec<SyncAdjustment> {
        let mut groups = self.sync_groups.write().await;
        let play_points = self.play_points.read().await;
        let play_delays = self.play_delays.read().await;

        let group = match groups.get_mut(&group_id) {
            Some(g) => g,
            None => return vec![],
        };

        // Check if it's time to check sync
        let now = Instant::now();
        if now < group.next_check_sync_time {
            return vec![];
        }
        group.next_check_sync_time = now + CHECK_SYNC_INTERVAL;

        // Need at least 2 players to sync
        if group.player_count() < 2 {
            return vec![];
        }

        // Collect recent play points from all players
        let recent_threshold = now - PLAYPOINT_RECENT_THRESHOLD;
        let mut player_play_points: Vec<PlayerPlayPoint> = Vec::new();

        for player in group.all_players() {
            if let Some(play_point) = play_points.get(&player) {
                // Check if play point is recent enough
                if play_point.timestamp > recent_threshold {
                    // Adjust position by play delay
                    let delay_secs = *play_delays.get(&player).unwrap_or(&0) as f64 / 1000.0;
                    player_play_points.push(PlayerPlayPoint {
                        player,
                        position: play_point.position + delay_secs,
                    });
                }
            }
        }

        // Need play points from all players
        if player_play_points.len() < group.player_count() {
            return vec![];
        }

        // Sort by decreasing position (most ahead first)
        player_play_points.sort_by(|a, b| {
            b.position
                .partial_cmp(&a.position)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Find reference player (most behind)
        // In a real implementation, we'd check if player can skipAhead
        // For now, assume all players support skipAhead, so reference is last
        let reference_idx = player_play_points.len() - 1;
        let reference_position = player_play_points[reference_idx].position;

        // Calculate adjustments
        let mut adjustments = Vec::new();

        for (i, player_point) in player_play_points.iter().enumerate() {
            if i == reference_idx {
                continue; // Skip reference player
            }

            let delta_secs = (player_point.position - reference_position).abs();
            let delta = Duration::from_secs_f64(delta_secs);

            // Skip if delta is outside adjustment range
            if delta < MIN_DEVIATION_ADJUST || delta > MAX_DEVIATION_ADJUST {
                continue;
            }

            // Apply adjustment
            if i < reference_idx {
                // Player is ahead of reference - skip ahead
                adjustments.push(SyncAdjustment::SkipAhead {
                    player: player_point.player,
                    delta,
                });
            } else {
                // Player is behind reference - pause
                adjustments.push(SyncAdjustment::PauseFor {
                    player: player_point.player,
                    delta,
                });
            }
        }

        adjustments
    }

    /// Get all sync groups
    pub async fn get_all_groups(&self) -> Vec<SyncGroup> {
        let groups = self.sync_groups.read().await;
        groups.values().cloned().collect()
    }
}

impl Default for SyncManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_group() {
        let manager = SyncManager::new();
        let master = PlayerId::new_v4();

        let group_id = manager.create_group(master).await;
        let group = manager.get_group(master).await.unwrap();

        assert_eq!(group.master, master);
        assert_eq!(group.slaves.len(), 0);
        assert_eq!(group.id, group_id);
    }

    #[tokio::test]
    async fn test_add_to_group() {
        let manager = SyncManager::new();
        let master = PlayerId::new_v4();
        let slave = PlayerId::new_v4();

        let group_id = manager.create_group(master).await;
        manager.add_to_group(group_id, slave).await.unwrap();

        let group = manager.get_group(master).await.unwrap();
        assert_eq!(group.slaves.len(), 1);
        assert!(group.is_slave(&slave));
    }

    #[tokio::test]
    async fn test_sync_check() {
        let manager = SyncManager::new();
        let master = PlayerId::new_v4();
        let slave = PlayerId::new_v4();

        let group_id = manager.create_group(master).await;
        manager.add_to_group(group_id, slave).await.unwrap();

        // Set play points - slave is 50ms ahead
        manager.update_play_point(master, 10.0).await;
        manager.update_play_point(slave, 10.050).await;

        let adjustments = manager.check_sync(group_id).await;
        assert_eq!(adjustments.len(), 1);

        match &adjustments[0] {
            SyncAdjustment::SkipAhead { player, delta } => {
                assert_eq!(*player, slave);
                assert!(delta.as_millis() >= 50);
            }
            _ => panic!("Expected SkipAhead adjustment"),
        }
    }
}
