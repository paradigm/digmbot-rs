use crate::{event::*, plugin::*};
use anyhow::Result;

pub struct IgnoreBots;

#[serenity::async_trait]
impl Plugin for IgnoreBots {
    fn name(&self) -> &'static str {
        "ignore_bots"
    }

    async fn usage(&self, _ctx: &Context) -> Option<String> {
        None
    }

    async fn handle(&self, _ctx: &Context, event: &Event) -> Result<EventHandled> {
        let Event::Message(msg) = event else {
            return Ok(EventHandled::No);
        };

        if msg.author.bot {
            Ok(EventHandled::Yes)
        } else {
            Ok(EventHandled::No)
        }
    }
}
