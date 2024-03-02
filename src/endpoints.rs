//! HTTP endpoints.

use crate::{
    shared::{AppError, AppState, CacheEntry, UserInfo, SESSION_USER_INFO_KEY},
    utils::{
        auth::{code_to_user_info, get_user_info, AuthCallback},
        parse_vatsim_timestamp,
    },
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

pub async fn handler_home(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, StatusCode> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await.unwrap();
    let template = state.templates.get_template("home").unwrap();
    let rendered = template.render(context! { user_info }).unwrap();
    Ok(Html(rendered))
}

pub async fn handler_404(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, StatusCode> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await.unwrap();
    let template = state.templates.get_template("404").unwrap();
    let rendered = template.render(context! { user_info }).unwrap();
    Ok(Html(rendered))
}

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

pub async fn snippet_online_controllers(
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, AppError> {
    #[derive(Serialize)]
    struct OnlineController {
        cid: u64,
        name: String,
        online_for: String,
    }

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

    let now = chrono::Utc::now();
    let data = Vatsim::new().await?.get_v3_data().await?;
    let online: Vec<_> = data
        .controllers
        .iter()
        .filter(|controller| {
            let prefix_match = state
                .config
                .stats
                .position_prefixes
                .iter()
                .any(|prefix| controller.callsign.starts_with(prefix));
            if !prefix_match {
                return false;
            }
            state
                .config
                .stats
                .position_suffixes
                .iter()
                .any(|suffix| controller.callsign.ends_with(suffix))
        })
        .map(|controller| {
            let logon = parse_vatsim_timestamp(&controller.logon_time).unwrap();
            let seconds = (now - logon).num_seconds() as u32;
            OnlineController {
                cid: controller.cid,
                name: controller.name.clone(),
                online_for: format!("{}h{}m", seconds / 3600, (seconds / 60) % 60),
            }
        })
        .collect();

    let template = state.templates.get_template("snippet_online_controllers")?;
    let rendered = template.render(context! { online })?;
    state
        .cache
        .insert(cache_key, CacheEntry::new(rendered.clone()));
    Ok(Html(rendered))
}
