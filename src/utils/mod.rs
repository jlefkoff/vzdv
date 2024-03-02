use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

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

#[cfg(test)]
pub mod tests {
    use super::parse_vatsim_timestamp;

    #[test]
    fn test_parse_vatsim_timestamp() {
        parse_vatsim_timestamp("2024-03-02T16:20:37.0439318Z").unwrap();
    }
}
