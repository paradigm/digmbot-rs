use crate::{context::Context, helper::UserHelper, log_internal};
use anyhow::{anyhow, Result};
use serenity::all::ChannelId;

/// LLM generation settings
pub struct LlmSettings<'a> {
    pub model_name: &'a str,
    pub system: &'a str,
    pub context_size: usize,
    pub temperature: f32,
}

#[derive(serde::Serialize)]
pub struct LlmChatRequest {
    /// LLM model name
    model: String,
    /// Whether to stream one token at a time, or return entire response is one go
    stream: bool,
    /// Chat conversation to continue.
    messages: Vec<ChatMessage>,
    /// Context size
    num_ctx: usize,
    /// LLM temperature
    temperature: f32,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ChatMessage {
    role: ChatMessageRole,
    content: String,
}

#[allow(non_camel_case_types)] // Serialized literally; case matters
#[derive(serde::Serialize, serde::Deserialize)]
enum ChatMessageRole {
    system,
    user,
    assistant,
}

#[derive(serde::Deserialize)]
struct LLmChatResponse {
    message: ChatMessage,
}

// #[derive(serde::Serialize)]
// struct LlmCompletionRequest {
//     /// LLM model name
//     model: String,
//     /// System prompt
//     system: String,
//     /// Whether to stream one token at a time, or return entire response is one go
//     stream: bool,
//     /// Start of text for which the bot should generation further text.
//     prompt: String,
//     /// Context size
//     num_ctx: usize,
//     /// LLM temperature
//     temperature: f32,
// }
//
// #[derive(serde::Deserialize)]
// struct LlmCompletionResponse {
//     response: String,
// }

impl LlmChatRequest {
    pub async fn from_recent_history(
        ctx: &Context<'_>,
        channel_id: ChannelId,
        settings: &LlmSettings<'_>,
    ) -> Result<Self> {
        let guild_id = channel_id
            .to_channel(ctx.cache_http)
            .await?
            .guild()
            .map(|g| g.guild_id);

        let mut vstate = ctx.vstate.write().await;
        let history = vstate.history.get(ctx, channel_id).await?;

        let bot = ctx.cache.current_user().clone(); // clone to avoid async/send safety
        let bot_id = bot.id;
        let bot_name = bot.nick_in_guild(ctx, guild_id).await;

        let interlocutor_name = &history
            .last()
            .ok_or(anyhow!(
                "LlmChatRequest::from_recent_history() called without any history"
            ))?
            .author_name;

        let system = settings
            .system
            .replace("{{bot}}", bot_name.as_str())
            .replace("{{user}}", interlocutor_name);

        // Build in reverse order so that we can stop adding if the accumulated content gets too
        // long.
        let mut total_bytes = system.len(); // include not yet added system message size
        let mut messages = Vec::new();
        for entry in history.iter().rev() {
            let (role, content) = if entry.author_id == bot_id {
                let content = entry.human_format_content.clone();
                (ChatMessageRole::assistant, content)
            } else {
                let content = format!("{}: {}", entry.author_name, &entry.human_format_content);
                (ChatMessageRole::user, content)
            };
            total_bytes += content.len();
            // Use byte count as a crude estimate of tokens.
            if total_bytes / 3 > settings.context_size {
                break;
            }
            messages.push(ChatMessage { role, content });
        }

        // Add system message at the end of about-to-be-reversed message history so it's at the
        // start
        messages.push(ChatMessage {
            role: ChatMessageRole::system,
            content: system,
        });

        // Reverse back to chronological order.
        messages.reverse();

        Ok(Self {
            model: settings.model_name.to_owned(),
            messages,
            stream: false,
            temperature: settings.temperature,
            num_ctx: settings.context_size,
        })
    }

    pub async fn post(&self, ctx: &Context<'_>) -> Result<String> {
        let cfg = ctx.cfg.read().await;
        let url = cfg.llm_general.chat_url.as_str();

        log_internal!("Sending request to chat endpoint {}... ", url);
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .json(self)
            .send()
            .await?
            .json::<LLmChatResponse>()
            .await?;
        log_internal!("Sending request to chat endpoint {}... done", url);
        let response_content = response.message.content;

        // TODO: split messages longer than the discord max of 2000 characters into multiple
        // messages.  Put some time between them to avoid Discord thinking of it as spam.
        if response_content.len() >= 1900 {
            return Ok("I blabbed too long and my message was longer than the discord post limit and paradigm didn't implement a solution to cut a post up into multiple messages".to_string());
        }

        Ok(response_content)
    }
}
