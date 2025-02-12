use crate::{event::*, plugin::*};
use anyhow::Result;

/// Initializes and maintains room history
pub struct History;

#[serenity::async_trait]
impl Plugin for History {
    fn name(&self) -> &'static str {
        "history"
    }

    async fn usage(&self, _ctx: &Context) -> Option<String> {
        None
    }

    async fn handle(&self, ctx: &Context, event: &Event) -> Result<EventHandled> {
        let Event::Message(msg) = event else {
            return Ok(EventHandled::No);
        };

        ctx.vstate.write().await.history.push(ctx, msg).await?;

        Ok(EventHandled::No)
    }
}
