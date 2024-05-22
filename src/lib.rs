//! New vZVD core logic.

#![deny(clippy::all)]

use anyhow::Result;
use shared::Config;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    Executor, SqlitePool,
};
use std::path::Path;

pub mod endpoints;
pub mod middleware;
pub mod shared;
pub mod utils;

/// Read the TOML file at the given path and load into the app's
/// configuration file.
pub fn load_config(path: &Path) -> Result<Config> {
    let text = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&text)?;
    Ok(config)
}

/// Connect to the SQLite file at the destination, if it exists. If it does
/// not, a new file is created and statements to create tables are executed.
pub async fn load_db(config: &Config) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(&config.database.file)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);
    let pool = if !Path::new(&config.database.file).exists() {
        let options = options.create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;
        pool.execute(shared::sql::CREATE_TABLES).await?;
        pool
    } else {
        SqlitePool::connect_with(options).await?
    };
    Ok(pool)
}
