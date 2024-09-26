//! Endpoints for editing and controlling aspects of the site.

use crate::{
    email::{self, send_mail},
    flashed_messages::{self, MessageLevel},
    shared::{reject_if_not_in, AppError, AppState, UserInfo, SESSION_USER_INFO_KEY},
};
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use log::{info, warn};
use minijinja::{context, Environment};
use rev_buf_reader::RevBufReader;
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, io::BufRead, sync::Arc};
use tower_sessions::Session;
use vzdv::{
    sql::{self, Controller, Feedback, FeedbackForReview, Resource, VisitorRequest},
    vatusa::{self, add_visiting_controller, get_multiple_controller_info},
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
    let pending_feedback: Vec<FeedbackForReview> =
        sqlx::query_as(sql::GET_PENDING_FEEDBACK_FOR_REVIEW)
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
    let user_info = user_info.unwrap();
    let db_feedback: Option<Feedback> = sqlx::query_as(sql::GET_FEEDBACK_BY_ID)
        .bind(feedback_form.id)
        .fetch_optional(&state.db)
        .await?;
    if let Some(feedback) = db_feedback {
        if feedback_form.action == "Archive" {
            sqlx::query(sql::UPDATE_FEEDBACK_TAKE_ACTION)
                .bind(user_info.cid)
                .bind("archive")
                .bind(false)
                .bind(feedback_form.id)
                .execute(&state.db)
                .await?;
            info!("{} archived feedback {}", user_info.cid, feedback.id);
            flashed_messages::push_flashed_message(
                session,
                MessageLevel::Success,
                "Feedback archived",
            )
            .await?;
        } else if feedback_form.action == "Delete" {
            sqlx::query(sql::DELETE_FROM_FEEDBACK)
                .bind(feedback_form.id)
                .execute(&state.db)
                .await?;
            info!(
                "{} deleted {} feedback {} for {} by {}",
                user_info.cid,
                feedback.rating,
                feedback.id,
                feedback.controller,
                feedback.submitter_cid
            );
            flashed_messages::push_flashed_message(
                session,
                MessageLevel::Success,
                "Feedback deleted",
            )
            .await?;
        } else if feedback_form.action == "Post to Discord" {
            let controller: Option<Controller> = sqlx::query_as(sql::GET_CONTROLLER_BY_CID)
                .bind(feedback.controller)
                .fetch_optional(&state.db)
                .await?;
            GENERAL_HTTP_CLIENT
                .post(&state.config.discord.webhooks.feedback)
                .json(&json!({
                    "content": "",
                    "embeds": [{
                        "title": "Feedback received",
                        "fields": [
                            {
                                "name": "Controller",
                                "value": controller.map(|c| format!("{} {}", c.first_name, c.last_name)).unwrap_or_default()
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
            info!(
                "{} submitted feedback {} to Discord",
                user_info.cid, feedback.id
            );
            sqlx::query(sql::UPDATE_FEEDBACK_TAKE_ACTION)
                .bind(user_info.cid)
                .bind("post")
                .bind(true)
                .bind(feedback_form.id)
                .execute(&state.db)
                .await?;
            flashed_messages::push_flashed_message(
                session,
                MessageLevel::Success,
                "Feedback shared",
            )
            .await?;
        }
    } else {
        flashed_messages::push_flashed_message(session, MessageLevel::Error, "Feedback not found")
            .await?;
    }

    Ok(Redirect::to("/admin/feedback").into_response())
}

/// Admin page to manually send emails.
async fn page_email_manual_send(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Response, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if let Some(redirect) = reject_if_not_in(&state, &user_info, PermissionsGroup::Admin).await {
        return Ok(redirect.into_response());
    }
    let all_controllers: Vec<Controller> = sqlx::query_as(sql::GET_ALL_CONTROLLERS)
        .fetch_all(&state.db)
        .await?;
    let template = state.templates.get_template("admin/manual_email")?;
    let rendered = template.render(context! { user_info, all_controllers })?;
    Ok(Html(rendered).into_response())
}

#[derive(Debug, Deserialize)]
struct ManualEmailForm {
    recipient: u32,
    template: String,
}

/// Form submission to manually send an email.
async fn post_email_manual_send(
    State(state): State<Arc<AppState>>,
    session: Session,
    Form(manual_email_form): Form<ManualEmailForm>,
) -> Result<Response, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if let Some(redirect) = reject_if_not_in(&state, &user_info, PermissionsGroup::Admin).await {
        return Ok(redirect.into_response());
    }
    let controller: Option<Controller> = sqlx::query_as(sql::GET_CONTROLLER_BY_CID)
        .bind(manual_email_form.recipient)
        .fetch_optional(&state.db)
        .await?;
    let controller = match controller {
        Some(c) => c,
        None => {
            flashed_messages::push_flashed_message(
                session,
                MessageLevel::Error,
                "Unknown controller",
            )
            .await?;
            return Ok(Redirect::to("/admin/email/manual").into_response());
        }
    };
    let controller_info = vatusa::get_controller_info(
        manual_email_form.recipient,
        Some(&state.config.vatsim.vatusa_api_key),
    )
    .await
    .map_err(|err| AppError::GenericFallback("getting controller info", err))?;
    let email = match controller_info.email {
        Some(e) => e,
        None => {
            flashed_messages::push_flashed_message(
                session,
                MessageLevel::Error,
                "Could not get controller's email from VATUSA",
            )
            .await?;
            return Ok(Redirect::to("/admin/email/manual").into_response());
        }
    };
    send_mail(
        &state.config,
        &state.db,
        &format!("{} {}", controller.first_name, controller.last_name),
        &email,
        &manual_email_form.template,
    )
    .await?;
    flashed_messages::push_flashed_message(session, MessageLevel::Info, "Email sent").await?;
    Ok(Redirect::to("/admin/email/manual").into_response())
}

/// Page for logs.
///
/// Read the last hundred lines from each of the log files
/// and show them in the page.
async fn page_logs(
    State(state): State<Arc<AppState>>,
    session: Session,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if let Some(redirect) = reject_if_not_in(&state, &user_info, PermissionsGroup::Admin).await {
        return Ok(redirect.into_response());
    }
    let line_count: u64 = match params.get("lines") {
        Some(n) => match n.parse() {
            Ok(n) => n,
            Err(_) => {
                warn!("Error parsing 'lines' query param on logs page");
                100
            }
        },
        None => 100,
    };

    let file_names = ["vzdv_site.log", "vzdv_tasks.log", "vzdv_bot.log"];
    let mut logs: HashMap<&str, String> = HashMap::new();
    for name in file_names {
        let mut buffer = Vec::new();
        let file = std::fs::File::open(name).unwrap();
        let reader = RevBufReader::new(file);
        let mut by_line = reader.lines();
        for _ in 0..line_count {
            if let Some(line) = by_line.next() {
                let line = line.unwrap();
                buffer.push(line);
            } else {
                break;
            }
        }
        buffer.reverse();
        logs.insert(name, buffer.join("<br>"));
    }

    let template = state.templates.get_template("admin/logs")?;
    let rendered = template.render(context! { user_info, logs, line_count })?;
    Ok(Html(rendered).into_response())
}

/// Page for managing visitor applications.
async fn page_visitor_applications(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Response, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if let Some(redirect) = reject_if_not_in(&state, &user_info, PermissionsGroup::Admin).await {
        return Ok(redirect.into_response());
    }
    let requests: Vec<VisitorRequest> = sqlx::query_as(sql::GET_ALL_VISITOR_REQUESTS)
        .fetch_all(&state.db)
        .await?;
    let request_cids: Vec<_> = requests.iter().map(|request| request.cid).collect();
    let controller_info = get_multiple_controller_info(&request_cids).await;
    let already_visiting = request_cids.iter().fold(HashMap::new(), |mut map, cid| {
        let info = controller_info.iter().find(|&info| info.cid == *cid);
        if let Some(info) = info {
            let already_visiting: Vec<String> = info
                .visiting_facilities
                .as_ref()
                .map(|visiting| {
                    visiting
                        .iter()
                        .map(|visit| visit.facility.to_owned())
                        .collect()
                })
                .unwrap_or_default();
            map.insert(cid, already_visiting.join(", "));
        }
        map
    });

    let flashed_messages = flashed_messages::drain_flashed_messages(session).await?;
    let template = state.templates.get_template("admin/visitor_applications")?;
    let rendered = template.render(context! {
        user_info,
        flashed_messages,
        requests,
        already_visiting,
    })?;
    Ok(Html(rendered).into_response())
}

#[derive(Deserialize)]
struct VisitorApplicationActionForm {
    action: String,
}

async fn post_visitor_application_action(
    State(state): State<Arc<AppState>>,
    session: Session,
    Path(id): Path<u32>,
    Form(action_form): Form<VisitorApplicationActionForm>,
) -> Result<Redirect, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if let Some(redirect) = reject_if_not_in(&state, &user_info, PermissionsGroup::Admin).await {
        return Ok(redirect);
    }
    let user_info = user_info.unwrap();
    let request: Option<VisitorRequest> = sqlx::query_as(sql::GET_VISITOR_REQUEST_BY_ID)
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    let request = match request {
        Some(r) => r,
        None => {
            flashed_messages::push_flashed_message(
                session,
                MessageLevel::Error,
                "Visitor application not found",
            )
            .await?;
            return Ok(Redirect::to("/admin/visitor_applications"));
        }
    };
    let controller_info =
        vatusa::get_controller_info(request.cid, Some(&state.config.vatsim.vatusa_api_key))
            .await
            .map_err(|err| AppError::GenericFallback("getting controller info", err))?;
    info!(
        "{} taking action {} on visitor request {id}",
        user_info.cid, action_form.action
    );

    if action_form.action == "accept" {
        // add to roster
        add_visiting_controller(request.cid, &state.config.vatsim.vatusa_api_key)
            .await
            .map_err(|err| AppError::GenericFallback("could not add visitor", err))?;

        // inform if possible
        if let Some(email_address) = controller_info.email {
            send_mail(
                &state.config,
                &state.db,
                &format!("{} {}", request.first_name, request.last_name),
                &email_address,
                email::templates::VISITOR_ACCEPTED,
            )
            .await?;
            flashed_messages::push_flashed_message(
                session,
                MessageLevel::Success,
                "Visitor request accepted and the controller was emailed of the decision.",
            )
            .await?;
        } else {
            warn!("No email address found for {}", request.cid);
            flashed_messages::push_flashed_message(
                session,
                MessageLevel::Success,
                "Visitor request accepted, but their email could not be determined so no email was sent.",
            )
            .await?;
        }
    } else if action_form.action == "deny" {
        // inform if possible
        if let Some(email_address) = controller_info.email {
            send_mail(
                &state.config,
                &state.db,
                &format!("{} {}", request.first_name, request.last_name),
                &email_address,
                email::templates::VISITOR_DENIED,
            )
            .await?;
            flashed_messages::push_flashed_message(
                session,
                MessageLevel::Success,
                "Visitor request denied and the controller was emailed of the decision.",
            )
            .await?;
        } else {
            warn!("No email address found for {}", request.cid);
            flashed_messages::push_flashed_message(
                session,
                MessageLevel::Success,
                "Visitor request denied, but their email could not be determined so no email was sent.",
            )
            .await?;
        }
    }

    // delete the request
    sqlx::query(sql::DELETE_VISITOR_REQUEST)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Redirect::to("/admin/visitor_applications"))
}

/// Page for managing the site's resource documents and links.
async fn page_resources(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Response, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if let Some(redirect) =
        reject_if_not_in(&state, &user_info, PermissionsGroup::NamedPosition).await
    {
        return Ok(redirect.into_response());
    }
    let resources: Vec<Resource> = sqlx::query_as(sql::GET_ALL_RESOURCES)
        .fetch_all(&state.db)
        .await?;
    let flashed_messages = flashed_messages::drain_flashed_messages(session).await?;
    let template = state.templates.get_template("admin/resources")?;
    let rendered = template.render(context! { user_info, flashed_messages, resources })?;
    Ok(Html(rendered).into_response())
}

/// This file's routes and templates.
pub fn router(templates: &mut Environment) -> Router<Arc<AppState>> {
    templates
        .add_template(
            "admin/feedback",
            include_str!("../../templates/admin/feedback.jinja"),
        )
        .unwrap();
    templates
        .add_template(
            "admin/manual_email",
            include_str!("../../templates/admin/manual_email.jinja"),
        )
        .unwrap();
    templates
        .add_template(
            "admin/logs",
            include_str!("../../templates/admin/logs.jinja"),
        )
        .unwrap();
    templates
        .add_template(
            "admin/visitor_applications",
            include_str!("../../templates/admin/visitor_applications.jinja"),
        )
        .unwrap();
    templates
        .add_template(
            "admin/resources",
            include_str!("../../templates/admin/resources.jinja"),
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
        .route(
            "/admin/email/manual",
            get(page_email_manual_send).post(post_email_manual_send),
        )
        .route("/admin/logs", get(page_logs))
        .route(
            "/admin/visitor_applications",
            get(page_visitor_applications),
        )
        .route(
            "/admin/visitor_applications/:id",
            get(post_visitor_application_action),
        )
        .route("/admin/resources", get(page_resources))
}
