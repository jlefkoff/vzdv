//! vZDV Discord bot.

#![allow(unused)] // TODO remove
#![deny(clippy::all)]
#![deny(unsafe_code)]

use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use log::{debug, error, info, warn};
use sqlx::{Pool, Sqlite};
use std::{path::PathBuf, sync::Arc};
use twilight_gateway::{Event, Intents, Shard, ShardId};
use twilight_http::Client as HttpClient;
use vzdv::{config::Config, general_setup};

mod tasks;

/// vZDV Discord bot.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Load the config from a specific file.
    ///
    /// [default: vzdv.toml]
    #[arg(long)]
    config: Option<PathBuf>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

/// Parse a bot ID from the token.
///
/// This function panics instead of returning a Result, as the token
/// must confirm to this layout in order to be valid for Discord.
fn bot_id_from_token(token: &str) -> u64 {
    std::str::from_utf8(
        &general_purpose::STANDARD_NO_PAD
            .decode(token.split('.').next().unwrap())
            .unwrap(),
    )
    .unwrap()
    .parse()
    .unwrap()
}

/// Entrypoint.
#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let (config, db) = general_setup(cli.debug, "vzdv_bot", cli.config).await;
    let config = Arc::new(config);

    let token = &config.discord.bot_token;
    let bot_id = bot_id_from_token(token);
    let intents = Intents::GUILD_MEMBERS;
    let mut shard = Shard::new(ShardId::ONE, token.clone(), intents);
    let http = Arc::new(HttpClient::new(token.clone()));

    debug!("Spawning background tasks");

    {
        let config = config.clone();
        let db = db.clone();
        let http = http.clone();
        tokio::spawn(async move {
            tasks::online::process(config, db, http).await;
        });
    };

    {
        let config = config.clone();
        let db = db.clone();
        let http = http.clone();
        tokio::spawn(async move {
            tasks::roles::process(config, db, http).await;
        });
    };

    {
        let config = config.clone();
        let db = db.clone();
        let http = http.clone();
        tokio::spawn(async move {
            tasks::off_roster::process(config, db, http).await;
        });
    };

    info!("Waiting for events");
    loop {
        let event = match shard.next_event().await {
            Ok(event) => event,
            Err(source) => {
                warn!("Error receiving event: {:?}", source);
                if source.is_fatal() {
                    break;
                }
                continue;
            }
        };
        let http = http.clone();
        let config = config.clone();
        let db: Pool<Sqlite> = db.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_event(event, http, bot_id, &config, &db).await {
                error!("Error in future: {e}");
            }
        });
    }
}

/// Handle all events send through the Gateway connection.
async fn handle_event(
    event: Event,
    http: Arc<HttpClient>,
    bot_id: u64,
    config: &Config,
    db: &Pool<Sqlite>,
) -> Result<()> {
    // TODO
    Ok(())
}
