use std::sync::Arc;

use anyhow::Result;
use log::info;
use sqlx::{Pool, Sqlite};
use twilight_gateway::Event;
use twilight_http::{client::InteractionClient, Client};
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::interaction::{InteractionData, InteractionType},
    channel::message::MessageFlags,
    http::interaction::InteractionResponse,
    id::Id,
};
use twilight_util::builder::InteractionResponseDataBuilder;
use vzdv::{
    config::Config,
    controller_can_see,
    sql::{self, Controller},
};

#[derive(Debug, CommandModel, CreateCommand)]
#[command(name = "event", desc = "Post event info or positions")]
pub struct EventCommand;

fn quick_resp(message: &str) -> InteractionResponse {
    InteractionResponse {
        kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
        data: Some(
            InteractionResponseDataBuilder::new()
                .flags(MessageFlags::EPHEMERAL)
                .content(message)
                .build(),
        ),
    }
}

/// Command handler.
pub async fn handler(
    event: &Event,
    config: &Arc<Config>,
    db: &Pool<Sqlite>,
    http: &Arc<Client>,
    bot_id: u64,
) -> Result<()> {
    if let Event::InteractionCreate(event) = event {
        if event.kind != InteractionType::ApplicationCommand {
            return Ok(());
        }
        let interaction = http.interaction(Id::new(bot_id));
        if let InteractionData::ApplicationCommand(app_command) = &event.0.data.as_ref().unwrap() {
            info!("Application command: {}", &app_command.name);
            let user_id = match event.author_id() {
                Some(id) => id,
                None => {
                    // I don't know when this would be triggered
                    interaction
                        .create_response(
                            event.id,
                            &event.token,
                            &quick_resp("Discord isn't sharing your user ID"),
                        )
                        .await?;
                    return Ok(());
                }
            };
            let controller: Option<Controller> = sqlx::query_as(sql::GET_CONTROLLER_BY_DISCORD_ID)
                .bind(user_id.get().to_string())
                .fetch_optional(db)
                .await?;
            let controller = match controller {
                Some(c) => c,
                None => {
                    // unknown user
                    interaction
                        .create_response(
                            event.id,
                            &event.token,
                            &quick_resp("You have not linked your Discord to the website"),
                        )
                        .await?;
                    return Ok(());
                }
            };
            if !controller_can_see(&Some(controller), vzdv::PermissionsGroup::EventsTeam) {
                // insufficient permissions
                interaction
                    .create_response(
                        event.id,
                        &event.token,
                        &quick_resp("This command is for event staff"),
                    )
                    .await?;
                return Ok(());
            }

            // ...

            interaction.create_response(
                event.id,
                &event.token,
                &InteractionResponse {
                    kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseDataBuilder::new().content("ACK").build())
                }
            ).await?;
        }
    }

    Ok(())
}
