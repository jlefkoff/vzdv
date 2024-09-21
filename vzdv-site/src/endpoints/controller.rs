//! HTTP endpoints for controller pages.

use crate::{
    flashed_messages::{self, MessageLevel},
    shared::{reject_if_not_in, AppError, AppState, UserInfo, SESSION_USER_INFO_KEY},
};
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use chrono::Utc;
use log::info;
use minijinja::{context, Environment};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tower_sessions::Session;
use vzdv::{
    retrieve_all_in_use_ois,
    sql::{self, Certification, Controller},
    ControllerRating, PermissionsGroup,
};

/// Overview page for a user.
///
/// Shows additional information and controls for training staff.
async fn page_controller(
    State(state): State<Arc<AppState>>,
    session: Session,
    Path(cid): Path<u32>,
) -> Result<Response, AppError> {
    #[derive(Debug, Serialize)]
    struct CertNameValue<'a> {
        name: &'a str,
        value: &'a str,
    }

    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    let controller: Option<Controller> = sqlx::query_as(sql::GET_CONTROLLER_BY_CID)
        .bind(cid)
        .fetch_optional(&state.db)
        .await?;
    let controller = match controller {
        Some(c) => c,
        None => {
            flashed_messages::push_flashed_message(
                session,
                flashed_messages::MessageLevel::Error,
                "Controller not found",
            )
            .await?;
            return Ok(Redirect::to("/facility/roster").into_response());
        }
    };
    let rating_str = ControllerRating::try_from(controller.rating)
        .map_err(|err| AppError::GenericFallback("unknown controller rating", err))?
        .as_str();

    let db_certs: Vec<Certification> = sqlx::query_as(sql::GET_ALL_CERTIFICATIONS_FOR)
        .bind(cid)
        .fetch_all(&state.db)
        .await?;
    let mut certifications: Vec<CertNameValue> =
        Vec::with_capacity(state.config.training.certifications.len());
    let none = String::from("None");
    for name in &state.config.training.certifications {
        let db_match = db_certs.iter().find(|cert| &cert.name == name);
        let value: &str = match db_match {
            Some(row) => &row.value,
            None => &none,
        };
        certifications.push(CertNameValue { name, value });
    }

    let flashed_messages = flashed_messages::drain_flashed_messages(session).await?;
    let template = state.templates.get_template("controller/controller")?;
    let rendered: String = template.render(context! {
        user_info,
        controller,
        rating_str,
        certifications,
        flashed_messages
    })?;
    Ok(Html(rendered).into_response())
}

/// API endpoint to unlink a controller's Discord account.
async fn api_unlink_discord(
    State(state): State<Arc<AppState>>,
    session: Session,
    Path(cid): Path<u32>,
) -> Result<Redirect, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if let Some(redirect) = reject_if_not_in(&state, &user_info, PermissionsGroup::Admin).await {
        return Ok(redirect);
    }
    sqlx::query(sql::UNSET_CONTROLLER_DISCORD_ID)
        .bind(cid)
        .execute(&state.db)
        .await?;
    flashed_messages::push_flashed_message(session, MessageLevel::Info, "Discord unlinked").await?;
    info!(
        "{} unlinked Discord account from {cid}",
        user_info.unwrap().cid
    );
    Ok(Redirect::to(&format!("/controllers/{cid}")))
}

#[derive(Deserialize)]
struct ChangeInitialsForm {
    initials: String,
}

/// Form submission to set a controller's operating initials.
async fn post_change_ois(
    State(state): State<Arc<AppState>>,
    session: Session,
    Path(cid): Path<u32>,
    Form(initials_form): Form<ChangeInitialsForm>,
) -> Result<Redirect, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if let Some(redirect) = reject_if_not_in(&state, &user_info, PermissionsGroup::Admin).await {
        return Ok(redirect);
    }
    let initials = initials_form.initials.to_uppercase();

    // assert unique
    if !initials.is_empty() {
        let in_use = retrieve_all_in_use_ois(&state.db).await.map_err(|err| {
            AppError::GenericFallback("Error accessing DB to get existing OIs", err)
        })?;
        if in_use.contains(&initials) {
            flashed_messages::push_flashed_message(
                session,
                MessageLevel::Error,
                "Those OIs are already in use",
            )
            .await?;
            return Ok(Redirect::to(&format!("/controller/{cid}")));
        }
    }

    // update
    sqlx::query(sql::UPDATE_CONTROLLER_OIS)
        .bind(cid)
        .bind(&initials)
        .execute(&state.db)
        .await?;
    flashed_messages::push_flashed_message(
        session,
        MessageLevel::Info,
        "Operating initials updated",
    )
    .await?;
    info!(
        "{} updated OIs for {cid} to: '{initials}'",
        user_info.unwrap().cid,
    );
    Ok(Redirect::to(&format!("/controller/{cid}")))
}

/// Form submission to set the controller's certifications.
///
/// Not used to set their network rating; that process is handled
/// through VATUSA/VATSIM.
async fn post_change_certs(
    State(state): State<Arc<AppState>>,
    session: Session,
    Path(cid): Path<u32>,
    Form(certs_form): Form<HashMap<String, String>>,
) -> Result<Redirect, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    if let Some(redirect) =
        reject_if_not_in(&state, &user_info, PermissionsGroup::TrainingTeam).await
    {
        return Ok(redirect);
    }

    let by_cid = user_info.unwrap().cid;
    let db_certs: Vec<Certification> = sqlx::query_as(sql::GET_ALL_CERTIFICATIONS_FOR)
        .bind(cid)
        .fetch_all(&state.db)
        .await?;
    for (key, value) in &certs_form {
        let existing = db_certs.iter().find(|c| &c.name == key);
        match existing {
            Some(existing) => {
                sqlx::query(sql::UPDATE_CERTIFICATION)
                    .bind(existing.id)
                    .bind(value)
                    .bind(Utc::now())
                    .bind(by_cid)
                    .execute(&state.db)
                    .await?;
                info!("{by_cid} updated cert for {cid} of {key} -> {value}");
            }
            None => {
                sqlx::query(sql::CREATE_CERTIFICATION)
                    .bind(cid)
                    .bind(key)
                    .bind(value)
                    .bind(Utc::now())
                    .bind(by_cid)
                    .execute(&state.db)
                    .await?;
                info!("{by_cid} created new cert for {cid} of {key} -> {value}");
            }
        }
    }

    flashed_messages::push_flashed_message(session, MessageLevel::Info, "Updated certifications")
        .await?;
    Ok(Redirect::to(&format!("/controller/{cid}")))
}

pub fn router(templates: &mut Environment) -> Router<Arc<AppState>> {
    templates
        .add_template(
            "controller/controller",
            include_str!("../../templates/controller/controller.jinja"),
        )
        .unwrap();

    Router::new()
        .route("/controller/:cid", get(page_controller))
        .route("/controller/:cid/discord/unlink", post(api_unlink_discord))
        .route("/controller/:cid/ois", post(post_change_ois))
        .route("/controller/:cid/certs", post(post_change_certs))
}
