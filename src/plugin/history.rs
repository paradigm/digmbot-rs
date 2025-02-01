use crate::{event::*, helper::MessageHelper, plugin::*};
use anyhow::{anyhow, Result};
use serenity::{
    all::{ChannelId, Message, UserId},
    builder::GetMessages,
    prelude::TypeMapKey,
};
use std::collections::{hash_map, HashMap};

/// How many messages the bot should backfill when it first sees a channel.
const BACKFILL_HISTORY_LIMIT: u8 = 50;
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

        let channel_id = msg.channel_id;
        let mut state = ctx.data.write().await;
        let mut history = state
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
    pub content: String,
    /// Message formatted similarly to IRC
    /// Useful for:
    /// - Human-readable, e.g. debugging
    /// - Simple LLM workflows, although the LLM will not know how to mention (i.e. `@`-ping)
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
                // Backfill
                // Ignore errors here.  May be serenity crate bug?
                let mut backfill_messages = channel_id
                    .messages(ctx, GetMessages::new().limit(BACKFILL_HISTORY_LIMIT))
                    .await
                    .unwrap_or_default();

                let mut messages = Vec::new();
                // Iterate in reverse order so the messages are in chronological order
                for msg in backfill_messages.iter().rev() {
                    let author_id = msg.author.id;
                    let content = msg.content.clone();
                    let human_format = msg.human_format(ctx).await;
                    let entry = HistoryEntry {
                        author_id,
                        content,
                        human_format,
                    };
                    messages.push(entry);
                }

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
        let mut messages = self.get_mut(ctx, channel_id).await?;
        Ok(messages)
    }

    pub async fn push(&mut self, ctx: &Context, msg: &Message) -> Result<()> {
        let channel_id = msg.channel_id;
        let author_id = msg.author.id;
        let content = msg.content.clone();
        let human_format = msg.human_format(ctx).await;
        let entry = HistoryEntry {
            author_id,
            content,
            human_format,
        };

        let history = self.get_mut(ctx, channel_id).await?;
        history.push(entry);

        while history.len() > HISTORY_MAX_MESSAGES {
            history.remove(0);
        }

        Ok(())
    }

    // Needs to be mut to backfill
    pub async fn human_format_concat(
        &mut self,
        ctx: &Context,
        channel_id: ChannelId,
    ) -> Result<String> {
        Ok(self
            .get_mut(ctx, channel_id)
            .await?
            .iter()
            .map(|entry| entry.human_format.as_str())
            .collect::<Vec<&str>>()
            .join("\n"))
    }
}
