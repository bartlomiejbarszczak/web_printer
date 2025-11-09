use sqlx::{sqlite::{SqlitePoolOptions, SqliteConnectOptions}, SqlitePool};
use std::str::FromStr;
use std::{fs, path::Path};
use std::time::Duration;
use sqlx::sqlite::{SqliteJournalMode, SqliteSynchronous};

pub mod migrations;


pub async fn init_database() -> Result<SqlitePool, sqlx::Error> {
    let db_path = Path::new("data/print_scan_manager.db");
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {sqlx::Error::Io(e)})?;
    }

    let options = SqliteConnectOptions::from_str(db_path.to_str().unwrap())?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .pragma("cache_size", "10000")
        .pragma("temp_store", "memory")
        .pragma("mmap_size", "134217728")
        .pragma("page_size", "4096");

    let pool = SqlitePoolOptions::new()
        .max_connections(7)
        .min_connections(2)
        .max_lifetime(Duration::from_secs(30 * 60))
        .connect_with(options)
        .await?;

    migrations::run_migrations(&pool).await?;

    Ok(pool)
}

