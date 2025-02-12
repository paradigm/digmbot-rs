use anyhow::{anyhow, Result};
use serenity::all::UserId;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use tokio::io::AsyncReadExt;

const PSTATE_PATH_REL_HOME: &str = ".config/digmbot/state.toml";

/// State which persists across sessions
#[derive(serde::Serialize, serde::Deserialize)]
pub struct PersistentState {
    pub vc_notify: VcNotify,
    pub rivals_ratings: RivalsRatings,
    pub rivals_ratings_owners: RivalsRatingsOwners,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VcNotify {
    pub followers: HashSet<UserId>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RivalsRatings(pub HashMap<String, usize>);

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RivalsRatingsOwners(pub HashMap<String, UserId>);

impl PersistentState {
    fn config_path() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|p| p.join(PSTATE_PATH_REL_HOME))
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

        let pstate: PersistentState = toml::from_str(&contents).map_err(|e| {
            anyhow!(
                "Could not parse state at `{}`: {}",
                path.to_string_lossy(),
                e
            )
        })?;

        Ok(pstate)
    }

    pub async fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let pstate_str = toml::to_string_pretty(&self)
            .map_err(|e| anyhow!("Could not serialize state: {}", e))?;

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                anyhow!(
                    "Could not create directory `{}`: {}",
                    parent.to_string_lossy(),
                    e
                )
            })?;
        }

        // Create a temporary file in the same directory.
        let tmp_path = path.with_extension("toml.new");

        tokio::fs::write(&tmp_path, pstate_str).await.map_err(|e| {
            anyhow!(
                "Could not write state to temporary file `{}`: {}",
                tmp_path.to_string_lossy(),
                e
            )
        })?;

        // Atomically rename the temporary file over the target file.
        tokio::fs::rename(&tmp_path, &path).await.map_err(|e| {
            anyhow!(
                "Could not rename temporary file `{}` to `{}`: {}",
                tmp_path.to_string_lossy(),
                path.to_string_lossy(),
                e
            )
        })?;

        Ok(())
    }
}
