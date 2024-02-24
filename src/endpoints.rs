//! HTTP endpoints.

use crate::shared::AppState;
use anyhow::Result;
use axum::{extract::State, http::StatusCode, response::Html};
use minijinja::context;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_sessions::Session;

#[derive(Serialize, Deserialize, Default)]
struct Counter(usize);

pub async fn handler_home(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, StatusCode> {
    let counter: Counter = session.get("COUNTER").await.unwrap().unwrap_or_default();
    let template = state.templates.get_template("home").unwrap();
    let rendered = template
        .render(context! {
            title => "Home",
            welcome_text => "Hello World!",
            counter => counter,
        })
        .unwrap();

    Ok(Html(rendered))
}
