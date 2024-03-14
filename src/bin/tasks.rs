//! New vZDV website background task runner.

#![deny(clippy::all)]

use anyhow::Result;
use chrono::Months;
use clap::Parser;
use log::{debug, error, info};
use sqlx::{Row, SqlitePool};
use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
};
use tokio::time;
use vatsim_utils::rest_api;
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

/// Update a single controller's stored data.
async fn update_controller_record(db: &SqlitePool, controller: &RosterMember) -> Result<()> {
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

/// Update the stored roster with fresh data from VATUSA.
async fn update_roster(config: &Config, db: &SqlitePool) -> Result<()> {
    let roster_data = get_roster(&config.vatsim.vatusa_facility_code, MembershipType::Both).await?;
    for controller in &roster_data.data {
        if let Err(e) = update_controller_record(db, controller).await {
            error!("Error updating controller in DB: {e}");
        };
    }
    // TODO handle removed members
    Ok(())
}

/// Update all controllers' stored activity data with data from VATSIM.
///
/// To do this, for each controller in the DB, their activity data will
/// be cleared and then re-stored as part of a transaction.
async fn update_activity(db: &SqlitePool) -> Result<()> {
    let controllers = sqlx::query(sql::GET_ALL_CONTROLLER_CIDS)
        .fetch_all(db)
        .await?;
    let five_months_ago = chrono::Utc::now()
        .checked_sub_months(Months::new(5))
        .unwrap()
        .format("%Y-%m-%d")
        .to_string();
    for row in controllers {
        let cid: u32 = row.try_get("cid")?;
        debug!("Getting activity for {cid}");

        let sessions =
            rest_api::get_atc_sessions(cid as u64, None, None, Some(&five_months_ago), None)
                .await?;
        // start a transaction and clear the controller's stored activity
        let mut tx = db.begin().await?;
        sqlx::query(sql::DELETE_FROM_ACTIVITY)
            .bind(cid)
            .execute(&mut *tx)
            .await?;

        // group the controller's activity by month
        let mut seconds_map: HashMap<String, f32> = HashMap::new();
        for session in sessions.results {
            let month = session.start[0..7].to_string();
            let seconds = session.minutes_on_callsign.parse::<f32>().unwrap() * 60.0;
            seconds_map
                .entry(month)
                .and_modify(|acc| *acc += seconds)
                .or_insert(seconds);
        }
        // for each relevant month, store their total controlled minutes in the DB
        for (month, seconds) in seconds_map {
            let minutes = (seconds / 60.0).round() as u32;
            sqlx::query(sql::INSERT_INTO_ACTIVITY)
                .bind(cid)
                .bind(month)
                .bind(minutes)
                .execute(&mut *tx)
                .await?;
        }
        // commit the changes to this controller's activity records
        tx.commit().await?;
        // wait a second to be nice to the VATSIM API
        time::sleep(time::Duration::from_secs(1)).await;
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

    info!("Starting tasks");

    let roster_handle = {
        let config = config.clone();
        let db = db.clone();
        tokio::spawn(async move {
            debug!("Waiting 10 seconds before starting roster sync");
            time::sleep(time::Duration::from_secs(10)).await;
            loop {
                info!("Querying roster");
                match update_roster(&config, &db).await {
                    Ok(_) => {
                        info!("Roster update successful");
                    }
                    Err(e) => {
                        error!("Error updating roster: {e}");
                    }
                }
                info!("Waiting 1 hour for next roster sync");
                time::sleep(time::Duration::from_secs(60 * 60)).await;
            }
        })
    };

    let activity_handle = {
        let db = db.clone();
        tokio::spawn(async move {
            debug!("Waiting 30 seconds before starting activity sync");
            time::sleep(time::Duration::from_secs(30)).await;
            loop {
                info!("Updating activity");
                match update_activity(&db).await {
                    Ok(_) => {
                        info!("Activity update successful");
                    }
                    Err(e) => {
                        error!("Error updating activity: {e}");
                    }
                }
                info!("Waiting 12 hours for next activity sync");
                time::sleep(time::Duration::from_secs(60 * 60 * 12)).await;
            }
        })
    };

    roster_handle.await.unwrap();
    activity_handle.await.unwrap();

    Ok(())
}
