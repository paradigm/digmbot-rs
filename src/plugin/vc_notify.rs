use crate::helper::UserIdHelper;
use crate::{event::*, plugin::*};
use anyhow::{anyhow, Result};
use serenity::all::{CreateMessage, Message, VoiceState};
use std::borrow::Cow;

pub struct VcNotify;

#[serenity::async_trait]
impl Plugin for VcNotify {
    fn name(&self) -> &'static str {
        "vc-notify"
    }

    async fn usage(&self, ctx: &Context) -> Option<String> {
        let prefix = &ctx.cfg.read().await.general.command_prefix;
        Some(format!(
            "{}{} <follow/unfollow> - voice channel activity notifications",
            prefix,
            self.name(),
        ))
    }

    async fn handle(&self, ctx: &Context, event: &Event) -> Result<EventHandled> {
        match event {
            Event::Message(msg) => handle_message(ctx, msg).await,
            Event::VoiceStateUpdate { old, new } => handle_voice_state_update(ctx, old, new).await,
            _ => Ok(EventHandled::No),
        }
    }
}

async fn handle_message(ctx: &Context<'_>, msg: &Message) -> Result<EventHandled> {
    let cmd_prefix = &ctx.cfg.read().await.general.command_prefix;

    let terms: Vec<&str> = msg.content.split_whitespace().collect();
    if terms.first().and_then(|cmd| cmd.strip_prefix(cmd_prefix)) != Some("vc-notify") {
        return Ok(EventHandled::No);
    }

    let id = msg.author.id;
    let pstate = &mut ctx.pstate.write().await;
    let followers = &mut pstate.vc_notify.followers;
    let following = followers.contains(&id);

    let response = match (terms.get(1), following) {
        (Some(&"follow"), true) => {
            Cow::Borrowed("You are already subscribed to voice channel activity notifications")
        }
        (Some(&"follow"), false) => {
            followers.insert(id);
            pstate.save().await?;
            Cow::Borrowed(
                "You have successfully subscribed to voice channel activity notifications",
            )
        }
        (Some(&"unfollow"), true) => {
            followers.remove(&id);
            pstate.save().await?;
            Cow::Borrowed(
                "You have successfully unsubscribed from voice channel activity notifications",
            )
        }
        (Some(&"unfollow"), false) => {
            Cow::Borrowed("You are not subscribed to voice channel activity notifications")
        }
        _ => Cow::Owned(format!("Invalid command.  See `{}help`", cmd_prefix)),
    };

    msg.reply(ctx.cache_http, response).await?;
    Ok(EventHandled::Yes)
}

async fn handle_voice_state_update(
    ctx: &Context<'_>,
    old: &Option<VoiceState>,
    new: &VoiceState,
) -> Result<EventHandled> {
    let guild_id = new.guild_id.ok_or(anyhow!("unable to get guild_id"))?;
    let guild = guild_id.to_partial_guild(&ctx.http).await?;
    let old_channel_id = old.as_ref().and_then(|o| o.channel_id);
    let afk_channel_id = guild.afk_metadata.as_ref().map(|afk| afk.afk_channel_id);

    // Only care about a newly available person:
    // - New joins to something other than the AFK channel
    // - Someone leaving the AFK channel to a non-AFK channel
    match (old_channel_id, new.channel_id, afk_channel_id) {
        // Someone joined a VC channel, and there is no AFK channel, so it's not that
        (None, Some(_), None) => {}
        // Someone joined a VC channel, and it's not the AFK channel
        (None, Some(new), Some(afk)) if new != afk => {}
        // Someone moved from the AFK channel to another, non-AFK channel
        (Some(old), Some(new), Some(afk)) if old == afk && new != afk => {}
        // Some situation we don't care about such as:
        // - Someone not in VC joining strait to the AFK channel
        // - Someone between (non-AFK) VC channels
        // - Someone leaving VC to the AFK channel
        // - Someone leaving VC entirely
        // - Some non-room-change event like muting/unmuting
        _ => return Ok(EventHandled::No),
    }

    // Check if this user is the only VC user
    let mut voice_user_count = 0;
    for (channel_id, channel) in guild.channels(&ctx.http).await? {
        if channel.kind != serenity::model::channel::ChannelType::Voice {
            continue;
        }
        if Some(channel_id) == afk_channel_id {
            continue;
        }
        voice_user_count += channel.members(ctx.cache_http)?.len();
    }

    // Already users in VC
    if voice_user_count > 1 {
        return Ok(EventHandled::No);
    }

    // Notify registered users
    let pstate = &ctx.pstate.write().await;
    let followers = &pstate.vc_notify.followers;

    let channel_name = new
        .channel_id
        .map(|id| format!("<#{}>", id))
        .unwrap_or("a VC channel".to_string());

    let cmd_prefix = &ctx.cfg.read().await.general.command_prefix;
    let new_user_name = new.user_id.nick_in_guild(ctx, Some(guild_id)).await;
    let message = CreateMessage::new().content(format!(
        "{} joined VC channel {} in {}\n\
            \n\
            You can opt out of these notifications by replying `{}vc-notify unfollow`\n",
        new_user_name, channel_name, guild.name, cmd_prefix
    ));

    let timestamps = &mut ctx.vstate.write().await.notify_timestamp;

    for follower_id in followers.iter() {
        // Don't DM the user who just joined
        if *follower_id == new.user_id {
            continue;
        }
        // Don't DM the user too often
        if !timestamps.okay_to_notify(ctx, *follower_id).await {
            continue;
        }

        timestamps.update_notify_timestamp(*follower_id).await;

        follower_id
            .to_user(&ctx.http)
            .await?
            .direct_message(ctx.cache_http, message.clone())
            .await?;
    }

    // While we handled the event, we did not do so exclusively; other plugins might also want
    // to act on this event.
    Ok(EventHandled::No)
}
