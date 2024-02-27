//! Structs and data to be shared across multiple parts of the site.

#![allow(unused)]

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use minijinja::Environment;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

mod config;
pub use config::{Config, DEFAULT_CONFIG_FILE_NAME};
pub mod sql;

/// Wrapper of anyhow's `Error` type.
pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

/// Site's shared config. Made available in all handlers.
pub struct AppState {
    pub config: config::Config,
    pub db: SqlitePool,
    pub templates: Environment<'static>,
}

/// Key for user info CRUD in session.
pub const SESSION_USER_INFO_KEY: &str = "USER_INFO";

/// Data stored in the user's session.
#[derive(Debug, Deserialize, Serialize)]
pub struct UserInfo {
    pub cid: u32,
    pub first_name: String,
    pub last_name: String,
}

#[allow(clippy::upper_case_acronyms)]
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

#[allow(clippy::upper_case_acronyms)]
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
    INS,
    MTR,
}
