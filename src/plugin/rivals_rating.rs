use crate::config::Config;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serenity::{
    all::{Message, UserId},
    prelude::TypeMapKey,
};
use std::{collections::HashMap, path::PathBuf};
use tokio::fs as tokio_fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

use crate::{event::*, plugin::*};

const STOCK_VALUE: f64 = 150.0; // 150% rating difference equates to one stock
const MAX_DELTA: f64 = 300.0; // maximum allowed rating difference (in percent) to update ratings
const ELO_K_FACTOR: f64 = 20.0; // K-factor such that when two equally rated players compete, change = 20*(1-0.5)=10
const ELO_DIVISOR: f64 = 400.0; // divisor for the Elo expected score formula

/// A record of a match played by a player.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MatchResult {
    Win,
    Loss,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MatchRecord {
    pub opponent: String,
    pub result: MatchResult,
    pub self_rating: f64,
    pub opponent_rating: f64,
    pub rating_change: f64,
    pub timestamp: u64, // UNIX timestamp in seconds
}

/// A Rivals player.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub name: String,
    /// Store the owner as a u64 for easy serialization.
    pub owner: UserId,
    pub rating: f64,
    pub match_history: Vec<MatchRecord>,
}

/// The global state for the Rivals plugin.
#[derive(Serialize, Deserialize, Debug)]
pub struct RivalsState {
    pub players: HashMap<String, Player>, // keyed by player name (unique)
}

impl RivalsState {
    /// Save the current state to disk.
    pub async fn save(&self) -> Result<()> {
        let serialized = serde_json::to_string_pretty(self)
            .map_err(|e| anyhow!("Failed to serialize Rivals state: {}", e))?;
        let path = config_path("rivals_rating.json")?;
        // Write the serialized data to file.
        let mut file = tokio_fs::File::create(&path).await?;
        file.write_all(serialized.as_bytes()).await?;
        Ok(())
    }

    /// Load the state from disk, or create a new empty state if not present.
    pub async fn load() -> Result<Self> {
        let path = config_path("rivals_rating.json")?;
        match tokio_fs::read(&path).await {
            Ok(data) => {
                let state: RivalsState = serde_json::from_slice(&data)
                    .map_err(|e| anyhow!("Failed to deserialize Rivals state: {}", e))?;
                Ok(state)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(RivalsState {
                players: HashMap::new(),
            }),
            Err(e) => Err(anyhow!("Failed to read rivals.json: {}", e)),
        }
    }
}

/// Serenity’s TypeMapKey implementation for storing our Rivals state.
pub struct RivalsStateKey;
impl TypeMapKey for RivalsStateKey {
    type Value = RivalsState;
}

//
// The Rivals Plugin – implementing the Plugin trait
//

pub struct PluginRivalsRating;

#[serenity::async_trait]
impl Plugin for PluginRivalsRating {
    fn name(&self) -> &'static str {
        "Rivals"
    }

    async fn usage(&self, cfg: &RwLock<Config>) -> Option<String> {
        let prefix = &cfg.read().await.general.command_prefix;
        Some(format!("{}rivals create <initial_rating> [player_name] - Create a new player\n\
             {}rivals delete <player_name> - Delete one of your players\n\
             {}rivals list - List all players (ordered by rating descending)\n\
             {}rivals history <player_name> - Show a player's match history\n\
             {}rivals preview <player1> <player2> - Show ratings and starting handicap\n\
             {}rivals report <loser_player> <winner_player> - Report a match result (you must own the loser)",
             prefix, prefix, prefix, prefix, prefix, prefix))
    }

    async fn init(&self, ctx: &Context) -> Result<()> {
        // Attempt to load the state from disk; if not found, initialize an empty state.
        let state = RivalsState::load().await?;
        ctx.data.write().await.insert::<RivalsStateKey>(state);
        Ok(())
    }

    async fn handle(&self, event: &crate::event::Event) -> Result<EventHandled> {
        match event {
            Event::Message { ctx, msg } => handle_message(ctx, msg).await,
            _ => Ok(EventHandled::No),
        }
    }
}

//
// Helper functions for command handling
//

async fn handle_message(ctx: &Context, msg: &Message) -> Result<EventHandled> {
    // Only process messages that start with ";rivals"
    let config = ctx.data.read().await;
    let prefix = &cfg.read().await.general.command_prefix;
    let command_name = format!("{}rivals", prefix);

    let tokens: Vec<&str> = msg.content.split_whitespace().collect();
    if tokens.is_empty() || tokens[0] != command_name {
        return Ok(EventHandled::No);
    }

    // Disallow abusive users
    if msg.author.id == 758891804489416725 {
        msg.reply(ctx, "No Moos allowed").await?;
        return Ok(EventHandled::Yes);
    }

    // We expect at least a subcommand
    if tokens.len() < 2 {
        msg.reply(ctx, "Missing subcommand. Try `;help` for usage.")
            .await?;
        return Ok(EventHandled::Yes);
    }

    // Dispatch subcommand
    let subcommand = tokens[1];
    match subcommand {
        "create" => handle_create(ctx, msg, &tokens[2..]).await,
        "delete" => handle_delete(ctx, msg, &tokens[2..]).await,
        "list" => handle_list(ctx, msg).await,
        "history" => handle_history(ctx, msg, &tokens[2..]).await,
        "preview" => handle_preview(ctx, msg, &tokens[2..]).await,
        "report" => handle_report(ctx, msg, &tokens[2..]).await,
        _ => {
            msg.reply(
                ctx,
                "Unknown subcommand for ;rivals. Use `;help` for usage.",
            )
            .await?;
            Ok(EventHandled::Yes)
        }
    }
}

/// Create a new player.
/// Usage: ;rivals create <initial_rating> [player_name]
async fn handle_create(ctx: &Context, msg: &Message, args: &[&str]) -> Result<EventHandled> {
    let config = ctx.data.read().await;
    let prefix = &config
        .get::<RwLock<Config>>()
        .unwrap()
        .read()
        .await
        .general
        .command_prefix;
    let command_name = format!("{}rivals create", prefix);

    if args.is_empty() {
        msg.reply(
            ctx,
            format!("Usage: {} <initial_rating> [player_name]", command_name),
        )
        .await?;
        return Ok(EventHandled::Yes);
    }
    // Parse the initial rating
    let initial_rating: f64 = args[0]
        .parse()
        .map_err(|_| anyhow!("Invalid initial rating provided"))?;

    // Use the provided player name if any; otherwise default to the Discord user's name.
    let player_name = if args.len() >= 2 {
        args[1..].join(" ")
    } else {
        msg.author.name.clone()
    };

    // Access the state.
    let mut data = ctx.data.write().await;
    let state = data
        .get_mut::<RivalsStateKey>()
        .ok_or(anyhow!("Rivals state not initialized"))?;

    if state.players.contains_key(&player_name) {
        msg.reply(
            ctx,
            format!("A player named '{}' already exists.", player_name),
        )
        .await?;
        return Ok(EventHandled::Yes);
    }

    let new_player = Player {
        name: player_name.clone(),
        owner: msg.author.id,
        rating: initial_rating,
        match_history: Vec::new(),
    };

    state.players.insert(player_name.clone(), new_player);
    state.save().await?;

    msg.reply(
        ctx,
        format!(
            "Player '{}' created with an initial rating of {}%.",
            player_name, initial_rating
        ),
    )
    .await?;

    Ok(EventHandled::Yes)
}

/// Delete an existing player.
/// Usage: ;rivals delete <player_name>
async fn handle_delete(ctx: &Context, msg: &Message, args: &[&str]) -> Result<EventHandled> {
    let config = ctx.data.read().await;
    let prefix = &config
        .get::<RwLock<Config>>()
        .unwrap()
        .read()
        .await
        .general
        .command_prefix;
    let command_name = format!("{}rivals delete", prefix);

    if args.is_empty() {
        msg.reply(ctx, format!("Usage: {} <player_name>", command_name))
            .await?;
        return Ok(EventHandled::Yes);
    }
    let player_name = args.join(" ");

    let mut data = ctx.data.write().await;
    let state = data
        .get_mut::<RivalsStateKey>()
        .ok_or(anyhow!("Rivals state not initialized"))?;

    if let Some(player) = state.players.get(&player_name) {
        if player.owner != msg.author.id {
            msg.reply(
                ctx,
                "You are not authorized to delete this player (you are not the creator).",
            )
            .await?;
            return Ok(EventHandled::Yes);
        }
    } else {
        msg.reply(ctx, format!("Player '{}' not found.", player_name))
            .await?;
        return Ok(EventHandled::Yes);
    }

    state.players.remove(&player_name);
    state.save().await?;

    msg.reply(ctx, format!("Player '{}' has been deleted.", player_name))
        .await?;
    Ok(EventHandled::Yes)
}

/// List all players ordered by rating (best first).
/// Usage: ;rivals list
async fn handle_list(ctx: &Context, msg: &Message) -> Result<EventHandled> {
    let config = ctx.data.read().await;
    let prefix = &config
        .get::<RwLock<Config>>()
        .unwrap()
        .read()
        .await
        .general
        .command_prefix;
    let _command_name = format!("{}rivals list", prefix); // Not used, but kept for consistency

    let data = ctx.data.read().await;
    let state = data
        .get::<RivalsStateKey>()
        .ok_or(anyhow!("Rivals state not initialized"))?;

    if state.players.is_empty() {
        msg.reply(ctx, "No players found.").await?;
        return Ok(EventHandled::Yes);
    }

    // Collect and sort players by rating (descending)
    let mut players: Vec<&Player> = state.players.values().collect();
    players.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap());

    let mut response = String::from("**Players:**\n");
    for player in players {
        // Mention the owner by constructing a Discord mention (<@USERID>)
        response.push_str(&format!(
            "**{}** – Rating: {}% – Owner: <@{}>\n",
            player.name, player.rating, player.owner
        ));
    }

    msg.reply(ctx, response).await?;
    Ok(EventHandled::Yes)
}

/// Show a player's match history.
/// Usage: ;rivals history <player_name>
async fn handle_history(ctx: &Context, msg: &Message, args: &[&str]) -> Result<EventHandled> {
    let config = ctx.data.read().await;
    let prefix = &config
        .get::<RwLock<Config>>()
        .unwrap()
        .read()
        .await
        .general
        .command_prefix;
    let command_name = format!("{}rivals history", prefix);

    if args.is_empty() {
        msg.reply(ctx, format!("Usage: {} <player_name>", command_name))
            .await?;
        return Ok(EventHandled::Yes);
    }
    let player_name = args.join(" ");

    let data = ctx.data.read().await;
    let state = data
        .get::<RivalsStateKey>()
        .ok_or(anyhow!("Rivals state not initialized"))?;

    let player = match state.players.get(&player_name) {
        Some(p) => p,
        None => {
            msg.reply(ctx, format!("Player '{}' not found.", player_name))
                .await?;
            return Ok(EventHandled::Yes);
        }
    };

    if player.match_history.is_empty() {
        msg.reply(
            ctx,
            format!("Player '{}' has no match history.", player_name),
        )
        .await?;
        return Ok(EventHandled::Yes);
    }

    // Sort match history by timestamp.
    let mut history = player.match_history.clone();
    history.sort_by_key(|rec| rec.timestamp);

    let mut response = format!("**Match history for '{}'**\n", player_name);
    for rec in history {
        let sign = if rec.rating_change >= 0.0 { "+" } else { "" };
        response.push_str(&format!(
            "Opponent: **{}** | Result: **{:?}** | Starting ratings: {}% vs {}% | Change: {}{}{:.2}%\n",
            rec.opponent,
            rec.result,
            rec.self_rating,
            rec.opponent_rating,
            sign,
            "",
            rec.rating_change.abs()
        ));
    }
    msg.reply(ctx, response).await?;
    Ok(EventHandled::Yes)
}

/// Preview a match between two players.
/// Usage: ;rivals preview <player1> <player2>
async fn handle_preview(ctx: &Context, msg: &Message, args: &[&str]) -> Result<EventHandled> {
    let config = ctx.data.read().await;
    let prefix = &config
        .get::<RwLock<Config>>()
        .unwrap()
        .read()
        .await
        .general
        .command_prefix;
    let command_name = format!("{}rivals preview", prefix);
    if args.len() < 2 {
        msg.reply(ctx, format!("Usage: {} <player1> <player2>", command_name))
            .await?;
        return Ok(EventHandled::Yes);
    }
    let player1_name = args[0].to_string();
    let player2_name = args[1].to_string();

    let data = ctx.data.read().await;
    let state = data
        .get::<RivalsStateKey>()
        .ok_or(anyhow!("Rivals state not initialized"))?;

    let player1 = match state.players.get(&player1_name) {
        Some(p) => p,
        None => {
            msg.reply(ctx, format!("Player '{}' not found.", player1_name))
                .await?;
            return Ok(EventHandled::Yes);
        }
    };
    let player2 = match state.players.get(&player2_name) {
        Some(p) => p,
        None => {
            msg.reply(ctx, format!("Player '{}' not found.", player2_name))
                .await?;
            return Ok(EventHandled::Yes);
        }
    };

    // Compute the rating difference and handicap.
    let (stronger, weaker) = if player1.rating >= player2.rating {
        (player1, player2)
    } else {
        (player2, player1)
    };
    let diff = stronger.rating - weaker.rating;
    let stocks_lost = (diff / STOCK_VALUE).floor() as u32;
    let extra_damage = diff - (stocks_lost as f64 * STOCK_VALUE);

    let mut response = format!(
        "Player **{}**: {}%\nPlayer **{}**: {}%\n\n",
        player1.name, player1.rating, player2.name, player2.rating
    );
    response.push_str(&format!(
        "Handicap: **{}** is stronger and should start with {} stock(s) down plus an extra {:.2}% damage.\n",
        stronger.name, stocks_lost, extra_damage
    ));
    if diff > MAX_DELTA {
        response.push_str(&format!(
            "\nNote: The rating difference of {:.2}% exceeds the maximum allowed delta ({}%), so rating updates would not occur.",
            diff, MAX_DELTA
        ));
    }
    msg.reply(ctx, response).await?;
    Ok(EventHandled::Yes)
}

/// Report a match result where your player lost to another player's win.
/// Usage: ;rivals report <loser_player> <winner_player>
async fn handle_report(ctx: &Context, msg: &Message, args: &[&str]) -> Result<EventHandled> {
    let config = ctx.data.read().await;
    let prefix = &config
        .get::<RwLock<Config>>()
        .unwrap()
        .read()
        .await
        .general
        .command_prefix;
    let command_name = format!("{}rivals report", prefix);

    if args.len() < 2 {
        msg.reply(
            ctx,
            format!("Usage: {} <loser_player> <winner_player>", command_name),
        )
        .await?;
        return Ok(EventHandled::Yes);
    }
    let loser_name = args[0].to_string();
    let winner_name = args[1].to_string();

    let mut data = ctx.data.write().await;
    let state = data
        .get_mut::<RivalsStateKey>()
        .ok_or(anyhow!("Rivals state not initialized"))?;

    // First, check that both players exist.
    if !state.players.contains_key(&loser_name) {
        msg.reply(ctx, format!("Player '{}' not found.", loser_name))
            .await?;
        return Ok(EventHandled::Yes);
    }
    if !state.players.contains_key(&winner_name) {
        msg.reply(ctx, format!("Player '{}' not found.", winner_name))
            .await?;
        return Ok(EventHandled::Yes);
    }

    // Verify that the report is coming from the owner of the loser.
    if state.players.get(&loser_name).unwrap().owner != msg.author.id {
        msg.reply(
            ctx,
            "You can only report matches for players that you created (i.e. that you own).",
        )
        .await?;
        return Ok(EventHandled::Yes);
    }
    // Prevent self-play by checking that the players are owned by different users.
    if state.players.get(&loser_name).unwrap().owner
        == state.players.get(&winner_name).unwrap().owner
    {
        msg.reply(ctx, "You cannot report a match against your own player.")
            .await?;
        return Ok(EventHandled::Yes);
    }

    // Remove both players from the HashMap so that we can get two independent mutable values.
    let mut loser = state
        .players
        .remove(&loser_name)
        .ok_or(anyhow!("Loser player missing unexpectedly"))?;
    let mut winner = state
        .players
        .remove(&winner_name)
        .ok_or(anyhow!("Winner player missing unexpectedly"))?;

    // Check if the rating difference is within limits.
    let diff = (winner.rating - loser.rating).abs();
    if diff > MAX_DELTA {
        // Reinsert the players back before returning.
        state.players.insert(loser_name.clone(), loser);
        state.players.insert(winner_name.clone(), winner);
        msg.reply(
            ctx,
            format!(
                "The rating difference of {:.2}% exceeds the maximum allowed delta of {}%. No update performed.",
                diff, MAX_DELTA
            ),
        )
        .await?;
        return Ok(EventHandled::Yes);
    }

    // Save current ratings for history before updating.
    let loser_old_rating = loser.rating;
    let winner_old_rating = winner.rating;

    // Compute expected scores using the Elo formula.
    let expected_winner = 1.0 / (1.0 + 10f64.powf((loser.rating - winner.rating) / ELO_DIVISOR));
    let expected_loser = 1.0 - expected_winner;

    // Calculate rating changes.
    let winner_change = ELO_K_FACTOR * (1.0 - expected_winner);
    let loser_change = -ELO_K_FACTOR * expected_loser;

    // Update ratings.
    winner.rating += winner_change;
    loser.rating += loser_change;

    // Get the current timestamp (seconds since UNIX_EPOCH)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    // Create match records.
    let winner_record = MatchRecord {
        opponent: loser.name.clone(),
        result: MatchResult::Win,
        self_rating: winner_old_rating,
        opponent_rating: loser_old_rating,
        rating_change: winner_change,
        timestamp,
    };
    let loser_record = MatchRecord {
        opponent: winner.name.clone(),
        result: MatchResult::Loss,
        self_rating: loser_old_rating,
        opponent_rating: winner_old_rating,
        rating_change: loser_change,
        timestamp,
    };

    // Append match records.
    winner.match_history.push(winner_record);
    loser.match_history.push(loser_record);

    // Reinsert the updated players.
    state.players.insert(loser_name.clone(), loser);
    state.players.insert(winner_name.clone(), winner);

    // Save the updated state.
    state.save().await?;

    let response = format!(
        "Match reported:\n**{}** loses to **{}**.\nRating updates: **{}**: +{:.2}%, **{}**: -{:.2}%",
        loser_name,
        winner_name,
        winner_name,
        winner_change,
        loser_name,
        loser_change.abs()
    );
    msg.reply(ctx, response).await?;
    Ok(EventHandled::Yes)
}

/// Helper to get the configuration file path for a given filename.
/// (This assumes a helper function `config_path` exists in crate::helper.)
fn config_path(filename: &str) -> Result<PathBuf> {
    // For simplicity, assume the config file is in the current directory.
    // In production, this might be in a dedicated configuration folder.
    let mut path = std::env::current_dir()?;
    path.push(filename);
    Ok(path)
}
