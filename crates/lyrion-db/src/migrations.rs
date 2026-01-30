//! Database migration utilities
//! SQLx will automatically handle migrations from the migrations/ directory

use sqlx::SqlitePool;
use anyhow::Result;

/// Get current schema version
pub async fn get_schema_version(pool: &SqlitePool) -> Result<Option<i32>> {
    let version: Option<String> = sqlx::query_scalar(
        "SELECT value FROM metainformation WHERE name = 'schema_version'"
    )
    .fetch_optional(pool)
    .await?;

    Ok(version.and_then(|v| v.parse().ok()))
}

/// Set schema version
pub async fn set_schema_version(pool: &SqlitePool, version: i32) -> Result<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO metainformation (name, value) VALUES ('schema_version', ?)"
    )
    .bind(version.to_string())
    .execute(pool)
    .await?;

    Ok(())
}
