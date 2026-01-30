//! Core player state machine and synchronization logic for Lyrion Music Server

pub mod player;
pub mod sync;
pub mod streaming;

pub use player::{Player, PlayerId, Song};
pub use sync::{SyncGroup, SyncManager, SyncAdjustment, PlayPoint};
pub use streaming::{StreamingController, StreamingEvent, PlayingState, StreamingState};
