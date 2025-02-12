//! Manages player ratings and associated handicaps for a Super Smash Bros Melee-style game called
//! Rivals.
//!
//! Ratings units are in percentage and tied to SSBM/Rivals damage percentages.  The difference in
//! player ratings indicates the amount of damage the stronger player should start with for an even
//! match.  Multiples of a constant threshold percentage are treated as stocks.  For example, if
//! this threshold is 150%, then when two players with a 200% difference in rating play, the
//! stronger should start one stock down and with 50% damage.
//!
//! If player ratings are too far apart, this system breaks down, and thus only player ratings
//! within a certain window are valid when updating rating values.
//!
//! After a match, the player ratings are updated similarly to Elo ratings, but scaled such that an
//! evenly rated match results in the winner gaining 10% rating and the loser losing 10% rating.
//!
//! A discord user may have multiple players registered here.  For example, they may like to have
//! their different game characters ratings tracked independently.  The owner is stored when
//! creating a player.  Only the player owner or a bot owner may delete a player.  Only the losing
//! player owner or a bot owner may report a match.

use crate::{
    context::Context,
    event::{Event, EventHandled},
    helper::{MessageHelper, UserHelper},
    llm::LlmChatRequest,
    plugin::Plugin,
};
use anyhow::{anyhow, Result};
use serenity::all::Message;
use std::borrow::Cow;
use std::cmp::Ordering;

// Constants for rating adjustments and handicaps.
const STOCK_VALUE: usize = 150; // 150% rating difference equates to one stock.
const MAX_DELTA: usize = 300; // Maximum allowed rating difference (in percent) to update ratings.
const K_FACTOR: f64 = 10.0; // Total rating change in an even match.

pub struct RivalsRating;

#[serenity::async_trait]
impl Plugin for RivalsRating {
    // Use "rivals" as the command trigger.
    fn name(&self) -> &'static str {
        "rivals"
    }

    async fn usage(&self, ctx: &Context) -> Option<String> {
        let prefix = &ctx.cfg.read().await.general.command_prefix;
        Some(format!(
            "{}rivals <subcommand> -- manage rivals ratings\n\
             | Subcommands:\n\
             | create <initial_rating> [player_name] - create a player\n\
             | delete <player_name> - delete a player\n\
             | list - list all players\n\
             | preview <player1> <player2> - show ratings and starting handicap\n\
             | report <player1> beat <player2> - report a match result (you must own the loser)",
            prefix
        ))
    }

    async fn handle(&self, ctx: &Context, event: &Event) -> Result<EventHandled> {
        let Some((msg, args_str)) = event.is_bot_cmd(ctx, self.name()).await else {
            return Ok(EventHandled::No);
        };

        let args: Vec<&str> = args_str.split_whitespace().collect();
        if args.is_empty() {
            msg.reply(
                ctx.cache_http,
                "Please provide a subcommand. See help for usage.",
            )
            .await?;
            return Ok(EventHandled::Yes);
        }

        match args[0].to_lowercase().as_str() {
            "create" => handle_create(ctx, msg, &args[1..]).await,
            "delete" => handle_delete(ctx, msg, &args[1..]).await,
            "list" => handle_list(ctx, msg).await,
            "preview" => handle_preview(ctx, msg, &args[1..]).await,
            "report" => handle_report(ctx, msg, &args[1..]).await,
            _ => {
                msg.reply(ctx.cache_http, "Unknown subcommand.").await?;
                Ok(EventHandled::Yes)
            }
        }
    }
}

async fn handle_create(ctx: &Context<'_>, msg: &Message, args: &[&str]) -> Result<EventHandled> {
    if args.is_empty() {
        msg.reply(
            ctx.cache_http,
            "Usage: create <initial_rating> [player_name]",
        )
        .await?;
        return Ok(EventHandled::Yes);
    }

    let initial_rating: usize = match args[0].parse() {
        Ok(rating) => rating,
        Err(_) => {
            msg.reply(ctx.cache_http, "Invalid initial rating: must be an integer")
                .await?;
            return Ok(EventHandled::Yes);
        }
    };

    // Use provided player name or default to the author's name.
    let player_name = if args.len() >= 2 {
        args[1].to_string()
    } else {
        msg.author.nick_in_guild(ctx, msg.guild_id).await
    };

    let mut pstate = ctx.pstate.write().await;
    // Check if the player already exists.
    if pstate.rivals_ratings.0.contains_key(&player_name) {
        msg.reply(
            ctx.cache_http,
            format!("Player `{}` already exists.", player_name),
        )
        .await?;
        return Ok(EventHandled::Yes);
    }

    pstate
        .rivals_ratings
        .0
        .insert(player_name.clone(), initial_rating);
    pstate
        .rivals_ratings_owners
        .0
        .insert(player_name.clone(), msg.author.id);

    pstate.save().await?;

    msg.reply(
        ctx.cache_http,
        format!(
            "Player `{}` created with initial rating {}%.",
            player_name, initial_rating
        ),
    )
    .await?;
    Ok(EventHandled::Yes)
}

async fn handle_delete(ctx: &Context<'_>, msg: &Message, args: &[&str]) -> Result<EventHandled> {
    if args.is_empty() {
        msg.reply(ctx.cache_http, "Usage: delete <player_name>")
            .await?;
        return Ok(EventHandled::Yes);
    }

    let player_name = args[0].to_string();
    let mut pstate = ctx.pstate.write().await;
    if !pstate.rivals_ratings.0.contains_key(&player_name) {
        msg.reply(
            ctx.cache_http,
            format!("Player `{}` not found.", player_name),
        )
        .await?;
        return Ok(EventHandled::Yes);
    }

    // Only the player owner or a bot owner may delete.
    if !msg.is_from_owner(ctx).await {
        if let Some(owner) = pstate.rivals_ratings_owners.0.get(&player_name) {
            if *owner != msg.author.id {
                let typing = msg.channel_id.start_typing(ctx.http);
                let cfg = ctx.cfg.read().await;
                let llm_settings = cfg.llm_permission_denied.as_llm_settings();
                let response =
                    LlmChatRequest::from_recent_history(ctx, msg.channel_id, &llm_settings)
                        .await?
                        .post(ctx)
                        .await
                        .map(Cow::Owned)?;
                typing.stop();
                msg.reply(ctx.cache_http, response).await?;
                return Ok(EventHandled::Yes);
            }
        } else {
            let typing = msg.channel_id.start_typing(ctx.http);
            let cfg = ctx.cfg.read().await;
            let llm_settings = cfg.llm_permission_denied.as_llm_settings();
            let response = LlmChatRequest::from_recent_history(ctx, msg.channel_id, &llm_settings)
                .await?
                .post(ctx)
                .await
                .map(Cow::Owned)?;
            typing.stop();
            msg.reply(ctx.cache_http, response).await?;
            return Ok(EventHandled::Yes);
        }
    }

    pstate.rivals_ratings.0.remove(&player_name);
    pstate.rivals_ratings_owners.0.remove(&player_name);
    pstate.save().await?;

    msg.reply(
        ctx.cache_http,
        format!("Player `{}` has been deleted.", player_name),
    )
    .await?;
    Ok(EventHandled::Yes)
}

async fn handle_list(ctx: &Context<'_>, msg: &Message) -> Result<EventHandled> {
    let pstate = ctx.pstate.read().await;
    if pstate.rivals_ratings.0.is_empty() {
        msg.reply(ctx.cache_http, "No players registered yet.")
            .await?;
        return Ok(EventHandled::Yes);
    }

    let mut list = Vec::new();

    // Collect and sort by rating
    for (player, rating) in &pstate.rivals_ratings.0 {
        let owner_id = pstate
            .rivals_ratings_owners
            .0
            .get(player)
            .cloned()
            .ok_or(anyhow!("Could not find owner for `{}`", player))?;
        list.push((player, rating, owner_id));
    }
    list.sort_unstable_by_key(|k| k.1);

    let mut response = String::from("Registered players:\n");
    for (player, rating, owner_id) in list.iter().rev() {
        response.push_str(&format!(
            "• `{}`: {}% (owner: <@{}>)\n",
            player, rating, owner_id
        ));
    }

    msg.reply(ctx.cache_http, response).await?;
    Ok(EventHandled::Yes)
}

async fn handle_preview(ctx: &Context<'_>, msg: &Message, args: &[&str]) -> Result<EventHandled> {
    if args.len() < 2 {
        msg.reply(ctx.cache_http, "Usage: preview <player1> <player2>")
            .await?;
        return Ok(EventHandled::Yes);
    }

    let player1 = args[0];
    let player2 = args[1];

    let pstate = ctx.pstate.read().await;
    let rating1 = match pstate.rivals_ratings.0.get(player1) {
        Some(&r) => r,
        None => {
            msg.reply(ctx.cache_http, format!("Player `{}` not found.", player1))
                .await?;
            return Ok(EventHandled::Yes);
        }
    };
    let rating2 = match pstate.rivals_ratings.0.get(player2) {
        Some(&r) => r,
        None => {
            msg.reply(ctx.cache_http, format!("Player `{}` not found.", player2))
                .await?;
            return Ok(EventHandled::Yes);
        }
    };

    let (higher, high_rating, low_rating) = match rating1.cmp(&rating2) {
        Ordering::Greater => (player1, rating1, rating2),
        Ordering::Less => (player2, rating2, rating1),
        Ordering::Equal => {
            msg.reply(
                ctx.cache_http,
                format!(
                    "Both `{}` and `{}` have equal ratings ({}%). No handicap.",
                    player1, player2, rating1
                ),
            )
            .await?;
            return Ok(EventHandled::Yes);
        }
    };

    let diff = high_rating - low_rating;
    let stocks = diff / STOCK_VALUE;
    let remainder = diff % STOCK_VALUE;

    let handicap = format!(
        "Handicap: `{}` should start with {} stock(s) and {}% extra damage.",
        higher, stocks, remainder
    );

    let response = format!(
        "Player ratings:\n• `{}`: {}%\n• `{}`: {}%\n{}",
        player1, rating1, player2, rating2, handicap
    );
    msg.reply(ctx.cache_http, response).await?;
    Ok(EventHandled::Yes)
}

async fn handle_report(ctx: &Context<'_>, msg: &Message, args: &[&str]) -> Result<EventHandled> {
    // Expected format: report <winner> beat <loser>
    if args.len() < 3 || args[1].to_lowercase() != "beat" {
        msg.reply(ctx.cache_http, "Usage: report <player1> beat <player2>")
            .await?;
        return Ok(EventHandled::Yes);
    }

    let winner_name = args[0];
    let loser_name = args[2];

    if winner_name == loser_name {
        msg.reply(
            ctx.cache_http,
            "Winner and loser cannot be the same player.",
        )
        .await?;
        return Ok(EventHandled::Yes);
    }

    let mut pstate = ctx.pstate.write().await;
    let winner_rating = match pstate.rivals_ratings.0.get(winner_name) {
        Some(&r) => r,
        None => {
            msg.reply(
                ctx.cache_http,
                format!("Player `{}` not found.", winner_name),
            )
            .await?;
            return Ok(EventHandled::Yes);
        }
    };
    let loser_rating = match pstate.rivals_ratings.0.get(loser_name) {
        Some(&r) => r,
        None => {
            msg.reply(
                ctx.cache_http,
                format!("Player `{}` not found.", loser_name),
            )
            .await?;
            return Ok(EventHandled::Yes);
        }
    };

    // Only the loser’s owner or a bot owner may report a match.
    if !msg.is_from_owner(ctx).await {
        if let Some(owner) = pstate.rivals_ratings_owners.0.get(loser_name) {
            if *owner != msg.author.id {
                let typing = msg.channel_id.start_typing(ctx.http);
                let cfg = ctx.cfg.read().await;
                let llm_settings = cfg.llm_permission_denied.as_llm_settings();
                let response =
                    LlmChatRequest::from_recent_history(ctx, msg.channel_id, &llm_settings)
                        .await?
                        .post(ctx)
                        .await
                        .map(Cow::Owned)?;
                typing.stop();
                msg.reply(ctx.cache_http, response).await?;
                return Ok(EventHandled::Yes);
            }
        } else {
            let typing = msg.channel_id.start_typing(ctx.http);
            let cfg = ctx.cfg.read().await;
            let llm_settings = cfg.llm_permission_denied.as_llm_settings();
            let response = LlmChatRequest::from_recent_history(ctx, msg.channel_id, &llm_settings)
                .await?
                .post(ctx)
                .await
                .map(Cow::Owned)?;
            typing.stop();
            msg.reply(ctx.cache_http, response).await?;
            return Ok(EventHandled::Yes);
        }
    }

    // Disallow update if ratings are too far apart.
    let rating_diff = if winner_rating > loser_rating {
        winner_rating - loser_rating
    } else {
        loser_rating - winner_rating
    };

    if rating_diff > MAX_DELTA {
        msg.reply(
            ctx.cache_http,
            "Player ratings are too far apart to update.",
        )
        .await?;
        return Ok(EventHandled::Yes);
    }

    // Calculate expected score for the winner using a logistic curve.
    // Using D = 200 for scaling.
    let expected_winner =
        1.0 / (1.0 + 10f64.powf((loser_rating as f64 - winner_rating as f64) / 200.0));
    let change = K_FACTOR * (1.0 - expected_winner);
    let new_winner = ((winner_rating as f64) + change).round() as usize;
    let new_loser = ((loser_rating as f64) - change).round() as usize;

    pstate
        .rivals_ratings
        .0
        .insert(winner_name.to_owned(), new_winner);
    pstate
        .rivals_ratings
        .0
        .insert(loser_name.to_owned(), new_loser);
    pstate.save().await?;

    let response = format!(
        "Match reported:\n• Winner `{}`: {}% → {}%\n• Loser `{}`: {}% → {}%",
        winner_name, winner_rating, new_winner, loser_name, loser_rating, new_loser
    );
    msg.reply(ctx.cache_http, response).await?;
    Ok(EventHandled::Yes)
}
