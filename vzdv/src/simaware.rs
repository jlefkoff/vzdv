use crate::GENERAL_HTTP_CLIENT;
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

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
