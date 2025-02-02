use crate::{event::*, helper::MessageHelper, log_internal, logging::*, plugin::*};
use anyhow::{anyhow, Result};
use serenity::{
    all::{ChannelId, Message, UserId},
    builder::GetMessages,
    prelude::TypeMapKey,
};
use std::collections::HashMap;

/// How many messages the bot should backfill when it first sees a channel.
// const BACKFILL_HISTORY_LIMIT: u8 = 50;
const BACKFILL_HISTORY_LIMIT: u8 = 0;
/// Total number of historical messages the bot should retain.
const HISTORY_MAX_MESSAGES: usize = 50;

/// Initializes and maintains room history
pub struct PluginHistory;

#[serenity::async_trait]
impl Plugin for PluginHistory {
    fn name(&self) -> &'static str {
        "History"
    }

    fn usage(&self) -> Option<&'static str> {
        None
    }

    async fn init(&self, ctx: &Context) -> Result<()> {
        // Given activity likely occurs when the bot is off, it doesn't make sense to save room
        // history to disk.  Instead, we dynamically backfill when the bot becomes interested in a
        // channel for the first time in a given session, then dynamically update
        ctx.data.write().await.insert::<History>(History::new());
        Ok(())
    }

    async fn handle(&self, event: &Event) -> Result<EventHandled> {
        let Event::Message { ctx, msg } = event else {
            return Ok(EventHandled::No);
        };

        let mut state = ctx.data.write().await;
        let history = state
            .get_mut::<History>()
            .ok_or(anyhow!("History uninitialized"))?;
        history.push(ctx, msg).await?;
        drop(state);

        // Allow other plugins to consume this event
        Ok(EventHandled::No)
    }
}

// Per-channel message history
pub struct History(HashMap<ChannelId, Vec<HistoryEntry>>);

#[derive(Clone)]
pub struct HistoryEntry {
    pub author_id: UserId,
    /// Message content formatted in a human and LLM-readable format, similar to IRC
    /// Useful for:
    /// - Human-readable debug logs
    /// - Simple LLM workflows
    ///
    /// Information needed to do things like mention (i.e. `@`-ping) has been scrubbed.
    pub human_format: String,
}

// Serenity crate system to support this type in the Serenity Context
impl TypeMapKey for History {
    type Value = History;
}

impl History {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub async fn get_mut(
        &mut self,
        ctx: &Context,
        channel_id: ChannelId,
    ) -> Result<&mut Vec<HistoryEntry>> {
        match self.0.entry(channel_id) {
            std::collections::hash_map::Entry::Occupied(o) => Ok(o.into_mut()),
            std::collections::hash_map::Entry::Vacant(v) => {
                log_internal!(
                    "Backfilling \"{}\" history... ",
                    channel_id.color(ctx).await
                );

                // Backfill
                // Ignore errors here.  May be serenity crate bug?
                let backfill_messages = channel_id
                    .messages(ctx, GetMessages::new().limit(BACKFILL_HISTORY_LIMIT))
                    .await
                    .unwrap_or_default();

                let mut messages = Vec::new();
                // Iterate in reverse order so the messages are in chronological order
                for msg in backfill_messages.iter().rev() {
                    let author_id = msg.author.id;
                    let human_format = format!(
                        "{}: {}",
                        msg.author.display_name(),
                        msg.human_format_content(ctx).await
                    );
                    let entry = HistoryEntry {
                        author_id,
                        human_format,
                    };
                    messages.push(entry);
                }

                log_internal!(
                    "Backfilling \"{}\" history... done",
                    channel_id.color(ctx).await
                );
                Ok(v.insert(messages))
            }
        }
    }

    // Needs to be mut to backfill
    pub async fn get(
        &mut self,
        ctx: &Context,
        channel_id: ChannelId,
    ) -> Result<&Vec<HistoryEntry>> {
        let messages = self.get_mut(ctx, channel_id).await?;
        Ok(messages)
    }

    pub async fn push(&mut self, ctx: &Context, msg: &Message) -> Result<()> {
        let channel_id = msg.channel_id;
        let author_id = msg.author.id;
        let human_format = format!(
            "{}: {}",
            msg.author.display_name(),
            msg.human_format_content(ctx).await
        );
        let entry = HistoryEntry {
            author_id,
            human_format,
        };

        let history = self.get_mut(ctx, channel_id).await?;
        history.push(entry);

        while history.len() > HISTORY_MAX_MESSAGES {
            history.remove(0);
        }

        Ok(())
    }
}
