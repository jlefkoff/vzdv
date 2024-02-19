use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database: ConfigDatabase,
}

#[derive(Debug, Deserialize)]
pub struct ConfigDatabase {
    pub file: String,
}
