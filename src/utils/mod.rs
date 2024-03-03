use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::Serialize;

pub mod auth;

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

#[derive(Serialize)]
pub struct AirportWeather<'a> {
    pub name: &'a str,
    pub weather: &'static str,
    pub raw: &'a str,
}

/// Parse a METAR to determine if conditions are VMC or IMC.
pub fn parse_metar(line: &str) -> Result<AirportWeather> {
    let visibility: u8 = 0;
    let parts: Vec<_> = line.split(' ').collect();
    let airport = parts.first().ok_or_else(|| anyhow!("Blank metar?"))?;
    let mut cloud_layer = 1_001;
    for part in &parts {
        if part.starts_with("BKN") || part.starts_with("OVC") {
            cloud_layer = part
                .chars()
                .skip_while(|c| c.is_alphabetic())
                .collect::<String>()
                .parse::<u16>()?
                * 100;
            break;
        }
    }
    Ok(AirportWeather {
        name: airport,
        weather: if visibility >= 3 && cloud_layer > 1_000 {
            "IMC"
        } else {
            "VMC"
        },
        raw: line,
    })
}

#[cfg(test)]
pub mod tests {
    use super::{parse_metar, parse_vatsim_timestamp};

    #[test]
    fn test_parse_vatsim_timestamp() {
        parse_vatsim_timestamp("2024-03-02T16:20:37.0439318Z").unwrap();
    }

    #[test]
    fn test_parse_metar() {
        let ret = parse_metar("KDEN 030253Z 22013KT 10SM SCT100 BKN160 13/M12 A2943 RMK AO2 PK WND 21036/0211 SLP924 T01331117 58005").unwrap();
        assert_eq!(ret.name, "KDEN");
        assert_eq!(ret.weather, "VMC");
    }
}
