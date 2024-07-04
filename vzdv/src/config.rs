use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// Default place to look for the config file.
pub const DEFAULT_CONFIG_FILE_NAME: &str = "vzdv.toml";

/// App configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    pub database: ConfigDatabase,
    pub staff: ConfigStaff,
    pub vatsim: ConfigVatsim,
    pub airports: ConfigAirports,
    pub stats: ConfigStats,
    pub discord: ConfigDiscord,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigDatabase {
    pub file: String,
    pub resource_category_ordering: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigStaffOverride {
    pub role: String,
    pub cid: u32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigStaff {
    pub overrides: Vec<ConfigStaffOverride>,
    pub email_domain: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigVatsim {
    pub oauth_url_base: String,
    pub oauth_client_id: String,
    pub oauth_client_secret: String,
    pub oauth_client_callback_url: String,
    pub vatusa_api_key: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigAirports {
    pub all: Vec<Airport>,
    pub weather_for: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Airport {
    pub code: String,
    pub name: String,
    pub location: String,
    pub towered: bool,
    pub class: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigStats {
    pub position_prefixes: Vec<String>,
    pub position_suffixes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigDiscord {
    pub join_link: String,
    pub bot_token: String,
    pub webhooks: ConfigDiscordWebhooks,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigDiscordWebhooks {
    pub staffing_request: String,
    pub feedback: String,
}

impl Config {
    /// Read the TOML file at the given path and load into the app's configuration file.
    pub fn load_from_disk(path: &Path) -> Result<Self> {
        if !Path::new(path).exists() {
            bail!("Config file \"{}\" not found", path.display());
        }
        let text = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&text)?;
        Ok(config)
    }
}
