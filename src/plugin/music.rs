use crate::{event::*, plugin::*};
use anyhow::Result;
use tokio::sync::RwLock;
use crate::config::Config;

pub struct PluginMusic;

#[serenity::async_trait]
impl Plugin for PluginMusic {
    fn name(&self) -> &'static str {
        "Music"
    }

    fn usage(&self, cfg: &RwLock<Config>) -> Option<String> {
        let prefix = &cfg.read().await.general.command_prefix;
        Some(format!("{}music - fetch random music from YouTube", prefix))
    }

    async fn init(&self, _ctx: &Context) -> Result<()> {
        Ok(())
    }

    async fn handle(&self, event: &Event) -> Result<EventHandled> {
        let Some((ctx, msg)) = event.is_bot_cmd(";music") else {
            return Ok(EventHandled::No);
        };

        const MUSIC_URL: &str = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
        msg.reply(&ctx, MUSIC_URL).await?;
        Ok(EventHandled::Yes)
    }
}
