use crate::{
    event::*,
    helper::*,
    log_internal,
    plugin::{
        history::{History, HistoryEntry},
        *,
    },
};
use anyhow::{anyhow, Result};
use serenity::{all::UserId, prelude::TypeMapKey};
use std::io::ErrorKind;
use tokio::sync::RwLock;
use crate::config::Config;

/// Queries an LLM for a response
pub struct PluginLlm;

#[allow(unused)]
#[derive(serde::Deserialize, Default, Clone)]
struct LlmSettings {
    chat_url: String,
    model_name: String,
    system: String,
    context_size: usize,
}

// Non-chat completion, e.g inline code completion
//
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

#[derive(serde::Serialize)]
struct LlmChatRequest {
    /// LLM model name
    model: String,
    /// Whether to stream one token at a time, or return entire response is one go
    stream: bool,
    /// Chat conversation to continue.  Use this for completing chat history.
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
struct LlmChatResponse {
    message: ChatMessage,
}

#[serenity::async_trait]
impl Plugin for PluginLlm {
    fn name(&self) -> &'static str {
        "LLM"
    }

    fn usage(&self, _cfg: &RwLock<Config>) -> Option<String> {
        None
    }

    async fn init(&self, ctx: &Context) -> Result<()> {
        let settings = LlmSettings::load(ctx).await?;

        if let Some(settings) = settings {
            ctx.data.write().await.insert::<LlmSettings>(settings);
        } else {
            log_internal!("No (valid) llm_settings.json found, disabling LLM subsystem");
        }

        Ok(())
    }

    async fn handle(&self, event: &Event) -> Result<EventHandled> {
        let Event::Message { ctx, msg } = event else {
            return Ok(EventHandled::No);
        };

        // Ignore all bot messages, including my own
        if msg.author.bot {
            return Ok(EventHandled::No);
        }

        // Only respond if the message is to me
        if !msg.is_to_me(ctx).await? {
            return Ok(EventHandled::No);
        }

        // Interpret scenario where llm settings are not configured as opting out of this plugin
        let state = ctx.data.read().await;
        let Some(settings) = state.get::<LlmSettings>().cloned() else {
            log_internal!("+ No (valid) llm_settings.json found, skipping LLM response");
            return Ok(EventHandled::No);
        };
        // Make sure to drop this read-lock on global state before we perform a following
        // write-lock lest we deadlock.
        drop(state);

        // All the checks to see if we want to make an LLM completion request have passed.  Since
        // the request may take some time, tell Discord (and thus users in the server) that we're
        // "typing" to indicate we're processing the request.
        let typing = msg.channel_id.start_typing(&ctx.http);

        // Needs to be writable to backfill
        let mut state = ctx.data.write().await;
        let history = state
            .get_mut::<History>()
            .ok_or(anyhow!("History uninitialized"))?;
        let history = history.get(ctx, msg.channel_id).await?.clone();

        let bot_id = ctx.cache.current_user().id;
        let response = LlmChatRequest::new(&settings, &history, bot_id)
            .await?
            .post(&settings.chat_url)
            .await?;

        typing.stop();
        msg.reply(ctx, &response).await?;

        Ok(EventHandled::Yes)
    }
}

impl LlmChatRequest {
    async fn new(settings: &LlmSettings, history: &[HistoryEntry], bot_id: UserId) -> Result<Self> {
        // Start with a system message.
        let mut messages = Vec::new();
        messages.push(ChatMessage {
            role: ChatMessageRole::system,
            content: settings.system.clone(),
        });

        // Use reverse order so that we can stop adding if the accumulated content gets too long.
        let mut total_len = settings.system.len();
        let mut history_messages = Vec::new();
        // Iterate from the most recent to the oldest.
        for entry in history.iter().rev() {
            let (role, content) = if entry.author_id == bot_id {
                let content = entry.human_format_content.clone();
                (ChatMessageRole::assistant, content)
            } else {
                let content = format!("{}: {}", entry.author_name, &entry.human_format_content);
                (ChatMessageRole::user, content)
            };
            total_len += content.len();
            // Use content length as a crude estimate of tokens.
            if total_len / 3 > settings.context_size {
                break;
            }
            history_messages.push(ChatMessage { role, content });
        }

        // Reverse back to chronological order.
        history_messages.reverse();
        messages.extend(history_messages);

        Ok(Self {
            model: settings.model_name.clone(),
            messages,
            stream: false,
            temperature: 0.8, // TODO: make configurable?
            num_ctx: settings.context_size,
        })
    }

    async fn post(&self, url: &str) -> Result<String> {
        for msg in &self.messages {
            println!("> {}", msg.content);
        }

        log_internal!("Sending request to chat endpoint {}... ", url);
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .json(self)
            .send()
            .await?
            .json::<LlmChatResponse>()
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

// Serenity crate system to support this type in the Serenity Context
impl TypeMapKey for LlmSettings {
    type Value = LlmSettings;
}

impl LlmSettings {
    async fn load(ctx: &Context) -> Result<Option<Self>> {
        let mut settings: LlmSettings =
            match config_path("llm_settings.json")?.read_to_bytes().await {
                Ok(data) => serde_json::from_slice(&data)
                    .map_err(|e| anyhow!("Failed to deserialize llm_settings.json: {}", e))?,
                Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(anyhow!("Failed to read vc_notify.json: {}", e)),
            };

        // system_prompt may contain a `{}` placeholder for the bot name.
        settings.system = settings
            .system
            .replace("{}", &ctx.cache.current_user().name);

        Ok(Some(settings))
    }
}
