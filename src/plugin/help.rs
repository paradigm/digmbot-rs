use crate::{event::*, helper::BotSettings, plugin::*};
use anyhow::Result;

pub struct PluginHelp;

#[serenity::async_trait]
impl Plugin for PluginHelp {
    fn name(&self) -> &'static str {
        "Help"
    }

    fn usage(&self) -> Option<&'static str> {
        Some("help - you are here")
    }

    async fn init(&self, _ctx: &Context) -> Result<()> {
        Ok(())
    }

    async fn handle(&self, event: &Event) -> Result<EventHandled> {
        let Some((ctx, msg)) = event.is_bot_cmd("help") else {
            return Ok(EventHandled::No);
        };

        let data = ctx.data.read().await;
        let settings = data.get::<BotSettings>().expect("Bot settings not found");
        let prefix = settings.command_prefix.clone();
        drop(data);
        
        let mut reply = String::new();
        reply.push_str("```\n");
        reply.push_str("Commands:\n");
        for plugin in crate::plugin::plugins() {
            if let Some(usage) = plugin.usage() {
                let formatted = usage
                    .lines()
                    .map(|line| {
                        if line.starts_with('|') {
                            line.to_string()
                        } else {
                            format!("{}{}", prefix, line)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                reply.push_str(&formatted);
                reply.push('\n');
            }
        }
        reply.push_str("```\n");

        msg.reply(ctx, &reply).await?;
        Ok(EventHandled::Yes)
    }
}
