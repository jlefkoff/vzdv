use serde::Deserialize;

/// Default place to look for the config file.
pub const DEFAULT_CONFIG_FILE_NAME: &str = "site_config.toml";

/// App configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub general: ConfigGeneral,
    pub database: ConfigDatabase,
    pub airports: ConfigAirports,
}

#[derive(Debug, Deserialize)]
pub struct ConfigGeneral {
    pub facility_short: String,
    pub facility_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigDatabase {
    pub file: String,
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
