use crate::{event::*, plugin::*};
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

    async fn init(&self, _ctx: &Context) -> Result<()> {
        Ok(())
    }

    async fn handle(&self, event: &Event) -> Result<EventHandled> {
        let Event::Ready { ctx: _, ready: _ } = event else {
            return Ok(EventHandled::No);
        };

        // Connected to server
        Ok(EventHandled::Yes)
    }
}
