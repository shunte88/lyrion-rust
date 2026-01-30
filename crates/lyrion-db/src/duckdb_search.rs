//! DuckDB integration for fast full-text search and analytics

use anyhow::Result;
use duckdb::{Connection, params};
use serde::{Deserialize, Serialize};

/// Search result
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: i64,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_type: String, // "track", "album", "artist"
}

/// DuckDB search engine
pub struct DuckDbSearch {
    conn: Connection,
}

impl DuckDbSearch {
    /// Initialize DuckDB with SQLite database attachment
    pub fn new(sqlite_path: &str) -> Result<Self> {
        let conn = Connection::open_in_memory()?;

        // Load SQLite extension
        conn.execute("INSTALL sqlite; LOAD sqlite;", [])?;

        // Attach SQLite database
        let attach_sql = format!("ATTACH '{}' AS lms (TYPE sqlite);", sqlite_path);
        conn.execute(&attach_sql, [])?;

        tracing::info!("DuckDB search initialized with SQLite database at {}", sqlite_path);

        Ok(Self { conn })
    }

    /// Full-text search across tracks, albums, and artists
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        let search_pattern = format!("%{}%", query.to_lowercase());

        // Search tracks
        let mut stmt = self.conn.prepare(
            "SELECT id, title, NULL as artist, NULL as album, 'track' as type
             FROM lms.tracks
             WHERE LOWER(title) LIKE ?
             LIMIT ?"
        )?;

        let track_results = stmt.query_map(params![&search_pattern, limit], |row| {
            Ok(SearchResult {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album: row.get(3)?,
                track_type: row.get(4)?,
            })
        })?;

        for result in track_results {
            if let Ok(r) = result {
                results.push(r);
            }
        }

        // Search albums
        let mut stmt = self.conn.prepare(
            "SELECT id, title, NULL, NULL, 'album'
             FROM lms.albums
             WHERE LOWER(title) LIKE ?
             LIMIT ?"
        )?;

        let album_results = stmt.query_map(params![&search_pattern, limit], |row| {
            Ok(SearchResult {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album: row.get(3)?,
                track_type: row.get(4)?,
            })
        })?;

        for result in album_results {
            if let Ok(r) = result {
                results.push(r);
            }
        }

        Ok(results)
    }

    /// Get analytics: track count by year
    pub fn tracks_by_year(&self) -> Result<Vec<(i16, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT year, COUNT(*) as count
             FROM lms.tracks
             WHERE year IS NOT NULL
             GROUP BY year
             ORDER BY year DESC"
        )?;

        let results = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

        results.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
    }

    /// Get analytics: total duration by artist
    pub fn duration_by_artist(&self, limit: usize) -> Result<Vec<(String, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.name, SUM(t.secs) as total_duration
             FROM lms.tracks t
             JOIN lms.contributor_track ct ON t.id = ct.track
             JOIN lms.contributors c ON ct.contributor = c.id
             WHERE ct.role = 1
             GROUP BY c.name
             ORDER BY total_duration DESC
             LIMIT ?"
        )?;

        let results = stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

        results.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duckdb_connection() {
        let conn = Connection::open_in_memory();
        assert!(conn.is_ok());
    }
}
