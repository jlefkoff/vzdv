use crate::shared::{AppError, AppState, UserInfo, SESSION_USER_INFO_KEY};
use axum::{extract::State, response::Html, routing::get, Router};
use minijinja::{context, Environment};
use std::sync::Arc;
use tower_sessions::Session;

async fn page_feedback(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    let template = state.templates.get_template("admin/feedback")?;
    let rendered = template.render(context! { user_info })?;
    Ok(Html(rendered))
}

/// This file's routes and templates.
pub fn router(templates: &mut Environment) -> Router<Arc<AppState>> {
    templates
        .add_template(
            "admin/feedback",
            include_str!("../../templates/admin/feedback.jinja"),
        )
        .unwrap();

    Router::new().route("/admin/feedback", get(page_feedback))
}
