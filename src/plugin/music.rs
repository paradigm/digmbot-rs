use crate::{event::*, plugin::*};
use anyhow::Result;

pub struct PluginMusic;

#[serenity::async_trait]
impl Plugin for PluginMusic {
    fn name(&self) -> &'static str {
        "Music"
    }

    fn usage(&self) -> Option<&'static str> {
        Some(";music - fetch random music from YouTube")
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
