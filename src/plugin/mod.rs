use crate::event::EventHandled;
use anyhow::Result;
use serenity::all::Context;

mod debug;
mod help;
mod history;
mod llm;
mod ready;
mod rickroll;
mod vc_notify;
mod xkcd;

#[serenity::async_trait]
pub trait Plugin: Sync + Send {
    /// Plugin name.  Used for debug
    fn name(&self) -> &'static str;
    /// Help message line.  None if no help message
    fn usage(&self) -> Option<&'static str>;
    /// Initialize state information
    async fn init(&self, ctx: &Context) -> Result<()>;
    /// Potentially handle event.  Returns:
    /// - Ok(EventHandled::Yes) if the event has been handled and no other plugin should attempt to
    /// handle it
    /// - Ok(EventHandled::No) if another plugin should attempt to handle the event
    /// - Err if an error occurred
    async fn handle(&self, event: &crate::event::Event) -> Result<EventHandled>;
}

/// Ordered list of available plugins
pub fn plugins() -> Vec<Box<dyn Plugin>> {
    use crate::plugin::*;

    vec![
        // Core bot operations
        Box::new(debug::PluginDebug),
        Box::new(ready::PluginReady),
        Box::new(history::PluginHistory),
        Box::new(help::PluginHelp),
        // Random stuff
        Box::new(xkcd::PluginXkcd),
        Box::new(rickroll::PluginRickRoll),
        // Voice Chat
        Box::new(vc_notify::PluginVcNotify),
        // LLM fallback, used if no other plugin handles the event.
        // Keep last.
        Box::new(llm::PluginLlm),
    ]
}
