//! vZDV Discord bot.

#![deny(clippy::all)]
#![deny(unsafe_code)]

use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use log::{debug, error, info, warn};
use sqlx::{Pool, Sqlite};
use std::{
    env,
    path::{Path, PathBuf},
    sync::Arc,
};
use twilight_gateway::{Event, Intents, Shard, ShardId};
use twilight_http::Client as HttpClient;
use vzdv::{
    config::{self, Config},
    db::load_db,
};

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
    if cli.debug {
        env::set_var("RUST_LOG", "info,vzdv_bot=debug");
    } else if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();
    debug!("Logging configured");

    debug!("Loading");
    let config_location = match cli.config {
        Some(path) => path,
        None => Path::new(config::DEFAULT_CONFIG_FILE_NAME).to_owned(),
    };
    debug!("Loading from config file");
    let config = match Config::load_from_disk(&config_location) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!("Could not load config: {e}");
            std::process::exit(1);
        }
    };
    debug!("Creating DB connection");
    let db = match load_db(&config).await {
        Ok(db) => db,
        Err(e) => {
            error!("Could not load DB: {e}");
            std::process::exit(1);
        }
    };

    let token = &config.discord.bot_token;
    let bot_id = bot_id_from_token(token);
    let intents = Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;
    let mut shard = Shard::new(ShardId::ONE, token.clone(), intents);
    let http = Arc::new(HttpClient::new(token.clone()));

    // TODO start background tasks

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
