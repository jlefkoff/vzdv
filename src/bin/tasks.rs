//! New vZDV website background task runner.

#![deny(clippy::all)]

use anyhow::{Context, Result};
use chrono::Months;
use clap::Parser;
use log::{debug, error, info};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
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
    utils::{
        position_in_facility_airspace,
        vatusa::{get_roster, MembershipType, RosterMember},
    },
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
async fn update_controller_record(
    config: &Config,
    db: &SqlitePool,
    controller: &RosterMember,
) -> Result<()> {
    let roles = controller
        .roles
        .iter()
        .filter(|role| role.facility == config.vatsim.vatusa_facility_code)
        .map(|role| role.role.as_str())
        // there's 1 controller in ZDV who actually has an "INS" role in addition to their controller rating
        .filter(|&role| role != "INS")
        .collect::<Vec<_>>()
        .join(",");
    sqlx::query(sql::UPSERT_USER_TASK)
        .bind(controller.cid)
        .bind(&controller.first_name)
        .bind(&controller.last_name)
        .bind(&controller.email)
        .bind(controller.rating)
        .bind(&controller.facility)
        // controller *will* be on the roster since that's what the VATSIM API is showing
        .bind(true)
        .bind(roles)
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
    /*
     * Don't use a transaction here; instead, attempt to update every controller's
     * data. Don't error-out unless VATSIM doesn't give any data.
     */
    let roster_data = get_roster(&config.vatsim.vatusa_facility_code, MembershipType::Both).await?;
    debug!("Got roster response");
    for controller in &roster_data.data {
        if let Err(e) = update_controller_record(config, db, controller).await {
            error!("Error updating controller {} in DB: {e}", controller.cid);
        };
    }

    debug!("Checking for removed controllers");
    let current_controllers: Vec<_> = roster_data
        .data
        .iter()
        .map(|controller| controller.cid)
        .collect();
    let db_controllers: Vec<SqliteRow> = sqlx::query(sql::GET_ALL_CONTROLLER_CIDS)
        .fetch_all(db)
        .await?;
    for row in db_controllers {
        let cid: u32 = row.try_get("cid")?;
        if !current_controllers.contains(&cid) {
            debug!("Controller {cid} is not on the roster");
            if let Err(e) = sqlx::query(sql::UPDATE_REMOVED_FROM_ROSTER)
                .bind(cid)
                .execute(db)
                .await
            {
                error!("Error updating controller {cid} to show off-roster: {e}")
            }
        }
    }
    Ok(())
}

/// Update all controllers' stored activity data with data from VATSIM.
///
/// For each controller in the DB, their activity data will be cleared,
/// and then (for on-roster controllers) fetched and stored in the DB as
/// part of a transaction.
async fn update_activity(config: &Config, db: &SqlitePool) -> Result<()> {
    // prep cids for on-roster controllers and a 5-month-ago timestamp that the API recognizes
    let controllers = sqlx::query(sql::GET_ALL_ROSTER_CONTROLLER_CIDS)
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
        /*
         * Get the last 5 months of the controller's activity.
         *
         * I'm not (currently) worried about pagination as even the facility's most
         * active controllers don't have enough sessions in this time range to go over
         * the endpoint's single-page response limit.
         */
        let sessions =
            rest_api::get_atc_sessions(cid as u64, None, None, Some(&five_months_ago), None)
                .await
                .with_context(|| format!("Processing CID {cid}"))?;
        // group the controller's activity by month
        let mut seconds_map: HashMap<String, f32> = HashMap::new();
        for session in sessions.results {
            // filter to only sessions in the facility
            if !position_in_facility_airspace(config, &session.callsign) {
                continue;
            }

            let month = session.start[0..7].to_string();
            let seconds = session.minutes_on_callsign.parse::<f32>().unwrap() * 60.0;
            seconds_map
                .entry(month)
                .and_modify(|acc| *acc += seconds)
                .or_insert(seconds);
        }

        // transaction for the ~6 queries
        let mut tx = db.begin().await?;
        // clear the controller's existing records in prep for replacement
        sqlx::query(sql::DELETE_ACTIVITY_FOR_CID)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("Processing CID {cid}"))?;
        // for each relevant month, store their total controlled minutes in the DB
        for (month, seconds) in seconds_map {
            let minutes = (seconds / 60.0).round() as u32;
            sqlx::query(sql::INSERT_INTO_ACTIVITY)
                .bind(cid)
                .bind(month)
                .bind(minutes)
                .execute(&mut *tx)
                .await
                .with_context(|| format!("Processing CID {cid}"))?;
        }
        // commit the controller's changes
        tx.commit().await?;

        // wait a second to be nice to the VATSIM API
        time::sleep(time::Duration::from_secs(1)).await;
    }

    Ok(())
}

#[tokio::main]
async fn main() {
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
                debug!("Waiting 4 hours for next roster sync");
                time::sleep(time::Duration::from_secs(60 * 60 * 4)).await;
            }
        })
    };

    let activity_handle = {
        let config = config.clone();
        let db = db.clone();
        tokio::spawn(async move {
            debug!("Waiting 60 seconds before starting activity sync");
            time::sleep(time::Duration::from_secs(60)).await;
            loop {
                info!("Updating activity");
                match update_activity(&config, &db).await {
                    Ok(_) => {
                        info!("Activity update successful");
                    }
                    Err(e) => {
                        error!("Error updating activity: {e}");
                    }
                }
                debug!("Waiting 12 hours for next activity sync");
                time::sleep(time::Duration::from_secs(60 * 60 * 12)).await;
            }
        })
    };

    roster_handle.await.unwrap();
    activity_handle.await.unwrap();
}
