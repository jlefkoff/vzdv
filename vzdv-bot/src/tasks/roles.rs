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
    ControllerRating,
};

/// Resolve the guild member's roles, adding and removing as necessary.
async fn resolve_roles(user_id: u64, roles: &[(&str, bool)], http: &Arc<Client>) -> Result<()> {
    todo!()
}

/// Single loop execution.
async fn tick(config: &Arc<Config>, db: &Pool<Sqlite>, http: &Arc<Client>) -> Result<()> {
    info!("Role tick");
    let guild_id = Id::new(config.discord.guild_id);
    let members = http
        .guild_members(guild_id)
        .limit(3)?
        .await?
        .model()
        .await?;
    for member in &members {
        debug!("Processing user {}", member.user.id);
        let mut to_resolve: Vec<(&str, bool)> = Vec::new();
        let controller: Option<Controller> = sqlx::query_as(sql::GET_CONTROLLER_BY_DISCORD_ID)
            .bind(member.user.id.get().to_string())
            .fetch_optional(db)
            .await?;
        let controller = match controller {
            Some(c) => c,
            None => {
                // no linked controller; strip all roles
                info!(
                    "No linked controller record; stripping all roles from {} ({})",
                    member.nick.as_ref().unwrap_or(&member.user.name),
                    member.user.id.get()
                );
                for role in &member.roles {
                    http.remove_guild_member_role(guild_id, member.user.id, *role)
                        .await?;
                }
                return Ok(());
            }
        };

        // membership
        to_resolve.push((
            &config.discord.roles.home_controller,
            controller.home_facility == "ZDV",
        ));
        to_resolve.push((
            &config.discord.roles.visiting_controller,
            controller.is_on_roster && controller.home_facility != "ZDV",
        ));
        to_resolve.push((&config.discord.roles.guest, !controller.is_on_roster));

        // network rating
        to_resolve.push((
            &config.discord.roles.administrator,
            controller.rating == ControllerRating::ADM.as_id(),
        ));
        to_resolve.push((
            &config.discord.roles.supervisor,
            controller.rating == ControllerRating::SUP.as_id(),
        ));
        to_resolve.push((
            &config.discord.roles.instructor_3,
            controller.rating == ControllerRating::I3.as_id(),
        ));
        to_resolve.push((
            &config.discord.roles.instructor_1,
            controller.rating == ControllerRating::I1.as_id(),
        ));
        to_resolve.push((
            &config.discord.roles.controller_3,
            controller.rating == ControllerRating::C3.as_id(),
        ));
        to_resolve.push((
            &config.discord.roles.controller_1,
            controller.rating == ControllerRating::C1.as_id(),
        ));
        to_resolve.push((
            &config.discord.roles.student_3,
            controller.rating == ControllerRating::S3.as_id(),
        ));
        to_resolve.push((
            &config.discord.roles.student_2,
            controller.rating == ControllerRating::S2.as_id(),
        ));
        to_resolve.push((
            &config.discord.roles.student_1,
            controller.rating == ControllerRating::S1.as_id(),
        ));
        to_resolve.push((
            &config.discord.roles.observer,
            controller.rating == ControllerRating::OBS.as_id(),
        ));

        // staff
        if ["ATM", "DATM", "TA"]
            .iter()
            .any(|role| controller.roles.contains(role))
        {
            to_resolve.push((&config.discord.roles.sr_staff, true));
            to_resolve.push((&config.discord.roles.jr_staff, false));
        } else if ["EC", "FE", "WM"]
            .iter()
            .any(|role| controller.roles.contains(role))
        {
            to_resolve.push((&config.discord.roles.sr_staff, false));
            to_resolve.push((&config.discord.roles.jr_staff, true));
        } else {
            to_resolve.push((&config.discord.roles.sr_staff, false));
            to_resolve.push((&config.discord.roles.jr_staff, false));
        }
        // Note: probably will let "staff teams" be manually assigned, same with VATUSA/VATGOV

        if let Err(e) = resolve_roles(member.user.id.get(), &to_resolve, http).await {
            error!(
                "Error resolving roles for {} ({}): {e}",
                member.nick.as_ref().unwrap_or(&member.user.name),
                member.user.id.get()
            );
        }

        // TODO set nickname
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
