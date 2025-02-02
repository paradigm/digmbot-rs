use anyhow::{anyhow, Result};
use serenity::all::Context;
use std::{collections::HashMap, path::PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[serenity::async_trait]
pub trait MessageHelper {
    fn guild_name(&self, ctx: &Context) -> String;
    async fn channel_name(&self, ctx: &Context) -> String;
    fn author_name(&self, ctx: &Context) -> &str;
    async fn human_format(&self, ctx: &Context) -> String;
    async fn is_to_me(&self, ctx: &Context) -> Result<bool>;
}

#[serenity::async_trait]
impl MessageHelper for serenity::all::Message {
    fn guild_name(&self, ctx: &Context) -> String {
        if self.guild_id.is_none() {
            "DirectMessage".to_string()
        } else if let Some(guild) = self.guild(&ctx.cache) {
            guild.name.clone()
        } else {
            "<unknown-guild>".to_string()
        }
    }

    async fn channel_name(&self, ctx: &Context) -> String {
        self.channel_id
            .name(&ctx.http)
            .await
            .unwrap_or_else(|_| "<unknown-channel>".into())
    }

    fn author_name(&self, _ctx: &Context) -> &str {
        self.author.display_name()
    }

    async fn human_format(&self, ctx: &Context) -> String {
        let author_name = self.author_name(ctx);
        // Serenity provides a message.content_safe() method which uses global discord names rather
        // than per-server names.  Thus, we're just reimplementing the logic here with the
        // preferred name.
        // let content = self.content_safe(&ctx.cache);
        let mut content = self.content.clone();

        // Create a mapping from mention strings to their names
        let mut mention_map: HashMap<String, String> = HashMap::new();

        // Map user mentions
        for user in &self.mentions {
            let user_id = user.id;
            let mention_with_nickname = format!("<@!{}>", user_id);
            let mention_without_nickname = format!("<@{}>", user_id);

            // Choose to use username or display name
            let username = format!("@{}", user_id.name(ctx).await);

            // Map both mention formats to the username
            mention_map.insert(mention_with_nickname, username.clone());
            mention_map.insert(mention_without_nickname, username.clone());
        }

        if let Some(guild) = self.guild(&ctx.cache) {
            // Map role mentions
            for role_id in &self.mention_roles {
                let mention = format!("<@&{}>", role_id);

                if let Some(role) = guild.roles.get(role_id) {
                    let role_name = role.name.clone();
                    mention_map.insert(mention, format!("@{}", role_name));
                } else {
                    mention_map.insert(mention, "@UnknownRole".to_string());
                }
            }

            // Map channel mentions
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

        // Replace all mentions with their names
        for (mention, name) in mention_map {
            content = content.replace(&mention, &name);
        }

        format!("{}: {}", author_name, content)
    }

    async fn is_to_me(&self, ctx: &Context) -> Result<bool> {
        // mentions me, the bot, directly
        if self.mentions_me(&ctx).await? {
            return Ok(true);
        }

        // Is a reply to a comment the bot made
        let my_id = ctx.cache.current_user().id;
        if let Some(reference) = &self.message_reference {
            if let Some(msg_id) = reference.message_id {
                if self.channel_id.message(&ctx, msg_id).await?.author.id == my_id {
                    return Ok(true);
                }
            }
        }

        // mentions a role I'm in within the guild
        let message_roles = &self.mention_roles;
        if message_roles.is_empty() {
            return Ok(false);
        }
        let Some(guild) = &self.guild(&ctx.cache) else {
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
}

#[serenity::async_trait]
pub trait UserIdHelper {
    async fn name(&self, ctx: &Context) -> String;
}

#[serenity::async_trait]
impl UserIdHelper for serenity::all::UserId {
    async fn name(&self, ctx: &Context) -> String {
        let Ok(user) = self.to_user(&ctx.http).await else {
            return format!("<unknown-user-{}>", *self);
        };

        user.display_name().to_owned()
    }
}

#[serenity::async_trait]
pub trait VoiceStateHelper {
    async fn channel_name(&self, ctx: &Context) -> String;
}

#[serenity::async_trait]
impl VoiceStateHelper for serenity::all::VoiceState {
    async fn channel_name(&self, ctx: &Context) -> String {
        let Some(id) = self.channel_id else {
            return "<unknown-channel>".to_owned();
        };

        id.name(&ctx.http)
            .await
            .unwrap_or_else(|_| "<unknown-channel>".into())
    }
}

#[serenity::async_trait]
pub trait PathHelper {
    async fn read_to_string(&self) -> std::io::Result<String>;
    async fn read_to_bytes(&self) -> std::io::Result<Vec<u8>>;
    async fn write(&self, s: &str) -> std::io::Result<Vec<u8>>;
}

#[serenity::async_trait]
impl PathHelper for PathBuf {
    async fn read_to_string(&self) -> std::io::Result<String> {
        let mut file = tokio::fs::File::open(self).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;

        Ok(contents)
    }

    async fn read_to_bytes(&self) -> std::io::Result<Vec<u8>> {
        let mut file = tokio::fs::File::open(self).await?;
        let mut contents: Vec<u8> = vec![];
        file.read_to_end(&mut contents).await?;

        Ok(contents)
    }

    async fn write(&self, s: &str) -> std::io::Result<Vec<u8>> {
        let mut file = tokio::fs::File::open(self).await?;
        let contents = s.as_bytes();
        file.write_all(contents).await?;

        Ok(contents.to_vec())
    }
}

// Path to file within digmbot settings, configuration, and state directory.
// On Linux, this would be ~/.config/digmbot/<filename>
pub fn config_path(filename: &str) -> Result<PathBuf> {
    let mut path = dirs::home_dir().ok_or(anyhow!("Could not find home directory"))?;

    path.push(".config");
    path.push("digmbot");

    if !path.exists() {
        std::fs::create_dir_all(&path)?;
    }

    path.push(filename);

    Ok(path)
}
