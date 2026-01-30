//! HTTP audio streaming with transcoding support

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    http::{StatusCode, header},
    body::Body,
};
use serde::Deserialize;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use std::path::PathBuf;

use crate::AppState;
use lyrion_db::Track;
use lyrion_transcode::{TranscodePipeline, needs_transcoding, get_format_from_path};

#[derive(Deserialize)]
pub struct StreamParams {
    #[serde(default)]
    pub format: Option<String>,
}

/// Stream audio file to player
/// Route: GET /stream/:track_id
pub async fn stream_track(
    State(state): State<AppState>,
    Path(track_id): Path<i64>,
    Query(params): Query<StreamParams>,
) -> Result<Response, StatusCode> {
    tracing::info!("=== STREAM REQUEST START === track_id: {}", track_id);

    // Get track from database
    tracing::info!("Looking up track {} in database...", track_id);
    let track = Track::find_by_id(&state.db_pool, track_id)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::error!("Track {} not found in database", track_id);
            StatusCode::NOT_FOUND
        })?;

    tracing::info!("Track found: {:?}", track.url);

    // Determine source format
    tracing::info!("Determining source format from path...");
    let source_format = get_format_from_path(&track.url)
        .ok_or_else(|| {
            tracing::error!("Could not determine format from path: {}", track.url);
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        })?;
    tracing::info!("Source format: {}", source_format);

    // Determine target format
    let target_format = params.format.as_deref().unwrap_or(&source_format);
    tracing::info!("Target format: {}", target_format);

    let needs_transcode = needs_transcoding(&source_format, target_format);
    tracing::info!(
        "Streaming track {} ({}) - {}file: {}",
        track_id,
        track.title.as_deref().unwrap_or("Unknown"),
        if needs_transcode { "transcoding " } else { "" },
        track.url
    );

    // Check if file exists
    let file_path = PathBuf::from(&track.url);
    tracing::info!("Checking if file exists: {:?}", file_path);
    if !file_path.exists() {
        tracing::error!("File not found: {}", track.url);
        return Err(StatusCode::NOT_FOUND);
    }
    tracing::info!("File exists, proceeding with streaming");

    // Determine if transcoding is needed
    if needs_transcode {
        tracing::info!("Using transcoding path");
        stream_with_transcoding(&track.url, &source_format, target_format).await
    } else {
        tracing::info!("Using direct streaming path");
        stream_direct(&track.url, &source_format).await
    }
}

/// Stream file directly without transcoding
async fn stream_direct(file_path: &str, format: &str) -> Result<Response, StatusCode> {
    tracing::info!("stream_direct: Opening file: {}", file_path);

    let file = File::open(file_path).await.map_err(|e| {
        tracing::error!("Failed to open file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("stream_direct: File opened successfully");

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let content_type = match format {
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "m4a" | "aac" => "audio/aac",
        "ogg" => "audio/ogg",
        _ => "application/octet-stream",
    };

    tracing::info!("stream_direct: Creating response with content_type: {}", content_type);

    let response = (
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type)],
        body,
    )
        .into_response();

    tracing::info!("stream_direct: Response created successfully");

    Ok(response)
}

/// Stream with transcoding
async fn stream_with_transcoding(
    file_path: &str,
    from_format: &str,
    to_format: &str,
) -> Result<Response, StatusCode> {
    tracing::info!("Transcoding {} -> {}: {}", from_format, to_format, file_path);

    let mut pipeline = TranscodePipeline::new(file_path, from_format, to_format).map_err(|e| {
        tracing::error!("Failed to create transcoding pipeline: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let stdout = pipeline.take_stdout().ok_or_else(|| {
        tracing::error!("Failed to get pipeline output");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let stream = ReaderStream::new(stdout);
    let body = Body::from_stream(stream);

    let content_type = match to_format {
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    };

    // Spawn task to wait for pipeline completion
    tokio::spawn(async move {
        // Wait for pipeline to complete
        if let Err(e) = pipeline.wait().await {
            tracing::error!("Transcoding pipeline failed: {}", e);
        } else {
            tracing::debug!("Transcoding pipeline completed successfully");
        }
    });

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (header::TRANSFER_ENCODING, "chunked"),
        ],
        body,
    )
        .into_response())
}

/// Stream with ICY metadata injection
/// This is used for Shoutcast-style streaming with metadata
pub async fn stream_with_icy(
    State(state): State<AppState>,
    Path(track_id): Path<i64>,
    Query(params): Query<StreamParams>,
) -> Result<Response, StatusCode> {
    // Get track info
    let track = Track::find_by_id(&state.db_pool, track_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    tracing::debug!("ICY stream requested for track {}", track_id);

    // For now, delegate to regular streaming
    // TODO: Implement ICY metadata injection every 32KB
    stream_track(State(state), Path(track_id), Query(params)).await
}
