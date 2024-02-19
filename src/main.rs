use crate::shared::{AppState, Config};
use anyhow::Result;
use axum::{routing::get, Router};
use log::{debug, error, info};
use minijinja::Environment;
use sqlx::{sqlite::SqliteConnectOptions, Executor, SqlitePool};
use std::{env, path::Path, sync::Arc};

mod endpoints;
mod shared;
mod sql;

fn load_config() -> Result<Config> {
    let text = std::fs::read_to_string(shared::CONFIG_FILE_NAME)?;
    let config: Config = toml::from_str(&text)?;
    Ok(config)
}

fn load_templates() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template("layout", include_str!("../templates/layout.jinja"))
        .unwrap();
    env.add_template("home", include_str!("../templates/home.jinja"))
        .unwrap();
    env
}

async fn load_db(config: &Config) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new().filename(&config.database.file);
    let pool = if !Path::new(&config.database.file).exists() {
        let options = options.create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;
        pool.execute(sql::CREATE_TABLES).await?;
        pool
    } else {
        SqlitePool::connect_with(options).await?
    };
    Ok(pool)
}

#[tokio::main]
async fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info,new_site=debug");
    }
    pretty_env_logger::init();

    debug!("Loading");
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            error!("Could not load config: {e}");
            std::process::exit(1);
        }
    };
    let db = match load_db(&config).await {
        Ok(db) => db,
        Err(e) => {
            error!("Could not load DB: {e}");
            std::process::exit(1);
        }
    };
    let templates = load_templates();
    debug!("Loaded");

    debug!("Setting up app");
    let app_state = Arc::new(AppState {
        config,
        db,
        templates,
    });
    let app = Router::new()
        .route("/", get(endpoints::handler_home))
        .with_state(app_state);
    debug!("Set up");

    info!("Listening on http://0.0.0.0:3000/");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
