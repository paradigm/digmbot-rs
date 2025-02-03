use crate::{event::*, helper::*, plugin::*};
use anyhow::{anyhow, Result};
use serenity::{
    all::{CreateMessage, GuildId, UserId, VoiceState},
    prelude::TypeMapKey,
};
use std::{
    collections::{HashMap, HashSet},
    io::ErrorKind,
    time::{Duration, Instant},
};

// Maximum rate to notify users. This avoids spam should someone repeatedly join and leave VC.
const NOTIFY_RATE_LIMIT: Duration = Duration::from_secs(60 * 10);

// Manage subscriptions
pub struct PluginVcNotify;

enum VcNotifyCmd {
    Follow,
    Unfollow,
}

#[serenity::async_trait]
impl Plugin for PluginVcNotify {
    fn name(&self) -> &'static str {
        "VcNotify"
    }

    fn usage(&self) -> Option<&'static str> {
        Some(
            "vc-notify <follow/unfollow> [server-id] - voice channel activity notifications\n\
             |  If messaging in server, [server-id] is derived from context.\n\
             |  If DMing rather than messaging in-server, requires [server-id].\n\
             |  Right-click on server icon > copy ID to get server-id",
        )
    }

    async fn init(&self, ctx: &Context) -> Result<()> {
        let state = VcNotifyState {
            subscribers: VcNotifySubscribers::load().await?,
            notified_timestamp: HashMap::new(),
        };

        ctx.data.write().await.insert::<VcNotifyState>(state);

        Ok(())
    }

    async fn handle(&self, event: &Event) -> Result<EventHandled> {
        match event {
            Event::Message { ctx: _, msg: _ } => handle_event(event).await,
            Event::VoiceStateUpdate { ctx, old, new } => {
                handle_voice_state_update(ctx, old, new).await
            }
            _ => Ok(EventHandled::No),
        }
    }
}

async fn handle_event(event: &Event) -> Result<EventHandled> {
    let Some((ctx, msg)) = event.is_bot_cmd("vc-notify") else {
        return Ok(EventHandled::No);
    };
    
    let terms: Vec<&str> = msg.content.split_whitespace().collect();
    let cmd = match terms.get(1) {
        Some(&"follow") => VcNotifyCmd::Follow,
        Some(&"unfollow") => VcNotifyCmd::Unfollow,
        _ => {
            msg.reply(ctx, "Invalid command.  See `;help`").await?;
            return Ok(EventHandled::Yes);
        }
    };

    let guild_id = match terms.get(2) {
        Some(id) => match id.parse::<GuildId>() {
            Ok(id) => id,
            Err(_) => {
                msg.reply(ctx, "Invalid `server-id`, see `;help`").await?;
                return Ok(EventHandled::Yes);
            }
        },
        None => match msg.guild(&ctx.cache).map(|guild| guild.id) {
            Some(guild_id) => guild_id,
            None => {
                msg.reply(ctx, "Missing `server-id`, see `;help`").await?;
                return Ok(EventHandled::Yes);
            }
        },
    };

    // Sanity check guild_id
    let guild_name = guild_id
        .to_guild_cached(&ctx.cache)
        .map(|guild| guild.name.clone());
    let guild_name = match guild_name {
        Some(name) => name,
        None => {
            msg.reply(
                ctx,
                "I am not in that server, and so I can't help you there.",
            )
            .await?;
            return Ok(EventHandled::Yes);
        }
    };

    let mut state = ctx.data.write().await;
    let state = state
        .get_mut::<VcNotifyState>()
        .ok_or(anyhow!("VcNotify uninitialized"))?;

    let result = match cmd {
        VcNotifyCmd::Follow => {
            if state.subscribers.follow(guild_id, msg.author.id).await? {
                msg.reply(ctx, format!(
                        "You have successfully subscribed to voice channel activity notifications for {} ({})",
                        guild_name,
                        guild_id
                    )).await
            } else {
                msg.reply(ctx, format!(
                        "You are already subscribed to voice channel activity notifications for {} ({})",
                        guild_name,
                        guild_id
                    )).await
            }
        }
        VcNotifyCmd::Unfollow => {
            if state.subscribers.unfollow(guild_id, msg.author.id).await? {
                msg.reply(ctx, format!(
                        "You have successfully unsubscribed from voice channel activity notifications for {} ({})",
                        guild_name,
                        guild_id
                    )).await
            } else {
                msg.reply(ctx, format!(
                        "You are not subscribed to voice channel activity notifications for {} ({})",
                        guild_name,
                        guild_id
                    )).await
            }
        }
    };

    result.map(|_| EventHandled::Yes).map_err(Into::into)
}

async fn handle_voice_state_update(
    ctx: &Context,
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
        voice_user_count += channel.members(ctx)?.len();
    }

    // Already users in VC
    if voice_user_count != 1 {
        return Ok(EventHandled::No);
    }

    // Notify registered users
    let state = ctx.data.read().await;
    let state = state
        .get::<VcNotifyState>()
        .ok_or(anyhow!("VcNotify state uninitialized"))?;
    let subscribers = state.subscribers.get_subscribers(&guild_id);
    let channel_name = new
        .channel_id
        .map(|id| format!("<#{}>", id))
        .unwrap_or("a VC channel".to_string());

    let new_user_name = new.user_id.name(ctx).await;
    let message = CreateMessage::new().content(format!(
        "{} joined VC channel {} in {}\n\
            \n\
            You can opt out of these notifications by replying `;vc-notify unfollow {}`\n",
        new_user_name, channel_name, guild.name, guild.id
    ));

    let now = std::time::Instant::now();
    for subscriber_id in subscribers.into_iter() {
        // Don't DM the user who just joined
        if subscriber_id == new.user_id {
            continue;
        }
        // Don't DM the user too often
        if let Some(last) = state.notified_timestamp.get(&subscriber_id) {
            if now.duration_since(*last) < NOTIFY_RATE_LIMIT {
                continue;
            }
        }

        subscriber_id
            .to_user(&ctx.http)
            .await?
            .direct_message(ctx, message.clone())
            .await?;
    }

    // While we handled the event, we did not do so exclusively; other plugins might also want
    // to act on this event.
    Ok(EventHandled::No)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct VcNotifySubscribers(HashMap<GuildId, HashSet<UserId>>);

struct VcNotifyState {
    subscribers: VcNotifySubscribers,
    notified_timestamp: HashMap<UserId, Instant>,
}

// Serenity crate system to support this type in the Serenity Context
impl TypeMapKey for VcNotifyState {
    type Value = VcNotifyState;
}

impl VcNotifySubscribers {
    async fn save(&self) -> Result<()> {
        let serialized = serde_json::to_string(self)
            .map_err(|e| anyhow!("Failed to serialize VcNotify subscribers: {}", e))?;

        config_path("vc_notify.json")?
            .write(&serialized)
            .await
            .map(|_| ())
            .map_err(|e| anyhow!("Failed to write VcNotify subscribers: {}", e))
    }

    async fn load() -> Result<Self> {
        let subscribers = match config_path("vc_notify.json")?.read_to_bytes().await {
            Ok(data) => serde_json::from_slice(&data)
                .map_err(|e| anyhow!("Failed to deserialize VcNotify: {}", e))?,
            Err(e) if e.kind() == ErrorKind::NotFound => HashMap::new(),
            Err(e) => return Err(anyhow!("Failed to read vc_notify.json: {}", e)),
        };
        Ok(Self(subscribers))
    }

    async fn follow(&mut self, guild_id: GuildId, user_id: UserId) -> Result<bool> {
        let made_change = self.0.entry(guild_id).or_default().insert(user_id);

        if made_change {
            self.save().await?;
        }

        Ok(made_change)
    }

    async fn unfollow(&mut self, guild_id: GuildId, user_id: UserId) -> Result<bool> {
        let Some(guild_entry) = self.0.get_mut(&guild_id) else {
            return Ok(false);
        };

        let made_change = guild_entry.remove(&user_id);

        if made_change {
            self.save().await?;
        }

        Ok(made_change)
    }

    fn get_subscribers(&self, guild_id: &GuildId) -> Vec<UserId> {
        self.0
            .get(guild_id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }
}
