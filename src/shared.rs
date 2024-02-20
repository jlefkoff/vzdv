//! Structs and data to be shared across multiple files.

use minijinja::Environment;
use serde::Deserialize;
use sqlx::SqlitePool;

/// Site's shared config. Available in all handlers.
pub struct AppState {
    pub config: Config,
    pub db: SqlitePool,
    pub templates: Environment<'static>,
}

/// Default place to look for the config file.
pub const DEFAULT_CONFIG_FILE_NAME: &str = "site_config.toml";

/// App configuration. Includes, but isn't limited
/// to, the configuration for just the site.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub database: ConfigDatabase,
}

#[derive(Debug, Deserialize)]
pub struct ConfigDatabase {
    pub file: String,
}
