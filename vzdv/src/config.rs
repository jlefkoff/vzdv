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
    pub email: ConfigEmail,
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
    pub guild_id: u64,
    pub webhooks: ConfigDiscordWebhooks,
    pub roles: ConfigDiscordRoles,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigDiscordWebhooks {
    pub staffing_request: String,
    pub feedback: String,
    pub new_visitor_app: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigDiscordRoles {
    // status
    pub guest: String,
    pub controller_otm: String,
    pub home_controller: String,
    pub visiting_controller: String,
    pub neighboring_controller: String,
    pub event_controller: String,

    // staff
    pub sr_staff: String,
    pub jr_staff: String,
    pub vatusa_vatgov_staff: String,

    // staff teams
    pub training_staff: String,
    pub event_team: String,
    pub fe_team: String,
    pub web_team: String,
    pub ace_team: String,

    // ratings
    pub administrator: String,
    pub supervisor: String,
    pub instructor_3: String,
    pub instructor_1: String,
    pub controller_3: String,
    pub controller_1: String,
    pub student_3: String,
    pub student_2: String,
    pub student_1: String,
    pub observer: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigEmailTemplate {
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigEmail {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub from: String,
    pub reply_to: String,

    pub visitor_accepted_template: ConfigEmailTemplate,
    pub visitor_denied_template: ConfigEmailTemplate,
    pub visitor_removed_template: ConfigEmailTemplate,
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
