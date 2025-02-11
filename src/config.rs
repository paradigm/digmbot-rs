use crate::helper::PathHelper;
use anyhow::{anyhow, Result};
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub general: General,
    pub llm: Llm,
    pub vc_notify: VcNotify,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct General {
    pub discord_token: String,
    pub bot_owners: Vec<String>,
    pub command_prefix: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Llm {
    pub chat_url: String,
    pub completion_url: String,
    pub model_name: String,
    pub system_prompt: String,
    pub context_size: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VcNotify {
    pub global_names: Vec<String>,
}

impl Config {
    pub async fn load() -> Result<Self> {
        let config_str = Self::config_path()?.read_to_string().await?;
        let config: Config = toml::from_str(&config_str)?;

        Ok(config)
    }

    fn config_path() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|p| p.join(".config"))
            .map(|p| p.join(".digmbot.toml"))
            .ok_or(anyhow!("Could not find home directory"))
    }
}
