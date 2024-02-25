//! HTTP endpoints.

use crate::shared::{AppState, UserInfo, SESSION_USER_INFO_KEY};
use anyhow::Result;
use axum::{extract::State, http::StatusCode, response::Html};
use minijinja::context;
use std::sync::Arc;
use tower_sessions::Session;

pub async fn handler_404(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, StatusCode> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await.unwrap();
    let template = state.templates.get_template("404").unwrap();
    let rendered = template.render(context! { user_info }).unwrap();

    Ok(Html(rendered))
}

pub async fn handler_home(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, StatusCode> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await.unwrap();
    let template = state.templates.get_template("home").unwrap();
    let rendered = template.render(context! { user_info }).unwrap();

    Ok(Html(rendered))
}
