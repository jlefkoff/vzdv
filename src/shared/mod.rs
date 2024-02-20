//! Structs and data to be shared across multiple parts of the site.

#![allow(unused, clippy::upper_case_acronyms)]

use minijinja::Environment;
use sqlx::SqlitePool;

mod config;
pub use config::{Config, DEFAULT_CONFIG_FILE_NAME};
pub mod sql;

/// Site's shared config. Made available in all handlers.
pub struct AppState {
    pub config: config::Config,
    pub db: SqlitePool,
    pub templates: Environment<'static>,
}

pub enum ControllerRating {
    OBS,
    S1,
    S2,
    S3,
    C1,
    C3,
    L1,
    L3,
    SUP,
    ADM,
    INA,
}

pub enum ControllerStatus {
    Active,
    Inactive,
    LeaveOfAbsence,
}

pub enum StaffPosition {
    None,
    ATM,
    DATM,
    TA,
    FE,
    EC,
    WM,
    AFE,
    AEC,
    AWM,
}
