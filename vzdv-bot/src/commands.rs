use std::sync::Arc;

use anyhow::Result;
use log::info;
use sqlx::{Pool, Sqlite};
use twilight_gateway::Event;
use twilight_http::Client;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::interaction::{InteractionData, InteractionType},
    http::interaction::InteractionResponse,
    id::Id,
};
use twilight_util::builder::InteractionResponseDataBuilder;
use vzdv::config::Config;

#[derive(Debug, CommandModel, CreateCommand)]
#[command(name = "event", desc = "Post event info or positions")]
pub struct EventCommand;

/// Command handler.
pub async fn handler(
    e: &Event,
    config: &Arc<Config>,
    db: &Pool<Sqlite>,
    http: &Arc<Client>,
    bot_id: u64,
) -> Result<()> {
    if let Event::InteractionCreate(event) = e {
        if event.kind != InteractionType::ApplicationCommand {
            return Ok(());
        }
        let interaction = http.interaction(Id::new(bot_id));
        if let InteractionData::ApplicationCommand(app_command) = &event.0.data.as_ref().unwrap() {
            info!("Application command: {}", &app_command.name);
            interaction.create_response(
                event.id,
                &event.token,
                &InteractionResponse {
                    kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseDataBuilder::new().content("foobar").build())
                }
            ).await?;
        }
    }

    Ok(())
}
