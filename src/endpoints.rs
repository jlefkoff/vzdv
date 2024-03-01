//! HTTP endpoints.

use crate::{
    shared::{AppError, AppState, UserInfo, SESSION_USER_INFO_KEY},
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
use std::sync::Arc;
use tower_sessions::Session;

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

pub async fn handler_auth_login(
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

pub async fn handler_auth_callback(
    query: Query<AuthCallback>,
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, AppError> {
    debug!("Auth callback");
    let token_data = code_to_user_info(&query.code, &state).await?;
    debug!("Got token data");
    let user_info = get_user_info(&token_data.access_token).await?;
    debug!("Got user info");

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

pub async fn handler_auth_logout(session: Session) -> Result<Redirect, AppError> {
    session.delete().await?;
    Ok(Redirect::to("/"))
}
