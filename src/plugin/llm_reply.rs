use crate::helper::MessageHelper;
use crate::llm::LlmChatRequest;
use crate::{event::*, plugin::*};
use anyhow::Result;

pub struct LlmReply;

#[serenity::async_trait]
impl Plugin for LlmReply {
    fn name(&self) -> &'static str {
        "llm_reply"
    }

    async fn usage(&self, _ctx: &Context) -> Option<String> {
        None
    }

    async fn handle(&self, ctx: &Context, event: &Event) -> Result<EventHandled> {
        let Event::Message(msg) = event else {
            return Ok(EventHandled::No);
        };

        // Only respond if the message is to the bot
        if !msg.is_to_me(ctx).await? {
            return Ok(EventHandled::No);
        }

        let typing = msg.channel_id.start_typing(ctx.http);

        let cfg = ctx.cfg.read().await;
        let llm_settings = cfg.llm_reply.as_llm_settings();
        let response = LlmChatRequest::from_recent_history(ctx, msg.channel_id, &llm_settings)
            .await?
            .post(ctx)
            .await?;

        msg.reply(ctx.cache_http, response).await?;
        typing.stop();
        Ok(EventHandled::Yes)
    }
}
