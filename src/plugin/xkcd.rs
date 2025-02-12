use crate::{event::*, plugin::*};
use anyhow::Result;

pub struct Xkcd;

#[serenity::async_trait]
impl Plugin for Xkcd {
    fn name(&self) -> &'static str {
        "xkcd"
    }

    async fn usage(&self, ctx: &Context) -> Option<String> {
        let prefix = &ctx.cfg.read().await.general.command_prefix;
        Some(format!(
            "{}{} - show random xkcd comic",
            prefix,
            self.name()
        ))
    }

    async fn handle(&self, ctx: &Context, event: &Event) -> Result<EventHandled> {
        let Some((msg, _)) = event.is_bot_cmd(ctx, self.name()).await else {
            return Ok(EventHandled::No);
        };

        const XKCD_RANDOM_URL: &str = "https://xkcd.com/221/";
        msg.reply(ctx.cache_http, XKCD_RANDOM_URL).await?;
        Ok(EventHandled::Yes)
    }
}
