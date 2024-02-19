use crate::shared::AppState;
use anyhow::Result;
use axum::{extract::State, http::StatusCode, response::Html};
use minijinja::context;
use std::sync::Arc;

pub async fn handler_home(State(state): State<Arc<AppState>>) -> Result<Html<String>, StatusCode> {
    let template = state.templates.get_template("home").unwrap();

    let rendered = template
        .render(context! {
            title => "Home",
            welcome_text => "Hello World!",
        })
        .unwrap();

    Ok(Html(rendered))
}
