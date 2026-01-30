//! Database models
//! Ported from Slim/Schema/*.pm

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Track model (from tracks table)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Track {
    pub id: i64,
    pub url: String,
    pub title: Option<String>,
    pub titlesort: Option<String>,
    pub titlesearch: Option<String>,
    pub customsearch: Option<String>,
    pub album: Option<i64>,
    pub tracknum: Option<i32>,
    pub content_type: Option<String>,
    pub timestamp: Option<i64>,
    pub filesize: Option<i64>,
    pub audio_size: Option<i64>,
    pub audio_offset: Option<i64>,
    pub year: Option<i16>,
    pub secs: Option<f32>,
    pub cover: Option<Vec<u8>>,
    pub vbr_scale: Option<String>,
    pub bitrate: Option<f32>,
    pub samplerate: Option<i32>,
    pub samplesize: Option<i32>,
    pub channels: Option<i8>,
    pub block_alignment: Option<i32>,
    pub endian: Option<bool>,
    pub bpm: Option<i16>,
    pub tagversion: Option<String>,
    pub drm: Option<bool>,
    pub disc: Option<i8>,
    pub audio: Option<bool>,
    pub remote: Option<bool>,
    pub lossless: Option<bool>,
    pub lyrics: Option<String>,
    pub musicbrainz_id: Option<String>,
    pub musicmagic_mixable: Option<bool>,
    pub replay_gain: Option<f32>,
    pub replay_peak: Option<f32>,
    pub extid: Option<String>,
    pub metadata_hash: Option<String>,
}

/// Album model (from albums table)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Album {
    pub id: i64,
    pub title: Option<String>,
    pub titlesort: Option<String>,
    pub titlesearch: Option<String>,
    pub customsearch: Option<String>,
    pub compilation: Option<bool>,
    pub year: Option<i16>,
    pub artwork: Option<String>,
    pub disc: Option<i8>,
    pub discc: Option<i8>,
    pub replay_gain: Option<f32>,
    pub replay_peak: Option<f32>,
    pub musicbrainz_id: Option<String>,
    pub musicmagic_mixable: Option<bool>,
    pub contributor: Option<i64>,
}

/// Contributor model (from contributors table - artists, composers, etc.)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Contributor {
    pub id: i64,
    pub name: Option<String>,
    pub namesort: Option<String>,
    pub namesearch: Option<String>,
    pub customsearch: Option<String>,
    pub musicbrainz_id: Option<String>,
    pub musicmagic_mixable: Option<bool>,
}

/// Genre model (from genres table)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Genre {
    pub id: i64,
    pub name: Option<String>,
    pub namesort: Option<String>,
    pub namesearch: Option<String>,
    pub customsearch: Option<String>,
    pub musicmagic_mixable: Option<bool>,
}

/// Playlist track association
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PlaylistTrack {
    pub id: i64,
    pub position: Option<i32>,
    pub playlist: Option<i64>,
    pub track: Option<i64>,
}

/// Contributor role constants (from Slim/Schema.pm)
pub mod roles {
    pub const ARTIST: i32 = 1;
    pub const COMPOSER: i32 = 2;
    pub const CONDUCTOR: i32 = 3;
    pub const BAND: i32 = 4;
    pub const ALBUMARTIST: i32 = 5;
    pub const TRACKARTIST: i32 = 6;
}

/// Track queries
impl Track {
    /// Find track by ID
    pub async fn find_by_id(pool: &sqlx::SqlitePool, id: i64) -> Result<Option<Track>, sqlx::Error> {
        sqlx::query_as::<_, Track>("SELECT * FROM tracks WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Find tracks by album
    pub async fn find_by_album(pool: &sqlx::SqlitePool, album_id: i64) -> Result<Vec<Track>, sqlx::Error> {
        sqlx::query_as::<_, Track>("SELECT * FROM tracks WHERE album = ? ORDER BY disc, tracknum")
            .bind(album_id)
            .fetch_all(pool)
            .await
    }

    /// Search tracks by title
    pub async fn search_by_title(pool: &sqlx::SqlitePool, query: &str) -> Result<Vec<Track>, sqlx::Error> {
        let search = format!("%{}%", query);
        sqlx::query_as::<_, Track>("SELECT * FROM tracks WHERE titlesearch LIKE ? LIMIT 100")
            .bind(search)
            .fetch_all(pool)
            .await
    }

    /// Insert new track
    pub async fn insert(&self, pool: &sqlx::SqlitePool) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO tracks (url, title, titlesort, titlesearch, album, tracknum, content_type,
             timestamp, filesize, year, secs, cover, bitrate, samplerate, channels, disc, audio, remote, lossless, metadata_hash)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&self.url)
        .bind(&self.title)
        .bind(&self.titlesort)
        .bind(&self.titlesearch)
        .bind(self.album)
        .bind(self.tracknum)
        .bind(&self.content_type)
        .bind(self.timestamp)
        .bind(self.filesize)
        .bind(self.year)
        .bind(self.secs)
        .bind(&self.cover)
        .bind(self.bitrate)
        .bind(self.samplerate)
        .bind(self.channels)
        .bind(self.disc)
        .bind(self.audio)
        .bind(self.remote)
        .bind(self.lossless)
        .bind(&self.metadata_hash)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }
}

/// Album queries
impl Album {
    /// Find album by ID
    pub async fn find_by_id(pool: &sqlx::SqlitePool, id: i64) -> Result<Option<Album>, sqlx::Error> {
        sqlx::query_as::<_, Album>("SELECT * FROM albums WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Search albums by title
    pub async fn search_by_title(pool: &sqlx::SqlitePool, query: &str) -> Result<Vec<Album>, sqlx::Error> {
        let search = format!("%{}%", query);
        sqlx::query_as::<_, Album>("SELECT * FROM albums WHERE titlesearch LIKE ? LIMIT 100")
            .bind(search)
            .fetch_all(pool)
            .await
    }

    /// Insert new album
    pub async fn insert(&self, pool: &sqlx::SqlitePool) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO albums (title, titlesort, titlesearch, compilation, year, disc, discc)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&self.title)
        .bind(&self.titlesort)
        .bind(&self.titlesearch)
        .bind(self.compilation)
        .bind(self.year)
        .bind(self.disc)
        .bind(self.discc)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }
}

/// Contributor queries
impl Contributor {
    /// Find contributor by name (or create if doesn't exist)
    pub async fn find_or_create(pool: &sqlx::SqlitePool, name: &str) -> Result<i64, sqlx::Error> {
        // Search for existing
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM contributors WHERE namesearch = ?"
        )
        .bind(name.to_lowercase())
        .fetch_optional(pool)
        .await?;

        if let Some((id,)) = existing {
            return Ok(id);
        }

        // Create new
        let result = sqlx::query(
            "INSERT INTO contributors (name, namesort, namesearch) VALUES (?, ?, ?)"
        )
        .bind(name)
        .bind(name)
        .bind(name.to_lowercase())
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }
}

/// Genre queries
impl Genre {
    /// Find genre by name (or create if doesn't exist)
    pub async fn find_or_create(pool: &sqlx::SqlitePool, name: &str) -> Result<i64, sqlx::Error> {
        // Search for existing
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM genres WHERE namesearch = ?"
        )
        .bind(name.to_lowercase())
        .fetch_optional(pool)
        .await?;

        if let Some((id,)) = existing {
            return Ok(id);
        }

        // Create new
        let result = sqlx::query(
            "INSERT INTO genres (name, namesort, namesearch) VALUES (?, ?, ?)"
        )
        .bind(name)
        .bind(name)
        .bind(name.to_lowercase())
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }
}
