use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// Default place to look for the config file.
pub const DEFAULT_CONFIG_FILE_NAME: &str = "vzdv.toml";

/// App configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    pub hosted_domain: String,
    pub database: ConfigDatabase,
    pub staff: ConfigStaff,
    pub vatsim: ConfigVatsim,
    pub training: ConfigTraining,
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
pub struct ConfigStaff {
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
pub struct ConfigTraining {
    pub certifications: Vec<String>,
    pub training_types: Vec<String>,
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
    pub auth: ConfigDiscordAuth,
    pub guild_id: u64,
    pub online_channel: u64,
    pub online_message: Option<u64>,
    pub off_roster_channel: u64,
    pub webhooks: ConfigDiscordWebhooks,
    pub roles: ConfigDiscordRoles,
    pub owner_id: u64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigDiscordAuth {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigDiscordWebhooks {
    pub staffing_request: String,
    pub feedback: String,
    pub new_visitor_app: String,
    pub errors: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigDiscordRoles {
    // status
    pub guest: u64,
    pub controller_otm: u64,
    pub home_controller: u64,
    pub visiting_controller: u64,
    pub neighboring_controller: u64,
    pub event_controller: u64,

    // staff
    pub sr_staff: u64,
    pub jr_staff: u64,
    pub vatusa_vatgov_staff: u64,

    // staff teams
    pub training_staff: u64,
    pub event_team: u64,
    pub fe_team: u64,
    pub web_team: u64,
    pub ace_team: u64,

    // ratings
    pub administrator: u64,
    pub supervisor: u64,
    pub instructor_3: u64,
    pub instructor_1: u64,
    pub controller_3: u64,
    pub controller_1: u64,
    pub student_3: u64,
    pub student_2: u64,
    pub student_1: u64,
    pub observer: u64,
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
