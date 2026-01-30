//! Database connection pool wrapper

use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

/// Thread-safe database pool wrapper
#[derive(Clone)]
pub struct DatabasePool {
    pool: Arc<SqlitePool>,
}

impl DatabasePool {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    pub fn get(&self) -> &SqlitePool {
        &self.pool
    }
}
