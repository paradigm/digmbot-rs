use crate::{event::*, plugin::*};
use anyhow::Result;
use tokio::sync::RwLock;
use crate::config::Config;

/// Initializes state when the connection to Discord is ready.
pub struct PluginReady;

#[serenity::async_trait]
impl Plugin for PluginReady {
    fn name(&self) -> &'static str {
        "Ready"
    }

    fn usage(&self, _cfg: &RwLock<Config>) -> Option<String> {
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
