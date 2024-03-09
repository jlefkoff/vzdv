//! HTTP endpoints for logging in and out.

use crate::{
    shared::{sql, AppError, AppState, UserInfo, SESSION_USER_INFO_KEY},
    utils::auth::{code_to_user_info, get_user_info, oauth_redirect_start, AuthCallback},
};
use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, Redirect},
    routing::get,
    Router,
};
use log::debug;
use minijinja::{context, Environment};
use std::sync::Arc;
use tower_sessions::Session;

/// Login page.
///
/// Doesn't actually have a template to render; the user is immediately redirected to
/// either the homepage if they're already logged in, or the VATSIM OAuth page to start
/// their login flow.
async fn page_auth_login(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Redirect, StatusCode> {
    // if already logged in, just redirect to homepage
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await.unwrap();
    if user_info.is_some() {
        debug!("Already logged-in user hit login page");
        return Ok(Redirect::to("/"));
    }
    let redirect_url = oauth_redirect_start(&state.config);
    Ok(Redirect::to(&redirect_url))
}

/// Auth callback.
///
/// The user is redirected here from VATSIM OAuth providing, in
/// the URL, a code to use in getting an access token for them.
async fn page_auth_callback(
    query: Query<AuthCallback>,
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, AppError> {
    let token_data = code_to_user_info(&query.code, &state).await?;
    let user_info = get_user_info(&token_data.access_token).await?;

    let to_session = UserInfo {
        cid: user_info.data.cid.parse()?,
        first_name: user_info.data.personal.name_first,
        last_name: user_info.data.personal.name_last,
    };
    session
        .insert(SESSION_USER_INFO_KEY, to_session.clone())
        .await?;
    sqlx::query(sql::UPSERT_USER)
        .bind(to_session.cid)
        .bind(&to_session.first_name)
        .bind(&to_session.last_name)
        .bind(&user_info.data.personal.email)
        .execute(&state.db)
        .await?;

    debug!("Completed log in for {}", user_info.data.cid);
    let template = state.templates.get_template("login_complete")?;
    let rendered = template.render(context! { user_info => to_session })?;
    Ok(Html(rendered))
}

/// Clear session and redirect to homepage.
async fn page_auth_logout(session: Session) -> Result<Redirect, AppError> {
    // don't need to check if there's something here
    session.delete().await?;
    Ok(Redirect::to("/"))
}

/// This file's routes and templates.
pub fn router(templates: &mut Environment) -> Router<Arc<AppState>> {
    templates
        .add_template(
            "login_complete",
            include_str!("../../templates/login_complete.jinja"),
        )
        .unwrap();

    Router::new()
        .route("/auth/log_in", get(page_auth_login))
        .route("/auth/logout", get(page_auth_logout))
        .route("/auth/callback", get(page_auth_callback))
}
