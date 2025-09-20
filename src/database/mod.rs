use sqlx::{sqlite::{SqlitePoolOptions, SqliteConnectOptions}, SqlitePool};
use std::str::FromStr;
use std::{fs, path::Path};

pub mod migrations;


pub async fn init_database() -> Result<SqlitePool, sqlx::Error> {
    let db_path = Path::new("data/print_scan_manager.db");
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {sqlx::Error::Io(e)})?;
    }

    let options = SqliteConnectOptions::from_str(db_path.to_str().unwrap())?
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    migrations::run_migrations(&pool).await?;

    // Pi Zero 2W optimizations
    sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await?;
    sqlx::query("PRAGMA synchronous=NORMAL").execute(&pool).await?;
    sqlx::query("PRAGMA cache_size=10000").execute(&pool).await?;
    sqlx::query("PRAGMA temp_store=memory").execute(&pool).await?;

    Ok(pool)
}

