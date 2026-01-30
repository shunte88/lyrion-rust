//! Playlist management
//! Handles per-player playlist state and operations

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Represents a single track in a playlist
#[derive(Debug, Clone)]
pub struct PlaylistTrack {
    pub id: i64,
    pub url: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<f64>,
}

/// Player playlist state
#[derive(Debug, Clone)]
pub struct Playlist {
    /// List of tracks in playlist
    pub tracks: Vec<PlaylistTrack>,
    /// Current playing track index
    pub current_index: Option<usize>,
    /// Repeat mode: 0=off, 1=song, 2=playlist
    pub repeat: u8,
    /// Shuffle mode: 0=off, 1=song, 2=album
    pub shuffle: u8,
    /// Whether playback is currently active
    pub playing: bool,
}

impl Playlist {
    /// Create a new empty playlist
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            current_index: None,
            repeat: 0,
            shuffle: 0,
            playing: false,
        }
    }

    /// Add a track to the end of the playlist
    pub fn add_track(&mut self, track: PlaylistTrack) {
        self.tracks.push(track);
        // If this is the first track, set it as current
        if self.tracks.len() == 1 {
            self.current_index = Some(0);
        }
    }

    /// Add multiple tracks to the playlist
    pub fn add_tracks(&mut self, tracks: Vec<PlaylistTrack>) {
        let was_empty = self.tracks.is_empty();
        self.tracks.extend(tracks);
        if was_empty && !self.tracks.is_empty() {
            self.current_index = Some(0);
        }
    }

    /// Insert a track at a specific position
    pub fn insert_track(&mut self, index: usize, track: PlaylistTrack) {
        if index <= self.tracks.len() {
            self.tracks.insert(index, track);
            // Adjust current index if needed
            if let Some(current) = self.current_index {
                if index <= current {
                    self.current_index = Some(current + 1);
                }
            } else if self.tracks.len() == 1 {
                self.current_index = Some(0);
            }
        }
    }

    /// Remove a track at a specific index
    pub fn remove_track(&mut self, index: usize) -> Option<PlaylistTrack> {
        if index < self.tracks.len() {
            let track = self.tracks.remove(index);
            // Adjust current index
            if let Some(current) = self.current_index {
                if index == current {
                    // Removed current track
                    if self.tracks.is_empty() {
                        self.current_index = None;
                    } else if current >= self.tracks.len() {
                        self.current_index = Some(self.tracks.len() - 1);
                    }
                } else if index < current {
                    self.current_index = Some(current - 1);
                }
            }
            Some(track)
        } else {
            None
        }
    }

    /// Clear the playlist
    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current_index = None;
    }

    /// Get the current track
    pub fn current_track(&self) -> Option<&PlaylistTrack> {
        self.current_index.and_then(|i| self.tracks.get(i))
    }

    /// Move to next track
    pub fn next(&mut self) -> Option<&PlaylistTrack> {
        if self.tracks.is_empty() {
            return None;
        }

        self.current_index = match self.current_index {
            Some(current) => {
                let next_index = current + 1;
                if next_index < self.tracks.len() {
                    Some(next_index)
                } else if self.repeat == 2 {
                    // Repeat playlist
                    Some(0)
                } else {
                    // End of playlist
                    Some(current) // Stay at last track
                }
            }
            None => Some(0),
        };

        self.current_track()
    }

    /// Move to previous track
    pub fn previous(&mut self) -> Option<&PlaylistTrack> {
        if self.tracks.is_empty() {
            return None;
        }

        self.current_index = match self.current_index {
            Some(current) => {
                if current > 0 {
                    Some(current - 1)
                } else if self.repeat == 2 {
                    // Repeat playlist - go to end
                    Some(self.tracks.len() - 1)
                } else {
                    // Stay at first track
                    Some(0)
                }
            }
            None => Some(0),
        };

        self.current_track()
    }

    /// Jump to a specific track index
    pub fn jump_to(&mut self, index: usize) -> Option<&PlaylistTrack> {
        if index < self.tracks.len() {
            self.current_index = Some(index);
            self.current_track()
        } else {
            None
        }
    }

    /// Get total number of tracks
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    /// Check if playlist is empty
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Get all tracks
    pub fn all_tracks(&self) -> &[PlaylistTrack] {
        &self.tracks
    }
}

impl Default for Playlist {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages playlists for all players
#[derive(Clone)]
pub struct PlaylistManager {
    /// Map of player MAC to their playlist
    playlists: Arc<RwLock<HashMap<String, Playlist>>>,
}

impl PlaylistManager {
    /// Create a new playlist manager
    pub fn new() -> Self {
        Self {
            playlists: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a player's playlist (creates if doesn't exist)
    pub async fn get_playlist(&self, player_id: &str) -> Playlist {
        let playlists = self.playlists.read().await;
        playlists.get(player_id).cloned().unwrap_or_default()
    }

    /// Update a player's playlist
    pub async fn set_playlist(&self, player_id: &str, playlist: Playlist) {
        let mut playlists = self.playlists.write().await;
        playlists.insert(player_id.to_string(), playlist);
    }

    /// Add a track to a player's playlist
    pub async fn add_track(&self, player_id: &str, track: PlaylistTrack) {
        let mut playlist = self.get_playlist(player_id).await;
        playlist.add_track(track);
        self.set_playlist(player_id, playlist).await;
    }

    /// Clear a player's playlist
    pub async fn clear(&self, player_id: &str) {
        let mut playlists = self.playlists.write().await;
        playlists.remove(player_id);
    }

    /// Get next track for a player
    pub async fn next(&self, player_id: &str) -> Option<PlaylistTrack> {
        let mut playlist = self.get_playlist(player_id).await;
        let track = playlist.next().cloned();
        self.set_playlist(player_id, playlist).await;
        track
    }

    /// Get previous track for a player
    pub async fn previous(&self, player_id: &str) -> Option<PlaylistTrack> {
        let mut playlist = self.get_playlist(player_id).await;
        let track = playlist.previous().cloned();
        self.set_playlist(player_id, playlist).await;
        track
    }

    /// Jump to a specific track index
    pub async fn jump_to(&self, player_id: &str, index: usize) -> Option<PlaylistTrack> {
        let mut playlist = self.get_playlist(player_id).await;
        let track = playlist.jump_to(index).cloned();
        self.set_playlist(player_id, playlist).await;
        track
    }

    /// Set shuffle mode for a player
    pub async fn set_shuffle(&self, player_id: &str, mode: u8) {
        let mut playlist = self.get_playlist(player_id).await;
        playlist.shuffle = mode;
        self.set_playlist(player_id, playlist).await;
    }

    /// Set repeat mode for a player
    pub async fn set_repeat(&self, player_id: &str, mode: u8) {
        let mut playlist = self.get_playlist(player_id).await;
        playlist.repeat = mode;
        self.set_playlist(player_id, playlist).await;
    }

    /// Set playing state for a player
    pub async fn set_playing(&self, player_id: &str, playing: bool) {
        let mut playlist = self.get_playlist(player_id).await;
        playlist.playing = playing;
        self.set_playlist(player_id, playlist).await;
    }

    /// Get playing state for a player
    pub async fn is_playing(&self, player_id: &str) -> bool {
        let playlist = self.get_playlist(player_id).await;
        playlist.playing
    }
}

impl Default for PlaylistManager {
    fn default() -> Self {
        Self::new()
    }
}
