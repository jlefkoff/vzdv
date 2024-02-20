//! New vZVD website.

// don't permit any lint-able bad practices
#![deny(clippy::all)]

use crate::shared::{AppState, Config};
use anyhow::Result;
use axum::{routing::get, Router};
use clap::Parser;
use log::{debug, error, info};
use minijinja::Environment;
use sqlx::{sqlite::SqliteConnectOptions, Executor, SqlitePool};
use std::{
    env,
    path::{Path, PathBuf},
    sync::Arc,
};

mod endpoints;
mod shared;
mod sql;

/// New vZDV website.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Load the config from a specific file.
    ///
    /// Defaults to "./site_config.toml".
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

/// Read the TOML file at the given path and load
/// into the app's configuration file.
fn load_config(path: &Path) -> Result<Config> {
    let text = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&text)?;
    Ok(config)
}

/// Load all template files into the binary via
/// the stdlib `include_str!` macro and supply
/// to the minijinja environment.
fn load_templates() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template("layout", include_str!("../templates/layout.jinja"))
        .unwrap();
    env.add_template("home", include_str!("../templates/home.jinja"))
        .unwrap();
    env
}

/// Connect to the SQLite file at the destination,
/// if it exists. If it does not, a new file is
/// created and statements to create tables are
/// executed.
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

/// Create all the endpoints and connect with the state.
fn load_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(endpoints::handler_home))
        .with_state(app_state)
}

/// Entrypoint.
#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if cli.debug {
        env::set_var("RUST_LOG", "info,vzdv=debug");
    } else if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();
    debug!("Logging configured");

    debug!("Loading");
    let config_location = match cli.config {
        Some(path) => path,
        None => Path::new(shared::DEFAULT_CONFIG_FILE_NAME).to_owned(),
    };
    debug!("Loading from config file at: {}", config_location.display());
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
    let templates = load_templates();
    debug!("Loaded");

    debug!("Setting up app");
    let app_state = Arc::new(AppState {
        config,
        db,
        templates,
    });
    let app = load_router(app_state);
    debug!("Set up");

    let host_and_port = format!("{}:{}", cli.host, cli.port);
    info!("Listening on http://{host_and_port}/");
    let listener = tokio::net::TcpListener::bind(&host_and_port).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
