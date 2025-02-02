use crate::{event::*, helper::*, plugin::*};
use anyhow::Result;
use serenity::all::Context;

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
                let user = ctx.cache.current_user().clone();
                let username = user.name.as_str();

                println!(
                    "* Connected {} server(s) as {}",
                    ready.guilds.len(),
                    username
                );
            }
            Event::Message { ctx, msg } => {
                println!(
                    "* {}::{}::{}",
                    msg.guild_name(ctx),
                    msg.channel_name(ctx).await,
                    msg.human_format(ctx).await,
                );
            }
            Event::VoiceStateUpdate { ctx, old, new } => match (old, new.channel_id) {
                (Some(old), Some(new_id)) if old.channel_id == Some(new_id) => {
                    // State change within same channel, e.g. mute/unmute
                    // Not currently debug logging this
                }
                (Some(old), Some(_)) => println!(
                    "* {} moved VC channel from \"{}\" to \"{}\"",
                    new.user_id.name(ctx).await,
                    old.channel_name(ctx).await,
                    new.channel_name(ctx).await,
                ),
                (Some(old), None) => println!(
                    "* {} left VC channel \"{}\"",
                    new.user_id.name(ctx).await,
                    old.channel_name(ctx).await
                ),
                (None, Some(_)) => println!(
                    "* {} joined VC channel \"{}\"",
                    new.user_id.name(ctx).await,
                    new.channel_name(ctx).await,
                ),
                (None, None) => println!("* Unknown voice state update"),
            },
            Event::ReactionAdd { ctx, reaction } => {
                let username = if let Some(id) = reaction.user_id {
                    id.name(ctx).await
                } else {
                    "<unknown-user>".to_string()
                };

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

                println!(
                    "* {} reacted to message \"{}\" with \"{}\"",
                    username, message, emoji
                );
            }
            Event::ReactionRemove { ctx, reaction } => {
                let username = if let Some(id) = reaction.user_id {
                    id.name(ctx).await
                } else {
                    "<unknown-user>".to_string()
                };

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

                println!(
                    "* {} removed reaction \"{}\" from message \"{}\"",
                    username, emoji, message
                );
            }
        }

        Ok(EventHandled::No)
    }
}
