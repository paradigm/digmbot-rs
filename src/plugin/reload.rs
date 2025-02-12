use crate::helper::MessageHelper;
use crate::llm::LlmChatRequest;
use crate::{event::*, plugin::*};
use anyhow::Result;
use std::borrow::Cow;

pub struct Reload;

#[serenity::async_trait]
impl Plugin for Reload {
    fn name(&self) -> &'static str {
        "reload"
    }

    async fn usage(&self, ctx: &Context) -> Option<String> {
        let prefix = &ctx.cfg.read().await.general.command_prefix;
        Some(format!(
            "{}{} - reload config (bot owner only)",
            prefix,
            self.name()
        ))
    }

    async fn handle(&self, ctx: &Context, event: &Event) -> Result<EventHandled> {
        let Some((msg, _)) = event.is_bot_cmd(ctx, self.name()).await else {
            return Ok(EventHandled::No);
        };

        let response = if msg.is_from_owner(ctx).await {
            ctx.cfg.write().await.reload().await?;
            Cow::Borrowed("Configuration reloaded successfully")
        } else {
            let typing = msg.channel_id.start_typing(ctx.http);
            let cfg = ctx.cfg.read().await;
            let llm_settings = cfg.llm_permission_denied.as_llm_settings();
            let response = LlmChatRequest::from_recent_history(ctx, msg.channel_id, &llm_settings)
                .await?
                .post(ctx)
                .await
                .map(Cow::Owned)?;
            typing.stop();
            response
        };

        msg.reply(ctx.cache_http, response).await?;
        Ok(EventHandled::Yes)
    }
}
