//! Various utility structs and functions.

use crate::shared::{
    sql::{self, Controller},
    Config,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use once_cell::sync::Lazy;
use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use std::collections::HashMap;

pub mod auth;
pub mod flashed_messages;
pub mod vatusa;

// I don't know what this is, but there's a SUP in ZDV that has this rating.
const IGNORE_MISSING_STAFF_POSITIONS_FOR: [&str; 1] = ["FACCBT"];

/// HTTP client for making external requests.
///
/// Include an HTTP Agent of the project's repo for contact.
pub static GENERAL_HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    ClientBuilder::new()
        .user_agent("github.com/celeo/vzdv")
        .build()
        .expect("Could not construct HTTP client")
});

/// Parse a VATSIM timestamp into a `chrono::DateTime`.
pub fn parse_vatsim_timestamp(stamp: &str) -> Result<DateTime<Utc>> {
    let naive = NaiveDateTime::parse_from_str(stamp, "%Y-%m-%dT%H:%M:%S%.fZ")?;
    let utc = match Utc.from_local_datetime(&naive) {
        chrono::LocalResult::Single(t) => t,
        _ => {
            return Err(anyhow!("Could not parse VATSIM timestamp"));
        }
    };
    Ok(utc)
}

/// Derived weather conditions.
#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Debug, PartialEq)]
pub enum WeatherConditions {
    VFR,
    MVFR,
    IFR,
    LIFR,
}

/// Parsed weather information for an airport.
#[derive(Serialize)]
pub struct AirportWeather<'a> {
    pub name: &'a str,
    pub conditions: WeatherConditions,
    pub visibility: u8,
    pub ceiling: u16,
    pub raw: &'a str,
}

/// Parse a METAR into a struct of data.
pub fn parse_metar(line: &str) -> Result<AirportWeather> {
    let parts: Vec<_> = line.split(' ').collect();
    let airport = parts.first().ok_or_else(|| anyhow!("Blank metar?"))?;
    let mut ceiling = 3_456;
    for part in &parts {
        if part.starts_with("BKN") || part.starts_with("OVC") {
            ceiling = part
                .chars()
                .skip_while(|c| c.is_alphabetic())
                .take_while(|c| c.is_numeric())
                .collect::<String>()
                .parse::<u16>()?
                * 100;
            break;
        }
    }

    let visibility: u8 = parts
        .iter()
        .find(|part| part.ends_with("SM"))
        .map(|part| {
            let vis = part.replace("SM", "");
            if vis.contains('/') {
                0
            } else {
                vis.parse().unwrap()
            }
        })
        .unwrap_or(0);

    let conditions = if visibility > 5 && ceiling > 3_000 {
        WeatherConditions::VFR
    } else if visibility >= 3 && ceiling > 1_000 {
        WeatherConditions::MVFR
    } else if visibility >= 1 && ceiling > 500 {
        WeatherConditions::IFR
    } else {
        WeatherConditions::LIFR
    };

    Ok(AirportWeather {
        name: airport,
        conditions,
        visibility,
        ceiling,
        raw: line,
    })
}

/// Query the SimAware data endpoint for its data on active pilot sessions.
///
/// This endpoint should be cached so as to not hit the SimAware server too frequently.
pub async fn get_simaware_data() -> Result<HashMap<u64, String>> {
    #[derive(Deserialize)]
    struct Pilot {
        cid: u64,
    }

    #[derive(Deserialize)]
    struct TopLevel {
        pilots: HashMap<String, Pilot>,
    }

    let mut mapping = HashMap::new();
    let data: TopLevel = GENERAL_HTTP_CLIENT
        .get("https://r2.simaware.ca/api/livedata/data.json")
        .send()
        .await?
        .json()
        .await?;
    for (id, pilot) in data.pilots {
        mapping.insert(pilot.cid, id);
    }
    Ok(mapping)
}

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

#[cfg(test)]
pub mod tests {
    use super::{
        determine_staff_positions, parse_metar, parse_vatsim_timestamp,
        position_in_facility_airspace, WeatherConditions,
    };
    use crate::shared::{config::ConfigStaffOverride, sql::Controller, Config};
    use pretty_assertions::assert_eq;

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
