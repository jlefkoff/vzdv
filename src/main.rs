use crate::{config::Config, shared::AppState};
use anyhow::Result;
use axum::{routing::get, Router};
use minijinja::Environment;
use sqlx::{sqlite::SqliteConnectOptions, Executor, SqlitePool};
use std::{path::Path, sync::Arc};

mod config;
mod endpoints;
mod shared;
mod sql;

fn load_config() -> Result<Config> {
    let text = std::fs::read_to_string("./config.toml")?;
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
async fn main() -> Result<()> {
    let config = load_config()?;
    let db = load_db(&config).await?;
    let templates = load_templates();

    let app_state = Arc::new(AppState {
        config,
        db,
        templates,
    });

    let app = Router::new()
        .route("/", get(endpoints::handler_home))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
