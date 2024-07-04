//! HTTP endpoints for user-specific pages.

use crate::shared::{AppError, AppState, UserInfo, SESSION_USER_INFO_KEY};
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use minijinja::{context, Environment};
use std::sync::Arc;
use tower_sessions::Session;
use vzdv::vatusa;

/// Retrieve and show the user their training records from VATUSA.
async fn page_training_notes(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Response, AppError> {
    use voca_rs::Voca;

    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if user_info.is_none() {
        return Ok(Redirect::to("/").into_response());
    }
    let mut training_records = vatusa::get_training_records(
        &state.config.vatsim.vatusa_api_key,
        user_info.as_ref().unwrap().cid,
    )
    .await?;
    for record in &mut training_records {
        record.notes = record.notes._strip_tags();
    }
    let template = state.templates.get_template("user/training_notes")?;
    let rendered = template.render(context! { user_info, training_records })?;
    Ok(Html(rendered).into_response())
}

/// Show the user a link to the Discord server, as well as provide
/// the start of the Discord OAuth flow for account linking.
async fn page_discord(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Response, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if user_info.is_none() {
        return Ok(Redirect::to("/").into_response());
    }
    let template = state.templates.get_template("user/discord")?;
    let rendered = template.render(context! {
       user_info,
       join_link => &state.config.discord.join_link
    })?;
    Ok(Html(rendered).into_response())
}

pub fn router(templates: &mut Environment) -> Router<Arc<AppState>> {
    templates
        .add_template(
            "user/training_notes",
            include_str!("../../templates/user/training_notes.jinja"),
        )
        .unwrap();
    templates
        .add_template(
            "user/discord",
            include_str!("../../templates/user/discord.jinja"),
        )
        .unwrap();

    Router::new()
        .route("/user/training_notes", get(page_training_notes))
        .route("/user/discord", get(page_discord))
}
