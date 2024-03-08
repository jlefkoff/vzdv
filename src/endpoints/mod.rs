//! HTTP endpoints.

use crate::shared::{AppState, UserInfo, SESSION_USER_INFO_KEY};
use anyhow::Result;
use axum::{extract::State, http::StatusCode, response::Html, routing::get, Router};
use minijinja::{context, Environment};
use std::sync::Arc;
use tower_sessions::Session;

pub mod auth;
pub mod homepage;

/// 404 not found page.
///
/// Redirected to whenever the router cannot find a valid handler for the requested path.
async fn handler_404(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, StatusCode> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await.unwrap();
    let template = state.templates.get_template("404").unwrap();
    let rendered = template.render(context! { user_info }).unwrap();
    Ok(Html(rendered))
}

/// This file's routes and templates.
pub fn router(templates: &mut Environment) -> Router<Arc<AppState>> {
    templates
        .add_template("404", include_str!("../../templates/404.jinja"))
        .unwrap();

    Router::new().route("/404", get(handler_404))
}
