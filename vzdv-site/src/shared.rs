//! Structs and data to be shared across multiple parts of the site.

use anyhow::{anyhow, Result};
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use chrono::{NaiveDateTime, TimeZone};
use log::error;
use mini_moka::sync::Cache;
use minijinja::{context, Environment};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Instant};
use tower_sessions_sqlx_store::sqlx::SqlitePool;
use vzdv::{
    config::Config,
    controller_can_see,
    sql::{self, Controller},
    PermissionsGroup,
};

/// Wrapper around `anyhow`'s `Error` type, which is itself a wrapper
/// around the stdlib's `Error` type.
pub struct AppError(anyhow::Error);

/// Try to construct the error page.
fn try_build_error_page() -> Result<String> {
    let mut env = Environment::new();
    env.add_template("_layout", include_str!("../templates/_layout.jinja"))?;
    env.add_template("_error", include_str!("../templates/_error.jinja"))?;
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
    pub config: Config,
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
    pub is_staff: bool, // TODO I'm not sure I like this here
}

/// Returns a response to redirect to the homepage for non-staff users.
///
/// This function checks the database to ensure that the staff member is
/// still actually a staff member at the time of making the request.
///
/// So long as the permissions being checked against aren't `PermissionsGroup::Anon`,
/// it's safe to assume that `user_info` is `Some<UserInfo>`.
pub async fn reject_if_not_in(
    state: &Arc<AppState>,
    user_info: &Option<UserInfo>,
    permissions: PermissionsGroup,
) -> Option<Redirect> {
    if is_user_member_of(state, user_info, permissions).await {
        None
    } else {
        Some(Redirect::to("/"))
    }
}

/// Return whether the user is a member of the corresponding staff group.
///
/// This function checks the database to ensure that the staff member is
/// still actually a staff member at the time of making the request.
///
/// So long as the permissions being checked against aren't `PermissionsGroup::Anon`,
/// it's safe to assume that `user_info` is `Some<UserInfo>`.
pub async fn is_user_member_of(
    state: &Arc<AppState>,
    user_info: &Option<UserInfo>,
    permissions: PermissionsGroup,
) -> bool {
    if user_info.is_none() {
        return false;
    }
    let user_info = user_info.as_ref().unwrap();
    let controller: Option<Controller> = match sqlx::query_as(sql::GET_CONTROLLER_BY_CID)
        .bind(user_info.cid)
        .fetch_optional(&state.db)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            error!(
                "Could not look up staff controller with CID {}: {e}",
                user_info.cid
            );
            return false;
        }
    };
    controller_can_see(&controller, permissions)
}

/// Convert an HTML `datetime-local` input and JS timezone name to a UTC timestamp.
///
/// Kind of annoying.
pub fn js_timestamp_to_utc(timestamp: &str, timezone: &str) -> Result<NaiveDateTime> {
    let tz: chrono_tz::Tz = timezone.parse()?;
    let original = NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M")?;
    let converted = tz
        .from_local_datetime(&original)
        .single()
        .ok_or_else(|| anyhow!("Error parsing HTML datetime"))?
        .naive_utc();
    Ok(converted)
}
