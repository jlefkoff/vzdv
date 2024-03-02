//! HTTP endpoints.

use crate::{
    shared::{AppError, AppState, CacheEntry, UserInfo, SESSION_USER_INFO_KEY},
    utils::auth::{code_to_user_info, get_user_info, AuthCallback},
};
use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use log::debug;
use minijinja::context;
use serde::Serialize;
use std::{sync::Arc, time::Instant};
use tower_sessions::Session;
use vatsim_utils::live_api::Vatsim;

/// Define a simple endpoint that returns a rendered template
/// with the standard context data.
macro_rules! simple {
    (
        $fn_name:ident,
        $template_name:literal
    ) => {
        pub async fn $fn_name(
            State(state): State<Arc<AppState>>,
            session: Session,
        ) -> Result<Html<String>, StatusCode> {
            let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await.unwrap();
            let template = state.templates.get_template($template_name).unwrap();
            let rendered = template.render(context! { user_info }).unwrap();
            Ok(Html(rendered))
        }
    };
}

simple!(handler_404, "404");
simple!(handler_home, "home");

pub async fn page_auth_login(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Redirect, StatusCode> {
    // if already logged in, just redirect to homepage
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await.unwrap();
    if user_info.is_some() {
        debug!("Already logged-in user hit login page");
        return Ok(Redirect::to("/"));
    }
    // build url and redirect to VATSIM OAuth URL
    let redirect_url = format!(
        "https://auth-dev.vatsim.net/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}",
        state.config.vatsim.oauth_client_id,
        state.config.vatsim.oauth_client_calback_url,
        "full_name email vatsim_details"
    );
    Ok(Redirect::to(&redirect_url))
}

pub async fn page_auth_callback(
    query: Query<AuthCallback>,
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, AppError> {
    let token_data = code_to_user_info(&query.code, &state).await?;
    let user_info = get_user_info(&token_data.access_token).await?;

    let to_session = UserInfo {
        cid: user_info.data.cid.parse()?,
        first_name: user_info.data.personal.name_first,
        last_name: user_info.data.personal.name_last,
    };
    session
        .insert(SESSION_USER_INFO_KEY, to_session.clone())
        .await?;
    // TODO update DB with user info
    debug!("Completed log in for {}", user_info.data.cid);
    let template = state.templates.get_template("login_complete")?;
    let rendered = template.render(context! { user_info => to_session })?;
    Ok(Html(rendered))
}

pub async fn page_auth_logout(session: Session) -> Result<Redirect, AppError> {
    session.delete().await?;
    Ok(Redirect::to("/"))
}

#[derive(Serialize)]
struct OnlineController {
    cid: u64,
    name: String,
}

pub async fn snippet_online_controllers(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, AppError> {
    let cache_key = "ONLINE_CONTROLLERS";
    // cache data for 60 seconds
    if let Some(cached) = state.cache.get(&cache_key) {
        let elapsed = Instant::now() - cached.inserted;
        if elapsed.as_secs() < 60 {
            return Ok(Html(cached.data));
        }
        debug!("Cache timeout on online controllers");
        state.cache.invalidate(&cache_key);
    }

    let vatsim = Vatsim::new().await?;
    let data = vatsim.get_v3_data().await?;
    let mut online: Vec<OnlineController> = Vec::new();

    for controller in data.controllers {
        let prefix_match = state
            .config
            .stats
            .position_prefixes
            .iter()
            .any(|prefix| controller.callsign.starts_with(prefix));
        if !prefix_match {
            continue;
        }
        let suffix_match = state
            .config
            .stats
            .position_suffixes
            .iter()
            .any(|suffix| controller.callsign.ends_with(suffix));
        if !suffix_match {
            continue;
        }
        online.push(OnlineController {
            cid: controller.cid,
            name: controller.name,
        });
    }

    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await.unwrap();
    let template = state.templates.get_template("snippet_online_controllers")?;
    let rendered = template.render(context! { user_info , online })?;
    state
        .cache
        .insert(cache_key, CacheEntry::new(rendered.clone()));
    Ok(Html(rendered))
}
