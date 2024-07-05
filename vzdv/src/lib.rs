//! vZDV site, tasks, and bot core and shared logic.

#![deny(clippy::all)]
#![deny(unsafe_code)]

use anyhow::Result;
use config::Config;
use once_cell::sync::Lazy;
use reqwest::ClientBuilder;
use sql::Controller;
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use std::collections::HashMap;

pub mod aviation;
pub mod config;
pub mod db;
pub mod sql;
pub mod vatsim;
pub mod vatusa;

// I don't know what this is, but there's a SUP in ZDV that has this rating.
const IGNORE_MISSING_STAFF_POSITIONS_FOR: [&str; 1] = ["FACCBT"];

/// HTTP client for making external requests.
///
/// Include an HTTP user agent of the project's repo for contact.
pub static GENERAL_HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    ClientBuilder::new()
        .user_agent("github.com/celeo/vzdv")
        .build()
        .expect("Could not construct HTTP client")
});

/// Check whether the VATSIM session position is in this facility's airspace.
///
/// Relies on the config's "stats.position_prefixes" and suffixes.
pub fn position_in_facility_airspace(config: &Config, position: &str) -> bool {
    let prefix_match = config
        .stats
        .position_prefixes
        .iter()
        .any(|prefix| position.starts_with(prefix));
    if !prefix_match {
        return false;
    }
    config
        .stats
        .position_suffixes
        .iter()
        .any(|suffix| position.ends_with(suffix))
}

/// Retrieve a mapping of controller CID to first and last names.
pub async fn get_controller_cids_and_names(
    db: &Pool<Sqlite>,
) -> Result<HashMap<u64, (String, String)>> {
    let mut cid_name_map: HashMap<u64, (String, String)> = HashMap::new();
    let rows: Vec<SqliteRow> = sqlx::query(sql::GET_CONTROLLER_CIDS_AND_NAMES)
        .fetch_all(db)
        .await?;
    rows.iter().for_each(|row| {
        let cid: u32 = row.try_get("cid").unwrap();
        let first_name: String = row.try_get("first_name").unwrap();
        let last_name: String = row.try_get("last_name").unwrap();
        cid_name_map.insert(cid as u64, (first_name, last_name));
    });
    Ok(cid_name_map)
}

/// Determine the staff position of the controller.
///
/// VATUSA does not differentiate between the official staff position (say, FE)
/// and their assistants (e.g. AFE). At the VATUSA level, they're the same. Here,
/// we do want to determine that difference.
///
/// This function will return all positions in the event the controller holds more
/// than one, like being an Instructor and also the FE, or a Mentor and an AEC.
pub fn determine_staff_positions(controller: &Controller, config: &Config) -> Vec<String> {
    let mut ret_roles = Vec::new();
    let db_roles: Vec<_> = controller.roles.split_terminator(',').collect();
    for role in db_roles {
        if IGNORE_MISSING_STAFF_POSITIONS_FOR.contains(&role) {
            continue;
        }
        let ovr = config.staff.overrides.iter().find(|o| o.role == role);
        if let Some(ovr) = ovr {
            if ovr.cid == controller.cid {
                ret_roles.push(role.to_owned());
            } else {
                ret_roles.push(format!("A{role}"));
            }
        } else {
            ret_roles.push(role.to_owned());
        }
    }
    if controller.home_facility == "ZDV" && [8, 9, 10].contains(&controller.rating) {
        ret_roles.push("INS".to_owned());
    }
    ret_roles
}

#[allow(clippy::upper_case_acronyms)]
pub enum ControllerRating {
    OBS,
    S1,
    S2,
    S3,
    C1,
    C3,
    L1,
    L3,
    SUP,
    ADM,
    INA,
}

pub enum ControllerStatus {
    Active,
    Inactive,
    LeaveOfAbsence,
}

#[allow(clippy::upper_case_acronyms)]
pub enum StaffPosition {
    None,
    ATM,
    DATM,
    TA,
    FE,
    EC,
    WM,
    AFE,
    AEC,
    AWM,
    INS,
    MTR,
}

#[cfg(test)]
pub mod tests {
    use super::{determine_staff_positions, position_in_facility_airspace};
    use crate::{
        aviation::{parse_metar, WeatherConditions},
        config::{Config, ConfigStaffOverride},
        sql::Controller,
        vatsim::parse_vatsim_timestamp,
    };

    #[test]
    fn test_parse_vatsim_timestamp() {
        parse_vatsim_timestamp("2024-03-02T16:20:37.0439318Z").unwrap();
    }

    #[test]
    fn test_parse_metar() {
        let ret = parse_metar("KDEN 030253Z 22013KT 10SM SCT100 BKN160 13/M12 A2943 RMK AO2 PK WND 21036/0211 SLP924 T01331117 58005").unwrap();
        assert_eq!(ret.name, "KDEN");
        assert_eq!(ret.conditions, WeatherConditions::VFR);

        let ret = parse_metar("KDEN 2SM BNK005").unwrap();
        assert_eq!(ret.conditions, WeatherConditions::IFR);

        let ret = parse_metar("KDEN 4SM OVC020").unwrap();
        assert_eq!(ret.conditions, WeatherConditions::MVFR);

        let ret = parse_metar("KDEN 1/2SM OVC001").unwrap();
        assert_eq!(ret.conditions, WeatherConditions::LIFR);
    }

    #[test]
    fn test_position_in_facility_airspace() {
        let mut config = Config::default();
        config.stats.position_prefixes.push("DEN".to_string());
        config.stats.position_suffixes.push("_TWR".to_string());

        assert!(position_in_facility_airspace(&config, "DEN_2_TWR"));
        assert!(!position_in_facility_airspace(&config, "SAN_GND"));
    }

    #[test]
    fn test_determine_staff_positions_empty() {
        let mut controller = Controller::default();
        controller.cid = 123;
        let config = Config::default();

        assert!(determine_staff_positions(&controller, &config).is_empty());
    }

    #[test]
    fn test_determine_staff_positions_shared() {
        let mut controller = Controller::default();
        controller.cid = 123;
        controller.roles = "MTR".to_owned();
        let config = Config::default();

        assert_eq!(determine_staff_positions(&controller, &config), vec!["MTR"]);
    }

    #[test]
    fn test_determine_staff_positions_single() {
        let mut controller = Controller::default();
        controller.cid = 123;
        controller.roles = "FE".to_owned();
        let config = Config::default();

        assert_eq!(determine_staff_positions(&controller, &config), vec!["FE"]);
    }

    #[test]
    fn test_determine_staff_positions_single_assistant() {
        let mut controller = Controller::default();
        controller.cid = 123;
        controller.roles = "FE".to_owned();
        let mut config = Config::default();
        config.staff.overrides.push(ConfigStaffOverride {
            role: "FE".to_owned(),
            cid: 456,
        });

        assert_eq!(determine_staff_positions(&controller, &config), vec!["AFE"]);
    }

    #[test]
    fn test_determine_staff_positions_multiple() {
        let mut controller = Controller::default();
        controller.cid = 123;
        controller.roles = "FE,MTR".to_owned();
        let mut config = Config::default();
        config.staff.overrides.push(ConfigStaffOverride {
            role: "FE".to_owned(),
            cid: 456,
        });

        assert_eq!(
            determine_staff_positions(&controller, &config),
            vec!["AFE", "MTR"]
        );
    }

    #[test]
    fn test_determine_staff_positions_instructor() {
        let mut controller = Controller::default();
        controller.cid = 123;
        controller.rating = 10;
        controller.home_facility = "ZDV".to_owned();
        let config = Config::default();

        assert_eq!(determine_staff_positions(&controller, &config), vec!["INS"]);
    }

    #[test]
    fn test_determine_staff_positions_ingore() {
        let mut controller = Controller::default();
        controller.cid = 123;
        controller.roles = "FACCBT".to_owned();
        let config = Config::default();

        assert!(determine_staff_positions(&controller, &config).is_empty());
    }
}
