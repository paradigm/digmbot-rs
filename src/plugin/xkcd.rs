use crate::{event::*, plugin::*};
use anyhow::Result;

pub struct PluginXkcd;

#[serenity::async_trait]
impl Plugin for PluginXkcd {
    fn name(&self) -> &'static str {
        "Xkcd"
    }

    fn usage(&self) -> Option<&'static str> {
        Some("xkcd - show random xkcd comic")
    }

    async fn init(&self, _ctx: &Context) -> Result<()> {
        Ok(())
    }

    async fn handle(&self, event: &Event) -> Result<EventHandled> {
        let Some((ctx, msg)) = event.is_bot_cmd("xkcd") else {
            return Ok(EventHandled::No);
        };

        const XKCD_RANDOM_URL: &str = "https://xkcd.com/221/";
        msg.reply(&ctx, XKCD_RANDOM_URL).await?;
        Ok(EventHandled::Yes)
    }
}
