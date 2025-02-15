use crate::{event::*, helper::*, plugin::*};
use anyhow::Result;
use serenity::all::ReactionType;

pub struct React;

#[serenity::async_trait]
impl Plugin for React {
    fn name(&self) -> &'static str {
        "react"
    }

    async fn usage(&self, _ctx: &Context) -> Option<String> {
        None
    }

    async fn handle(&self, ctx: &Context, event: &Event) -> Result<EventHandled> {
        let Event::Message(msg) = event else {
            return Ok(EventHandled::No);
        };

        let guild_id = msg.guild_id;
        let bot_id = ctx.cache.current_user().id;
        let bot_name = bot_id.nick_in_guild(ctx, guild_id).await;
        if !msg.content.contains(&bot_name) {
            return Ok(EventHandled::No);
        }

        let reaction = "\u{1F440}".to_owned(); // unicode eyes
        let reaction = ReactionType::Unicode(reaction);
        msg.react(ctx.cache_http, reaction).await?;
        Ok(EventHandled::Yes)
    }
}
