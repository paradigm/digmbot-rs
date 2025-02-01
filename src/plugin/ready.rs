use crate::{event::*, helper::*, plugin::*};
use anyhow::Result;

/// Initializes state when the connection to Discord is ready.
pub struct PluginReady;

#[serenity::async_trait]
impl Plugin for PluginReady {
    fn name(&self) -> &'static str {
        "Ready"
    }

    fn usage(&self) -> Option<&'static str> {
        None
    }

    async fn init(&self, ctx: &Context) -> Result<()> {
        Ok(())
    }

    async fn handle(&self, event: &Event) -> Result<EventHandled> {
        let Event::Ready { ctx, ready } = event else {
            return Ok(EventHandled::No);
        };

        // Connected to server
        Ok(EventHandled::Yes)
    }
}
