use crate::llm::LlmSettings;
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use tokio::io::AsyncReadExt;

const CONFIG_PATH_REL_HOME: &str = ".config/digmbot/config.toml";

/// Bot configuration
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub general: General,
    pub history: History,
    pub llm_general: LlmGeneral,
    pub llm_reply: LlmReply,
    pub llm_permission_denied: LlmPermissionDenied,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct General {
    pub discord_token: String,
    pub bot_owners: Vec<String>,
    pub command_prefix: String,
    pub notification_limit_seconds: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct History {
    pub channel_backfill_message_count: u8,
    pub channel_max_message_count: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LlmGeneral {
    pub chat_url: String,
    pub completion_url: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmReply {
    pub model_name: String,
    pub system: String,
    pub context_size: usize,
    pub temperature: f32,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LlmPermissionDenied {
    pub model_name: String,
    pub system: String,
    pub context_size: usize,
    pub temperature: f32,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VcNotify {
    pub global_names: Vec<String>,
}

impl Config {
    fn config_path() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|p| p.join(CONFIG_PATH_REL_HOME))
            .ok_or(anyhow!("Could not find home directory"))
    }

    pub async fn load() -> Result<Self> {
        let path = Self::config_path()?;

        let mut file = tokio::fs::File::open(&path).await.map_err(|e| {
            anyhow!(
                "Could not open configuration at `{}`: {}",
                path.to_string_lossy(),
                e
            )
        })?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).await.map_err(|e| {
            anyhow!(
                "Could not read configuration at `{}`: {}",
                path.to_string_lossy(),
                e
            )
        })?;

        let config: Config = toml::from_str(&contents).map_err(|e| {
            anyhow!(
                "Could not parse configuration at `{}`: {}",
                path.to_string_lossy(),
                e
            )
        })?;

        Ok(config)
    }

    pub async fn reload(&mut self) -> Result<()> {
        let new = Self::load().await?;
        *self = new;
        Ok(())
    }
}

impl<'a> LlmReply {
    pub fn as_llm_settings(&'a self) -> LlmSettings<'a> {
        LlmSettings {
            model_name: &self.model_name,
            system: &self.system,
            context_size: self.context_size,
            temperature: self.temperature,
        }
    }
}

impl<'a> LlmPermissionDenied {
    pub fn as_llm_settings(&'a self) -> LlmSettings<'a> {
        LlmSettings {
            model_name: &self.model_name,
            system: &self.system,
            context_size: self.context_size,
            temperature: self.temperature,
        }
    }
}
