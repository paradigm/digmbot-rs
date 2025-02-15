use crate::{
    context::Context,
    event::{Event, EventHandled},
};
use anyhow::Result;

mod debug;
mod help;
mod history;
mod ignore_bots;
mod llm_reply;
mod music;
mod react;
mod reload;
mod rivals_rating;
mod vc_notify;
mod xkcd;

#[serenity::async_trait]
pub trait Plugin: Sync + Send {
    /// Plugin name.  Used for debug
    fn name(&self) -> &'static str;
    /// Plugin usage description in help.  None if no help message
    async fn usage(&self, ctx: &Context) -> Option<String>;
    /// Potentially handle event.  Returns:
    /// - Ok(EventHandled::Yes) if the event has been handled and no other plugin should attempt to
    /// handle it
    /// - Ok(EventHandled::No) if another plugin should attempt to handle the event
    /// - Err if an error occurred
    async fn handle(&self, ctx: &Context, event: &Event) -> Result<EventHandled>;
}

/// Ordered list of available plugins
pub fn plugins() -> Vec<Box<dyn Plugin>> {
    use crate::plugin::*;

    vec![
        // Core bot operations
        Box::new(debug::Debug),
        Box::new(history::History),
        // In order to avoid two bots triggering each other into spam, we consider bot created
        // messages "handled" at this point such that they don't activate any following plugins.
        Box::new(ignore_bots::IgnoreBots),
        // Miscellaneous plugins
        Box::new(help::Help),
        Box::new(xkcd::Xkcd),
        Box::new(music::Music),
        Box::new(reload::Reload),
        Box::new(vc_notify::VcNotify),
        Box::new(rivals_rating::RivalsRating),
        // Generic responses, used if no other plugin handles the event.
        // Keep last.
        Box::new(llm_reply::LlmReply),
        Box::new(react::React),
    ]
}
