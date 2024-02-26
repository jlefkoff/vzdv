//! HTTP endpoints.

use crate::shared::{AppState, UserInfo, SESSION_USER_INFO_KEY};
use anyhow::Result;
use axum::{extract::State, http::StatusCode, response::Html};
use minijinja::context;
use std::sync::Arc;
use tower_sessions::Session;

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
