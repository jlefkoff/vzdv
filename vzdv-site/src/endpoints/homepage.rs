//! HTTP endpoints for the homepage.

use crate::{
    flashed_messages,
    shared::{AppError, AppState, CacheEntry, UserInfo, SESSION_USER_INFO_KEY},
};
use anyhow::{anyhow, Result};
use axum::{extract::State, response::Html, routing::get, Router};
use log::warn;
use minijinja::{context, Environment};
use serde::Serialize;
use std::{sync::Arc, time::Instant};
use tower_sessions::Session;
use vatsim_utils::live_api::Vatsim;
use vzdv::{aviation::parse_metar, vatsim::get_online_facility_controllers, GENERAL_HTTP_CLIENT};

/// Homepage.
async fn page_home(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    let template = state.templates.get_template("homepage/home")?;
    let flashed_messages = flashed_messages::drain_flashed_messages(session).await?;
    let rendered = template.render(context! { user_info, flashed_messages })?;
    Ok(Html(rendered))
}

/// Render a list of online controllers.
async fn snippet_online_controllers(
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, AppError> {
    // cache this endpoint's returned data for 60 seconds
    let cache_key = "ONLINE_CONTROLLERS";
    if let Some(cached) = state.cache.get(&cache_key) {
        let elapsed = Instant::now() - cached.inserted;
        if elapsed.as_secs() < 60 {
            return Ok(Html(cached.data));
        }
        state.cache.invalidate(&cache_key);
    }

    let online = get_online_facility_controllers(&state.db, &state.config).await?;
    let template = state
        .templates
        .get_template("homepage/online_controllers")?;
    let rendered = template.render(context! { online })?;
    state
        .cache
        .insert(cache_key, CacheEntry::new(rendered.clone()));
    Ok(Html(rendered))
}

async fn snippet_weather(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    // cache this endpoint's returned data for 5 minutes
    let cache_key = "WEATHER_BRIEF";
    if let Some(cached) = state.cache.get(&cache_key) {
        let elapsed = Instant::now() - cached.inserted;
        if elapsed.as_secs() < 300 {
            return Ok(Html(cached.data));
        }
        state.cache.invalidate(&cache_key);
    }

    let resp = GENERAL_HTTP_CLIENT
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
                let airport = line.split(' ').next().unwrap_or("Unknown");
                warn!("METAR parsing failure for {airport}: {e}");
                e
            })
        })
        .collect();

    let template = state.templates.get_template("homepage/weather")?;
    let rendered = template.render(context! { weather })?;
    state
        .cache
        .insert(cache_key, CacheEntry::new(rendered.clone()));
    Ok(Html(rendered))
}

async fn snippet_flights(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    #[derive(Serialize, Default)]
    struct OnlineFlights {
        within: u16,
        from: u16,
        to: u16,
    }

    // cache this endpoint's returned data for 60 seconds
    let cache_key = "ONLINE_FLIGHTS_HOMEPAGE";
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

    let template = state.templates.get_template("homepage/flights")?;
    let rendered = template.render(context! { flights })?;
    state
        .cache
        .insert(cache_key, CacheEntry::new(rendered.clone()));
    Ok(Html(rendered))
}

/// This file's routes and templates.
pub fn router(templates: &mut Environment) -> Router<Arc<AppState>> {
    templates
        .add_template(
            "homepage/home",
            include_str!("../../templates/homepage/home.jinja"),
        )
        .unwrap();
    templates
        .add_template(
            "homepage/online_controllers",
            include_str!("../../templates/homepage/online_controllers.jinja"),
        )
        .unwrap();
    templates
        .add_template(
            "homepage/weather",
            include_str!("../../templates/homepage/weather.jinja"),
        )
        .unwrap();
    templates
        .add_template(
            "homepage/flights",
            include_str!("../../templates/homepage/flights.jinja"),
        )
        .unwrap();

    Router::new()
        .route("/", get(page_home))
        .route("/home/online/controllers", get(snippet_online_controllers))
        .route("/home/online/flights", get(snippet_flights))
        .route("/home/weather", get(snippet_weather))
}
