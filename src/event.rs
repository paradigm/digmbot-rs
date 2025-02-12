use crate::context::Context;
use serenity::all::{Message, Reaction, Ready, VoiceState};

/// A Discord event
pub enum Event {
    Ready(Ready),
    Message(Message),
    VoiceStateUpdate {
        old: Option<VoiceState>,
        new: VoiceState,
    },
    ReactionAdd(Reaction),
    ReactionRemove(Reaction),
}

impl Event {
    /// When an event occurs, iterate over all the plugins to see if any can/should handle it.
    pub async fn handle(self, ctx: Context<'_>) {
        for plugin in crate::plugin::plugins() {
            match plugin.handle(&ctx, &self).await {
                Ok(EventHandled::Yes) => return,
                Ok(EventHandled::No) => continue,
                Err(err) => eprintln!("Error in plugin `{}`: {}", plugin.name(), err),
            }
        }
    }

    /// Check if a message should be interpreted as a special bot command.
    ///
    /// If so, returns message and the remaining text after the command.
    pub async fn is_bot_cmd(&self, ctx: &Context<'_>, cmd: &str) -> Option<(&Message, &str)> {
        let Event::Message(msg) = self else {
            return None;
        };

        let cfg = ctx.cfg.read().await;
        let prefix = cfg.general.command_prefix.as_str();
        let content = msg
            .content
            .as_str()
            .strip_prefix(prefix)?
            .strip_prefix(cmd)?;

        Some((msg, content))
    }
}

pub enum EventHandled {
    Yes,
    No,
}
