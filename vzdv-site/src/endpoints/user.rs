//! HTTP endpoints for user-specific pages.

use crate::{
    discord, flashed_messages,
    shared::{AppError, AppState, UserInfo, SESSION_USER_INFO_KEY},
};
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use log::{debug, info, warn};
use minijinja::{context, Environment};
use std::{collections::HashMap, sync::Arc};
use tower_sessions::Session;
use vzdv::{
    sql::{self, Controller},
    vatusa,
};

/// Retrieve and show the user their training records from VATUSA.
async fn page_training_notes(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Response, AppError> {
    use voca_rs::Voca;

    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    let user_info = match user_info {
        Some(info) => info,
        None => return Ok(Redirect::to("/").into_response()),
    };
    let mut training_records =
        vatusa::get_training_records(&state.config.vatsim.vatusa_api_key, user_info.cid)
            .await
            .map_err(|error| AppError::GenericFallback("getting training records", error))?;
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
    let user_info = match user_info {
        Some(info) => info,
        None => return Ok(Redirect::to("/").into_response()),
    };
    let controller: Controller = sqlx::query_as(sql::GET_CONTROLLER_BY_CID)
        .bind(user_info.cid)
        .fetch_one(&state.db)
        .await?;
    let template = state.templates.get_template("user/discord")?;
    let flashed_messages = flashed_messages::drain_flashed_messages(session).await?;
    let rendered: String = template.render(context! {
        user_info,
        oauth_link => discord::get_oauth_link(&state.config),
        join_link => &state.config.discord.join_link,
        discord_id => controller.discord_id,
        flashed_messages
    })?;
    Ok(Html(rendered).into_response())
}

/// Navigation from the Discord OAuth flow.
async fn page_discord_callback(
    State(state): State<Arc<AppState>>,
    session: Session,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Redirect, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    let user_info = match user_info {
        Some(info) => info,
        None => {
            warn!("Unknown user hit Discord link callback page");
            flashed_messages::push_flashed_message(
                session,
                flashed_messages::MessageLevel::Error,
                "Not logged in",
            )
            .await?;
            return Ok(Redirect::to("/"));
        }
    };
    if let Some(code) = params.get("code") {
        debug!("Getting Discord info in callback");
        let access_token = discord::code_to_token(code, &state.config).await?;
        let discord_user_id = discord::get_token_user_id(&access_token).await?;
        sqlx::query(sql::SET_CONTROLLER_DISCORD_ID)
            .bind(&discord_user_id)
            .bind(user_info.cid)
            .execute(&state.db)
            .await?;
        flashed_messages::push_flashed_message(
            session,
            flashed_messages::MessageLevel::Info,
            "Discord account linked",
        )
        .await?;
        info!(
            "Set Discord ID for controller {} to {}",
            user_info.cid, discord_user_id
        );
    } else {
        warn!(
            "Discord callback page hit by {} without code param",
            user_info.cid
        );
        flashed_messages::push_flashed_message(
            session,
            flashed_messages::MessageLevel::Error,
            "Could not link your Discord account - not enough info provided",
        )
        .await?;
    }
    Ok(Redirect::to("/user/discord"))
}

/// Overview page for a user.
///
/// Shows additional information and controls for training staff.
async fn page_user(
    State(state): State<Arc<AppState>>,
    session: Session,
    Path(cid): Path<u32>,
) -> Result<Response, AppError> {
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
    let template = state.templates.get_template("user/overview")?;
    let rendered: String = template.render(context! {
        user_info,
        controller
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
    templates
        .add_template(
            "user/overview",
            include_str!("../../templates/user/overview.jinja"),
        )
        .unwrap();

    Router::new()
        .route("/user/training_notes", get(page_training_notes))
        .route("/user/discord", get(page_discord))
        .route("/user/discord/callback", get(page_discord_callback))
        .route("/user/:cid", get(page_user))
}
