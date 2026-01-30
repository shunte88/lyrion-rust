//! Database layer for Lyrion Music Server
//! Ported from Slim/Schema.pm and SQL/SQLite schema files

pub mod models;
pub mod pool;
pub mod migrations;
pub mod duckdb_search;

pub use models::*;
pub use pool::DatabasePool;

use anyhow::Result;
use sqlx::sqlite::SqlitePool;

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub path: String,
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "lyrion-rust.db".to_string(),
            max_connections: 10,
        }
    }
}

/// Initialize database with migrations
pub async fn initialize_database(config: &DatabaseConfig) -> Result<SqlitePool> {

    tracing::info!("Migrating database...");

    let db_url = format!("sqlite:{}?mode=rwc", config.path);

    tracing::info!("We want DuckDB!");
    
    let pool = SqlitePool::connect(&db_url).await?;

    // Run migrations
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await?;

    tracing::info!("Database initialized at {}", config.path);

    Ok(pool)
}

/// Migrate from existing Perl database
pub async fn migrate_from_perl(perl_db_path: &str, rust_db_path: &str) -> Result<()> {
    // Copy the database file
    std::fs::copy(perl_db_path, rust_db_path)?;

    tracing::info!("Copied database from {} to {}", perl_db_path, rust_db_path);

    // Connect and verify schema version
    let db_url = format!("sqlite:{}?mode=rw", rust_db_path);
    let pool = SqlitePool::connect(&db_url).await?;

    let version: Option<String> = sqlx::query_scalar(
        "SELECT value FROM metainformation WHERE name = 'schema_version'"
    )
    .fetch_optional(&pool)
    .await?;

    match version {
        Some(v) => {
            tracing::info!("Existing database schema version: {}", v);
            let ver: i32 = v.parse().unwrap_or(0);
            if ver < 26 {
                tracing::warn!("Schema version {} is outdated. Running migrations...", ver);
                sqlx::migrate!("../../migrations").run(&pool).await?;
            }
        }
        None => {
            tracing::warn!("No schema version found. Database may need initialization.");
        }
    }

    pool.close().await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initialize_database() {
        let config = DatabaseConfig {
            path: ":memory:".to_string(),
            max_connections: 5,
        };

        let result = initialize_database(&config).await;
        assert!(result.is_ok());
    }
}
