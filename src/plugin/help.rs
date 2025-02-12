use crate::{event::*, plugin::*};
use anyhow::Result;

pub struct Help;

#[serenity::async_trait]
impl Plugin for Help {
    fn name(&self) -> &'static str {
        "help"
    }

    async fn usage(&self, ctx: &Context) -> Option<String> {
        let prefix = &ctx.cfg.read().await.general.command_prefix;
        Some(format!(
            "{}{} - show this help message",
            prefix,
            self.name()
        ))
    }

    async fn handle(&self, ctx: &Context, event: &Event) -> Result<EventHandled> {
        let Some((msg, _)) = event.is_bot_cmd(ctx, self.name()).await else {
            return Ok(EventHandled::No);
        };

        let mut reply = String::new();
        reply.push_str("```\n");
        reply.push_str("Commands:\n");
        for plugin in crate::plugin::plugins() {
            if let Some(usage) = plugin.usage(ctx).await {
                reply.push_str(&usage);
                reply.push('\n');
            }
        }
        reply.push_str("```\n");

        msg.reply(ctx.cache_http, &reply).await?;
        Ok(EventHandled::Yes)
    }
}
