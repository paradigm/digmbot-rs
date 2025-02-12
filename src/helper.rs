//! Miscellaneous convenience methods

use crate::context::Context;
use anyhow::Result;
use serenity::all::GuildId;
use std::collections::HashMap;

#[serenity::async_trait]
pub trait UserIdHelper {
    async fn nick_in_guild(&self, ctx: &Context, guild_id: Option<GuildId>) -> String;
}

#[serenity::async_trait]
impl UserIdHelper for serenity::all::UserId {
    async fn nick_in_guild(&self, ctx: &Context, guild_id: Option<GuildId>) -> String {
        let user = match self.to_user(ctx.cache_http).await {
            Ok(user) => user,
            Err(_) => return format!("<unknown-user-{}", *self),
        };

        user.nick_in_guild(ctx, guild_id).await
    }
}

#[serenity::async_trait]
pub trait UserHelper {
    async fn nick_in_guild(&self, ctx: &Context, guild_id: Option<GuildId>) -> String;
}

#[serenity::async_trait]
impl UserHelper for serenity::all::User {
    async fn nick_in_guild(&self, ctx: &Context, guild_id: Option<GuildId>) -> String {
        let nick_in_guild = match guild_id {
            Some(guild_id) => self.nick_in(ctx.cache_http, guild_id).await,
            None => None,
        };

        // May not be in a guild, e.g. DM.  Fall back to global username.
        match nick_in_guild {
            Some(nick_in_guild) => nick_in_guild,
            None => self.name.clone(),
        }
    }
}

#[serenity::async_trait]
pub trait MessageHelper {
    async fn human_format_content(&self, ctx: &Context) -> Result<String>;
    async fn is_to_me(&self, ctx: &Context) -> Result<bool>;
    async fn is_from_owner(&self, ctx: &Context) -> bool;
}

#[serenity::async_trait]
impl MessageHelper for serenity::all::Message {
    /// Convert discord-formatted message content, which may contain non-user-friendly markup, to a
    /// human-friendly format.  Also useful for LLMs.
    ///
    /// Serenity provides a message.content_safe() method which uses global discord names rather
    /// than our preferred per-server names.  Thus, we're reimplementing the logic here with the
    /// preferred name.
    async fn human_format_content(&self, ctx: &Context) -> Result<String> {
        let mut content = self.content.clone();

        // Create a mapping from mention strings to their names
        let mut mention_map: HashMap<String, String> = HashMap::new();

        // Map user mentions (e.g. `<@!1234567890>`)
        for user in &self.mentions {
            let user_id = user.id;
            let mention_with_nickname = format!("<@!{}>", user_id);
            let mention_without_nickname = format!("<@{}>", user_id);

            let name = user.id.nick_in_guild(ctx, self.guild_id).await;

            // Map both mention formats to the username
            mention_map.insert(mention_with_nickname, name.clone());
            mention_map.insert(mention_without_nickname, name.clone());
        }

        if let Some(guild) = self.guild(ctx.cache) {
            // Map role mentions (e.g. `<@&1234567890>`)
            for role_id in &self.mention_roles {
                let mention = format!("<@&{}>", role_id);

                if let Some(role) = guild.roles.get(role_id) {
                    let role_name = role.name.clone();
                    mention_map.insert(mention, format!("@{}", role_name));
                } else {
                    mention_map.insert(mention, "@UnknownRole".to_string());
                }
            }

            // Map channel mentions (e.g. `<@#1234567890>`)
            for channel in &self.mention_channels {
                let channel_id = channel.id;
                let mention = format!("<#{}>", channel_id);

                if let Some(channel) = guild.channels.get(&channel_id) {
                    let channel_name = format!("#{}", channel.name);
                    mention_map.insert(mention, channel_name);
                } else {
                    mention_map.insert(mention, "#UnknownChannel".to_string());
                }
            }
        }

        // Replace all mentions with their human-facing names
        for (mention, name) in mention_map {
            content = content.replace(&mention, &name);
        }

        Ok(content)
    }

    async fn is_to_me(&self, ctx: &Context) -> Result<bool> {
        // mentions me, the bot, directly
        if self.mentions_me(ctx.cache_http).await? {
            return Ok(true);
        }

        // Is a reply to a comment the bot made
        let my_id = ctx.cache.current_user().id;
        if let Some(reference) = &self.message_reference {
            if let Some(msg_id) = reference.message_id {
                if self
                    .channel_id
                    .message(ctx.cache_http, msg_id)
                    .await?
                    .author
                    .id
                    == my_id
                {
                    return Ok(true);
                }
            }
        }

        // mentions a role I'm in within the guild
        let message_roles = &self.mention_roles;
        if message_roles.is_empty() {
            return Ok(false);
        }
        let Some(guild) = &self.guild(ctx.cache) else {
            return Ok(false);
        };
        let Some(my_member) = &guild.members.get(&my_id) else {
            return Ok(false);
        };
        let my_roles = &my_member.roles;
        for role_id in message_roles {
            if my_roles.contains(role_id) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn is_from_owner(&self, ctx: &Context) -> bool {
        let owners = &ctx.cfg.read().await.general.bot_owners;
        let author_global_name = &self.author.name;

        owners.contains(author_global_name)
    }
}
