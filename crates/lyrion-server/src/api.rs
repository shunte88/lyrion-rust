//! REST API handlers

use axum::{
    extract::{State, Query},
    Json,
    response::IntoResponse,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::AppState;
use lyrion_db::{Track, models};

/// Track with resolved names for API responses
#[derive(Debug, Serialize)]
pub struct TrackResponse {
    pub id: i64,
    pub url: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub tracknum: Option<i32>,
    pub year: Option<i16>,
    pub duration: Option<f32>,
    pub filesize: Option<i64>,
    pub bitrate: Option<f32>,
    pub samplerate: Option<i32>,
    pub content_type: Option<String>,
    pub has_cover: bool,
    pub bpm: Option<i16>,
    pub lyrics: Option<String>,
    pub musicbrainz_id: Option<String>,
    pub replay_gain: Option<f32>,
    pub replay_peak: Option<f32>,
}

/// List connected players
pub async fn list_players(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let players = state.slimproto_server.get_players().await;

    let response: Vec<_> = players
        .iter()
        .map(|(mac, conn)| {
            // Generate friendly player name from device type
            // Based on Slim::Player::Client model names
            let device_name = match conn.device_id {
                1 => "SliMP3",
                2 => "Squeezebox 1",
                3 => "Softsqueeze",
                4 => "Squeezebox 2",
                5 => "Transporter",
                6 => "Softsqueeze",
                7 => "Squeezebox Receiver",
                8 => "SqueezeSlave",
                9 => "Squeezebox Controller",
                10 => "Squeezebox Boom",
                11 => "Softsqueeze",
                12 => "SqueezePlay",
                13 => "Squeezebox Radio",
                14 => "Squeezebox Touch",
                _ => "Unknown Player",
            };

            serde_json::json!({
                "mac": mac,
                "name": format!("{} ({})", device_name, format_mac(mac)),
                "device_id": conn.device_id,
                "revision": conn.revision,
                "uuid": conn.uuid,
            })
        })
        .collect();

    Json(response)
}

/// Format MAC address as human-readable string (last 4 chars)
fn format_mac(mac: &str) -> String {
    let clean = mac.replace(':', "");
    if clean.len() >= 4 {
        clean[clean.len() - 4..].to_uppercase()
    } else {
        clean.to_uppercase()
    }
}

#[derive(Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    50
}

const MAX_LIMIT: i64 = 100;

/// List tracks with pagination
pub async fn list_tracks(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Vec<TrackResponse>>, StatusCode> {
    // Cap limit at MAX_LIMIT to prevent slow queries on large collections
    let limit = params.limit.min(MAX_LIMIT);

    // Optimized query using scalar subqueries instead of JOINs+GROUP BY
    // This avoids cartesian product from multiple artists/genres per track
    let tracks = sqlx::query_as::<_, TrackRow>(
        "SELECT
            t.id, t.url, t.title, t.tracknum, t.year, t.secs, t.filesize,
            t.bitrate, t.samplerate, t.content_type, t.cover as has_cover,
            t.bpm, t.lyrics, t.musicbrainz_id, t.replay_gain, t.replay_peak,
            (SELECT a.title FROM albums a WHERE a.id = t.album) as album_name,
            (SELECT c.name FROM contributors c
             JOIN contributor_track ct ON c.id = ct.contributor
             WHERE ct.track = t.id AND ct.role IN (1, 5, 6)
             LIMIT 1) as artist_name,
            (SELECT g.name FROM genres g
             JOIN genre_track gt ON g.id = gt.genre
             WHERE gt.track = t.id
             LIMIT 1) as genre_name
        FROM tracks t
        WHERE t.audio = 1
        ORDER BY t.titlesort
        LIMIT ? OFFSET ?"
    )
    .bind(limit)
    .bind(params.offset)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response: Vec<TrackResponse> = tracks.into_iter().map(|t| TrackResponse {
        id: t.id,
        url: t.url,
        title: t.title,
        artist: t.artist_name,
        album: t.album_name,
        genre: t.genre_name,
        tracknum: t.tracknum,
        year: t.year,
        duration: t.secs,
        filesize: t.filesize,
        bitrate: t.bitrate,
        samplerate: t.samplerate,
        content_type: t.content_type,
        has_cover: t.has_cover.is_some(),
        bpm: t.bpm,
        lyrics: t.lyrics,
        musicbrainz_id: t.musicbrainz_id,
        replay_gain: t.replay_gain,
        replay_peak: t.replay_peak,
    }).collect();

    Ok(Json(response))
}

#[derive(sqlx::FromRow)]
struct TrackRow {
    id: i64,
    url: String,
    title: Option<String>,
    tracknum: Option<i32>,
    year: Option<i16>,
    secs: Option<f32>,
    filesize: Option<i64>,
    bitrate: Option<f32>,
    samplerate: Option<i32>,
    content_type: Option<String>,
    has_cover: Option<Vec<u8>>,
    bpm: Option<i16>,
    lyrics: Option<String>,
    musicbrainz_id: Option<String>,
    replay_gain: Option<f32>,
    replay_peak: Option<f32>,
    album_name: Option<String>,
    artist_name: Option<String>,
    genre_name: Option<String>,
}

#[derive(Deserialize)]
pub struct SearchParams {
    q: String,
    #[serde(default = "default_search_limit")]
    limit: usize,
}

fn default_search_limit() -> usize {
    100
}

/// Search tracks
pub async fn search_tracks(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<TrackResponse>>, StatusCode> {
    let search_term = format!("%{}%", params.q.to_lowercase());

    // Optimized search using scalar subqueries
    let tracks = sqlx::query_as::<_, TrackRow>(
        "SELECT
            t.id, t.url, t.title, t.tracknum, t.year, t.secs, t.filesize,
            t.bitrate, t.samplerate, t.content_type, t.cover as has_cover,
            t.bpm, t.lyrics, t.musicbrainz_id, t.replay_gain, t.replay_peak,
            (SELECT a.title FROM albums a WHERE a.id = t.album) as album_name,
            (SELECT c.name FROM contributors c
             JOIN contributor_track ct ON c.id = ct.contributor
             WHERE ct.track = t.id AND ct.role IN (1, 5, 6)
             LIMIT 1) as artist_name,
            (SELECT g.name FROM genres g
             JOIN genre_track gt ON g.id = gt.genre
             WHERE gt.track = t.id
             LIMIT 1) as genre_name
        FROM tracks t
        WHERE t.audio = 1
        AND (t.titlesearch LIKE ?
             OR EXISTS (SELECT 1 FROM albums a WHERE a.id = t.album AND a.titlesearch LIKE ?)
             OR EXISTS (SELECT 1 FROM contributor_track ct
                        JOIN contributors c ON ct.contributor = c.id
                        WHERE ct.track = t.id AND c.namesearch LIKE ?))
        LIMIT 100"
    )
    .bind(&search_term)
    .bind(&search_term)
    .bind(&search_term)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response: Vec<TrackResponse> = tracks.into_iter().map(|t| TrackResponse {
        id: t.id,
        url: t.url,
        title: t.title,
        artist: t.artist_name,
        album: t.album_name,
        genre: t.genre_name,
        tracknum: t.tracknum,
        year: t.year,
        duration: t.secs,
        filesize: t.filesize,
        bitrate: t.bitrate,
        samplerate: t.samplerate,
        content_type: t.content_type,
        has_cover: t.has_cover.is_some(),
        bpm: t.bpm,
        lyrics: t.lyrics,
        musicbrainz_id: t.musicbrainz_id,
        replay_gain: t.replay_gain,
        replay_peak: t.replay_peak,
    }).collect();

    Ok(Json(response))
}

/// Get cover art for a track
pub async fn get_cover_art(
    State(state): State<AppState>,
    axum::extract::Path(track_id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let cover: Option<(Option<Vec<u8>>,)> = sqlx::query_as(
        "SELECT cover FROM tracks WHERE id = ?"
    )
    .bind(track_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some((Some(data),)) = cover {
        Ok((
            [(axum::http::header::CONTENT_TYPE, "image/jpeg")],
            data
        ))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
