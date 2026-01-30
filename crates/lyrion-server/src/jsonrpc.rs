//! JSON-RPC endpoint handler
//! Compatible with Logitech Media Server JSON-RPC API

use axum::{
    extract::State,
    Json,
    response::IntoResponse,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Vec<Value>>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub id: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

/// JSON-RPC handler
pub async fn jsonrpc_handler(
    State(state): State<AppState>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    tracing::debug!("JSON-RPC request: {:?}", request);

    let result = match request.method.as_str() {
        "slim.request" => handle_slim_request(&state, request.params).await,
        _ => Err(JsonRpcError {
            code: -32601,
            message: format!("Method '{}' not found", request.method),
        }),
    };

    let response = match result {
        Ok(value) => JsonRpcResponse {
            id: request.id,
            result: Some(value),
            error: None,
        },
        Err(error) => JsonRpcResponse {
            id: request.id,
            result: None,
            error: Some(error),
        },
    };

    Json(response)
}

/// Handle slim.request JSON-RPC method
async fn handle_slim_request(
    state: &AppState,
    params: Option<Vec<Value>>,
) -> Result<Value, JsonRpcError> {
    let params = params.ok_or(JsonRpcError {
        code: -32602,
        message: "Invalid params".to_string(),
    })?;

    if params.len() < 2 {
        return Err(JsonRpcError {
            code: -32602,
            message: "slim.request requires [player_id, [command, ...args]]".to_string(),
        });
    }

    let player_id = params[0]
        .as_str()
        .ok_or(JsonRpcError {
            code: -32602,
            message: "player_id must be a string".to_string(),
        })?;

    let command_array = params[1]
        .as_array()
        .ok_or(JsonRpcError {
            code: -32602,
            message: "command must be an array".to_string(),
        })?;

    if command_array.is_empty() {
        return Err(JsonRpcError {
            code: -32602,
            message: "command array cannot be empty".to_string(),
        });
    }

    let command = command_array[0]
        .as_str()
        .ok_or(JsonRpcError {
            code: -32602,
            message: "command must be a string".to_string(),
        })?;

    // Handle commands
    match command {
        "status" => {
            // Return actual player status
            let playlist = state.playlist_manager.get_playlist(player_id).await;

            let tracks: Vec<_> = playlist.all_tracks().iter().enumerate().map(|(i, track)| {
                json!({
                    "playlist index": i,
                    "id": track.id,
                    "title": track.title,
                    "artist": track.artist,
                    "album": track.album,
                })
            }).collect();

            // Use actual playing state from playlist
            let mode = if playlist.playing { "play" } else { "stop" };

            Ok(json!({
                "player_id": player_id,
                "mode": mode,
                "time": 0, // TODO: Get actual position
                "duration": 0, // TODO: Get actual duration
                "playlist_loop": tracks,
                "playlist_cur_index": playlist.current_index.unwrap_or(0),
                "mixer_volume": 50, // TODO: Get actual volume
            }))
        }
        "play" => {
            tracing::info!("=== PLAY COMMAND RECEIVED === player_id: {}", player_id);
            // Play command: start playback
            // Format: ["play"] to resume, or ["play", track_id] to play specific track

            if command_array.len() > 1 {
                tracing::info!("Play with track_id, array length: {}", command_array.len());
                // Play specific track
                let track_id = command_array[1]
                    .as_i64()
                    .ok_or(JsonRpcError {
                        code: -32602,
                        message: "track_id must be an integer".to_string(),
                    })?;

                // Look up track in database
                tracing::info!("Looking up track {} in database", track_id);
                let track = sqlx::query_as::<_, (i64, String, String)>(
                    "SELECT id, url, content_type FROM tracks WHERE id = ?"
                )
                .bind(track_id)
                .fetch_optional(&state.db_pool)
                .await
                .map_err(|e| {
                    tracing::error!("Database error: {}", e);
                    JsonRpcError {
                        code: -32603,
                        message: format!("Database error: {}", e),
                    }
                })?
                .ok_or_else(|| {
                    tracing::error!("Track {} not found", track_id);
                    JsonRpcError {
                        code: -32602,
                        message: format!("Track {} not found", track_id),
                    }
                })?;

                let (db_track_id, _url, content_type) = track;
                tracing::info!("Track found: id={}, content_type={}", db_track_id, content_type);

                // Build HTTP stream URL
                let server_port = 9000; // TODO: Get from config
                // Convert server IP to u32 (192.168.1.210)
                let server_ip: u32 = (192u32 << 24) | (168u32 << 16) | (1u32 << 8) | 210u32;
                let stream_url = format!("GET /stream/{} HTTP/1.0\r\n\r\n", db_track_id);

                // Determine format from content_type and set appropriate parameters
                let (format, pcm_sample_size, pcm_sample_rate, pcm_channels, pcm_endian, output_threshold) = match content_type.as_str() {
                    "wav" | "pcm" => {
                        // PCM: p=16bit, 3=44.1kHz, 2=stereo, 1=little-endian
                        (b'p', 1, 3, 2, 1, 0)
                    }
                    "mp3" => {
                        (b'm', b'?', b'?', b'?', b'?', 1)
                    }
                    "flac" | "flc" => {
                        (b'f', b'?', b'?', b'?', b'?', 0)
                    }
                    "ogg" => {
                        (b'o', b'?', b'?', b'?', b'?', 20)
                    }
                    "aac" | "mp4" => {
                        (b'a', b'?', b'?', b'?', b'?', 0)
                    }
                    _ => {
                        // default to mp3 decoder
                        (b'm', b'?', b'?', b'?', b'?', 1)
                    }
                };

                // Build strm s command
                use lyrion_protocol::StreamCommand;
                tracing::info!("Building strm command with format: {:?}, server: {}:{}", format as char, server_ip, server_port);
                let cmd = StreamCommand::Start {
                    autostart: 1, // 75% buffer before autostart
                    format,
                    pcm_sample_size,
                    pcm_sample_rate,
                    pcm_channels,
                    pcm_endian,
                    buffer_threshold: 30, // 30KB
                    spdif_enable: 0,
                    transition_duration: 0,
                    transition_type: 0,
                    flags: 0,
                    output_threshold,
                    reserved: 0,
                    replay_gain: 0,
                    server_port,
                    server_ip, // actual server IP
                    request_string: stream_url.clone(),
                };

                tracing::info!("Sending strm command to player: {}", player_id);
                // Send command to player
                match state.slimproto_server.send_command(player_id, cmd).await {
                    Ok(_) => {
                        tracing::info!("Successfully sent strm command to player {}", player_id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to send command to player {}: {}", player_id, e);
                        return Err(JsonRpcError {
                            code: -32603,
                            message: format!("Failed to send command to player: {}", e),
                        });
                    }
                }

                tracing::info!("Play command completed successfully");

                // Update playlist playing state
                state.playlist_manager.set_playing(player_id, true).await;

                // Broadcast status update to WebSocket clients
                let _ = state.ws_broadcast.send(crate::websocket::WsMessage::PlayerStatus(
                    crate::websocket::PlayerStatusUpdate {
                        player_id: player_id.to_string(),
                        playing: true,
                        position: Some(0.0),
                        volume: None,
                        current_track_id: Some(track_id),
                    }
                ));

                Ok(json!({
                    "player_id": player_id,
                    "command": "play",
                    "track_id": track_id,
                    "status": "playing"
                }))
            } else {
                // Resume playback (unpause)
                use lyrion_protocol::StreamCommand;
                let cmd = StreamCommand::Unpause {
                    interval_ms: 0, // resume immediately
                };

                state.slimproto_server.send_command(player_id, cmd)
                    .await
                    .map_err(|e| JsonRpcError {
                        code: -32603,
                        message: format!("Failed to send command to player: {}", e),
                    })?;

                // Update playlist playing state
                state.playlist_manager.set_playing(player_id, true).await;

                // Broadcast status update to WebSocket clients
                let _ = state.ws_broadcast.send(crate::websocket::WsMessage::PlayerStatus(
                    crate::websocket::PlayerStatusUpdate {
                        player_id: player_id.to_string(),
                        playing: true,
                        position: None,
                        volume: None,
                        current_track_id: None,
                    }
                ));

                Ok(json!({
                    "player_id": player_id,
                    "command": "play",
                    "status": "resumed"
                }))
            }
        }
        "pause" => {
            // Pause playback
            // Format: ["pause"] to toggle pause, or ["pause", 1] to pause, ["pause", 0] to unpause
            let should_pause = if command_array.len() > 1 {
                command_array[1].as_i64().unwrap_or(1) != 0
            } else {
                true // Default to pause if no argument
            };

            if should_pause {
                // Send pause command (pause indefinitely = interval 0)
                use lyrion_protocol::StreamCommand;
                let cmd = StreamCommand::PauseFor {
                    interval_ms: 0,
                };

                state.slimproto_server.send_command(player_id, cmd)
                    .await
                    .map_err(|e| JsonRpcError {
                        code: -32603,
                        message: format!("Failed to send pause command: {}", e),
                    })?;

                // Update playlist playing state
                state.playlist_manager.set_playing(player_id, false).await;

                // Broadcast status update to WebSocket clients
                let _ = state.ws_broadcast.send(crate::websocket::WsMessage::PlayerStatus(
                    crate::websocket::PlayerStatusUpdate {
                        player_id: player_id.to_string(),
                        playing: false,
                        position: None,
                        volume: None,
                        current_track_id: None,
                    }
                ));

                Ok(json!({
                    "player_id": player_id,
                    "command": "pause",
                    "status": "paused"
                }))
            } else {
                // Unpause
                use lyrion_protocol::StreamCommand;
                let cmd = StreamCommand::Unpause {
                    interval_ms: 0,
                };

                state.slimproto_server.send_command(player_id, cmd)
                    .await
                    .map_err(|e| JsonRpcError {
                        code: -32603,
                        message: format!("Failed to send unpause command: {}", e),
                    })?;

                // Update playlist playing state
                state.playlist_manager.set_playing(player_id, true).await;

                // Broadcast status update to WebSocket clients
                let _ = state.ws_broadcast.send(crate::websocket::WsMessage::PlayerStatus(
                    crate::websocket::PlayerStatusUpdate {
                        player_id: player_id.to_string(),
                        playing: true,
                        position: None,
                        volume: None,
                        current_track_id: None,
                    }
                ));

                Ok(json!({
                    "player_id": player_id,
                    "command": "pause",
                    "status": "resumed"
                }))
            }
        }
        "stop" => {
            // Stop playback
            use lyrion_protocol::StreamCommand;
            let cmd = StreamCommand::Stop;

            state.slimproto_server.send_command(player_id, cmd)
                .await
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: format!("Failed to send stop command: {}", e),
                })?;

            // Update playlist playing state
            state.playlist_manager.set_playing(player_id, false).await;

            // Broadcast status update to WebSocket clients
            let _ = state.ws_broadcast.send(crate::websocket::WsMessage::PlayerStatus(
                crate::websocket::PlayerStatusUpdate {
                    player_id: player_id.to_string(),
                    playing: false,
                    position: Some(0.0),
                    volume: None,
                    current_track_id: None,
                }
            ));

            Ok(json!({
                "player_id": player_id,
                "command": "stop",
                "status": "stopped"
            }))
        }
        "sync" => {
            // Sync command: sync this player with another player
            // Format: ["sync", "master_mac"] or ["sync", "-"] to unsync
            if command_array.len() < 2 {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "sync command requires target player MAC".to_string(),
                });
            }

            let target = command_array[1]
                .as_str()
                .ok_or(JsonRpcError {
                    code: -32602,
                    message: "target MAC must be a string".to_string(),
                })?;

            if target == "-" {
                // Unsync this player
                // Parse player_id as UUID
                let player_uuid = uuid::Uuid::parse_str(player_id).map_err(|_| JsonRpcError {
                    code: -32602,
                    message: "Invalid player UUID".to_string(),
                })?;

                match state.sync_manager.remove_from_group(player_uuid).await {
                    Ok(_) => {
                        tracing::info!("Player {} unsynced", player_id);
                        Ok(json!({
                            "player_id": player_id,
                            "command": "sync",
                            "status": "unsynced"
                        }))
                    }
                    Err(e) => {
                        tracing::warn!("Failed to unsync player {}: {}", player_id, e);
                        Ok(json!({
                            "error": e
                        }))
                    }
                }
            } else {
                // Sync with target player
                // Parse both UUIDs
                let player_uuid = uuid::Uuid::parse_str(player_id).map_err(|_| JsonRpcError {
                    code: -32602,
                    message: "Invalid player UUID".to_string(),
                })?;

                let target_uuid = uuid::Uuid::parse_str(target).map_err(|_| JsonRpcError {
                    code: -32602,
                    message: "Invalid target UUID".to_string(),
                })?;

                // Check if target player has a group, or create one
                let group_id = if let Some(group) = state.sync_manager.get_group(target_uuid).await {
                    // Join existing group
                    group.id
                } else {
                    // Create new group with target as master
                    state.sync_manager.create_group(target_uuid).await
                };

                match state.sync_manager.add_to_group(group_id, player_uuid).await {
                    Ok(_) => {
                        tracing::info!("Player {} synced with {}", player_id, target);
                        Ok(json!({
                            "player_id": player_id,
                            "command": "sync",
                            "status": "synced",
                            "master": target,
                            "group_id": group_id.to_string()
                        }))
                    }
                    Err(e) => {
                        tracing::warn!("Failed to sync player {} with {}: {}", player_id, target, e);
                        Ok(json!({
                            "error": e
                        }))
                    }
                }
            }
        }
        "syncgroupid" => {
            // Get sync group ID for this player
            let player_uuid = uuid::Uuid::parse_str(player_id).map_err(|_| JsonRpcError {
                code: -32602,
                message: "Invalid player UUID".to_string(),
            })?;

            if let Some(group) = state.sync_manager.get_group(player_uuid).await {
                Ok(json!({
                    "player_id": player_id,
                    "group_id": group.id.to_string(),
                    "master": group.master.to_string(),
                    "slaves": group.slaves.iter().map(|s| s.to_string()).collect::<Vec<_>>()
                }))
            } else {
                Ok(json!({
                    "player_id": player_id,
                    "group_id": null
                }))
            }
        }
        "mixer" => {
            // Volume control command
            // Format: ["mixer", "volume", value] to set absolute volume (0-100)
            // Format: ["mixer", "volume", "+5"] or ["mixer", "volume", "-5"] for relative
            if command_array.len() < 3 {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "mixer command requires [\"mixer\", \"volume\", value]".to_string(),
                });
            }

            let subcommand = command_array[1]
                .as_str()
                .ok_or(JsonRpcError {
                    code: -32602,
                    message: "mixer subcommand must be a string".to_string(),
                })?;

            if subcommand != "volume" {
                return Err(JsonRpcError {
                    code: -32602,
                    message: format!("Unknown mixer subcommand: {}", subcommand),
                });
            }

            // Parse volume value
            let volume_value = &command_array[2];
            let volume_str = if let Some(s) = volume_value.as_str() {
                s.to_string()
            } else if let Some(i) = volume_value.as_i64() {
                i.to_string()
            } else {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "volume value must be a number or string".to_string(),
                });
            };

            // TODO: Get current volume from player state
            let old_volume: u8 = 50; // Placeholder

            let new_volume: u8 = if volume_str.starts_with('+') || volume_str.starts_with('-') {
                // Relative volume change
                let delta: i16 = volume_str.parse().map_err(|_| JsonRpcError {
                    code: -32602,
                    message: "Invalid relative volume value".to_string(),
                })?;
                ((old_volume as i16 + delta).max(0).min(100)) as u8
            } else {
                // Absolute volume
                volume_str.parse().map_err(|_| JsonRpcError {
                    code: -32602,
                    message: "Invalid volume value".to_string(),
                })?
            };

            // Create and send audio gain command
            use lyrion_protocol::AudioGainCommand;
            let cmd = AudioGainCommand::from_volume(new_volume, old_volume);

            state.slimproto_server.send_audio_gain(player_id, cmd)
                .await
                .map_err(|e| JsonRpcError {
                    code: -32603,
                    message: format!("Failed to send volume command: {}", e),
                })?;

            // Broadcast volume update to WebSocket clients
            let _ = state.ws_broadcast.send(crate::websocket::WsMessage::PlayerStatus(
                crate::websocket::PlayerStatusUpdate {
                    player_id: player_id.to_string(),
                    playing: true,
                    position: None,
                    volume: Some(new_volume as i32),
                    current_track_id: None,
                }
            ));

            Ok(json!({
                "player_id": player_id,
                "command": "mixer",
                "volume": new_volume
            }))
        }
        "time" => {
            // Seek/skip command
            // Format: ["time", position] to seek to absolute position in seconds
            // Format: ["time", "+10"] or ["time", "-10"] for relative seek
            if command_array.len() < 2 {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "time command requires position value".to_string(),
                });
            }

            let time_value = &command_array[1];
            let time_str = if let Some(s) = time_value.as_str() {
                s.to_string()
            } else if let Some(f) = time_value.as_f64() {
                f.to_string()
            } else if let Some(i) = time_value.as_i64() {
                i.to_string()
            } else {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "time value must be a number or string".to_string(),
                });
            };

            // Parse time value
            if time_str.starts_with('+') {
                // Skip ahead (relative forward)
                let seconds: f64 = time_str[1..].parse().map_err(|_| JsonRpcError {
                    code: -32602,
                    message: "Invalid time value".to_string(),
                })?;
                let interval_ms = (seconds * 1000.0) as u32;

                use lyrion_protocol::StreamCommand;
                let cmd = StreamCommand::SkipAhead { interval_ms };

                state.slimproto_server.send_command(player_id, cmd)
                    .await
                    .map_err(|e| JsonRpcError {
                        code: -32603,
                        message: format!("Failed to send skip command: {}", e),
                    })?;

                Ok(json!({
                    "player_id": player_id,
                    "command": "time",
                    "skip_ahead_seconds": seconds
                }))
            } else if time_str.starts_with('-') {
                // Skip back (pause for negative interval - squeezelite interprets this as rewind)
                let seconds: f64 = time_str[1..].parse().map_err(|_| JsonRpcError {
                    code: -32602,
                    message: "Invalid time value".to_string(),
                })?;
                let interval_ms = (seconds * 1000.0) as u32;

                // For backward skip, we use PauseFor with the interval
                // This tells the player to go back in time
                use lyrion_protocol::StreamCommand;
                let cmd = StreamCommand::PauseFor { interval_ms };

                state.slimproto_server.send_command(player_id, cmd)
                    .await
                    .map_err(|e| JsonRpcError {
                        code: -32603,
                        message: format!("Failed to send skip back command: {}", e),
                    })?;

                Ok(json!({
                    "player_id": player_id,
                    "command": "time",
                    "skip_back_seconds": seconds
                }))
            } else {
                // Absolute seek position
                // This requires stopping current stream and restarting with seekdata
                // For now, return not implemented
                Ok(json!({
                    "error": "Absolute seek not yet implemented - use relative (+/- seconds)"
                }))
            }
        }
        "playlistcontrol" => {
            // Playlist control commands
            // Format: ["playlistcontrol", "cmd", ...args]
            if command_array.len() < 2 {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "playlistcontrol requires subcommand".to_string(),
                });
            }

            let subcmd = command_array[1]
                .as_str()
                .ok_or(JsonRpcError {
                    code: -32602,
                    message: "playlistcontrol subcommand must be a string".to_string(),
                })?;

            match subcmd {
                "cmd:add" => {
                    // Add track to playlist
                    // Format: ["playlistcontrol", "cmd:add", "item_id", track_id]
                    if command_array.len() < 4 {
                        return Err(JsonRpcError {
                            code: -32602,
                            message: "cmd:add requires track_id".to_string(),
                        });
                    }

                    let track_id = command_array[3]
                        .as_i64()
                        .ok_or(JsonRpcError {
                            code: -32602,
                            message: "track_id must be an integer".to_string(),
                        })?;

                    // Look up track in database with artist and album information
                    let track = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, Option<String>)>(
                        "SELECT t.id, t.url, t.title, c.name as artist, a.title as album \
                         FROM tracks t \
                         LEFT JOIN albums a ON t.album = a.id \
                         LEFT JOIN contributor_track ct ON t.id = ct.track AND ct.role = 1 \
                         LEFT JOIN contributors c ON ct.contributor = c.id \
                         WHERE t.id = ?"
                    )
                    .bind(track_id)
                    .fetch_optional(&state.db_pool)
                    .await
                    .map_err(|e| JsonRpcError {
                        code: -32603,
                        message: format!("Database error: {}", e),
                    })?
                    .ok_or(JsonRpcError {
                        code: -32602,
                        message: format!("Track {} not found", track_id),
                    })?;

                    use crate::playlist::PlaylistTrack;
                    let playlist_track = PlaylistTrack {
                        id: track.0,
                        url: track.1,
                        title: track.2,
                        artist: track.3,
                        album: track.4,
                        duration: None, // TODO: Get duration from track
                    };

                    state.playlist_manager.add_track(player_id, playlist_track).await;

                    Ok(json!({
                        "player_id": player_id,
                        "command": "playlistcontrol",
                        "action": "add",
                        "track_id": track_id
                    }))
                }
                "cmd:clear" => {
                    // Clear playlist
                    state.playlist_manager.clear(player_id).await;

                    Ok(json!({
                        "player_id": player_id,
                        "command": "playlistcontrol",
                        "action": "clear"
                    }))
                }
                "cmd:jump" => {
                    // Jump to track index
                    // Format: ["playlistcontrol", "cmd:jump", index]
                    if command_array.len() < 3 {
                        return Err(JsonRpcError {
                            code: -32602,
                            message: "cmd:jump requires index".to_string(),
                        });
                    }

                    let index = command_array[2]
                        .as_u64()
                        .ok_or(JsonRpcError {
                            code: -32602,
                            message: "index must be a number".to_string(),
                        })? as usize;

                    if let Some(track) = state.playlist_manager.jump_to(player_id, index).await {
                        // Start playing the track
                        use lyrion_protocol::StreamCommand;
                        let server_ip: u32 = (192u32 << 24) | (168u32 << 16) | (1u32 << 8) | 210u32;
                        let stream_url = format!("GET /stream/{} HTTP/1.0\r\n\r\n", track.id);

                        let cmd = StreamCommand::Start {
                            autostart: 1,
                            format: b'p', // Default to PCM
                            pcm_sample_size: 1,
                            pcm_sample_rate: 3,
                            pcm_channels: 2,
                            pcm_endian: 1,
                            buffer_threshold: 30,
                            spdif_enable: 0,
                            transition_duration: 0,
                            transition_type: 0,
                            flags: 0,
                            output_threshold: 0,
                            reserved: 0,
                            replay_gain: 0,
                            server_port: 9000,
                            server_ip,
                            request_string: stream_url,
                        };

                        state.slimproto_server.send_command(player_id, cmd).await
                            .map_err(|e| JsonRpcError {
                                code: -32603,
                                message: format!("Failed to send command: {}", e),
                            })?;

                        Ok(json!({
                            "player_id": player_id,
                            "command": "playlistcontrol",
                            "action": "jump",
                            "index": index,
                            "track_id": track.id
                        }))
                    } else {
                        Err(JsonRpcError {
                            code: -32602,
                            message: format!("Invalid index: {}", index),
                        })
                    }
                }
                _ => Ok(json!({
                    "error": format!("Unknown playlistcontrol command: {}", subcmd)
                })),
            }
        }
        "button" => {
            // Button press commands (used for next/previous)
            // Format: ["button", "arrow_right"] for next, ["button", "arrow_left"] for previous
            if command_array.len() < 2 {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "button command requires button name".to_string(),
                });
            }

            let button = command_array[1]
                .as_str()
                .ok_or(JsonRpcError {
                    code: -32602,
                    message: "button name must be a string".to_string(),
                })?;

            match button {
                "arrow_right" | "jump_rew" => {
                    // Next track
                    if let Some(track) = state.playlist_manager.next(player_id).await {
                        // Start playing the track
                        use lyrion_protocol::StreamCommand;
                        let server_ip: u32 = (192u32 << 24) | (168u32 << 16) | (1u32 << 8) | 210u32;
                        let stream_url = format!("GET /stream/{} HTTP/1.0\r\n\r\n", track.id);

                        let cmd = StreamCommand::Start {
                            autostart: 1,
                            format: b'p',
                            pcm_sample_size: 1,
                            pcm_sample_rate: 3,
                            pcm_channels: 2,
                            pcm_endian: 1,
                            buffer_threshold: 30,
                            spdif_enable: 0,
                            transition_duration: 0,
                            transition_type: 0,
                            flags: 0,
                            output_threshold: 0,
                            reserved: 0,
                            replay_gain: 0,
                            server_port: 9000,
                            server_ip,
                            request_string: stream_url,
                        };

                        state.slimproto_server.send_command(player_id, cmd).await
                            .map_err(|e| JsonRpcError {
                                code: -32603,
                                message: format!("Failed to send command: {}", e),
                            })?;

                        Ok(json!({
                            "player_id": player_id,
                            "command": "button",
                            "button": "next",
                            "track_id": track.id
                        }))
                    } else {
                        Ok(json!({
                            "error": "No next track in playlist"
                        }))
                    }
                }
                "arrow_left" | "jump_fwd" => {
                    // Previous track
                    if let Some(track) = state.playlist_manager.previous(player_id).await {
                        // Start playing the track
                        use lyrion_protocol::StreamCommand;
                        let server_ip: u32 = (192u32 << 24) | (168u32 << 16) | (1u32 << 8) | 210u32;
                        let stream_url = format!("GET /stream/{} HTTP/1.0\r\n\r\n", track.id);

                        let cmd = StreamCommand::Start {
                            autostart: 1,
                            format: b'p',
                            pcm_sample_size: 1,
                            pcm_sample_rate: 3,
                            pcm_channels: 2,
                            pcm_endian: 1,
                            buffer_threshold: 30,
                            spdif_enable: 0,
                            transition_duration: 0,
                            transition_type: 0,
                            flags: 0,
                            output_threshold: 0,
                            reserved: 0,
                            replay_gain: 0,
                            server_port: 9000,
                            server_ip,
                            request_string: stream_url,
                        };

                        state.slimproto_server.send_command(player_id, cmd).await
                            .map_err(|e| JsonRpcError {
                                code: -32603,
                                message: format!("Failed to send command: {}", e),
                            })?;

                        Ok(json!({
                            "player_id": player_id,
                            "command": "button",
                            "button": "previous",
                            "track_id": track.id
                        }))
                    } else {
                        Ok(json!({
                            "error": "No previous track in playlist"
                        }))
                    }
                }
                _ => Ok(json!({
                    "error": format!("Unknown button: {}", button)
                })),
            }
        }
        "playlist" => {
            // Playlist commands
            // Format: ["playlist", subcommand, ...args]
            if command_array.len() < 2 {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "playlist command requires subcommand".to_string(),
                });
            }

            let subcmd = command_array[1]
                .as_str()
                .ok_or(JsonRpcError {
                    code: -32602,
                    message: "playlist subcommand must be a string".to_string(),
                })?;

            match subcmd {
                "tracks" => {
                    // Get current playlist
                    let playlist = state.playlist_manager.get_playlist(player_id).await;

                    let tracks: Vec<_> = playlist.all_tracks().iter().enumerate().map(|(i, track)| {
                        json!({
                            "playlist index": i,
                            "id": track.id,
                            "title": track.title,
                            "artist": track.artist,
                            "album": track.album,
                        })
                    }).collect();

                    Ok(json!({
                        "player_id": player_id,
                        "count": playlist.len(),
                        "playlist_loop": tracks
                    }))
                }
                "shuffle" => {
                    // Shuffle command: ["playlist", "shuffle", mode]
                    // mode: 0=off, 1=songs, 2=albums
                    if command_array.len() < 3 {
                        return Err(JsonRpcError {
                            code: -32602,
                            message: "shuffle requires mode (0, 1, or 2)".to_string(),
                        });
                    }

                    let mode = command_array[2]
                        .as_i64()
                        .or_else(|| command_array[2].as_str().and_then(|s| s.parse::<i64>().ok()))
                        .ok_or(JsonRpcError {
                            code: -32602,
                            message: "shuffle mode must be a number".to_string(),
                        })?;

                    // Store shuffle mode (0-2)
                    state.playlist_manager.set_shuffle(player_id, mode as u8).await;

                    Ok(json!({
                        "player_id": player_id,
                        "shuffle": mode
                    }))
                }
                "repeat" => {
                    // Repeat command: ["playlist", "repeat", mode]
                    // mode: 0=off, 1=song, 2=playlist
                    if command_array.len() < 3 {
                        return Err(JsonRpcError {
                            code: -32602,
                            message: "repeat requires mode (0, 1, or 2)".to_string(),
                        });
                    }

                    let mode = command_array[2]
                        .as_i64()
                        .or_else(|| command_array[2].as_str().and_then(|s| s.parse::<i64>().ok()))
                        .ok_or(JsonRpcError {
                            code: -32602,
                            message: "repeat mode must be a number".to_string(),
                        })?;

                    // Store repeat mode (0-2)
                    state.playlist_manager.set_repeat(player_id, mode as u8).await;

                    Ok(json!({
                        "player_id": player_id,
                        "repeat": mode
                    }))
                }
                _ => Ok(json!({
                    "error": format!("Unknown playlist subcommand: {}", subcmd)
                })),
            }
        }
        _ => Ok(json!({
            "error": format!("Command '{}' not yet implemented", command)
        })),
    }
}
