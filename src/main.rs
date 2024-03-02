//! New vZVD website.

#![deny(clippy::all)]

use crate::shared::{AppState, Config};
use anyhow::Result;
use axum::{middleware as axum_middleware, response::Redirect, routing::get, Router};
use clap::Parser;
use log::{debug, error, info};
use mini_moka::sync::Cache;
use minijinja::Environment;
use sqlx::{sqlite::SqliteConnectOptions, Executor, SqlitePool};
use std::{
    env,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;

mod endpoints;
mod middleware;
mod shared;
mod utils;

/// vZDV website.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Load the config from a specific file.
    ///
    /// [default: site_config.toml]
    #[arg(long)]
    config: Option<PathBuf>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Host to run on
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Port to run on
    #[arg(long, default_value_t = 3000)]
    port: u16,
}

/// Read the TOML file at the given path and load into the app's
/// configuration file.
fn load_config(path: &Path) -> Result<Config> {
    let text = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&text)?;
    Ok(config)
}

/// Load all template files into the binary via the stdlib `include_str!`
/// macro and supply to the minijinja environment.
fn load_templates() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template("layout", include_str!("../templates/layout.jinja"))
        .unwrap();
    env.add_template("home", include_str!("../templates/home.jinja"))
        .unwrap();
    env.add_template(
        "login_complete",
        include_str!("../templates/login_complete.jinja"),
    )
    .unwrap();
    env.add_template("404", include_str!("../templates/404.jinja"))
        .unwrap();
    env.add_template(
        "snippet_online_controllers",
        include_str!("../templates/snippets/online_controllers.jinja"),
    )
    .unwrap();
    env
}

/// Connect to the SQLite file at the destination, if it exists. If it does
/// not, a new file is created and statements to create tables are executed.
async fn load_db(config: &Config) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new().filename(&config.database.file);
    let pool = if !Path::new(&config.database.file).exists() {
        let options = options.create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;
        pool.execute(shared::sql::CREATE_TABLES).await?;
        pool
    } else {
        SqlitePool::connect_with(options).await?
    };
    Ok(pool)
}

/// Create all the endpoints, include middleware, and connect with the
/// state and session management layer.
fn load_router(
    app_state: Arc<AppState>,
    sessions_layer: SessionManagerLayer<SqliteStore>,
) -> Router {
    Router::new()
        .route("/404", get(endpoints::handler_404))
        .route("/", get(endpoints::handler_home))
        .route("/auth/log_in", get(endpoints::page_auth_login))
        .route("/auth/logout", get(endpoints::page_auth_logout))
        .route("/auth/callback", get(endpoints::page_auth_callback))
        .route(
            "/snippets/online_controllers",
            get(endpoints::snippet_online_controllers),
        )
        .layer(
            ServiceBuilder::new()
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(axum_middleware::from_fn(middleware::logging))
                .layer(sessions_layer),
        )
        .fallback(|| async { Redirect::to("/404") })
        .with_state(app_state)
}

// https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Entrypoint.
#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if cli.debug {
        env::set_var("RUST_LOG", "info,tracing::span=warn,vzdv=debug");
    } else if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "tracing::span=warn,info");
    }
    pretty_env_logger::init();
    debug!("Logging configured");

    debug!("Loading");
    let config_location = match cli.config {
        Some(path) => path,
        None => Path::new(shared::DEFAULT_CONFIG_FILE_NAME).to_owned(),
    };
    debug!(
        "> Loading from config file at: {}",
        config_location.display()
    );
    let config = match load_config(&config_location) {
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
    let sessions = SqliteStore::new(db.clone());
    if let Err(e) = sessions.migrate().await {
        error!("Could not create table for sessions: {e}");
        return;
    }
    let session_layer = SessionManagerLayer::new(sessions);
    let templates = load_templates();
    let cache = Cache::new(10);
    debug!("Loaded");

    debug!("Setting up app");
    let app_state = Arc::new(AppState {
        config,
        db,
        templates,
        cache,
    });
    let app = load_router(app_state, session_layer);
    debug!("Set up");

    let host_and_port = format!("{}:{}", cli.host, cli.port);
    info!("Listening on http://{host_and_port}/");
    let listener = tokio::net::TcpListener::bind(&host_and_port).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    info!("Done");
}
