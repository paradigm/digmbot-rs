use crate::{event::*, plugin::*};
use anyhow::Result;

pub struct PluginHelp;

#[serenity::async_trait]
impl Plugin for PluginHelp {
    fn name(&self) -> &'static str {
        "Help"
    }

    async fn usage(&self, _cfg: &RwLock<Config>) -> Option<String> {
        None
    }

    async fn init(&self, _ctx: &Context) -> Result<()> {
        Ok(())
    }

    async fn handle(&self, event: &Event) -> Result<EventHandled> {
        let Some((ctx, msg)) = event.is_bot_cmd(";help") else {
            return Ok(EventHandled::No);
        };

        let mut reply = String::new();
        reply.push_str("```\n");
        reply.push_str("Commands:\n");
        for plugin in crate::plugin::plugins() {
            if let Some(usage) = plugin.usage(cfg).await {
                reply.push_str(&usage);
                reply.push('\n');
            }
        }
        reply.push_str("```\n");

        msg.reply(ctx, &reply).await?;
        Ok(EventHandled::Yes)
    }
}
