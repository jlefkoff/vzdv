//! Endpoints for viewing and registering for events.
//!
//! The CRUD of events themselves is under /admin routes.

use crate::{
    flashed_messages,
    shared::{AppError, AppState, UserInfo, SESSION_USER_INFO_KEY},
};
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use minijinja::{context, Environment};
use std::sync::Arc;
use tower_sessions::Session;
use vzdv::sql::{self, Event};

/// Render a snippet that lists published upcoming events.
///
/// No controls are rendered; instead each event should link to the full
/// page for that single event.
async fn snippet_get_upcoming_events(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    let events: Vec<Event> = sqlx::query_as(sql::GET_UPCOMING_EVENTS)
        .bind(chrono::Utc::now())
        .fetch_all(&state.db)
        .await?;
    let template = state
        .templates
        .get_template("events/upcoming_events_snippet")?;
    let rendered = template.render(context! { user_info, events })?;
    Ok(Html(rendered))
}

/// Render a full page of upcoming events.
///
/// Basically what the homepage does, but without the rest of the homepage.
async fn get_upcoming_events(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    let template = state.templates.get_template("events/upcoming_events")?;
    let rendered = template.render(context! { user_info })?;
    Ok(Html(rendered))
}

/// Render the full page for a single event, including controls for signup.
async fn page_get_event(
    State(state): State<Arc<AppState>>,
    session: Session,
    Path(id): Path<u32>,
) -> Result<Response, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    let event: Option<Event> = sqlx::query_as(sql::GET_EVENT)
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    match event {
        Some(event) => {
            let template = state.templates.get_template("events/event")?;
            let rendered = template.render(context! { user_info, event })?;
            Ok(Html(rendered).into_response())
        }
        None => {
            flashed_messages::push_flashed_message(
                session,
                flashed_messages::FlashedMessageLevel::Error,
                "Event not found",
            )
            .await?;
            Ok(Redirect::to("/").into_response())
        }
    }
}

/// This file's routes and templates.
pub fn router(template: &mut Environment) -> Router<Arc<AppState>> {
    template
        .add_template(
            "events/upcoming_events_snippet",
            include_str!("../../templates/events/upcoming_events_snippet.jinja"),
        )
        .unwrap();
    template
        .add_template(
            "events/upcoming_events",
            include_str!("../../templates/events/upcoming_events.jinja"),
        )
        .unwrap();
    template
        .add_template(
            "events/event",
            include_str!("../../templates/events/event.jinja"),
        )
        .unwrap();

    Router::new()
        .route("/events/upcoming", get(snippet_get_upcoming_events))
        .route("/events/", get(get_upcoming_events))
        .route("/events/:id", get(page_get_event))
}
