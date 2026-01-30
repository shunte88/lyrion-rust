//! Streaming controller state machine
//! Ported from Slim/Player/StreamingController.pm
//!
//! This handles the complex state management for audio playback including:
//! - Streaming state (IDLE, STREAMING, STREAMOUT, TRACKWAIT)
//! - Playing state (STOPPED, BUFFERING, WAITING_TO_SYNC, PLAYING, PAUSED)
//! - State transitions based on events
//! - Multi-player synchronization

use crate::{PlayerId, player::Song};
use std::time::{Duration, Instant};
use std::collections::VecDeque;

/// Streaming state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingState {
    Idle,
    Streaming,
    StreamOut,
    TrackWait,
}

/// Playing state machine states (audio)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayingState {
    Stopped,
    Buffering,
    WaitingToSync,
    Playing,
    Paused,
}

/// Events that trigger state transitions
#[derive(Debug, Clone, Copy)]
pub enum StreamingEvent {
    Stop,
    Play,
    ContinuePlay,
    Pause,
    Resume,
    Flush,
    Skip,
    JumpToTime(Duration),
    NextTrackReady,
    NextTrackError,
    LocalEndOfStream,
    BufferReady,
    Started,
    Underrun,
    TrackDone,
}

/// Streaming controller for managing playback state
#[derive(Clone)]
pub struct StreamingController {
    /// Master player ID
    pub master_id: PlayerId,

    /// Current streaming state
    pub streaming_state: StreamingState,

    /// Current playing state
    pub playing_state: PlayingState,

    /// Song queue
    pub song_queue: VecDeque<Song>,

    /// Currently playing song
    pub current_song: Option<Song>,

    /// Next track to play
    pub next_track: Option<Song>,

    /// Resume time (for pause)
    pub resume_time: Option<Duration>,

    /// Frame data for sync calculations
    pub frame_data: Vec<(u64, Duration)>,

    /// Last state change timestamp
    pub last_state_change: Instant,

    /// Rebuffering flag
    pub rebuffering: bool,

    /// Consecutive error count
    pub consecutive_errors: u32,

    /// Sync group ID (if part of a sync group)
    pub sync_group_id: Option<String>,
}

impl StreamingController {
    /// Create new streaming controller
    pub fn new(master_id: PlayerId) -> Self {
        Self {
            master_id,
            streaming_state: StreamingState::Idle,
            playing_state: PlayingState::Stopped,
            song_queue: VecDeque::new(),
            current_song: None,
            next_track: None,
            resume_time: None,
            frame_data: Vec::new(),
            last_state_change: Instant::now(),
            rebuffering: false,
            consecutive_errors: 0,
            sync_group_id: None,
        }
    }

    /// Handle an event and transition states
    pub fn handle_event(&mut self, event: StreamingEvent) -> Result<(), String> {
        tracing::debug!(
            player_id = ?self.master_id,
            streaming_state = ?self.streaming_state,
            playing_state = ?self.playing_state,
            event = ?event,
            "Handling streaming event"
        );

        // Validate state combination is valid
        if !self.is_valid_state_combination() {
            return Err(format!(
                "Invalid state combination: streaming={:?}, playing={:?}",
                self.streaming_state, self.playing_state
            ));
        }

        // Handle event based on current state
        match event {
            StreamingEvent::Stop => self.handle_stop(),
            StreamingEvent::Play => self.handle_play(),
            StreamingEvent::ContinuePlay => self.handle_continue_play(),
            StreamingEvent::Pause => self.handle_pause(),
            StreamingEvent::Resume => self.handle_resume(),
            StreamingEvent::Skip => self.handle_skip(),
            StreamingEvent::JumpToTime(time) => self.handle_jump_to_time(time),
            StreamingEvent::NextTrackReady => self.handle_next_track_ready(),
            StreamingEvent::BufferReady => self.handle_buffer_ready(),
            StreamingEvent::Started => self.handle_started(),
            StreamingEvent::LocalEndOfStream => self.handle_end_of_stream(),
            StreamingEvent::Underrun => self.handle_underrun(),
            StreamingEvent::TrackDone => self.handle_track_done(),
            _ => {
                tracing::warn!("Unhandled event: {:?}", event);
                Ok(())
            }
        }
    }

    /// Check if current state combination is valid
    fn is_valid_state_combination(&self) -> bool {
        use StreamingState::*;
        use PlayingState::*;

        match (self.streaming_state, self.playing_state) {
            (Idle, Stopped) => true,
            (Idle, Playing) => true,
            (Idle, Paused) => true,
            (Streaming, Buffering) => true,
            (Streaming, WaitingToSync) => true,
            (Streaming, Playing) => true,
            (Streaming, Paused) => true,
            (StreamOut, Buffering) => true,
            (StreamOut, WaitingToSync) => true,
            (StreamOut, Playing) => true,
            (StreamOut, Paused) => true,
            (TrackWait, Stopped) => true,
            (TrackWait, Playing) => true,
            (TrackWait, Paused) => true,
            _ => false,
        }
    }

    /// Set streaming state
    fn set_streaming_state(&mut self, state: StreamingState) {
        tracing::debug!(
            player_id = ?self.master_id,
            old_state = ?self.streaming_state,
            new_state = ?state,
            "Streaming state transition"
        );
        self.streaming_state = state;
        self.last_state_change = Instant::now();
    }

    /// Set playing state
    fn set_playing_state(&mut self, state: PlayingState) {
        tracing::debug!(
            player_id = ?self.master_id,
            old_state = ?self.playing_state,
            new_state = ?state,
            "Playing state transition"
        );
        self.playing_state = state;
        self.last_state_change = Instant::now();
    }

    // Event handlers

    fn handle_stop(&mut self) -> Result<(), String> {
        self.set_streaming_state(StreamingState::Idle);
        self.set_playing_state(PlayingState::Stopped);
        self.resume_time = None;
        Ok(())
    }

    fn handle_play(&mut self) -> Result<(), String> {
        // Get next track from queue
        if let Some(song) = self.song_queue.pop_front() {
            self.current_song = Some(song);
            self.set_streaming_state(StreamingState::TrackWait);
            self.set_playing_state(PlayingState::Stopped);
            // Signal to start loading track
        } else {
            return Err("No tracks in queue".to_string());
        }
        Ok(())
    }

    fn handle_continue_play(&mut self) -> Result<(), String> {
        match (self.streaming_state, self.playing_state) {
            (StreamingState::Idle, PlayingState::Playing) |
            (StreamingState::Streaming, PlayingState::Playing) |
            (StreamingState::StreamOut, PlayingState::Playing) => {
                // Continue current playback
                Ok(())
            }
            _ => {
                // Start next track
                self.handle_play()
            }
        }
    }

    fn handle_pause(&mut self) -> Result<(), String> {
        if self.playing_state == PlayingState::Playing {
            self.set_playing_state(PlayingState::Paused);
            // Store current position for resume
            self.resume_time = Some(Duration::from_secs(0)); // TODO: Get actual position
        }
        Ok(())
    }

    fn handle_resume(&mut self) -> Result<(), String> {
        if self.playing_state == PlayingState::Paused {
            self.set_playing_state(PlayingState::Playing);
            self.resume_time = None;
        }
        Ok(())
    }

    fn handle_skip(&mut self) -> Result<(), String> {
        // Stop current track and get next
        self.current_song = None;
        self.handle_play()
    }

    fn handle_jump_to_time(&mut self, time: Duration) -> Result<(), String> {
        tracing::debug!("Jumping to time: {:?}", time);
        // TODO: Implement seeking
        Ok(())
    }

    fn handle_next_track_ready(&mut self) -> Result<(), String> {
        if self.streaming_state == StreamingState::TrackWait {
            self.set_streaming_state(StreamingState::Streaming);
            self.set_playing_state(PlayingState::Buffering);
        }
        Ok(())
    }

    fn handle_buffer_ready(&mut self) -> Result<(), String> {
        if self.playing_state == PlayingState::Buffering {
            if self.sync_group_id.is_some() {
                self.set_playing_state(PlayingState::WaitingToSync);
            } else {
                self.set_playing_state(PlayingState::Playing);
            }
        }
        Ok(())
    }

    fn handle_started(&mut self) -> Result<(), String> {
        if self.playing_state == PlayingState::Buffering ||
           self.playing_state == PlayingState::WaitingToSync {
            self.set_playing_state(PlayingState::Playing);
        }
        Ok(())
    }

    fn handle_end_of_stream(&mut self) -> Result<(), String> {
        if self.streaming_state == StreamingState::Streaming {
            self.set_streaming_state(StreamingState::StreamOut);
        }
        Ok(())
    }

    fn handle_underrun(&mut self) -> Result<(), String> {
        if self.playing_state == PlayingState::Playing {
            self.set_playing_state(PlayingState::Buffering);
            self.rebuffering = true;
        }
        Ok(())
    }

    fn handle_track_done(&mut self) -> Result<(), String> {
        self.current_song = None;

        if !self.song_queue.is_empty() {
            // More tracks to play
            self.handle_play()
        } else {
            // End of playlist
            self.set_streaming_state(StreamingState::Idle);
            self.set_playing_state(PlayingState::Stopped);
            Ok(())
        }
    }

    /// Add song to queue
    pub fn enqueue(&mut self, song: Song) {
        self.song_queue.push_back(song);
    }

    /// Clear queue
    pub fn clear_queue(&mut self) {
        self.song_queue.clear();
    }

    /// Get current position in track
    pub fn current_position(&self) -> Duration {
        // TODO: Calculate from frame data
        Duration::from_secs(0)
    }

    /// Get buffer fullness (0-100%)
    pub fn buffer_fullness(&self) -> f32 {
        // TODO: Calculate from player status
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_initial_state() {
        let controller = StreamingController::new(Uuid::new_v4());
        assert_eq!(controller.streaming_state, StreamingState::Idle);
        assert_eq!(controller.playing_state, PlayingState::Stopped);
    }

    #[test]
    fn test_play_transition() {
        let mut controller = StreamingController::new(Uuid::new_v4());

        // Add a song to queue
        controller.enqueue(Song {
            track_id: 1,
            url: "test.mp3".to_string(),
            duration: Some(Duration::from_secs(180)),
        });

        // Play should transition to TrackWait
        assert!(controller.handle_event(StreamingEvent::Play).is_ok());
        assert_eq!(controller.streaming_state, StreamingState::TrackWait);
    }

    #[test]
    fn test_pause_resume() {
        let mut controller = StreamingController::new(Uuid::new_v4());
        controller.set_playing_state(PlayingState::Playing);

        // Pause
        assert!(controller.handle_event(StreamingEvent::Pause).is_ok());
        assert_eq!(controller.playing_state, PlayingState::Paused);

        // Resume
        assert!(controller.handle_event(StreamingEvent::Resume).is_ok());
        assert_eq!(controller.playing_state, PlayingState::Playing);
    }
}
