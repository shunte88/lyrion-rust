//! Player state machine
//! Ported from Slim/Player/StreamingController.pm

use std::collections::VecDeque;
use std::time::Duration;
use uuid::Uuid;

pub type PlayerId = Uuid;

/// Streaming state machine states
/// From StreamingController.pm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingState {
    Idle,
    Streaming,
    TrackWait,
    StreamOut,
}

/// Playing state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayingState {
    Stopped,
    Buffering,
    Playing,
    Paused,
}

/// Song representation in the queue
#[derive(Debug, Clone)]
pub struct Song {
    pub track_id: i64,
    pub url: String,
    pub duration: Option<Duration>,
}

/// Frame data for sync calculations
#[derive(Debug, Clone)]
pub struct FrameData {
    pub byte_offset: u64,
    pub timestamp: Duration,
}

/// Main player state
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub mac_address: [u8; 6],
    pub streaming_state: StreamingState,
    pub playing_state: PlayingState,
    pub song_queue: VecDeque<Song>,
    pub frame_data: Vec<FrameData>,
    pub current_position: Duration,
    pub volume: u8,
}

impl Player {
    pub fn new(mac_address: [u8; 6], name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            mac_address,
            streaming_state: StreamingState::Idle,
            playing_state: PlayingState::Stopped,
            song_queue: VecDeque::new(),
            frame_data: Vec::new(),
            current_position: Duration::ZERO,
            volume: 50,
        }
    }

    /// Transition streaming state
    pub fn set_streaming_state(&mut self, state: StreamingState) {
        tracing::debug!(
            player_id = ?self.id,
            old_state = ?self.streaming_state,
            new_state = ?state,
            "Streaming state transition"
        );
        self.streaming_state = state;
    }

    /// Transition playing state
    pub fn set_playing_state(&mut self, state: PlayingState) {
        tracing::debug!(
            player_id = ?self.id,
            old_state = ?self.playing_state,
            new_state = ?state,
            "Playing state transition"
        );
        self.playing_state = state;
    }

    /// Add track to queue
    pub fn enqueue(&mut self, song: Song) {
        self.song_queue.push_back(song);
    }

    /// Get current song
    pub fn current_song(&self) -> Option<&Song> {
        self.song_queue.front()
    }

    /// Skip to next track
    pub fn skip(&mut self) -> Option<Song> {
        self.song_queue.pop_front()
    }
}
