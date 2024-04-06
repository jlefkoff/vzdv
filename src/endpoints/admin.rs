use crate::{
    shared::{
        sql::{self, Controller, Feedback},
        AppError, AppState, UserInfo, SESSION_USER_INFO_KEY,
    },
    utils::{flashed_messages, GENERAL_HTTP_CLIENT},
};
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use log::{error, warn};
use minijinja::{context, Environment};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tower_sessions::Session;

/// Access control by staff position.
///
/// ## Limitations
///
/// - Mentors, Instructors, TA, ATM, DATM (+ WM) can CRUD training notes, ratings, and certs.
/// - TA, ATM, DATM (+ WM) can view and take action on feedback
/// - TA, ATM, DATM (+ WM) can view and take action on visitor applications
/// - EC, AEC, ATM, DATM (+ WM) can CRUD events
///
/// ## Unused roles
///
/// FE, AFE, and AWM are not granted any special access.
///
#[allow(unused)]
enum StaffRequirement {
    /// Training staff (Mentors, Instructors, TA) and admins (ATM, DATM, WM)
    TrainingStaff,
    /// Events staff (EC, AEC) and admins (ATM, DATM, WM)
    EventStaff,
    /// Any senior staff position (ATM, DATM, TA) plus WM
    SeniorStaff,
    /// Just the ATM and DATM plus WM
    Admins,
}

impl StaffRequirement {
    /// Return a list of matching roles to satisfy the requirement.
    ///
    /// While the WM is not, by default, part of any of these groups,
    /// their role satisfies all requirements.
    fn matching_roles(&self) -> Vec<&'static str> {
        match self {
            Self::TrainingStaff => vec!["ATM", "DATM", "TA", "MTR", "INS", "WM"],
            Self::EventStaff => vec!["ATM", "DATM", "EC", "AEC", "WM"],
            Self::SeniorStaff => vec!["ATM", "DATM", "TA", "WM"],
            Self::Admins => vec!["ATM", "DATM", "WM"],
        }
    }
}

/// Returns a response to redirect to the homepage for non-staff users.
///
/// Also asserts that `user_info.is_some()`, so later unwrapping it is safe.
async fn reject_if_not_staff(
    state: &Arc<AppState>,
    user_info: &Option<UserInfo>,
    staff_type: StaffRequirement,
) -> Option<Response> {
    let resp = Some(Redirect::to("/").into_response());
    if user_info.is_none() {
        return resp;
    }
    let user_info = user_info.as_ref().unwrap();
    if !user_info.is_staff {
        return resp;
    }
    let controller: Option<Controller> = match sqlx::query_as(sql::GET_CONTROLLER_BY_CID)
        .bind(user_info.cid)
        .fetch_optional(&state.db)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            error!(
                "Could not look up staff controller with CID {}: {e}",
                user_info.cid
            );
            return resp;
        }
    };
    let controller = match controller {
        Some(c) => c,
        None => {
            warn!(
                "No located controller by CID {} for staff check",
                user_info.cid
            );
            return resp;
        }
    };
    if controller.roles.is_empty() {
        return resp;
    }
    let satisfied = controller
        .roles
        .split_terminator(' ')
        .any(|role| staff_type.matching_roles().contains(&role));
    if satisfied {
        None
    } else {
        resp
    }
}

/// Page for managing controller feedback.
///
/// Feedback must be reviewed by staff before being posted to Discord.
async fn page_feedback(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Response, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if let Some(redirect) =
        reject_if_not_staff(&state, &user_info, StaffRequirement::SeniorStaff).await
    {
        return Ok(redirect);
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
        reject_if_not_staff(&state, &user_info, StaffRequirement::TrainingStaff).await
    {
        return Ok(redirect);
    }
    let db_feedback: Option<Feedback> = sqlx::query_as(sql::GET_FEEDBACK_BY_ID)
        .bind(feedback_form.id)
        .fetch_optional(&state.db)
        .await?;
    if let Some(feedback) = db_feedback {
        if feedback_form.action == "Ignore" {
            sqlx::query(sql::UPDATE_FEEDBACK_IGNORE)
                .bind(user_info.unwrap().cid)
                .bind("ignore")
                .bind(false)
                .execute(&state.db)
                .await?;
            flashed_messages::push_flashed_message(
                session,
                flashed_messages::FlashedMessageLevel::Success,
                "Feedback ignored",
            )
            .await?;
        } else {
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
            sqlx::query(sql::UPDATE_FEEDBACK_IGNORE)
                .bind(user_info.unwrap().cid)
                .bind("post")
                .bind(true)
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
 *  - set controller rank
 *  - add to / remove from the roster
 *  - add / remove certifications
 *  - add / remove staff ranks (incl. mentor and assoc. positions)
 *  - add training note (unless I'm sending users to VATUSA here)
 */

// TODO allow managing the roster

// TODO allow creating and modifying events

// TODO allow managing visitor requests

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
