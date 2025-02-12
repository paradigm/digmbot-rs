use crate::{
    context::Context,
    helper::{MessageHelper, UserHelper},
    log_internal,
    logging::AsyncPrintColor,
};
use anyhow::Result;
use serenity::all::{ChannelId, GetMessages, Message, UserId};
use std::{collections::HashMap, time::Duration};
use tokio::time::Instant;

/// State which is lost across sessions
pub struct VolatileState {
    pub history: History,
    pub notify_timestamp: NotifyTimestamp,
}

pub struct History(HashMap<ChannelId, Vec<HistoryEntry>>);

pub struct HistoryEntry {
    pub author_id: UserId,
    pub author_name: String,
    /// Translate Discord markup such as `<@123>` to human (and LLM) understandable formats such as
    /// usernames.
    pub human_format_content: String,
}

pub struct NotifyTimestamp(HashMap<UserId, Instant>);

impl VolatileState {
    pub async fn new() -> Self {
        Self {
            history: History::new(),
            notify_timestamp: NotifyTimestamp::new(),
        }
    }
}

impl<'a> History {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub async fn backfill(
        &'a mut self,
        ctx: &Context<'_>,
        channel_id: ChannelId,
    ) -> Result<&'a mut Vec<HistoryEntry>> {
        use std::collections::hash_map::Entry::*;
        let vacant_entry = match self.0.entry(channel_id) {
            Occupied(occupied_entry) => return Ok(occupied_entry.into_mut()),
            Vacant(vacant_entry) => vacant_entry,
        };

        let backfill_limit = ctx.cfg.read().await.history.channel_backfill_message_count;

        log_internal!(
            "Backfilling the last {} messages in \"{}\"... ",
            backfill_limit,
            channel_id.color(ctx.http).await,
        );

        // Ignore errors here.  May be serenity crate bug?
        let backfill_messages = channel_id
            .messages(ctx.cache_http, GetMessages::new().limit(backfill_limit))
            .await
            .unwrap_or_default();

        // Messages are provided newest to oldest.  Iterate in reverse order so the messages are in chronological order.
        let mut messages = Vec::new();
        for msg in backfill_messages.iter().rev() {
            let author_id = msg.author.id;
            let author_name = msg.author.nick_in_guild(ctx, msg.guild_id).await;
            let human_format_content = msg.human_format_content(ctx).await?;

            let entry = HistoryEntry {
                author_id,
                author_name,
                human_format_content,
            };
            messages.push(entry);
        }

        let channel_history = vacant_entry.insert(messages);

        log_internal!(
            "Backfilling the last {} messages in \"{}\"... done",
            backfill_limit,
            channel_id.color(ctx.http).await,
        );

        Ok(channel_history)
    }

    pub async fn get_mut(
        &'a mut self,
        ctx: &'_ Context<'_>,
        channel_id: ChannelId,
    ) -> Result<&'a mut Vec<HistoryEntry>> {
        self.backfill(ctx, channel_id).await
    }

    // Need to be `&mut self` to backfill
    pub async fn get(
        &'a mut self,
        ctx: &'_ Context<'_>,
        channel_id: ChannelId,
    ) -> Result<&'a Vec<HistoryEntry>> {
        self.backfill(ctx, channel_id)
            .await
            .map(|history| &*history)
    }

    pub async fn push(&mut self, ctx: &Context<'_>, msg: &Message) -> Result<()> {
        let channel_id = msg.channel_id;
        let author_id = msg.author.id;
        let author_name = msg.author.nick_in_guild(ctx, msg.guild_id).await;
        let human_format_content = msg.human_format_content(ctx).await?;
        let entry = HistoryEntry {
            author_id,
            author_name,
            human_format_content,
        };

        let history = self.get_mut(ctx, channel_id).await?;
        history.push(entry);

        let history_max = ctx.cfg.read().await.history.channel_max_message_count;

        while history.len() > history_max {
            history.remove(0);
        }

        Ok(())
    }
}

impl NotifyTimestamp {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub async fn okay_to_notify(&self, ctx: &Context<'_>, id: UserId) -> bool {
        let now = tokio::time::Instant::now();
        let limit = ctx.cfg.read().await.general.notification_limit_seconds;
        let limit = Duration::from_secs(limit);

        match self.0.get(&id) {
            Some(last) if now.duration_since(*last) < limit => false,
            Some(_) => true,
            None => true,
        }
    }

    pub async fn update_notify_timestamp(&mut self, id: UserId) {
        let now = tokio::time::Instant::now();
        self.0.insert(id, now);
    }
}
