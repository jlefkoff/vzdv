use crate::{
    shared::{sql::INSERT_FEEDBACK, AppError, AppState, UserInfo, SESSION_USER_INFO_KEY},
    utils::flashed_messages,
};
use axum::{
    extract::State,
    response::{Html, Redirect},
    routing::{get, post},
    Form, Router,
};
use minijinja::{context, Environment};
use serde::Deserialize;
use std::sync::Arc;
use tower_sessions::Session;

/// View the feedback form.
///
/// The template handles requiring the user to be logged in.
async fn page_feedback_form(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await.unwrap();
    let flashed_messages = flashed_messages::drain_flashed_messages(session).await?;
    let template = state.templates.get_template("pilot_feedback").unwrap();
    let rendered = template
        .render(context! { user_info, flashed_messages })
        .unwrap();
    Ok(Html(rendered))
}

#[derive(Debug, Deserialize)]
struct FeedbackForm {
    controller: String,
    position: String,
    rating: String,
    comments: String,
}

/// Submit the feedback form.
async fn post_feedback_form(
    State(state): State<Arc<AppState>>,
    session: Session,
    Form(feedback): Form<FeedbackForm>,
) -> Result<Redirect, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await.unwrap();
    if let Some(user_info) = user_info {
        sqlx::query(INSERT_FEEDBACK)
            .bind(feedback.controller)
            .bind(feedback.position)
            .bind(feedback.rating)
            .bind(feedback.comments)
            .bind(sqlx::types::chrono::Utc::now())
            .bind(user_info.cid)
            .execute(&state.db)
            .await?;
        flashed_messages::push_flashed_message(
            session,
            flashed_messages::FlashedMessageLevel::Success,
            "Feedback submitted, thank you!",
        )
        .await?;
    } else {
        flashed_messages::push_flashed_message(
            session,
            flashed_messages::FlashedMessageLevel::Error,
            "You must be logged in to submit feedback.",
        )
        .await?;
    }

    Ok(Redirect::to("/pilots/feedback"))
}

/// This file's routes and templates.
pub fn router(templates: &mut Environment) -> Router<Arc<AppState>> {
    templates
        .add_template(
            "pilot_feedback",
            include_str!("../../templates/pilot_feedback.jinja"),
        )
        .unwrap();

    Router::new()
        .route("/pilots/feedback", get(page_feedback_form))
        .route("/pilots/feedback", post(post_feedback_form))
}
