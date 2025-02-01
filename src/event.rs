//! The Serenity crate we're using for the Discord API is designed around callbacks to handle
//! events.  However, this does not mesh well with our plugin framework here.  To resolve this,
//! this module translates the callbacks a distinct Event enum.

use serenity::{
    all::{EventHandler, Message, Reaction, Ready, VoiceState},
    client::Context,
};

/// A Discord event
pub enum Event {
    Handler,
    Ready {
        ctx: Context,
        ready: Ready,
    },
    Message {
        ctx: Context,
        msg: Message,
    },
    VoiceStateUpdate {
        ctx: Context,
        old: Option<VoiceState>,
        new: VoiceState,
    },
    ReactionAdd {
        ctx: Context,
        reaction: Reaction,
    },
    ReactionRemove {
        ctx: Context,
        reaction: Reaction,
    },
}

#[serenity::async_trait]
impl EventHandler for Event {
    async fn ready(&self, ctx: Context, ready: Ready) {
        Event::Ready { ctx, ready }.handle().await;
    }

    async fn message(&self, ctx: Context, msg: Message) {
        Event::Message { ctx, msg }.handle().await;
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        Event::VoiceStateUpdate { ctx, old, new }.handle().await;
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        Event::ReactionAdd { ctx, reaction }.handle().await;
    }

    async fn reaction_remove(&self, ctx: Context, reaction: Reaction) {
        Event::ReactionRemove { ctx, reaction }.handle().await;
    }
}

impl Event {
    // When an event occurs, iterate over all the plugins to see if any can/should handle it.
    pub async fn handle(self) {
        if let Event::Ready { ctx, ready } = &self {
            // Serenity crate ctx is ready to be used.  Initialize plugin states.
            for plugin in crate::plugin::plugins() {
                plugin.init(ctx).await;
            }
        }

        // Have plugin handle event
        for plugin in crate::plugin::plugins() {
            match plugin.handle(&self).await {
                Ok(EventHandled::Yes) => return,
                Ok(EventHandled::No) => continue,
                Err(err) => eprintln!("Error in plugin {}: {}", plugin.name(), err),
            }
        }
    }

    // Check if a message should be interpreted as a special bot command.
    //
    // These are typically prefixed with a semicolon, e. g. `;cmd foo bar baz`.
    pub fn is_bot_cmd(&self, cmd: &str) -> Option<(&Context, &Message)> {
        match self {
            Event::Message { ctx, msg }
                if msg.content.split_ascii_whitespace().next() == Some(cmd) =>
            {
                Some((ctx, msg))
            }
            _ => None,
        }
    }
}

pub enum EventHandled {
    Yes,
    No,
}
