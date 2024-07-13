use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use twilight_http::Client;
use vzdv::config::Config;

pub async fn process(config: Arc<Config>, db: Pool<Sqlite>, http: Arc<Client>) {
    // TODO
}
