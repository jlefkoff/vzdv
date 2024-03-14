use crate::shared::{AppError, AppState, UserInfo, SESSION_USER_INFO_KEY};
use axum::{extract::State, response::Html, routing::get, Router};
use minijinja::{context, Environment};
use std::sync::Arc;
use tower_sessions::Session;

/// View the full roster.
async fn page_roster(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    // TODO query DB for roster
    let template = state.templates.get_template("roster")?;
    let rendered = template.render(context! { user_info })?;
    Ok(Html(rendered))
}

pub fn router(templates: &mut Environment) -> Router<Arc<AppState>> {
    templates
        .add_template("roster", include_str!("../../templates/roster.jinja"))
        .unwrap();

    Router::new().route("/facility/roster", get(page_roster))
}
