//! Various utility structs and functions.

use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use once_cell::sync::Lazy;
use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::shared::Config;

pub mod auth;
pub mod flashed_messages;
pub mod vatusa;

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

#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Debug, PartialEq)]
pub enum WeatherConditions {
    VFR,
    MVFR,
    IFR,
    LIFR,
}

#[derive(Serialize)]
pub struct AirportWeather<'a> {
    pub name: &'a str,
    pub conditions: WeatherConditions,
    pub visibility: u8,
    pub ceiling: u16,
    pub raw: &'a str,
}

/// Parse a METAR to determine if conditions are VMC or IMC.
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
pub async fn simaware_data() -> Result<HashMap<u64, String>> {
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

#[cfg(test)]
pub mod tests {
    use super::{
        parse_metar, parse_vatsim_timestamp, position_in_facility_airspace, WeatherConditions,
    };
    use crate::shared::Config;

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
}
