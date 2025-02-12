use crate::{event::*, plugin::*};
use anyhow::Result;

pub struct Music;

#[serenity::async_trait]
impl Plugin for Music {
    fn name(&self) -> &'static str {
        "music"
    }

    async fn usage(&self, ctx: &Context) -> Option<String> {
        let prefix = &ctx.cfg.read().await.general.command_prefix;
        Some(format!(
            "{}{} - fetch random music from YouTube",
            prefix,
            self.name()
        ))
    }

    async fn handle(&self, ctx: &Context, event: &Event) -> Result<EventHandled> {
        let Some((msg, _)) = event.is_bot_cmd(ctx, self.name()).await else {
            return Ok(EventHandled::No);
        };

        const MUSIC_URL: &str = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
        msg.reply(ctx.cache_http, MUSIC_URL).await?;
        Ok(EventHandled::Yes)
    }
}
