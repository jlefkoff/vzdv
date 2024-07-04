use super::{parse_vatsim_timestamp, position_in_facility_airspace};
use crate::{shared::Config, utils::get_controller_cids_and_names};
use anyhow::Result;
use log::error;
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use vatsim_utils::live_api::Vatsim;

#[derive(Debug, Serialize)]
pub struct OnlineController {
    pub cid: u64,
    pub callsign: String,
    pub name: String,
    pub online_for: String,
}

/// Get facility controllers currently online.
pub async fn get_online_facility_controllers(
    db: &SqlitePool,
    config: &Config,
) -> Result<Vec<OnlineController>> {
    let cid_name_map = match get_controller_cids_and_names(db).await {
        Ok(map) => map,
        Err(e) => {
            error!("Error generating controller CID -> name map: {e}");
            HashMap::new()
        }
    };

    let now = chrono::Utc::now();
    let data = Vatsim::new().await?.get_v3_data().await?;
    let online: Vec<_> = data
        .controllers
        .iter()
        .filter(|controller| position_in_facility_airspace(config, &controller.callsign))
        .map(|controller| {
            let logon = parse_vatsim_timestamp(&controller.logon_time)
                .expect("Could not parse VATSIM timestamp");
            let seconds = (now - logon).num_seconds() as u32;
            OnlineController {
                cid: controller.cid,
                callsign: controller.callsign.clone(),
                name: cid_name_map
                    .get(&controller.cid)
                    .map(|s| format!("{} {}", s.0, s.1))
                    .unwrap_or(String::from("?")),
                online_for: format!("{}h{}m", seconds / 3600, (seconds / 60) % 60),
            }
        })
        .collect();

    Ok(online)
}
