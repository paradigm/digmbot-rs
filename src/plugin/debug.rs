use crate::{event::*, helper::*, log_event, logging::*, plugin::*};
use anyhow::Result;
use serenity::all::Context;
use std::borrow::Cow;

/// Prints debug information about event to stdout
pub struct PluginDebug;

#[serenity::async_trait]
impl Plugin for PluginDebug {
    fn name(&self) -> &'static str {
        "Debug"
    }

    fn usage(&self) -> Option<&'static str> {
        None
    }

    async fn init(&self, _ctx: &Context) -> Result<()> {
        Ok(())
    }

    async fn handle(&self, event: &Event) -> Result<EventHandled> {
        match event {
            Event::Handler => {}
            Event::Ready { ctx, ready } => {
                log_event!(
                    "Connected to {} server(s) as {}",
                    ready.guilds.len(),
                    ctx.cache.current_user().color(),
                );
            }
            Event::Message { ctx, msg } => {
                log_event!(
                    "{}{}{}{}{}{} {}",
                    msg.guild_id.color(ctx).await,
                    Glue {}.color(),
                    msg.channel_id.color(ctx).await,
                    Glue {}.color(),
                    msg.author.color(),
                    Glue {}.color(),
                    msg.human_format_content(ctx).await,
                );
            }
            Event::VoiceStateUpdate { ctx, old, new } => match (old, new.channel_id) {
                (Some(old), Some(new_id)) if old.channel_id == Some(new_id) => {
                    // State change within same channel, e.g. mute/unmute
                    // Not currently debug logging this
                }
                (Some(old), Some(_)) => log_event!(
                    "{} moved VC channel from \"{}\" to \"{}\"",
                    new.user_id.color(ctx).await,
                    old.channel_id.color(ctx).await,
                    new.channel_id.color(ctx).await,
                ),
                (Some(old), None) => log_event!(
                    "{} left VC channel \"{}\"",
                    new.user_id.color(ctx).await,
                    old.channel_id.color(ctx).await,
                ),
                (None, Some(_)) => log_event!(
                    "{} joined VC channel \"{}\"",
                    new.user_id.color(ctx).await,
                    new.channel_id.color(ctx).await,
                ),
                (None, None) => log_event!("Unknown voice state update"),
            },
            Event::ReactionAdd { ctx, reaction } => {
                let message = match reaction.message(ctx).await {
                    Ok(msg) => Cow::Owned(msg.human_format_content(ctx).await),
                    Err(_) => Cow::Borrowed("<unknown-message>"),
                };

                let emoji = match &reaction.emoji {
                    serenity::all::ReactionType::Custom {
                        animated: _,
                        id: _,
                        name,
                    } => name.clone().unwrap_or("<unknown-emoji>".to_owned()),
                    serenity::all::ReactionType::Unicode(s) => s.clone(),
                    _ => "<unknown-emoji>".to_owned(),
                };

                log_event!(
                    "{} reacted to message \"{}\" with \"{}\"",
                    reaction.user_id.color(ctx).await,
                    message,
                    emoji
                );
            }
            Event::ReactionRemove { ctx, reaction } => {
                let message = reaction
                    .message(ctx)
                    .await
                    .map(|msg| msg.content.clone())
                    .unwrap_or("<unknown-message>".to_string());

                let emoji = match &reaction.emoji {
                    serenity::all::ReactionType::Custom {
                        animated: _,
                        id: _,
                        name,
                    } => name.clone().unwrap_or("<unknown-emoji>".to_owned()),
                    serenity::all::ReactionType::Unicode(s) => s.clone(),
                    _ => "<unknown-emoji>".to_owned(),
                };

                log_event!(
                    "{} removed reaction \"{}\" from message \"{}\"",
                    reaction.user_id.color(ctx).await,
                    emoji,
                    message
                );
            }
        }

        Ok(EventHandled::No)
    }
}
