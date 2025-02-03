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
    let bot_settings = helper::BotSettings::load().await?;

    let token = helper::config_path("discord_token")?
        .read_to_string()
        .await
        .map_err(|e| anyhow!("Error reading discord_token: {}", e))?;

    let intents = GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::DIRECT_MESSAGE_REACTIONS
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(crate::event::Event::Handler)
        .await?;

    {
        let mut data = client.data.write().await;
        data.insert::<helper::BotSettings>(bot_settings);
    }

    client.start().await.map_err(Into::into)
}
