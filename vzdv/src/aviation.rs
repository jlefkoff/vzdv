use anyhow::{anyhow, Result};
use serde::Serialize;

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
