use anyhow::Result;
use chrono::Utc;
use log::{debug, error, info};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use std::{fmt::Write, sync::Arc, time::Duration};
use tokio::time::sleep;
use twilight_http::Client;
use twilight_model::{channel::message::Embed, id::Id};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder};
use vatsim_utils::live_api::Vatsim;
use vzdv::{
    config::Config,
    sql::{self, Controller},
    vatsim::get_online_facility_controllers,
};

async fn create_message(config: &Arc<Config>, db: &Pool<Sqlite>) -> Result<Embed> {
    // data.iter().fold(String::new(), |mut builder, controller| {
    //     writeln!(
    //         builder,
    //         "{} - {} - {}",
    //         controller.callsign, controller.name, controller.online_for
    //     );
    //     builder
    // })

    let data = get_online_facility_controllers(db, config).await?;

    let enroute: Vec<String> = Vec::new();
    let tracon: Vec<String> = Vec::new();
    let cab: Vec<String> = Vec::new();

    let embed = EmbedBuilder::new()
        .title("Online Controllers")
        .field(EmbedFieldBuilder::new("Enroute", "No controllers"))
        .field(EmbedFieldBuilder::new("TRACON", "No controllers"))
        .field(EmbedFieldBuilder::new("CAB", "No controllers"))
        .footer(EmbedFooterBuilder::new(
            Utc::now().format("%H:%M:%S").to_string(),
        ))
        .validate()?
        .build();
    Ok(embed)
}

/// Single loop execution.
async fn tick(config: &Arc<Config>, db: &Pool<Sqlite>, http: &Arc<Client>) -> Result<()> {
    let channel_id = Id::new(config.discord.online_channel);
    match config.discord.online_message {
        Some(id) => {
            http.update_message(channel_id, Id::new(id))
                .embeds(Some(&[create_message(config, db).await?]))?
                .await?;
        }
        None => {
            http.create_message(channel_id)
                .embeds(&[create_message(config, db).await?])?
                .await?;
        }
    }

    Ok(())
}

pub async fn process(config: Arc<Config>, db: Pool<Sqlite>, http: Arc<Client>) {
    // sleep(Duration::from_secs(30)).await;
    sleep(Duration::from_secs(5)).await;
    debug!("Starting online processing");

    loop {
        if let Err(e) = tick(&config, &db, &http).await {
            error!("Error in online processing tick: {e}");
        }
        sleep(Duration::from_secs(60)).await; // 1 minute
    }
}
