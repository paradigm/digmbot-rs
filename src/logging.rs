//! Logging to the terminal with colors

use serenity::all::Http;
use std::borrow::Cow;
use std::io::IsTerminal;
use std::sync::{Arc, LazyLock};

const DEFAULT: &str = "\x1b[0m";
const FG_BLUE: &str = "\x1b[38;5;33m";
const FG_CYAN: &str = "\x1b[36m";
const FG_GRAY: &str = "\x1b[90m";
const FG_GREEN: &str = "\x1b[32m";
const FG_MAGENTA: &str = "\x1b[35m";
const FG_YELLOW: &str = "\x1b[33m";

pub enum Color {
    Default,
    Event,
    Internal,
    User,
    Channel,
    Guild,
    Glue,
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Only print colors when printing to a terminal
        //
        // This won't change during the program's execution, so we can cache it.
        static STDOUT_IS_TERMINAL: LazyLock<bool> =
            LazyLock::new(|| std::io::stdout().is_terminal());

        if !*STDOUT_IS_TERMINAL {
            return Ok(());
        }

        write!(
            f,
            "{}",
            match self {
                Color::Default => DEFAULT,
                Color::Event => FG_YELLOW,
                Color::Internal => FG_MAGENTA,
                Color::User => FG_GREEN,
                Color::Channel => FG_CYAN,
                Color::Guild => FG_BLUE,
                Color::Glue => FG_GRAY,
            }
        )
    }
}

#[macro_export]
macro_rules! log_event {
    // Case: Only format string, no arguments
    ($fmtstr:expr) => {{
        println!(
            concat!("{}*{} ", $fmtstr),
            $crate::logging::Color::Event,
            $crate::logging::Color::Default
        )
    }};

    // Case: Format string with arguments, with optional trailing comma
    ($fmtstr:expr, $($args:expr),* $(,)?) => {{
        println!(
            concat!("{}*{} ", $fmtstr),
            $crate::logging::Color::Event,
            $crate::logging::Color::Default,
            $($args),*
        )
    }};
}

#[macro_export]
macro_rules! log_internal {
    // Case: Only format string, no arguments
    ($fmtstr:expr) => {{
        println!(
            concat!("{}+{} ", $fmtstr),
            $crate::logging::Color::Internal,
            $crate::logging::Color::Default
        )
    }};

    // Case: Format string with arguments, with optional trailing comma
    ($fmtstr:expr, $($args:expr),* $(,)?) => {{
        println!(
            concat!("{}+{} ", $fmtstr),
            $crate::logging::Color::Internal,
            $crate::logging::Color::Default,
            $($args),*
        )
    }};
}
pub trait PrintColor {
    fn color(&self) -> String;
}

#[serenity::async_trait]
pub trait AsyncPrintColor {
    async fn color(&self, http: &Arc<Http>) -> String;
}

// Field separator
pub struct Glue;
impl PrintColor for Glue {
    fn color(&self) -> String {
        format!("{}{}{}", Color::Glue, ":", Color::Default)
    }
}

impl PrintColor for serenity::all::CurrentUser {
    fn color(&self) -> String {
        format!("{}{}{}", Color::User, self.name.as_str(), Color::Default)
    }
}

impl PrintColor for serenity::all::User {
    fn color(&self) -> String {
        format!("{}{}{}", Color::User, self.name.as_str(), Color::Default)
    }
}

#[serenity::async_trait]
impl AsyncPrintColor for serenity::all::UserId {
    async fn color(&self, http: &Arc<Http>) -> String {
        let name = match self.to_user(http).await {
            Ok(user) => Cow::Owned(user.name),
            Err(_) => Cow::Borrowed("<unknown-user>"),
        };

        format!("{}{}{}", Color::User, name, Color::Default)
    }
}

#[serenity::async_trait]
impl AsyncPrintColor for Option<serenity::all::UserId> {
    async fn color(&self, http: &Arc<Http>) -> String {
        let name = match self {
            Some(user_id) => match user_id.to_user(http).await {
                Ok(user) => Cow::Owned(user.name),
                Err(_) => Cow::Borrowed("<unknown-user>"),
            },
            None => Cow::Borrowed("<unknown-user>"),
        };

        format!("{}{}{}", Color::User, name, Color::Default)
    }
}

#[serenity::async_trait]
impl AsyncPrintColor for Option<serenity::all::ChannelId> {
    async fn color(&self, http: &Arc<Http>) -> String {
        let name = match self {
            Some(channel_id) => match channel_id.name(http).await {
                Ok(name) => Cow::Owned(name),
                Err(_) => Cow::Borrowed("<unknown-channel>"),
            },
            None => Cow::Borrowed("<unknown-channel>"),
        };

        format!("{}{}{}", Color::Channel, name, Color::Default)
    }
}

#[serenity::async_trait]
impl AsyncPrintColor for serenity::all::ChannelId {
    async fn color(&self, http: &Arc<Http>) -> String {
        match self.name(http).await {
            Ok(name) => format!("{}{}{}", Color::Channel, name, Color::Default),
            Err(_) => format!(
                "{}{}{}",
                Color::Channel,
                "<unknown-channel>",
                Color::Default
            ),
        }
    }
}

#[serenity::async_trait]
impl AsyncPrintColor for Option<serenity::all::GuildId> {
    async fn color(&self, http: &Arc<Http>) -> String {
        let name = match self {
            Some(guild_id) => match guild_id.to_partial_guild(http).await {
                Ok(guild) => Cow::Owned(guild.name),
                Err(_) => Cow::Borrowed("<unknown-guild>"),
            },
            None => Cow::Borrowed("<direct-message>"),
        };

        format!("{}{}{}", Color::Guild, name, Color::Default)
    }
}
