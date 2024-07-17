//! Endpoints for viewing and registering for events.
//!
//! The CRUD of events themselves is under /admin routes.

use crate::{
    flashed_messages,
    shared::{reject_if_not_staff, AppError, AppState, UserInfo, SESSION_USER_INFO_KEY},
};
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use minijinja::{context, Environment};
use serde::Serialize;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tower_sessions::Session;
use vzdv::{
    sql::{self, Controller, Event, EventPosition, EventRegistration},
    ControllerRating, PermissionsGroup,
};

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
    // TODO show unpublished events to event staff here

    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    let template = state.templates.get_template("events/upcoming_events")?;
    let rendered = template.render(context! { user_info })?;
    Ok(Html(rendered))
}

// TODO opportunity for some minor speed improvements here by not loading
// controller records twice for each controller assigned to an event.

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
    if let Some(event) = event {
        let positions_raw: Vec<EventPosition> = sqlx::query_as(sql::GET_EVENT_POSITIONS)
            .bind(event.id)
            .fetch_all(&state.db)
            .await?;
        let positions = event_positions_extra(&positions_raw, &state.db).await?;
        let registrations = event_registrations_extra(event.id, &positions_raw, &state.db).await?;
        let not_staff_redirect =
            reject_if_not_staff(&state, &user_info, PermissionsGroup::EventsTeam).await;
        if !event.published {
            // only event staff can see unpublished events
            if let Some(redirect) = not_staff_redirect {
                return Ok(redirect);
            }
        }
        let template = state.templates.get_template("events/event")?;
        let rendered = template.render(context! {
            user_info,
            event,
            positions,
            registrations,
            is_event_staff => not_staff_redirect.is_some()
        })?;
        Ok(Html(rendered).into_response())
    } else {
        flashed_messages::push_flashed_message(
            session,
            flashed_messages::FlashedMessageLevel::Error,
            "Event not found",
        )
        .await?;
        Ok(Redirect::to("/").into_response())
    }
}

#[derive(Serialize)]
struct EventPositionDisplay {
    name: String,
    category: String,
    controller: String,
}

/// Supply event positions with the controller's name, if set.
async fn event_positions_extra(
    positions: &[EventPosition],
    db: &Pool<Sqlite>,
) -> anyhow::Result<Vec<EventPositionDisplay>> {
    let mut ret = Vec::with_capacity(positions.len());
    for position in positions {
        if let Some(pos_cid) = position.cid {
            let controller: Option<Controller> = sqlx::query_as(sql::GET_CONTROLLER_BY_CID)
                .bind(pos_cid)
                .fetch_optional(db)
                .await?;
            if let Some(controller) = controller {
                ret.push(EventPositionDisplay {
                    name: position.name.clone(),
                    category: position.category.clone(),
                    controller: format!(
                        "{} {} ({})",
                        controller.first_name,
                        controller.last_name,
                        controller.operating_initials.unwrap_or_default()
                    ),
                });
                continue;
            }
        }
        ret.push(EventPositionDisplay {
            name: position.name.clone(),
            category: position.category.clone(),
            controller: "unassigned".to_string(),
        });
    }
    Ok(ret)
}

#[derive(Serialize)]
struct EventRegistrationDisplay {
    controller: String,
    choice_1: String,
    choice_2: String,
    choice_3: String,
    notes: String,
}

/// Supply event registration data with controller and position names.
async fn event_registrations_extra(
    event_id: u32,
    positions: &[EventPosition],
    db: &Pool<Sqlite>,
) -> anyhow::Result<Vec<EventRegistrationDisplay>> {
    let registrations: Vec<EventRegistration> = sqlx::query_as(sql::GET_EVENT_REGISTRATIONS)
        .bind(event_id)
        .fetch_all(db)
        .await?;
    let mut ret = Vec::with_capacity(registrations.len());

    for registration in &registrations {
        let c_1 = positions
            .iter()
            .find(|pos| pos.id == registration.choice_1)
            .map(|pos| pos.name.clone());
        let c_2 = positions
            .iter()
            .find(|pos| pos.id == registration.choice_2)
            .map(|pos| pos.name.clone());
        let c_3 = positions
            .iter()
            .find(|pos| pos.id == registration.choice_3)
            .map(|pos| pos.name.clone());
        let controller: Option<Controller> = sqlx::query_as(sql::GET_CONTROLLER_BY_CID)
            .bind(registration.cid)
            .fetch_optional(db)
            .await?;
        let controller = match controller {
            Some(c) => format!(
                "{} {} ({}) - {}",
                c.first_name,
                c.last_name,
                c.operating_initials.unwrap_or_default(),
                ControllerRating::try_from(c.rating)
                    .map(|r| r.as_str())
                    .unwrap_or(""),
            ),
            None => "???".to_string(),
        };
        let notes = match registration.notes.as_ref() {
            Some(s) => s.clone(),
            None => String::new(),
        };
        ret.push(EventRegistrationDisplay {
            controller,
            choice_1: c_1.unwrap_or_default(),
            choice_2: c_2.unwrap_or_default(),
            choice_3: c_3.unwrap_or_default(),
            notes,
        });
    }

    Ok(ret)
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
