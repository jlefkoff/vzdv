use minijinja::Environment;
use serde::Deserialize;
use sqlx::SqlitePool;

pub struct AppState {
    pub config: Config,
    pub db: SqlitePool,
    pub templates: Environment<'static>,
}

pub const CONFIG_FILE_NAME: &str = "site_config.toml";

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database: ConfigDatabase,
}

#[derive(Debug, Deserialize)]
pub struct ConfigDatabase {
    pub file: String,
}
