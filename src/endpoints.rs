//! HTTP endpoints.

use crate::{
    shared::{AppError, AppState, CacheEntry, UserInfo, SESSION_USER_INFO_KEY},
    utils::{
        auth::{code_to_user_info, get_user_info, oauth_redirect_start, AuthCallback},
        parse_metar, parse_vatsim_timestamp,
    },
};
use anyhow::{anyhow, Result};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use log::{debug, warn};
use minijinja::context;
use serde::Serialize;
use std::{sync::Arc, time::Instant};
use tower_sessions::Session;
use vatsim_utils::live_api::Vatsim;

/// Homepage.
pub async fn handler_home(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, StatusCode> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await.unwrap();
    let template = state.templates.get_template("home").unwrap();
    let rendered = template.render(context! { user_info }).unwrap();
    Ok(Html(rendered))
}

/// 404 not found page.
///
/// Redirected to whenever the router cannot find a valid handler for the requested path.
pub async fn handler_404(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, StatusCode> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await.unwrap();
    let template = state.templates.get_template("404").unwrap();
    let rendered = template.render(context! { user_info }).unwrap();
    Ok(Html(rendered))
}

/// Login page.
///
/// Doesn't actually have a template to render; the user is immediately redirected to
/// either the homepage if they're already logged in, or the VATSIM OAuth page to start
/// their login flow.
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
    let redirect_url = oauth_redirect_start(&state.config);
    Ok(Redirect::to(&redirect_url))
}

/// Auth callback.
///
/// The user is redirected here from VATSIM OAuth providing, in
/// the URL, a code to use in getting an access token for them.
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

/// Clear session and redirect to homepage.
pub async fn page_auth_logout(session: Session) -> Result<Redirect, AppError> {
    // don't need to check if there's something here
    session.delete().await?;
    Ok(Redirect::to("/"))
}

/// Render a list of online controllers.
pub async fn snippet_online_controllers(
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, AppError> {
    #[derive(Serialize)]
    struct OnlineController {
        cid: u64,
        name: String,
        online_for: String,
    }

    // cache this endpoint's returned data for 60 seconds
    let cache_key = "ONLINE_CONTROLLERS";
    if let Some(cached) = state.cache.get(&cache_key) {
        let elapsed = Instant::now() - cached.inserted;
        if elapsed.as_secs() < 60 {
            return Ok(Html(cached.data));
        }
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

pub async fn snippet_weather(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    // cache this endpoint's returned data for 5 minutes
    let cache_key = "WEATHER_BRIEF";
    if let Some(cached) = state.cache.get(&cache_key) {
        let elapsed = Instant::now() - cached.inserted;
        if elapsed.as_secs() < 300 {
            return Ok(Html(cached.data));
        }
        state.cache.invalidate(&cache_key);
    }

    let client = reqwest::ClientBuilder::new()
        .user_agent("github.com/celeo/vzdv")
        .build()?;
    let resp = client
        .get(format!(
            "https://metar.vatsim.net/{}",
            state.config.airports.weather_for.join(",")
        ))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!("Got status {} from METAR API", resp.status().as_u16()).into());
    }
    let text = resp.text().await?;
    let weather: Vec<_> = text
        .split_terminator('\n')
        .flat_map(|line| {
            parse_metar(line).map_err(|e| {
                warn!("Metar parsing failure: {e}");
                e
            })
        })
        .collect();

    let template = state.templates.get_template("snippet_weather")?;
    let rendered = template.render(context! { weather })?;
    state
        .cache
        .insert(cache_key, CacheEntry::new(rendered.clone()));
    Ok(Html(rendered))
}

pub async fn snippet_flights(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    #[derive(Serialize, Default)]
    struct OnlineFlights {
        within: u16,
        from: u16,
        to: u16,
    }

    // cache this endpoint's returned data for 60 seconds
    let cache_key = "ONLINE_FLIGHTS";
    if let Some(cached) = state.cache.get(&cache_key) {
        let elapsed = Instant::now() - cached.inserted;
        if elapsed.as_secs() < 60 {
            return Ok(Html(cached.data));
        }
        state.cache.invalidate(&cache_key);
    }

    let artcc_fields: Vec<_> = state
        .config
        .airports
        .all
        .iter()
        .map(|airport| &airport.code)
        .collect();
    let data = Vatsim::new().await?.get_v3_data().await?;
    let flights: OnlineFlights =
        data.pilots
            .iter()
            .fold(OnlineFlights::default(), |mut flights, flight| {
                if let Some(plan) = &flight.flight_plan {
                    let from = artcc_fields.contains(&&plan.departure);
                    let to = artcc_fields.contains(&&plan.arrival);
                    match (from, to) {
                        (true, true) => flights.within += 1,
                        (false, true) => flights.to += 1,
                        (true, false) => flights.from += 1,
                        _ => {}
                    }
                };
                flights
            });

    let template = state.templates.get_template("snippet_flights")?;
    let rendered = template.render(context! { flights })?;
    state
        .cache
        .insert(cache_key, CacheEntry::new(rendered.clone()));
    Ok(Html(rendered))
}
