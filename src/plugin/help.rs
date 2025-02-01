use crate::{event::*, plugin::*};
use anyhow::Result;

pub struct PluginHelp;

#[serenity::async_trait]
impl Plugin for PluginHelp {
    fn name(&self) -> &'static str {
        "Help"
    }

    fn usage(&self) -> Option<&'static str> {
        Some(";help - you are here")
    }

    async fn init(&self, ctx: &Context) -> Result<()> {
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
            if let Some(usage) = plugin.usage() {
                reply.push_str(usage);
                reply.push('\n');
            }
        }
        reply.push_str("```\n");

        msg.reply(ctx, &reply).await?;
        Ok(EventHandled::Yes)
    }
}
