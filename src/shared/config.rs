use serde::Deserialize;

/// Default place to look for the config file.
pub const DEFAULT_CONFIG_FILE_NAME: &str = "site_config.toml";

/// App configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub database: ConfigDatabase,
    pub vatsim: ConfigVatsim,
    pub airports: ConfigAirports,
    pub stats: ConfigStats,
}

#[derive(Debug, Deserialize)]
pub struct ConfigDatabase {
    pub file: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigVatsim {
    pub oauth_client_id: String,
    pub oauth_client_secret: String,
    pub oauth_client_calback_url: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigAirports {
    pub all: Vec<Airport>,
    pub weather_for: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Airport {
    pub code: String,
    pub name: String,
    pub location: String,
    pub towered: bool,
}

#[derive(Debug, Deserialize)]
pub struct ConfigStats {
    pub position_prefixes: Vec<String>,
    pub position_suffixes: Vec<String>,
}
