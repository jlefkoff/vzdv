use log::debug;
use sqlx::{Pool, Sqlite};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use twilight_http::Client;
use vzdv::config::Config;

pub async fn process(config: Arc<Config>, db: Pool<Sqlite>, http: Arc<Client>) {
    sleep(Duration::from_secs(30)).await;
    debug!("Starting online message processing");

    // TODO
}
