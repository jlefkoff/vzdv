//! HTTP endpoints for user-specific pages.

use crate::{
    discord, flashed_messages,
    shared::{AppError, AppState, UserInfo, SESSION_USER_INFO_KEY},
};
use axum::{
    extract::{Query, State},
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
            .map_err(|e| AppError::GenericFallback("getting VATUSA training records", e))?;
    for record in &mut training_records {
        record.notes = record.notes._strip_tags();
    }
    let template = state.templates.get_template("user/training_notes")?;
    let rendered = template.render(context! { user_info, training_records })?;
    Ok(Html(rendered).into_response())
}

/// Allow the user to request training
async fn page_request_training(
  State(state): State<Arc<AppState>>,
  session: Session,
) -> Result<Response, AppError> {
  let training_types = &state.config.training.training_types;
  let flashed_messages = flashed_messages::drain_flashed_messages(session).await?;
  let template = state.templates.get_template("user/request_training")?;
  let rendered =
      template.render(context! { flashed_messages, training_types })?;
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
            .bind(user_info.cid)
            .bind(&discord_user_id)
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
          "user/request_training",
          include_str!("../../templates/user/request_training.jinja"),
      )
      .unwrap();

    Router::new()
        .route("/user/training_notes", get(page_training_notes))
        .route("/user/discord", get(page_discord))
        .route("/user/discord/callback", get(page_discord_callback))
        .route("/user/request_training", get(page_request_training))
}
