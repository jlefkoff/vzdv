use crate::config::Config;
use minijinja::Environment;
use sqlx::SqlitePool;

pub struct AppState {
    pub config: Config,
    pub db: SqlitePool,
    pub templates: Environment<'static>,
}
