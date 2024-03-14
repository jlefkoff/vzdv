//! New vZDV website background task runner.

#![deny(clippy::all)]

use anyhow::Result;
use clap::Parser;
use log::{debug, error, info};
use sqlx::SqlitePool;
use std::{
    env,
    path::{Path, PathBuf},
};
use tokio::time;
use vzdv::{
    load_config, load_db,
    shared::{self, sql, Config},
    utils::vatusa::{get_roster, MembershipType, RosterMember},
};

/// vZDV task runner.
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
}

async fn update_single(db: &SqlitePool, controller: &RosterMember) -> Result<()> {
    let roles = controller
        .roles
        .iter()
        .map(|role| format!("{}:{}", role.facility, role.role))
        .collect::<Vec<_>>()
        .join(",");
    sqlx::query(sql::UPSERT_USER_TASK)
        .bind(controller.cid)
        .bind(&controller.first_name)
        .bind(&controller.last_name)
        .bind(&controller.email)
        .bind(controller.rating)
        .bind(&controller.facility)
        .bind(roles)
        .bind(chrono::Utc::now())
        .execute(db)
        .await?;
    debug!(
        "{} {} ({}) updated in DB",
        &controller.first_name, &controller.last_name, controller.cid
    );
    Ok(())
}

async fn update_roster(config: &Config, db: &SqlitePool) -> Result<()> {
    let roster_data = get_roster(&config.vatsim.vatusa_facility_code, MembershipType::Both).await?;
    for controller in &roster_data.data {
        if let Err(e) = update_single(db, controller).await {
            error!("Error updating controller in DB: {e}");
        };
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    if cli.debug {
        env::set_var("RUST_LOG", "info,tasks=debug");
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
    debug!("Creating DB connection");
    let db = match load_db(&config).await {
        Ok(db) => db,
        Err(e) => {
            error!("Could not load DB: {e}");
            std::process::exit(1);
        }
    };

    // debug!("Waiting 10 seconds before starting loop");
    // time::sleep(time::Duration::from_secs(10)).await;
    info!("Starting loop");

    loop {
        debug!("Querying");
        match update_roster(&config, &db).await {
            Ok(_) => {
                info!("Roster update successful");
            }
            Err(e) => {
                error!("Error updating roster: {e}");
            }
        }
        info!("Update complete; waiting 1 hour");
        time::sleep(time::Duration::from_secs(3_600)).await;
    }
}
