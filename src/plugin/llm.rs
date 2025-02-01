use crate::{
    event::*,
    helper::*,
    plugin::history::{History, HistoryEntry},
    plugin::*,
};
use anyhow::{anyhow, Result};
use serenity::{all::UserId, prelude::TypeMapKey};
use std::io::ErrorKind;

/// Queries an LLM for a response
pub struct PluginLlm;

#[derive(serde::Serialize)]
struct LlmCompletionRequest {
    // LLM model name
    model: String,
    // System prompt
    system: String,
    // Whether to stream one token at a time, or return entire response is one go
    stream: bool,
    // Text to complete.
    prompt: String,
    // Context size
    num_ctx: usize,
    // LLM temperature
    temperature: f32,
}

#[derive(serde::Deserialize)]
struct LlmCompletionResponse {
    response: String,
}

#[serenity::async_trait]
impl Plugin for PluginLlm {
    fn name(&self) -> &'static str {
        "LLM"
    }

    fn usage(&self) -> Option<&'static str> {
        None
    }

    async fn init(&self, ctx: &Context) -> Result<()> {
        let settings = LlmSettings::load(ctx).await?;

        if let Some(settings) = settings {
            ctx.data.write().await.insert::<LlmSettings>(settings);
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
            return Ok(EventHandled::No);
        };
        drop(state);

        // All the checks to see if we want to make an LLM completion request have passed.  Since
        // the request may take some time, tell Discord (and thus users in the server) that we're
        // "typing" to indicate we're processing the request.
        let typing = msg.channel_id.start_typing(&ctx.http);

        // Needs to be writable to backfill
        let mut state = ctx.data.write().await;
        let mut history = state
            .get_mut::<History>()
            .ok_or(anyhow!("History uninitialized"))?;
        let history = history.get(ctx, msg.channel_id).await?.clone();
        drop(state);

        let bot_id = ctx.cache.current_user().id;
        let bot_name = ctx.cache.current_user().display_name().to_owned();
        let response = LlmCompletionRequest::new(&settings, &history, bot_id, &bot_name)
            .await?
            .post(&settings.completion_url)
            .await?;

        // TODO: split messages longer than the discord max of 2000 characters into multiple
        // messages.  Put some time between them to avoid Discord thinking of it as spam.

        typing.stop();
        msg.reply(ctx, &response).await?;

        Ok(EventHandled::Yes)
    }
}

impl LlmCompletionRequest {
    async fn new(
        settings: &LlmSettings,
        history: &[HistoryEntry],
        bot_id: UserId,
        bot_name: &str,
    ) -> Result<Self> {
        // The llm runner will add these implicitly.  However, we still want to calculate them for
        // the context length overhead.
        let system = format!(
            "{}{}{}",
            settings.system_msg_start, settings.system, settings.system_msg_end
        );
        let bot_opening = format!("{}{}: ", settings.bot_msg_start, bot_name);

        // Convert history into llm format with turn-taking tokens
        //
        // Go through in reverse, checking that we fit within context before adding
        // older item.
        let mut body = String::new();
        for HistoryEntry {
            author_id,
            content,
            human_format,
        } in history.iter().rev()
        {
            let (msg_start, msg_end) = if *author_id == bot_id {
                (&settings.bot_msg_start, &settings.bot_msg_end)
            } else {
                (&settings.user_msg_start, &settings.user_msg_end)
            };

            let new_msg = format!("{}{}{}", msg_start, human_format, msg_end);

            if system.len() + body.len() + new_msg.len() + bot_opening.len() > settings.context_size
            {
                break;
            }
            body.insert_str(0, &new_msg);
        }

        let prompt = format!("{}{}", body, bot_opening);

        Ok(Self {
            model: settings.model_name.clone(),
            system: settings.system.clone(),
            stream: false,
            prompt,
            num_ctx: settings.context_size,
            temperature: 0.8,
        })
    }

    async fn post(&self, url: &str) -> Result<String> {
        print!("Querying LLM server... ");
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .json(&self)
            .send()
            .await?
            .json::<LlmCompletionResponse>()
            .await?;
        println!("done");

        if response.response.len() >= 1900 {
            return Ok("I blabbed too long and my message was longer than the discord post limit and paradigm didn't implement a solution to cut a post up into multiple messages".to_string());
        }

        Ok(response.response)
    }
}

#[derive(serde::Deserialize, Default, Clone)]
struct LlmSettings {
    completion_url: String,
    model_name: String,
    system: String,
    context_size: usize,
    system_msg_start: String,
    system_msg_end: String,
    user_msg_start: String,
    user_msg_end: String,
    bot_msg_start: String,
    bot_msg_end: String,
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
