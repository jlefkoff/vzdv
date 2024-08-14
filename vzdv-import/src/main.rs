//! vZDV importer to get data from existing site.

#![deny(clippy::all)]
#![deny(unsafe_code)]

use anyhow::{bail, Result};
use clap::Parser;
use log::{debug, error, info};
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use std::{collections::HashMap, path::PathBuf};
use vzdv::{general_setup, GENERAL_HTTP_CLIENT};

const ROSTER_URL: &str = "https://api.zdvartcc.org/v1/user/all";

/// vZDV importer to get data from existing site.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Load the config from a specific file.
    ///
    /// [default: vzdv.toml]
    #[arg(long)]
    config: Option<PathBuf>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[derive(Deserialize)]
struct AdhCertification {
    display_name: String,
    value: String,
}

#[derive(Deserialize)]
struct AdhController {
    cid: u32,
    operating_initials: String,
    certifications: HashMap<String, AdhCertification>,
}

async fn get_adh_data() -> Result<Vec<AdhController>> {
    let response = GENERAL_HTTP_CLIENT.get(ROSTER_URL).send().await?;
    if !response.status().is_success() {
        bail!(
            "Got status {} from ZDV ADH roster endpoint",
            response.status().as_u16()
        );
    }
    let data: Vec<AdhController> = response.json().await?;
    info!("Got {} controllers from the ZDV roster", data.len());
    Ok(data)
}

async fn update_single(db: &Pool<Sqlite>, controller: &AdhController) -> Result<()> {
    sqlx::query("UPDATE controller SET operating_initials=$1 where cid=$2")
        .bind(controller.operating_initials.clone())
        .bind(controller.cid)
        .execute(db)
        .await?;

    sqlx::query("DELETE FROM certification WHERE cid=$1")
        .bind(controller.cid)
        .execute(db)
        .await?;
    for certification in controller.certifications.values() {
        if certification.value == "none" {
            continue;
        }
        sqlx::query("INSERT INTO certification (id, cid, name, value, changed_on, set_by) VALUES (NULL, $1, $2, $3, $4, $5)")
            .bind(controller.cid)
            .bind(&certification.display_name)
            .bind(&certification.value)
            .bind(chrono::Utc::now())
            .bind(0)
            .execute(db)
            .await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let (_config, db) = general_setup(cli.debug, "vzdv_import", cli.config).await;

    info!("Retrieving data");
    let data = match get_adh_data().await {
        Ok(d) => d,
        Err(e) => {
            error!("Error getting data: {e}");
            return;
        }
    };

    for controller in data {
        debug!("Updating {}", controller.cid);
        if let Err(e) = update_single(&db, &controller).await {
            error!("Error updating controller {}: {e}", controller.cid);
        }
    }

    info!("Complete");
}
