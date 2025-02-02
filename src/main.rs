use crate::helper::PathHelper;
use anyhow::anyhow;

mod event;
mod helper;
mod logging;
mod plugin;

use anyhow::Result;
use serenity::prelude::{Client, GatewayIntents};

#[tokio::main]
async fn main() -> Result<()> {
    let token = helper::config_path("discord_token")?
        .read_to_string()
        .await
        .map_err(|e| anyhow!("Error reading discord_token: {}", e))?;

    // Things we want discord to tell us about.
    let intents = GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::DIRECT_MESSAGE_REACTIONS
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::MESSAGE_CONTENT;

    Client::builder(&token, intents)
        .event_handler(crate::event::Event::Handler)
        .await?
        .start()
        .await
        .map_err(Into::into)
}
