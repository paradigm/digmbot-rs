use crate::config::Config;
use crate::{event::*, plugin::*};
use anyhow::Result;
use tokio::sync::RwLock;

pub struct PluginXkcd;

#[serenity::async_trait]
impl Plugin for PluginXkcd {
    fn name(&self) -> &'static str {
        "Xkcd"
    }

    async fn usage(&self, cfg: &RwLock<Config>) -> Option<String> {
        let prefix = &cfg.read().await.general.command_prefix;
        Some(format!("{}xkcd - show random xkcd comic", prefix))
    }

    async fn init(&self, _ctx: &Context) -> Result<()> {
        Ok(())
    }

    async fn handle(&self, event: &Event) -> Result<EventHandled> {
        let Some((ctx, msg)) = event.is_bot_cmd(";xkcd") else {
            return Ok(EventHandled::No);
        };

        const XKCD_RANDOM_URL: &str = "https://xkcd.com/221/";
        msg.reply(&ctx, XKCD_RANDOM_URL).await?;
        Ok(EventHandled::Yes)
    }
}
