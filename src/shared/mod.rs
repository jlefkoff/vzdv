//! Structs and data to be shared across multiple parts of the site.

#![allow(unused)]

use std::time::Instant;

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use log::error;
use mini_moka::sync::Cache;
use minijinja::{context, Environment};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;

mod config;
pub use config::{Config, DEFAULT_CONFIG_FILE_NAME};
pub mod sql;

/// Wrapper around `anyhow`'s `Error` type, which is itself a wrapper
/// around the stdlib's `Error` type.
pub struct AppError(anyhow::Error);

/// Try to construct the error page.
fn try_build_error_page() -> anyhow::Result<String> {
    let mut env = Environment::new();
    env.add_template("_layout", include_str!("../../templates/_layout.jinja"))?;
    env.add_template("_error", include_str!("../../templates/_error.jinja"))?;
    let template = env.get_template("_error")?;
    let rendered = template.render(context! {})?;
    Ok(rendered)
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        error!("Unhandled error: {}", self.0);
        // attempt to construct the error page, falling back to plain text if anything failed
        if let Ok(body) = try_build_error_page() {
            (StatusCode::INTERNAL_SERVER_ERROR, Html(body)).into_response()
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went very wrong",
            )
                .into_response()
        }
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

/// Data wrapper for items in the server-side cache.
#[derive(Clone)]
pub struct CacheEntry {
    pub inserted: Instant,
    pub data: String,
}

impl CacheEntry {
    /// Wrap the data with a timestamp.
    pub fn new(data: String) -> Self {
        Self {
            inserted: Instant::now(),
            data,
        }
    }
}

/// App's state, available in all handlers via an extractor.
pub struct AppState {
    /// App config
    pub config: config::Config,
    /// Access to the DB
    pub db: SqlitePool,
    /// Loaded templates
    pub templates: Environment<'static>,
    /// Server-side cache
    pub cache: Cache<&'static str, CacheEntry>,
}

/// Key for user info CRUD in session.
pub const SESSION_USER_INFO_KEY: &str = "USER_INFO";
/// Key for flashed messages CRUD in session.
pub const SESSION_FLASHED_MESSAGES_KEY: &str = "FLASHED_MESSAGES";

/// Data stored in the user's session.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserInfo {
    pub cid: u32,
    pub first_name: String,
    pub last_name: String,
    pub is_staff: bool,
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
