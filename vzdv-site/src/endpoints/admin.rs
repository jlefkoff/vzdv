//! Endpoints for editing and controlling aspects of the site.

use crate::{
    flashed_messages,
    shared::{reject_if_not_in, AppError, AppState, UserInfo, SESSION_USER_INFO_KEY},
};
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use minijinja::{context, Environment};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tower_sessions::Session;
use vzdv::{
    sql::{self, Feedback},
    PermissionsGroup, GENERAL_HTTP_CLIENT,
};

/// Page for managing controller feedback.
///
/// Feedback must be reviewed by staff before being posted to Discord.
async fn page_feedback(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Response, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if let Some(redirect) = reject_if_not_in(&state, &user_info, PermissionsGroup::Admin).await {
        return Ok(redirect.into_response());
    }
    let template = state.templates.get_template("admin/feedback")?;
    let pending_feedback: Vec<Feedback> = sqlx::query_as(sql::GET_ALL_PENDING_FEEDBACK)
        .fetch_all(&state.db)
        .await?;
    let flashed_messages = flashed_messages::drain_flashed_messages(session).await?;
    let rendered = template.render(context! {
        user_info,
        flashed_messages,
        pending_feedback,
    })?;
    Ok(Html(rendered).into_response())
}

#[derive(Debug, Deserialize)]
struct FeedbackReviewForm {
    id: u32,
    action: String,
}

/// Handler for staff members taking action on feedback.
async fn post_feedback_form_handle(
    State(state): State<Arc<AppState>>,
    session: Session,
    Form(feedback_form): Form<FeedbackReviewForm>,
) -> Result<Response, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if let Some(redirect) =
        reject_if_not_in(&state, &user_info, PermissionsGroup::TrainingTeam).await
    {
        return Ok(redirect.into_response());
    }
    let db_feedback: Option<Feedback> = sqlx::query_as(sql::GET_FEEDBACK_BY_ID)
        .bind(feedback_form.id)
        .fetch_optional(&state.db)
        .await?;
    if let Some(feedback) = db_feedback {
        if feedback_form.action == "Archive" {
            sqlx::query(sql::UPDATE_FEEDBACK_TAKE_ACTION)
                .bind(user_info.unwrap().cid)
                .bind("archive")
                .bind(false)
                .bind(feedback_form.id)
                .execute(&state.db)
                .await?;
            flashed_messages::push_flashed_message(
                session,
                flashed_messages::FlashedMessageLevel::Success,
                "Feedback archived",
            )
            .await?;
        } else if feedback_form.action == "Delete" {
            sqlx::query(sql::DELETE_FROM_FEEDBACK)
                .bind(feedback_form.id)
                .execute(&state.db)
                .await?;
            flashed_messages::push_flashed_message(
                session,
                flashed_messages::FlashedMessageLevel::Success,
                "Feedback deleted",
            )
            .await?;
        } else if feedback_form.action == "Post to Discord" {
            GENERAL_HTTP_CLIENT
                .post(&state.config.discord.webhooks.feedback)
                .json(&json!({
                    "content": "",
                    "embeds": [{
                        "title": "Feedback received",
                        "fields": [
                            {
                                "name": "Controller",
                                "value": feedback.controller
                            },
                            {
                                "name": "Position",
                                "value": feedback.position
                            },
                            {
                                "name": "Rating",
                                "value": feedback.rating
                            },
                            {
                                "name": "Comments",
                                "value": feedback.comments
                            }
                        ]
                    }]
                }))
                .send()
                .await?;
            sqlx::query(sql::UPDATE_FEEDBACK_TAKE_ACTION)
                .bind(user_info.unwrap().cid)
                .bind("post")
                .bind(true)
                .bind(feedback_form.id)
                .execute(&state.db)
                .await?;
            flashed_messages::push_flashed_message(
                session,
                flashed_messages::FlashedMessageLevel::Success,
                "Feedback shared",
            )
            .await?;
        }
    } else {
        flashed_messages::push_flashed_message(
            session,
            flashed_messages::FlashedMessageLevel::Error,
            "Feedback not found",
        )
        .await?;
    }

    Ok(Redirect::to("/admin/feedback").into_response())
}

/**
 * TODO manage a controller
 *
 * Things to do:
 *  - set controller rating
 *  - add to / remove from the roster
 *  - add / remove certifications
 *  - add / remove staff ranks (incl. mentor and assoc. positions)
 *  - add training note (do it on this site, then post to VATUSA)
 *
 * TODO allow managing the roster
 * TODO allow creating and modifying events
 * TODO allow managing visitor requests
 * TODO allow running reports on the roster to find controllers who aren't
 *      meeting specified activity requirements
 */

/// This file's routes and templates.
pub fn router(templates: &mut Environment) -> Router<Arc<AppState>> {
    templates
        .add_template(
            "admin/feedback",
            include_str!("../../templates/admin/feedback.jinja"),
        )
        .unwrap();
    templates.add_filter("nice_date", |date: String| {
        chrono::DateTime::parse_from_rfc3339(&date)
            .unwrap()
            .format("%m/%d/%Y %H:%M:%S")
            .to_string()
    });

    Router::new()
        .route("/admin/feedback", get(page_feedback))
        .route("/admin/feedback", post(post_feedback_form_handle))
    // .route("/admin/roster/:cid", get(page_controller))
}
