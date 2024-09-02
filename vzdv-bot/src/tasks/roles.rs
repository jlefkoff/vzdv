use anyhow::Result;
use log::{debug, error, info};
use sqlx::{Pool, Sqlite};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use twilight_http::Client;
use twilight_model::id::Id;
use vzdv::{
    config::Config,
    sql::{self, Controller},
};

/// Single loop execution.
async fn tick(config: &Arc<Config>, db: &Pool<Sqlite>, http: &Arc<Client>) -> Result<()> {
    info!("Role tick");
    let members = http
        .guild_members(Id::new(config.discord.guild_id))
        .limit(3)?
        .await?
        .model()
        .await?;
    for member in &members {
        debug!("Processing user {}", member.user.id);
        let controller: Option<Controller> = sqlx::query_as(sql::GET_CONTROLLER_BY_DISCORD_ID)
            .bind(member.user.id.get().to_string())
            .fetch_optional(db)
            .await?;
        let membership_role = match controller {
            Some(controller) => {
                if controller.home_facility == "ZDV" {
                    &config.discord.roles.home_controller
                } else {
                    &config.discord.roles.visiting_controller
                }
            }
            None => &config.discord.roles.guest,
        };

        info!(
            "{} should have membership role {}",
            member.user.name, membership_role
        );

        // TODO add that role; remove the others

        // ...
    }

    Ok(())
}

// Processing loop.
pub async fn process(config: Arc<Config>, db: Pool<Sqlite>, http: Arc<Client>) {
    sleep(Duration::from_secs(30)).await;
    debug!("Starting roles processing");

    loop {
        if let Err(e) = tick(&config, &db, &http).await {
            error!("Error in roles processing tick: {e}");
        }
        sleep(Duration::from_secs(60 * 5)).await; // 5 minutes
    }
}
