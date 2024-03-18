//! Temporary program to be used with "existing_site_scrape.js".
//!
//! Will be deleted when this new site is pushed to production.

use anyhow::Result;
use serde::Deserialize;
use std::path::Path;
use vzdv::{load_config, load_db, shared::sql::Controller};

#[derive(Debug, Deserialize)]
struct Cert {
    name: String,
    level: String,
}

#[derive(Debug, Deserialize)]
struct ExportData {
    ois: String,
    name: String,
    certs: Vec<Cert>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config(Path::new("site_config.toml"))?;
    let db = load_db(&config).await?;
    let export_text = std::fs::read_to_string("tmp.json")?;
    let export_data: Vec<ExportData> = serde_json::from_str(&export_text)?;
    for data_item in export_data {
        let first_name = data_item.name.split(' ').next().unwrap();
        let last_name = data_item.name.split(' ').nth(1).unwrap();
        sqlx::query(
            "UPDATE controller SET operating_initials=$1 WHERE first_name=$2 AND last_name=$3",
        )
        .bind(&data_item.ois)
        .bind(first_name)
        .bind(last_name)
        .execute(&db)
        .await?;

        if let Some(controller) =
            sqlx::query_as::<_, Controller>("SELECT * FROM controller WHERE operating_initials=$1")
                .bind(&data_item.ois)
                .fetch_optional(&db)
                .await?
        {
            sqlx::query("DELETE FROM certification WHERE cid=$1")
                .bind(controller.cid)
                .execute(&db)
                .await?;
            for cert in data_item.certs {
                sqlx::query(
                    "
            INSERT INTO certification
                (id, cid, name, value, changed_on, set_by)
            VALUES
                (NULL, $1, $2, $3, '2024-03-16T18:00:00', 0)
            ",
                )
                .bind(controller.cid)
                .bind(&cert.name)
                .bind(&cert.level)
                .execute(&db)
                .await?;
            }
        } else {
            eprintln!("Could not find controller with OIs {}", &data_item.ois);
        }
    }

    Ok(())
}
