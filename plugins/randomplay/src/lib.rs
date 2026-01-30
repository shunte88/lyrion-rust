//! RandomPlay Plugin
//!
//! Generates random playlists from the music library with various mix modes:
//! - Tracks: Random individual tracks
//! - Albums: Random full albums
//! - Artists: Random tracks from random artists
//! - Years: Random tracks from random years

use lyrion_plugins::{
    HttpRequest, HttpResponse, HttpRoute, Plugin, PluginContext, PluginManifest, PluginResult,
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// RandomPlay plugin state
pub struct RandomPlayPlugin {
    /// Database connection pool
    db_pool: Option<sqlx::SqlitePool>,

    /// Number of tracks to generate per request
    track_count: usize,
}

impl RandomPlayPlugin {
    /// Create a new RandomPlay plugin
    pub fn new() -> Self {
        Self {
            db_pool: None,
            track_count: 20,
        }
    }

    /// Generate random tracks
    async fn generate_random_tracks(&self, count: usize) -> PluginResult<Vec<Track>> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| "Database not initialized".to_string())?;

        let tracks: Vec<Track> = sqlx::query_as::<_, Track>(
            "SELECT tracks.id, tracks.url, tracks.title,
                    NULL as artist, NULL as album, tracks.year, tracks.secs
             FROM tracks
             WHERE tracks.audio = 1
             ORDER BY RANDOM()
             LIMIT ?",
        )
        .bind(count as i64)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database query failed: {}", e))?;

        Ok(tracks)
    }

    /// Generate random albums
    async fn generate_random_albums(&self, count: usize) -> PluginResult<Vec<Track>> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| "Database not initialized".to_string())?;

        // First get random albums
        let albums: Vec<AlbumInfo> = sqlx::query_as::<_, AlbumInfo>(
            "SELECT DISTINCT album.id, album.title
             FROM albums album
             JOIN tracks ON tracks.album = album.id
             WHERE tracks.audio = 1
             ORDER BY RANDOM()
             LIMIT ?",
        )
        .bind(count as i64)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database query failed: {}", e))?;

        // Get all tracks from those albums
        let mut all_tracks = Vec::new();
        for album in albums {
            let tracks: Vec<Track> = sqlx::query_as::<_, Track>(
                "SELECT tracks.id, tracks.url, tracks.title,
                        NULL as artist, NULL as album, tracks.year, tracks.secs
                 FROM tracks
                 WHERE tracks.album = ? AND tracks.audio = 1
                 ORDER BY tracks.disc, tracks.tracknum",
            )
            .bind(album.id)
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Database query failed: {}", e))?;

            all_tracks.extend(tracks);
        }

        Ok(all_tracks)
    }

    /// Generate random tracks from random artists
    async fn generate_random_artists(&self, count: usize) -> PluginResult<Vec<Track>> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| "Database not initialized".to_string())?;

        // Get random artists
        let artists: Vec<ArtistInfo> = sqlx::query_as::<_, ArtistInfo>(
            "SELECT DISTINCT contributor.id, contributor.name
             FROM contributors contributor
             JOIN contributor_track ON contributor_track.contributor = contributor.id
             JOIN tracks ON tracks.id = contributor_track.track
             WHERE tracks.audio = 1
             ORDER BY RANDOM()
             LIMIT ?",
        )
        .bind(count as i64)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database query failed: {}", e))?;

        // Get tracks from those artists (5 tracks per artist)
        let mut all_tracks = Vec::new();
        for artist in artists {
            let tracks: Vec<Track> = sqlx::query_as::<_, Track>(
                "SELECT tracks.id, tracks.url, tracks.title,
                        NULL as artist, NULL as album, tracks.year, tracks.secs
                 FROM tracks
                 JOIN contributor_track ON contributor_track.track = tracks.id
                 WHERE contributor_track.contributor = ? AND tracks.audio = 1
                 ORDER BY RANDOM()
                 LIMIT 5",
            )
            .bind(artist.id)
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Database query failed: {}", e))?;

            all_tracks.extend(tracks);
        }

        // Shuffle to mix artists
        let mut rng = rand::thread_rng();
        all_tracks.shuffle(&mut rng);

        Ok(all_tracks)
    }

    /// Generate random tracks from random years
    async fn generate_random_years(&self, count: usize) -> PluginResult<Vec<Track>> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| "Database not initialized".to_string())?;

        // Get random years
        let years: Vec<YearInfo> = sqlx::query_as::<_, YearInfo>(
            "SELECT DISTINCT year
             FROM tracks
             WHERE year IS NOT NULL AND year > 0 AND audio = 1
             ORDER BY RANDOM()
             LIMIT ?",
        )
        .bind(count as i64)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database query failed: {}", e))?;

        // Get tracks from those years (5 tracks per year)
        let mut all_tracks = Vec::new();
        for year_info in years {
            let tracks: Vec<Track> = sqlx::query_as::<_, Track>(
                "SELECT tracks.id, tracks.url, tracks.title,
                        NULL as artist, NULL as album, tracks.year, tracks.secs
                 FROM tracks
                 WHERE tracks.year = ? AND tracks.audio = 1
                 ORDER BY RANDOM()
                 LIMIT 5",
            )
            .bind(year_info.year)
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Database query failed: {}", e))?;

            all_tracks.extend(tracks);
        }

        // Shuffle to mix years
        let mut rng = rand::thread_rng();
        all_tracks.shuffle(&mut rng);

        Ok(all_tracks)
    }
}

impl Plugin for RandomPlayPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            name: "RandomPlay".to_string(),
            version: "1.0.0".to_string(),
            author: "Lyrion Community".to_string(),
            description: Some("Generate random playlists by track, album, artist, or year".to_string()),
            min_server_version: Some("0.1.0".to_string()),
            dependencies: vec![],
            capabilities: vec!["cli".to_string(), "http".to_string()],
            enforced: false,
        }
    }

    fn init(&mut self, context: &PluginContext) -> Result<(), String> {
        // Store database pool
        self.db_pool = Some(context.db_pool.clone());

        // Read track count preference if available
        if let Some(count_str) = context.preferences.get("newtracks") {
            if let Ok(count) = count_str.parse::<usize>() {
                self.track_count = count;
            }
        }

        Ok(())
    }

    fn shutdown(&mut self) {
        self.db_pool = None;
    }

    fn handle_command(
        &mut self,
        command: &str,
        params: &HashMap<String, String>,
    ) -> Option<String> {
        if command != "randomplay" {
            return None;
        }

        let mode = params.get("mode").map(|s| s.as_str()).unwrap_or("tracks");
        let count = params
            .get("count")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(self.track_count);

        // Clone database pool for async operation
        let db_pool = self.db_pool.clone()?;

        // Create a new Tokio runtime for executing async code from sync context
        let runtime = tokio::runtime::Runtime::new().ok()?;

        let result = runtime.block_on(async {
            let plugin = RandomPlayPlugin {
                db_pool: Some(db_pool),
                track_count: count,
            };

            match mode {
                "tracks" => plugin.generate_random_tracks(count).await,
                "albums" => plugin.generate_random_albums(5).await,
                "artists" => plugin.generate_random_artists(10).await,
                "years" => plugin.generate_random_years(10).await,
                _ => Err(format!("Unknown mode: {}", mode).into()),
            }
        });

        match result {
            Ok(tracks) => {
                let response = RandomPlayResponse {
                    mode: mode.to_string(),
                    count: tracks.len(),
                    tracks,
                };
                serde_json::to_string(&response).ok()
            }
            Err(e) => Some(format!("Error: {}", e)),
        }
    }

    fn http_routes(&self) -> Vec<HttpRoute> {
        vec![
            HttpRoute {
                method: "GET".to_string(),
                path: "/plugins/randomplay/tracks".to_string(),
                handler_id: "random_tracks".to_string(),
            },
            HttpRoute {
                method: "GET".to_string(),
                path: "/plugins/randomplay/albums".to_string(),
                handler_id: "random_albums".to_string(),
            },
            HttpRoute {
                method: "GET".to_string(),
                path: "/plugins/randomplay/artists".to_string(),
                handler_id: "random_artists".to_string(),
            },
            HttpRoute {
                method: "GET".to_string(),
                path: "/plugins/randomplay/years".to_string(),
                handler_id: "random_years".to_string(),
            },
        ]
    }

    fn handle_http_request(&mut self, request: HttpRequest) -> Result<HttpResponse, String> {
        // Extract count parameter from query string
        let count = request
            .query
            .get("count")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(self.track_count);

        // Clone database pool for async operation
        let db_pool = self.db_pool.clone().ok_or_else(|| "Database not initialized".to_string())?;
        let path = request.path.clone();

        // Create a new Tokio runtime for executing async code from sync context
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        let result = runtime.block_on(async {
            let plugin = RandomPlayPlugin {
                db_pool: Some(db_pool),
                track_count: count,
            };

            if path.ends_with("/tracks") {
                plugin.generate_random_tracks(count).await
            } else if path.ends_with("/albums") {
                plugin.generate_random_albums(5).await
            } else if path.ends_with("/artists") {
                plugin.generate_random_artists(10).await
            } else if path.ends_with("/years") {
                plugin.generate_random_years(10).await
            } else {
                Err("Unknown endpoint".to_string().into())
            }
        });

        match result {
            Ok(tracks) => {
                let response = RandomPlayResponse {
                    mode: if request.path.ends_with("/tracks") {
                        "tracks"
                    } else if request.path.ends_with("/albums") {
                        "albums"
                    } else if request.path.ends_with("/artists") {
                        "artists"
                    } else {
                        "years"
                    }
                    .to_string(),
                    count: tracks.len(),
                    tracks,
                };
                HttpResponse::json(response).map_err(|e| e.to_string())
            }
            Err(e) => Ok(HttpResponse::error(500, format!("Error: {}", e))),
        }
    }

    fn settings_schema(&self) -> Option<String> {
        Some(
            r#"{
            "type": "object",
            "properties": {
                "newtracks": {
                    "type": "integer",
                    "title": "Number of tracks to add",
                    "default": 20,
                    "minimum": 1,
                    "maximum": 100
                }
            }
        }"#
            .to_string(),
        )
    }
}

/// Track information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Track {
    id: i64,
    url: String,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    year: Option<i32>,
    secs: Option<f64>,
}

// Implement FromRow manually for Track
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Track {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Track {
            id: row.try_get("id")?,
            url: row.try_get("url")?,
            title: row.try_get("title").ok(),
            artist: row.try_get("artist").ok(),
            album: row.try_get("album").ok(),
            year: row.try_get("year").ok(),
            secs: row.try_get("secs").ok(),
        })
    }
}

/// Album information
#[derive(Debug)]
struct AlbumInfo {
    id: i64,
    title: Option<String>,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for AlbumInfo {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(AlbumInfo {
            id: row.try_get("id")?,
            title: row.try_get("title").ok(),
        })
    }
}

/// Artist information
#[derive(Debug)]
struct ArtistInfo {
    id: i64,
    name: Option<String>,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for ArtistInfo {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(ArtistInfo {
            id: row.try_get("id")?,
            name: row.try_get("name").ok(),
        })
    }
}

/// Year information
#[derive(Debug)]
struct YearInfo {
    year: i32,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for YearInfo {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(YearInfo {
            year: row.try_get("year")?,
        })
    }
}

/// Response format for random play requests
#[derive(Debug, Serialize)]
struct RandomPlayResponse {
    mode: String,
    count: usize,
    tracks: Vec<Track>,
}

/// Plugin constructor - called by plugin loader
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    let plugin = Box::new(RandomPlayPlugin::new());
    Box::into_raw(plugin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manifest() {
        let plugin = RandomPlayPlugin::new();
        let manifest = plugin.manifest();
        assert_eq!(manifest.name, "RandomPlay");
        assert_eq!(manifest.version, "1.0.0");
        assert!(manifest.capabilities.contains(&"cli".to_string()));
        assert!(manifest.capabilities.contains(&"http".to_string()));
    }

    #[test]
    fn test_http_routes() {
        let plugin = RandomPlayPlugin::new();
        let routes = plugin.http_routes();
        assert_eq!(routes.len(), 4);
        assert!(routes.iter().any(|r| r.path == "/plugins/randomplay/tracks"));
        assert!(routes.iter().any(|r| r.path == "/plugins/randomplay/albums"));
    }
}
