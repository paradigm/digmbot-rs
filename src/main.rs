mod config;
mod context;
mod event;
mod handler;
mod helper;
mod llm;
mod logging;
mod persistent_state;
mod plugin;
mod volatile_state;

use serenity::{all::GatewayIntents, Client};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = crate::config::Config::load().await?;
    let token = cfg.general.discord_token.clone();
    let pstate = crate::persistent_state::PersistentState::load().await?;
    let vstate = crate::volatile_state::VolatileState::new().await;
    let handler = handler::Handler::new(cfg, pstate, vstate);

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
        .event_handler(handler)
        .await?
        .start()
        .await
        .map_err(Into::into)
}
